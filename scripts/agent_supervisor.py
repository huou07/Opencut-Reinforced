#!/usr/bin/env python3
"""Run fresh checkpoint runners and complete them only after hosted evidence."""

from __future__ import annotations

import argparse
import copy
import datetime as _datetime
import hashlib
import json
import os
import re
import subprocess
import sys
import time
from pathlib import Path
from typing import Any, Callable, Mapping, Sequence

SCRIPT_DIR = Path(__file__).resolve().parent
REPO_ROOT = SCRIPT_DIR.parent
sys.path.insert(0, str(SCRIPT_DIR))

import execution_evidence  # noqa: E402
import execution_plan  # noqa: E402


class SupervisorError(RuntimeError):
    """Raised when the direct-main execution contract is not satisfied."""


EXPLICIT_PROTECTED_PATHS = {
    "AGENTS.md",
    "docs/execution/PLAN.json",
    "docs/execution/STATE.json",
    "docs/execution/EVIDENCE_POLICY.json",
    "docs/execution/ARCHITECTURE_INVARIANTS.md",
    "docs/execution/architecture-policy.json",
    "scripts/agent_supervisor.py",
    "scripts/execution_plan.py",
    "scripts/execution_evidence.py",
    "scripts/check_execution_plan.py",
    "scripts/check_architecture_policy.py",
    "scripts/test_execution_infra.py",
    "docs/execution/AMENDMENT_BASELINE.json",
}
AMENDMENT_MARKER = "docs/execution/AMENDMENT_BASELINE.json"
AMENDMENT_CONTROL_PATHS = EXPLICIT_PROTECTED_PATHS | {
    "AGENTS.md",
    "docs/INDEX.md",
    "docs/ROADMAP.md",
    "docs/TESTING.md",
    "docs/adr/README.md",
    "docs/adr/0008-product-acceptance-and-execution-quality.md",
    "docs/execution/README.md",
    "docs/execution/PHASE_SPEC_TEMPLATE.md",
    "docs/execution/AGENT_EXECUTION.md",
    "docs/execution/phases/PHASE_9.md",
    "docs/execution/evidence/README.md",
}
PROTECTED_DIRECTORY_PREFIXES = (
    "docs/execution/phases/",
    "docs/execution/evidence/",
    ".github/workflows/",
)
REPAIR_ALLOWED_CONTROL_PATHS = {
    "docs/execution/PLAN.json",
    "docs/execution/STATE.json",
    "docs/execution/architecture-policy.json",
    "docs/execution/AGENT_EXECUTION.md",
    "scripts/agent_supervisor.py",
    "scripts/execution_plan.py",
    "scripts/check_execution_plan.py",
    "scripts/check_architecture_policy.py",
    "scripts/execution_evidence.py",
    "scripts/test_execution_infra.py",
}
REPAIR_ALLOWED_PREFIXES = ("docs/execution/phases/",)
REPAIR_TEST_PREFIXES = (
    "apps/or_app/test/",
    "apps/or_app/integration_test/",
)


def git_output(repo_root: Path, *arguments: str) -> str:
    result = subprocess.run(
        ["git", *arguments],
        cwd=repo_root,
        capture_output=True,
        text=True,
        check=True,
    )
    return result.stdout.strip()


def parse_porcelain_status_paths(status: str) -> list[str]:
    """Extract paths while preserving Git's two-character status columns."""

    return [line[3:] for line in status.splitlines() if len(line) >= 4]


def git_status_paths(repo_root: Path) -> list[str]:
    result = subprocess.run(
        ["git", "status", "--porcelain", "--untracked-files=all"],
        cwd=repo_root,
        capture_output=True,
        text=True,
        check=True,
    )
    return parse_porcelain_status_paths(result.stdout)


def is_protected_execution_path(path: str) -> bool:
    normalized = path.replace("\\", "/")
    while normalized.startswith("./"):
        normalized = normalized[2:]
    return normalized in EXPLICIT_PROTECTED_PATHS or any(
        normalized.startswith(prefix) for prefix in PROTECTED_DIRECTORY_PREFIXES
    )


def _all_protected_paths(repo_root: Path) -> list[str]:
    paths = set(EXPLICIT_PROTECTED_PATHS)
    for prefix in ("docs/execution/phases", "docs/execution/evidence", ".github/workflows"):
        directory = repo_root / prefix
        if directory.is_dir():
            paths.update(
                path.relative_to(repo_root).as_posix()
                for path in directory.rglob("*")
                if path.is_file()
            )
    return sorted(paths)


def capture_protected_surfaces(repo_root: Path = REPO_ROOT) -> dict[str, bytes | None]:
    """Capture control-plane files, including files created under protected directories."""

    snapshot: dict[str, bytes | None] = {}
    for relative_path in _all_protected_paths(repo_root):
        path = repo_root / relative_path
        snapshot[relative_path] = path.read_bytes() if path.is_file() else None
    return snapshot


def changed_protected_surfaces(
    before: Mapping[str, bytes | None],
    after: Mapping[str, bytes | None],
    allowed_paths: Sequence[str] = (),
) -> list[str]:
    allowed = set(allowed_paths)
    return sorted(
        path
        for path in set(before) | set(after)
        if path not in allowed and before.get(path) != after.get(path)
    )


def assert_protected_surfaces_unchanged(
    before: Mapping[str, bytes | None],
    after: Mapping[str, bytes | None],
    allowed_paths: Sequence[str] = (),
) -> None:
    changed = changed_protected_surfaces(before, after, allowed_paths)
    if changed:
        raise SupervisorError(
            "runner changed protected execution-control files: " + ", ".join(changed)
        )


def refuse_dirty_worktree(repo_root: Path) -> None:
    status = git_output(repo_root, "status", "--porcelain")
    if status:
        raise SupervisorError("refusing dirty worktree")


def ensure_start_state(repo_root: Path = REPO_ROOT) -> dict[str, str]:
    refuse_dirty_worktree(repo_root)
    branch = git_output(repo_root, "branch", "--show-current")
    if branch != "main":
        raise SupervisorError(f"supervisor requires branch main, found {branch or 'detached'}")
    git_output(repo_root, "fetch", "--prune", "origin")
    head = git_output(repo_root, "rev-parse", "HEAD")
    origin = git_output(repo_root, "rev-parse", "origin/main")
    if head != origin:
        raise SupervisorError("supervisor requires HEAD == origin/main")
    counts = git_output(repo_root, "rev-list", "--left-right", "--count", "HEAD...origin/main")
    if counts.replace("\t", " ").split() != ["0", "0"]:
        raise SupervisorError(f"supervisor requires ahead/behind 0/0, found {counts}")
    return {"branch": branch, "head": head, "origin_main": origin, "ahead_behind": counts}


def invoke_runner(repo_root: Path, runner: str | Path, prompt: str) -> int:
    runner_path = Path(runner)
    if not runner_path.is_absolute():
        raise SupervisorError("runner must be an absolute executable path")
    if not runner_path.is_file():
        raise SupervisorError(f"runner does not exist: {runner_path}")
    result = subprocess.run([str(runner_path)], input=prompt, text=True, cwd=repo_root)
    if result.returncode != 0:
        raise SupervisorError(f"runner exited with status {result.returncode}")
    return result.returncode


def assert_state_advanced_once(
    before: dict[str, Any],
    after: dict[str, Any],
    plan: dict[str, Any],
    *,
    candidate_contract_versions: Mapping[str, Any] | None = None,
    policy: Mapping[str, Any] | None = None,
    repo_root: Path = REPO_ROOT,
) -> str:
    """Validate the exact checkpoint, phase, and contract-version state transition."""

    if set(before) != set(after):
        raise SupervisorError("state transition changed the set of root fields")
    allowed_root_changes = {
        "checkpoints",
        "current_next",
        "phase_status",
        "last_updated",
        "verified_contract_versions",
    }
    for key in before:
        if key not in allowed_root_changes and before[key] != after[key]:
            raise SupervisorError(f"state transition changed unrelated root field {key}")

    before_checkpoints = before.get("checkpoints")
    after_checkpoints = after.get("checkpoints")
    if not isinstance(before_checkpoints, dict) or not isinstance(after_checkpoints, dict):
        raise SupervisorError("state checkpoints are not objects")
    if set(before_checkpoints) != set(after_checkpoints):
        raise SupervisorError("state changed the set of checkpoint ids")
    transitions = [
        checkpoint_id
        for checkpoint_id in before_checkpoints
        if before_checkpoints[checkpoint_id] != after_checkpoints[checkpoint_id]
    ]
    current_next_ids = [
        checkpoint_id
        for checkpoint_id, status in before_checkpoints.items()
        if status == "NEXT"
    ]
    if len(current_next_ids) != 1:
        raise SupervisorError("before-state must have exactly one NEXT checkpoint")
    checkpoint_id = current_next_ids[0]
    if after_checkpoints[checkpoint_id] != "DONE":
        raise SupervisorError(f"checkpoint {checkpoint_id} did not transition NEXT -> DONE")
    checkpoint = execution_plan.checkpoint_for_id(plan, checkpoint_id)
    expected_next = checkpoint["next_checkpoint_relation"]
    allowed_transitions = {checkpoint_id}
    if expected_next is not None:
        if before_checkpoints.get(expected_next) != "PLANNED":
            raise SupervisorError(f"successor {expected_next} was not PLANNED before the transition")
        if after_checkpoints.get(expected_next) != "NEXT":
            raise SupervisorError(f"successor {expected_next} did not become NEXT")
        allowed_transitions.add(expected_next)
    elif any(status == "NEXT" for status in after_checkpoints.values()):
        raise SupervisorError("final checkpoint transition left an unexpected NEXT checkpoint")
    for other_id in before_checkpoints:
        if other_id not in allowed_transitions and before_checkpoints[other_id] != after_checkpoints[other_id]:
            raise SupervisorError(f"state changed unrelated checkpoint {other_id}")
    if set(transitions) != allowed_transitions:
        raise SupervisorError(
            "state must advance exactly one checkpoint and its planned successor; "
            f"changed: {', '.join(transitions) or 'none'}"
        )
    if after.get("current_next") != expected_next:
        raise SupervisorError(
            f"state current_next is {after.get('current_next')!r}, expected {expected_next!r}"
        )
    expected_phases = execution_plan.derive_phase_statuses(
        plan, after_checkpoints, repo_root
    )
    if after.get("phase_status") != expected_phases:
        raise SupervisorError("state phase_status does not match derived checkpoint status")
    try:
        _datetime.date.fromisoformat(after.get("last_updated", ""))
    except (TypeError, ValueError) as exc:
        raise SupervisorError("state last_updated must be an ISO date") from exc

    candidate = (
        execution_plan.read_contract_versions(repo_root)
        if candidate_contract_versions is None
        else candidate_contract_versions
    )
    transition_policy = (
        execution_plan.load_architecture_policy(repo_root) if policy is None else policy
    )
    try:
        candidate = execution_plan.validate_contract_transition(
            plan,
            before,
            transition_policy,
            candidate,
            checkpoint_id=checkpoint_id,
        )
        after_versions = execution_plan.validate_contract_versions(
            after.get("verified_contract_versions"), "STATE.verified_contract_versions"
        )
    except execution_plan.PlanError as exc:
        raise SupervisorError(str(exc)) from exc
    if after_versions != candidate:
        raise SupervisorError("verified contract versions do not match the exact candidate source")
    return checkpoint_id


def advance_state_once(
    before: dict[str, Any],
    plan: dict[str, Any],
    checkpoint_id: str,
    *,
    candidate_contract_versions: Mapping[str, Any] | None = None,
    policy: Mapping[str, Any] | None = None,
    repo_root: Path = REPO_ROOT,
    today: str | None = None,
) -> dict[str, Any]:
    """Create the only state transition the supervisor is allowed to write."""

    after = copy.deepcopy(before)
    if before.get("current_next") != checkpoint_id:
        raise SupervisorError(
            f"state current_next is {before.get('current_next')!r}, expected {checkpoint_id!r}"
        )
    checkpoint = execution_plan.checkpoint_for_id(plan, checkpoint_id)
    candidate = (
        execution_plan.read_contract_versions(repo_root)
        if candidate_contract_versions is None
        else candidate_contract_versions
    )
    transition_policy = (
        execution_plan.load_architecture_policy(repo_root) if policy is None else policy
    )
    try:
        candidate = execution_plan.validate_contract_transition(
            plan, before, transition_policy, candidate, checkpoint_id=checkpoint_id
        )
    except execution_plan.PlanError as exc:
        raise SupervisorError(str(exc)) from exc
    checkpoints = after.get("checkpoints")
    if not isinstance(checkpoints, dict):
        raise SupervisorError("state checkpoints are not an object")
    checkpoints[checkpoint_id] = "DONE"
    successor = checkpoint["next_checkpoint_relation"]
    if successor is None:
        after["current_next"] = None
    else:
        if checkpoints.get(successor) != "PLANNED":
            raise SupervisorError(f"successor {successor} is not PLANNED")
        checkpoints[successor] = "NEXT"
        after["current_next"] = successor
    after["last_updated"] = today or _datetime.date.today().isoformat()
    after["phase_status"] = execution_plan.derive_phase_statuses(
        plan, checkpoints, repo_root
    )
    after["verified_contract_versions"] = dict(candidate)
    assert_state_advanced_once(
        before,
        after,
        plan,
        candidate_contract_versions=candidate,
        policy=transition_policy,
        repo_root=repo_root,
    )
    return after


def validate_state_commit_paths(paths: Sequence[str], checkpoint_id: str) -> None:
    """Allow only STATE.json and this checkpoint's evidence record."""

    expected = {
        "docs/execution/STATE.json",
        f"docs/execution/evidence/{checkpoint_id}.json",
    }
    actual = {path.replace("\\", "/") for path in paths}
    if actual != expected:
        raise SupervisorError(
            "state completion commit must contain exactly STATE.json and one evidence file; "
            f"found: {', '.join(sorted(actual)) or 'none'}"
        )


def validate_resume_preconditions(
    *,
    resume_sha: str,
    head: str,
    origin_main: str,
    current_next: str | None,
    expected_checkpoint: str,
    worktree_clean: bool,
    baseline_is_ancestor: bool,
) -> None:
    if execution_evidence.SHA_PATTERN.fullmatch(resume_sha) is None:
        raise SupervisorError("--resume-sha must be a lowercase 40-character SHA")
    if not worktree_clean:
        raise SupervisorError("resume requires a clean worktree")
    if head != origin_main or resume_sha != head:
        raise SupervisorError("resume requires --resume-sha == HEAD == origin/main")
    if current_next != expected_checkpoint:
        raise SupervisorError("resume requires the same checkpoint to remain NEXT")
    if not baseline_is_ancestor:
        raise SupervisorError("resume SHA is not a descendant of the prior state baseline")


def checkpoint_prompt(repo_root: Path, resolution: dict[str, Any]) -> str:
    allowed_paths = resolution.get("runner_allowed_protected_paths", [])
    if allowed_paths:
        protection = (
            "Do not edit PLAN.json, STATE.json, EVIDENCE_POLICY.json, architecture invariants "
            "or policy, phase specs, execution supervisor/validator/evidence files, or completion "
            "evidence. This checkpoint authorizes changes to exactly this protected workflow path:\n"
            + "".join(f"  - {path}\n" for path in allowed_paths)
            + "No other protected execution-control surface may change; PLAN.json and STATE.json "
            "remain immutable.\n"
        )
    else:
        protection = (
            "Do not edit PLAN.json, STATE.json, EVIDENCE_POLICY.json, architecture invariants "
            "or policy, phase specs, execution supervisor/validator/evidence files, completion "
            "evidence, or protected workflow gates.\n"
        )
    return (
        "You are executing exactly one locked Opencut Reinforced checkpoint.\n"
        f"Repository root: {repo_root}\n"
        f"Goal: {resolution['goal']}\n"
        f"Checkpoint: {resolution['checkpoint_id']} — {resolution['title']}\n"
        f"Phase: {resolution['phase']}\n"
        f"Locked specification: {resolution['spec_document']}\n"
        f"Next relation: {resolution['next_checkpoint_relation'] or 'none'}\n\n"
        "Implement exactly this checkpoint and no successor checkpoint.\n"
        "You are not rewarded for the smallest implementation that makes existing tests green.\n"
        "Prove the primary real user journey through the actual product boundary. "
        "Unit and bridge tests remain useful lower-level evidence, but cannot replace product acceptance.\n"
        "Do not replace a real UI journey with bridge-only calls when UI is in scope, "
        "packaged dependencies with system-installed tools, persistent reopen with "
        "in-process-only tests, platform permissions with disabled enforcement, or a "
        "failed acceptance test with a synthetic equivalent.\n"
        "After two speculative fixes to one failing gate, stop. Another repair needs exact "
        "failure evidence, a falsifiable hypothesis, a discriminating reproduction, and a "
        "causal explanation. No generic retry or timeout tuning.\n"
        f"Required evidence classes: {', '.join(resolution.get('required_evidence_classes', []))}.\n"
        + protection
        + "Push implementation commits only; do not create a state/evidence "
        "completion commit. Do not claim DONE. The supervisor owns hosted verification and "
        "state advancement.\n\n"
        "Your final handoff must begin with:\n"
        "IMPLEMENTED — AWAITING SUPERVISOR EVIDENCE\n\n"
        "and include these sections: CHECKPOINT, BASELINE, IMPLEMENTATION, TESTS, LOCAL "
        "VERIFICATION, IMPLEMENTATION COMMIT, HOSTED EVIDENCE (AWAITING SUPERVISOR), STATE "
        "(checkpoint remains NEXT and successor remains PLANNED), and BLOCKERS. Report native "
        "Flutter runtime as NOT RUN — LOCAL NATIVE EXECUTION DISALLOWED BY POLICY. Stop on a "
        "conflict instead of repairing the plan creatively.\n"
    )


def _preflight_goal_data(repo_root: Path, goal: str) -> dict[str, Any]:
    status = git_output(repo_root, "status", "--porcelain", "--untracked-files=all")
    if status:
        raise SupervisorError("preflight requires a clean worktree")
    branch = git_output(repo_root, "branch", "--show-current")
    if branch != "main":
        raise SupervisorError(f"preflight requires branch main, found {branch or 'detached'}")
    head = git_output(repo_root, "rev-parse", "HEAD")
    origin = git_output(repo_root, "rev-parse", "origin/main")
    if head != origin:
        raise SupervisorError("preflight requires HEAD == origin/main")
    counts = git_output(repo_root, "rev-list", "--left-right", "--count", "HEAD...origin/main")
    if counts.replace("\t", " ").split() != ["0", "0"]:
        raise SupervisorError(f"preflight requires ahead/behind 0/0, found {counts}")
    plan, state = execution_plan.load_plan_state(repo_root)
    if state.get("current_next") == "9B" and plan.get("quality_contract_version") == 2:
        if not (repo_root / AMENDMENT_MARKER).is_file():
            raise SupervisorError("9B quality contract requires its amendment baseline marker")
        _validate_amendment_baseline(
            repo_root,
            _control_baseline(repo_root, "HEAD"),
        )
    summary = execution_plan.validate_plan(plan, state, repo_root)
    policy = execution_plan.load_architecture_policy(repo_root)
    candidate = execution_plan.read_contract_versions(repo_root)
    execution_plan.validate_contract_transition(plan, state, policy, candidate)
    resolution = execution_plan.resolve_goal(plan, state, goal, repo_root)
    evidence_policy = execution_evidence.load_policy(
        repo_root / "docs/execution/EVIDENCE_POLICY.json"
    )
    execution_evidence.required_class_sources(
        execution_plan.checkpoint_for_id(plan, resolution["checkpoint_id"]), evidence_policy
    )
    remaining = [
        checkpoint_id
        for checkpoint_id in resolution["goal_checkpoint_ids"]
        if state["checkpoints"][checkpoint_id] != "DONE"
    ]
    for checkpoint_id in remaining:
        checkpoint = execution_plan.checkpoint_for_id(plan, checkpoint_id)
        if not checkpoint["expected_project_schema_effect_category"]:
            raise execution_plan.PlanError(
                f"checkpoint {checkpoint_id} has no project-schema effect category"
            )
        if not checkpoint["expected_ipc_effect_category"]:
            raise execution_plan.PlanError(
                f"checkpoint {checkpoint_id} has no IPC effect category"
            )
    verified = execution_plan.validate_contract_versions(
        state["verified_contract_versions"], "STATE.verified_contract_versions"
    )
    report = "\n".join(
        (
            "PRECHECK PASS",
            f"goal: {goal}",
            f"current NEXT: {summary['next_checkpoint']}",
            "verified versions: "
            f"project={verified['project_schema']} recovery={verified['recovery_schema']} "
            f"ipc={verified['ipc_protocol']}",
            "candidate versions: "
            f"project={candidate['project_schema']} recovery={candidate['recovery_schema']} "
            f"ipc={candidate['ipc_protocol']}",
            "current transition: allowed",
            f"remaining checkpoints checked: {len(remaining)}",
            "model calls: 0",
        )
    )
    return {
        "plan": plan,
        "state": state,
        "policy": policy,
        "candidate_versions": candidate,
        "resolution": resolution,
        "remaining_checkpoint_ids": remaining,
        "report": report,
        "head": head,
    }


def preflight_goal(repo_root: Path, goal: str) -> str:
    """Read-only preflight; it does not fetch, poll GitHub, write, or invoke a runner."""

    try:
        return _preflight_goal_data(repo_root, goal)["report"]
    except execution_plan.PlanError as exc:
        raise SupervisorError(str(exc)) from exc


def _validate_candidate_transition(
    plan: dict[str, Any],
    state: Mapping[str, Any],
    policy: Mapping[str, Any],
    repo_root: Path,
    revision: str | None = None,
) -> execution_plan.ContractVersions:
    try:
        candidate = execution_plan.read_contract_versions(repo_root, revision)
        return execution_plan.validate_contract_transition(plan, state, policy, candidate)
    except execution_plan.PlanError as exc:
        raise SupervisorError(str(exc)) from exc


def prepare_goal(repo_root: Path, goal: str) -> str:
    ensure_start_state(repo_root)
    plan, state = execution_plan.load_plan_state(repo_root)
    if state.get("current_next") == "9B" and plan.get("quality_contract_version") == 2:
        if not (repo_root / AMENDMENT_MARKER).is_file():
            raise SupervisorError("9B quality contract requires its amendment baseline marker")
        _validate_amendment_baseline(
            repo_root,
            _control_baseline(repo_root, "HEAD"),
        )
    execution_plan.validate_plan(plan, state, repo_root)
    resolution = execution_plan.resolve_goal(plan, state, goal, repo_root)
    evidence_policy = execution_evidence.load_policy(
        repo_root / "docs/execution/EVIDENCE_POLICY.json"
    )
    execution_evidence.required_class_sources(
        execution_plan.checkpoint_for_id(plan, resolution["checkpoint_id"]), evidence_policy
    )
    return checkpoint_prompt(repo_root, resolution)


def _write_json(path: Path, value: Mapping[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def _run_local_completion_checks(repo_root: Path) -> None:
    commands = [
        ["python3", "scripts/check_execution_plan.py"],
        ["python3", "scripts/check_architecture_policy.py"],
        ["bash", "scripts/check-repo.sh"],
        ["git", "diff", "--check"],
    ]
    for command in commands:
        subprocess.run(command, cwd=repo_root, check=True)


def _run_pre_host_checks(repo_root: Path, implementation_sha: str) -> None:
    """Run headless Rust tests before spending time on hosted CI verification."""

    try:
        subprocess.run(["cargo", "test", "--workspace"], cwd=repo_root, check=True)
    except subprocess.CalledProcessError as exc:
        raise SupervisorError("pre-host verification failed: cargo test --workspace") from exc
    refuse_dirty_worktree(repo_root)
    if git_output(repo_root, "rev-parse", "HEAD") != implementation_sha:
        raise SupervisorError("HEAD changed during pre-host verification")
    if git_output(repo_root, "rev-parse", "origin/main") != implementation_sha:
        raise SupervisorError("origin/main changed during pre-host verification")


def _github_api_for_repo(repo_root: Path, policy: Mapping[str, Any]) -> execution_evidence.GitHubApi:
    owner, repository = execution_evidence.repository_identity(repo_root)
    expected = policy["repository"]
    if owner != expected["owner"] or repository != expected["name"]:
        raise SupervisorError("origin repository does not match EVIDENCE_POLICY.json")
    token = execution_evidence.select_token()
    return execution_evidence.GitHubApi(
        owner,
        repository,
        token=token,
        api_version=policy["github_api"]["api_version"],
    )


def verify_hosted_checkpoint(
    repo_root: Path,
    plan: dict[str, Any],
    checkpoint: Mapping[str, Any],
    implementation_sha: str,
    implementation_subject: str,
    *,
    implementation_origin_sha: str | None = None,
    api: execution_evidence.GitHubApi | None = None,
    clock: Callable[[], float] = time.monotonic,
    sleep: Callable[[float], None] = time.sleep,
    control_plane_receipt: Mapping[str, Any] | None = None,
) -> dict[str, Any]:
    policy = execution_evidence.load_policy(repo_root / "docs" / "execution" / "EVIDENCE_POLICY.json")
    github_api = api or _github_api_for_repo(repo_root, policy)
    gates = execution_evidence.wait_for_required_workflows(
        github_api,
        policy,
        implementation_sha,
        clock=clock,
        sleep=sleep,
    )
    if checkpoint.get("developer_preview_required") is True:
        preview = execution_evidence.wait_for_developer_preview(
            github_api,
            policy,
            implementation_sha,
            clock=clock,
            sleep=sleep,
        )
    else:
        preview = {"required": False}
    class_proofs = (
        execution_evidence.collect_evidence_class_proofs(
            github_api, policy, checkpoint, gates
        )
        if checkpoint.get("evidence_contract_version") == 2 else None
    )
    record = execution_evidence.build_evidence_record(
        checkpoint_id=str(checkpoint["id"]),
        implementation_sha=implementation_sha,
        implementation_subject=implementation_subject,
        gates=gates,
        developer_preview=preview,
        contract_versions=execution_plan.read_contract_versions(
            repo_root, implementation_sha
        ),
        implementation_origin_sha=implementation_origin_sha,
        evidence_classes=class_proofs,
    )
    if control_plane_receipt is not None:
        if not isinstance(control_plane_receipt, dict):
            raise SupervisorError("control plane receipt must be an object")
        record = dict(record, control_plane_receipt=dict(control_plane_receipt))
    execution_evidence.validate_evidence_record(
        record,
        checkpoint_id=str(checkpoint["id"]),
        checkpoint=checkpoint,
        policy=policy,
    )
    return {"policy": policy, "record": record, "api": github_api}


def _state_commit_subject(policy: Mapping[str, Any], checkpoint_id: str) -> str:
    template = policy["state_transition"]["commit_subject_template"]
    return template.replace("{checkpoint_id}", checkpoint_id)


def finalize_verified_checkpoint(
    repo_root: Path,
    *,
    plan: dict[str, Any],
    state: dict[str, Any],
    checkpoint: Mapping[str, Any],
    evidence_result: Mapping[str, Any],
    implementation_sha: str,
    api: execution_evidence.GitHubApi,
    run_local_checks: bool = True,
) -> dict[str, Any]:
    checkpoint_id = str(checkpoint["id"])
    policy = evidence_result["policy"]
    record = evidence_result["record"]
    if record.get("implementation_sha") != implementation_sha:
        raise SupervisorError("evidence implementation SHA does not match the verified implementation SHA")
    if git_output(repo_root, "rev-parse", "HEAD") != implementation_sha:
        raise SupervisorError("verified implementation SHA is not current HEAD")
    if "contract_versions" not in record:
        raise SupervisorError("new evidence record is missing contract_versions")
    try:
        candidate_versions = execution_plan.validate_contract_versions(
            record["contract_versions"], "evidence contract_versions"
        )
        exact_versions = execution_plan.read_contract_versions(repo_root, implementation_sha)
        if candidate_versions != exact_versions:
            raise SupervisorError("evidence contract versions do not match the verified tree")
        architecture_policy = execution_plan.load_architecture_policy(repo_root)
        execution_plan.validate_contract_transition(
            plan, state, architecture_policy, exact_versions, checkpoint_id=checkpoint_id
        )
    except execution_plan.PlanError as exc:
        raise SupervisorError(str(exc)) from exc
    execution_evidence.validate_evidence_record(
        record,
        checkpoint_id=checkpoint_id,
        checkpoint=checkpoint,
        policy=policy,
    )
    state_path = repo_root / "docs" / "execution" / "STATE.json"
    evidence_path = repo_root / "docs" / "execution" / "evidence" / f"{checkpoint_id}.json"
    original_state = state_path.read_bytes()
    if evidence_path.exists():
        raise SupervisorError(f"completion evidence already exists: {evidence_path}")
    after_state = advance_state_once(
        state,
        plan,
        checkpoint_id,
        candidate_contract_versions=exact_versions,
        policy=architecture_policy,
        repo_root=repo_root,
    )
    _write_json(evidence_path, record)
    _write_json(state_path, after_state)
    commit_created = False
    try:
        if run_local_checks:
            _run_local_completion_checks(repo_root)
        git_output(repo_root, "fetch", "--prune", "origin")
        if git_output(repo_root, "rev-parse", "origin/main") != implementation_sha:
            raise SupervisorError("origin/main moved after implementation verification")
        changed_paths = git_status_paths(repo_root)
        validate_state_commit_paths(changed_paths, checkpoint_id)
        subprocess.run(["git", "add", str(state_path), str(evidence_path)], cwd=repo_root, check=True)
        staged = git_output(repo_root, "diff", "--cached", "--name-only").splitlines()
        validate_state_commit_paths(staged, checkpoint_id)
        subject = _state_commit_subject(policy, checkpoint_id)
        subprocess.run(["git", "commit", "-m", subject], cwd=repo_root, check=True)
        commit_created = True
        subprocess.run(["git", "push", "origin", "main"], cwd=repo_root, check=True)
    except Exception:
        if not commit_created:
            subprocess.run(["git", "reset", "--", str(state_path), str(evidence_path)], cwd=repo_root, check=False)
            state_path.write_bytes(original_state)
            if evidence_path.exists():
                evidence_path.unlink()
        raise

    state_commit_sha = git_output(repo_root, "rev-parse", "HEAD")
    hygiene = execution_evidence.wait_for_workflow_gate(
        api,
        policy,
        "repository_hygiene",
        state_commit_sha,
    )
    return {
        "checkpoint": dict(checkpoint),
        "implementation_sha": implementation_sha,
        "implementation_subject": record["implementation_subject"],
        "record": record,
        "after_state": after_state,
        "state_commit_sha": state_commit_sha,
        "state_commit_subject": subject,
        "state_commit_hygiene": hygiene,
    }


def _resume_baseline(
    repo_root: Path, resume_sha: str, allowed_paths: Sequence[str] = ()
) -> str:
    # Exclude the resume commit when locating the prior state baseline so a
    # forbidden STATE.json edit in that implementation commit remains visible
    # in the protected-path diff below.
    baseline = _control_baseline(repo_root, f"{resume_sha}^")
    _validate_amendment_baseline(repo_root, baseline)
    try:
        subprocess.run(
            ["git", "merge-base", "--is-ancestor", baseline, resume_sha],
            cwd=repo_root,
            check=True,
            capture_output=True,
            text=True,
        )
    except subprocess.CalledProcessError as exc:
        raise SupervisorError("resume SHA is not a descendant of the prior state baseline") from exc
    changed = git_output(repo_root, "diff", "--name-only", f"{baseline}..{resume_sha}").splitlines()
    allowed = set(allowed_paths)
    protected = [
        path
        for path in changed
        if is_protected_execution_path(path) and path not in allowed
    ]
    if protected:
        raise SupervisorError(
            "resume implementation changed protected execution-control files: "
            + ", ".join(protected)
        )
    return baseline


def _control_baseline(repo_root: Path, revision: str) -> str:
    baseline = git_output(repo_root, "log", "-1", "--format=%H", revision, "--", "docs/execution/STATE.json")
    marker = git_output(repo_root, "log", "-1", "--format=%H", revision, "--", AMENDMENT_MARKER)
    if marker and git_output(repo_root, "merge-base", baseline, marker) == baseline:
        return marker
    return baseline


def _validate_amendment_baseline(repo_root: Path, baseline: str) -> None:
    """Trust this in-flight 9B amendment only if its commit changed control files."""

    if not git_output(repo_root, "ls-tree", "--name-only", baseline, "--", AMENDMENT_MARKER):
        return
    new_state = _json_at_revision(repo_root, baseline, "docs/execution/STATE.json")
    if new_state.get("current_next") != "9B":
        return  # Later supervisor completions retain the provenance marker.
    try:
        parent = git_output(repo_root, "rev-parse", f"{baseline}^1")
    except subprocess.CalledProcessError as exc:
        raise SupervisorError("amendment baseline must have a prior implementation") from exc
    introduced = git_output(
        repo_root, "diff", "--name-only", f"{parent}..{baseline}", "--", AMENDMENT_MARKER
    )
    if introduced != AMENDMENT_MARKER:
        raise SupervisorError("active 9B baseline must introduce the amendment marker")
    marker = _json_at_revision(repo_root, baseline, AMENDMENT_MARKER)
    if set(marker) != {"schema_version", "checkpoint_id", "prior_implementation_sha", "prior_failed_run_id", "changed_paths"}:
        raise SupervisorError("amendment baseline marker has invalid fields")
    if marker["schema_version"] != 1 or marker["checkpoint_id"] != "9B" or marker["prior_implementation_sha"] != parent:
        raise SupervisorError("amendment baseline has the wrong prior implementation")
    if marker["prior_failed_run_id"] != 37043830370:
        raise SupervisorError("amendment baseline has the wrong failed-run provenance")
    changed = set(git_output(repo_root, "diff", "--name-only", f"{parent}..{baseline}").splitlines())
    if marker["changed_paths"] != sorted(changed) or not changed <= AMENDMENT_CONTROL_PATHS:
        raise SupervisorError("amendment baseline contains non-control-plane changes")
    old_state = _json_at_revision(repo_root, parent, "docs/execution/STATE.json")
    if old_state.get("current_next") != "9B" or new_state.get("current_next") != "9B":
        raise SupervisorError("amendment baseline changed the active checkpoint")
    old_statuses = old_state.get("checkpoints", {})
    new_statuses = new_state.get("checkpoints", {})
    if not isinstance(old_statuses, dict) or not isinstance(new_statuses, dict):
        raise SupervisorError("amendment baseline has invalid checkpoint state")
    if new_statuses != {**old_statuses, "9B1": "PLANNED"} or old_statuses.get("9B") != "NEXT" or old_statuses.get("9C") != "PLANNED":
        raise SupervisorError("amendment baseline did not preserve 9B NEXT and 9C PLANNED")
    retained_fields = set(old_state) | set(new_state)
    retained_fields -= {"checkpoints", "last_updated"}
    if any(old_state.get(field) != new_state.get(field) for field in retained_fields):
        raise SupervisorError("amendment baseline changed retained execution state")
    old_plan = _json_at_revision(repo_root, parent, "docs/execution/PLAN.json")
    new_plan = _json_at_revision(repo_root, baseline, "docs/execution/PLAN.json")
    if git_output(repo_root, "ls-tree", "--name-only", parent, "--", AMENDMENT_MARKER):
        _validate_amendment_baseline(repo_root, parent)
        if old_plan != new_plan:
            raise SupervisorError("follow-up amendment changed the locked checkpoint graph")
    elif execution_plan.checkpoint_for_id(old_plan, "9B")["next_checkpoint_relation"] != "9C":
        raise SupervisorError("amendment baseline has an invalid successor insertion")
    if execution_plan.checkpoint_for_id(new_plan, "9B")["next_checkpoint_relation"] != "9B1":
        raise SupervisorError("amendment baseline has an invalid successor insertion")
    if execution_plan.checkpoint_for_id(new_plan, "9B1")["next_checkpoint_relation"] != "9C" or execution_plan.checkpoint_for_id(new_plan, "9C")["prerequisite_checkpoint_ids"] != ["9B1"]:
        raise SupervisorError("amendment baseline has an invalid 9B1 graph")
    checkpoint = execution_plan.checkpoint_for_id(new_plan, "9B")
    if new_plan.get("quality_contract_version") != 2 or checkpoint.get("evidence_contract_version") != 2:
        raise SupervisorError("amendment baseline lacks the new quality contract")
    for key in ("expected_project_schema_effect_category", "expected_ipc_effect_category"):
        if checkpoint[key] != execution_plan.checkpoint_for_id(old_plan, "9B")[key]:
            raise SupervisorError("amendment baseline changed the 9B model contract")
    evidence_path = repo_root / "docs/execution/evidence/9B.json"
    if evidence_path.exists() or "docs/execution/evidence/9B.json" in changed:
        raise SupervisorError("amendment baseline created 9B completion evidence")


def _git_file_bytes(repo_root: Path, revision: str, relative_path: str) -> bytes:
    result = subprocess.run(
        ["git", "show", f"{revision}:{relative_path}"],
        cwd=repo_root,
        capture_output=True,
        check=True,
    )
    return result.stdout


def _json_at_revision(repo_root: Path, revision: str, relative_path: str) -> dict[str, Any]:
    try:
        value = json.loads(_git_file_bytes(repo_root, revision, relative_path))
    except (json.JSONDecodeError, subprocess.CalledProcessError) as exc:
        raise SupervisorError(f"cannot read {relative_path} at {revision}") from exc
    if not isinstance(value, dict):
        raise SupervisorError(f"{relative_path} at {revision} must be a JSON object")
    return value


def _state_leaves_checkpoint_next(
    repo_root: Path, state: Mapping[str, Any], checkpoint_id: str, revision: str
) -> bool:
    statuses = state.get("checkpoints")
    phase_status = state.get("phase_status")
    if (
        state.get("schema_version") != 1
        or not isinstance(statuses, dict)
        or not statuses
        or any(
            not isinstance(key, str)
            or not isinstance(value, str)
            or value not in execution_plan.VALID_CHECKPOINT_STATUSES
            for key, value in statuses.items()
        )
        or not isinstance(phase_status, dict)
        or any(
            not isinstance(key, str)
            or not isinstance(value, str)
            or value not in execution_plan.VALID_PHASE_STATUSES
            for key, value in phase_status.items()
        )
    ):
        raise SupervisorError(f"invalid STATE.json at {revision}")
    next_checkpoints = [
        key for key, value in statuses.items() if value == "NEXT"
    ]
    if len(next_checkpoints) > 1 or state.get("current_next") != (
        next_checkpoints[0] if next_checkpoints else None
    ):
        raise SupervisorError(f"invalid STATE.json at {revision}")
    if not next_checkpoints and any(value != "DONE" for value in statuses.values()):
        raise SupervisorError(f"invalid STATE.json at {revision}")
    plan = _json_at_revision(repo_root, revision, "docs/execution/PLAN.json")
    try:
        checkpoints = execution_plan._checkpoint_map(plan, repo_root)
        if set(statuses) != set(checkpoints):
            raise execution_plan.PlanError("checkpoint statuses do not match PLAN.json")
        expected_phases = execution_plan.derive_phase_statuses(plan, statuses, repo_root)
        if phase_status != expected_phases:
            raise execution_plan.PlanError("phase statuses do not match checkpoint statuses")
        for checkpoint_key, checkpoint in checkpoints.items():
            prerequisites = checkpoint["prerequisite_checkpoint_ids"]
            if any(prerequisite not in checkpoints for prerequisite in prerequisites):
                raise execution_plan.PlanError(
                    f"checkpoint {checkpoint_key} has an unknown prerequisite"
                )
            if statuses[checkpoint_key] == "DONE" or checkpoint_key == state.get("current_next"):
                if any(statuses[prerequisite] != "DONE" for prerequisite in prerequisites):
                    raise execution_plan.PlanError(
                        f"checkpoint {checkpoint_key} has an unfinished prerequisite"
                    )
    except (KeyError, execution_plan.PlanError) as exc:
        raise SupervisorError(f"invalid STATE.json at {revision}: {exc}") from exc
    if "verified_contract_versions" in state:
        try:
            execution_plan.validate_contract_versions(
                state["verified_contract_versions"],
                f"STATE.verified_contract_versions at {revision}",
            )
        except execution_plan.PlanError as exc:
            raise SupervisorError(str(exc)) from exc
    return (
        state.get("current_next") == checkpoint_id
        and statuses.get(checkpoint_id) == "NEXT"
    )


def _state_baseline_for_checkpoint(
    repo_root: Path, failed_sha: str, checkpoint_id: str
) -> str:
    try:
        first_parent = git_output(repo_root, "rev-parse", "--verify", f"{failed_sha}^1")
        history = git_output(repo_root, "rev-list", "--first-parent", first_parent).splitlines()
    except subprocess.CalledProcessError as exc:
        raise SupervisorError("FAILED_SHA has no first-parent state baseline") from exc

    failed_fingerprint = _active_checkpoint_fingerprint(repo_root, failed_sha, checkpoint_id)
    baseline: str | None = None
    for revision in history:
        state_path = "docs/execution/STATE.json"
        if not git_output(repo_root, "ls-tree", revision, "--", state_path):
            if baseline is not None:
                return baseline
            raise SupervisorError(f"first parent has no {state_path}")
        state = _json_at_revision(repo_root, revision, state_path)
        if not _state_leaves_checkpoint_next(
            repo_root, state, checkpoint_id, revision
        ):
            if baseline is not None:
                return baseline
            raise SupervisorError(
                f"first parent does not leave checkpoint {checkpoint_id} NEXT"
            )
        if _active_checkpoint_fingerprint(repo_root, revision, checkpoint_id) != failed_fingerprint:
            if baseline is not None:
                return baseline
            raise SupervisorError(
                "first parent active checkpoint contract fingerprint does not match FAILED_SHA"
            )
        baseline = revision

    if baseline is not None:
        return baseline
    raise SupervisorError(f"could not find a state baseline for NEXT checkpoint {checkpoint_id}")


def _active_checkpoint_fingerprint(
    repo_root: Path, revision: str, checkpoint_id: str
) -> dict[str, Any]:
    plan = _json_at_revision(repo_root, revision, "docs/execution/PLAN.json")
    checkpoint = execution_plan.checkpoint_for_id(plan, checkpoint_id)
    policy_bytes = _git_file_bytes(
        repo_root, revision, "docs/execution/EVIDENCE_POLICY.json"
    )
    try:
        evidence_policy = json.loads(policy_bytes)
    except (json.JSONDecodeError, UnicodeDecodeError) as exc:
        raise SupervisorError(f"invalid EVIDENCE_POLICY.json at {revision}") from exc
    if not isinstance(evidence_policy, dict):
        raise SupervisorError(f"EVIDENCE_POLICY.json at {revision} must be an object")
    gates = evidence_policy.get("required_gates", {})
    preview = evidence_policy.get("developer_preview", {})
    if (
        not isinstance(gates, dict)
        or any(not isinstance(gate, dict) for gate in gates.values())
        or not isinstance(preview, dict)
    ):
        raise SupervisorError(f"EVIDENCE_POLICY.json at {revision} has invalid workflow identities")
    identities = {
        "required_gates": {
            gate_id: {
                "workflow_file": gate.get("workflow_file"),
                "workflow_name": gate.get("workflow_name"),
                "required_jobs": gate.get("required_jobs"),
            }
            for gate_id, gate in sorted(gates.items())
        },
        "developer_preview": {
            key: preview.get(key)
            for key in ("workflow_file", "workflow_name", "publish_job")
        },
    }
    spec_path = str(checkpoint["spec_document"])
    return {
        "checkpoint": checkpoint,
        "phase_spec_sha256": hashlib.sha256(
            _git_file_bytes(repo_root, revision, spec_path)
        ).hexdigest(),
        "architecture_invariants_sha256": hashlib.sha256(
            _git_file_bytes(
                repo_root, revision, "docs/execution/ARCHITECTURE_INVARIANTS.md"
            )
        ).hexdigest(),
        "evidence_policy_sha256": hashlib.sha256(policy_bytes).hexdigest(),
        "workflow_identities": identities,
    }


def _repair_history_paths(repo_root: Path, failed_sha: str, head: str) -> set[str]:
    commits = git_output(
        repo_root,
        "rev-list",
        "--first-parent",
        "--reverse",
        f"{failed_sha}..{head}",
    ).splitlines()
    changed: set[str] = set()
    for revision in commits:
        paths = git_output(
            repo_root,
            "diff-tree",
            "-m",
            "--no-commit-id",
            "--name-only",
            "-r",
            "--no-renames",
            revision,
        )
        changed.update(path for path in paths.splitlines() if path)
    return changed


def _repair_path_allowed(path: str) -> bool:
    if "\\" in path:
        return False
    parts = path.split("/")
    if any(part in {"", ".", ".."} for part in parts):
        return False
    crate_test = (
        len(parts) >= 4
        and parts[0] == "crates"
        and bool(parts[1])
        and parts[2] == "tests"
    )
    return (
        path in REPAIR_ALLOWED_CONTROL_PATHS
        or any(path.startswith(prefix) for prefix in REPAIR_ALLOWED_PREFIXES)
        or crate_test
        or any(path.startswith(prefix) for prefix in REPAIR_TEST_PREFIXES)
    )


def _state_at_repair_revision(
    repo_root: Path,
    revision: str,
    failed_state: Mapping[str, Any],
    baseline_versions: Mapping[str, int],
) -> None:
    current = _json_at_revision(repo_root, revision, "docs/execution/STATE.json")
    current_without_versions = {
        key: value for key, value in current.items() if key != "verified_contract_versions"
    }
    failed_without_versions = {
        key: value for key, value in failed_state.items() if key != "verified_contract_versions"
    }
    if current_without_versions != failed_without_versions:
        raise SupervisorError("repair history changed checkpoint or unrelated STATE data")
    if "verified_contract_versions" in current:
        try:
            versions = execution_plan.validate_contract_versions(
                current["verified_contract_versions"],
                "repair STATE.verified_contract_versions",
            )
        except execution_plan.PlanError as exc:
            raise SupervisorError(str(exc)) from exc
        if versions != baseline_versions:
            raise SupervisorError("repair history changed verified versions before completion")
    evidence_path = f"docs/execution/evidence/{failed_state['current_next']}.json"
    exists = subprocess.run(
        ["git", "cat-file", "-e", f"{revision}:{evidence_path}"],
        cwd=repo_root,
        capture_output=True,
    ).returncode == 0
    if exists:
        raise SupervisorError("repair history contains completion evidence for the active checkpoint")


def validate_repair_resume_history(
    repo_root: Path, failed_sha: str, checkpoint_id: str
) -> dict[str, Any]:
    """Prove original checkpoint integrity and trusted control-plane-only repair history."""

    if execution_evidence.SHA_PATTERN.fullmatch(failed_sha) is None:
        raise SupervisorError("--repair-resume-from must be a lowercase 40-character SHA")
    head = git_output(repo_root, "rev-parse", "HEAD")
    try:
        subprocess.run(
            ["git", "merge-base", "--is-ancestor", failed_sha, head],
            cwd=repo_root,
            check=True,
            capture_output=True,
        )
    except subprocess.CalledProcessError as exc:
        raise SupervisorError("FAILED_SHA is not an ancestor of current HEAD") from exc

    baseline = _state_baseline_for_checkpoint(repo_root, failed_sha, checkpoint_id)
    baseline_state = _json_at_revision(repo_root, baseline, "docs/execution/STATE.json")
    baseline_plan = _json_at_revision(repo_root, baseline, "docs/execution/PLAN.json")
    active = execution_plan.checkpoint_for_id(baseline_plan, checkpoint_id)
    if (
        baseline_state.get("current_next") != checkpoint_id
        or baseline_state.get("checkpoints", {}).get(checkpoint_id) != "NEXT"
    ):
        raise SupervisorError("state baseline did not introduce the active checkpoint as NEXT")
    allowed_paths = active.get("runner_allowed_protected_paths", [])
    if not isinstance(allowed_paths, list) or any(not isinstance(path, str) for path in allowed_paths):
        raise SupervisorError("state baseline has an invalid workflow allowance")
    changed_by_implementation = git_output(
        repo_root, "diff", "--name-only", f"{baseline}..{failed_sha}"
    ).splitlines()
    protected = [
        path
        for path in changed_by_implementation
        if is_protected_execution_path(path) and path not in set(allowed_paths)
    ]
    if protected:
        raise SupervisorError(
            "original implementation changed protected execution-control files: "
            + ", ".join(sorted(protected))
        )

    failed_state = _json_at_revision(repo_root, failed_sha, "docs/execution/STATE.json")
    if failed_state != baseline_state:
        raise SupervisorError("original implementation changed STATE.json")
    if (
        failed_state.get("current_next") != checkpoint_id
        or failed_state.get("checkpoints", {}).get(checkpoint_id) != "NEXT"
    ):
        raise SupervisorError("FAILED_SHA does not leave the active checkpoint NEXT")

    baseline_versions = execution_plan.read_contract_versions(repo_root, baseline)
    if "verified_contract_versions" in baseline_state:
        try:
            recorded = execution_plan.validate_contract_versions(
                baseline_state["verified_contract_versions"],
                "baseline STATE.verified_contract_versions",
            )
        except execution_plan.PlanError as exc:
            raise SupervisorError(str(exc)) from exc
        if recorded != baseline_versions:
            raise SupervisorError("state baseline verified versions do not match its source tree")
    else:
        baseline_state["verified_contract_versions"] = dict(baseline_versions)

    current_plan = _json_at_revision(repo_root, head, "docs/execution/PLAN.json")
    current_state = _json_at_revision(repo_root, head, "docs/execution/STATE.json")
    current_policy = _json_at_revision(
        repo_root, head, "docs/execution/architecture-policy.json"
    )
    failed_plan = _json_at_revision(repo_root, failed_sha, "docs/execution/PLAN.json")
    if _active_checkpoint_fingerprint(repo_root, failed_sha, checkpoint_id) != (
        _active_checkpoint_fingerprint(repo_root, head, checkpoint_id)
    ):
        raise SupervisorError("maintenance changed the active checkpoint contract fingerprint")
    if (
        current_state.get("current_next") != checkpoint_id
        or current_state.get("checkpoints", {}).get(checkpoint_id) != "NEXT"
    ):
        raise SupervisorError("current state no longer leaves the active checkpoint NEXT")
    if (repo_root / "docs" / "execution" / "evidence" / f"{checkpoint_id}.json").exists():
        raise SupervisorError("completion evidence already exists for the active checkpoint")
    changed_by_maintenance = _repair_history_paths(repo_root, failed_sha, head)
    rejected = sorted(path for path in changed_by_maintenance if not _repair_path_allowed(path))
    if rejected:
        raise SupervisorError(
            "repair maintenance changed non-control-plane files: " + ", ".join(rejected)
        )
    try:
        failed_candidate = execution_plan.read_contract_versions(repo_root, failed_sha)
        execution_plan.validate_contract_transition(
            failed_plan,
            baseline_state,
            current_policy,
            failed_candidate,
            checkpoint_id=checkpoint_id,
        )
        current_candidate = execution_plan.read_contract_versions(repo_root, head)
        execution_plan.validate_contract_transition(
            current_plan, current_state, current_policy, current_candidate
        )
    except execution_plan.PlanError as exc:
        raise SupervisorError(str(exc)) from exc
    commits = git_output(
        repo_root,
        "rev-list",
        "--first-parent",
        "--reverse",
        f"{failed_sha}..{head}",
    ).splitlines()
    for revision in commits:
        _state_at_repair_revision(repo_root, revision, failed_state, baseline_versions)
    return {
        "baseline_sha": baseline,
        "failed_sha": failed_sha,
        "head": head,
        "verified_versions": baseline_versions,
        "candidate_versions": current_candidate,
        "fingerprint": _active_checkpoint_fingerprint(repo_root, head, checkpoint_id),
    }


def repair_resume_goal(
    repo_root: Path,
    goal: str,
    failed_sha: str,
    *,
    api: execution_evidence.GitHubApi | None = None,
    clock: Callable[[], float] = time.monotonic,
    sleep: Callable[[float], None] = time.sleep,
) -> list[str]:
    """Verify a trusted repaired HEAD without ever invoking a model runner."""

    ensure_start_state(repo_root)
    preflight = _preflight_goal_data(repo_root, goal)
    checkpoint_id = str(preflight["state"]["current_next"])
    if goal != f"checkpoint:{checkpoint_id}":
        raise SupervisorError("repair-resume requires the exact current checkpoint goal")
    proof = validate_repair_resume_history(repo_root, failed_sha, checkpoint_id)
    if proof["head"] != preflight["head"]:
        raise SupervisorError("current HEAD changed during repair-resume validation")
    plan = preflight["plan"]
    state = preflight["state"]
    resolution = preflight["resolution"]
    result = _run_one_checkpoint(
        repo_root,
        plan=plan,
        state=state,
        resolution=resolution,
        implementation_sha=proof["head"],
        implementation_origin_sha=failed_sha,
        api=api,
        clock=clock,
        sleep=sleep,
    )
    return [_verified_report(result, plan)]


def _verified_report(result: Mapping[str, Any], plan: Mapping[str, Any]) -> str:
    checkpoint = result["checkpoint"]
    record = result["record"]
    gates = {gate["gate_id"]: gate for gate in record["gates"]}
    current_next = result["after_state"].get("current_next")
    next_checkpoint = execution_plan.checkpoint_for_id(plan, current_next) if current_next else None
    lines = [
        "## VERIFIED CHECKPOINT COMPLETION",
        f"checkpoint: {checkpoint['id']}",
        f"title: {checkpoint['title']}",
        "",
        "## IMPLEMENTATION",
        f"implementation SHA: {result['implementation_sha']}",
        f"implementation subject: {result['implementation_subject']}",
        "",
        "## HOSTED CI EVIDENCE",
    ]
    if record.get("implementation_origin_sha"):
        lines.insert(7, f"implementation origin SHA: {record['implementation_origin_sha']}")
    for gate_id in ("repository_hygiene", "platform_verification"):
        gate = gates[gate_id]
        required_jobs = ", ".join(job["name"] for job in gate["jobs"])
        lines.extend(
            [
                f"{gate['workflow_name']}:",
                f"run ID: {gate['run_id']}",
                f"URL: {gate['html_url']}",
                f"head SHA: {gate['head_sha']}",
                f"conclusion: {gate['conclusion']}",
                f"required jobs: {required_jobs}",
                "",
            ]
        )
    preview = record["developer_preview"]
    lines.extend(
        [
            "## DEVELOPER PREVIEW",
            f"required: {preview['required']}",
            f"status: {preview.get('status', 'not required')}",
            f"tag: {preview.get('tag', 'none')}",
            f"source SHA: {preview.get('source_sha', 'none')}",
            f"workflow run: {preview.get('workflow_run', {}).get('id', 'none')}",
            f"release URL: {preview.get('release_url', 'none')}",
            f"assets/checksums: {', '.join(asset['name'] for asset in preview.get('assets', [])) or 'not required'}",
            "",
            "## EVIDENCE",
            f"file: docs/execution/evidence/{checkpoint['id']}.json",
            "",
            "## STATE TRANSITION",
            f"checkpoint: {checkpoint['id']} NEXT → DONE",
            f"successor: {checkpoint['next_checkpoint_relation'] or 'none'} "
            f"{'PLANNED → NEXT' if checkpoint['next_checkpoint_relation'] else 'no successor'}",
            f"current NEXT: {current_next or 'none'}",
            "",
            "## STATE COMMIT",
            f"SHA: {result['state_commit_sha']}",
            f"subject: {result['state_commit_subject']}",
            "",
            "## STATE-COMMIT HYGIENE",
            f"run ID: {result['state_commit_hygiene']['run_id']}",
            f"URL: {result['state_commit_hygiene']['html_url']}",
            f"conclusion: {result['state_commit_hygiene']['conclusion']}",
            "",
            "## GIT",
            f"HEAD: {result['state_commit_sha']}",
            f"origin/main: {result['state_commit_sha']}",
            "ahead/behind: 0/0",
            "worktree: clean",
            "",
            "## NEXT",
            f"next checkpoint: {next_checkpoint['id'] if next_checkpoint else 'none'}",
            f"next phase: {next_checkpoint['phase'] if next_checkpoint else 'none'}",
            f"spec: {next_checkpoint['spec_document'] if next_checkpoint else 'none'}",
        ]
    )
    return "\n".join(lines)


def _run_one_checkpoint(
    repo_root: Path,
    *,
    plan: dict[str, Any],
    state: dict[str, Any],
    resolution: dict[str, Any],
    implementation_sha: str,
    implementation_origin_sha: str | None = None,
    api: execution_evidence.GitHubApi | None = None,
    clock: Callable[[], float] = time.monotonic,
    sleep: Callable[[float], None] = time.sleep,
) -> dict[str, Any]:
    checkpoint = execution_plan.checkpoint_for_id(plan, resolution["checkpoint_id"])
    _run_pre_host_checks(repo_root, implementation_sha)
    subject = git_output(repo_root, "show", "-s", "--format=%s", implementation_sha)
    evidence_result = verify_hosted_checkpoint(
        repo_root,
        plan,
        checkpoint,
        implementation_sha,
        subject,
        implementation_origin_sha=implementation_origin_sha,
        api=api,
        clock=clock,
        sleep=sleep,
    )
    return finalize_verified_checkpoint(
        repo_root,
        plan=plan,
        state=state,
        checkpoint=checkpoint,
        evidence_result=evidence_result,
        implementation_sha=implementation_sha,
        api=evidence_result["api"],
    )


COMPLETION_INTENT_PATH = ".git/or-v2-completion-intent.json"


def _read_store_object(store_root: Path, digest: str) -> dict[str, Any]:
    """Content-addressed controller object load; the digest is the identity."""
    if not isinstance(digest, str) or not re.fullmatch(r"[0-9a-f]{64}", digest):
        raise SupervisorError("handoff receipt digest must be a SHA-256 hex digest")
    root = Path(store_root)
    if not root.is_dir():
        raise SupervisorError("handoff controller store root is not a directory")
    from model_orchestrator import contracts as contracts, store as runtime_store
    path = root / "objects" / (digest + ".json")
    try:
        record = runtime_store._object(runtime_store._read(path))
        if contracts.canonical_digest(record) != digest:
            raise SupervisorError("handoff receipt object identity mismatch")
        return record
    except contracts.ContractError as exc:
        raise SupervisorError("cannot read immutable handoff object: " + str(exc)) from exc


def _bound_handoff_inputs(store_root, task_id, authorization_digest, remote_receipt_digest, snapshot=None):
    """Only published controller objects and the original immutable task bind handoff."""
    from model_orchestrator import contracts as contracts, store as runtime_store
    if not isinstance(task_id, str) or not task_id:
        raise SupervisorError("handoff requires an explicit task ID; task_id=None never invokes the supervisor")
    try:
        current = snapshot if snapshot is not None else runtime_store._read(Path(store_root) / 'state.json')
        task_state = current['tasks'].get(task_id)
        if (current.get('schema_version') != 1 or current.get('paused') is not False
                or current.get('active_task') is not None or not isinstance(task_state, dict)
                or task_state.get('status') != 'SETTLED' or task_state.get('stage') is not None):
            raise SupervisorError("handoff requires an unpaused settled controller task")
        published = current['object_digests']
        if (type(published) is not list or any(type(value) is not str or re.fullmatch('[0-9a-f]{64}', value) is None for value in published)
                or len(set(published)) != len(published) or type(task_state.get('lease_epoch')) is not int
                or task_state['lease_epoch'] < 1 or type(task_state.get('attempt')) is not int or task_state['attempt'] < 1):
            raise SupervisorError("handoff requires valid published objects and a claimed task lease")
        if not all(digest in published for digest in
                   (task_state['contract_digest'], authorization_digest, remote_receipt_digest)):
            raise SupervisorError("handoff objects are not published in the controller snapshot")
        contract = _read_store_object(store_root, task_state['contract_digest'])
        if contract['kind'] != 'task_contract' or contract['payload'].get('task_id') != task_id:
            raise SupervisorError("handoff task pointer is not the original immutable contract")
        task = contract['payload']
        schemas = contracts.load_protocol_schemas(REPO_ROOT)
        contracts.validate_task_contract(task, schemas, frozen_template=task)
        authorization_record = _read_store_object(store_root, authorization_digest)
        if authorization_record['kind'] != 'authorization' or authorization_record['payload'].get('task_id') != task_id:
            raise SupervisorError("handoff authorization binds a different task or object kind")
        authorization = authorization_record['payload'].get('authorization')
        if not isinstance(authorization, dict):
            raise SupervisorError("handoff authorization payload is malformed")
        binding = dict(authorization)
        binding.update({field: task.get(field) for field in ('task_id', 'checkpoint_id', 'base_sha', 'authority_digest', 'template_digest')})
        binding['lease_epoch'] = task_state['lease_epoch']
        contracts.validate_record(authorization, 'promotion_authorization',
                                  schemas, context=binding)
        for field in ('task_id', 'checkpoint_id', 'base_sha', 'authority_digest', 'template_digest'):
            if authorization.get(field) != task.get(field):
                raise SupervisorError("handoff authorization differs from immutable task: " + field)
        if authorization['lease_epoch'] != task_state['lease_epoch']:
            raise SupervisorError("handoff authorization has a stale task lease")
        remote_record = _read_store_object(store_root, remote_receipt_digest)
        if remote_record['kind'] != 'receipt' or remote_record['payload'].get('kind') != 'remote-promotion':
            raise SupervisorError("handoff remote object is not a remote promotion receipt")
        remote = remote_record['payload']['receipt']
        candidate = authorization['candidate_sha']
        contracts.validate_record(remote, 'remote_promotion_receipt', schemas,
            context={'task_id': task_id, 'authorization_digest': authorization_digest,
                     'base_sha': task['base_sha'], 'candidate_sha': candidate})
        if (remote.get('task_id') != task_id or remote.get('authorization_digest') != authorization_digest
                or remote.get('base_sha') != task['base_sha'] or remote.get('destination_ref') != 'refs/heads/main'
                or remote.get('observed_remote_sha') != candidate or remote.get('candidate_sha') != candidate):
            raise SupervisorError("handoff remote receipt has inconsistent task/authorization/candidate binding")
        return task, authorization, remote, current
    except (KeyError, TypeError, contracts.ContractError) as exc:
        raise SupervisorError("malformed or unreadable handoff controller snapshot: " + str(exc)) from exc


def _validate_handoff_head(repo_root, candidate_sha, *, refresh):
    # Optional index refresh and host Git helpers cannot write during disabled inspection.
    def observe(*args):
        return git_output(repo_root, '--no-optional-locks', '-c', 'core.hooksPath=' + os.devnull,
                          '-c', 'core.fsmonitor=false', *args)
    if observe("status", "--porcelain=v1", "--untracked-files=all"):
        raise SupervisorError("handoff requires a clean worktree")
    if refresh:
        observe("fetch", "--prune", "origin")
    head = observe("rev-parse", "HEAD")
    origin = observe("rev-parse", "origin/main")
    observed = observe("ls-remote", "origin", "refs/heads/main").split()
    if head != candidate_sha or origin != candidate_sha or observed != [candidate_sha, 'refs/heads/main']:
        raise SupervisorError("handoff requires HEAD == origin/main == live remote == promoted implementation SHA")


def validate_disabled_task_handoff(
    repo_root: Path, *, store, task_id: str, authorization_digest: str, remote_receipt_digest: str,
) -> dict[str, Any]:
    """Read-only disabled control-plane readiness; no product completion or adoption."""
    from model_orchestrator import contracts as contracts, store as runtime_store
    if type(store) is not runtime_store.RuntimeStore:
        raise SupervisorError("disabled handoff requires a trusted runtime store")
    try:
        snapshot = store.inspect()
        task, authorization, remote, _ = _bound_handoff_inputs(
            store.root, task_id, authorization_digest, remote_receipt_digest, snapshot)
        contracts.validate_phase_admission(store.authority, task, capability='M4')
        if task.get('task_kind') != 'control_plane_phase':
            raise SupervisorError("disabled handoff requires a control-plane phase task")
        if any(task.get(field) != 'none' for field in ('project_schema_effect', 'recovery_schema_effect', 'ipc_effect')):
            raise SupervisorError("disabled handoff cannot change product contracts")
        candidate_sha = authorization['candidate_sha']
        _validate_handoff_head(repo_root, candidate_sha, refresh=False)
        plan, state = execution_plan.load_plan_state(repo_root)
        versions = {'project_schema': 7, 'recovery_schema': 1, 'ipc_protocol': 1}
        if execution_plan.read_contract_versions(repo_root) != versions or state.get('verified_contract_versions') != versions:
            raise SupervisorError("disabled handoff requires preserved product contract versions 7/1/1")
        for path in ('docs/execution/PLAN.json', 'docs/execution/STATE.json'):
            if _git_file_bytes(repo_root, task['base_sha'], path) != _git_file_bytes(repo_root, candidate_sha, path):
                raise SupervisorError("disabled handoff changed product plan/state")
        if store.inspect() != snapshot:
            raise SupervisorError("controller changed during disabled handoff validation")
        return dict(schema_version=1, status='DISABLED_CONTROL_PLANE_READY', task_id=task_id,
                    task_checkpoint=task['checkpoint_id'], implementation_sha=candidate_sha,
                    task_contract_digest=contracts.canonical_digest(task), authorization_digest=authorization_digest,
                    supervisor_digest=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
                    remote_receipt_digest=remote_receipt_digest, product_next=state.get('current_next'),
                    verified_contract_versions=versions,
                    authority_semantics='DISABLED_FACTS_ONLY_NO_PRODUCT_COMPLETION_NO_ADOPTION')
    except (contracts.ContractError, execution_plan.PlanError, subprocess.CalledProcessError) as exc:
        raise SupervisorError("disabled handoff validation refused: " + str(exc)) from exc


def validate_task_handoff(
    repo_root: Path,
    *,
    task_id: str,
    checkpoint_id: str,
    store_root: Path,
    authorization_digest: str,
    remote_receipt_digest: str,
) -> dict[str, Any]:
    """Validate a task-bound V2 handoff without invoking anything hosted.

    Missing task ID, forged/stale receipts, wrong SHA/NEXT/main, or a dirty
    tree refuses before the supervisor is invoked. A positive handoff binds
    the original task plus all receipts and the frozen supervisor code.
    """
    if not isinstance(checkpoint_id, str) or not checkpoint_id:
        raise SupervisorError("handoff requires an explicit checkpoint ID")
    task, authorization, remote_receipt, snapshot = _bound_handoff_inputs(
        store_root, task_id, authorization_digest, remote_receipt_digest)
    if task.get('task_kind') != 'product_checkpoint':
        raise SupervisorError("disabled control-plane tasks cannot enter product handoff")
    task_checkpoint = task.get('checkpoint_id')
    if task_checkpoint != checkpoint_id:
        raise SupervisorError("immutable task checkpoint differs from product handoff checkpoint")
    from model_orchestrator import store as runtime_store, contracts as contracts
    try:
        bootstrap = runtime_store._read(Path(store_root) / 'bootstrap.json')
    except contracts.ContractError as exc:
        raise SupervisorError("product handoff requires a controller bootstrap: " + str(exc)) from exc
    if bootstrap.get('execution') == 'DISABLED_BUILD_ONLY':
        raise SupervisorError("disabled build authority cannot complete a product checkpoint")
    plan, state = execution_plan.load_plan_state(repo_root)
    if state.get("current_next") != checkpoint_id or state.get('checkpoints', {}).get(checkpoint_id) != 'NEXT':
        raise SupervisorError("handoff checkpoint is not current NEXT")
    execution_plan.checkpoint_for_id(plan, checkpoint_id)
    candidate_sha = authorization['candidate_sha']
    _validate_handoff_head(repo_root, candidate_sha, refresh=True)
    from model_orchestrator import store as runtime_store
    if runtime_store._read(Path(store_root) / 'state.json') != snapshot:
        raise SupervisorError("controller changed during product handoff validation")
    supervisor_digest = hashlib.sha256(Path(__file__).read_bytes()).hexdigest()
    return {
        "schema_version": 1,
        "task_id": task_id,
        "task_checkpoint": task_checkpoint,
        "checkpoint_id": checkpoint_id,
        "implementation_sha": candidate_sha,
        "authorization_digest": authorization_digest,
        "remote_receipt_digest": remote_receipt_digest,
        "supervisor_digest": supervisor_digest,
        "next": checkpoint_id,
        "head": candidate_sha,
    }


def write_completion_intent(repo_root: Path, intent: Mapping[str, Any]) -> Path:
    """Persist the exact completion intent before the state/evidence commit."""
    from execution_plan import validate_completion_intent as _validate_intent

    plan, state = execution_plan.load_plan_state(repo_root)
    _validate_intent(intent, plan, state)
    path = repo_root / COMPLETION_INTENT_PATH
    data = (json.dumps(dict(intent), sort_keys=True) + "\n").encode("utf-8")
    fd = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_TRUNC, 0o600)
    try:
        os.write(fd, data)
        os.fsync(fd)
    finally:
        os.close(fd)
    return path


def read_completion_intent(repo_root: Path) -> dict[str, Any] | None:
    path = repo_root / COMPLETION_INTENT_PATH
    if not path.is_file():
        return None
    try:
        intent = json.loads(path.read_bytes().decode("utf-8"))
    except (OSError, ValueError) as exc:
        raise SupervisorError(f"cannot read completion intent: {exc}") from exc
    if not isinstance(intent, dict):
        raise SupervisorError("completion intent must be a JSON object")
    return intent


def reconcile_completion_intent(repo_root: Path, intent: Mapping[str, Any]) -> str:
    """Reconcile preserved exact completion intent; never a second completion commit."""
    from execution_plan import reconcile_completion as _reconcile
    from execution_plan import validate_completion_intent as _validate_intent

    plan, state = execution_plan.load_plan_state(repo_root)
    _validate_intent(intent, plan, state)
    try:
        remote_head = git_output(repo_root, "rev-parse", "origin/main")
    except subprocess.CalledProcessError:
        remote_head = None
    try:
        local_head = git_output(repo_root, "rev-parse", "HEAD")
    except subprocess.CalledProcessError:
        local_head = None
    evidence_path = repo_root / "docs" / "execution" / "evidence" / f"{intent['checkpoint_id']}.json"
    if evidence_path.is_file():
        try:
            stored = json.loads(evidence_path.read_bytes().decode("utf-8"))
        except ValueError:
            stored = None
        if not isinstance(stored, dict):
            raise SupervisorError("completion evidence is corrupt; preserve for controlled recovery")
        if stored.get("implementation_sha") != intent["implementation_sha"]:
            raise SupervisorError("dirty evidence differs from preserved intent; never bless arbitrary data")
        evidence_exists = True
    else:
        evidence_exists = False
    decision = _reconcile(
        intent, remote_head=remote_head, local_head=local_head, evidence_exists=evidence_exists
    )
    if decision == "DIAGNOSE":
        raise SupervisorError("completion state does not match preserved intent; diagnose")
    return decision


def run_task_handoff(
    repo_root: Path,
    *,
    task_id: str,
    checkpoint_id: str,
    store_root: Path,
    authorization_digest: str,
    remote_receipt_digest: str,
    control_plane_receipt: Mapping[str, Any] | None = None,
    api: execution_evidence.GitHubApi | None = None,
    clock: Callable[[], float] = time.monotonic,
    sleep: Callable[[float], None] = time.sleep,
) -> str:
    """Task-bound V2 handoff: validate, verify hosted evidence, complete once."""
    handoff = validate_task_handoff(
        repo_root,
        task_id=task_id,
        checkpoint_id=checkpoint_id,
        store_root=store_root,
        authorization_digest=authorization_digest,
        remote_receipt_digest=remote_receipt_digest,
    )
    plan, state = execution_plan.load_plan_state(repo_root)
    checkpoint = execution_plan.checkpoint_for_id(plan, checkpoint_id)
    implementation_sha = handoff["implementation_sha"]
    _run_pre_host_checks(repo_root, implementation_sha)
    subject = git_output(repo_root, "show", "-s", "--format=%s", implementation_sha)
    evidence_result = verify_hosted_checkpoint(
        repo_root,
        plan,
        checkpoint,
        implementation_sha,
        subject,
        api=api,
        clock=clock,
        sleep=sleep,
        control_plane_receipt=control_plane_receipt,
    )
    record = evidence_result["record"]
    intent = {
        "schema_version": 1,
        "checkpoint_id": checkpoint_id,
        "implementation_sha": implementation_sha,
        "evidence_digest": hashlib.sha256(
            json.dumps(record, sort_keys=True, separators=(",", ":")).encode()
        ).hexdigest(),
        "state_path": "docs/execution/STATE.json",
        "evidence_path": f"docs/execution/evidence/{checkpoint_id}.json",
    }
    write_completion_intent(repo_root, intent)
    result = finalize_verified_checkpoint(
        repo_root,
        plan=plan,
        state=state,
        checkpoint=checkpoint,
        evidence_result=evidence_result,
        implementation_sha=implementation_sha,
        api=evidence_result["api"],
    )
    intent["recorded_completion_sha"] = result["state_commit_sha"]
    write_completion_intent(repo_root, intent)
    return _verified_report(result, plan)


def run_goal(
    repo_root: Path, goal: str, runner: str | None = None, resume_sha: str | None = None
) -> list[str]:
    ensure_start_state(repo_root)
    reports: list[str] = []
    resume_pending = resume_sha is not None
    while True:
        preflight = _preflight_goal_data(repo_root, goal)
        plan = preflight["plan"]
        state = preflight["state"]
        resolution = preflight["resolution"]
        if resume_pending:
            head = git_output(repo_root, "rev-parse", "HEAD")
            origin = git_output(repo_root, "rev-parse", "origin/main")
            allowed_paths = resolution["runner_allowed_protected_paths"]
            _resume_baseline(repo_root, resume_sha or "", allowed_paths)
            validate_resume_preconditions(
                resume_sha=resume_sha or "",
                head=head,
                origin_main=origin,
                current_next=state.get("current_next"),
                expected_checkpoint=resolution["checkpoint_id"],
                worktree_clean=not bool(git_output(repo_root, "status", "--porcelain")),
                baseline_is_ancestor=True,
            )
            _validate_candidate_transition(
                plan, state, preflight["policy"], repo_root
            )
            result = _run_one_checkpoint(
                repo_root,
                plan=plan,
                state=state,
                resolution=resolution,
                implementation_sha=resume_sha or "",
            )
            resume_pending = False
        else:
            if runner is None:
                raise SupervisorError("--runner is required unless --resume-sha is supplied")
            before_plan_bytes = (repo_root / "docs/execution/PLAN.json").read_bytes()
            before_state_bytes = (repo_root / "docs/execution/STATE.json").read_bytes()
            allowed_paths = resolution["runner_allowed_protected_paths"]
            protected_before = capture_protected_surfaces(repo_root)
            baseline_head = git_output(repo_root, "rev-parse", "HEAD")
            invoke_runner(repo_root, runner, checkpoint_prompt(repo_root, resolution))
            git_output(repo_root, "fetch", "--prune", "origin")
            if (repo_root / "docs/execution/PLAN.json").read_bytes() != before_plan_bytes:
                raise SupervisorError("runner changed immutable PLAN.json")
            if (repo_root / "docs/execution/STATE.json").read_bytes() != before_state_bytes:
                raise SupervisorError("runner changed STATE.json; only the supervisor may advance state")
            assert_protected_surfaces_unchanged(
                protected_before,
                capture_protected_surfaces(repo_root),
                allowed_paths,
            )
            refuse_dirty_worktree(repo_root)
            head = git_output(repo_root, "rev-parse", "HEAD")
            origin = git_output(repo_root, "rev-parse", "origin/main")
            if head != origin:
                raise SupervisorError("runner did not leave HEAD == origin/main")
            if head == baseline_head:
                raise SupervisorError("runner produced no new implementation commit")
            _validate_candidate_transition(
                plan, state, preflight["policy"], repo_root
            )
            result = _run_one_checkpoint(
                repo_root,
                plan=plan,
                state=state,
                resolution=resolution,
                implementation_sha=head,
            )
        reports.append(_verified_report(result, plan))
        if resolution["goal_complete_after_current"]:
            return reports


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--goal", required=True)
    modes = parser.add_mutually_exclusive_group(required=True)
    modes.add_argument("--runner", help="absolute executable runner path")
    modes.add_argument(
        "--resume-sha",
        help="resume hosted verification for an already-pushed exact implementation SHA",
    )
    modes.add_argument(
        "--prepare",
        action="store_true",
        help="print the authoritative prompt for the current checkpoint",
    )
    modes.add_argument(
        "--preflight",
        action="store_true",
        help="run deterministic read-only goal checks without invoking a runner",
    )
    modes.add_argument(
        "--repair-resume-from",
        metavar="FAILED_SHA",
        help="verify a trusted repaired HEAD for an already-landed implementation",
    )
    modes.add_argument(
        "--resume-task",
        metavar="TASK_ID",
        help="task-bound V2 handoff for an already-promoted implementation SHA",
    )
    parser.add_argument("--task-checkpoint", help="product checkpoint the V2 task promotes")
    parser.add_argument("--store-root", help="controller-owned runtime store root")
    parser.add_argument("--authorization-digest", help="promotion authorization object digest")
    parser.add_argument("--remote-receipt-digest", help="remote promotion receipt object digest")
    parser.add_argument("--control-plane-receipt", help="JSON file with the nested V2 completion receipt")
    args = parser.parse_args(argv)
    try:
        if args.preflight:
            print(preflight_goal(REPO_ROOT, args.goal))
            return 0
        if args.prepare:
            print(prepare_goal(REPO_ROOT, args.goal), end="")
            return 0
        if args.repair_resume_from:
            reports = repair_resume_goal(REPO_ROOT, args.goal, args.repair_resume_from)
        elif args.resume_task:
            for flag in ("task_checkpoint", "store_root", "authorization_digest", "remote_receipt_digest"):
                if not getattr(args, flag):
                    raise SupervisorError(f"--resume-task requires --{flag.replace('_', '-')}")
            receipt = None
            if args.control_plane_receipt:
                receipt = json.loads(Path(args.control_plane_receipt).read_text(encoding="utf-8"))
            reports = [
                run_task_handoff(
                    REPO_ROOT,
                    task_id=args.resume_task,
                    checkpoint_id=args.task_checkpoint,
                    store_root=Path(args.store_root),
                    authorization_digest=args.authorization_digest,
                    remote_receipt_digest=args.remote_receipt_digest,
                    control_plane_receipt=receipt,
                )
            ]
        else:
            reports = run_goal(REPO_ROOT, args.goal, args.runner, args.resume_sha)
    except (
        SupervisorError,
        execution_evidence.EvidenceError,
        execution_plan.PlanError,
        OSError,
        subprocess.CalledProcessError,
    ) as exc:
        print(f"Supervisor stopped: {exc}", file=sys.stderr)
        return 1
    for report in reports:
        print(report)
        print()
    print(f"Supervisor completed {args.goal}.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
