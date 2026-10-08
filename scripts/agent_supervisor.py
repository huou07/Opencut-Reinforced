#!/usr/bin/env python3
"""Run fresh checkpoint runners and complete them only after hosted evidence."""

from __future__ import annotations

import argparse
import ast
import copy
import datetime as _datetime
import hashlib
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
    "docs/execution/AMENDMENT_BASELINE.json",
    "docs/execution/DELEGATION.json",
    "docs/execution/MISSION.json",
}
AMENDMENT_MARKER = "docs/execution/AMENDMENT_BASELINE.json"
DELEGATION_MARKER = "docs/execution/DELEGATION.json"
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
RESUME_FOLLOWUP_CONTROL_PATHS = {
    "AGENTS.md",
    "docs/ARCHITECTURE.md",
    "docs/execution/AGENT_EXECUTION.md",
    "docs/execution/MISSION.json",
    "docs/execution/README.md",
    "docs/execution/PLAN.json",
    "docs/execution/EVIDENCE_POLICY.json",
    "docs/INDEX.md",
    "docs/OPEN_SOURCE_CONVERGENCE.md",
    "docs/PRODUCT.md",
    "docs/PRODUCT_ROADMAP.md",
    "docs/ROADMAP.md",
    "docs/TECHNICAL_PLAN.md",
    "scripts/agent_supervisor.py",
    "scripts/execution_plan.py",
    "scripts/test_execution_infra.py",
    DELEGATION_MARKER,
}


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


def allow_additive_evidence_policy_change(
    before: Mapping[str, bytes | None],
    repo_root: Path,
    plan: Mapping[str, Any],
    checkpoint_ids: Sequence[str],
) -> list[str]:
    path = "docs/execution/EVIDENCE_POLICY.json"
    prior_bytes = before.get(path)
    current_bytes = (repo_root / path).read_bytes()
    if prior_bytes == current_bytes:
        return []
    if prior_bytes is None:
        raise SupervisorError("runner cannot create the evidence policy")
    try:
        validate_evidence_policy_additions(
            json.loads(prior_bytes), json.loads(current_bytes), plan, checkpoint_ids
        )
        execution_evidence.load_policy(repo_root / path)
    except (json.JSONDecodeError, execution_evidence.EvidenceError) as exc:
        raise SupervisorError(f"runner added an invalid evidence binding: {exc}") from exc
    return [path]


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


def advance_state_batch(
    before: dict[str, Any],
    plan: dict[str, Any],
    checkpoint_ids: Sequence[str],
    *,
    candidate_contract_versions: Mapping[str, Any] | None = None,
    policy: Mapping[str, Any] | None = None,
    repo_root: Path = REPO_ROOT,
    today: str | None = None,
) -> dict[str, Any]:
    """Close an evidence-verified dependency-closed set in one state update."""

    ids = list(checkpoint_ids)
    if not ids or len(set(ids)) != len(ids):
        raise SupervisorError("completion batch must contain unique checkpoint ids")
    plan_ids = [item["id"] for item in plan["checkpoints"]]
    if any(checkpoint_id not in plan_ids for checkpoint_id in ids):
        raise SupervisorError("completion batch contains a checkpoint absent from PLAN.json")
    selected = set(ids)
    statuses = before.get("checkpoints")
    if not isinstance(statuses, dict):
        raise SupervisorError("state checkpoints are not an object")
    next_ids = [key for key, value in statuses.items() if value == "NEXT"]
    if len(next_ids) != 1 or next_ids[0] != before.get("current_next"):
        raise SupervisorError("state must have exactly one NEXT matching current_next")
    for checkpoint_id in ids:
        if statuses.get(checkpoint_id) not in {"NEXT", "PLANNED"}:
            raise SupervisorError(f"checkpoint {checkpoint_id} is not incomplete")
        checkpoint = execution_plan.checkpoint_for_id(plan, checkpoint_id)
        missing = [
            prerequisite
            for prerequisite in execution_plan._technical_dependencies(checkpoint)
            if statuses.get(prerequisite) != "DONE" and prerequisite not in selected
        ]
        if missing:
            raise SupervisorError(
                f"checkpoint {checkpoint_id} has unmet prerequisites: {', '.join(missing)}"
            )

    candidate = (
        execution_plan.read_contract_versions(repo_root)
        if candidate_contract_versions is None
        else execution_plan.validate_contract_versions(
            candidate_contract_versions, "candidate contract versions"
        )
    )
    transition_policy = (
        execution_plan.load_architecture_policy(repo_root) if policy is None else policy
    )
    previous = execution_plan.validate_contract_versions(
        before.get("verified_contract_versions"), "STATE.verified_contract_versions"
    )
    try:
        candidate = validate_batch_contract_transition(
            plan, before, transition_policy, candidate, ids
        )
    except execution_plan.PlanError as exc:
        raise SupervisorError(str(exc)) from exc

    after = copy.deepcopy(before)
    was_current_selected = next_ids[0] in selected
    for checkpoint_id in selected:
        after["checkpoints"][checkpoint_id] = "DONE"
    remaining = [
        checkpoint["id"]
        for checkpoint in plan["checkpoints"]
        if after["checkpoints"].get(checkpoint["id"]) != "DONE"
    ]
    if was_current_selected:
        for checkpoint_id in after["checkpoints"]:
            if after["checkpoints"][checkpoint_id] == "NEXT":
                after["checkpoints"][checkpoint_id] = "PLANNED"
        if remaining:
            after["checkpoints"][remaining[0]] = "NEXT"
            after["current_next"] = remaining[0]
        else:
            after["current_next"] = None
    after["last_updated"] = today or _datetime.date.today().isoformat()
    after["phase_status"] = execution_plan.derive_phase_statuses(
        plan, after["checkpoints"], repo_root
    )
    after["verified_contract_versions"] = dict(candidate)
    return after


def validate_batch_contract_transition(
    plan: dict[str, Any],
    state: Mapping[str, Any],
    policy: Mapping[str, Any],
    candidate_versions: Mapping[str, Any],
    checkpoint_ids: Sequence[str],
) -> execution_plan.ContractVersions:
    """Allow one selected gate to own a version change in a coherent batch."""

    verified = execution_plan.validate_contract_versions(
        state.get("verified_contract_versions"), "STATE.verified_contract_versions"
    )
    candidate = execution_plan.validate_contract_versions(
        candidate_versions, "candidate contract versions"
    )
    changed = [key for key in verified if candidate[key] != verified[key]]
    if not changed:
        return candidate
    rules = execution_plan.validate_contract_transition_policy(policy)
    if "recovery_schema" in changed:
        raise execution_plan.PlanError(
            "recovery schema changes require their own explicit owner and cannot be batched"
        )
    owners: set[str] = set()
    for version_key in changed:
        if version_key == "recovery_schema":
            continue
        category_key = (
            "expected_project_schema_effect_category"
            if version_key == "project_schema"
            else "expected_ipc_effect_category"
        )
        owner_categories = set(rules[version_key]["owner_categories"])
        matches = [
            checkpoint_id
            for checkpoint_id in checkpoint_ids
            if execution_plan.checkpoint_for_id(plan, checkpoint_id).get(category_key)
            in owner_categories
        ]
        if len(matches) != 1:
            raise execution_plan.PlanError(
                f"{version_key} version change needs exactly one selected contract owner"
            )
        owners.add(matches[0])
    if len(owners) != 1:
        raise execution_plan.PlanError(
            "a batch cannot combine version changes owned by different checkpoints"
        )
    return execution_plan.validate_contract_transition(
        plan,
        state,
        policy,
        candidate,
        checkpoint_id=next(iter(owners)),
    )


def validate_state_commit_paths(paths: Sequence[str], checkpoint_ids: str | Sequence[str]) -> None:
    """Allow only STATE.json and the matching supervisor evidence records."""

    ids = [checkpoint_ids] if isinstance(checkpoint_ids, str) else list(checkpoint_ids)
    expected = {"docs/execution/STATE.json"} | {
        f"docs/execution/evidence/{checkpoint_id}.json" for checkpoint_id in ids
    }
    actual = {path.replace("\\", "/") for path in paths}
    if actual != expected:
        raise SupervisorError(
            "state completion commit must contain exactly STATE.json and the selected evidence files; "
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
    resume_is_ancestor: bool = False,
) -> None:
    if execution_evidence.SHA_PATTERN.fullmatch(resume_sha) is None:
        raise SupervisorError("--resume-sha must be a lowercase 40-character SHA")
    if not worktree_clean:
        raise SupervisorError("resume requires a clean worktree")
    if head != origin_main:
        raise SupervisorError("resume requires HEAD == origin/main")
    if resume_sha != head and not resume_is_ancestor:
        raise SupervisorError("resume SHA must equal HEAD or be its verified ancestor")
    if current_next != expected_checkpoint:
        raise SupervisorError("resume requires the same checkpoint to remain NEXT")
    if not baseline_is_ancestor:
        raise SupervisorError("resume SHA is not a descendant of the prior state baseline")


def validate_historical_resume_paths(paths: Sequence[str]) -> None:
    forbidden = set(paths) - RESUME_FOLLOWUP_CONTROL_PATHS
    if forbidden:
        raise SupervisorError(
            "historical resume is allowed only across control-plane changes; "
            "product files changed after the candidate: "
            + ", ".join(sorted(forbidden))
        )


def validate_historical_plan_compatibility(
    candidate_plan: Mapping[str, Any], current_plan: Mapping[str, Any]
) -> None:
    """Allow dependency/order corrections while pinning every product contract."""

    if set(candidate_plan) != set(current_plan):
        raise SupervisorError("historical resume plan changed its top-level contract")
    for key in candidate_plan:
        if key != "checkpoints" and candidate_plan[key] != current_plan[key]:
            raise SupervisorError(f"historical resume changed plan contract {key}")
    old_checkpoints = {
        checkpoint["id"]: checkpoint for checkpoint in candidate_plan.get("checkpoints", [])
    }
    new_checkpoints = {
        checkpoint["id"]: checkpoint for checkpoint in current_plan.get("checkpoints", [])
    }
    if set(old_checkpoints) != set(new_checkpoints):
        raise SupervisorError("historical resume changed the requirement ID inventory")
    mutable_graph_fields = {
        "prerequisite_checkpoint_ids",
        "technical_dependency_checkpoint_ids",
        "next_checkpoint_relation",
    }
    for checkpoint_id, old in old_checkpoints.items():
        old_contract = {key: value for key, value in old.items() if key not in mutable_graph_fields}
        new = new_checkpoints[checkpoint_id]
        new_contract = {key: value for key, value in new.items() if key not in mutable_graph_fields}
        if old_contract != new_contract:
            raise SupervisorError(
                f"historical resume changed the product/evidence contract for {checkpoint_id}"
            )


def validate_evidence_policy_additions(
    before: Mapping[str, Any],
    after: Mapping[str, Any],
    plan: Mapping[str, Any],
    checkpoint_ids: Sequence[str],
) -> None:
    """Permit only additive evidence bindings for explicitly selected requirements."""

    if set(before) != set(after):
        raise SupervisorError("historical resume changed evidence policy fields")
    for key in before:
        if key != "evidence_class_proofs" and before[key] != after[key]:
            raise SupervisorError(f"historical resume changed evidence policy {key}")
    previous = before.get("evidence_class_proofs")
    current = after.get("evidence_class_proofs")
    if not isinstance(previous, dict) or not isinstance(current, dict):
        raise SupervisorError("evidence policy class proofs must be objects")
    if any(current.get(key) != value for key, value in previous.items()):
        raise SupervisorError("historical resume removed or changed an existing evidence binding")
    additions = set(current) - set(previous)
    selected = set(checkpoint_ids)
    if not additions <= selected:
        raise SupervisorError(
            "historical resume added evidence bindings outside the selected requirements: "
            + ", ".join(sorted(additions - selected))
        )
    for checkpoint_id in additions:
        checkpoint = execution_plan.checkpoint_for_id(dict(plan), checkpoint_id)
        classes = current[checkpoint_id]
        if not isinstance(classes, dict):
            raise SupervisorError(f"evidence bindings for {checkpoint_id} must be an object")
        required = set(checkpoint["required_evidence_classes"])
        if not required <= set(classes):
            raise SupervisorError(
                f"evidence bindings for {checkpoint_id} omit required classes: "
                + ", ".join(sorted(required - set(classes)))
            )
        for class_name, proofs in classes.items():
            if not isinstance(proofs, list) or not proofs:
                raise SupervisorError(
                    f"evidence binding {checkpoint_id}.{class_name} must name a proof"
                )


def validate_resume_supervisor_extension(before: bytes, after: bytes) -> None:
    """Allow only the additive proof-policy hook, preserving existing guards."""

    try:
        old_tree = ast.parse(before.decode("utf-8"))
        new_tree = ast.parse(after.decode("utf-8"))
    except (UnicodeDecodeError, SyntaxError) as exc:
        raise SupervisorError("resume supervisor extension is not valid Python") from exc

    def definitions(tree: ast.Module) -> dict[str, ast.AST]:
        return {
            node.name: node
            for node in tree.body
            if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef, ast.ClassDef))
        }

    old_defs = definitions(old_tree)
    new_defs = definitions(new_tree)
    new_names = set(new_defs) - set(old_defs)
    changed_names = {
        name
        for name in set(old_defs) & set(new_defs)
        if ast.dump(old_defs[name], include_attributes=False)
        != ast.dump(new_defs[name], include_attributes=False)
    }
    if new_names != {
        "allow_additive_evidence_policy_change",
        "validate_resume_supervisor_extension",
    }:
        raise SupervisorError("resume supervisor added an unexpected control helper")
    if changed_names != {"_resume_baseline", "checkpoint_prompt", "run_goal"}:
        raise SupervisorError("resume supervisor changed an existing control validator")

    def non_function_nodes(tree: ast.Module) -> list[str]:
        return [
            ast.dump(node, include_attributes=False)
            for node in tree.body
            if not isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef, ast.ClassDef))
            and not (
                isinstance(node, ast.Import)
                and len(node.names) == 1
                and node.names[0].name == "ast"
            )
        ]

    if non_function_nodes(old_tree) != non_function_nodes(new_tree):
        raise SupervisorError("resume supervisor changed imports or protected control constants")

    class RemoveEvidencePolicyHook(ast.NodeTransformer):
        removed = 0

        def visit_Assign(self, node: ast.Assign) -> ast.AST | None:
            value = node.value
            if (
                len(node.targets) == 1
                and isinstance(node.targets[0], ast.Name)
                and node.targets[0].id == "allowed_paths"
                and isinstance(value, ast.List)
                and len(value.elts) == 2
                and isinstance(value.elts[0], ast.Starred)
                and isinstance(value.elts[0].value, ast.Name)
                and value.elts[0].value.id == "allowed_paths"
                and isinstance(value.elts[1], ast.Starred)
                and isinstance(value.elts[1].value, ast.Call)
                and isinstance(value.elts[1].value.func, ast.Name)
                and value.elts[1].value.func.id == "allow_additive_evidence_policy_change"
            ):
                self.removed += 1
                return None
            return self.generic_visit(node)

    candidate_run_goal = copy.deepcopy(new_defs["run_goal"])
    remover = RemoveEvidencePolicyHook()
    remover.visit(candidate_run_goal)
    if remover.removed != 1 or ast.dump(
        candidate_run_goal, include_attributes=False
    ) != ast.dump(old_defs["run_goal"], include_attributes=False):
        raise SupervisorError("resume supervisor changed run authorization or verification logic")

    class RemoveResumeGuards(ast.NodeTransformer):
        removed: set[str] = set()

        def visit_If(self, node: ast.If) -> ast.AST | None:
            condition = ast.unparse(node.test)
            body = ast.unparse(node)
            if condition == "'docs/execution/EVIDENCE_POLICY.json' in changed":
                if "allowed.add('docs/execution/EVIDENCE_POLICY.json')" in body:
                    self.removed.add("policy")
                    return None
            if condition == "'scripts/agent_supervisor.py' in protected":
                if (
                    "validate_resume_supervisor_extension" not in body
                    or "protected.remove('scripts/agent_supervisor.py')" not in body
                ):
                    raise SupervisorError("resume supervisor allowance is malformed")
                self.removed.add("supervisor")
                return None
            if condition == "'AGENTS.md' in protected":
                if (
                    "all existing bindings and other policy fields remain immutable" not in body
                    or "protected.remove('AGENTS.md')" not in body
                ):
                    raise SupervisorError("resume AGENTS.md allowance is malformed")
                self.removed.add("agents")
                return None
            node = self.generic_visit(node)
            if isinstance(node.test, ast.Name) and node.test.id == "protected" and not node.body:
                return None
            return node

    candidate_resume = copy.deepcopy(new_defs["_resume_baseline"])
    resume_guards = RemoveResumeGuards()
    resume_guards.visit(candidate_resume)
    if resume_guards.removed != {"policy", "supervisor", "agents"} or ast.dump(
        candidate_resume, include_attributes=False
    ) != ast.dump(old_defs["_resume_baseline"], include_attributes=False):
        raise SupervisorError("resume supervisor changed baseline or protected-path rules")

    prompt_source = ast.get_source_segment(after.decode("utf-8"), new_defs["checkpoint_prompt"]) or ""
    if (
        "EVIDENCE_POLICY.json may only add named proof bindings" not in prompt_source
        or "all prior bindings and policy fields are immutable" not in prompt_source
    ):
        raise SupervisorError("resume supervisor prompt does not preserve evidence-policy immutability")
    helper = new_defs["allow_additive_evidence_policy_change"]
    calls = {
        node.func.id
        if isinstance(node.func, ast.Name)
        else ast.unparse(node.func)
        for node in ast.walk(helper)
        if isinstance(node, ast.Call)
    }
    path_assignments = [
        node
        for node in ast.walk(helper)
        if isinstance(node, ast.Assign)
        and any(isinstance(target, ast.Name) and target.id == "path" for target in node.targets)
        and isinstance(node.value, ast.Constant)
        and node.value.value == "docs/execution/EVIDENCE_POLICY.json"
    ]
    evidence_returns = [
        node
        for node in ast.walk(helper)
        if isinstance(node, ast.Return)
        and isinstance(node.value, ast.List)
        and len(node.value.elts) == 1
        and isinstance(node.value.elts[0], ast.Name)
        and node.value.elts[0].id == "path"
    ]
    if (
        "validate_evidence_policy_additions" not in calls
        or "execution_evidence.load_policy" not in calls
        or len(path_assignments) != 1
        or len(evidence_returns) != 1
    ):
        raise SupervisorError("resume supervisor evidence hook is not additive-only")


def checkpoint_prompt(
    repo_root: Path,
    resolution: dict[str, Any],
    checkpoint_ids: Sequence[str] | None = None,
) -> str:
    checkpoint_ids = list(checkpoint_ids or [resolution["checkpoint_id"]])
    batched = len(checkpoint_ids) > 1
    target = "\n".join(f"  - {item}" for item in checkpoint_ids)
    allowed_paths = resolution.get("runner_allowed_protected_paths", [])
    if allowed_paths:
        protection = (
            "Do not edit PLAN.json, STATE.json, architecture invariants "
            "or policy, phase specs, execution supervisor/validator/evidence files, or completion "
            "evidence. This checkpoint authorizes changes to exactly this protected workflow path:\n"
            + "".join(f"  - {path}\n" for path in allowed_paths)
            + "EVIDENCE_POLICY.json may only add named proof bindings for this selected requirement set; "
            "all prior bindings and policy fields are immutable and the supervisor validates each addition.\n"
            + "No other protected execution-control surface may change; PLAN.json and STATE.json "
            "remain immutable.\n"
        )
    else:
        protection = (
            "Do not edit PLAN.json, STATE.json, architecture invariants "
            "or policy, phase specs, execution supervisor/validator/evidence files, completion "
            "evidence, or protected workflow gates.\n"
            "EVIDENCE_POLICY.json may only add named proof bindings for this selected requirement set; "
            "all prior bindings and policy fields are immutable and the supervisor validates each addition.\n"
        )
    return (
        "You are executing an externally authorized Opencut Reinforced product scope.\n"
        f"Repository root: {repo_root}\n"
        f"Goal: {resolution['goal']}\n"
        f"Checkpoint: {resolution['checkpoint_id']} — {resolution['title']}\n"
        f"Current specification: {resolution['spec_document']}\n"
        f"Requirements selected for this verified implementation: {target}\n\n"
        + (
            "These requirement IDs are traceability labels, not separate job boundaries. "
            "Implement one coherent product change across them, preserving their full "
            "requirements and every real dependency. Do not edit PLAN, STATE, evidence, "
            "policy, or locked contracts.\n"
            if batched
            else "Implement exactly this selected requirement.\n"
        )
        + "You are not rewarded for the smallest implementation that makes existing tests green.\n"
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


def preflight_goal(
    repo_root: Path, goal: str, requirement_ids: Sequence[str] | None = None
) -> str:
    """Read-only preflight; it does not fetch, poll GitHub, write, or invoke a runner."""

    try:
        preflight = _preflight_goal_data(repo_root, goal)
        selected = resolve_completion_batch(
            preflight["plan"],
            preflight["state"],
            preflight["resolution"],
            requirement_ids,
        )
        return preflight["report"] + f"\nselected requirements: {', '.join(selected)}"
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


def _validate_candidate_batch_transition(
    plan: dict[str, Any],
    state: Mapping[str, Any],
    policy: Mapping[str, Any],
    repo_root: Path,
    checkpoint_ids: Sequence[str],
    revision: str | None = None,
) -> execution_plan.ContractVersions:
    try:
        candidate = execution_plan.read_contract_versions(repo_root, revision)
        return validate_batch_contract_transition(
            plan, state, policy, candidate, checkpoint_ids
        )
    except execution_plan.PlanError as exc:
        raise SupervisorError(str(exc)) from exc


def prepare_goal(
    repo_root: Path, goal: str, requirement_ids: Sequence[str] | None = None
) -> str:
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
    selected = resolve_completion_batch(plan, state, resolution, requirement_ids)
    return checkpoint_prompt(repo_root, resolution, selected)


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


def _run_pre_host_checks(
    repo_root: Path,
    implementation_sha: str,
    *,
    checkout_sha: str | None = None,
) -> None:
    """Run headless Rust tests before spending time on hosted CI verification."""

    checkout_sha = checkout_sha or implementation_sha
    if git_output(repo_root, "rev-parse", "HEAD") != checkout_sha:
        raise SupervisorError("checkout changed before pre-host verification")
    if git_output(repo_root, "rev-parse", "origin/main") != checkout_sha:
        raise SupervisorError("origin/main changed before pre-host verification")
    if implementation_sha != checkout_sha:
        try:
            subprocess.run(
                ["git", "merge-base", "--is-ancestor", implementation_sha, checkout_sha],
                cwd=repo_root,
                check=True,
                capture_output=True,
            )
        except subprocess.CalledProcessError as exc:
            raise SupervisorError("historical implementation is not an ancestor of checkout") from exc
    try:
        subprocess.run(["cargo", "test", "--workspace"], cwd=repo_root, check=True)
    except subprocess.CalledProcessError as exc:
        raise SupervisorError("pre-host verification failed: cargo test --workspace") from exc
    refuse_dirty_worktree(repo_root)
    if git_output(repo_root, "rev-parse", "HEAD") != checkout_sha:
        raise SupervisorError("HEAD changed during pre-host verification")
    if git_output(repo_root, "rev-parse", "origin/main") != checkout_sha:
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
    checkout_sha: str | None = None,
    api: execution_evidence.GitHubApi,
    run_local_checks: bool = True,
) -> dict[str, Any]:
    return finalize_verified_checkpoints(
        repo_root,
        plan=plan,
        state=state,
        checkpoints=[checkpoint],
        evidence_results=[evidence_result],
        implementation_sha=implementation_sha,
        checkout_sha=checkout_sha,
        api=api,
        run_local_checks=run_local_checks,
    )


def finalize_verified_checkpoints(
    repo_root: Path,
    *,
    plan: dict[str, Any],
    state: dict[str, Any],
    checkpoints: Sequence[Mapping[str, Any]],
    evidence_results: Sequence[Mapping[str, Any]],
    implementation_sha: str,
    checkout_sha: str | None = None,
    api: execution_evidence.GitHubApi,
    run_local_checks: bool = True,
) -> dict[str, Any]:
    ids = [str(checkpoint["id"]) for checkpoint in checkpoints]
    if not ids or len(set(ids)) != len(ids) or len(ids) != len(evidence_results):
        raise SupervisorError("verified checkpoint/evidence batch is empty or inconsistent")
    records: dict[str, Mapping[str, Any]] = {}
    policy: Mapping[str, Any] | None = None
    for checkpoint, evidence_result in zip(checkpoints, evidence_results):
        checkpoint_id = str(checkpoint["id"])
        policy = evidence_result["policy"]
        record = evidence_result["record"]
        if checkpoint_id in records:
            raise SupervisorError(f"duplicate completion evidence for {checkpoint_id}")
        if record.get("implementation_sha") != implementation_sha:
            raise SupervisorError(
                f"evidence implementation SHA does not match verified SHA for {checkpoint_id}"
            )
        execution_evidence.validate_evidence_record(
            record,
            checkpoint_id=checkpoint_id,
            checkpoint=checkpoint,
            policy=policy,
        )
        records[checkpoint_id] = record
    checkout_sha = checkout_sha or implementation_sha
    if git_output(repo_root, "rev-parse", "HEAD") != checkout_sha:
        raise SupervisorError("verification checkout changed before evidence finalization")
    first_record = records[ids[0]]
    if "contract_versions" not in first_record:
        raise SupervisorError("new evidence record is missing contract_versions")
    try:
        candidate_versions = execution_plan.validate_contract_versions(
            first_record["contract_versions"], "evidence contract_versions"
        )
        exact_versions = execution_plan.read_contract_versions(repo_root, implementation_sha)
        if candidate_versions != exact_versions:
            raise SupervisorError("evidence contract versions do not match the verified tree")
    except execution_plan.PlanError as exc:
        raise SupervisorError(str(exc)) from exc
    if any(record.get("contract_versions") != candidate_versions for record in records.values()):
        raise SupervisorError("batch evidence records disagree on contract versions")
    architecture_policy = execution_plan.load_architecture_policy(repo_root)
    state_path = repo_root / "docs" / "execution" / "STATE.json"
    evidence_paths = {
        checkpoint_id: repo_root / "docs" / "execution" / "evidence" / f"{checkpoint_id}.json"
        for checkpoint_id in ids
    }
    original_state = state_path.read_bytes()
    for checkpoint_id, evidence_path in evidence_paths.items():
        if evidence_path.exists():
            raise SupervisorError(f"completion evidence already exists: {evidence_path}")
    after_state = advance_state_batch(
        state,
        plan,
        ids,
        candidate_contract_versions=exact_versions,
        policy=architecture_policy,
        repo_root=repo_root,
    )
    for checkpoint_id, evidence_path in evidence_paths.items():
        _write_json(evidence_path, records[checkpoint_id])
    _write_json(state_path, after_state)
    commit_created = False
    try:
        if run_local_checks:
            _run_local_completion_checks(repo_root)
        git_output(repo_root, "fetch", "--prune", "origin")
        if git_output(repo_root, "rev-parse", "origin/main") != checkout_sha:
            raise SupervisorError("origin/main moved after implementation verification")
        changed_paths = git_status_paths(repo_root)
        validate_state_commit_paths(changed_paths, ids)
        subprocess.run(
            ["git", "add", str(state_path), *(str(path) for path in evidence_paths.values())],
            cwd=repo_root,
            check=True,
        )
        staged = git_output(repo_root, "diff", "--cached", "--name-only").splitlines()
        validate_state_commit_paths(staged, ids)
        assert policy is not None
        subject = (
            _state_commit_subject(policy, ids[0])
            if len(ids) == 1
            else f"chore(execution): record verified requirements {','.join(ids)}"
        )
        subprocess.run(["git", "commit", "-m", subject], cwd=repo_root, check=True)
        commit_created = True
        subprocess.run(["git", "push", "origin", "main"], cwd=repo_root, check=True)
    except Exception:
        if not commit_created:
            subprocess.run(
                ["git", "reset", "--", str(state_path), *(str(path) for path in evidence_paths.values())],
                cwd=repo_root,
                check=False,
            )
            state_path.write_bytes(original_state)
            for evidence_path in evidence_paths.values():
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
        "checkpoint": dict(checkpoints[0]),
        "checkpoints": [dict(checkpoint) for checkpoint in checkpoints],
        "implementation_sha": implementation_sha,
        "implementation_subject": first_record["implementation_subject"],
        "record": first_record,
        "records": dict(records),
        "after_state": after_state,
        "before_state_current_next": state.get("current_next"),
        "state_commit_sha": state_commit_sha,
        "state_commit_subject": subject,
        "state_commit_hygiene": hygiene,
    }


def _resume_baseline(
    repo_root: Path,
    resume_sha: str,
    allowed_paths: Sequence[str] = (),
    checkpoint_ids: Sequence[str] = (),
    delegation_goal: str | None = None,
) -> str:
    # Exclude the resume commit when locating the prior state baseline so a
    # forbidden STATE.json edit in that implementation commit remains visible
    # in the protected-path diff below.
    baseline = _delegation_resume_baseline(repo_root, resume_sha, delegation_goal)
    if baseline is None:
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
    if "docs/execution/EVIDENCE_POLICY.json" in changed:
        old_policy = _json_at_revision(
            repo_root, baseline, "docs/execution/EVIDENCE_POLICY.json"
        )
        new_policy = _json_at_revision(
            repo_root, resume_sha, "docs/execution/EVIDENCE_POLICY.json"
        )
        plan = _json_at_revision(repo_root, resume_sha, "docs/execution/PLAN.json")
        validate_evidence_policy_additions(
            old_policy, new_policy, plan, checkpoint_ids
        )
    allowed = set(allowed_paths)
    if "docs/execution/EVIDENCE_POLICY.json" in changed:
        allowed.add("docs/execution/EVIDENCE_POLICY.json")
    protected = [
        path
        for path in changed
        if is_protected_execution_path(path) and path not in allowed
    ]
    if protected:
        if "scripts/agent_supervisor.py" in protected:
            validate_resume_supervisor_extension(
                _git_file_bytes(repo_root, baseline, "scripts/agent_supervisor.py"),
                _git_file_bytes(repo_root, resume_sha, "scripts/agent_supervisor.py"),
            )
            protected.remove("scripts/agent_supervisor.py")
        if "AGENTS.md" in protected:
            agent_rules = _git_file_bytes(repo_root, resume_sha, "AGENTS.md").decode("utf-8")
            if (
                "EVIDENCE_POLICY.json` may only gain named proof bindings for selected" not in agent_rules
                or "all existing bindings and other policy fields remain immutable" not in agent_rules
                or "the supervisor validates every addition" not in agent_rules
            ):
                raise SupervisorError("resume changed AGENTS.md outside the additive evidence rule")
            protected.remove("AGENTS.md")
        if "scripts/test_execution_infra.py" in protected:
            validate_resume_test_additions(
                _git_file_bytes(repo_root, baseline, "scripts/test_execution_infra.py"),
                _git_file_bytes(repo_root, resume_sha, "scripts/test_execution_infra.py"),
            )
            protected.remove("scripts/test_execution_infra.py")
    if protected:
        raise SupervisorError(
            "resume implementation changed protected execution-control files: "
            + ", ".join(protected)
        )
    return baseline


def validate_resume_test_additions(before: bytes, after: bytes) -> None:
    """Allow new resume-guard tests while preserving every existing test exactly."""

    try:
        old_tree = ast.parse(before.decode("utf-8"))
        new_tree = ast.parse(after.decode("utf-8"))
    except (UnicodeDecodeError, SyntaxError) as exc:
        raise SupervisorError("resume test extension is not valid Python") from exc

    old_classes = {
        node.name: node for node in old_tree.body if isinstance(node, ast.ClassDef)
    }
    new_classes = {
        node.name: node for node in new_tree.body if isinstance(node, ast.ClassDef)
    }
    if set(old_classes) != set(new_classes):
        raise SupervisorError("resume test extension changed test classes")

    def module_nodes(tree: ast.Module) -> list[str]:
        return [
            ast.dump(node, include_attributes=False)
            for node in tree.body
            if not isinstance(node, ast.ClassDef)
        ]

    if module_nodes(old_tree) != module_nodes(new_tree):
        raise SupervisorError("resume test extension changed module-level guards")

    additions = 0
    for name, old_class in old_classes.items():
        new_class = new_classes[name]
        old_header, new_header = copy.copy(old_class), copy.copy(new_class)
        old_header.body = []
        new_header.body = []
        if ast.dump(old_header, include_attributes=False) != ast.dump(
            new_header, include_attributes=False
        ):
            raise SupervisorError("resume test extension changed a test class contract")

        old_methods = {
            node.name: node
            for node in old_class.body
            if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef))
        }
        new_methods = {
            node.name: node
            for node in new_class.body
            if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef))
        }
        old_other_nodes = [
            ast.dump(node, include_attributes=False)
            for node in old_class.body
            if not isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef))
        ]
        new_other_nodes = [
            ast.dump(node, include_attributes=False)
            for node in new_class.body
            if not isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef))
        ]
        if old_other_nodes != new_other_nodes:
            raise SupervisorError("resume test extension changed existing class guards")
        if not set(old_methods) <= set(new_methods):
            raise SupervisorError("resume test extension removed an existing test")
        for method_name, old_method in old_methods.items():
            if ast.dump(old_method, include_attributes=False) != ast.dump(
                new_methods[method_name], include_attributes=False
            ):
                raise SupervisorError("resume test extension changed an existing test")
        added_methods = set(new_methods) - set(old_methods)
        if any(not method_name.startswith("test_") for method_name in added_methods):
            raise SupervisorError("resume test extension may only add test methods")
        if added_methods and not any(
            isinstance(base, ast.Attribute)
            and base.attr == "TestCase"
            and isinstance(base.value, ast.Name)
            and base.value.id == "unittest"
            for base in old_class.bases
        ):
            raise SupervisorError("resume test methods must be added to unittest.TestCase classes")
        additions += len(added_methods)

    if additions == 0:
        raise SupervisorError("resume test extension did not add a test method")


def _control_baseline(repo_root: Path, revision: str) -> str:
    baseline = git_output(repo_root, "log", "-1", "--format=%H", revision, "--", "docs/execution/STATE.json")
    marker = git_output(repo_root, "log", "-1", "--format=%H", revision, "--", AMENDMENT_MARKER)
    if marker and git_output(repo_root, "merge-base", baseline, marker) == baseline:
        return marker
    return baseline


def _delegation_resume_baseline(
    repo_root: Path, resume_sha: str, goal: str | None
) -> str | None:
    """Select the pinned control baseline only while its immutable delegation is active."""

    head = git_output(repo_root, "rev-parse", "HEAD")
    marker_commit = git_output(
        repo_root, "log", "-1", "--format=%H", head, "--", DELEGATION_MARKER
    )
    if not marker_commit:
        return None

    record = _json_at_revision(repo_root, marker_commit, DELEGATION_MARKER)
    expected_keys = {
        "schema_version",
        "goal",
        "delegation_start_sha",
        "control_baseline_sha",
    }
    if set(record) != expected_keys or record.get("schema_version") != 1:
        raise SupervisorError("delegation record has an unsupported schema")
    record_goal = record.get("goal")
    delegation_start_sha = record.get("delegation_start_sha")
    control_baseline_sha = record.get("control_baseline_sha")
    if not isinstance(record_goal, str) or not record_goal:
        raise SupervisorError("delegation record goal is invalid")
    for value in (delegation_start_sha, control_baseline_sha):
        if not isinstance(value, str) or len(value) != 40 or value.lower() != value:
            raise SupervisorError("delegation record contains an invalid exact Git SHA")
        if any(character not in "0123456789abcdef" for character in value):
            raise SupervisorError("delegation record contains an invalid exact Git SHA")

    state_baseline = _control_baseline(repo_root, head)
    if _is_ancestor(repo_root, marker_commit, state_baseline):
        return None
    if goal != record_goal:
        return None

    candidate_state_baseline = _control_baseline(repo_root, f"{resume_sha}^")
    if candidate_state_baseline != state_baseline:
        raise SupervisorError("delegation resume does not match the current STATE baseline")
    for ancestor, descendant, message in (
        (delegation_start_sha, control_baseline_sha, "control baseline precedes delegation start"),
        (delegation_start_sha, state_baseline, "current STATE predates delegation start"),
        (state_baseline, control_baseline_sha, "control baseline is outside the active STATE history"),
        (control_baseline_sha, marker_commit, "delegation marker predates its control baseline"),
        (marker_commit, head, "delegation marker is not on current HEAD history"),
    ):
        if not _is_ancestor(repo_root, ancestor, descendant):
            raise SupervisorError(message)

    if _is_ancestor(repo_root, marker_commit, resume_sha):
        return marker_commit
    if _is_ancestor(repo_root, control_baseline_sha, resume_sha):
        return control_baseline_sha
    raise SupervisorError("resume SHA falls outside the active delegation history")


def _is_ancestor(repo_root: Path, ancestor: str, descendant: str) -> bool:
    return subprocess.run(
        ["git", "merge-base", "--is-ancestor", ancestor, descendant],
        cwd=repo_root,
        capture_output=True,
        check=False,
    ).returncode == 0


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
            prerequisites = execution_plan._technical_dependencies(checkpoint)
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
    checkpoints = result.get("checkpoints", [checkpoint])
    records = result.get("records", {checkpoint["id"]: record})
    gates = {gate["gate_id"]: gate for gate in record["gates"]}
    current_next = result["after_state"].get("current_next")
    next_checkpoint = execution_plan.checkpoint_for_id(plan, current_next) if current_next else None
    lines = [
        "## VERIFIED CHECKPOINT COMPLETION",
        f"checkpoint: {checkpoint['id']}",
        f"title: {checkpoint['title']}",
        "verified requirements: " + ", ".join(item["id"] for item in checkpoints),
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
        ]
    )
    for item in checkpoints:
        item_record = records[item["id"]]
        classes = ", ".join(
            proof["class"] for proof in item_record.get("evidence_classes", [])
        ) or "no additional class proofs"
        lines.append(
            f"evidence: docs/execution/evidence/{item['id']}.json; "
            f"implementation SHA {item_record['implementation_sha']}; classes: {classes}"
        )
    before_next = result.get("before_state_current_next")
    if before_next in records:
        lines.append(f"state: {before_next} NEXT → DONE")
    else:
        lines.append(f"state: NEXT cursor unchanged at {before_next or 'none'}")
    lines.extend(
        [
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
    checkout_sha: str | None = None,
    api: execution_evidence.GitHubApi | None = None,
    clock: Callable[[], float] = time.monotonic,
    sleep: Callable[[float], None] = time.sleep,
) -> dict[str, Any]:
    checkpoint = execution_plan.checkpoint_for_id(plan, resolution["checkpoint_id"])
    _run_pre_host_checks(repo_root, implementation_sha, checkout_sha=checkout_sha)
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
        checkout_sha=checkout_sha,
        api=evidence_result["api"],
    )


def _run_checkpoint_batch(
    repo_root: Path,
    *,
    plan: dict[str, Any],
    state: dict[str, Any],
    checkpoint_ids: Sequence[str],
    implementation_sha: str,
    implementation_origin_sha: str | None = None,
    checkout_sha: str | None = None,
    api: execution_evidence.GitHubApi | None = None,
    clock: Callable[[], float] = time.monotonic,
    sleep: Callable[[float], None] = time.sleep,
) -> dict[str, Any]:
    checkpoints = [execution_plan.checkpoint_for_id(plan, item) for item in checkpoint_ids]
    _run_pre_host_checks(repo_root, implementation_sha, checkout_sha=checkout_sha)
    subject = git_output(repo_root, "show", "-s", "--format=%s", implementation_sha)
    evidence_results = [
        verify_hosted_checkpoint(
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
        for checkpoint in checkpoints
    ]
    return finalize_verified_checkpoints(
        repo_root,
        plan=plan,
        state=state,
        checkpoints=checkpoints,
        evidence_results=evidence_results,
        implementation_sha=implementation_sha,
        checkout_sha=checkout_sha,
        api=evidence_results[0]["api"],
    )


def resolve_completion_batch(
    plan: Mapping[str, Any],
    state: Mapping[str, Any],
    resolution: Mapping[str, Any],
    requested_ids: Sequence[str] | None,
) -> list[str]:
    current = str(resolution["checkpoint_id"])
    ids = list(requested_ids or [current])
    if not ids or len(ids) != len(set(ids)):
        raise SupervisorError("--requirements must contain unique checkpoint ids")
    allowed = set(resolution["goal_checkpoint_ids"])
    unknown = [item for item in ids if item not in allowed]
    if unknown:
        raise SupervisorError(
            "requirements exceed the selected goal: " + ", ".join(unknown)
        )
    if state.get("checkpoints", {}).get(current) != "NEXT":
        raise SupervisorError("STATE.current_next does not identify the current NEXT checkpoint")
    statuses = state["checkpoints"]
    for checkpoint_id in ids:
        if statuses.get(checkpoint_id) not in {"NEXT", "PLANNED"}:
            raise SupervisorError(f"requirement {checkpoint_id} is not incomplete")
        checkpoint = execution_plan.checkpoint_for_id(dict(plan), checkpoint_id)
        missing = [
            item
            for item in execution_plan._technical_dependencies(checkpoint)
            if statuses.get(item) != "DONE" and item not in ids
        ]
        if missing:
            raise SupervisorError(
                f"requirement {checkpoint_id} is outside the dependency-closed batch; "
                f"unverified prerequisites: {', '.join(missing)}"
            )
    return ids


def enforce_active_mission_batch(
    repo_root: Path,
    checkpoint_ids: Sequence[str],
    explicitly_selected: bool,
) -> None:
    """Keep paused legacy execution bounded to the operator-authorized finish."""

    mission_path = repo_root / "docs" / "execution" / "MISSION.json"
    try:
        mission = json.loads(mission_path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise SupervisorError(f"cannot load active mission mode: {exc}") from exc
    if not isinstance(mission, dict) or mission.get("schema_version") != 1:
        raise SupervisorError("active mission mode has an unsupported schema")
    mode = mission.get("legacy_execution")
    if mode == "ACTIVE":
        return
    if mode != "PAUSED_AFTER_ALLOWED_REQUIREMENTS":
        raise SupervisorError("active mission mode does not authorize legacy execution")
    allowed = mission.get("finish_current_requirements")
    if (
        not isinstance(allowed, list)
        or not allowed
        or not all(isinstance(item, str) and item for item in allowed)
        or len(set(allowed)) != len(allowed)
    ):
        raise SupervisorError("active mission mode has an invalid finish-current allowlist")
    if not explicitly_selected or list(checkpoint_ids) != allowed:
        raise SupervisorError(
            "legacy roadmap execution is paused by the operator; explicitly select only "
            "the authorized in-flight requirements to finish this atomic boundary"
        )


def run_goal(
    repo_root: Path,
    goal: str,
    runner: str | None = None,
    resume_sha: str | None = None,
    requirement_ids: Sequence[str] | None = None,
) -> list[str]:
    ensure_start_state(repo_root)
    reports: list[str] = []
    resume_pending = resume_sha is not None
    was_resume = resume_pending
    batch_was_explicit = requirement_ids is not None
    while True:
        preflight = _preflight_goal_data(repo_root, goal)
        plan = preflight["plan"]
        state = preflight["state"]
        resolution = preflight["resolution"]
        batch_ids = resolve_completion_batch(plan, state, resolution, requirement_ids)
        if resume_pending:
            head = git_output(repo_root, "rev-parse", "HEAD")
            origin = git_output(repo_root, "rev-parse", "origin/main")
            allowed_paths = resolution["runner_allowed_protected_paths"]
            resume_is_ancestor = head != (resume_sha or "")
            if resume_is_ancestor:
                try:
                    subprocess.run(
                        ["git", "merge-base", "--is-ancestor", resume_sha or "", head],
                        cwd=repo_root,
                        check=True,
                        capture_output=True,
                        text=True,
                    )
                except subprocess.CalledProcessError as exc:
                    raise SupervisorError("resume SHA is not on origin/main history") from exc
                later_paths = set(
                    git_output(repo_root, "diff", "--name-only", f"{resume_sha}..{head}").splitlines()
                )
                validate_historical_resume_paths(sorted(later_paths))
                if "docs/execution/EVIDENCE_POLICY.json" in later_paths:
                    validate_evidence_policy_additions(
                        _json_at_revision(
                            repo_root, resume_sha, "docs/execution/EVIDENCE_POLICY.json"
                        ),
                        _json_at_revision(
                            repo_root, head, "docs/execution/EVIDENCE_POLICY.json"
                        ),
                        plan,
                        batch_ids,
                    )
                historical_plan = _json_at_revision(
                    repo_root, resume_sha or "", "docs/execution/PLAN.json"
                )
                validate_historical_plan_compatibility(historical_plan, plan)
                allowed_paths = sorted(set(allowed_paths) | RESUME_FOLLOWUP_CONTROL_PATHS)
            _resume_baseline(
                repo_root,
                resume_sha or "",
                allowed_paths,
                checkpoint_ids=batch_ids,
                delegation_goal=goal,
            )
            validate_resume_preconditions(
                resume_sha=resume_sha or "",
                head=head,
                origin_main=origin,
                current_next=state.get("current_next"),
                expected_checkpoint=resolution["checkpoint_id"],
                worktree_clean=not bool(git_output(repo_root, "status", "--porcelain")),
                baseline_is_ancestor=True,
                resume_is_ancestor=resume_is_ancestor,
            )
            _validate_candidate_batch_transition(
                plan, state, preflight["policy"], repo_root, batch_ids, resume_sha
            )
            if len(batch_ids) == 1 and batch_ids[0] == resolution["checkpoint_id"]:
                result = _run_one_checkpoint(
                    repo_root,
                    plan=plan,
                    state=state,
                    resolution=resolution,
                    implementation_sha=resume_sha or "",
                    checkout_sha=head,
                )
            else:
                result = _run_checkpoint_batch(
                    repo_root,
                    plan=plan,
                    state=state,
                    checkpoint_ids=batch_ids,
                    implementation_sha=resume_sha or "",
                    checkout_sha=head,
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
            invoke_runner(
                repo_root,
                runner,
                checkpoint_prompt(repo_root, resolution, batch_ids),
            )
            git_output(repo_root, "fetch", "--prune", "origin")
            if (repo_root / "docs/execution/PLAN.json").read_bytes() != before_plan_bytes:
                raise SupervisorError("runner changed immutable PLAN.json")
            if (repo_root / "docs/execution/STATE.json").read_bytes() != before_state_bytes:
                raise SupervisorError("runner changed STATE.json; only the supervisor may advance state")
            allowed_paths = [
                *allowed_paths,
                *allow_additive_evidence_policy_change(
                    protected_before, repo_root, plan, batch_ids
                ),
            ]
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
            _validate_candidate_batch_transition(
                plan, state, preflight["policy"], repo_root, batch_ids
            )
            if len(batch_ids) == 1 and batch_ids[0] == resolution["checkpoint_id"]:
                result = _run_one_checkpoint(
                    repo_root,
                    plan=plan,
                    state=state,
                    resolution=resolution,
                    implementation_sha=head,
                )
            else:
                result = _run_checkpoint_batch(
                    repo_root,
                    plan=plan,
                    state=state,
                    checkpoint_ids=batch_ids,
                    implementation_sha=head,
                )
        reports.append(_verified_report(result, plan))
        if batch_was_explicit or was_resume or resolution["goal_complete_after_current"]:
            return reports

def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--goal", required=True)
    parser.add_argument(
        "--requirements",
        help="comma-separated roadmap checkpoint IDs implemented coherently on this exact SHA",
    )
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
    args = parser.parse_args(argv)
    try:
        requirements = None
        if args.requirements is not None:
            requirements = [item.strip() for item in args.requirements.split(",") if item.strip()]
            if not requirements:
                raise SupervisorError("--requirements must contain at least one checkpoint id")
        if args.preflight:
            print(preflight_goal(REPO_ROOT, args.goal, requirements))
            return 0
        if args.prepare:
            print(prepare_goal(REPO_ROOT, args.goal, requirements), end="")
            return 0
        if args.runner or args.resume_sha or args.repair_resume_from:
            preflight = _preflight_goal_data(REPO_ROOT, args.goal)
            resolution = preflight["resolution"]
            if args.repair_resume_from:
                selected = [str(preflight["state"].get("current_next"))]
            else:
                selected = resolve_completion_batch(
                    preflight["plan"], preflight["state"], resolution, requirements
                )
            enforce_active_mission_batch(
                REPO_ROOT, selected, requirements is not None
            )
        if args.repair_resume_from:
            reports = repair_resume_goal(REPO_ROOT, args.goal, args.repair_resume_from)
        else:
            reports = run_goal(
                REPO_ROOT,
                args.goal,
                args.runner,
                args.resume_sha,
                requirements,
            )
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
