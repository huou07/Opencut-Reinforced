#!/usr/bin/env python3
"""Stdlib tests for the OR execution-plan infrastructure."""

from __future__ import annotations

import json
import os
import stat
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPT_DIR = Path(__file__).resolve().parent
REPO_ROOT = SCRIPT_DIR.parent
sys.path.insert(0, str(SCRIPT_DIR))

import agent_supervisor  # noqa: E402
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
    }
    return plan, state


class ExecutionPlanTests(unittest.TestCase):
    def test_current_repository_plan_is_valid(self) -> None:
        plan, state = execution_plan.load_plan_state(REPO_ROOT)
        summary = execution_plan.validate_plan(plan, state, REPO_ROOT)
        self.assertEqual(summary["next_checkpoint"], "6E2B")

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

    def test_single_transition_succeeds(self) -> None:
        plan, before = fixture_plan_state()
        after = json.loads(json.dumps(before))
        after["checkpoints"]["A"] = "DONE"  # type: ignore[index]
        after["checkpoints"]["B"] = "NEXT"  # type: ignore[index]
        after["current_next"] = "B"
        self.assertEqual(agent_supervisor.assert_state_advanced_once(before, after, plan), "A")

    def test_multiple_transition_fails(self) -> None:
        plan, before = fixture_plan_state()
        after = json.loads(json.dumps(before))
        after["checkpoints"]["A"] = "DONE"  # type: ignore[index]
        after["checkpoints"]["B"] = "DONE"  # type: ignore[index]
        after["current_next"] = None
        with self.assertRaises(agent_supervisor.SupervisorError):
            agent_supervisor.assert_state_advanced_once(before, after, plan)


if __name__ == "__main__":
    unittest.main()
