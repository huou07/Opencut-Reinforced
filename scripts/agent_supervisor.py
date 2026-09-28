#!/usr/bin/env python3
"""Run exactly one fresh external runner per validated execution checkpoint."""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path
from typing import Any

SCRIPT_DIR = Path(__file__).resolve().parent
REPO_ROOT = SCRIPT_DIR.parent
sys.path.insert(0, str(SCRIPT_DIR))

import execution_plan  # noqa: E402


class SupervisorError(RuntimeError):
    """Raised when the direct-main execution contract is not satisfied."""


def git_output(repo_root: Path, *arguments: str) -> str:
    result = subprocess.run(
        ["git", *arguments],
        cwd=repo_root,
        capture_output=True,
        text=True,
        check=True,
    )
    return result.stdout.strip()


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
    before_checkpoints = before.get("checkpoints")
    after_checkpoints = after.get("checkpoints")
    if not isinstance(before_checkpoints, dict) or not isinstance(after_checkpoints, dict):
        raise SupervisorError("state checkpoints are not objects")
    if set(before_checkpoints) != set(after_checkpoints):
        raise SupervisorError("runner changed the set of state checkpoint ids")
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
            raise SupervisorError(f"runner changed unrelated checkpoint {other_id}")
    if set(transitions) != allowed_transitions:
        raise SupervisorError(
            "runner must advance exactly one checkpoint and its planned successor; "
            f"changed: {', '.join(transitions) or 'none'}"
        )
    if after.get("current_next") != expected_next:
        raise SupervisorError(
            f"state current_next is {after.get('current_next')!r}, expected {expected_next!r}"
        )
    return checkpoint_id


def checkpoint_prompt(repo_root: Path, resolution: dict[str, Any]) -> str:
    return (
        "You are executing one locked Opencut Reinforced checkpoint.\n"
        f"Repository root: {repo_root}\n"
        f"Goal: {resolution['goal']}\n"
        f"Checkpoint: {resolution['checkpoint_id']} — {resolution['title']}\n"
        f"Phase: {resolution['phase']}\n"
        f"Locked specification: {resolution['spec_document']}\n"
        f"Next relation: {resolution['next_checkpoint_relation'] or 'none'}\n\n"
        "Read AGENTS.md, PLAN.json, STATE.json, and the locked specification. "
        "Implement only this checkpoint, update required docs/tests, commit the "
        "coherent change, and push through the repository workflow. Do not edit "
        "the locked specification or permanent invariants. Stop on a conflict "
        "instead of repairing the plan creatively.\n"
    )


def run_goal(repo_root: Path, goal: str, runner: str) -> None:
    ensure_start_state(repo_root)
    while True:
        plan, state = execution_plan.load_plan_state(repo_root)
        execution_plan.validate_plan(plan, state, repo_root)
        resolution = execution_plan.resolve_goal(plan, state, goal, repo_root)
        before_state = json.loads(json.dumps(state))
        invoke_runner(repo_root, runner, checkpoint_prompt(repo_root, resolution))
        git_output(repo_root, "fetch", "--prune", "origin")
        refuse_dirty_worktree(repo_root)
        head = git_output(repo_root, "rev-parse", "HEAD")
        origin = git_output(repo_root, "rev-parse", "origin/main")
        if head != origin:
            raise SupervisorError("runner did not leave HEAD == origin/main")
        after_plan, after_state = execution_plan.load_plan_state(repo_root)
        if after_plan != plan:
            raise SupervisorError("runner changed immutable PLAN.json")
        execution_plan.validate_plan(after_plan, after_state, repo_root)
        checkpoint_id = assert_state_advanced_once(before_state, after_state, after_plan)
        if checkpoint_id != resolution["checkpoint_id"]:
            raise SupervisorError(
                f"runner advanced {checkpoint_id}, expected {resolution['checkpoint_id']}"
            )
        if resolution["goal_complete_after_current"]:
            return


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--goal", required=True)
    parser.add_argument("--runner", required=True, help="absolute executable runner path")
    args = parser.parse_args()
    try:
        run_goal(REPO_ROOT, args.goal, args.runner)
    except (SupervisorError, execution_plan.PlanError, OSError, subprocess.CalledProcessError) as exc:
        print(f"Supervisor stopped: {exc}", file=sys.stderr)
        return 1
    print(f"Supervisor completed {args.goal}.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
