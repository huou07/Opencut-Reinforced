#!/usr/bin/env python3
"""Exact-SHA GitHub evidence helpers for the OR execution supervisor."""

from __future__ import annotations

import argparse
import datetime as _datetime
import json
import os
import re
import subprocess
import sys
import time
import urllib.error
import urllib.parse
import urllib.request
from pathlib import Path
from typing import Any, Callable, Mapping, Sequence


SCRIPT_DIR = Path(__file__).resolve().parent
REPO_ROOT = SCRIPT_DIR.parent
POLICY_PATH = REPO_ROOT / "docs" / "execution" / "EVIDENCE_POLICY.json"
DEFAULT_API_BASE = "https://api.github.com"
DEFAULT_USER_AGENT = "opencut-reinforced-execution-supervisor"
SHA_PATTERN = re.compile(r"^[0-9a-f]{40}$")
EVIDENCE_CLASSES = {
    "STATIC", "UNIT", "INTEGRATION", "NATIVE_RUNTIME", "PACKAGED_RUNTIME",
    "USER_JOURNEY", "CLEAN_ENVIRONMENT", "PERSISTENCE_RELAUNCH",
    "PERFORMANCE", "RESOURCE_STRESS", "CROSS_PLATFORM",
}


class EvidenceError(RuntimeError):
    """Raised when hosted evidence cannot prove a checkpoint."""


class RateLimitError(EvidenceError):
    """Raised when GitHub refuses a request because of API rate limits."""


class NotFoundError(EvidenceError):
    """Raised for a GitHub API 404 response."""


class WorkflowPending(EvidenceError):
    """Raised while an exact workflow run has not completed successfully."""


class NoWorkflowRun(WorkflowPending):
    """Raised when no exact workflow run exists yet."""


class WorkflowFailed(EvidenceError):
    """Raised when the newest exact workflow run completed unsuccessfully."""

    def __init__(self, message: str, run: Mapping[str, Any] | None = None) -> None:
        super().__init__(message)
        self.run = dict(run) if run is not None else None


class EvidenceTimeout(EvidenceError):
    """Raised when hosted verification exceeds the policy timeout."""


class PreviewRequiredError(EvidenceError):
    """Raised when a required preview cannot be verified or dispatched."""


def _require_object(value: Any, label: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise EvidenceError(f"{label} must be an object")
    return value


def _require_list(value: Any, label: str) -> list[Any]:
    if not isinstance(value, list):
        raise EvidenceError(f"{label} must be an array")
    return value


def _require_string(value: Any, label: str) -> str:
    if not isinstance(value, str) or not value:
        raise EvidenceError(f"{label} must be a non-empty string")
    return value


def _require_sha(value: Any, label: str) -> str:
    if not isinstance(value, str) or SHA_PATTERN.fullmatch(value) is None:
        raise EvidenceError(f"{label} must be a lowercase 40-character SHA")
    return value


def _require_int(value: Any, label: str, minimum: int = 0) -> int:
    if isinstance(value, bool) or not isinstance(value, int) or value < minimum:
        raise EvidenceError(f"{label} must be an integer >= {minimum}")
    return value


def load_policy(path: Path = POLICY_PATH) -> dict[str, Any]:
    try:
        policy = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise EvidenceError(f"cannot load evidence policy: {exc}") from exc
    validate_policy(policy)
    return policy


def validate_policy(policy: Mapping[str, Any]) -> None:
    """Validate the deterministic policy shape without network access."""

    if policy.get("schema_version") != 1:
        raise EvidenceError("evidence policy schema_version must be 1")
    _require_string(policy.get("enforced_from_checkpoint"), "enforced_from_checkpoint")
    repository = _require_object(policy.get("repository"), "repository")
    _require_string(repository.get("owner"), "repository.owner")
    _require_string(repository.get("name"), "repository.name")
    _require_string(repository.get("branch"), "repository.branch")

    github_api = _require_object(policy.get("github_api"), "github_api")
    _require_string(github_api.get("api_version"), "github_api.api_version")
    _require_int(github_api.get("authenticated_poll_seconds"), "github_api.authenticated_poll_seconds", 1)
    _require_int(github_api.get("unauthenticated_poll_seconds"), "github_api.unauthenticated_poll_seconds", 1)
    _require_int(github_api.get("timeout_seconds"), "github_api.timeout_seconds", 1)
    token_names = _require_list(github_api.get("token_environment_variables"), "github_api.token_environment_variables")
    if token_names != ["GH_TOKEN", "GITHUB_TOKEN"]:
        raise EvidenceError("github_api.token_environment_variables must prefer GH_TOKEN then GITHUB_TOKEN")
    if repository != {
        "owner": "huou07",
        "name": "Opencut-Reinforced",
        "branch": "main",
    }:
        raise EvidenceError("repository policy must target huou07/Opencut-Reinforced main")
    if github_api != {
        "api_version": "2022-11-28",
        "authenticated_poll_seconds": 15,
        "unauthenticated_poll_seconds": 90,
        "timeout_seconds": 7200,
        "token_environment_variables": ["GH_TOKEN", "GITHUB_TOKEN"],
    }:
        raise EvidenceError("github_api policy is not the approved deterministic contract")

    gates = _require_object(policy.get("required_gates"), "required_gates")
    if not gates:
        raise EvidenceError("required_gates must not be empty")
    for gate_id, raw_gate in gates.items():
        gate = _require_object(raw_gate, f"required_gates.{gate_id}")
        _require_string(gate.get("workflow_file"), f"required_gates.{gate_id}.workflow_file")
        _require_string(gate.get("workflow_name"), f"required_gates.{gate_id}.workflow_name")
        job_names = _require_list(gate.get("required_jobs"), f"required_gates.{gate_id}.required_jobs")
        if not job_names or any(not isinstance(name, str) or not name for name in job_names):
            raise EvidenceError(f"required_gates.{gate_id}.required_jobs must contain names")
    if gates != {
        "repository_hygiene": {
            "workflow_file": ".github/workflows/repo-hygiene.yml",
            "workflow_name": "Repository hygiene",
            "required_jobs": ["Repository hygiene"],
        },
        "platform_verification": {
            "workflow_file": ".github/workflows/platform-verification.yml",
            "workflow_name": "Platform verification",
            "required_jobs": [
                "Rust checks",
                "Flutter static and widget checks",
                "macOS native build and bridge smoke",
                "Linux native build",
                "Windows native build",
                "Android APK build",
            ],
        },
    }:
        raise EvidenceError("required_gates policy does not match the repository workflows")

    preview = _require_object(policy.get("developer_preview"), "developer_preview")
    _require_string(preview.get("workflow_file"), "developer_preview.workflow_file")
    _require_string(preview.get("workflow_name"), "developer_preview.workflow_name")
    _require_string(preview.get("tag_template"), "developer_preview.tag_template")
    _require_string(preview.get("publish_job"), "developer_preview.publish_job")
    allowed_events = _require_list(preview.get("allowed_events"), "developer_preview.allowed_events")
    if not allowed_events or any(not isinstance(event, str) or not event for event in allowed_events):
        raise EvidenceError("developer_preview.allowed_events must contain event names")
    assets = _require_list(preview.get("required_assets"), "developer_preview.required_assets")
    if not assets or any(not isinstance(asset, str) or not asset for asset in assets):
        raise EvidenceError("developer_preview.required_assets must contain names")
    if _require_int(preview.get("required_asset_count"), "developer_preview.required_asset_count", 1) < len(assets):
        raise EvidenceError("developer_preview.required_asset_count is smaller than required_assets")
    _require_string(preview.get("checksums_asset"), "developer_preview.checksums_asset")
    _require_string(preview.get("build_info_asset"), "developer_preview.build_info_asset")
    if preview != {
        "workflow_file": ".github/workflows/developer-preview.yml",
        "workflow_name": "Developer Preview",
        "allowed_events": ["workflow_dispatch", "schedule"],
        "publish_job": "Verify and publish prerelease",
        "tag_template": "dev-{sha12}",
        "required_asset_count": 11,
        "required_assets": [
            "opencut-reinforced-macos-debug.zip",
            "opencut-reinforced-windows-x64-debug.zip",
            "opencut-reinforced-linux-x64-debug.tar.gz",
            "opencut-reinforced-android-debug.apk",
            "opencut-reinforced-cli-macos.zip",
            "opencut-reinforced-cli-windows-x64.zip",
            "opencut-reinforced-cli-linux-x64.tar.gz",
            "SHA256SUMS.txt",
            "BUILD-INFO.txt",
            "FFMPEG-BUILD-INFO.txt",
            "ffmpeg-8.1.3-source.tar.xz",
        ],
        "checksums_asset": "SHA256SUMS.txt",
        "build_info_asset": "BUILD-INFO.txt",
    }:
        raise EvidenceError("developer_preview policy does not match the repository workflow")

    transition = _require_object(policy.get("state_transition"), "state_transition")
    if transition.get("runner_may_write_state") is not False:
        raise EvidenceError("state_transition.runner_may_write_state must be false")
    _require_string(transition.get("completion_status"), "state_transition.completion_status")
    _require_string(transition.get("successor_status"), "state_transition.successor_status")
    _require_string(transition.get("commit_subject_template"), "state_transition.commit_subject_template")
    paths = _require_list(transition.get("allowed_commit_paths"), "state_transition.allowed_commit_paths")
    if not paths or any(not isinstance(path, str) or not path for path in paths):
        raise EvidenceError("state_transition.allowed_commit_paths must contain paths")
    if transition != {
        "runner_may_write_state": False,
        "completion_status": "DONE",
        "successor_status": "NEXT",
        "allowed_commit_paths": [
            "docs/execution/STATE.json",
            "docs/execution/evidence/{checkpoint_id}.json",
        ],
        "commit_subject_template": "chore(execution): complete {checkpoint_id} after verified CI",
    }:
        raise EvidenceError("state_transition policy is not the approved deterministic contract")

    classes = _require_object(policy.get("evidence_classes"), "evidence_classes")
    if classes.get("enforced_from_checkpoint") != "9B":
        raise EvidenceError("evidence class enforcement must begin at 9B")
    allowed = _require_list(classes.get("allowed"), "evidence_classes.allowed")
    if any(not isinstance(item, str) for item in allowed) or len(allowed) != len(EVIDENCE_CLASSES) or set(allowed) != EVIDENCE_CLASSES:
        raise EvidenceError("evidence_classes.allowed must contain the locked class taxonomy")
    proof_map = _require_object(policy.get("evidence_class_proofs"), "evidence_class_proofs")
    for checkpoint_id, raw_classes in proof_map.items():
        _require_string(checkpoint_id, "evidence_class_proofs checkpoint")
        class_map = _require_object(raw_classes, f"evidence_class_proofs.{checkpoint_id}")
        for class_name, raw_sources in class_map.items():
            if class_name not in EVIDENCE_CLASSES:
                raise EvidenceError(f"unknown evidence class {class_name}")
            sources = _require_list(raw_sources, f"evidence_class_proofs.{checkpoint_id}.{class_name}")
            if not sources:
                raise EvidenceError(f"evidence class {class_name} has no proof source")
            seen: set[tuple[str, str, str]] = set()
            for raw_source in sources:
                source = _require_object(raw_source, f"{checkpoint_id}.{class_name} proof")
                if set(source) != {"gate_id", "job_name", "step_name"}:
                    raise EvidenceError(f"{checkpoint_id}.{class_name} proof has invalid fields")
                gate_id = _require_string(source["gate_id"], "proof.gate_id")
                job_name = _require_string(source["job_name"], "proof.job_name")
                step_name = _require_string(source["step_name"], "proof.step_name")
                if gate_id not in gates or job_name not in gates[gate_id]["required_jobs"]:
                    raise EvidenceError(f"{checkpoint_id}.{class_name} proof is outside required jobs")
                identity = (gate_id, job_name, step_name)
                if identity in seen:
                    raise EvidenceError(f"{checkpoint_id}.{class_name} has duplicate proof")
                seen.add(identity)


def parse_repository_identity(remote_url: str) -> tuple[str, str]:
    """Return ``(owner, repository)`` from an HTTPS or SSH Git remote."""

    value = remote_url.strip()
    if value.startswith("git@") and ":" in value:
        path = value.split(":", 1)[1]
    else:
        parsed = urllib.parse.urlsplit(value)
        path = parsed.path
    path = path.rstrip("/")
    if path.endswith(".git"):
        path = path[:-4]
    parts = [part for part in path.split("/") if part]
    if len(parts) != 2 or any(part in {".", ".."} for part in parts):
        raise EvidenceError("origin remote is not an owner/repository GitHub path")
    return parts[0], parts[1]


def repository_identity(repo_root: Path = REPO_ROOT) -> tuple[str, str]:
    try:
        result = subprocess.run(
            ["git", "config", "--get", "remote.origin.url"],
            cwd=repo_root,
            capture_output=True,
            text=True,
            check=True,
        )
    except (OSError, subprocess.CalledProcessError) as exc:
        raise EvidenceError("cannot read origin remote") from exc
    return parse_repository_identity(result.stdout)


def select_token(environment: Mapping[str, str] | None = None) -> str | None:
    """Select GH_TOKEN first, then GITHUB_TOKEN, without logging either value."""

    values = os.environ if environment is None else environment
    for name in ("GH_TOKEN", "GITHUB_TOKEN"):
        token = values.get(name)
        if token:
            return token
    return None


def polling_interval(policy: Mapping[str, Any], authenticated: bool) -> int:
    github_api = _require_object(policy.get("github_api"), "github_api")
    key = "authenticated_poll_seconds" if authenticated else "unauthenticated_poll_seconds"
    return _require_int(github_api.get(key), f"github_api.{key}", 1)


class GitHubApi:
    """Small standard-library GitHub REST client with injectable I/O for tests."""

    def __init__(
        self,
        owner: str,
        repository: str,
        *,
        token: str | None = None,
        api_version: str = "2022-11-28",
        base_url: str = DEFAULT_API_BASE,
        opener: Callable[..., Any] | None = None,
    ) -> None:
        self.owner = owner
        self.repository = repository
        self.token = token
        self.api_version = api_version
        self.base_url = base_url.rstrip("/")
        self._opener = opener or urllib.request.urlopen

    @property
    def authenticated(self) -> bool:
        return bool(self.token)

    def _url(self, path: str) -> str:
        if not path.startswith("/"):
            raise EvidenceError("GitHub API path must be absolute")
        return f"{self.base_url}{path}"

    def _headers(self) -> dict[str, str]:
        headers = {
            "Accept": "application/vnd.github+json",
            "X-GitHub-Api-Version": self.api_version,
            "User-Agent": DEFAULT_USER_AGENT,
        }
        if self.token:
            headers["Authorization"] = f"Bearer {self.token}"
        return headers

    def _decode(self, response: Any, method: str) -> Any:
        try:
            raw = response.read()
            if not raw:
                return None
            value = json.loads(raw.decode("utf-8"))
        except (UnicodeDecodeError, json.JSONDecodeError, AttributeError) as exc:
            raise EvidenceError(f"GitHub API {method} returned malformed JSON") from exc
        if not isinstance(value, (dict, list)):
            raise EvidenceError(f"GitHub API {method} returned an invalid JSON value")
        return value

    def _request(self, method: str, path: str, body: bytes | None = None) -> Any:
        headers = self._headers()
        if body is not None:
            headers["Content-Type"] = "application/json"
        request = urllib.request.Request(self._url(path), data=body, headers=headers, method=method)
        try:
            with self._opener(request, timeout=30) as response:
                value = self._decode(response, method)
                remaining = response.headers.get("X-RateLimit-Remaining", "")
                if remaining == "0":
                    raise RateLimitError(f"GitHub API rate limit reached for {method}")
                return value
        except urllib.error.HTTPError as exc:
            remaining = exc.headers.get("X-RateLimit-Remaining", "") if exc.headers else ""
            if exc.code in {403, 429} or remaining == "0":
                raise RateLimitError(f"GitHub API rate limit reached for {method}") from exc
            if exc.code == 404:
                raise NotFoundError(f"GitHub API resource not found for {method}") from exc
            raise EvidenceError(f"GitHub API {method} failed with HTTP {exc.code}") from exc
        except urllib.error.URLError as exc:
            raise EvidenceError(f"GitHub API {method} network request failed") from exc

    def get(self, path: str) -> Any:
        return self._request("GET", path)

    def post(self, path: str, payload: Mapping[str, Any]) -> Any:
        body = json.dumps(dict(payload), separators=(",", ":")).encode("utf-8")
        return self._request("POST", path, body)


def github_api_get(
    path: str,
    *,
    owner: str,
    repository: str,
    token: str | None = None,
    api_version: str = "2022-11-28",
    base_url: str = DEFAULT_API_BASE,
    opener: Callable[..., Any] | None = None,
) -> Any:
    """Perform one authenticated or public read-only GitHub API request."""

    return GitHubApi(
        owner,
        repository,
        token=token,
        api_version=api_version,
        base_url=base_url,
        opener=opener,
    ).get(path)


def _repo_path(policy: Mapping[str, Any], suffix: str) -> str:
    repository = _require_object(policy.get("repository"), "repository")
    owner = urllib.parse.quote(_require_string(repository.get("owner"), "repository.owner"), safe="")
    name = urllib.parse.quote(_require_string(repository.get("name"), "repository.name"), safe="")
    if not suffix.startswith("/"):
        suffix = "/" + suffix
    return f"/repos/{owner}/{name}{suffix}"


def _workflow_id(workflow_file: str) -> str:
    workflow_id = Path(workflow_file).name
    if not workflow_id:
        raise EvidenceError("workflow_file must include a filename")
    return workflow_id


def _workflow_runs_path(policy: Mapping[str, Any], workflow_file: str, implementation_sha: str) -> str:
    workflow = urllib.parse.quote(_workflow_id(workflow_file), safe="")
    params = urllib.parse.urlencode({"head_sha": implementation_sha, "per_page": 100})
    return _repo_path(policy, f"/actions/workflows/{workflow}/runs?{params}")


def _workflow_jobs_path(policy: Mapping[str, Any], run_id: int) -> str:
    params = urllib.parse.urlencode({"per_page": 100})
    return _repo_path(policy, f"/actions/runs/{run_id}/jobs?{params}")


def _run_timestamp(run: Mapping[str, Any]) -> str:
    for key in ("updated_at", "run_started_at", "created_at"):
        value = run.get(key)
        if isinstance(value, str):
            return value
    return ""


def _run_sort_key(run: Mapping[str, Any]) -> tuple[str, int, int]:
    attempt = run.get("run_attempt", 1)
    if isinstance(attempt, bool) or not isinstance(attempt, int):
        attempt = 1
    run_id = run.get("id", 0)
    if isinstance(run_id, bool) or not isinstance(run_id, int):
        run_id = 0
    return (_run_timestamp(run), attempt, run_id)


def _workflow_path_matches(actual: Any, expected: str) -> bool:
    if not isinstance(actual, str):
        return False
    normalized = actual.lstrip("./")
    expected_normalized = expected.lstrip("./")
    return normalized in {expected_normalized, f".github/workflows/{expected_normalized}"}


def _workflow_name(run: Mapping[str, Any]) -> Any:
    return run.get("name", run.get("workflow_name"))


def _validate_run_shape(run: Mapping[str, Any], label: str = "workflow run") -> None:
    for key in ("id", "head_sha", "head_branch", "event", "status"):
        if key not in run:
            raise EvidenceError(f"{label} is missing {key}")
    _require_int(run.get("id"), f"{label}.id", 1)
    _require_sha(run.get("head_sha"), f"{label}.head_sha")
    _require_string(run.get("head_branch"), f"{label}.head_branch")
    _require_string(run.get("event"), f"{label}.event")
    _require_string(run.get("status"), f"{label}.status")
    if _workflow_name(run) is None:
        raise EvidenceError(f"{label} is missing name")
    _require_string(_workflow_name(run), f"{label}.name")
    if "path" not in run:
        raise EvidenceError(f"{label} is missing path")
    _require_string(run.get("path"), f"{label}.path")


def validate_workflow_run(
    run: Mapping[str, Any],
    *,
    workflow_file: str,
    workflow_name: str,
    implementation_sha: str,
    branch: str,
    allowed_events: Sequence[str] = ("push",),
) -> None:
    """Require one exact successful workflow run, or raise a typed failure."""

    _validate_run_shape(run)
    if run["head_sha"] != implementation_sha:
        raise EvidenceError("workflow run has the wrong head SHA")
    if run["head_branch"] != branch:
        raise EvidenceError("workflow run has the wrong head branch")
    if run["event"] not in allowed_events:
        raise EvidenceError("workflow run has a disallowed event")
    if not _workflow_path_matches(run["path"], workflow_file):
        raise EvidenceError("workflow run has the wrong workflow file")
    if _workflow_name(run) != workflow_name:
        raise EvidenceError("workflow run has the wrong workflow name")
    if run["status"] != "completed":
        raise WorkflowPending(f"workflow run {run['id']} is {run['status']}")
    conclusion = run.get("conclusion")
    if conclusion != "success":
        if not isinstance(conclusion, str):
            conclusion = "missing"
        raise WorkflowFailed(f"workflow run {run['id']} concluded {conclusion}", run)


def select_exact_workflow_run(
    runs_payload: Mapping[str, Any] | Sequence[Any],
    *,
    workflow_file: str,
    workflow_name: str,
    implementation_sha: str,
    branch: str,
    allowed_events: Sequence[str] = ("push",),
) -> dict[str, Any]:
    """Select the newest exact run; stale successful runs never mask a newer run."""

    if isinstance(runs_payload, dict):
        runs = _require_list(runs_payload.get("workflow_runs"), "workflow_runs")
    elif isinstance(runs_payload, list):
        runs = runs_payload
    else:
        raise EvidenceError("workflow run response must be an object or array")
    if any(not isinstance(run, dict) for run in runs):
        raise EvidenceError("workflow_runs must contain objects")

    sha_runs = [run for run in runs if run.get("head_sha") == implementation_sha]
    if not sha_runs:
        raise NoWorkflowRun("no workflow run exists for the exact implementation SHA")
    identity_runs = [
        run
        for run in sha_runs
        if _workflow_path_matches(run.get("path"), workflow_file)
        and _workflow_name(run) == workflow_name
    ]
    if not identity_runs:
        raise EvidenceError("no run matches the required workflow file and name")
    candidate = max(identity_runs, key=_run_sort_key)
    validate_workflow_run(
        candidate,
        workflow_file=workflow_file,
        workflow_name=workflow_name,
        implementation_sha=implementation_sha,
        branch=branch,
        allowed_events=allowed_events,
    )
    return dict(candidate)


def validate_required_jobs(
    jobs_payload: Mapping[str, Any] | Sequence[Any], required_names: Sequence[str]
) -> list[dict[str, Any]]:
    """Require every policy job to be completed successfully."""

    if isinstance(jobs_payload, dict):
        jobs = _require_list(jobs_payload.get("jobs"), "jobs")
    elif isinstance(jobs_payload, list):
        jobs = jobs_payload
    else:
        raise EvidenceError("job response must be an object or array")
    if any(not isinstance(job, dict) for job in jobs):
        raise EvidenceError("jobs must contain objects")

    result: list[dict[str, Any]] = []
    for required_name in required_names:
        matches = [job for job in jobs if job.get("name") == required_name]
        if not matches:
            raise EvidenceError(f"required job is missing: {required_name}")
        job = max(matches, key=_run_sort_key)
        name = _require_string(job.get("name"), "job.name")
        status = _require_string(job.get("status"), f"job {name}.status")
        conclusion = job.get("conclusion")
        if status != "completed" or conclusion != "success":
            conclusion_text = conclusion if isinstance(conclusion, str) else "missing"
            raise EvidenceError(f"required job {name} is {status}/{conclusion_text}")
        result.append({"name": name, "status": status, "conclusion": conclusion})
    return result


def _required_job_failure_detail(
    jobs_payload: Mapping[str, Any] | Sequence[Any], required_names: Sequence[str]
) -> str:
    try:
        if isinstance(jobs_payload, dict):
            jobs = _require_list(jobs_payload.get("jobs"), "jobs")
        elif isinstance(jobs_payload, list):
            jobs = jobs_payload
        else:
            raise EvidenceError("job response must be an object or array")
        details: list[str] = []
        for required_name in required_names:
            matches = [job for job in jobs if isinstance(job, dict) and job.get("name") == required_name]
            if not matches:
                details.append(f"missing {required_name}")
                continue
            job = max(matches, key=_run_sort_key)
            status = job.get("status", "missing")
            conclusion = job.get("conclusion", "missing")
            if status != "completed" or conclusion != "success":
                details.append(f"{required_name} ({status}/{conclusion})")
        return ", ".join(details) if details else "all required jobs reported success"
    except EvidenceError as exc:
        return f"required jobs unavailable: {exc}"


def _workflow_gate_record(
    gate_id: str, gate: Mapping[str, Any], run: Mapping[str, Any], jobs: list[dict[str, Any]]
) -> dict[str, Any]:
    _validate_run_shape(run)
    return {
        "gate_id": gate_id,
        "workflow_name": gate["workflow_name"],
        "workflow_file": gate["workflow_file"],
        "run_id": run["id"],
        "run_attempt": run.get("run_attempt", 1),
        "head_sha": run["head_sha"],
        "head_branch": run["head_branch"],
        "event": run["event"],
        "status": run["status"],
        "conclusion": run.get("conclusion"),
        "html_url": _require_string(run.get("html_url"), "workflow run.html_url"),
        "jobs": jobs,
    }


def wait_for_workflow_gate(
    api: GitHubApi,
    policy: Mapping[str, Any],
    gate_id: str,
    implementation_sha: str,
    *,
    timeout_seconds: int | None = None,
    clock: Callable[[], float] = time.monotonic,
    sleep: Callable[[float], None] = time.sleep,
) -> dict[str, Any]:
    gates = _require_object(policy.get("required_gates"), "required_gates")
    gate = _require_object(gates.get(gate_id), f"required_gates.{gate_id}")
    repository = _require_object(policy.get("repository"), "repository")
    branch = _require_string(repository.get("branch"), "repository.branch")
    github_api = _require_object(policy.get("github_api"), "github_api")
    timeout = timeout_seconds if timeout_seconds is not None else _require_int(
        github_api.get("timeout_seconds"), "github_api.timeout_seconds", 1
    )
    interval = polling_interval(policy, api.authenticated)
    deadline = clock() + timeout
    while True:
        payload = api.get(_workflow_runs_path(policy, gate["workflow_file"], implementation_sha))
        try:
            run = select_exact_workflow_run(
                payload,
                workflow_file=gate["workflow_file"],
                workflow_name=gate["workflow_name"],
                implementation_sha=implementation_sha,
                branch=branch,
                allowed_events=("push",),
            )
            jobs_payload = api.get(_workflow_jobs_path(policy, int(run["id"])))
            try:
                jobs = validate_required_jobs(jobs_payload, gate["required_jobs"])
            except EvidenceError as exc:
                raise EvidenceError(
                    f"{gate_id} run {run['id']} ({run['html_url']}) has invalid required jobs: {exc}"
                ) from exc
            return _workflow_gate_record(gate_id, gate, run, jobs)
        except WorkflowFailed as exc:
            run = exc.run or {}
            try:
                job_detail = _required_job_failure_detail(
                    api.get(_workflow_jobs_path(policy, int(run["id"]))), gate["required_jobs"]
                )
            except RateLimitError:
                raise
            except (EvidenceError, KeyError, TypeError, ValueError) as job_exc:
                job_detail = f"required jobs unavailable: {job_exc}"
            raise WorkflowFailed(
                f"{gate_id} workflow run {run.get('id', 'unknown')} "
                f"({run.get('html_url', 'no URL')}) failed: {exc}; {job_detail}",
                run,
            ) from exc
        except WorkflowPending:
            remaining = deadline - clock()
            if remaining <= 0:
                raise EvidenceTimeout(f"timed out waiting for {gate_id} on exact SHA")
            sleep(min(interval, remaining))


def wait_for_required_workflows(
    api: GitHubApi,
    policy: Mapping[str, Any],
    implementation_sha: str,
    *,
    gate_ids: Sequence[str] | None = None,
    timeout_seconds: int | None = None,
    clock: Callable[[], float] = time.monotonic,
    sleep: Callable[[float], None] = time.sleep,
) -> list[dict[str, Any]]:
    gates = _require_object(policy.get("required_gates"), "required_gates")
    selected = list(gate_ids) if gate_ids is not None else list(gates)
    return [
        wait_for_workflow_gate(
            api,
            policy,
            gate_id,
            implementation_sha,
            timeout_seconds=timeout_seconds,
            clock=clock,
            sleep=sleep,
        )
        for gate_id in selected
    ]


def preview_tag(implementation_sha: str, policy: Mapping[str, Any]) -> str:
    _require_sha(implementation_sha, "implementation_sha")
    preview = _require_object(policy.get("developer_preview"), "developer_preview")
    template = _require_string(preview.get("tag_template"), "developer_preview.tag_template")
    return template.replace("{sha12}", implementation_sha[:12]).replace(
        "{implementation_sha}", implementation_sha
    )


def validate_preview_release(
    release: Mapping[str, Any], *, tag: str, implementation_sha: str, policy: Mapping[str, Any]
) -> list[dict[str, Any]]:
    preview = _require_object(policy.get("developer_preview"), "developer_preview")
    _require_sha(implementation_sha, "implementation_sha")
    if release.get("tag_name") != tag:
        raise EvidenceError("Developer Preview has the wrong tag")
    if release.get("prerelease") is not True:
        raise EvidenceError("Developer Preview is not a prerelease")
    release_url = _require_string(release.get("html_url"), "Developer Preview.html_url")
    del release_url
    assets = _require_list(release.get("assets"), "Developer Preview.assets")
    required_assets = [str(name) for name in _require_list(preview.get("required_assets"), "developer_preview.required_assets")]
    expected_count = _require_int(preview.get("required_asset_count"), "developer_preview.required_asset_count", 1)
    if len(assets) != expected_count:
        raise EvidenceError("Developer Preview has the wrong asset count")
    by_name: dict[str, dict[str, Any]] = {}
    for raw_asset in assets:
        asset = _require_object(raw_asset, "Developer Preview asset")
        name = _require_string(asset.get("name"), "Developer Preview asset.name")
        size = _require_int(asset.get("size"), f"Developer Preview asset {name}.size", 1)
        by_name[name] = {"name": name, "size": size}
    for name in required_assets:
        if name not in by_name:
            raise EvidenceError(f"Developer Preview asset is missing: {name}")
    for special_name in (preview["checksums_asset"], preview["build_info_asset"]):
        if special_name not in by_name:
            raise EvidenceError(f"Developer Preview metadata asset is missing: {special_name}")
    return [by_name[name] for name in required_assets]


def validate_preview_source_sha(resolved_sha: str, implementation_sha: str) -> None:
    if _require_sha(resolved_sha, "resolved preview source SHA") != _require_sha(
        implementation_sha, "implementation_sha"
    ):
        raise EvidenceError("Developer Preview tag points to the wrong source SHA")


def _tag_commit(api: GitHubApi, policy: Mapping[str, Any], tag: str) -> str:
    ref_payload = _require_object(api.get(_repo_path(policy, f"/git/ref/tags/{urllib.parse.quote(tag, safe='')}")), "tag ref")
    ref_object = _require_object(ref_payload.get("object"), "tag ref.object")
    object_type = _require_string(ref_object.get("type"), "tag ref.object.type")
    object_sha = _require_sha(ref_object.get("sha"), "tag ref.object.sha")
    if object_type == "commit":
        return object_sha
    if object_type != "tag":
        raise EvidenceError("Developer Preview tag does not resolve to a commit or annotated tag")
    tag_payload = _require_object(api.get(_repo_path(policy, f"/git/tags/{object_sha}")), "annotated tag")
    tag_object = _require_object(tag_payload.get("object"), "annotated tag.object")
    if tag_object.get("type") != "commit":
        raise EvidenceError("Developer Preview annotated tag does not resolve to a commit")
    return _require_sha(tag_object.get("sha"), "annotated tag.object.sha")


def wait_for_developer_preview(
    api: GitHubApi,
    policy: Mapping[str, Any],
    implementation_sha: str,
    *,
    dispatch_if_missing: bool = True,
    timeout_seconds: int | None = None,
    clock: Callable[[], float] = time.monotonic,
    sleep: Callable[[float], None] = time.sleep,
) -> dict[str, Any]:
    """Verify the existing preview workflow and exact release, dispatching only that workflow when allowed."""

    preview = _require_object(policy.get("developer_preview"), "developer_preview")
    repository = _require_object(policy.get("repository"), "repository")
    branch = _require_string(repository.get("branch"), "repository.branch")
    tag = preview_tag(implementation_sha, policy)
    github_api = _require_object(policy.get("github_api"), "github_api")
    timeout = timeout_seconds if timeout_seconds is not None else _require_int(
        github_api.get("timeout_seconds"), "github_api.timeout_seconds", 1
    )
    interval = polling_interval(policy, api.authenticated)
    deadline = clock() + timeout
    dispatched = False

    def dispatch_if_needed() -> None:
        nonlocal dispatched
        if not api.authenticated:
            raise PreviewRequiredError(
                "Developer Preview required but cannot be dispatched automatically"
            )
        if dispatch_if_missing and not dispatched:
            api.post(
                _repo_path(
                    policy,
                    f"/actions/workflows/{urllib.parse.quote(_workflow_id(preview['workflow_file']), safe='')}/dispatches",
                ),
                {"ref": branch},
            )
            dispatched = True

    while True:
        try:
            run_payload = api.get(_workflow_runs_path(policy, preview["workflow_file"], implementation_sha))
            run = select_exact_workflow_run(
                run_payload,
                workflow_file=preview["workflow_file"],
                workflow_name=preview["workflow_name"],
                implementation_sha=implementation_sha,
                branch=branch,
                allowed_events=tuple(preview["allowed_events"]),
            )
            jobs = validate_required_jobs(
                api.get(_workflow_jobs_path(policy, int(run["id"]))), [preview["publish_job"]]
            )
            release = api.get(_repo_path(policy, f"/releases/tags/{urllib.parse.quote(tag, safe='')}"))
            assets = validate_preview_release(
                release, tag=tag, implementation_sha=implementation_sha, policy=policy
            )
            resolved_sha = _tag_commit(api, policy, tag)
            validate_preview_source_sha(resolved_sha, implementation_sha)
            return {
                "required": True,
                "status": "verified",
                "tag": tag,
                "source_sha": implementation_sha,
                "workflow_file": preview["workflow_file"],
                "workflow_name": preview["workflow_name"],
                "workflow_run": {
                    "id": run["id"],
                    "attempt": run.get("run_attempt", 1),
                    "url": _require_string(run.get("html_url"), "preview workflow run.html_url"),
                    "status": run["status"],
                    "conclusion": run.get("conclusion"),
                    "jobs": jobs,
                },
                "release_url": _require_string(release.get("html_url"), "Developer Preview.html_url"),
                "assets": assets,
                "checksums": preview["checksums_asset"],
                "build_info": preview["build_info_asset"],
            }
        except NotFoundError:
            dispatch_if_needed()
            remaining = deadline - clock()
            if remaining <= 0:
                raise EvidenceTimeout("timed out waiting for Developer Preview")
            sleep(min(interval, remaining))
        except NoWorkflowRun:
            dispatch_if_needed()
            remaining = deadline - clock()
            if remaining <= 0:
                raise EvidenceTimeout("timed out waiting for Developer Preview workflow")
            sleep(min(interval, remaining))
        except WorkflowPending:
            remaining = deadline - clock()
            if remaining <= 0:
                raise EvidenceTimeout("timed out waiting for Developer Preview workflow")
            sleep(min(interval, remaining))


def build_evidence_record(
    *,
    checkpoint_id: str,
    implementation_sha: str,
    implementation_subject: str,
    gates: Sequence[Mapping[str, Any]],
    developer_preview: Mapping[str, Any],
    contract_versions: Mapping[str, Any],
    implementation_origin_sha: str | None = None,
    verified_at_utc: str | None = None,
    evidence_classes: Sequence[Mapping[str, Any]] | None = None,
) -> dict[str, Any]:
    _require_sha(implementation_sha, "implementation_sha")
    _require_string(checkpoint_id, "checkpoint_id")
    _require_string(implementation_subject, "implementation_subject")
    timestamp = verified_at_utc or _datetime.datetime.now(_datetime.timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z")
    versions = _validate_contract_versions(contract_versions, "contract_versions")
    record = {
        "schema_version": 2 if evidence_classes is not None else 1,
        "checkpoint_id": checkpoint_id,
        "implementation_sha": implementation_sha,
        "implementation_subject": implementation_subject,
        "verified_at_utc": timestamp,
        "gates": [dict(gate) for gate in gates],
        "developer_preview": dict(developer_preview),
        "contract_versions": versions,
    }
    if implementation_origin_sha is not None:
        record["implementation_origin_sha"] = _require_sha(
            implementation_origin_sha, "implementation_origin_sha"
        )
    if evidence_classes is not None:
        record["evidence_classes"] = [dict(proof) for proof in evidence_classes]
    return record


def required_class_sources(
    checkpoint: Mapping[str, Any], policy: Mapping[str, Any]
) -> dict[str, list[dict[str, str]]]:
    if checkpoint.get("evidence_contract_version") != 2:
        return {}
    checkpoint_id = _require_string(checkpoint.get("id"), "checkpoint.id")
    required = _require_list(checkpoint.get("required_evidence_classes"), "required_evidence_classes")
    sources = _require_object(
        _require_object(policy.get("evidence_class_proofs"), "evidence_class_proofs").get(checkpoint_id),
        f"evidence_class_proofs.{checkpoint_id}",
    )
    if set(sources) != set(required):
        raise EvidenceError(f"checkpoint {checkpoint_id} evidence proof classes do not match PLAN.json")
    return {name: sources[name] for name in required}


def collect_evidence_class_proofs(
    api: GitHubApi,
    policy: Mapping[str, Any],
    checkpoint: Mapping[str, Any],
    gates: Sequence[Mapping[str, Any]],
) -> list[dict[str, Any]]:
    """Bind each required class to a successful named step in an exact-SHA gate."""

    sources = required_class_sources(checkpoint, policy)
    by_gate = {gate["gate_id"]: gate for gate in gates}
    job_cache: dict[str, list[dict[str, Any]]] = {}
    proofs: list[dict[str, Any]] = []
    for class_name, bindings in sources.items():
        for binding in bindings:
            gate_id = binding["gate_id"]
            gate = by_gate[gate_id]
            if gate_id not in job_cache:
                payload = api.get(_workflow_jobs_path(policy, gate["run_id"]))
                raw_jobs = _require_list(
                    _require_object(payload, f"{gate_id} jobs").get("jobs"), f"{gate_id} jobs.jobs"
                )
                job_cache[gate_id] = [_require_object(job, f"{gate_id} proof job") for job in raw_jobs]
            matches = [job for job in job_cache[gate_id] if job.get("name") == binding["job_name"]]
            if not matches:
                raise EvidenceError(f"{class_name} proof job is missing: {binding['job_name']}")
            job = max(matches, key=_run_sort_key)
            if job.get("status") != "completed" or job.get("conclusion") != "success":
                raise EvidenceError(f"{class_name} proof job did not succeed")
            steps = _require_list(job.get("steps"), f"{class_name} proof steps")
            named = [step for step in steps if isinstance(step, dict) and step.get("name") == binding["step_name"]]
            if len(named) != 1 or named[0].get("status") != "completed" or named[0].get("conclusion") != "success":
                raise EvidenceError(f"{class_name} proof step did not succeed: {binding['step_name']}")
            proofs.append({
                "class": class_name,
                "gate_id": gate_id,
                "run_id": gate["run_id"],
                "job_name": binding["job_name"],
                "job_id": _require_int(job.get("id"), "proof.job_id", 1),
                "step_name": binding["step_name"],
                "step_number": _require_int(named[0].get("number"), "proof.step_number", 1),
                "status": "completed",
                "conclusion": "success",
            })
    return proofs


def _validate_contract_versions(value: Any, label: str) -> dict[str, int]:
    versions = _require_object(value, label)
    expected = {"project_schema", "recovery_schema", "ipc_protocol"}
    if set(versions) != expected:
        raise EvidenceError(f"{label} must contain exactly: {', '.join(sorted(expected))}")
    for key, version in versions.items():
        if not isinstance(version, int) or isinstance(version, bool) or version < 1:
            raise EvidenceError(f"{label}.{key} must be a positive integer")
    return {key: versions[key] for key in sorted(versions)}


def _validate_timestamp(value: Any) -> None:
    timestamp = _require_string(value, "verified_at_utc")
    try:
        parsed = _datetime.datetime.fromisoformat(timestamp.replace("Z", "+00:00"))
    except ValueError as exc:
        raise EvidenceError("verified_at_utc must be ISO-8601") from exc
    if parsed.tzinfo is None or parsed.utcoffset() is None:
        raise EvidenceError("verified_at_utc must include a UTC offset")
    if parsed.utcoffset() != _datetime.timedelta(0):
        raise EvidenceError("verified_at_utc must be UTC")


def validate_evidence_record(
    record: Mapping[str, Any],
    *,
    checkpoint_id: str,
    checkpoint: Mapping[str, Any],
    policy: Mapping[str, Any],
) -> None:
    """Validate an offline supervisor-owned completion attestation."""

    required_schema = checkpoint.get("evidence_contract_version", 1)
    if record.get("schema_version") != required_schema:
        raise EvidenceError(f"evidence record schema_version must be {required_schema}")
    if record.get("checkpoint_id") != checkpoint_id:
        raise EvidenceError("evidence record checkpoint_id does not match")
    implementation_sha = _require_sha(record.get("implementation_sha"), "implementation_sha")
    if "implementation_origin_sha" in record:
        _require_sha(record["implementation_origin_sha"], "implementation_origin_sha")
    if "contract_versions" in record:
        _validate_contract_versions(record["contract_versions"], "contract_versions")
    _require_string(record.get("implementation_subject"), "implementation_subject")
    _validate_timestamp(record.get("verified_at_utc"))

    gates = _require_list(record.get("gates"), "gates")
    required_gates = _require_object(policy.get("required_gates"), "required_gates")
    by_id: dict[str, dict[str, Any]] = {}
    for raw_gate in gates:
        gate = _require_object(raw_gate, "gate")
        gate_id = _require_string(gate.get("gate_id"), "gate.gate_id")
        if gate_id in by_id:
            raise EvidenceError(f"duplicate evidence gate: {gate_id}")
        by_id[gate_id] = gate
    if set(by_id) != set(required_gates):
        missing = sorted(set(required_gates) - set(by_id))
        extra = sorted(set(by_id) - set(required_gates))
        details: list[str] = []
        if missing:
            details.append(f"missing gates: {', '.join(missing)}")
        if extra:
            details.append(f"unknown gates: {', '.join(extra)}")
        raise EvidenceError("; ".join(details))
    repository = _require_object(policy.get("repository"), "repository")
    branch = _require_string(repository.get("branch"), "repository.branch")
    for gate_id, policy_gate in required_gates.items():
        gate = by_id[gate_id]
        if gate.get("workflow_name") != policy_gate["workflow_name"]:
            raise EvidenceError(f"gate {gate_id} has the wrong workflow name")
        if gate.get("workflow_file") != policy_gate["workflow_file"]:
            raise EvidenceError(f"gate {gate_id} has the wrong workflow file")
        if gate.get("head_sha") != implementation_sha:
            raise EvidenceError(f"gate {gate_id} has the wrong head SHA")
        if gate.get("head_branch") != branch:
            raise EvidenceError(f"gate {gate_id} has the wrong head branch")
        if gate.get("event") != "push":
            raise EvidenceError(f"gate {gate_id} has the wrong event")
        if gate.get("status") != "completed" or gate.get("conclusion") != "success":
            raise EvidenceError(f"gate {gate_id} is not completed successfully")
        _require_int(gate.get("run_id"), f"gate {gate_id}.run_id", 1)
        _require_int(gate.get("run_attempt"), f"gate {gate_id}.run_attempt", 1)
        _require_string(gate.get("html_url"), f"gate {gate_id}.html_url")
        jobs = validate_required_jobs(gate.get("jobs"), policy_gate["required_jobs"])
        if jobs != gate.get("jobs"):
            raise EvidenceError(f"gate {gate_id}.jobs contains unexpected entries")

    if required_schema == 2:
        sources = required_class_sources(checkpoint, policy)
        proofs = _require_list(record.get("evidence_classes"), "evidence_classes")
        expected = [
            (name, binding["gate_id"], binding["job_name"], binding["step_name"])
            for name, bindings in sources.items()
            for binding in bindings
        ]
        actual = []
        for raw_proof in proofs:
            proof = _require_object(raw_proof, "evidence class proof")
            if set(proof) != {
                "class", "gate_id", "run_id", "job_name", "job_id",
                "step_name", "step_number", "status", "conclusion",
            }:
                raise EvidenceError("evidence class proof has invalid fields")
            gate_id = _require_string(proof["gate_id"], "proof.gate_id")
            if gate_id not in by_id or proof["run_id"] != by_id[gate_id]["run_id"]:
                raise EvidenceError("evidence class proof has the wrong exact-SHA run")
            if proof["job_name"] not in {job["name"] for job in by_id[gate_id]["jobs"]}:
                raise EvidenceError("evidence class proof has an unverified job")
            _require_int(proof["job_id"], "proof.job_id", 1)
            _require_int(proof["step_number"], "proof.step_number", 1)
            if proof["status"] != "completed" or proof["conclusion"] != "success":
                raise EvidenceError("evidence class proof step did not succeed")
            actual.append((proof["class"], gate_id, proof["job_name"], proof["step_name"]))
        if actual != expected:
            raise EvidenceError("evidence class proofs do not match PLAN.json and policy")
    elif "evidence_classes" in record:
        raise EvidenceError("legacy evidence must not claim new evidence classes")

    preview = _require_object(record.get("developer_preview"), "developer_preview")
    required = checkpoint.get("developer_preview_required") is True
    if preview.get("required") is not required:
        raise EvidenceError("developer_preview.required does not match PLAN.json")
    if not required:
        if preview.keys() != {"required"}:
            raise EvidenceError("unrequired developer_preview must contain only required=false")
    else:
        if preview.get("status") != "verified" or preview.get("source_sha") != implementation_sha:
            raise EvidenceError("required Developer Preview evidence is incomplete")
        preview_policy = _require_object(policy.get("developer_preview"), "developer_preview")
        if preview.get("tag") != preview_tag(implementation_sha, policy):
            raise EvidenceError("developer_preview.tag does not match policy")
        if preview.get("workflow_file") != preview_policy["workflow_file"]:
            raise EvidenceError("developer_preview.workflow_file does not match policy")
        if preview.get("workflow_name") != preview_policy["workflow_name"]:
            raise EvidenceError("developer_preview.workflow_name does not match policy")
        _require_string(preview.get("release_url"), "developer_preview.release_url")
        workflow_run = _require_object(preview.get("workflow_run"), "developer_preview.workflow_run")
        _require_int(workflow_run.get("id"), "developer_preview.workflow_run.id", 1)
        _require_int(workflow_run.get("attempt"), "developer_preview.workflow_run.attempt", 1)
        _require_string(workflow_run.get("url"), "developer_preview.workflow_run.url")
        if workflow_run.get("status") != "completed" or workflow_run.get("conclusion") != "success":
            raise EvidenceError("developer_preview.workflow_run is not successful")
        validate_required_jobs(workflow_run.get("jobs"), [preview_policy["publish_job"]])
        assets = _require_list(preview.get("assets"), "developer_preview.assets")
        if len(assets) != preview_policy["required_asset_count"]:
            raise EvidenceError("developer_preview.assets has the wrong count")
        asset_names: set[str] = set()
        for raw_asset in assets:
            asset = _require_object(raw_asset, "developer_preview asset")
            name = _require_string(asset.get("name"), "developer_preview asset.name")
            _require_int(asset.get("size"), f"developer_preview asset {name}.size", 1)
            asset_names.add(name)
        for required_asset in preview_policy["required_assets"]:
            if required_asset not in asset_names:
                raise EvidenceError(f"developer_preview.assets is missing {required_asset}")
        if preview.get("checksums") != preview_policy["checksums_asset"]:
            raise EvidenceError("developer_preview.checksums does not match policy")
        if preview.get("build_info") != preview_policy["build_info_asset"]:
            raise EvidenceError("developer_preview.build_info does not match policy")


def platform_verification_runs_for_push(changed_paths: Sequence[str]) -> bool:
    """Mirror the workflow path filter: any non-state/evidence change runs it."""

    if not changed_paths:
        return False
    return any(
        path != "docs/execution/STATE.json"
        and not path.startswith("docs/execution/evidence/")
        for path in changed_paths
    )


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--policy", type=Path, default=POLICY_PATH)
    args = parser.parse_args(argv)
    try:
        policy = load_policy(args.policy)
        print(json.dumps(policy, indent=2, sort_keys=True))
        return 0
    except EvidenceError as exc:
        print(f"Evidence policy error: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
