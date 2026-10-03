#!/usr/bin/env python3
"""M0 contract and adoption-schema tests for the model orchestrator V2.

These tests exercise the machine contracts only: schema validation, invalid
state rejection, transition/adoption validation, frozen-source-closure and
authority checks, and the fail-closed full-auto predicate. They deliberately do
not launch workers, models, containers, or the supervisor.
"""

from __future__ import annotations

import copy
import json
import os
import stat
import sys
import tempfile
import unittest
from pathlib import Path

TEST_DIR = Path(__file__).resolve().parent
SCRIPTS_DIR = TEST_DIR.parents[1]
REPO_ROOT = TEST_DIR.parents[2]
sys.path.insert(0, str(SCRIPTS_DIR))

import model_orchestrator.contracts as contracts  # noqa: E402


def valid_task(schemas: dict) -> dict:
    return {
        "schema_version": 1,
        "task_id": "task-9B-001",
        "task_kind": "product_checkpoint",
        "repository": "huou07/Opencut-Reinforced",
        "checkpoint_id": "9B",
        "base_sha": "a" * 40,
        "candidate_branch": "wip/task",
        "authority_digest": "b" * 64,
        "template_digest": "c" * 64,
        "goal": "Implement the bounded checkpoint goal.",
        "out_of_scope": ["No roadmap chaining."],
        "allowed_paths": ["apps/or_app/lib"],
        "forbidden_paths": ["docs/execution/STATE.json"],
        "required_documentation": ["docs/PRODUCT.md"],
        "required_tests": ["flutter test"],
        "invariant_ids": ["INV-PRODUCT"],
        "project_schema_effect": "none",
        "recovery_schema_effect": "none",
        "ipc_effect": "none",
        "user_action": "User opens a project.",
        "observable_outcome": "The project previews.",
        "production_boundary": "Packaged desktop application.",
        "error_cases": ["Missing file shows an actionable error."],
        "persistence_expectation": "Reopening restores state.",
        "permission_expectation": "No undeclared host capability.",
        "required_check_ids": ["unit"],
        "check_argv": [{"argv": ["flutter", "test"]}],
        "harness_digest": "d" * 64,
        "case_inventory": ["unit"],
        "execution_boundary": "controller_verifier",
        "expected_result": "zero exit",
        "resource_limits": {"wall_seconds": 600},
        "evidence_classes": ["UNIT", "USER_JOURNEY"],
        "acceptance_cases": ["CP44"],
        "performance_applicability": "not_applicable",
        "performance_budgets": {},
        "role_enrollment_ids": ["impl-1"],
        "budget": {"tokens": 100000},
        "stop_conditions": ["Stop on contradiction."],
        "resume_stage": "NONE",
    }


class StrictJsonTests(unittest.TestCase):
    def test_duplicate_keys_are_rejected(self) -> None:
        with self.assertRaises(contracts.ContractError):
            contracts.load_json_strict('{"a": 1, "a": 2}', "fixture")

    def test_non_finite_numbers_are_rejected(self) -> None:
        with self.assertRaises(contracts.ContractError):
            contracts.load_json_strict('{"a": Infinity}', "fixture")

    def test_canonical_digest_is_stable_and_sorted(self) -> None:
        self.assertEqual(
            contracts.canonical_digest({"b": 1, "a": 2}),
            contracts.canonical_digest({"a": 2, "b": 1}),
        )
        self.assertEqual(len(contracts.canonical_digest({"a": 1})), 64)


class TaskContractTests(unittest.TestCase):
    """CP03: forged/wrong-type/unknown-field/traversal/corrupt fail closed."""

    @classmethod
    def setUpClass(cls) -> None:
        cls.schemas = contracts.load_protocol_schemas(REPO_ROOT)

    def test_valid_task_contract_passes(self) -> None:
        contracts.validate_task_contract(valid_task(self.schemas), self.schemas)

    def test_missing_required_field_fails(self) -> None:
        task = valid_task(self.schemas)
        del task["goal"]
        with self.assertRaisesRegex(contracts.ContractError, "missing required"):
            contracts.validate_task_contract(task, self.schemas)

    def test_unknown_field_fails(self) -> None:
        task = valid_task(self.schemas)
        task["smuggled"] = "value"
        with self.assertRaisesRegex(contracts.ContractError, "unknown fields"):
            contracts.validate_task_contract(task, self.schemas)

    def test_wrong_type_and_bool_as_int_fail(self) -> None:
        task = valid_task(self.schemas)
        task["schema_version"] = True
        with self.assertRaises(contracts.ContractError):
            contracts.validate_task_contract(task, self.schemas)
        task = valid_task(self.schemas)
        task["task_id"] = 7
        with self.assertRaises(contracts.ContractError):
            contracts.validate_task_contract(task, self.schemas)

    def test_unknown_enum_value_fails(self) -> None:
        task = valid_task(self.schemas)
        task["execution_boundary"] = "host_shell"
        with self.assertRaises(contracts.ContractError):
            contracts.validate_task_contract(task, self.schemas)

    def test_path_traversal_fails(self) -> None:
        task = valid_task(self.schemas)
        task["allowed_paths"] = ["../etc/passwd"]
        with self.assertRaises(contracts.ContractError):
            contracts.validate_task_contract(task, self.schemas)
        task = valid_task(self.schemas)
        task["allowed_paths"] = ["/absolute/path"]
        with self.assertRaises(contracts.ContractError):
            contracts.validate_task_contract(task, self.schemas)

    def test_corrupt_receipt_fails(self) -> None:
        receipt = {
            "schema_version": 1,
            "task_id": "task-9B-001",
            "candidate_sha": "a" * 40,
            "check_id": "unit",
            "command_digest": "b" * 64,
            "exit_code": True,
            "outcome": "MAYBE",
            "executed_cases": ["unit"],
            "skipped_cases": [],
            "artifact_digest": None,
        }
        with self.assertRaises(contracts.ContractError):
            contracts.validate_record(receipt, "verification_receipt", self.schemas)

    def test_unknown_record_schema_fails(self) -> None:
        with self.assertRaisesRegex(contracts.ContractError, "unknown record schema"):
            contracts.validate_record({}, "not_a_record", self.schemas)


class AuthorityTests(unittest.TestCase):
    """CP01/CP02/CP04: frozen closure, root resolution, and drift refusal."""

    def _tree(self, directory: str) -> Path:
        root = Path(directory)
        (root / "a.txt").write_text("a", encoding="utf-8")
        (root / "sub").mkdir()
        (root / "sub" / "b.txt").write_text("b", encoding="utf-8")
        return root

    def test_frozen_source_closure_detects_each_path(self) -> None:
        for path in contracts.FROZEN_CONTROL_PATHS:
            self.assertEqual(contracts.frozen_source_closure_violations([path]), [path])
        self.assertEqual(
            contracts.frozen_source_closure_violations(["apps/or_app/lib/x.dart"]), []
        )

    def test_authority_root_requires_explicit_absolute_existing_path(self) -> None:
        with self.assertRaises(contracts.ContractError):
            contracts.resolve_authority_root(None)
        with self.assertRaises(contracts.ContractError):
            contracts.resolve_authority_root("relative/path")
        with self.assertRaises(contracts.ContractError):
            contracts.resolve_authority_root("/nonexistent/authority/root")

    def test_manifest_is_sorted_and_rejects_symlinks(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = self._tree(directory)
            manifest = contracts.build_manifest(root, ["sub/b.txt", "a.txt"])
            self.assertEqual([entry["path"] for entry in manifest], ["a.txt", "sub/b.txt"])
            os.symlink(root / "a.txt", root / "link.txt")
            with self.assertRaisesRegex(contracts.ContractError, "symlink"):
                contracts.build_manifest(root, ["link.txt"])

    def test_authority_manifest_and_drift_refusal(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = self._tree(directory)
            payload = contracts.load_authority_manifest(
                root,
                release_oid="a" * 40,
                base_oid="b" * 40,
                relative_paths=["a.txt", "sub/b.txt"],
                sandbox_digest="c" * 64,
                contract_versions={"project_schema": 7, "recovery_schema": 1, "ipc_protocol": 1},
            )
            self.assertEqual(len(payload["authority_digest"]), 64)
            contracts.verify_authority_binding(payload, copy.deepcopy(payload))

            tampered = copy.deepcopy(payload)
            tampered["manifest"][0]["sha256"] = "0" * 64
            with self.assertRaisesRegex(contracts.ContractError, "manifest"):
                contracts.verify_authority_binding(payload, tampered)

            mixed = copy.deepcopy(payload)
            mixed["release_oid"] = "d" * 40
            with self.assertRaisesRegex(contracts.ContractError, "release_oid"):
                contracts.verify_authority_binding(payload, mixed)

    def test_wrong_digest_inputs_are_refused(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = self._tree(directory)
            with self.assertRaises(contracts.ContractError):
                contracts.load_authority_manifest(
                    root,
                    release_oid="not-a-sha",
                    base_oid="b" * 40,
                    relative_paths=["a.txt"],
                    sandbox_digest="c" * 64,
                    contract_versions={},
                )
            with self.assertRaises(contracts.ContractError):
                contracts.load_authority_manifest(
                    root,
                    release_oid="a" * 40,
                    base_oid="b" * 40,
                    relative_paths=["a.txt"],
                    sandbox_digest="short",
                    contract_versions={},
                )

    def test_missing_frozen_path_is_refused(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = self._tree(directory)
            with self.assertRaisesRegex(contracts.ContractError, "missing"):
                contracts.build_manifest(root, ["does/not/exist.txt"])


class AdoptionTests(unittest.TestCase):
    """CP05/CP33 and the negative adoption-state matrix."""

    @classmethod
    def setUpClass(cls) -> None:
        cls.schemas = contracts.load_protocol_schemas(REPO_ROOT)

    def _certified(self) -> dict:
        record = contracts.proposal_record(
            completed_phases=contracts.IMPLEMENTATION_PHASES,
            acceptance_cases_passed=contracts.ACCEPTANCE_CASE_IDS,
            implementation_phase="M5",
            certification="CERTIFIED",
            parent_sha="a" * 40,
        )
        record.update(
            {
                "amendment_state": "ADOPTED",
                "adopted_release_sha": "b" * 40,
                "plan_digest": "c" * 64,
                "state_digest": "d" * 64,
                "full_auto_eligible": True,
            }
        )
        return record

    def test_proposal_is_valid_but_not_full_auto(self) -> None:
        record = contracts.proposal_record()
        contracts.validate_adoption_record(record, self.schemas)
        self.assertFalse(contracts.full_auto_eligible(record))

    def test_fully_certified_release_is_full_auto_eligible(self) -> None:
        record = self._certified()
        contracts.validate_adoption_record(record, self.schemas)
        self.assertTrue(contracts.full_auto_eligible(record))

    def test_architecture_freeze_alone_is_not_full_auto(self) -> None:
        record = contracts.proposal_record()
        record["full_auto_eligible"] = True
        with self.assertRaisesRegex(contracts.ContractError, "fail-closed predicate"):
            contracts.validate_adoption_record(record, self.schemas)

    def test_certification_before_implementation_is_rejected(self) -> None:
        record = contracts.proposal_record(certification="CERTIFIED")
        with self.assertRaisesRegex(contracts.ContractError, "precede implementation"):
            contracts.validate_adoption_record(record, self.schemas)

    def test_full_auto_claimed_before_certification_is_rejected(self) -> None:
        record = self._certified()
        record["certification"] = "NONE"
        record["implementation_phase"] = "M4"
        with self.assertRaises(contracts.ContractError):
            contracts.validate_adoption_record(record, self.schemas)

    def test_adopted_without_architecture_freeze_is_rejected(self) -> None:
        record = self._certified()
        record["architecture_frozen"] = False
        with self.assertRaisesRegex(contracts.ContractError, "frozen architecture"):
            contracts.validate_adoption_record(record, self.schemas)

    def test_adopted_without_release_sha_is_rejected(self) -> None:
        record = self._certified()
        record["adopted_release_sha"] = None
        with self.assertRaisesRegex(contracts.ContractError, "operator-pinned release"):
            contracts.validate_adoption_record(record, self.schemas)

    def test_proposed_with_release_sha_is_rejected(self) -> None:
        record = contracts.proposal_record()
        record["adopted_release_sha"] = "b" * 40
        with self.assertRaisesRegex(contracts.ContractError, "proposed amendment"):
            contracts.validate_adoption_record(record, self.schemas)

    def test_implementation_before_adoption_is_rejected(self) -> None:
        record = contracts.proposal_record(implementation_phase="M1")
        with self.assertRaisesRegex(contracts.ContractError, "before the amendment is adopted"):
            contracts.validate_adoption_record(record, self.schemas)

    def test_unknown_schema_version_is_rejected(self) -> None:
        document = contracts.load_protocol_schemas(REPO_ROOT)
        document["schema_version"] = 3
        with self.assertRaises(contracts.ContractError):
            contracts.validate_protocol_schemas(document)

    def test_v1_authority_is_not_v2_authority(self) -> None:
        with self.assertRaises(contracts.ContractError):
            contracts.validate_authority_source("V1_PROTOTYPE")
        with self.assertRaises(contracts.ContractError):
            contracts.validate_authority_source(
                "V2_FROZEN_CONTROL_RELEASE", contracts.V1_BRANCH
            )
        with self.assertRaises(contracts.ContractError):
            contracts.validate_authority_source(
                "V2_FROZEN_CONTROL_RELEASE", contracts.AUDITED_PROTOTYPE_SHA
            )
        contracts.validate_authority_source("V2_FROZEN_CONTROL_RELEASE")

    def test_self_appointed_marker_is_rejected(self) -> None:
        with self.assertRaises(contracts.ContractError):
            contracts.validate_adoption_provenance(
                architecture_spec_sha=contracts.ARCHITECTURE_SPEC_SHA,
                external_release_sha="a" * 40,
                marker_contains_own_sha=True,
            )
        with self.assertRaises(contracts.ContractError):
            contracts.validate_adoption_provenance(
                architecture_spec_sha="0" * 40,
                external_release_sha="a" * 40,
            )
        with self.assertRaises(contracts.ContractError):
            contracts.validate_adoption_provenance(
                architecture_spec_sha=contracts.ARCHITECTURE_SPEC_SHA,
                external_release_sha=None,
            )

    def test_adoption_diff_is_control_only(self) -> None:
        allowed = ["AGENTS.md", "scripts/model_orchestrator/contracts.py"]
        self.assertEqual(contracts.validate_adoption_diff(allowed), [])
        self.assertEqual(
            contracts.validate_adoption_diff(["docs/execution/PLAN.json"]),
            ["docs/execution/PLAN.json"],
        )
        self.assertEqual(
            contracts.validate_adoption_diff(["crates/or_core/src/lib.rs"]),
            ["crates/or_core/src/lib.rs"],
        )
        self.assertEqual(
            contracts.validate_adoption_diff(["apps/or_app/lib/main.dart"]),
            ["apps/or_app/lib/main.dart"],
        )
        self.assertEqual(
            contracts.validate_adoption_diff(["docs/execution/evidence/9B.json"]),
            ["docs/execution/evidence/9B.json"],
        )


class CompletionEvidenceTests(unittest.TestCase):
    """CP05: legacy boundary preserved; new adopted completion needs a receipt."""

    @classmethod
    def setUpClass(cls) -> None:
        cls.schemas = contracts.load_protocol_schemas(REPO_ROOT)

    def test_legacy_record_is_valid_without_control_release(self) -> None:
        legacy = {"schema_version": 1, "checkpoint_id": "9B"}
        contracts.validate_completion_evidence(
            legacy, self.schemas, control_release_active=False
        )
        legacy_two = {"schema_version": 2, "checkpoint_id": "9B", "evidence_classes": {}}
        contracts.validate_completion_evidence(
            legacy_two, self.schemas, control_release_active=False
        )

    def test_active_release_requires_nested_receipt(self) -> None:
        with self.assertRaisesRegex(contracts.ContractError, "nested control_plane_receipt"):
            contracts.validate_completion_evidence(
                {"schema_version": 2, "checkpoint_id": "9B"},
                self.schemas,
                control_release_active=True,
            )

    def test_active_release_rejects_invalid_nested_receipt(self) -> None:
        record = {
            "schema_version": 2,
            "checkpoint_id": "9B",
            "control_plane_receipt": {"schema_version": 1, "task_id": "x"},
        }
        with self.assertRaises(contracts.ContractError):
            contracts.validate_completion_evidence(
                record, self.schemas, control_release_active=True
            )

    def test_unknown_completion_schema_version_is_rejected(self) -> None:
        with self.assertRaises(contracts.ContractError):
            contracts.validate_completion_evidence(
                {"schema_version": 3, "checkpoint_id": "9B"},
                self.schemas,
                control_release_active=False,
            )


class RepositoryContractTests(unittest.TestCase):
    def test_repository_contract_documents_are_valid(self) -> None:
        contracts.validate_v2_contract_documents(REPO_ROOT)

    def test_tampered_full_auto_flag_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            target = root / contracts.AUTOMATION_DIR
            target.mkdir(parents=True)
            for name in (
                "PROTOCOL_SCHEMAS.json",
                "V2_CONTRACT.json",
                "MODEL_POLICY.json",
                "SANDBOX_POLICY.json",
                "TASK_TEMPLATES.json",
                "CHECKS.json",
            ):
                (target / name).write_bytes((REPO_ROOT / contracts.AUTOMATION_DIR / name).read_bytes())
            contract_path = target / "V2_CONTRACT.json"
            contract = json.loads(contract_path.read_text(encoding="utf-8"))
            contract["adoption"]["full_auto_eligible"] = True
            contract_path.write_text(json.dumps(contract), encoding="utf-8")
            with self.assertRaisesRegex(contracts.ContractError, "full-auto"):
                contracts.validate_v2_contract_documents(root)

    def test_enrolled_model_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            target = root / contracts.AUTOMATION_DIR
            target.mkdir(parents=True)
            for name in (
                "PROTOCOL_SCHEMAS.json",
                "V2_CONTRACT.json",
                "MODEL_POLICY.json",
                "SANDBOX_POLICY.json",
                "TASK_TEMPLATES.json",
                "CHECKS.json",
            ):
                (target / name).write_bytes((REPO_ROOT / contracts.AUTOMATION_DIR / name).read_bytes())
            policy_path = target / "MODEL_POLICY.json"
            policy = json.loads(policy_path.read_text(encoding="utf-8"))
            policy["role_policy"]["IMPLEMENTATION"]["enrolled_models"] = ["some-model"]
            policy_path.write_text(json.dumps(policy), encoding="utf-8")
            with self.assertRaisesRegex(contracts.ContractError, "enroll models"):
                contracts.validate_v2_contract_documents(root)


class ControlAmendmentMarkerTests(unittest.TestCase):
    """The schema-2 adoption marker is defined but never self-certifying."""

    @classmethod
    def setUpClass(cls) -> None:
        cls.schemas = contracts.load_protocol_schemas(REPO_ROOT)

    def _marker(self) -> dict:
        return {
            "schema_version": 2,
            "kind": "model-orchestrator-v2-adoption",
            "checkpoint_id": "9B",
            "parent_sha": "a" * 40,
            "architecture_spec_sha": contracts.ARCHITECTURE_SPEC_SHA,
            "authority_manifest_digest": "b" * 64,
            "changed_paths": ["AGENTS.md", "scripts/model_orchestrator/contracts.py"],
            "legacy_quality_amendment": {
                "marker_sha": "c" * 40,
                "prior_implementation_sha": contracts.PRIOR_IMPLEMENTATION_SHA,
                "prior_failed_run_id": contracts.PRIOR_FAILED_RUN_ID,
            },
            "state_digest": "d" * 64,
            "plan_digest": "e" * 64,
            "verified_contract_versions": {
                "project_schema": 7,
                "recovery_schema": 1,
                "ipc_protocol": 1,
            },
        }

    def test_valid_marker_passes(self) -> None:
        contracts.validate_control_amendment_marker(
            self._marker(), self.schemas, expected_checkpoint="9B"
        )

    def test_wrong_checkpoint_is_rejected(self) -> None:
        marker = self._marker()
        marker["checkpoint_id"] = "9C"
        with self.assertRaisesRegex(contracts.ContractError, "current NEXT"):
            contracts.validate_control_amendment_marker(
                marker, self.schemas, expected_checkpoint="9B"
            )

    def test_wrong_architecture_sha_is_rejected(self) -> None:
        marker = self._marker()
        marker["architecture_spec_sha"] = "0" * 40
        with self.assertRaisesRegex(contracts.ContractError, "frozen architecture"):
            contracts.validate_control_amendment_marker(marker, self.schemas)

    def test_product_path_in_marker_is_rejected(self) -> None:
        marker = self._marker()
        marker["changed_paths"] = ["AGENTS.md", "apps/or_app/lib/main.dart"]
        with self.assertRaisesRegex(contracts.ContractError, "non-control paths"):
            contracts.validate_control_amendment_marker(marker, self.schemas)

    def test_marker_cannot_hold_its_own_sha(self) -> None:
        marker = self._marker()
        marker["marker_sha"] = "f" * 40
        with self.assertRaisesRegex(contracts.ContractError, "unknown fields"):
            contracts.validate_control_amendment_marker(marker, self.schemas)

    def test_wrong_legacy_provenance_is_rejected(self) -> None:
        marker = self._marker()
        marker["legacy_quality_amendment"]["prior_failed_run_id"] = 1
        with self.assertRaisesRegex(contracts.ContractError, "failed-run provenance"):
            contracts.validate_control_amendment_marker(marker, self.schemas)

    def test_bool_contract_version_is_rejected(self) -> None:
        marker = self._marker()
        marker["verified_contract_versions"]["project_schema"] = True
        with self.assertRaises(contracts.ContractError):
            contracts.validate_control_amendment_marker(marker, self.schemas)


if __name__ == "__main__":
    unittest.main()
