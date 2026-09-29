#!/usr/bin/env python3
"""Run fresh checkpoint runners and complete them only after hosted evidence."""

from __future__ import annotations

import argparse
import copy
import datetime as _datetime
import json
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
}
PROTECTED_DIRECTORY_PREFIXES = (
    "docs/execution/phases/",
    "docs/execution/evidence/",
    ".github/workflows/",
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
    normalized = path.replace("\\", "/").lstrip("./")
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
    before: Mapping[str, bytes | None], after: Mapping[str, bytes | None]
) -> list[str]:
    return sorted(
        path
        for path in set(before) | set(after)
        if before.get(path) != after.get(path)
    )


def assert_protected_surfaces_unchanged(
    before: Mapping[str, bytes | None], after: Mapping[str, bytes | None]
) -> None:
    changed = changed_protected_surfaces(before, after)
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
    before: dict[str, Any], after: dict[str, Any], plan: dict[str, Any]
) -> str:
    """Validate exactly one NEXT -> DONE transition and its planned successor."""

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
    return checkpoint_id


def advance_state_once(
    before: dict[str, Any], plan: dict[str, Any], checkpoint_id: str, *, today: str | None = None
) -> dict[str, Any]:
    """Create the only state transition the supervisor is allowed to write."""

    after = copy.deepcopy(before)
    if before.get("current_next") != checkpoint_id:
        raise SupervisorError(
            f"state current_next is {before.get('current_next')!r}, expected {checkpoint_id!r}"
        )
    checkpoint = execution_plan.checkpoint_for_id(plan, checkpoint_id)
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
    assert_state_advanced_once(before, after, plan)
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
    return (
        "You are executing exactly one locked Opencut Reinforced checkpoint.\n"
        f"Repository root: {repo_root}\n"
        f"Goal: {resolution['goal']}\n"
        f"Checkpoint: {resolution['checkpoint_id']} — {resolution['title']}\n"
        f"Phase: {resolution['phase']}\n"
        f"Locked specification: {resolution['spec_document']}\n"
        f"Next relation: {resolution['next_checkpoint_relation'] or 'none'}\n\n"
        "Implement exactly this checkpoint and no successor checkpoint. Do not edit PLAN.json, "
        "STATE.json, EVIDENCE_POLICY.json, architecture invariants or policy, phase specs, "
        "execution supervisor/validator/evidence files, completion evidence, or protected "
        "workflow gates. Push implementation commits only; do not create a state/evidence "
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


def prepare_goal(repo_root: Path, goal: str) -> str:
    ensure_start_state(repo_root)
    plan, state = execution_plan.load_plan_state(repo_root)
    execution_plan.validate_plan(plan, state, repo_root)
    resolution = execution_plan.resolve_goal(plan, state, goal, repo_root)
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
    api: execution_evidence.GitHubApi | None = None,
    clock: Callable[[], float] = time.monotonic,
    sleep: Callable[[float], None] = time.sleep,
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
    record = execution_evidence.build_evidence_record(
        checkpoint_id=str(checkpoint["id"]),
        implementation_sha=implementation_sha,
        implementation_subject=implementation_subject,
        gates=gates,
        developer_preview=preview,
    )
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
    after_state = advance_state_once(state, plan, checkpoint_id)
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


def _resume_baseline(repo_root: Path, resume_sha: str) -> str:
    baseline = git_output(repo_root, "log", "-1", "--format=%H", "--", "docs/execution/STATE.json")
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
    protected = [path for path in changed if is_protected_execution_path(path)]
    if protected:
        raise SupervisorError(
            "resume implementation changed protected execution-control files: "
            + ", ".join(protected)
        )
    return baseline


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
    api: execution_evidence.GitHubApi | None = None,
    clock: Callable[[], float] = time.monotonic,
    sleep: Callable[[float], None] = time.sleep,
) -> dict[str, Any]:
    checkpoint = execution_plan.checkpoint_for_id(plan, resolution["checkpoint_id"])
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


def run_goal(
    repo_root: Path, goal: str, runner: str | None = None, resume_sha: str | None = None
) -> list[str]:
    ensure_start_state(repo_root)
    reports: list[str] = []
    resume_pending = resume_sha is not None
    while True:
        plan, state = execution_plan.load_plan_state(repo_root)
        execution_plan.validate_plan(plan, state, repo_root)
        resolution = execution_plan.resolve_goal(plan, state, goal, repo_root)
        if resume_pending:
            head = git_output(repo_root, "rev-parse", "HEAD")
            origin = git_output(repo_root, "rev-parse", "origin/main")
            _resume_baseline(repo_root, resume_sha or "")
            validate_resume_preconditions(
                resume_sha=resume_sha or "",
                head=head,
                origin_main=origin,
                current_next=state.get("current_next"),
                expected_checkpoint=resolution["checkpoint_id"],
                worktree_clean=not bool(git_output(repo_root, "status", "--porcelain")),
                baseline_is_ancestor=True,
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
            protected_before = capture_protected_surfaces(repo_root)
            invoke_runner(repo_root, runner, checkpoint_prompt(repo_root, resolution))
            git_output(repo_root, "fetch", "--prune", "origin")
            if (repo_root / "docs/execution/PLAN.json").read_bytes() != before_plan_bytes:
                raise SupervisorError("runner changed immutable PLAN.json")
            if (repo_root / "docs/execution/STATE.json").read_bytes() != before_state_bytes:
                raise SupervisorError("runner changed STATE.json; only the supervisor may advance state")
            assert_protected_surfaces_unchanged(protected_before, capture_protected_surfaces(repo_root))
            refuse_dirty_worktree(repo_root)
            head = git_output(repo_root, "rev-parse", "HEAD")
            origin = git_output(repo_root, "rev-parse", "origin/main")
            if head != origin:
                raise SupervisorError("runner did not leave HEAD == origin/main")
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
    args = parser.parse_args(argv)
    try:
        if args.prepare:
            print(prepare_goal(REPO_ROOT, args.goal), end="")
            return 0
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
