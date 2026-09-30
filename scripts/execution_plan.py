#!/usr/bin/env python3
"""Inspect and validate OR's immutable execution graph and mutable state."""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path, PurePosixPath, PureWindowsPath
from typing import Any, Iterable


REPO_ROOT = Path(__file__).resolve().parents[1]
PLAN_PATH = REPO_ROOT / "docs" / "execution" / "PLAN.json"
STATE_PATH = REPO_ROOT / "docs" / "execution" / "STATE.json"
VALID_CHECKPOINT_STATUSES = {"DONE", "NEXT", "PLANNED"}
VALID_PHASE_STATUSES = {"DONE", "IN_PROGRESS", "PLANNED"}


class PlanError(ValueError):
    """Raised when the execution graph or state violates its contract."""


def load_json(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise PlanError(f"cannot load {path}: {exc}") from exc
    if not isinstance(value, dict):
        raise PlanError(f"{path} must contain a JSON object")
    return value


def load_plan_state(repo_root: Path = REPO_ROOT) -> tuple[dict[str, Any], dict[str, Any]]:
    execution_dir = repo_root / "docs" / "execution"
    return load_json(execution_dir / "PLAN.json"), load_json(execution_dir / "STATE.json")


def _require_object(value: Any, label: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise PlanError(f"{label} must be an object")
    return value


def _require_list(value: Any, label: str) -> list[Any]:
    if not isinstance(value, list):
        raise PlanError(f"{label} must be an array")
    return value


def _require_string(value: Any, label: str) -> str:
    if not isinstance(value, str) or not value:
        raise PlanError(f"{label} must be a non-empty string")
    return value


def _validate_runner_allowed_protected_paths(
    checkpoint_id: str,
    checkpoint: dict[str, Any],
    value: Any,
    repo_root: Path,
) -> None:
    label = f"checkpoint {checkpoint_id}.runner_allowed_protected_paths"
    paths = _require_list(value, label)
    if not checkpoint["architecture_gate"]:
        raise PlanError(f"{label} requires architecture_gate = true")

    seen: set[str] = set()
    for index, raw_path in enumerate(paths):
        path_label = f"{label}[{index}]"
        path = _require_string(raw_path, path_label)
        parsed = PurePosixPath(path)
        if (
            "\\" in path
            or path != parsed.as_posix()
            or parsed.is_absolute()
            or PureWindowsPath(path).is_absolute()
            or any(part in {"", ".", ".."} for part in path.split("/"))
            or any(character in path for character in "*?[]")
            or not path.startswith(".github/workflows/")
        ):
            raise PlanError(
                f"{path_label} must be a normalized exact file path under .github/workflows/"
            )
        if path in seen:
            raise PlanError(f"{label} contains duplicate path: {path}")
        if (repo_root / Path(*parsed.parts)).is_dir():
            raise PlanError(f"{path_label} must name a file, not a directory")
        seen.add(path)


def _checkpoint_map(
    plan: dict[str, Any], repo_root: Path = REPO_ROOT
) -> dict[str, dict[str, Any]]:
    raw_checkpoints = _require_list(plan.get("checkpoints"), "plan.checkpoints")
    result: dict[str, dict[str, Any]] = {}
    required = {
        "id",
        "phase",
        "title",
        "spec_document",
        "prerequisite_checkpoint_ids",
        "milestone_membership",
        "user_visible",
        "developer_preview_required",
        "architecture_gate",
        "expected_project_schema_effect_category",
        "expected_ipc_effect_category",
        "dependency_change_policy",
        "next_checkpoint_relation",
    }
    for index, raw in enumerate(raw_checkpoints):
        checkpoint = _require_object(raw, f"plan.checkpoints[{index}]")
        missing = required - checkpoint.keys()
        if missing:
            raise PlanError(
                f"checkpoint {index} is missing fields: {', '.join(sorted(missing))}"
            )
        checkpoint_id = _require_string(checkpoint["id"], f"checkpoint {index}.id")
        if checkpoint_id in result:
            raise PlanError(f"duplicate checkpoint id: {checkpoint_id}")
        if not isinstance(checkpoint["phase"], int):
            raise PlanError(f"checkpoint {checkpoint_id}.phase must be an integer")
        _require_string(checkpoint["title"], f"checkpoint {checkpoint_id}.title")
        _require_string(
            checkpoint["spec_document"], f"checkpoint {checkpoint_id}.spec_document"
        )
        prerequisites = _require_list(
            checkpoint["prerequisite_checkpoint_ids"],
            f"checkpoint {checkpoint_id}.prerequisite_checkpoint_ids",
        )
        if any(not isinstance(item, str) or not item for item in prerequisites):
            raise PlanError(f"checkpoint {checkpoint_id} has an invalid prerequisite id")
        memberships = _require_list(
            checkpoint["milestone_membership"],
            f"checkpoint {checkpoint_id}.milestone_membership",
        )
        if any(not isinstance(item, str) or not item for item in memberships):
            raise PlanError(f"checkpoint {checkpoint_id} has an invalid milestone id")
        for field in ("user_visible", "developer_preview_required", "architecture_gate"):
            if not isinstance(checkpoint[field], bool):
                raise PlanError(f"checkpoint {checkpoint_id}.{field} must be boolean")
        if "runner_allowed_protected_paths" in checkpoint:
            _validate_runner_allowed_protected_paths(
                checkpoint_id,
                checkpoint,
                checkpoint["runner_allowed_protected_paths"],
                repo_root,
            )
        _require_string(
            checkpoint["expected_project_schema_effect_category"],
            f"checkpoint {checkpoint_id}.expected_project_schema_effect_category",
        )
        _require_string(
            checkpoint["expected_ipc_effect_category"],
            f"checkpoint {checkpoint_id}.expected_ipc_effect_category",
        )
        _require_string(
            checkpoint["dependency_change_policy"],
            f"checkpoint {checkpoint_id}.dependency_change_policy",
        )
        relation = checkpoint["next_checkpoint_relation"]
        if relation is not None and (not isinstance(relation, str) or not relation):
            raise PlanError(f"checkpoint {checkpoint_id}.next_checkpoint_relation is invalid")
        result[checkpoint_id] = checkpoint
    return result


def _assert_acyclic(checkpoints: dict[str, dict[str, Any]]) -> None:
    visiting: set[str] = set()
    visited: set[str] = set()

    def visit(checkpoint_id: str) -> None:
        if checkpoint_id in visiting:
            raise PlanError(f"checkpoint prerequisite graph contains a cycle at {checkpoint_id}")
        if checkpoint_id in visited:
            return
        visiting.add(checkpoint_id)
        for prerequisite in checkpoints[checkpoint_id]["prerequisite_checkpoint_ids"]:
            visit(prerequisite)
        visiting.remove(checkpoint_id)
        visited.add(checkpoint_id)

    for checkpoint_id in checkpoints:
        visit(checkpoint_id)


def _validate_owner_category(
    checkpoint_id: str,
    category: str,
    suffix: str,
    field: str,
    owner_field: str,
    valid_owner_categories: set[str],
    checkpoints: dict[str, dict[str, Any]],
) -> None:
    owner_id = category.removesuffix(suffix)
    owner = checkpoints.get(owner_id)
    if owner is None:
        raise PlanError(f"checkpoint {checkpoint_id}.{field} has missing owner {owner_id!r}")

    ancestors: set[str] = set()
    pending = list(checkpoints[checkpoint_id]["prerequisite_checkpoint_ids"])
    while pending:
        prerequisite = pending.pop()
        if prerequisite in ancestors:
            continue
        ancestors.add(prerequisite)
        pending.extend(checkpoints[prerequisite]["prerequisite_checkpoint_ids"])
    if owner_id not in ancestors:
        raise PlanError(
            f"checkpoint {checkpoint_id}.{field} owner {owner_id} is not an earlier prerequisite"
        )
    if owner[owner_field] not in valid_owner_categories:
        raise PlanError(
            f"checkpoint {checkpoint_id}.{field} owner {owner_id} does not own "
            f"a {field} gate"
        )


def _validate_effect_categories(
    checkpoints: dict[str, dict[str, Any]], statuses: dict[str, str]
) -> None:
    for checkpoint_id, checkpoint in checkpoints.items():
        project_field = "expected_project_schema_effect_category"
        project_category = checkpoint[project_field]
        if project_category in {"none", "explicit-model-gate", "typed-model-gate"}:
            pass
        elif re.fullmatch(r"schema-v[1-9][0-9]*", project_category):
            if statuses[checkpoint_id] != "DONE":
                raise PlanError(
                    f"checkpoint {checkpoint_id}.{project_field} legacy category "
                    f"{project_category} requires status DONE"
                )
        elif project_category.endswith("-model-only"):
            _validate_owner_category(
                checkpoint_id,
                project_category,
                "-model-only",
                project_field,
                project_field,
                {"explicit-model-gate", "typed-model-gate"},
                checkpoints,
            )
        else:
            raise PlanError(
                f"checkpoint {checkpoint_id}.{project_field} has unsupported "
                f"category: {project_category}"
            )

        ipc_field = "expected_ipc_effect_category"
        ipc_category = checkpoint[ipc_field]
        if ipc_category in {"none", "explicit-contract-gate"}:
            continue
        if ipc_category.endswith("-contract-only"):
            _validate_owner_category(
                checkpoint_id,
                ipc_category,
                "-contract-only",
                ipc_field,
                ipc_field,
                {"explicit-contract-gate"},
                checkpoints,
            )
            continue
        raise PlanError(
            f"checkpoint {checkpoint_id}.{ipc_field} has unsupported category: {ipc_category}"
        )


def _derive_phase_status(
    phase_id: str, checkpoints: dict[str, dict[str, Any]], statuses: dict[str, str]
) -> str:
    children = [
        checkpoint_id
        for checkpoint_id, checkpoint in checkpoints.items()
        if str(checkpoint["phase"]) == phase_id
    ]
    if not children:
        if phase_id == "5":
            return "DONE"
        raise PlanError(f"phase {phase_id} has no checkpoints")
    child_statuses = {statuses[checkpoint_id] for checkpoint_id in children}
    if child_statuses == {"DONE"}:
        return "DONE"
    if "NEXT" in child_statuses or "DONE" in child_statuses:
        return "IN_PROGRESS"
    return "PLANNED"


def _validate_completion_evidence(
    plan: dict[str, Any],
    statuses: dict[str, str],
    checkpoints: dict[str, dict[str, Any]],
    repo_root: Path,
) -> None:
    """Validate supervisor attestations for checkpoints at the policy boundary."""

    policy_path = repo_root / "docs" / "execution" / "EVIDENCE_POLICY.json"
    checkpoint_ids = [checkpoint["id"] for checkpoint in plan["checkpoints"]]
    if "7A" not in checkpoint_ids and not policy_path.exists():
        return
    if not policy_path.is_file():
        raise PlanError(f"evidence policy is missing: {policy_path}")
    try:
        from execution_evidence import EvidenceError, load_policy, validate_evidence_record

        policy = load_policy(policy_path)
        boundary = policy["enforced_from_checkpoint"]
        if boundary not in checkpoint_ids:
            raise PlanError(f"evidence boundary is not in PLAN.json: {boundary}")
        boundary_index = checkpoint_ids.index(boundary)
        for checkpoint_id, status in statuses.items():
            if status != "DONE" or checkpoint_ids.index(checkpoint_id) < boundary_index:
                continue
            evidence_path = repo_root / "docs" / "execution" / "evidence" / f"{checkpoint_id}.json"
            if not evidence_path.is_file():
                raise PlanError(f"completion evidence is missing: {evidence_path}")
            evidence = load_json(evidence_path)
            try:
                validate_evidence_record(
                    evidence,
                    checkpoint_id=checkpoint_id,
                    checkpoint=checkpoints[checkpoint_id],
                    policy=policy,
                )
            except EvidenceError as exc:
                raise PlanError(f"invalid completion evidence for {checkpoint_id}: {exc}") from exc
    except ImportError as exc:
        raise PlanError("execution evidence module is unavailable") from exc
    except EvidenceError as exc:
        raise PlanError(f"invalid evidence policy: {exc}") from exc


def validate_plan(
    plan: dict[str, Any], state: dict[str, Any], repo_root: Path = REPO_ROOT
) -> dict[str, Any]:
    """Validate plan/state and return a compact summary.

    This function intentionally performs no writes. It is imported by the
    checker, the supervisor, and the stdlib test suite.
    """

    if plan.get("schema_version") != 1:
        raise PlanError("PLAN.json schema_version must be 1")
    if state.get("schema_version") != 1:
        raise PlanError("STATE.json schema_version must be 1")
    _require_string(plan.get("plan_id"), "plan.plan_id")
    checkpoints = _checkpoint_map(plan, repo_root)
    if not checkpoints:
        raise PlanError("plan must contain at least one checkpoint")

    phases = _require_object(plan.get("phases"), "plan.phases")
    for phase_id, phase in phases.items():
        if not isinstance(phase_id, str) or not phase_id:
            raise PlanError("plan phase ids must be non-empty strings")
        phase_object = _require_object(phase, f"plan.phases.{phase_id}")
        _require_string(phase_object.get("title"), f"plan.phases.{phase_id}.title")
        spec_document = _require_string(
            phase_object.get("spec_document"), f"plan.phases.{phase_id}.spec_document"
        )
        if not (repo_root / spec_document).is_file():
            raise PlanError(f"phase specification is missing: {spec_document}")

    for checkpoint_id, checkpoint in checkpoints.items():
        if str(checkpoint["phase"]) not in phases:
            raise PlanError(f"checkpoint {checkpoint_id} refers to unknown phase")
        spec_document = checkpoint["spec_document"]
        if not (repo_root / spec_document).is_file():
            raise PlanError(f"checkpoint {checkpoint_id} specification is missing: {spec_document}")
        for prerequisite in checkpoint["prerequisite_checkpoint_ids"]:
            if prerequisite not in checkpoints:
                raise PlanError(
                    f"checkpoint {checkpoint_id} refers to missing prerequisite {prerequisite}"
                )
        relation = checkpoint["next_checkpoint_relation"]
        if relation is not None:
            if relation not in checkpoints:
                raise PlanError(f"checkpoint {checkpoint_id} refers to missing next {relation}")
            if checkpoint_id not in checkpoints[relation]["prerequisite_checkpoint_ids"]:
                raise PlanError(
                    f"checkpoint {checkpoint_id}.next_checkpoint_relation {relation} "
                    "does not depend on the current checkpoint"
                )
        elif any(
            checkpoint_id in other["prerequisite_checkpoint_ids"]
            for other in checkpoints.values()
        ):
            raise PlanError(f"checkpoint {checkpoint_id} has no next relation but has a successor")

    _assert_acyclic(checkpoints)

    state_checkpoints = _require_object(state.get("checkpoints"), "state.checkpoints")
    state_ids = set(state_checkpoints)
    plan_ids = set(checkpoints)
    if state_ids != plan_ids:
        missing = sorted(plan_ids - state_ids)
        unknown = sorted(state_ids - plan_ids)
        details: list[str] = []
        if missing:
            details.append(f"missing state checkpoints: {', '.join(missing)}")
        if unknown:
            details.append(f"unknown state checkpoints: {', '.join(unknown)}")
        raise PlanError("; ".join(details))
    statuses: dict[str, str] = {}
    for checkpoint_id, raw_status in state_checkpoints.items():
        if raw_status not in VALID_CHECKPOINT_STATUSES:
            raise PlanError(f"invalid status for {checkpoint_id}: {raw_status}")
        statuses[checkpoint_id] = raw_status

    _validate_effect_categories(checkpoints, statuses)

    next_ids = [checkpoint_id for checkpoint_id, status in statuses.items() if status == "NEXT"]
    if len(next_ids) > 1:
        raise PlanError(f"state must have at most one NEXT checkpoint, found: {', '.join(next_ids)}")
    if not next_ids and any(status != "DONE" for status in statuses.values()):
        raise PlanError("state must have one NEXT checkpoint until all checkpoints are DONE")
    next_id = next_ids[0] if next_ids else None
    if next_id is not None:
        for prerequisite in checkpoints[next_id]["prerequisite_checkpoint_ids"]:
            if statuses[prerequisite] != "DONE":
                raise PlanError(f"NEXT checkpoint {next_id} has unfinished prerequisite {prerequisite}")
    for checkpoint_id, status in statuses.items():
        if status == "DONE":
            unfinished = [
                prerequisite
                for prerequisite in checkpoints[checkpoint_id]["prerequisite_checkpoint_ids"]
                if statuses[prerequisite] != "DONE"
            ]
            if unfinished:
                raise PlanError(
                    f"DONE checkpoint {checkpoint_id} has unfinished prerequisite(s): "
                    f"{', '.join(unfinished)}"
                )

    milestones = _require_object(plan.get("milestones"), "plan.milestones")
    for milestone_id, raw_milestone in milestones.items():
        milestone = _require_object(raw_milestone, f"plan.milestones.{milestone_id}")
        members = _require_list(
            milestone.get("checkpoint_ids"), f"plan.milestones.{milestone_id}.checkpoint_ids"
        )
        if not members:
            raise PlanError(f"milestone {milestone_id} must contain checkpoints")
        if len(set(members)) != len(members):
            raise PlanError(f"milestone {milestone_id} contains duplicate checkpoints")
        if any(member not in checkpoints for member in members):
            raise PlanError(f"milestone {milestone_id} contains an unknown checkpoint")
        completion_id = _require_string(
            milestone.get("completion_checkpoint_id"),
            f"plan.milestones.{milestone_id}.completion_checkpoint_id",
        )
        if completion_id not in members:
            raise PlanError(f"milestone {milestone_id} completion checkpoint is not a member")
    known_milestones = set(milestones)
    for checkpoint_id, checkpoint in checkpoints.items():
        for milestone_id in checkpoint["milestone_membership"]:
            if milestone_id not in known_milestones:
                raise PlanError(
                    f"checkpoint {checkpoint_id} refers to unknown milestone {milestone_id}"
                )

    phase_status = _require_object(state.get("phase_status"), "state.phase_status")
    expected_phase_ids = set(phases) | {"5"}
    if set(phase_status) != expected_phase_ids:
        raise PlanError("state.phase_status must contain plan phases and legacy phase 5")
    for phase_id, raw_status in phase_status.items():
        if raw_status not in VALID_PHASE_STATUSES:
            raise PlanError(f"invalid phase status for {phase_id}: {raw_status}")
        expected = _derive_phase_status(phase_id, checkpoints, statuses)
        if raw_status != expected:
            raise PlanError(
                f"phase {phase_id} status is {raw_status}, expected derived status {expected}"
            )
    if state.get("current_next") != next_id:
        raise PlanError(
            f"state.current_next is {state.get('current_next')!r}, expected {next_id!r}"
        )

    _validate_completion_evidence(plan, statuses, checkpoints, repo_root)

    return {
        "checkpoint_count": len(checkpoints),
        "next_checkpoint": next_id,
        "done_count": sum(status == "DONE" for status in statuses.values()),
        "planned_count": sum(status == "PLANNED" for status in statuses.values()),
        "phase_status": dict(phase_status),
    }


def checkpoint_for_id(plan: dict[str, Any], checkpoint_id: str) -> dict[str, Any]:
    for checkpoint in _require_list(plan.get("checkpoints"), "plan.checkpoints"):
        if isinstance(checkpoint, dict) and checkpoint.get("id") == checkpoint_id:
            return checkpoint
    raise PlanError(f"unknown checkpoint: {checkpoint_id}")


def resolve_goal(
    plan: dict[str, Any], state: dict[str, Any], goal: str, repo_root: Path = REPO_ROOT
) -> dict[str, Any]:
    """Resolve a goal to the only checkpoint the next runner may execute."""

    summary = validate_plan(plan, state, repo_root)
    current_next = summary["next_checkpoint"]
    if current_next is None:
        raise PlanError("all checkpoints are complete; there is no NEXT checkpoint")
    if ":" not in goal:
        raise PlanError("goal must be checkpoint:<ID>, phase:<N>, or milestone:<name>")
    goal_kind, goal_value = goal.split(":", 1)
    checkpoint = checkpoint_for_id(plan, current_next)
    if goal_kind == "checkpoint":
        if goal_value != current_next:
            raise PlanError(f"requested checkpoint {goal_value} is not current NEXT {current_next}")
        goal_members = [current_next]
    elif goal_kind == "phase":
        if not goal_value.isdigit() or str(checkpoint["phase"]) != goal_value:
            raise PlanError(f"current NEXT {current_next} is not in requested phase {goal_value}")
        goal_members = [
            item["id"]
            for item in _require_list(plan.get("checkpoints"), "plan.checkpoints")
            if isinstance(item, dict) and str(item.get("phase")) == goal_value
        ]
    elif goal_kind == "milestone":
        milestones = _require_object(plan.get("milestones"), "plan.milestones")
        if goal_value not in milestones:
            raise PlanError(f"unknown milestone: {goal_value}")
        milestone = _require_object(milestones[goal_value], f"plan.milestones.{goal_value}")
        goal_members = list(_require_list(milestone.get("checkpoint_ids"), "milestone.checkpoint_ids"))
        if current_next not in goal_members:
            raise PlanError(f"current NEXT {current_next} is outside milestone {goal_value}")
    else:
        raise PlanError(f"unknown goal kind: {goal_kind}")
    return {
        "goal": goal,
        "checkpoint_id": current_next,
        "title": checkpoint["title"],
        "phase": checkpoint["phase"],
        "spec_document": checkpoint["spec_document"],
        "goal_checkpoint_ids": goal_members,
        "goal_complete_after_current": current_next == goal_members[-1],
        "next_checkpoint_relation": checkpoint["next_checkpoint_relation"],
        "runner_allowed_protected_paths": checkpoint.get("runner_allowed_protected_paths", []),
    }


def _checkpoint_context(plan: dict[str, Any], state: dict[str, Any], checkpoint_id: str) -> dict[str, Any]:
    checkpoint = checkpoint_for_id(plan, checkpoint_id)
    return {
        "checkpoint": checkpoint,
        "state": state["checkpoints"].get(checkpoint_id),
        "current_next": state.get("current_next"),
    }


def _text(value: Any) -> str:
    return json.dumps(value, indent=2, sort_keys=True)


def main(argv: Iterable[str] | None = None) -> int:
    arguments = list(sys.argv[1:] if argv is None else argv)
    json_output = False
    if "--json" in arguments:
        json_output = True
        arguments.remove("--json")
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=("status", "next", "context", "goal", "check"))
    parser.add_argument("value", nargs="?")
    args = parser.parse_args(arguments)
    try:
        plan, state = load_plan_state()
        summary = validate_plan(plan, state, REPO_ROOT)
        if args.command == "check":
            result: Any = {"valid": True, **summary}
        elif args.command == "status":
            result = {
                "current_next": summary["next_checkpoint"],
                "phase_status": summary["phase_status"],
                "done_count": summary["done_count"],
                "planned_count": summary["planned_count"],
            }
        elif args.command == "next":
            if summary["next_checkpoint"] is None:
                result = {"current_next": None, "message": "all checkpoints are complete"}
            else:
                result = _checkpoint_context(plan, state, summary["next_checkpoint"])
        elif args.command == "context":
            if not args.value:
                raise PlanError("context requires a checkpoint id")
            result = _checkpoint_context(plan, state, args.value)
        else:
            if not args.value:
                raise PlanError(f"{args.command} requires a goal")
            result = resolve_goal(plan, state, args.value, REPO_ROOT)
        if json_output:
            print(_text(result))
        elif isinstance(result, dict):
            if args.command == "next":
                checkpoint = result.get("checkpoint")
                if checkpoint:
                    print(f"{checkpoint['id']} — {checkpoint['title']}")
                    print(f"phase: {checkpoint['phase']}")
                    print(f"spec: {checkpoint['spec_document']}")
                else:
                    print(result.get("message", "all checkpoints are complete"))
            elif args.command == "status":
                print(f"current NEXT: {result['current_next'] or 'none'}")
                for phase_id, status in result["phase_status"].items():
                    print(f"phase {phase_id}: {status}")
            elif args.command == "goal":
                print(
                    f"{result['goal']} -> {result['checkpoint_id']} — {result['title']} "
                    f"(phase {result['phase']})"
                )
            elif args.command == "context":
                checkpoint = result["checkpoint"]
                print(f"{checkpoint['id']} — {checkpoint['title']}")
                print(_text(checkpoint))
            else:
                print("execution plan valid")
        return 0
    except (PlanError, OSError) as exc:
        print(f"FAIL: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
