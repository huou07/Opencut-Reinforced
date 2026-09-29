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
from pathlib import Path

SCRIPT_DIR = Path(__file__).resolve().parent
REPO_ROOT = SCRIPT_DIR.parent
sys.path.insert(0, str(SCRIPT_DIR))

import agent_supervisor  # noqa: E402
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
    }
    return plan, state


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
            self.assertEqual(execution_plan.validate_plan(plan, state, root)["next_checkpoint"], "7B")


class EvidenceTests(unittest.TestCase):
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
            9,
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
        self.assertEqual(len(result["assets"]), 9)

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

    def test_verified_state_transition_is_exactly_one_checkpoint(self) -> None:
        plan, state = fixture_plan_state(["7A", "7B"])
        after = agent_supervisor.advance_state_once(state, plan, "7A", today="2026-09-28")
        self.assertEqual(after["checkpoints"]["7A"], "DONE")
        self.assertEqual(after["checkpoints"]["7B"], "NEXT")
        self.assertEqual(after["current_next"], "7B")

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
