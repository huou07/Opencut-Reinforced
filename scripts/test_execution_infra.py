#!/usr/bin/env python3
"""Stdlib tests for the OR execution-plan infrastructure."""

from __future__ import annotations

import json
import os
import stat
import subprocess
import sys
import tempfile
import urllib.error
import unittest
from contextlib import redirect_stdout
from io import StringIO
from pathlib import Path
from unittest import mock

SCRIPT_DIR = Path(__file__).resolve().parent
REPO_ROOT = SCRIPT_DIR.parent
sys.path.insert(0, str(SCRIPT_DIR))

import agent_supervisor  # noqa: E402
import check_architecture_policy  # noqa: E402
import execution_evidence  # noqa: E402
import execution_plan  # noqa: E402


def checkpoint(checkpoint_id: str, prerequisites: list[str], next_id: str | None) -> dict[str, object]:
    return {
        "id": checkpoint_id,
        "phase": 6,
        "title": checkpoint_id,
        "spec_document": "docs/execution/phases/PHASE_6.md",
        "prerequisite_checkpoint_ids": prerequisites,
        "milestone_membership": ["test"],
        "user_visible": False,
        "developer_preview_required": False,
        "architecture_gate": False,
        "expected_project_schema_effect_category": "none",
        "expected_ipc_effect_category": "none",
        "dependency_change_policy": "No new dependency.",
        "next_checkpoint_relation": next_id,
    }


def fixture_plan_state(ids: list[str] = ["A", "B"]) -> tuple[dict[str, object], dict[str, object]]:
    plan_checkpoints = [
        checkpoint(ids[0], [], ids[1] if len(ids) > 1 else None),
    ]
    if len(ids) > 1:
        plan_checkpoints.append(checkpoint(ids[1], [ids[0]], None))
    plan = {
        "schema_version": 1,
        "plan_id": "test-plan",
        "authority_order": [],
        "milestones": {
            "test": {
                "title": "Test",
                "checkpoint_ids": ids,
                "completion_checkpoint_id": ids[-1],
            }
        },
        "phases": {
            "6": {
                "title": "Test phase",
                "spec_document": "docs/execution/phases/PHASE_6.md",
            }
        },
        "checkpoints": plan_checkpoints,
    }
    statuses = {ids[0]: "NEXT"}
    statuses.update({checkpoint_id: "PLANNED" for checkpoint_id in ids[1:]})
    state = {
        "schema_version": 1,
        "repository": "test",
        "last_updated": "2026-09-28",
        "architecture_execution_lock": "DONE",
        "phase_status": {"5": "DONE", "6": "IN_PROGRESS"},
        "current_next": ids[0],
        "checkpoints": statuses,
        "verified_contract_versions": {
            "project_schema": 7,
            "recovery_schema": 1,
            "ipc_protocol": 1,
        },
    }
    return plan, state


def contract_versions(project: int = 7, recovery: int = 1, ipc: int = 1) -> dict[str, int]:
    return {
        "project_schema": project,
        "recovery_schema": recovery,
        "ipc_protocol": ipc,
    }


def roadmap_state_before(
    plan: dict[str, object], checkpoint_id: str, verified: dict[str, int]
) -> dict[str, object]:
    checkpoints = plan["checkpoints"]  # type: ignore[index]
    statuses: dict[str, str] = {}
    reached = False
    for checkpoint_spec in checkpoints:
        current_id = checkpoint_spec["id"]
        if current_id == checkpoint_id:
            reached = True
            statuses[current_id] = "NEXT"
        else:
            statuses[current_id] = "PLANNED" if reached else "DONE"
    state: dict[str, object] = {
        "schema_version": 1,
        "repository": "Opencut Reinforced",
        "last_updated": "2026-09-30",
        "architecture_execution_lock": "DONE",
        "phase_status": {},
        "current_next": checkpoint_id,
        "checkpoints": statuses,
        "verified_contract_versions": dict(verified),
    }
    state["phase_status"] = execution_plan.derive_phase_statuses(
        plan, statuses, REPO_ROOT
    )
    return state


def seven_f_one_plan_state() -> tuple[dict[str, object], dict[str, object]]:
    """Retain historical 7F1 transition tests after the live plan moves on."""

    plan, _current = execution_plan.load_plan_state(REPO_ROOT)
    return plan, roadmap_state_before(plan, "7F1", contract_versions(project=5))


def nine_b_plan_state() -> tuple[dict[str, object], dict[str, object]]:
    """Retain the amended in-flight 9B scenario after completion advances STATE."""

    plan, _current = execution_plan.load_plan_state(REPO_ROOT)
    return plan, roadmap_state_before(plan, "9B", contract_versions(project=7))


def git_blob_from_main(revision: str, relative_path: str) -> bytes:
    return subprocess.run(
        ["git", "show", f"{revision}:{relative_path}"],
        cwd=REPO_ROOT,
        capture_output=True,
        check=True,
    ).stdout


def commit_test_files(root: Path, files: dict[str, bytes | str], subject: str) -> str:
    for relative_path, value in files.items():
        path = root / relative_path
        path.parent.mkdir(parents=True, exist_ok=True)
        if isinstance(value, bytes):
            path.write_bytes(value)
        else:
            path.write_text(value, encoding="utf-8")
    subprocess.run(["git", "add", "-A"], cwd=root, check=True)
    subprocess.run(["git", "commit", "-qm", subject], cwd=root, check=True)
    return subprocess.run(
        ["git", "rev-parse", "HEAD"],
        cwd=root,
        capture_output=True,
        text=True,
        check=True,
    ).stdout.strip()


def make_repair_history(
    root: Path,
    *,
    original_changes: dict[str, bytes | str] | None = None,
    failed_changes: dict[str, bytes | str] | None = None,
    previous_contract_active: bool = False,
    historical_state: bytes | str | None = None,
) -> tuple[str, str]:
    subprocess.run(["git", "init", "-q", "-b", "main", str(root)], check=True)
    subprocess.run(["git", "config", "user.name", "Execution Test"], cwd=root, check=True)
    subprocess.run(
        ["git", "config", "user.email", "execution-test@example.invalid"],
        cwd=root,
        check=True,
    )
    commit_test_files(root, {"README.md": b"bootstrap\n"}, "repository bootstrap")
    historical_state_value = (
        historical_state
        if historical_state is not None
        else git_blob_from_main(
            "76cecbc1d255084a1b5e04941cd62e321bd1c562", "docs/execution/STATE.json"
        )
    )
    commit_test_files(
        root,
        {
            "docs/execution/PLAN.json": git_blob_from_main(
                "76cecbc1d255084a1b5e04941cd62e321bd1c562", "docs/execution/PLAN.json"
            ),
            "docs/execution/STATE.json": historical_state_value,
        },
        "execution infrastructure adds state",
    )
    tracked = (
        "docs/execution/PLAN.json",
        "docs/execution/STATE.json",
        "docs/execution/architecture-policy.json",
        "docs/execution/AGENT_EXECUTION.md",
        "docs/execution/EVIDENCE_POLICY.json",
        "docs/execution/ARCHITECTURE_INVARIANTS.md",
        "docs/execution/phases/PHASE_7.md",
        "docs/execution/phases/PHASE_16.md",
        ".github/workflows/platform-verification.yml",
        ".github/workflows/developer-preview.yml",
        "crates/or_core/src/project_document.rs",
        "crates/or_core/src/project_recovery.rs",
        "crates/or_ipc/src/protocol.rs",
    )
    base_files = {
        path: (REPO_ROOT / path).read_bytes()
        for path in tracked
    }
    # This fixture exercises the historical 7F1 repair contract, so keep its
    # plan, state, and source versions at that revision as the roadmap advances.
    for path in (
        "docs/execution/PLAN.json",
        "docs/execution/STATE.json",
        "crates/or_core/src/project_document.rs",
        "crates/or_core/src/project_recovery.rs",
        "crates/or_ipc/src/protocol.rs",
    ):
        base_files[path] = git_blob_from_main("ed1633a", path)
    base_files["docs/execution/phases/PHASE_8.md"] = (
        REPO_ROOT / "docs/execution/phases/PHASE_8.md"
    ).read_bytes()
    if original_changes:
        base_files.update(original_changes)
    if previous_contract_active:
        previous_files = dict(base_files)
        previous_plan = json.loads(previous_files["docs/execution/PLAN.json"])
        execution_plan.checkpoint_for_id(previous_plan, "7F1")["title"] = "Earlier 7F1 contract"
        previous_files["docs/execution/PLAN.json"] = json.dumps(previous_plan, indent=2) + "\n"
        commit_test_files(root, previous_files, "trusted maintenance changes active contract")
    commit_test_files(root, base_files, "final 7F1 state baseline")

    platform_workflow = ".github/workflows/platform-verification.yml"
    failed_files: dict[str, bytes | str] = {
        platform_workflow: (REPO_ROOT / platform_workflow).read_bytes()
        + b"\n# failed gate fixture\n"
    }
    if failed_changes:
        failed_files.update(failed_changes)
    failed_sha = commit_test_files(root, failed_files, "ci: original 7F1 gate attempt")

    plan = json.loads((root / "docs/execution/PLAN.json").read_text(encoding="utf-8"))
    future_checkpoint = execution_plan.checkpoint_for_id(plan, "16G")
    future_checkpoint["dependency_change_policy"] += " Future gate detail reviewed."
    state_bytes = (root / "docs/execution/STATE.json").read_bytes()
    maintenance_files: dict[str, bytes | str] = {
        "docs/execution/PLAN.json": json.dumps(plan, indent=2) + "\n",
        "docs/execution/STATE.json": state_bytes,
        "docs/execution/phases/PHASE_8.md": b"future phase 8 reviewed\n",
        "scripts/execution_plan.py": (REPO_ROOT / "scripts/execution_plan.py").read_bytes(),
        "scripts/test_execution_infra.py": (
            REPO_ROOT / "scripts/test_execution_infra.py"
        ).read_bytes(),
    }
    head = commit_test_files(root, maintenance_files, "fix: trusted future control-plane maintenance")
    return failed_sha, head


def evidence_policy() -> dict[str, object]:
    return execution_evidence.load_policy(REPO_ROOT / "docs" / "execution" / "EVIDENCE_POLICY.json")


def valid_gate(gate_id: str, implementation_sha: str, run_id: int = 101) -> dict[str, object]:
    policy = evidence_policy()
    gate = policy["required_gates"][gate_id]  # type: ignore[index]
    jobs = [
        {"name": name, "status": "completed", "conclusion": "success"}
        for name in gate["required_jobs"]  # type: ignore[index]
    ]
    return {
        "gate_id": gate_id,
        "workflow_name": gate["workflow_name"],  # type: ignore[index]
        "workflow_file": gate["workflow_file"],  # type: ignore[index]
        "run_id": run_id,
        "run_attempt": 1,
        "head_sha": implementation_sha,
        "head_branch": "main",
        "event": "push",
        "status": "completed",
        "conclusion": "success",
        "html_url": f"https://github.com/huou07/Opencut-Reinforced/actions/runs/{run_id}",
        "jobs": jobs,
    }


def valid_evidence(
    checkpoint_id: str = "7A", implementation_sha: str = "a" * 40
) -> dict[str, object]:
    return execution_evidence.build_evidence_record(
        checkpoint_id=checkpoint_id,
        implementation_sha=implementation_sha,
        implementation_subject="feat: test implementation",
        gates=[valid_gate("repository_hygiene", implementation_sha), valid_gate("platform_verification", implementation_sha, 102)],
        developer_preview={"required": False},
        contract_versions=execution_plan.read_contract_versions(REPO_ROOT),
        verified_at_utc="2026-09-28T00:00:00Z",
    )


def valid_preview(implementation_sha: str = "a" * 40) -> dict[str, object]:
    policy = evidence_policy()
    preview = policy["developer_preview"]  # type: ignore[index]
    tag = execution_evidence.preview_tag(implementation_sha, policy)
    return {
        "required": True,
        "status": "verified",
        "tag": tag,
        "source_sha": implementation_sha,
        "workflow_file": preview["workflow_file"],  # type: ignore[index]
        "workflow_name": preview["workflow_name"],  # type: ignore[index]
        "workflow_run": {
            "id": 303,
            "attempt": 1,
            "url": "https://github.com/huou07/Opencut-Reinforced/actions/runs/303",
            "status": "completed",
            "conclusion": "success",
            "jobs": [
                {
                    "name": preview["publish_job"],  # type: ignore[index]
                    "status": "completed",
                    "conclusion": "success",
                }
            ],
        },
        "release_url": f"https://github.com/huou07/Opencut-Reinforced/releases/tag/{tag}",
        "assets": [{"name": name, "size": 1} for name in preview["required_assets"]],  # type: ignore[index]
        "checksums": preview["checksums_asset"],  # type: ignore[index]
        "build_info": preview["build_info_asset"],  # type: ignore[index]
    }


def workflow_run(
    implementation_sha: str = "a" * 40,
    *,
    run_id: int = 101,
    status: str = "completed",
    conclusion: str | None = "success",
    branch: str = "main",
    event: str = "push",
    updated_at: str = "2026-09-28T00:00:00Z",
    workflow_file: str = ".github/workflows/platform-verification.yml",
    workflow_name: str = "Platform verification",
) -> dict[str, object]:
    return {
        "id": run_id,
        "run_attempt": 1,
        "head_sha": implementation_sha,
        "head_branch": branch,
        "event": event,
        "status": status,
        "conclusion": conclusion,
        "path": workflow_file,
        "name": workflow_name,
        "html_url": f"https://github.com/huou07/Opencut-Reinforced/actions/runs/{run_id}",
        "updated_at": updated_at,
    }


class FakeApi:
    def __init__(self, payload: object, *, authenticated: bool = False) -> None:
        self.payload = payload
        self.authenticated = authenticated
        self.calls: list[str] = []

    def get(self, path: str) -> object:
        self.calls.append(path)
        return self.payload


class SequenceApi:
    def __init__(self, responses: list[object], *, authenticated: bool = False) -> None:
        self.responses = iter(responses)
        self.authenticated = authenticated
        self.calls: list[str] = []
        self.posts: list[tuple[str, object]] = []

    def get(self, path: str) -> object:
        self.calls.append(path)
        response = next(self.responses)
        if isinstance(response, BaseException):
            raise response
        return response

    def post(self, path: str, payload: object) -> object:
        self.posts.append((path, payload))
        return {}


class ApiResponse:
    def __init__(self, payload: object, headers: dict[str, str] | None = None) -> None:
        self.payload = payload
        self.headers = headers or {}

    def __enter__(self) -> "ApiResponse":
        return self

    def __exit__(self, *_args: object) -> None:
        return None

    def read(self) -> bytes:
        return json.dumps(self.payload).encode("utf-8")


class AdvancingClock:
    def __init__(self, values: list[float]) -> None:
        self.values = iter(values)

    def __call__(self) -> float:
        return next(self.values)


class ExecutionPlanTests(unittest.TestCase):
    def test_current_repository_plan_is_valid(self) -> None:
        plan, state = execution_plan.load_plan_state(REPO_ROOT)
        summary = execution_plan.validate_plan(plan, state, REPO_ROOT)
        self.assertEqual(summary["next_checkpoint"], state["current_next"])

    def test_16d_project_schema_effect_is_none_and_ipc_contract_is_retained(self) -> None:
        plan, state = execution_plan.load_plan_state(REPO_ROOT)
        checkpoint_16d = execution_plan.checkpoint_for_id(plan, "16D")
        self.assertEqual(checkpoint_16d["expected_project_schema_effect_category"], "none")
        self.assertEqual(
            checkpoint_16d["expected_ipc_effect_category"], "explicit-contract-gate"
        )
        execution_plan.validate_plan(plan, state, REPO_ROOT)

    def test_project_schema_rejects_ipc_category(self) -> None:
        plan, state = fixture_plan_state()
        plan["checkpoints"][0]["expected_project_schema_effect_category"] = "explicit-contract-gate"  # type: ignore[index]
        with self.assertRaises(execution_plan.PlanError):
            execution_plan.validate_plan(plan, state, REPO_ROOT)

    def test_ipc_rejects_explicit_model_category(self) -> None:
        plan, state = fixture_plan_state()
        plan["checkpoints"][0]["expected_ipc_effect_category"] = "explicit-model-gate"  # type: ignore[index]
        with self.assertRaises(execution_plan.PlanError):
            execution_plan.validate_plan(plan, state, REPO_ROOT)

    def test_ipc_rejects_typed_model_category(self) -> None:
        plan, state = fixture_plan_state()
        plan["checkpoints"][0]["expected_ipc_effect_category"] = "typed-model-gate"  # type: ignore[index]
        with self.assertRaises(execution_plan.PlanError):
            execution_plan.validate_plan(plan, state, REPO_ROOT)

    def test_valid_project_model_gates(self) -> None:
        for category in ("explicit-model-gate", "typed-model-gate"):
            with self.subTest(category=category):
                plan, state = fixture_plan_state(["7A", "7B"])
                plan["checkpoints"][0]["expected_project_schema_effect_category"] = category  # type: ignore[index]
                execution_plan.validate_plan(plan, state, REPO_ROOT)

    def test_valid_ipc_contract_gate(self) -> None:
        plan, state = fixture_plan_state(["7A", "7B"])
        plan["checkpoints"][0]["expected_ipc_effect_category"] = "explicit-contract-gate"  # type: ignore[index]
        execution_plan.validate_plan(plan, state, REPO_ROOT)

    def test_8a_derived_model_and_contract_categories_remain_valid(self) -> None:
        plan, state = execution_plan.load_plan_state(REPO_ROOT)
        checkpoint_8b = execution_plan.checkpoint_for_id(plan, "8B")
        self.assertEqual(
            checkpoint_8b["expected_project_schema_effect_category"], "8A-model-only"
        )
        self.assertEqual(
            checkpoint_8b["expected_ipc_effect_category"], "8A-contract-only"
        )
        execution_plan.validate_plan(plan, state, REPO_ROOT)

    def test_model_only_requires_existing_owner(self) -> None:
        plan, state = fixture_plan_state()
        plan["checkpoints"][1]["expected_project_schema_effect_category"] = "MISSING-model-only"  # type: ignore[index]
        with self.assertRaises(execution_plan.PlanError):
            execution_plan.validate_plan(plan, state, REPO_ROOT)

    def test_contract_only_requires_existing_owner(self) -> None:
        plan, state = fixture_plan_state()
        plan["checkpoints"][1]["expected_ipc_effect_category"] = "MISSING-contract-only"  # type: ignore[index]
        with self.assertRaises(execution_plan.PlanError):
            execution_plan.validate_plan(plan, state, REPO_ROOT)

    def test_model_only_requires_an_earlier_model_gate_owner(self) -> None:
        for owner_category, related in (("none", True), ("explicit-model-gate", False)):
            with self.subTest(owner_category=owner_category, related=related):
                plan, state = fixture_plan_state()
                plan["checkpoints"][0]["expected_project_schema_effect_category"] = owner_category  # type: ignore[index]
                plan["checkpoints"][1]["expected_project_schema_effect_category"] = "A-model-only"  # type: ignore[index]
                if not related:
                    plan["checkpoints"][0]["next_checkpoint_relation"] = None  # type: ignore[index]
                    plan["checkpoints"][1]["prerequisite_checkpoint_ids"] = []  # type: ignore[index]
                with self.assertRaises(execution_plan.PlanError):
                    execution_plan.validate_plan(plan, state, REPO_ROOT)

    def test_contract_only_requires_an_earlier_contract_gate_owner(self) -> None:
        for owner_category, related in (("none", True), ("explicit-contract-gate", False)):
            with self.subTest(owner_category=owner_category, related=related):
                plan, state = fixture_plan_state()
                plan["checkpoints"][0]["expected_ipc_effect_category"] = owner_category  # type: ignore[index]
                plan["checkpoints"][1]["expected_ipc_effect_category"] = "A-contract-only"  # type: ignore[index]
                if not related:
                    plan["checkpoints"][0]["next_checkpoint_relation"] = None  # type: ignore[index]
                    plan["checkpoints"][1]["prerequisite_checkpoint_ids"] = []  # type: ignore[index]
                with self.assertRaises(execution_plan.PlanError):
                    execution_plan.validate_plan(plan, state, REPO_ROOT)

    def test_completed_historical_schema_categories_remain_valid(self) -> None:
        plan, state = execution_plan.load_plan_state(REPO_ROOT)
        self.assertEqual(state["checkpoints"]["6A"], "DONE")  # type: ignore[index]
        self.assertEqual(state["checkpoints"]["6E2A"], "DONE")  # type: ignore[index]
        self.assertEqual(
            execution_plan.checkpoint_for_id(plan, "6A")["expected_project_schema_effect_category"],
            "schema-v3",
        )
        self.assertEqual(
            execution_plan.checkpoint_for_id(plan, "6E2A")["expected_project_schema_effect_category"],
            "schema-v4",
        )
        execution_plan.validate_plan(plan, state, REPO_ROOT)

    def test_schema_version_category_is_rejected_for_ipc(self) -> None:
        plan, state = fixture_plan_state()
        plan["checkpoints"][0]["expected_ipc_effect_category"] = "schema-v5"  # type: ignore[index]
        with self.assertRaises(execution_plan.PlanError):
            execution_plan.validate_plan(plan, state, REPO_ROOT)

    def test_new_schema_version_category_is_rejected_for_planned_checkpoint(self) -> None:
        plan, state = fixture_plan_state()
        plan["checkpoints"][0]["expected_project_schema_effect_category"] = "schema-v5"  # type: ignore[index]
        with self.assertRaises(execution_plan.PlanError):
            execution_plan.validate_plan(plan, state, REPO_ROOT)

    def test_runner_allowed_workflow_path_is_valid_for_architecture_gate(self) -> None:
        plan, state = fixture_plan_state(["7A", "7B"])
        plan["checkpoints"][0]["architecture_gate"] = True  # type: ignore[index]
        plan["checkpoints"][0]["runner_allowed_protected_paths"] = [  # type: ignore[index]
            ".github/workflows/platform-verification.yml"
        ]
        execution_plan.validate_plan(plan, state, REPO_ROOT)

    def test_invalid_runner_allowed_workflow_paths_are_rejected(self) -> None:
        invalid_allowlists = (
            None,
            [""],
            ["docs/execution/STATE.json"],
            ["scripts/agent_supervisor.py"],
            [".github/workflows/"],
            ["../something"],
            ["/github/workflows/platform-verification.yml"],
            [r".github\workflows\platform-verification.yml"],
            [".github/workflows/*.yml"],
            [".github/workflows/platform-verification.yml"] * 2,
        )
        for allowlist in invalid_allowlists:
            with self.subTest(allowlist=allowlist):
                plan, state = fixture_plan_state()
                plan["checkpoints"][0]["architecture_gate"] = True  # type: ignore[index]
                plan["checkpoints"][0]["runner_allowed_protected_paths"] = allowlist  # type: ignore[index]
                with self.assertRaises(execution_plan.PlanError):
                    execution_plan.validate_plan(plan, state, REPO_ROOT)

    def test_runner_allowed_workflow_path_requires_architecture_gate(self) -> None:
        plan, state = fixture_plan_state()
        plan["checkpoints"][0]["runner_allowed_protected_paths"] = [  # type: ignore[index]
            ".github/workflows/platform-verification.yml"
        ]
        with self.assertRaises(execution_plan.PlanError):
            execution_plan.validate_plan(plan, state, REPO_ROOT)

    def test_post_7f0_architecture_gate_graph_is_next_and_valid(self) -> None:
        plan, state = seven_f_one_plan_state()
        resolution = execution_plan.resolve_goal(
            plan, state, "checkpoint:7F1", REPO_ROOT
        )
        self.assertEqual(resolution["checkpoint_id"], "7F1")
        self.assertEqual(state["current_next"], "7F1")
        statuses = state["checkpoints"]
        for checkpoint_id in ("7A", "7B", "7C0", "7C", "7D", "7E"):
            self.assertEqual(statuses[checkpoint_id], "DONE")  # type: ignore[index]
        self.assertEqual(statuses["7F0"], "DONE")  # type: ignore[index]
        self.assertEqual(statuses["7F1"], "NEXT")  # type: ignore[index]
        for checkpoint_id in ("7F", "7G", "7H"):
            self.assertEqual(statuses[checkpoint_id], "PLANNED")  # type: ignore[index]
        for checkpoint_id in ("8A", "8B", "8C", "8D", "8E", "8F"):
            self.assertEqual(statuses[checkpoint_id], "PLANNED")  # type: ignore[index]

        checkpoint_7f0 = execution_plan.checkpoint_for_id(plan, "7F0")
        checkpoint_7f1 = execution_plan.checkpoint_for_id(plan, "7F1")
        checkpoint_7f = execution_plan.checkpoint_for_id(plan, "7F")
        self.assertEqual(checkpoint_7f0["prerequisite_checkpoint_ids"], ["7E"])
        self.assertEqual(checkpoint_7f0["next_checkpoint_relation"], "7F1")
        self.assertEqual(
            checkpoint_7f0["milestone_membership"], ["desktop-mvp", "full-roadmap"]
        )
        self.assertFalse(checkpoint_7f0["user_visible"])
        self.assertFalse(checkpoint_7f0["developer_preview_required"])
        self.assertTrue(checkpoint_7f0["architecture_gate"])
        self.assertEqual(checkpoint_7f1["prerequisite_checkpoint_ids"], ["7F0"])
        self.assertEqual(checkpoint_7f1["next_checkpoint_relation"], "7F")
        self.assertFalse(checkpoint_7f1["user_visible"])
        self.assertFalse(checkpoint_7f1["developer_preview_required"])
        self.assertTrue(checkpoint_7f1["architecture_gate"])
        self.assertEqual(checkpoint_7f1["expected_project_schema_effect_category"], "none")
        self.assertEqual(checkpoint_7f1["expected_ipc_effect_category"], "none")
        self.assertEqual(
            checkpoint_7f1["runner_allowed_protected_paths"],
            [".github/workflows/platform-verification.yml"],
        )
        self.assertEqual(checkpoint_7f["prerequisite_checkpoint_ids"], ["7F1"])
        self.assertEqual(
            checkpoint_7f0["expected_project_schema_effect_category"],
            "explicit-model-gate",
        )
        self.assertEqual(
            checkpoint_7f0["expected_ipc_effect_category"],
            "explicit-contract-gate",
        )
        self.assertEqual(checkpoint_7f["expected_project_schema_effect_category"], "none")
        checkpoint_8f = execution_plan.checkpoint_for_id(plan, "8F")
        self.assertEqual(
            checkpoint_8f["expected_ipc_effect_category"],
            "explicit-contract-gate",
        )
        for milestone_id in ("desktop-mvp", "full-roadmap"):
            ordered_ids = plan["milestones"][milestone_id]["checkpoint_ids"]  # type: ignore[index]
            self.assertEqual(ordered_ids.index("7F0") + 1, ordered_ids.index("7F1"))
            self.assertEqual(ordered_ids.index("7F1") + 1, ordered_ids.index("7F"))

        checkpoint_8f = execution_plan.checkpoint_for_id(plan, "8F")
        checkpoint_9a0 = execution_plan.checkpoint_for_id(plan, "9A0")
        checkpoint_9a = execution_plan.checkpoint_for_id(plan, "9A")
        self.assertEqual(checkpoint_9a0["phase"], 9)
        self.assertEqual(checkpoint_9a0["prerequisite_checkpoint_ids"], ["8F"])
        self.assertEqual(checkpoint_8f["next_checkpoint_relation"], "9A0")
        self.assertEqual(checkpoint_9a0["next_checkpoint_relation"], "9A")
        self.assertEqual(checkpoint_9a["prerequisite_checkpoint_ids"], ["9A0"])
        self.assertEqual(checkpoint_9a0["milestone_membership"], ["full-roadmap"])
        self.assertFalse(checkpoint_9a0["user_visible"])
        self.assertFalse(checkpoint_9a0["developer_preview_required"])
        self.assertTrue(checkpoint_9a0["architecture_gate"])
        self.assertEqual(checkpoint_9a0["expected_project_schema_effect_category"], "none")
        self.assertEqual(checkpoint_9a0["expected_ipc_effect_category"], "none")
        self.assertEqual(
            checkpoint_9a0["runner_allowed_protected_paths"],
            [".github/workflows/platform-verification.yml"],
        )
        self.assertEqual(statuses["9A0"], "PLANNED")  # type: ignore[index]
        self.assertNotIn(
            "9A0", plan["milestones"]["desktop-mvp"]["checkpoint_ids"]  # type: ignore[index]
        )
        full_roadmap_ids = plan["milestones"]["full-roadmap"]["checkpoint_ids"]  # type: ignore[index]
        self.assertEqual(full_roadmap_ids.index("8F") + 1, full_roadmap_ids.index("9A0"))
        self.assertEqual(full_roadmap_ids.index("9A0") + 1, full_roadmap_ids.index("9A"))

        for checkpoint_id in ("9A", "9B", "9D", "9E", "10A", "16A", "16G"):
            self.assertEqual(
                execution_plan.checkpoint_for_id(plan, checkpoint_id)[
                    "runner_allowed_protected_paths"
                ],
                [".github/workflows/platform-verification.yml"],
            )
        for checkpoint_id in ("8D", "8E"):
            self.assertIn(
                ".github/workflows/platform-verification.yml",
                execution_plan.checkpoint_for_id(plan, checkpoint_id)[
                    "runner_allowed_protected_paths"
                ],
            )
        for checkpoint_id in ("7H", "8F"):
            self.assertIn(
                ".github/workflows/developer-preview.yml",
                execution_plan.checkpoint_for_id(plan, checkpoint_id)[
                    "runner_allowed_protected_paths"
                ],
            )

        def evidence_existed_at_7f1(checkpoint_id: str) -> bool:
            return subprocess.run(
                ["git", "cat-file", "-e", f"ed1633a:docs/execution/evidence/{checkpoint_id}.json"],
                cwd=REPO_ROOT,
                capture_output=True,
            ).returncode == 0

        self.assertTrue(evidence_existed_at_7f1("7F0"))
        for checkpoint_id in ("7F1", "9A0", "7F"):
            self.assertFalse(evidence_existed_at_7f1(checkpoint_id))

        checkpoint_7d = execution_plan.checkpoint_for_id(plan, "7D")
        policy = checkpoint_7d["dependency_change_policy"]
        self.assertIn("optional", policy)
        self.assertIn("measured need", policy)
        self.assertIn("build/license evidence", policy)
        self.assertIn("retain software-only fallback and continue", policy)
        self.assertEqual(state["phase_status"]["7"], "IN_PROGRESS")  # type: ignore[index]
        execution_plan.validate_plan(plan, state, REPO_ROOT)

    def test_or_core_dependency_policy_covers_frozen_runtime_engines(self) -> None:
        policy = execution_plan.load_json(
            REPO_ROOT / "docs/execution/architecture-policy.json"
        )
        patterns = policy["or_core_forbidden_direct_dependency_name_patterns"]
        self.assertIsInstance(patterns, list)
        for dependency in ("cosmic-text", "cosmic_text", "wasmi", "whisper_rs", "whisper_cpp"):
            with self.subTest(dependency=dependency):
                names = check_architecture_policy._direct_dependency_names(
                    f"[dependencies]\n{dependency} = \"1.0\"\n"
                )
                self.assertTrue(
                    any(
                        pattern.lower() in name
                        for pattern in patterns
                        for name in names
                    ),
                    f"or_core dependency policy does not reject {dependency}",
                )

    def test_duplicate_checkpoint_is_rejected(self) -> None:
        plan, state = fixture_plan_state()
        plan["checkpoints"].append(plan["checkpoints"][0])  # type: ignore[index]
        with self.assertRaises(execution_plan.PlanError):
            execution_plan.validate_plan(plan, state, REPO_ROOT)

    def test_missing_dependency_is_rejected(self) -> None:
        plan, state = fixture_plan_state()
        plan["checkpoints"][1]["prerequisite_checkpoint_ids"] = ["MISSING"]  # type: ignore[index]
        with self.assertRaises(execution_plan.PlanError):
            execution_plan.validate_plan(plan, state, REPO_ROOT)

    def test_cycle_is_rejected(self) -> None:
        plan, state = fixture_plan_state()
        plan["checkpoints"][0]["prerequisite_checkpoint_ids"] = ["B"]  # type: ignore[index]
        with self.assertRaises(execution_plan.PlanError):
            execution_plan.validate_plan(plan, state, REPO_ROOT)

    def test_two_next_checkpoints_are_rejected(self) -> None:
        plan, state = fixture_plan_state()
        state["checkpoints"]["B"] = "NEXT"  # type: ignore[index]
        with self.assertRaises(execution_plan.PlanError):
            execution_plan.validate_plan(plan, state, REPO_ROOT)

    def test_done_checkpoint_with_unfinished_dependency_is_rejected(self) -> None:
        plan, state = fixture_plan_state()
        state["checkpoints"]["B"] = "DONE"  # type: ignore[index]
        with self.assertRaises(execution_plan.PlanError):
            execution_plan.validate_plan(plan, state, REPO_ROOT)

    def test_unknown_state_checkpoint_is_rejected(self) -> None:
        plan, state = fixture_plan_state()
        state["checkpoints"]["UNKNOWN"] = "PLANNED"  # type: ignore[index]
        with self.assertRaises(execution_plan.PlanError):
            execution_plan.validate_plan(plan, state, REPO_ROOT)

    def test_old_done_checkpoint_is_grandfathered_without_evidence(self) -> None:
        plan, state = fixture_plan_state(["6E2B", "7A"])
        state["checkpoints"]["6E2B"] = "DONE"  # type: ignore[index]
        state["checkpoints"]["7A"] = "NEXT"  # type: ignore[index]
        state["current_next"] = "7A"
        self.assertEqual(execution_plan.validate_plan(plan, state, REPO_ROOT)["next_checkpoint"], "7A")

    def test_done_enforced_checkpoint_requires_valid_evidence(self) -> None:
        plan, state = fixture_plan_state(["7A", "7B"])
        state["checkpoints"]["7A"] = "DONE"  # type: ignore[index]
        state["checkpoints"]["7B"] = "NEXT"  # type: ignore[index]
        state["current_next"] = "7B"
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "docs/execution/phases").mkdir(parents=True)
            (root / "docs/execution/evidence").mkdir(parents=True)
            (root / "docs/execution/phases/PHASE_6.md").write_text("phase\n", encoding="utf-8")
            (root / "docs/execution/EVIDENCE_POLICY.json").write_text(
                json.dumps(evidence_policy()), encoding="utf-8"
            )
            with self.assertRaises(execution_plan.PlanError):
                execution_plan.validate_plan(plan, state, root)
            (root / "docs/execution/evidence/7A.json").write_text(
                json.dumps(valid_evidence()), encoding="utf-8"
            )
            self.assertEqual(
                execution_plan.validate_plan(plan, state, root)["next_checkpoint"], "7B"
            )


class ContractVersionTests(unittest.TestCase):
    def setUp(self) -> None:
        self.policy = execution_plan.load_architecture_policy(REPO_ROOT)

    def test_central_reader_uses_current_source_constants(self) -> None:
        self.assertEqual(execution_plan.read_contract_versions(REPO_ROOT), contract_versions(project=7))

    def test_explicit_model_gate_may_retain_or_increment_once(self) -> None:
        for candidate in (4, 5):
            with self.subTest(candidate=candidate):
                plan, state = fixture_plan_state(["7A", "7B"])
                plan["checkpoints"][0]["expected_project_schema_effect_category"] = "explicit-model-gate"  # type: ignore[index]
                state["verified_contract_versions"] = contract_versions(project=4)
                execution_plan.validate_plan(plan, state, REPO_ROOT)
                result = execution_plan.validate_contract_transition(
                    plan, state, self.policy, contract_versions(project=candidate)
                )
                self.assertEqual(result["project_schema"], candidate)

    def test_project_increment_overflow_and_decrement_are_rejected(self) -> None:
        for candidate in (3, 6):
            with self.subTest(candidate=candidate):
                plan, state = fixture_plan_state(["7A", "7B"])
                plan["checkpoints"][0]["expected_project_schema_effect_category"] = "typed-model-gate"  # type: ignore[index]
                state["verified_contract_versions"] = contract_versions(project=4)
                with self.assertRaises(execution_plan.PlanError):
                    execution_plan.validate_contract_transition(
                        plan, state, self.policy, contract_versions(project=candidate)
                    )

    def test_project_non_owner_cannot_bump(self) -> None:
        plan, state = fixture_plan_state(["7A", "7B"])
        state["verified_contract_versions"] = contract_versions(project=4)
        with self.assertRaises(execution_plan.PlanError):
            execution_plan.validate_contract_transition(
                plan, state, self.policy, contract_versions(project=5)
            )

    def test_project_owner_derived_checkpoint_cannot_bump(self) -> None:
        plan, state = fixture_plan_state()
        plan["checkpoints"][0]["expected_project_schema_effect_category"] = "explicit-model-gate"  # type: ignore[index]
        plan["checkpoints"][1]["expected_project_schema_effect_category"] = "A-model-only"  # type: ignore[index]
        state["checkpoints"] = {"A": "DONE", "B": "NEXT"}  # type: ignore[assignment]
        state["current_next"] = "B"
        state["verified_contract_versions"] = contract_versions(project=4)
        stable = execution_plan.validate_contract_transition(
            plan, state, self.policy, contract_versions(project=4)
        )
        self.assertEqual(stable["project_schema"], 4)
        with self.assertRaises(execution_plan.PlanError):
            execution_plan.validate_contract_transition(
                plan, state, self.policy, contract_versions(project=5)
            )

    def test_explicit_ipc_gate_may_retain_or_increment_once(self) -> None:
        for candidate in (1, 2):
            with self.subTest(candidate=candidate):
                plan, state = fixture_plan_state(["7A", "7B"])
                plan["checkpoints"][0]["expected_ipc_effect_category"] = "explicit-contract-gate"  # type: ignore[index]
                state["verified_contract_versions"] = contract_versions(ipc=1)
                result = execution_plan.validate_contract_transition(
                    plan, state, self.policy, contract_versions(ipc=candidate)
                )
                self.assertEqual(result["ipc_protocol"], candidate)

    def test_ipc_increment_overflow_is_rejected(self) -> None:
        plan, state = fixture_plan_state(["7A", "7B"])
        plan["checkpoints"][0]["expected_ipc_effect_category"] = "explicit-contract-gate"  # type: ignore[index]
        state["verified_contract_versions"] = contract_versions()
        with self.assertRaises(execution_plan.PlanError):
            execution_plan.validate_contract_transition(
                plan, state, self.policy, contract_versions(ipc=3)
            )

    def test_ipc_non_owner_and_contract_only_cannot_bump(self) -> None:
        plan, state = fixture_plan_state(["7A", "7B"])
        state["verified_contract_versions"] = contract_versions(ipc=1)
        with self.assertRaises(execution_plan.PlanError):
            execution_plan.validate_contract_transition(
                plan, state, self.policy, contract_versions(ipc=2)
            )

        plan, state = fixture_plan_state()
        plan["checkpoints"][0]["expected_ipc_effect_category"] = "explicit-contract-gate"  # type: ignore[index]
        plan["checkpoints"][1]["expected_ipc_effect_category"] = "A-contract-only"  # type: ignore[index]
        state["checkpoints"] = {"A": "DONE", "B": "NEXT"}  # type: ignore[assignment]
        state["current_next"] = "B"
        state["verified_contract_versions"] = contract_versions(ipc=1)
        with self.assertRaises(execution_plan.PlanError):
            execution_plan.validate_contract_transition(
                plan, state, self.policy, contract_versions(ipc=2)
            )

    def test_recovery_version_is_fixed_without_an_owner(self) -> None:
        plan, state = fixture_plan_state(["7A", "7B"])
        state["verified_contract_versions"] = contract_versions(recovery=1)
        with self.assertRaises(execution_plan.PlanError):
            execution_plan.validate_contract_transition(
                plan, state, self.policy, contract_versions(recovery=2)
            )

    def test_current_9b_retains_verified_schema_seven(self) -> None:
        plan, state = nine_b_plan_state()
        self.assertEqual(state["current_next"], "9B")
        self.assertEqual(state["verified_contract_versions"], contract_versions(project=7))
        self.assertEqual(
            execution_plan.validate_contract_transition(
                plan,
                state,
                self.policy,
                execution_plan.read_contract_versions(REPO_ROOT),
            ),
            contract_versions(project=7),
        )

    def test_desktop_mvp_and_16d_category_ownership_remain_locked(self) -> None:
        plan, state = execution_plan.load_plan_state(REPO_ROOT)
        expected = {
            "7F0": ("explicit-model-gate", "explicit-contract-gate"),
            "7F1": ("none", "none"),
            "7F": ("none", "none"),
            "7G": ("none", "none"),
            "7H": ("none", "none"),
            "8A": ("explicit-model-gate", "explicit-contract-gate"),
            "8B": ("8A-model-only", "8A-contract-only"),
            "8C": ("8A-model-only", "none"),
            "8D": ("8A-model-only", "none"),
            "8E": ("8A-model-only", "none"),
            "8F": ("8A-model-only", "explicit-contract-gate"),
            "9A0": ("none", "none"),
            "9A": ("explicit-model-gate", "none"),
            "16D": ("none", "explicit-contract-gate"),
        }
        for checkpoint_id, categories in expected.items():
            checkpoint_spec = execution_plan.checkpoint_for_id(plan, checkpoint_id)
            actual = (
                checkpoint_spec["expected_project_schema_effect_category"],
                checkpoint_spec["expected_ipc_effect_category"],
            )
            with self.subTest(checkpoint=checkpoint_id):
                self.assertEqual(actual, categories)
        execution_plan.validate_plan(plan, state, REPO_ROOT)

    def test_7f1_completion_snapshots_versions_for_the_following_non_owner(self) -> None:
        plan, state = seven_f_one_plan_state()
        after = agent_supervisor.advance_state_once(
            state,
            plan,
            "7F1",
            candidate_contract_versions=contract_versions(project=5),
            policy=self.policy,
            today="2026-09-30",
        )
        self.assertEqual(after["verified_contract_versions"], contract_versions(project=5))
        self.assertEqual(after["current_next"], "7F")
        self.assertEqual(
            execution_plan.validate_contract_transition(
                plan, after, self.policy, contract_versions(project=5)
            )["project_schema"],
            5,
        )


class QualityEvidenceTests(unittest.TestCase):
    def test_9b_fixtures_survive_the_legitimate_9b1_state_transition(self) -> None:
        plan, _ = execution_plan.load_plan_state(REPO_ROOT)
        later = roadmap_state_before(plan, "9B1", contract_versions())
        with mock.patch.object(execution_plan, "load_plan_state", return_value=(plan, later)):
            self.test_generated_prompt_preserves_quality_requirements()
            self.test_9b1_graph_and_evidence_classes_are_locked()

    def test_generated_prompt_preserves_quality_requirements(self) -> None:
        plan, state = nine_b_plan_state()
        resolution = execution_plan.resolve_goal(plan, state, "checkpoint:9B", REPO_ROOT)
        prompt = agent_supervisor.checkpoint_prompt(REPO_ROOT, resolution)
        for requirement in (
            "You are not rewarded for the smallest implementation that makes existing tests green.",
            "actual product boundary", "bridge-only calls", "system-installed tools",
            "in-process-only tests", "disabled enforcement", "synthetic equivalent",
            "After two speculative fixes", "falsifiable hypothesis", "No generic retry",
            "Required evidence classes: STATIC, UNIT, INTEGRATION, NATIVE_RUNTIME, USER_JOURNEY, PERFORMANCE, RESOURCE_STRESS.",
        ):
            self.assertIn(requirement, prompt)

    def test_9b1_graph_and_evidence_classes_are_locked(self) -> None:
        plan, state = nine_b_plan_state()
        self.assertEqual(state["current_next"], "9B")
        self.assertEqual(state["checkpoints"]["9B1"], "PLANNED")
        self.assertEqual(execution_plan.checkpoint_for_id(plan, "9B")["next_checkpoint_relation"], "9B1")
        hardening = execution_plan.checkpoint_for_id(plan, "9B1")
        self.assertEqual(hardening["next_checkpoint_relation"], "9C")
        self.assertTrue(hardening["developer_preview_required"])
        self.assertEqual(set(hardening["required_evidence_classes"]), execution_plan.EVIDENCE_CLASSES)
        weakened = json.loads(json.dumps(plan))
        execution_plan.checkpoint_for_id(weakened, "9B1")["required_evidence_classes"].remove("CLEAN_ENVIRONMENT")
        with self.assertRaisesRegex(execution_plan.PlanError, "lacks packaged hardening"):
            execution_plan.validate_plan(weakened, state, REPO_ROOT)

    def test_9b_class_proofs_require_successful_named_hosted_steps(self) -> None:
        plan, _ = execution_plan.load_plan_state(REPO_ROOT)
        checkpoint_spec = execution_plan.checkpoint_for_id(plan, "9B")
        policy = evidence_policy()
        sha = "a" * 40
        gates = [valid_gate("repository_hygiene", sha), valid_gate("platform_verification", sha, 102)]
        jobs = {
            101: [{"id": 11, "name": "Repository hygiene", "status": "completed", "conclusion": "success",
                   "steps": [{"name": "Run repository checks", "number": 2, "status": "completed", "conclusion": "success"}]}],
            102: [
                {"id": 21, "name": "Rust checks", "status": "completed", "conclusion": "success",
                 "steps": [{"name": "Run Rust tests", "number": 8, "status": "completed", "conclusion": "success"}]},
                {"id": 22, "name": "Android APK build", "status": "completed", "conclusion": "success",
                 "steps": [
                     {"name": "Verify FFmpeg runtime and Flutter Rust bridge on x86_64 emulator", "number": 12, "status": "completed", "conclusion": "success"},
                     {"name": "Verify Android SAF preview user journey and resource bounds", "number": 13, "status": "completed", "conclusion": "success"},
                 ]},
            ],
        }
        class Api:
            def get(self, path: str) -> dict[str, object]:
                run_id = int(path.split("/actions/runs/")[1].split("/")[0])
                return {"jobs": jobs[run_id]}

        proofs = execution_evidence.collect_evidence_class_proofs(Api(), policy, checkpoint_spec, gates)
        record = execution_evidence.build_evidence_record(
            checkpoint_id="9B", implementation_sha=sha, implementation_subject="fix: test",
            gates=gates, developer_preview={"required": False},
            contract_versions=contract_versions(), evidence_classes=proofs,
        )
        execution_evidence.validate_evidence_record(record, checkpoint_id="9B", checkpoint=checkpoint_spec, policy=policy)
        self.assertEqual(record["schema_version"], 2)
        jobs[102][1]["steps"].pop()
        with self.assertRaisesRegex(execution_evidence.EvidenceError, "proof step did not succeed"):
            execution_evidence.collect_evidence_class_proofs(Api(), policy, checkpoint_spec, gates)
        record["evidence_classes"].pop()
        with self.assertRaisesRegex(execution_evidence.EvidenceError, "do not match PLAN.json"):
            execution_evidence.validate_evidence_record(record, checkpoint_id="9B", checkpoint=checkpoint_spec, policy=policy)

    def test_9c_class_proofs_bind_mobile_acceptance_to_hosted_android_steps(self) -> None:
        plan, _ = execution_plan.load_plan_state(REPO_ROOT)
        checkpoint_spec = execution_plan.checkpoint_for_id(plan, "9C")
        policy = evidence_policy()
        sha = "a" * 40
        gates = [valid_gate("repository_hygiene", sha), valid_gate("platform_verification", sha, 102)]
        jobs = {
            101: [{
                "id": 11,
                "name": "Repository hygiene",
                "status": "completed",
                "conclusion": "success",
                "steps": [{"name": "Run repository checks", "number": 2, "status": "completed", "conclusion": "success"}],
            }],
            102: [
                {
                    "id": 21,
                    "name": "Flutter static and widget checks",
                    "status": "completed",
                    "conclusion": "success",
                    "steps": [{"name": "Run Flutter widget tests", "number": 3, "status": "completed", "conclusion": "success"}],
                },
                {
                    "id": 22,
                    "name": "Android APK build",
                    "status": "completed",
                    "conclusion": "success",
                    "steps": [{"name": "Verify Android SAF preview user journey and resource bounds", "number": 13, "status": "completed", "conclusion": "success"}],
                },
            ],
        }

        class Api:
            def get(self, path: str) -> dict[str, object]:
                run_id = int(path.split("/actions/runs/")[1].split("/")[0])
                return {"jobs": jobs[run_id]}

        proofs = execution_evidence.collect_evidence_class_proofs(Api(), policy, checkpoint_spec, gates)
        record = execution_evidence.build_evidence_record(
            checkpoint_id="9C", implementation_sha=sha, implementation_subject="feat: mobile editor UX",
            gates=gates, developer_preview={"required": False},
            contract_versions=contract_versions(), evidence_classes=proofs,
        )
        execution_evidence.validate_evidence_record(record, checkpoint_id="9C", checkpoint=checkpoint_spec, policy=policy)
        self.assertEqual({proof["class"] for proof in proofs}, {"STATIC", "UNIT", "INTEGRATION", "USER_JOURNEY"})

    def test_in_flight_amendment_baseline_excludes_product_commits(self) -> None:
        plan, state = nine_b_plan_state()
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            subprocess.run(["git", "init", "-q", "-b", "main", str(root)], check=True)
            subprocess.run(["git", "config", "user.name", "Execution Test"], cwd=root, check=True)
            subprocess.run(["git", "config", "user.email", "execution-test@example.invalid"], cwd=root, check=True)
            parent = commit_test_files(root, {
                "docs/execution/PLAN.json": git_blob_from_main("5b6a496", "docs/execution/PLAN.json"),
                "docs/execution/STATE.json": git_blob_from_main("5b6a496", "docs/execution/STATE.json"),
            }, "original 9B implementation")
            changed = sorted(("docs/execution/PLAN.json", "docs/execution/STATE.json", agent_supervisor.AMENDMENT_MARKER))
            marker = {"schema_version": 1, "checkpoint_id": "9B", "prior_implementation_sha": parent,
                      "prior_failed_run_id": 37043830370, "changed_paths": changed}
            amendment = commit_test_files(root, {
                "docs/execution/PLAN.json": json.dumps(plan),
                "docs/execution/STATE.json": json.dumps(state),
                agent_supervisor.AMENDMENT_MARKER: json.dumps(marker),
            }, "chore(execution): amend 9B quality baseline")
            agent_supervisor._validate_amendment_baseline(root, amendment)
            subprocess.run(["git", "checkout", "-q", parent], cwd=root, check=True)
            altered_state = json.loads(json.dumps(state))
            altered_state["verified_contract_versions"]["project_schema"] = 8
            bad_versions = commit_test_files(root, {
                "docs/execution/PLAN.json": json.dumps(plan),
                "docs/execution/STATE.json": json.dumps(altered_state),
                agent_supervisor.AMENDMENT_MARKER: json.dumps(marker),
            }, "untrusted schema advance")
            with self.assertRaisesRegex(agent_supervisor.SupervisorError, "retained execution state"):
                agent_supervisor._validate_amendment_baseline(root, bad_versions)
            subprocess.run(["git", "checkout", "-q", amendment], cwd=root, check=True)
            marker["prior_implementation_sha"] = amendment
            marker["changed_paths"] = sorted((agent_supervisor.AMENDMENT_MARKER, "scripts/test_execution_infra.py"))
            amendment = commit_test_files(root, {
                agent_supervisor.AMENDMENT_MARKER: json.dumps(marker),
                "scripts/test_execution_infra.py": "# Historical fixture correction\n",
            }, "chore(execution): correct fixtures under the trusted baseline")
            agent_supervisor._validate_amendment_baseline(root, amendment)
            implementation = commit_test_files(root, {"apps/or_app/lib/fix.dart": "fix\n"}, "fix(android): SAF preview")
            self.assertEqual(agent_supervisor._resume_baseline(root, implementation), amendment)
            marker["prior_implementation_sha"] = implementation
            untrusted_followup = commit_test_files(root, {
                agent_supervisor.AMENDMENT_MARKER: json.dumps(marker),
                "scripts/test_execution_infra.py": "# Cannot trust intervening product work\n",
            }, "untrusted follow-up after product commit")
            with self.assertRaisesRegex(agent_supervisor.SupervisorError, "must introduce"):
                agent_supervisor._validate_amendment_baseline(root, untrusted_followup)
            subprocess.run(["git", "checkout", "-q", implementation], cwd=root, check=True)
            state = json.loads((root / "docs/execution/STATE.json").read_text())
            state["last_updated"] = "2026-10-04"
            inherited = commit_test_files(root, {"docs/execution/STATE.json": json.dumps(state)}, "untrusted later state baseline")
            with self.assertRaisesRegex(agent_supervisor.SupervisorError, "must introduce"):
                agent_supervisor._validate_amendment_baseline(root, inherited)
            marker["prior_implementation_sha"] = inherited
            marker["changed_paths"] = sorted((agent_supervisor.AMENDMENT_MARKER, "apps/or_app/lib/other.dart"))
            commit_test_files(root, {agent_supervisor.AMENDMENT_MARKER: json.dumps(marker),
                                     "apps/or_app/lib/other.dart": "bad\n"}, "untrusted mixed amendment")
            with self.assertRaisesRegex(agent_supervisor.SupervisorError, "non-control-plane"):
                agent_supervisor._validate_amendment_baseline(root, agent_supervisor.git_output(root, "rev-parse", "HEAD"))


class EvidenceTests(unittest.TestCase):
    def test_repair_evidence_records_exact_head_origin_and_contract_versions(self) -> None:
        implementation_sha = "c" * 40
        failed_sha = "9" * 40
        plan, _state = execution_plan.load_plan_state(REPO_ROOT)
        checkpoint_spec = execution_plan.checkpoint_for_id(plan, "7F0")
        with (
            mock.patch.object(
                execution_evidence,
                "wait_for_required_workflows",
                return_value=[
                    valid_gate("repository_hygiene", implementation_sha),
                    valid_gate("platform_verification", implementation_sha, 102),
                ],
            ),
            mock.patch.object(
                execution_plan,
                "read_contract_versions",
                return_value=contract_versions(project=5),
            ),
        ):
            result = agent_supervisor.verify_hosted_checkpoint(
                REPO_ROOT,
                plan,
                checkpoint_spec,
                implementation_sha,
                "fix(execution): make contract gates transition-aware",
                implementation_origin_sha=failed_sha,
                api=FakeApi([]),
            )
        record = result["record"]
        self.assertEqual(record["implementation_sha"], implementation_sha)
        self.assertEqual(record["implementation_origin_sha"], failed_sha)
        self.assertEqual(record["contract_versions"], contract_versions(project=5))
        execution_evidence.validate_evidence_record(
            record,
            checkpoint_id="7F0",
            checkpoint=checkpoint_spec,
            policy=result["policy"],
        )

    def test_historical_evidence_without_optional_contract_fields_remains_valid(self) -> None:
        plan, _state = execution_plan.load_plan_state(REPO_ROOT)
        record = valid_evidence()
        record.pop("contract_versions")
        record.pop("implementation_origin_sha", None)
        execution_evidence.validate_evidence_record(
            record,
            checkpoint_id="7A",
            checkpoint=execution_plan.checkpoint_for_id(plan, "7A"),
            policy=evidence_policy(),
        )

    def test_repository_identity_supports_https_and_ssh_remotes(self) -> None:
        expected = ("huou07", "Opencut-Reinforced")
        for remote in (
            "https://github.com/huou07/Opencut-Reinforced.git",
            "git@github.com:huou07/Opencut-Reinforced.git",
            "ssh://git@github.com/huou07/Opencut-Reinforced",
        ):
            with self.subTest(remote=remote):
                self.assertEqual(execution_evidence.parse_repository_identity(remote), expected)

    def test_workflow_api_uses_filename_endpoint_with_repository_path_policy(self) -> None:
        path = execution_evidence._workflow_runs_path(
            evidence_policy(),
            ".github/workflows/platform-verification.yml",
            "a" * 40,
        )
        self.assertIn("/actions/workflows/platform-verification.yml/runs?", path)
        self.assertNotIn("/actions/workflows/.github", path)

    def test_exact_sha_successful_workflow_is_accepted(self) -> None:
        run = workflow_run()
        selected = execution_evidence.select_exact_workflow_run(
            {"workflow_runs": [run]},
            workflow_file=run["path"],  # type: ignore[arg-type]
            workflow_name=run["name"],  # type: ignore[arg-type]
            implementation_sha=run["head_sha"],  # type: ignore[arg-type]
            branch="main",
        )
        self.assertEqual(selected["id"], 101)

    def test_wrong_sha_is_rejected(self) -> None:
        with self.assertRaises(execution_evidence.WorkflowPending):
            execution_evidence.select_exact_workflow_run(
                {"workflow_runs": [workflow_run("b" * 40)]},
                workflow_file=".github/workflows/platform-verification.yml",
                workflow_name="Platform verification",
                implementation_sha="a" * 40,
                branch="main",
            )

    def test_wrong_branch_is_rejected(self) -> None:
        with self.assertRaises(execution_evidence.EvidenceError):
            execution_evidence.select_exact_workflow_run(
                {"workflow_runs": [workflow_run(branch="feature") ]},
                workflow_file=".github/workflows/platform-verification.yml",
                workflow_name="Platform verification",
                implementation_sha="a" * 40,
                branch="main",
            )

    def test_pull_request_and_workflow_dispatch_events_are_rejected_for_product_gates(self) -> None:
        for event in ("pull_request", "workflow_dispatch"):
            with self.subTest(event=event), self.assertRaises(execution_evidence.EvidenceError):
                execution_evidence.select_exact_workflow_run(
                    {"workflow_runs": [workflow_run(event=event)]},
                    workflow_file=".github/workflows/platform-verification.yml",
                    workflow_name="Platform verification",
                    implementation_sha="a" * 40,
                    branch="main",
                )

    def test_queued_and_in_progress_runs_are_not_success(self) -> None:
        for status in ("queued", "in_progress"):
            with self.subTest(status=status), self.assertRaises(execution_evidence.WorkflowPending):
                execution_evidence.select_exact_workflow_run(
                    {"workflow_runs": [workflow_run(status=status, conclusion=None)]},
                    workflow_file=".github/workflows/platform-verification.yml",
                    workflow_name="Platform verification",
                    implementation_sha="a" * 40,
                    branch="main",
                )

    def test_failed_and_cancelled_runs_are_rejected(self) -> None:
        for conclusion in ("failure", "cancelled"):
            with self.subTest(conclusion=conclusion), self.assertRaises(execution_evidence.WorkflowFailed):
                execution_evidence.select_exact_workflow_run(
                    {"workflow_runs": [workflow_run(conclusion=conclusion)]},
                    workflow_file=".github/workflows/platform-verification.yml",
                    workflow_name="Platform verification",
                    implementation_sha="a" * 40,
                    branch="main",
                )

    def test_failed_workflow_reports_run_url_and_failed_required_job(self) -> None:
        failed = workflow_run(conclusion="failure")
        api = SequenceApi(
            [
                {"workflow_runs": [failed]},
                {
                    "jobs": [
                        {
                            "name": "Rust checks",
                            "status": "completed",
                            "conclusion": "failure",
                        }
                    ]
                },
            ]
        )
        with self.assertRaises(execution_evidence.WorkflowFailed) as context:
            execution_evidence.wait_for_workflow_gate(
                api, evidence_policy(), "platform_verification", "a" * 40
            )
        message = str(context.exception)
        self.assertIn("platform_verification", message)
        self.assertIn("101", message)
        self.assertIn("actions/runs/101", message)
        self.assertIn("Rust checks (completed/failure)", message)

    def test_newer_duplicate_exact_run_wins_deterministically(self) -> None:
        older = workflow_run(run_id=101, updated_at="2026-09-28T00:00:00Z")
        newer = workflow_run(run_id=202, updated_at="2026-09-28T00:01:00Z")
        selected = execution_evidence.select_exact_workflow_run(
            {"workflow_runs": [newer, older]},
            workflow_file=".github/workflows/platform-verification.yml",
            workflow_name="Platform verification",
            implementation_sha="a" * 40,
            branch="main",
        )
        self.assertEqual(selected["id"], 202)

    def test_malformed_workflow_response_is_rejected(self) -> None:
        with self.assertRaises(execution_evidence.EvidenceError):
            execution_evidence.select_exact_workflow_run(
                {"workflow_runs": [{"head_sha": "a" * 40}]},
                workflow_file=".github/workflows/platform-verification.yml",
                workflow_name="Platform verification",
                implementation_sha="a" * 40,
                branch="main",
            )

    def test_required_jobs_must_exist_and_succeed(self) -> None:
        required = ["Rust checks", "Flutter static and widget checks"]
        good = [{"name": name, "status": "completed", "conclusion": "success"} for name in required]
        self.assertEqual(
            execution_evidence.validate_required_jobs({"jobs": good}, required), good
        )
        with self.assertRaises(execution_evidence.EvidenceError):
            execution_evidence.validate_required_jobs({"jobs": good[:1]}, required)
        failed = [dict(good[0]), dict(good[1])]
        failed[1]["conclusion"] = "failure"
        with self.assertRaises(execution_evidence.EvidenceError):
            execution_evidence.validate_required_jobs({"jobs": failed}, required)

    def test_timeout_is_testable_without_sleeping(self) -> None:
        api = FakeApi({"workflow_runs": [workflow_run(status="queued", conclusion=None)]})
        clock = AdvancingClock([0.0, 0.5, 1.1])
        with self.assertRaises(execution_evidence.EvidenceTimeout):
            execution_evidence.wait_for_workflow_gate(
                api,
                evidence_policy(),
                "platform_verification",
                "a" * 40,
                timeout_seconds=1,
                clock=clock,
                sleep=lambda _seconds: None,
            )

    def test_token_preference_and_polling_intervals(self) -> None:
        policy = evidence_policy()
        self.assertEqual(
            execution_evidence.select_token({"GH_TOKEN": "first", "GITHUB_TOKEN": "second"}),
            "first",
        )
        self.assertEqual(execution_evidence.select_token({"GITHUB_TOKEN": "second"}), "second")
        self.assertIsNone(execution_evidence.select_token({}))
        self.assertEqual(execution_evidence.polling_interval(policy, True), 15)
        self.assertEqual(execution_evidence.polling_interval(policy, False), 90)

    def test_policy_rejects_required_gate_contract_mutation(self) -> None:
        policy = evidence_policy()
        policy["required_gates"]["platform_verification"]["required_jobs"] = []  # type: ignore[index]
        with self.assertRaises(execution_evidence.EvidenceError):
            execution_evidence.validate_policy(policy)

    def test_workflow_wait_verifies_exact_run_and_all_jobs(self) -> None:
        policy = evidence_policy()
        gate = policy["required_gates"]["platform_verification"]  # type: ignore[index]
        run = workflow_run(
            workflow_file=gate["workflow_file"],  # type: ignore[arg-type]
            workflow_name=gate["workflow_name"],  # type: ignore[arg-type]
        )
        jobs = {
            "jobs": [
                {"name": name, "status": "completed", "conclusion": "success"}
                for name in gate["required_jobs"]  # type: ignore[index]
            ]
        }
        api = SequenceApi([{"workflow_runs": [run]}, jobs])
        result = execution_evidence.wait_for_workflow_gate(
            api, policy, "platform_verification", "a" * 40
        )
        self.assertEqual(result["run_id"], 101)
        self.assertEqual(len(result["jobs"]), 6)
        self.assertIn("/actions/workflows/platform-verification.yml/runs?", api.calls[0])

    def test_token_is_not_exposed_in_api_error_text(self) -> None:
        secret = "do-not-print-this-token"

        def opener(_request: object, **_kwargs: object) -> object:
            raise urllib.error.HTTPError(
                "https://api.github.com", 429, "rate limited", {"X-RateLimit-Remaining": "0"}, None
            )

        api = execution_evidence.GitHubApi("huou07", "Opencut-Reinforced", token=secret, opener=opener)
        with self.assertRaises(execution_evidence.RateLimitError) as context:
            api.get("/repos/huou07/Opencut-Reinforced/actions/runs")
        self.assertNotIn(secret, str(context.exception))

    def test_success_response_with_exhausted_rate_limit_stops(self) -> None:
        def opener(_request: object, **_kwargs: object) -> ApiResponse:
            return ApiResponse({}, {"X-RateLimit-Remaining": "0"})

        api = execution_evidence.GitHubApi(
            "huou07", "Opencut-Reinforced", opener=opener
        )
        with self.assertRaises(execution_evidence.RateLimitError):
            api.get("/repos/huou07/Opencut-Reinforced/actions/runs")

    def test_preview_release_contract(self) -> None:
        policy = evidence_policy()
        sha = "a" * 40
        tag = execution_evidence.preview_tag(sha, policy)
        asset_names = policy["developer_preview"]["required_assets"]  # type: ignore[index]
        release = {
            "tag_name": tag,
            "prerelease": True,
            "html_url": "https://github.com/huou07/Opencut-Reinforced/releases/tag/" + tag,
            "assets": [{"name": name, "size": 1} for name in asset_names],
        }
        self.assertEqual(
            len(execution_evidence.validate_preview_release(release, tag=tag, implementation_sha=sha, policy=policy)),
            11,
        )
        self.assertEqual(len(asset_names), 11)
        self.assertEqual(
            asset_names[-2:],
            ["FFMPEG-BUILD-INFO.txt", "ffmpeg-8.1.3-source.tar.xz"],
        )
        for mutation in (
            {"tag_name": "wrong"},
            {"prerelease": False},
        ):
            invalid = dict(release)
            invalid.update(mutation)
            with self.subTest(mutation=mutation), self.assertRaises(execution_evidence.EvidenceError):
                execution_evidence.validate_preview_release(invalid, tag=tag, implementation_sha=sha, policy=policy)
        for missing in ("SHA256SUMS.txt", "BUILD-INFO.txt", asset_names[0]):
            invalid = dict(release)
            invalid["assets"] = [asset for asset in release["assets"] if asset["name"] != missing]
            with self.subTest(missing=missing), self.assertRaises(execution_evidence.EvidenceError):
                execution_evidence.validate_preview_release(invalid, tag=tag, implementation_sha=sha, policy=policy)
        invalid = dict(release)
        invalid["assets"] = [dict(asset) for asset in release["assets"]]
        invalid["assets"][0]["size"] = 0
        with self.assertRaises(execution_evidence.EvidenceError):
            execution_evidence.validate_preview_release(invalid, tag=tag, implementation_sha=sha, policy=policy)

    def test_preview_workflow_failure_is_rejected_without_network(self) -> None:
        policy = evidence_policy()
        preview = policy["developer_preview"]  # type: ignore[index]
        with self.assertRaises(execution_evidence.WorkflowFailed):
            execution_evidence.select_exact_workflow_run(
                {"workflow_runs": [workflow_run(conclusion="failure", workflow_file=preview["workflow_file"], workflow_name=preview["workflow_name"], event="workflow_dispatch")]},
                workflow_file=preview["workflow_file"],  # type: ignore[arg-type]
                workflow_name=preview["workflow_name"],  # type: ignore[arg-type]
                implementation_sha="a" * 40,
                branch="main",
                allowed_events=tuple(preview["allowed_events"]),  # type: ignore[arg-type]
            )

    def test_preview_source_sha_must_match_exact_implementation(self) -> None:
        with self.assertRaises(execution_evidence.EvidenceError):
            execution_evidence.validate_preview_source_sha("b" * 40, "a" * 40)

    def test_required_preview_verification_uses_exact_release_and_tag(self) -> None:
        policy = evidence_policy()
        sha = "a" * 40
        preview = policy["developer_preview"]  # type: ignore[index]
        run = workflow_run(
            event="workflow_dispatch",
            workflow_file=preview["workflow_file"],  # type: ignore[arg-type]
            workflow_name=preview["workflow_name"],  # type: ignore[arg-type]
        )
        jobs = {
            "jobs": [
                {
                    "name": preview["publish_job"],  # type: ignore[index]
                    "status": "completed",
                    "conclusion": "success",
                }
            ]
        }
        tag = execution_evidence.preview_tag(sha, policy)
        release = {
            "tag_name": tag,
            "prerelease": True,
            "html_url": f"https://github.com/huou07/Opencut-Reinforced/releases/tag/{tag}",
            "assets": [
                {"name": name, "size": 1}
                for name in preview["required_assets"]  # type: ignore[index]
            ],
        }
        api = SequenceApi(
            [
                {"workflow_runs": [run]},
                jobs,
                release,
                {"object": {"type": "commit", "sha": sha}},
            ]
        )
        result = execution_evidence.wait_for_developer_preview(
            api, policy, sha, sleep=lambda _seconds: None
        )
        self.assertEqual(result["tag"], tag)
        self.assertEqual(result["source_sha"], sha)
        self.assertEqual(len(result["assets"]), 11)

    def test_required_preview_without_token_stops_before_dispatch(self) -> None:
        policy = evidence_policy()
        api = SequenceApi([{"workflow_runs": []}], authenticated=False)
        with self.assertRaises(execution_evidence.PreviewRequiredError):
            execution_evidence.wait_for_developer_preview(
                api, policy, "a" * 40, sleep=lambda _seconds: None
            )
        self.assertEqual(api.posts, [])

    def test_authenticated_preview_dispatches_only_existing_workflow(self) -> None:
        policy = evidence_policy()
        sha = "a" * 40
        preview = policy["developer_preview"]  # type: ignore[index]
        run = workflow_run(
            event="workflow_dispatch",
            workflow_file=preview["workflow_file"],  # type: ignore[arg-type]
            workflow_name=preview["workflow_name"],  # type: ignore[arg-type]
        )
        jobs = {
            "jobs": [
                {
                    "name": preview["publish_job"],  # type: ignore[index]
                    "status": "completed",
                    "conclusion": "success",
                }
            ]
        }
        tag = execution_evidence.preview_tag(sha, policy)
        release = {
            "tag_name": tag,
            "prerelease": True,
            "html_url": f"https://github.com/huou07/Opencut-Reinforced/releases/tag/{tag}",
            "assets": [
                {"name": name, "size": 1}
                for name in preview["required_assets"]  # type: ignore[index]
            ],
        }
        api = SequenceApi(
            [
                {"workflow_runs": []},
                {"workflow_runs": [run]},
                jobs,
                release,
                {"object": {"type": "commit", "sha": sha}},
            ],
            authenticated=True,
        )
        execution_evidence.wait_for_developer_preview(
            api, policy, sha, sleep=lambda _seconds: None
        )
        self.assertEqual(
            api.posts,
            [
                (
                    "/repos/huou07/Opencut-Reinforced/actions/workflows/developer-preview.yml/dispatches",
                    {"ref": "main"},
                )
            ],
        )

    def test_required_preview_evidence_is_structurally_valid(self) -> None:
        sha = "a" * 40
        record = valid_evidence()
        record["checkpoint_id"] = "7H"
        record["developer_preview"] = valid_preview(sha)
        checkpoint = {"developer_preview_required": True}
        execution_evidence.validate_evidence_record(
            record,
            checkpoint_id="7H",
            checkpoint=checkpoint,
            policy=evidence_policy(),
        )

    def test_evidence_timestamp_must_be_utc_and_preview_assets_nonempty(self) -> None:
        record = valid_evidence()
        record["verified_at_utc"] = "2026-09-28T07:00:00+07:00"
        with self.assertRaises(execution_evidence.EvidenceError):
            execution_evidence.validate_evidence_record(
                record,
                checkpoint_id="7A",
                checkpoint={"developer_preview_required": False},
                policy=evidence_policy(),
            )

        record = valid_evidence()
        record["checkpoint_id"] = "7H"
        preview = valid_preview()
        preview["assets"] = [dict(asset) for asset in preview["assets"]]
        preview["assets"][0]["size"] = 0
        record["developer_preview"] = preview
        with self.assertRaises(execution_evidence.EvidenceError):
            execution_evidence.validate_evidence_record(
                record,
                checkpoint_id="7H",
                checkpoint={"developer_preview_required": True},
                policy=evidence_policy(),
            )

    def test_invalid_evidence_cannot_advance_or_write_state(self) -> None:
        plan, state = fixture_plan_state(["7A", "7B"])
        checkpoint_spec = plan["checkpoints"][0]  # type: ignore[index]
        valid = valid_evidence()
        invalid_records = []

        wrong_checkpoint = dict(valid)
        wrong_checkpoint["checkpoint_id"] = "7B"
        invalid_records.append((wrong_checkpoint, "7A", checkpoint_spec))

        missing_gate = dict(valid)
        missing_gate["gates"] = valid["gates"][:1]  # type: ignore[index]
        invalid_records.append((missing_gate, "7A", checkpoint_spec))

        failed_gate = json.loads(json.dumps(valid))
        failed_gate["gates"][0]["conclusion"] = "failure"
        invalid_records.append((failed_gate, "7A", checkpoint_spec))

        with tempfile.TemporaryDirectory() as directory:
            state_path = Path(directory) / "STATE.json"
            before = json.dumps(state, sort_keys=True).encode("utf-8")
            state_path.write_bytes(before)
            for record, checkpoint_id, checkpoint_value in invalid_records:
                with self.subTest(checkpoint_id=checkpoint_id, record=record):
                    with self.assertRaises(execution_evidence.EvidenceError):
                        execution_evidence.validate_evidence_record(
                            record,
                            checkpoint_id=checkpoint_id,
                            checkpoint=checkpoint_value,
                            policy=evidence_policy(),
                        )
                    self.assertEqual(state_path.read_bytes(), before)

            evidence_result = {
                "policy": evidence_policy(),
                "record": valid,
                "api": FakeApi(None),
            }
            with self.assertRaises(agent_supervisor.SupervisorError):
                agent_supervisor.finalize_verified_checkpoint(
                    Path(directory),
                    plan=plan,
                    state=state,
                    checkpoint=checkpoint_spec,
                    evidence_result=evidence_result,
                    implementation_sha="b" * 40,
                    api=FakeApi(None),
                    run_local_checks=False,
                )
            self.assertEqual(state_path.read_bytes(), before)


class SupervisorBoundaryTests(unittest.TestCase):
    def test_porcelain_status_paths_preserve_status_columns(self) -> None:
        self.assertEqual(
            agent_supervisor.parse_porcelain_status_paths(
                " M docs/execution/STATE.json\n"
                "?? docs/execution/evidence/7A.json\n"
            ),
            [
                "docs/execution/STATE.json",
                "docs/execution/evidence/7A.json",
            ],
        )

    def test_runner_state_and_control_surface_mutations_are_rejected(self) -> None:
        before = {
            "docs/execution/STATE.json": b"state",
            "docs/execution/PLAN.json": b"plan",
            "docs/execution/phases/PHASE_7.md": b"phase",
            "docs/execution/evidence/fake.json": None,
            "scripts/agent_supervisor.py": b"supervisor",
        }
        for path in before:
            after = dict(before)
            after[path] = b"changed"
            with self.subTest(path=path), self.assertRaises(agent_supervisor.SupervisorError):
                agent_supervisor.assert_protected_surfaces_unchanged(before, after)

    def test_normal_checkpoint_rejects_protected_workflow_mutation(self) -> None:
        before = {".github/workflows/platform-verification.yml": b"workflow"}
        after = {".github/workflows/platform-verification.yml": b"changed"}
        with self.assertRaises(agent_supervisor.SupervisorError):
            agent_supervisor.assert_protected_surfaces_unchanged(before, after)

    def test_checkpoint_workflow_allowances_are_exact_and_reject_other_protected_paths(self) -> None:
        plan, state = execution_plan.load_plan_state(REPO_ROOT)
        execution_plan.validate_plan(plan, state, REPO_ROOT)
        platform_path = ".github/workflows/platform-verification.yml"
        preview_path = ".github/workflows/developer-preview.yml"
        expected_allowances = {
            "7C0": [platform_path],
            "7C": [platform_path],
            "7E": [platform_path],
            "7F0": [platform_path],
            "7F1": [platform_path],
            "7F": [platform_path],
            "7H": [platform_path, preview_path],
            "8D": [platform_path],
            "8E": [platform_path],
            "8F": [platform_path, preview_path],
            "9A0": [platform_path],
            "9A": [platform_path],
            "9B": [platform_path],
            "9B1": [platform_path, preview_path],
            "9D": [platform_path],
            "9E": [platform_path],
            "10A": [platform_path],
            "16A": [platform_path],
            "16G": [platform_path],
        }
        forbidden_paths = (
            ".github/workflows/release.yml",
            "docs/execution/STATE.json",
            "docs/execution/PLAN.json",
            "docs/execution/EVIDENCE_POLICY.json",
            "docs/execution/phases/PHASE_7.md",
            "scripts/agent_supervisor.py",
            "docs/execution/evidence/7C0.json",
        )
        before = {
            path: b"baseline"
            for path in (platform_path, preview_path, *forbidden_paths)
        }
        for checkpoint_id, allowed_paths in expected_allowances.items():
            with self.subTest(checkpoint_id=checkpoint_id):
                checkpoint = execution_plan.checkpoint_for_id(plan, checkpoint_id)
                self.assertEqual(checkpoint["runner_allowed_protected_paths"], allowed_paths)
                after = dict(before)
                for path in allowed_paths:
                    after[path] = b"workflow changed"
                agent_supervisor.assert_protected_surfaces_unchanged(
                    before, after, allowed_paths
                )

                for path in forbidden_paths:
                    with self.subTest(path=path):
                        rejected = dict(after)
                        rejected[path] = b"also changed"
                        with self.assertRaises(agent_supervisor.SupervisorError):
                            agent_supervisor.assert_protected_surfaces_unchanged(
                                before, rejected, allowed_paths
                            )

    def test_verified_state_transition_is_exactly_one_checkpoint(self) -> None:
        plan, state = fixture_plan_state(["7A", "7B"])
        after = agent_supervisor.advance_state_once(state, plan, "7A", today="2026-09-28")
        self.assertEqual(after["checkpoints"]["7A"], "DONE")
        self.assertEqual(after["checkpoints"]["7B"], "NEXT")
        self.assertEqual(after["current_next"], "7B")

    def test_7c0_state_advancement_makes_only_7c_next(self) -> None:
        plan, state = execution_plan.load_plan_state(REPO_ROOT)
        state["checkpoints"] = dict(state["checkpoints"])  # type: ignore[arg-type]
        state["verified_contract_versions"] = contract_versions(project=4)
        checkpoints = state["checkpoints"]
        after_7b = False
        for planned_checkpoint in plan["checkpoints"]:  # type: ignore[union-attr]
            checkpoint_id = planned_checkpoint["id"]
            if after_7b:
                checkpoints[checkpoint_id] = "PLANNED"  # type: ignore[index]
            if checkpoint_id == "7B":
                after_7b = True
        checkpoints["7C0"] = "NEXT"  # type: ignore[index]
        state["current_next"] = "7C0"
        candidate = contract_versions(project=4)
        after = agent_supervisor.advance_state_once(
            state,
            plan,
            "7C0",
            candidate_contract_versions=candidate,
            today="2026-09-29",
        )
        self.assertEqual(after["checkpoints"]["7B"], "DONE")
        self.assertEqual(after["checkpoints"]["7C0"], "DONE")
        self.assertEqual(after["checkpoints"]["7C"], "NEXT")
        self.assertEqual(after["checkpoints"]["7D"], "PLANNED")
        self.assertEqual(after["current_next"], "7C")
        self.assertEqual(
            agent_supervisor.assert_state_advanced_once(
                state, after, plan, candidate_contract_versions=candidate
            ),
            "7C0",
        )

    def test_same_phase_transition_keeps_phase_in_progress(self) -> None:
        plan, _ = execution_plan.load_plan_state(REPO_ROOT)
        state = roadmap_state_before(plan, "7F", contract_versions(project=5))
        after = agent_supervisor.advance_state_once(
            state,
            plan,
            "7F",
            candidate_contract_versions=contract_versions(project=5),
        )
        self.assertEqual(after["phase_status"]["7"], "IN_PROGRESS")  # type: ignore[index]
        self.assertEqual(after["current_next"], "7G")

    def test_7h_to_8a_completes_phase_7_and_starts_phase_8(self) -> None:
        plan, _ = execution_plan.load_plan_state(REPO_ROOT)
        versions = contract_versions(project=5)
        state = roadmap_state_before(plan, "7H", versions)
        after = agent_supervisor.advance_state_once(
            state, plan, "7H", candidate_contract_versions=versions
        )
        self.assertEqual(after["phase_status"]["7"], "DONE")  # type: ignore[index]
        self.assertEqual(after["phase_status"]["8"], "IN_PROGRESS")  # type: ignore[index]
        self.assertEqual(after["current_next"], "8A")

    def test_8f_to_9a0_completes_phase_8_and_starts_phase_9(self) -> None:
        plan, _ = execution_plan.load_plan_state(REPO_ROOT)
        versions = contract_versions(project=5)
        state = roadmap_state_before(plan, "8F", versions)
        after = agent_supervisor.advance_state_once(
            state, plan, "8F", candidate_contract_versions=versions
        )
        self.assertEqual(after["phase_status"]["8"], "DONE")  # type: ignore[index]
        self.assertEqual(after["phase_status"]["9"], "IN_PROGRESS")  # type: ignore[index]
        self.assertEqual(after["current_next"], "9A0")

    def test_16g_completion_marks_phase_done_and_clears_next(self) -> None:
        plan, _ = execution_plan.load_plan_state(REPO_ROOT)
        versions = contract_versions(project=5)
        state = roadmap_state_before(plan, "16G", versions)
        after = agent_supervisor.advance_state_once(
            state, plan, "16G", candidate_contract_versions=versions
        )
        self.assertEqual(after["phase_status"]["16"], "DONE")  # type: ignore[index]
        self.assertIsNone(after["current_next"])
        self.assertTrue(all(status == "DONE" for status in after["checkpoints"].values()))  # type: ignore[union-attr]
        policy = execution_plan.load_architecture_policy(REPO_ROOT)
        self.assertEqual(
            execution_plan.validate_contract_transition(plan, after, policy, versions), versions
        )
        with self.assertRaises(execution_plan.PlanError):
            execution_plan.validate_contract_transition(
                plan, after, policy, contract_versions(project=6)
            )

    def test_resume_baseline_uses_exact_workflow_allowance(self) -> None:
        allowed_path = ".github/workflows/platform-verification.yml"
        additional_paths = (
            ".github/workflows/release.yml",
            "docs/execution/STATE.json",
            "docs/execution/PLAN.json",
            "docs/execution/phases/PHASE_7.md",
            "scripts/agent_supervisor.py",
            "docs/execution/evidence/7C0.json",
        )
        cases = [([allowed_path], True)]
        cases.extend(([allowed_path, path], False) for path in additional_paths)
        cases.append(([allowed_path], False))

        for changed_paths, should_pass in cases:
            allowed = [allowed_path] if should_pass or len(changed_paths) > 1 else []
            with self.subTest(changed_paths=changed_paths, allowed=allowed):
                with tempfile.TemporaryDirectory() as directory:
                    root = Path(directory)
                    subprocess.run(["git", "init", "-q", str(root)], check=True)
                    subprocess.run(
                        ["git", "config", "user.name", "Execution Test"],
                        cwd=root,
                        check=True,
                    )
                    subprocess.run(
                        ["git", "config", "user.email", "execution-test@example.invalid"],
                        cwd=root,
                        check=True,
                    )
                    baseline_paths = {
                        allowed_path,
                        *additional_paths,
                        "docs/execution/STATE.json",
                    }
                    for relative_path in baseline_paths:
                        path = root / relative_path
                        path.parent.mkdir(parents=True, exist_ok=True)
                        path.write_text("baseline\n", encoding="utf-8")
                    subprocess.run(["git", "add", "."], cwd=root, check=True)
                    subprocess.run(
                        ["git", "commit", "-qm", "state baseline"], cwd=root, check=True
                    )
                    baseline = subprocess.run(
                        ["git", "rev-parse", "HEAD"],
                        cwd=root,
                        capture_output=True,
                        text=True,
                        check=True,
                    ).stdout.strip()
                    for relative_path in changed_paths:
                        (root / relative_path).write_text("implementation\n", encoding="utf-8")
                    subprocess.run(["git", "add", "."], cwd=root, check=True)
                    subprocess.run(
                        ["git", "commit", "-qm", "implementation"], cwd=root, check=True
                    )
                    implementation_sha = subprocess.run(
                        ["git", "rev-parse", "HEAD"],
                        cwd=root,
                        capture_output=True,
                        text=True,
                        check=True,
                    ).stdout.strip()

                    if should_pass:
                        self.assertEqual(
                            agent_supervisor._resume_baseline(root, implementation_sha, allowed),
                            baseline,
                        )
                    else:
                        with self.assertRaises(agent_supervisor.SupervisorError):
                            agent_supervisor._resume_baseline(
                                root, implementation_sha, allowed
                            )

    def test_state_commit_path_rejects_any_third_file(self) -> None:
        agent_supervisor.validate_state_commit_paths(
            ["docs/execution/STATE.json", "docs/execution/evidence/7A.json"], "7A"
        )
        with self.assertRaises(agent_supervisor.SupervisorError):
            agent_supervisor.validate_state_commit_paths(
                [
                    "docs/execution/STATE.json",
                    "docs/execution/evidence/7A.json",
                    "README.md",
                ],
                "7A",
            )

    def test_resume_preconditions_cover_exact_sha_state_and_git_guards(self) -> None:
        sha = "a" * 40
        valid = {
            "resume_sha": sha,
            "head": sha,
            "origin_main": sha,
            "current_next": "7A",
            "expected_checkpoint": "7A",
            "worktree_clean": True,
            "baseline_is_ancestor": True,
        }
        agent_supervisor.validate_resume_preconditions(**valid)
        for key, value in (
            ("resume_sha", "b" * 40),
            ("head", "b" * 40),
            ("origin_main", "b" * 40),
            ("current_next", "7B"),
        ):
            invalid = dict(valid)
            invalid[key] = value
            with self.subTest(key=key), self.assertRaises(agent_supervisor.SupervisorError):
                agent_supervisor.validate_resume_preconditions(**invalid)
        for key in ("worktree_clean", "baseline_is_ancestor"):
            invalid = dict(valid)
            invalid[key] = False
            with self.subTest(key=key), self.assertRaises(agent_supervisor.SupervisorError):
                agent_supervisor.validate_resume_preconditions(**invalid)

    def test_platform_path_filter_only_skips_metadata_changes(self) -> None:
        self.assertFalse(
            execution_evidence.platform_verification_runs_for_push(
                ["docs/execution/STATE.json", "docs/execution/evidence/7A.json"]
            )
        )
        self.assertTrue(
            execution_evidence.platform_verification_runs_for_push(
                ["docs/execution/STATE.json", "scripts/execution_plan.py"]
            )
        )
        workflow = (REPO_ROOT / ".github/workflows/platform-verification.yml").read_text(encoding="utf-8")
        self.assertIn("paths-ignore:", workflow)
        self.assertIn("docs/execution/STATE.json", workflow)
        self.assertIn("docs/execution/evidence/**", workflow)

    def test_prompt_requires_runner_handoff_contract(self) -> None:
        prompt = agent_supervisor.checkpoint_prompt(
            REPO_ROOT,
            {
                "goal": "checkpoint:7A",
                "checkpoint_id": "7A",
                "title": "Realtime architecture and capability foundation",
                "phase": 7,
                "spec_document": "docs/execution/phases/PHASE_7.md",
                "next_checkpoint_relation": "7B",
            },
        )
        self.assertIn("IMPLEMENTED — AWAITING SUPERVISOR EVIDENCE", prompt)
        self.assertIn("Do not edit PLAN.json, STATE.json", prompt)
        self.assertIn("supervisor owns hosted verification", prompt)
        self.assertIn("protected workflow gates", prompt)
        self.assertNotIn("authorizes changes to exactly", prompt)

    def test_prompt_names_only_checkpoint_workflow_allowance(self) -> None:
        plan, state = seven_f_one_plan_state()
        expected_path = ".github/workflows/platform-verification.yml"
        for checkpoint_id in ("7C0", "7C", "7E", "7F0", "7F1", "7F", "8D", "8E"):
            checkpoint = execution_plan.checkpoint_for_id(plan, checkpoint_id)
            self.assertEqual(checkpoint["runner_allowed_protected_paths"], [expected_path])

        checkpoint_7d = execution_plan.checkpoint_for_id(plan, "7D")
        self.assertEqual(checkpoint_7d.get("runner_allowed_protected_paths", []), [])
        resolution = execution_plan.resolve_goal(plan, state, "checkpoint:7F1", REPO_ROOT)
        self.assertEqual(resolution["runner_allowed_protected_paths"], [expected_path])
        prompt = agent_supervisor.checkpoint_prompt(REPO_ROOT, resolution)
        self.assertIn("authorizes changes to exactly this protected workflow path", prompt)

        checkpoint_7h = execution_plan.checkpoint_for_id(plan, "7H")
        prompt_7h = agent_supervisor.checkpoint_prompt(
            REPO_ROOT,
            {
                "goal": "checkpoint:7H",
                "checkpoint_id": "7H",
                "title": checkpoint_7h["title"],
                "phase": checkpoint_7h["phase"],
                "spec_document": checkpoint_7h["spec_document"],
                "next_checkpoint_relation": checkpoint_7h["next_checkpoint_relation"],
                "runner_allowed_protected_paths": checkpoint_7h[
                    "runner_allowed_protected_paths"
                ],
            },
        )
        self.assertIn("authorizes changes to exactly this protected workflow path", prompt_7h)
        self.assertEqual(prompt_7h.count(f"  - {expected_path}\n"), 1)
        self.assertEqual(
            prompt_7h.count("  - .github/workflows/developer-preview.yml\n"), 1
        )
        self.assertIn("No other protected execution-control surface may change", prompt_7h)

    def test_prepare_goal_returns_the_existing_authoritative_prompt(self) -> None:
        plan, state = fixture_plan_state(["7A", "7B"])
        with (
            mock.patch.object(agent_supervisor, "ensure_start_state") as ensure_start_state,
            mock.patch.object(execution_plan, "load_plan_state", return_value=(plan, state)),
        ):
            prompt = agent_supervisor.prepare_goal(REPO_ROOT, "checkpoint:7A")

        resolution = execution_plan.resolve_goal(plan, state, "checkpoint:7A", REPO_ROOT)
        self.assertEqual(prompt, agent_supervisor.checkpoint_prompt(REPO_ROOT, resolution))
        ensure_start_state.assert_called_once_with(REPO_ROOT)

    def test_prepare_goal_does_not_invoke_a_runner(self) -> None:
        plan, state = fixture_plan_state(["7A", "7B"])
        with (
            mock.patch.object(agent_supervisor, "ensure_start_state"),
            mock.patch.object(execution_plan, "load_plan_state", return_value=(plan, state)),
            mock.patch.object(agent_supervisor, "invoke_runner") as invoke_runner,
        ):
            agent_supervisor.prepare_goal(REPO_ROOT, "checkpoint:7A")

        invoke_runner.assert_not_called()

    def test_prepare_goal_does_not_mutate_plan_state_or_evidence(self) -> None:
        plan_path = REPO_ROOT / "docs/execution/PLAN.json"
        state_path = REPO_ROOT / "docs/execution/STATE.json"
        evidence_dir = REPO_ROOT / "docs/execution/evidence"
        before = {
            plan_path: plan_path.read_bytes(),
            state_path: state_path.read_bytes(),
            evidence_dir: {
                path.name: path.read_bytes() for path in evidence_dir.iterdir() if path.is_file()
            },
        }
        with (
            mock.patch.object(agent_supervisor, "ensure_start_state"),
            mock.patch.object(execution_plan, "load_plan_state", return_value=nine_b_plan_state()),
        ):
            prompt = agent_supervisor.prepare_goal(REPO_ROOT, "checkpoint:9B")
        after = {
            plan_path: plan_path.read_bytes(),
            state_path: state_path.read_bytes(),
            evidence_dir: {
                path.name: path.read_bytes() for path in evidence_dir.iterdir() if path.is_file()
            },
        }

        self.assertEqual(before, after)
        self.assertIn("Checkpoint: 9B", prompt)
        self.assertTrue((evidence_dir / "9A.json").is_file())

    def test_prepare_cli_prints_prompt_without_running_a_runner(self) -> None:
        output = StringIO()
        with (
            mock.patch.object(agent_supervisor, "prepare_goal", return_value="prompt\n"),
            mock.patch.object(agent_supervisor, "invoke_runner") as invoke_runner,
            redirect_stdout(output),
        ):
            result = agent_supervisor.main(["--goal", "checkpoint:7C", "--prepare"])

        self.assertEqual(result, 0)
        self.assertEqual(output.getvalue(), "prompt\n")
        invoke_runner.assert_not_called()

    def test_cli_requires_exactly_one_execution_mode(self) -> None:
        invalid_modes = (
            ("--prepare", "--runner", "/tmp/runner"),
            ("--prepare", "--resume-sha", "a" * 40),
            ("--runner", "/tmp/runner", "--resume-sha", "a" * 40),
            (),
        )
        for mode in invalid_modes:
            with self.subTest(mode=mode), self.assertRaises(SystemExit) as error:
                agent_supervisor.main(["--goal", "checkpoint:7C", *mode])
            self.assertEqual(error.exception.code, 2)


class PreflightTests(unittest.TestCase):
    def _read_only_git(self, _repo_root: Path, *arguments: str) -> str:
        values = {
            ("status", "--porcelain", "--untracked-files=all"): "",
            ("branch", "--show-current"): "main",
            ("rev-parse", "HEAD"): "a" * 40,
            ("rev-parse", "origin/main"): "a" * 40,
            ("rev-list", "--left-right", "--count", "HEAD...origin/main"): "0\t0",
        }
        return values[arguments]

    def test_desktop_mvp_preflight_is_read_only_and_uses_no_runner_or_github(self) -> None:
        state_path = REPO_ROOT / "docs/execution/STATE.json"
        plan_path = REPO_ROOT / "docs/execution/PLAN.json"
        before_state = state_path.read_bytes()
        before_plan = plan_path.read_bytes()
        stdout = StringIO()
        with (
            mock.patch.object(agent_supervisor, "git_output", side_effect=self._read_only_git),
            mock.patch.object(execution_plan, "load_plan_state", return_value=seven_f_one_plan_state()),
            mock.patch.object(execution_plan, "read_contract_versions", return_value=contract_versions(project=5)),
            mock.patch.object(agent_supervisor, "invoke_runner") as invoke_runner,
            mock.patch.object(agent_supervisor, "verify_hosted_checkpoint") as verify,
            mock.patch.object(agent_supervisor, "_github_api_for_repo") as github_api,
            mock.patch.object(agent_supervisor.subprocess, "run", side_effect=AssertionError("preflight must not run subprocesses")),
            redirect_stdout(stdout),
        ):
            result = agent_supervisor.main(
                ["--goal", "milestone:desktop-mvp", "--preflight"]
            )
        self.assertEqual(result, 0)
        self.assertIn("PRECHECK PASS", stdout.getvalue())
        self.assertIn("current NEXT: 7F1", stdout.getvalue())
        self.assertIn("verified versions: project=5 recovery=1 ipc=1", stdout.getvalue())
        self.assertIn("candidate versions: project=5 recovery=1 ipc=1", stdout.getvalue())
        self.assertIn("remaining checkpoints checked: 10", stdout.getvalue())
        self.assertIn("model calls: 0", stdout.getvalue())
        invoke_runner.assert_not_called()
        verify.assert_not_called()
        github_api.assert_not_called()
        self.assertEqual(state_path.read_bytes(), before_state)
        self.assertEqual(plan_path.read_bytes(), before_plan)

    def _run_goal_with_plan(self, plan: dict[str, object], state: dict[str, object]) -> mock.Mock:
        runner = mock.Mock()
        with (
            mock.patch.object(agent_supervisor, "ensure_start_state"),
            mock.patch.object(agent_supervisor, "git_output", side_effect=self._read_only_git),
            mock.patch.object(
                execution_plan, "load_plan_state", return_value=(plan, state)
            ),
            mock.patch.object(agent_supervisor, "invoke_runner", runner),
        ):
            with self.assertRaises((agent_supervisor.SupervisorError, execution_plan.PlanError)):
                agent_supervisor.run_goal(REPO_ROOT, "checkpoint:7F1", "/runner")
        return runner

    def test_unknown_category_fails_before_runner(self) -> None:
        plan, state = seven_f_one_plan_state()
        execution_plan.checkpoint_for_id(plan, "7F1")["expected_project_schema_effect_category"] = "unknown"
        runner = self._run_goal_with_plan(plan, state)
        runner.assert_not_called()

    def test_illegal_transition_fails_before_runner(self) -> None:
        plan, state = seven_f_one_plan_state()
        with mock.patch.object(
            execution_plan,
            "read_contract_versions",
            return_value=contract_versions(project=6),
        ):
            runner = self._run_goal_with_plan(plan, state)
        runner.assert_not_called()

    def test_invalid_owner_derived_category_fails_before_runner(self) -> None:
        plan, state = seven_f_one_plan_state()
        execution_plan.checkpoint_for_id(plan, "8B")["expected_project_schema_effect_category"] = "16A-model-only"
        runner = self._run_goal_with_plan(plan, state)
        runner.assert_not_called()

    def test_invalid_workflow_allowlist_fails_before_runner(self) -> None:
        plan, state = seven_f_one_plan_state()
        checkpoint = execution_plan.checkpoint_for_id(plan, "7F1")
        checkpoint["runner_allowed_protected_paths"] = [".github/workflows/*.yml"]
        runner = self._run_goal_with_plan(plan, state)
        runner.assert_not_called()


class RepairResumeTests(unittest.TestCase):
    def _history(self, directory: str) -> tuple[Path, str, str]:
        root = Path(directory) / "repo"
        failed_sha, head = make_repair_history(root)
        return root, failed_sha, head

    def _reject(self, root: Path, failed_sha: str, message: str) -> None:
        with self.assertRaisesRegex(agent_supervisor.SupervisorError, message):
            agent_supervisor.validate_repair_resume_history(root, failed_sha, "7F1")

    def test_baseline_walk_stops_before_bootstrap_without_state(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root, failed_sha, _head = self._history(directory)
            baseline = agent_supervisor._state_baseline_for_checkpoint(
                root, failed_sha, "7F1"
            )
            first_parent = subprocess.run(
                ["git", "rev-parse", f"{baseline}^1"],
                cwd=root,
                capture_output=True,
                text=True,
                check=True,
            ).stdout.strip()
            bootstrap = subprocess.run(
                ["git", "rev-parse", f"{first_parent}^1"],
                cwd=root,
                capture_output=True,
                text=True,
                check=True,
            ).stdout.strip()
            missing_state = subprocess.run(
                ["git", "ls-tree", bootstrap, "--", "docs/execution/STATE.json"],
                cwd=root,
                capture_output=True,
                text=True,
                check=True,
            ).stdout
            expected = subprocess.run(
                ["git", "rev-parse", f"{failed_sha}^1"],
                cwd=root,
                capture_output=True,
                text=True,
                check=True,
            ).stdout.strip()
            self.assertEqual(baseline, expected)
            self.assertEqual(missing_state, "")

    def test_baseline_stops_at_active_contract_change_while_checkpoint_remains_next(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory) / "repo"
            failed_sha, _head = make_repair_history(root, previous_contract_active=True)
            baseline = agent_supervisor._state_baseline_for_checkpoint(
                root, failed_sha, "7F1"
            )
            previous = subprocess.run(
                ["git", "rev-parse", f"{baseline}^1"],
                cwd=root,
                capture_output=True,
                text=True,
                check=True,
            ).stdout.strip()
            previous_state = agent_supervisor._json_at_revision(
                root, previous, "docs/execution/STATE.json"
            )
            self.assertEqual(previous_state["current_next"], "7F1")
            self.assertEqual(previous_state["checkpoints"]["7F1"], "NEXT")
            self.assertNotEqual(
                agent_supervisor._active_checkpoint_fingerprint(root, previous, "7F1"),
                agent_supervisor._active_checkpoint_fingerprint(root, failed_sha, "7F1"),
            )
            expected = subprocess.run(
                ["git", "rev-parse", f"{failed_sha}^1"],
                cwd=root,
                capture_output=True,
                text=True,
                check=True,
            ).stdout.strip()
            self.assertEqual(baseline, expected)
            self.assertNotEqual(baseline, previous)

    def test_malformed_state_in_execution_history_fails_closed(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory) / "repo"
            failed_sha, _head = make_repair_history(
                root, historical_state=b"{malformed state\n"
            )
            with self.assertRaisesRegex(
                agent_supervisor.SupervisorError,
                "cannot read docs/execution/STATE.json",
            ):
                agent_supervisor._state_baseline_for_checkpoint(root, failed_sha, "7F1")

    def test_trusted_future_plan_and_spec_maintenance_with_version_metadata_is_allowed(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root, failed_sha, head = self._history(directory)
            proof = agent_supervisor.validate_repair_resume_history(root, failed_sha, "7F1")
        self.assertEqual(proof["head"], head)
        self.assertEqual(proof["failed_sha"], failed_sha)
        self.assertEqual(proof["verified_versions"], contract_versions(project=5))
        self.assertEqual(proof["candidate_versions"], contract_versions(project=5))
        self.assertEqual(proof["fingerprint"]["checkpoint"]["id"], "7F1")

    def test_test_only_correction_is_allowed_and_keeps_active_fingerprint(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root, failed_sha, _head = self._history(directory)
            test_path = "crates/or_ipc/tests/local_transport.rs"
            corrected_head = commit_test_files(
                root, {test_path: b"corrected exact catalog expectations\n"}, "fix stale IPC test"
            )
            original_fingerprint = agent_supervisor._active_checkpoint_fingerprint(
                root, failed_sha, "7F1"
            )
            corrected_fingerprint = agent_supervisor._active_checkpoint_fingerprint(
                root, corrected_head, "7F1"
            )
            proof = agent_supervisor.validate_repair_resume_history(root, failed_sha, "7F1")
            changed_paths = agent_supervisor._repair_history_paths(
                root, failed_sha, corrected_head
            )
        self.assertEqual(original_fingerprint, corrected_fingerprint)
        self.assertEqual(proof["head"], corrected_head)
        self.assertIn(test_path, changed_paths)

    def test_repair_test_surface_excludes_runtime_and_production_paths(self) -> None:
        allowed = (
            "crates/or_ipc/tests/local_transport.rs",
            "crates/or_core/tests/project_storage.rs",
            "apps/or_app/test/widget_test.dart",
            "apps/or_app/integration_test/core_bridge_test.dart",
        )
        rejected = (
            "crates/or_ipc/src/server.rs",
            "crates/or_core/src/application.rs",
            "Cargo.toml",
            "Cargo.lock",
            "apps/or_app/lib/main.dart",
            ".github/workflows/platform-verification.yml",
        )
        self.assertTrue(all(agent_supervisor._repair_path_allowed(path) for path in allowed))
        self.assertTrue(all(not agent_supervisor._repair_path_allowed(path) for path in rejected))

    def test_failed_sha_must_be_a_lowercase_full_sha(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root, _failed_sha, _head = self._history(directory)
            for invalid in ("A" * 40, "a" * 39, "not-a-sha"):
                with self.subTest(invalid=invalid), self.assertRaisesRegex(
                    agent_supervisor.SupervisorError, "lowercase 40-character SHA"
                ):
                    agent_supervisor.validate_repair_resume_history(root, invalid, "7F1")

    def test_failed_sha_must_be_an_ancestor(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root, _failed_sha, _head = self._history(directory)
            tree = subprocess.run(
                ["git", "rev-parse", "HEAD^{tree}"],
                cwd=root,
                capture_output=True,
                text=True,
                check=True,
            ).stdout.strip()
            orphan = subprocess.run(
                ["git", "commit-tree", tree, "-m", "orphan"],
                cwd=root,
                capture_output=True,
                text=True,
                check=True,
            ).stdout.strip()
            self._reject(root, orphan, "not an ancestor")

    def test_repair_requires_the_same_checkpoint_to_remain_next(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root, failed_sha, _head = self._history(directory)
            state_path = "docs/execution/STATE.json"
            state = json.loads((root / state_path).read_text(encoding="utf-8"))
            state["current_next"] = "7F"
            commit_test_files(root, {state_path: json.dumps(state, indent=2) + "\n"}, "move NEXT")
            self._reject(root, failed_sha, "current state no longer leaves the active checkpoint NEXT")

    def test_repair_rejects_existing_active_checkpoint_evidence(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root, failed_sha, _head = self._history(directory)
            evidence = root / "docs/execution/evidence/7F1.json"
            evidence.parent.mkdir(parents=True)
            evidence.write_text("{}\n", encoding="utf-8")
            self._reject(root, failed_sha, "completion evidence already exists")

    def test_original_implementation_protected_path_check_still_applies(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory) / "repo"
            protected = "docs/execution/architecture-policy.json"
            failed_sha, _head = make_repair_history(
                root,
                failed_changes={
                    protected: (REPO_ROOT / protected).read_bytes() + b"\n"
                },
            )
            self._reject(root, failed_sha, "original implementation changed protected")

    def test_original_implementation_cannot_change_active_checkpoint_contract(self) -> None:
        for label, path in (
            ("plan", "docs/execution/PLAN.json"),
            ("phase", "docs/execution/phases/PHASE_7.md"),
        ):
            with self.subTest(label=label), tempfile.TemporaryDirectory() as directory:
                if label == "plan":
                    plan = json.loads((REPO_ROOT / path).read_bytes())
                    execution_plan.checkpoint_for_id(plan, "7F1")["title"] = "redefined"
                    value: bytes | str = json.dumps(plan, indent=2) + "\n"
                else:
                    value = (REPO_ROOT / path).read_bytes() + b"\nchanged contract\n"
                root = Path(directory) / "repo"
                failed_sha, _head = make_repair_history(
                    root, failed_changes={path: value}
                )
                self._reject(root, failed_sha, "fingerprint")

    def test_original_implementation_cannot_change_state(self) -> None:
        state = json.loads((REPO_ROOT / "docs/execution/STATE.json").read_text(encoding="utf-8"))
        state["last_updated"] = "changed by implementation"
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory) / "repo"
            failed_sha, _head = make_repair_history(
                root,
                failed_changes={
                    "docs/execution/STATE.json": json.dumps(state, indent=2) + "\n"
                },
            )
            self._reject(root, failed_sha, "original implementation changed protected")

    def test_product_workflow_evidence_policy_and_evidence_maintenance_are_rejected(self) -> None:
        mutations = {
            "product": (
                {"crates/or_core/src/project_document.rs": b"tampered product\n"},
                "maintenance changed non-control-plane files",
            ),
            "ipc_runtime": (
                {"crates/or_ipc/src/server.rs": b"tampered IPC runtime\n"},
                "maintenance changed non-control-plane files",
            ),
            "core_runtime": (
                {"crates/or_core/src/application.rs": b"tampered core runtime\n"},
                "maintenance changed non-control-plane files",
            ),
            "manifest": (
                {"Cargo.toml": b"tampered manifest\n"},
                "maintenance changed non-control-plane files",
            ),
            "lockfile": (
                {"Cargo.lock": b"tampered lockfile\n"},
                "maintenance changed non-control-plane files",
            ),
            "flutter_runtime": (
                {"apps/or_app/lib/main.dart": b"tampered Flutter runtime\n"},
                "maintenance changed non-control-plane files",
            ),
            "workflow": (
                {".github/workflows/platform-verification.yml": b"tampered workflow\n"},
                "maintenance changed non-control-plane files",
            ),
            "evidence_policy": (
                "docs/execution/EVIDENCE_POLICY.json",
                "maintenance changed the active checkpoint contract fingerprint",
            ),
            "evidence": (
                {"docs/execution/evidence/7F1.json": b"{}\n"},
                "completion evidence already exists",
            ),
        }
        for label, (mutation, message) in mutations.items():
            with self.subTest(label=label), tempfile.TemporaryDirectory() as directory:
                root, failed_sha, _head = self._history(directory)
                if label == "evidence_policy":
                    policy_path = str(mutation)
                    policy = json.loads((root / policy_path).read_text(encoding="utf-8"))
                    policy["required_gates"]["repository_hygiene"]["workflow_name"] = (
                        "Changed workflow identity"
                    )
                    mutation = {policy_path: json.dumps(policy, indent=2) + "\n"}
                commit_test_files(root, mutation, f"change {label}")
                self._reject(root, failed_sha, message)

    def test_active_plan_entry_and_phase_spec_changes_are_rejected(self) -> None:
        for path in ("plan", "phase"):
            with self.subTest(path=path), tempfile.TemporaryDirectory() as directory:
                root, failed_sha, _head = self._history(directory)
                if path == "plan":
                    plan = json.loads((root / "docs/execution/PLAN.json").read_text(encoding="utf-8"))
                    execution_plan.checkpoint_for_id(plan, "7F1")["title"] = "redefined"
                    files = {"docs/execution/PLAN.json": json.dumps(plan, indent=2) + "\n"}
                else:
                    files = {"docs/execution/phases/PHASE_7.md": b"redefined active phase\n"}
                commit_test_files(root, files, f"redefine active {path}")
                self._reject(root, failed_sha, "active checkpoint contract fingerprint")

    def test_state_status_current_next_and_phase_mutations_are_rejected(self) -> None:
        mutations = ("status", "current_next", "phase_status")
        for mutation in mutations:
            with self.subTest(mutation=mutation), tempfile.TemporaryDirectory() as directory:
                root, failed_sha, _head = self._history(directory)
                state_path = "docs/execution/STATE.json"
                state = json.loads((root / state_path).read_text(encoding="utf-8"))
                if mutation == "status":
                    state["checkpoints"]["7F"] = "DONE"
                elif mutation == "current_next":
                    state["current_next"] = "7F"
                else:
                    state["phase_status"]["7"] = "DONE"
                commit_test_files(
                    root,
                    {state_path: json.dumps(state, indent=2) + "\n"},
                    f"change state {mutation}",
                )
                with self.assertRaises(agent_supervisor.SupervisorError):
                    agent_supervisor.validate_repair_resume_history(root, failed_sha, "7F1")

    def test_repair_orchestration_verifies_current_head_and_never_runs_a_runner(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root, failed_sha, head = self._history(directory)
            plan = json.loads((root / "docs/execution/PLAN.json").read_text(encoding="utf-8"))
            state = json.loads((root / "docs/execution/STATE.json").read_text(encoding="utf-8"))
            resolution = {
                "checkpoint_id": "7F1",
                "goal_complete_after_current": True,
                "goal_checkpoint_ids": ["7F1"],
            }
            preflight = {
                "plan": plan,
                "state": state,
                "resolution": resolution,
                "head": head,
            }
            api = object()
            evidence_result = {"api": api}
            final_result = {"checkpoint": {"id": "7F1"}}
            with (
                mock.patch.object(agent_supervisor, "ensure_start_state"),
                mock.patch.object(agent_supervisor, "_preflight_goal_data", return_value=preflight),
                mock.patch.object(agent_supervisor, "_run_pre_host_checks"),
                mock.patch.object(
                    agent_supervisor,
                    "verify_hosted_checkpoint",
                    return_value=evidence_result,
                ) as verify,
                mock.patch.object(
                    agent_supervisor, "finalize_verified_checkpoint", return_value=final_result
                ) as finalize,
                mock.patch.object(agent_supervisor, "_verified_report", return_value="verified 7F1"),
                mock.patch.object(
                    agent_supervisor,
                    "invoke_runner",
                    side_effect=AssertionError("repair-resume must never invoke a runner"),
                ) as invoke,
            ):
                reports = agent_supervisor.repair_resume_goal(
                    root, "checkpoint:7F1", failed_sha
                )
            self.assertEqual(reports, ["verified 7F1"])
            self.assertEqual(verify.call_args.args[3], head)
            self.assertEqual(
                verify.call_args.kwargs["implementation_origin_sha"], failed_sha
            )
            self.assertEqual(finalize.call_args.kwargs["implementation_sha"], head)
            invoke.assert_not_called()


class SupervisorTests(unittest.TestCase):
    def test_dirty_worktree_is_refused(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            subprocess.run(["git", "init", "-q", str(root)], check=True)
            (root / "uncommitted.txt").write_text("dirty\n", encoding="utf-8")
            with self.assertRaises(agent_supervisor.SupervisorError):
                agent_supervisor.refuse_dirty_worktree(root)

    def test_nonzero_fake_runner_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            runner = Path(directory) / "runner.py"
            runner.write_text("#!/usr/bin/env python3\nraise SystemExit(7)\n", encoding="utf-8")
            runner.chmod(runner.stat().st_mode | stat.S_IXUSR)
            with self.assertRaises(agent_supervisor.SupervisorError):
                agent_supervisor.invoke_runner(Path(directory), runner, "prompt")

    def test_zero_exit_runner_without_new_commit_stops_before_hosted_verification(self) -> None:
        plan, state = seven_f_one_plan_state()
        previous_state_commit = "3da60ecb1ef2f234032a54da9cbaeae917004933"

        def fake_git_output(_repo_root: Path, *arguments: str) -> str:
            values = {
                ("rev-parse", "HEAD"): previous_state_commit,
                ("fetch", "--prune", "origin"): "",
                ("status", "--porcelain"): "",
                ("status", "--porcelain", "--untracked-files=all"): "",
                ("branch", "--show-current"): "main",
                ("rev-parse", "origin/main"): previous_state_commit,
                ("rev-list", "--left-right", "--count", "HEAD...origin/main"): "0\t0",
                ("show", "-s", "--format=%s", previous_state_commit): (
                    "chore(execution): complete 7C after verified CI"
                ),
            }
            return values[arguments]

        with (
            mock.patch.object(agent_supervisor, "ensure_start_state"),
            mock.patch.object(execution_plan, "read_contract_versions", return_value=contract_versions(project=5)),
            mock.patch.object(
                execution_plan, "load_plan_state", return_value=(plan, state)
            ),
            mock.patch.object(agent_supervisor, "git_output", side_effect=fake_git_output),
            mock.patch.object(agent_supervisor, "invoke_runner", return_value=0) as invoke,
            mock.patch.object(
                agent_supervisor,
                "verify_hosted_checkpoint",
                side_effect=AssertionError("hosted polling must not start"),
            ) as verify_hosted,
        ):
            with self.assertRaisesRegex(
                agent_supervisor.SupervisorError,
                "runner produced no new implementation commit",
            ):
                agent_supervisor.run_goal(REPO_ROOT, "checkpoint:7F1", "/runner")
        invoke.assert_called_once()
        verify_hosted.assert_not_called()

    def test_new_direct_runner_commit_is_verified_by_its_exact_sha(self) -> None:
        plan, state = seven_f_one_plan_state()
        baseline_sha = "a" * 40
        implementation_sha = "b" * 40
        current_head = {"sha": baseline_sha}

        def fake_git_output(_repo_root: Path, *arguments: str) -> str:
            if arguments == ("rev-parse", "HEAD"):
                return current_head["sha"]
            if arguments == ("rev-parse", "origin/main"):
                return current_head["sha"]
            if arguments in (
                ("fetch", "--prune", "origin"),
                ("status", "--porcelain"),
                ("status", "--porcelain", "--untracked-files=all"),
            ):
                return ""
            if arguments == ("branch", "--show-current"):
                return "main"
            if arguments == ("rev-list", "--left-right", "--count", "HEAD...origin/main"):
                return "0\t0"
            raise AssertionError(f"unexpected git command: {arguments}")

        def runner_commits(_repo_root: Path, _runner: str, _prompt: str) -> int:
            current_head["sha"] = implementation_sha
            return 0

        with (
            mock.patch.object(agent_supervisor, "ensure_start_state"),
            mock.patch.object(execution_plan, "read_contract_versions", return_value=contract_versions(project=5)),
            mock.patch.object(
                execution_plan, "load_plan_state", return_value=(plan, state)
            ),
            mock.patch.object(agent_supervisor, "git_output", side_effect=fake_git_output),
            mock.patch.object(agent_supervisor, "invoke_runner", side_effect=runner_commits),
            mock.patch.object(
                agent_supervisor, "_run_one_checkpoint", return_value={"checkpoint_id": "A"}
            ) as run_one,
            mock.patch.object(agent_supervisor, "_verified_report", return_value="verified A"),
        ):
            reports = agent_supervisor.run_goal(REPO_ROOT, "checkpoint:7F1", "/runner")

        self.assertEqual(reports, ["verified A"])
        run_one.assert_called_once()
        self.assertEqual(
            run_one.call_args.kwargs["implementation_sha"], implementation_sha
        )

    def test_pre_host_stage_runs_the_headless_workspace_suite(self) -> None:
        implementation_sha = "a" * 40
        with (
            mock.patch.object(agent_supervisor.subprocess, "run") as run,
            mock.patch.object(agent_supervisor, "refuse_dirty_worktree"),
            mock.patch.object(agent_supervisor, "git_output", return_value=implementation_sha),
        ):
            agent_supervisor._run_pre_host_checks(REPO_ROOT, implementation_sha)
        run.assert_called_once_with(
            ["cargo", "test", "--workspace"], cwd=REPO_ROOT, check=True
        )

    def test_pre_host_failure_stops_before_hosted_polling_state_change_or_retry(self) -> None:
        plan, state = seven_f_one_plan_state()
        state_path = REPO_ROOT / "docs/execution/STATE.json"
        before_state = state_path.read_bytes()
        baseline_sha = "a" * 40
        implementation_sha = "b" * 40
        current_head = {"sha": baseline_sha}

        def fake_git_output(_repo_root: Path, *arguments: str) -> str:
            if arguments == ("status", "--porcelain", "--untracked-files=all"):
                return ""
            if arguments == ("status", "--porcelain"):
                return ""
            if arguments == ("branch", "--show-current"):
                return "main"
            if arguments == ("rev-parse", "HEAD"):
                return current_head["sha"]
            if arguments == ("rev-parse", "origin/main"):
                return current_head["sha"]
            if arguments == ("rev-list", "--left-right", "--count", "HEAD...origin/main"):
                return "0\t0"
            if arguments == ("fetch", "--prune", "origin"):
                return ""
            raise AssertionError(f"unexpected git command: {arguments}")

        def runner_commits(_repo_root: Path, _runner: str, _prompt: str) -> int:
            current_head["sha"] = implementation_sha
            return 0

        with (
            mock.patch.object(agent_supervisor, "ensure_start_state"),
            mock.patch.object(execution_plan, "read_contract_versions", return_value=contract_versions(project=5)),
            mock.patch.object(
                execution_plan, "load_plan_state", return_value=(plan, state)
            ),
            mock.patch.object(agent_supervisor, "git_output", side_effect=fake_git_output),
            mock.patch.object(
                agent_supervisor, "invoke_runner", side_effect=runner_commits
            ) as invoke,
            mock.patch.object(
                agent_supervisor.subprocess,
                "run",
                side_effect=subprocess.CalledProcessError(
                    1, ["cargo", "test", "--workspace"]
                ),
            ) as local_test,
            mock.patch.object(agent_supervisor, "verify_hosted_checkpoint") as verify,
            mock.patch.object(agent_supervisor, "_github_api_for_repo") as github_api,
            mock.patch.object(agent_supervisor, "finalize_verified_checkpoint") as finalize,
        ):
            with self.assertRaisesRegex(
                agent_supervisor.SupervisorError, "pre-host verification failed"
            ):
                agent_supervisor.run_goal(
                    REPO_ROOT, "milestone:desktop-mvp", "/runner"
                )

        invoke.assert_called_once()
        local_test.assert_called_once_with(
            ["cargo", "test", "--workspace"], cwd=REPO_ROOT, check=True
        )
        verify.assert_not_called()
        github_api.assert_not_called()
        finalize.assert_not_called()
        self.assertEqual(state_path.read_bytes(), before_state)

    def test_phase_goal_runs_one_fresh_runner_per_checkpoint(self) -> None:
        plan, initial_state = seven_f_one_plan_state()
        state_box = {"state": initial_state}
        phase_checkpoint_ids = [
            checkpoint["id"]
            for checkpoint in plan["checkpoints"]
            if checkpoint["phase"] == 7
        ]
        expected_checkpoints = phase_checkpoint_ids[phase_checkpoint_ids.index("7F1") :]
        head = {"sha": "3da60ecb1ef2f234032a54da9cbaeae917004933"}
        commit_shas = iter("abcdefg"[: len(expected_checkpoints)])
        prompts: list[str] = []
        completed: list[tuple[str, str]] = []

        def fake_git_output(_repo_root: Path, *arguments: str) -> str:
            if arguments == ("rev-parse", "HEAD"):
                return head["sha"]
            if arguments == ("rev-parse", "origin/main"):
                return head["sha"]
            if arguments in (
                ("fetch", "--prune", "origin"),
                ("status", "--porcelain"),
                ("status", "--porcelain", "--untracked-files=all"),
            ):
                return ""
            if arguments == ("branch", "--show-current"):
                return "main"
            if arguments == ("rev-list", "--left-right", "--count", "HEAD...origin/main"):
                return "0\t0"
            raise AssertionError(f"unexpected git command: {arguments}")

        def runner_commits(_repo_root: Path, _runner: str, prompt: str) -> int:
            prompts.append(prompt)
            head["sha"] = next(commit_shas) * 40
            return 0

        def verify_one(
            _repo_root: Path,
            *,
            plan: dict[str, object],
            state: dict[str, object],
            resolution: dict[str, object],
            implementation_sha: str,
        ) -> dict[str, str]:
            checkpoint_id = str(resolution["checkpoint_id"])
            completed.append((checkpoint_id, implementation_sha))
            state_box["state"] = agent_supervisor.advance_state_once(
                state, plan, checkpoint_id, today="2026-09-29"
            )
            return {"checkpoint_id": checkpoint_id}

        def validate_phase_state(
            _plan: dict[str, object], state: dict[str, object], _repo_root: Path
        ) -> dict[str, str | None]:
            return {"next_checkpoint": state["current_next"]}  # type: ignore[dict-item]

        with (
            mock.patch.object(agent_supervisor, "ensure_start_state"),
            mock.patch.object(execution_plan, "read_contract_versions", return_value=contract_versions(project=5)),
            mock.patch.object(
                execution_plan,
                "load_plan_state",
                side_effect=lambda _repo_root: (plan, state_box["state"]),
            ),
            mock.patch.object(
                execution_plan, "validate_plan", side_effect=validate_phase_state
            ),
            mock.patch.object(agent_supervisor, "git_output", side_effect=fake_git_output),
            mock.patch.object(agent_supervisor, "invoke_runner", side_effect=runner_commits) as invoke,
            mock.patch.object(agent_supervisor, "_run_one_checkpoint", side_effect=verify_one),
            mock.patch.object(
                agent_supervisor,
                "_verified_report",
                side_effect=lambda result, _plan: str(result["checkpoint_id"]),
            ),
        ):
            reports = agent_supervisor.run_goal(REPO_ROOT, "phase:7", "/runner")

        self.assertEqual(reports, expected_checkpoints)
        self.assertEqual(invoke.call_count, len(expected_checkpoints))
        self.assertEqual(
            [
                next(
                    checkpoint_id
                    for checkpoint_id in expected_checkpoints
                    if f"Checkpoint: {checkpoint_id} —" in prompt
                )
                for prompt in prompts
            ],
            expected_checkpoints,
        )
        self.assertEqual(
            completed,
            [
                (checkpoint_id, sha * 40)
                for checkpoint_id, sha in zip(expected_checkpoints, "abcdefg")
            ],
        )

    def test_single_transition_succeeds(self) -> None:
        plan, before = fixture_plan_state()
        after = agent_supervisor.advance_state_once(before, plan, "A")
        self.assertEqual(
            agent_supervisor.assert_state_advanced_once(before, after, plan), "A"
        )

    def test_multiple_transition_fails(self) -> None:
        plan, before = fixture_plan_state()
        after = json.loads(json.dumps(before))
        after["checkpoints"]["A"] = "DONE"  # type: ignore[index]
        after["checkpoints"]["B"] = "DONE"  # type: ignore[index]
        after["current_next"] = None
        with self.assertRaises(agent_supervisor.SupervisorError):
            agent_supervisor.assert_state_advanced_once(before, after, plan)

    def test_state_transition_rejects_unrelated_root_mutation(self) -> None:
        plan, before = fixture_plan_state()
        after = agent_supervisor.advance_state_once(before, plan, "A")
        after["repository"] = "tampered"
        with self.assertRaisesRegex(agent_supervisor.SupervisorError, "unrelated root field"):
            agent_supervisor.assert_state_advanced_once(before, after, plan)

    def test_state_transition_rejects_unrelated_checkpoint_status(self) -> None:
        plan, before = fixture_plan_state(["A", "B", "C"])
        after = agent_supervisor.advance_state_once(before, plan, "A")
        after["checkpoints"]["C"] = "DONE"  # type: ignore[index]
        with self.assertRaisesRegex(agent_supervisor.SupervisorError, "unrelated checkpoint C"):
            agent_supervisor.assert_state_advanced_once(before, after, plan)

    def test_state_transition_rejects_arbitrary_phase_status(self) -> None:
        plan, before = fixture_plan_state()
        after = agent_supervisor.advance_state_once(before, plan, "A")
        after["phase_status"]["6"] = "DONE"  # type: ignore[index]
        with self.assertRaisesRegex(agent_supervisor.SupervisorError, "derived checkpoint status"):
            agent_supervisor.assert_state_advanced_once(before, after, plan)

    def test_state_transition_rejects_unproven_verified_version(self) -> None:
        plan, before = fixture_plan_state()
        after = agent_supervisor.advance_state_once(before, plan, "A")
        after["verified_contract_versions"] = contract_versions(project=4)
        with self.assertRaisesRegex(
            agent_supervisor.SupervisorError, "do not match the exact candidate source"
        ):
            agent_supervisor.assert_state_advanced_once(before, after, plan)


if __name__ == "__main__":
    unittest.main()
