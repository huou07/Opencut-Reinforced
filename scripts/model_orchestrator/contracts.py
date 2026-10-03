#!/usr/bin/env python3
"""Frozen V2 control-plane contracts: authority, schemas, and adoption state.

This module is M0 (contracts and adoption schema) of the model orchestrator V2
implementation. It performs **no** runtime orchestration, worker dispatch,
model routing, promotion, or supervisor invocation. It only validates the
machine-readable contracts that later phases execute against.

Design rules taken from the frozen specification:

* all model output is untrusted; this module never treats a model statement as
  authority, and it never grants full autonomous operation;
* architecture freeze alone does **not** confer authority or full-auto
  eligibility; adoption requires the separately authorized control release;
* legacy schema-1/2 evidence remains valid at its historical boundary, and a
  caller cannot declare itself legacy;
* unknown fields, wrong types, bool-as-int, duplicate JSON keys, non-finite
  numbers, path traversal, and unsupported versions fail closed.
"""

from __future__ import annotations

import hashlib
import json
import re
import stat
from pathlib import Path
from typing import Any, Iterable, Mapping, Sequence

SCHEMA_VERSION = 2
ACTIVATION_DISABLED = "DISABLED_PENDING_SEPARATE_ADOPTION"

AUTOMATION_DIR = "docs/execution/automation"
PROTOCOL_SCHEMAS_PATH = f"{AUTOMATION_DIR}/PROTOCOL_SCHEMAS.json"
V2_CONTRACT_PATH = f"{AUTOMATION_DIR}/V2_CONTRACT.json"
MODEL_POLICY_PATH = f"{AUTOMATION_DIR}/MODEL_POLICY.json"
SANDBOX_POLICY_PATH = f"{AUTOMATION_DIR}/SANDBOX_POLICY.json"
TASK_TEMPLATES_PATH = f"{AUTOMATION_DIR}/TASK_TEMPLATES.json"
CHECKS_PATH = f"{AUTOMATION_DIR}/CHECKS.json"

ARCHITECTURE_SPEC_SHA = "c7d46dfdadb01f2088273daae4fc5edf9870a2b2"
TRUSTED_DESIGN_BASE = "915a4a8b951643e475683e4cdf56996118ec7d1e"
AUDITED_PROTOTYPE_SHA = "19c94f9677e0ff55ef37fb88b31824435d28e514"
V1_BRANCH = "control/model-orchestrator-v1"
PRIOR_FAILED_RUN_ID = 37043830370
PRIOR_IMPLEMENTATION_SHA = "f2b737ba1ac9d7fbaf6c8cb25217d06b14aa9480"
LEGACY_QUALITY_AMENDMENT_FIELDS = {"marker_sha", "prior_implementation_sha", "prior_failed_run_id"}
CONTRACT_VERSION_KEYS = {"project_schema", "recovery_schema", "ipc_protocol"}

IMPLEMENTATION_PHASES = ("M0", "M1", "M2", "M3", "M4", "M5")
ACCEPTANCE_CASE_IDS = tuple(f"CP{index:02d}" for index in range(1, 49))
M0_OWNED_CASES = ("CP01", "CP02", "CP03", "CP04", "CP05", "CP33")

# The complete frozen authority closure. A candidate changing any of these
# paths independently must be refused by the guard before any invocation.
FROZEN_CONTROL_PATHS = (
    "AGENTS.md",
    "docs/execution/PLAN.json",
    "docs/execution/STATE.json",
    "docs/execution/EVIDENCE_POLICY.json",
    "docs/execution/ARCHITECTURE_INVARIANTS.md",
    "docs/execution/architecture-policy.json",
    "docs/execution/AGENT_EXECUTION.md",
    "docs/execution/PHASE_SPEC_TEMPLATE.md",
    "docs/execution/AMENDMENT_BASELINE.json",
    "scripts/agent_supervisor.py",
    "scripts/execution_plan.py",
    "scripts/execution_evidence.py",
    "scripts/check_execution_plan.py",
    "scripts/check_architecture_policy.py",
    "scripts/test_execution_infra.py",
    "scripts/model_orchestrator/__init__.py",
    "scripts/model_orchestrator/contracts.py",
    PROTOCOL_SCHEMAS_PATH,
    V2_CONTRACT_PATH,
    MODEL_POLICY_PATH,
    SANDBOX_POLICY_PATH,
    TASK_TEMPLATES_PATH,
    CHECKS_PATH,
)

# The exact M0-M5 adoption inventory plus the documentation index/link files.
# A control-only adoption may change nothing outside this set.
ADOPTION_ALLOWED_PATHS = frozenset(
    {
        "AGENTS.md",
        "docs/INDEX.md",
        "docs/adr/README.md",
        "docs/adr/0009-model-orchestrator-v2.md",
        "docs/TESTING.md",
        "docs/TOOLING.md",
        "docs/execution/README.md",
        "docs/execution/ARCHITECTURE_INVARIANTS.md",
        "docs/execution/AGENT_EXECUTION.md",
        "docs/execution/architecture-policy.json",
        "docs/execution/AMENDMENT_BASELINE.json",
        "docs/execution/EVIDENCE_POLICY.json",
        "docs/execution/PHASE_SPEC_TEMPLATE.md",
        "scripts/model_orchestrator/__init__.py",
        "scripts/model_orchestrator/contracts.py",
        "scripts/model_orchestrator/store.py",
        "scripts/model_orchestrator/sandbox.py",
        "scripts/model_orchestrator/guards.py",
        "scripts/model_orchestrator/verification.py",
        "scripts/model_orchestrator/adapters.py",
        "scripts/model_orchestrator/orchestrator.py",
        "scripts/model_orchestrator/__main__.py",
        "scripts/model_orchestrator/promotion.py",
        "scripts/model_orchestrator/push_guard.py",
        "scripts/agent_supervisor.py",
        "scripts/execution_plan.py",
        "scripts/execution_evidence.py",
        "scripts/check_execution_plan.py",
        "scripts/check_architecture_policy.py",
        "scripts/test_execution_infra.py",
        "scripts/model_orchestrator/tests/__init__.py",
        "scripts/model_orchestrator/tests/test_contracts.py",
        "scripts/model_orchestrator/tests/test_isolation_and_state.py",
        "scripts/model_orchestrator/tests/test_verification.py",
        "scripts/model_orchestrator/tests/test_lifecycle.py",
        "scripts/model_orchestrator/tests/test_promotion_and_handoff.py",
        "scripts/model_orchestrator/tests/test_live_acceptance.py",
        PROTOCOL_SCHEMAS_PATH,
        V2_CONTRACT_PATH,
        MODEL_POLICY_PATH,
        SANDBOX_POLICY_PATH,
        TASK_TEMPLATES_PATH,
        CHECKS_PATH,
        ".opencode/agents/model-dispatcher.md",
        ".opencode/agents/orch-worker.md",
        ".opencode/agents/orch-reviewer.md",
        ".github/workflows/control-plane-acceptance.yml",
        ".github/workflows/repo-hygiene.yml",
        ".github/workflows/platform-verification.yml",
        ".github/workflows/developer-preview.yml",
    }
)

FORBIDDEN_ADOPTION_PATHS = (
    "docs/execution/PLAN.json",
    "docs/execution/STATE.json",
    "docs/execution/phases/",
    "docs/execution/evidence/",
    "DESIGN.md",
    "docs/UX_ACCEPTANCE.md",
    "docs/ARCHITECTURE.md",
    "docs/TECHNICAL_PLAN.md",
    "docs/PRODUCT.md",
    "docs/ROADMAP.md",
    "docs/SECURITY_LICENSING.md",
    "crates/",
    "apps/",
    "packages/",
    "assets/",
    "prototype/",
)

_ID_RE = re.compile(r"^[A-Za-z0-9_-]{1,64}$")
_SHA_RE = re.compile(r"^[0-9a-f]{40}$")
_DIGEST_RE = re.compile(r"^[0-9a-f]{64}$")

_KNOWN_TYPE_KEYS = {
    "integer", "boolean", "string", "id", "sha", "digest", "enum",
    "string_list", "path_list", "object", "object_list",
    "nullable_id", "nullable_sha", "nullable_digest", "nullable_string",
}


class ContractError(ValueError):
    """Raised when a frozen V2 contract is absent, malformed, or contradictory."""


def _reject_constant(token: str) -> Any:
    raise ContractError(f"non-finite JSON number is not allowed: {token}")


def _reject_duplicate_keys(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise ContractError(f"duplicate JSON key is not allowed: {key}")
        result[key] = value
    return result


def load_json_strict(text: str, label: str) -> Any:
    """Parse JSON rejecting duplicate keys and non-finite numbers."""

    try:
        return json.loads(
            text,
            object_pairs_hook=_reject_duplicate_keys,
            parse_constant=_reject_constant,
        )
    except ContractError:
        raise
    except json.JSONDecodeError as exc:
        raise ContractError(f"{label} is not valid JSON: {exc}") from exc


def load_json_file(path: Path) -> Any:
    try:
        text = path.read_text(encoding="utf-8")
    except OSError as exc:
        raise ContractError(f"cannot read {path}: {exc}") from exc
    return load_json_strict(text, str(path))


def load_object(path: Path, label: str) -> dict[str, Any]:
    value = load_json_file(path)
    if not isinstance(value, dict):
        raise ContractError(f"{label} must be a JSON object")
    return value


def canonical_json(value: Any) -> str:
    """Canonical compact JSON with sorted keys; non-finite numbers fail."""

    try:
        return json.dumps(value, sort_keys=True, separators=(",", ":"), allow_nan=False)
    except (TypeError, ValueError) as exc:
        raise ContractError(f"value is not canonically serializable: {exc}") from exc


def canonical_digest(value: Any) -> str:
    return hashlib.sha256(canonical_json(value).encode("utf-8")).hexdigest()


def _require_object(value: Any, label: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise ContractError(f"{label} must be an object")
    return value


# --------------------------------------------------------------------------
# Schema document loading and self-validation
# --------------------------------------------------------------------------


def load_protocol_schemas(repo_root: Path) -> dict[str, Any]:
    document = load_object(repo_root / PROTOCOL_SCHEMAS_PATH, "protocol schemas")
    validate_protocol_schemas(document)
    return document


def validate_protocol_schemas(document: Mapping[str, Any]) -> None:
    if document.get("schema_version") != SCHEMA_VERSION:
        raise ContractError("protocol schemas schema_version must be 2")
    if document.get("activation") != ACTIVATION_DISABLED:
        raise ContractError("protocol schemas must remain disabled pending adoption")

    lifecycle = _require_object(document.get("adoption_lifecycle"), "adoption_lifecycle")
    states = lifecycle.get("states")
    transitions = lifecycle.get("transitions")
    if not isinstance(states, list) or not states or any(not isinstance(item, str) for item in states):
        raise ContractError("adoption_lifecycle.states must be a name list")
    if not isinstance(transitions, dict) or set(transitions) != set(states):
        raise ContractError("adoption_lifecycle.transitions must cover every state exactly")
    for state, targets in transitions.items():
        if not isinstance(targets, list) or any(target not in states for target in targets):
            raise ContractError(f"adoption transition {state} names an unknown state")
    if lifecycle.get("architecture_freeze_confers_authority") is not False:
        raise ContractError("architecture freeze must not confer authority")

    record_types = _require_object(document.get("record_types"), "record_types")
    records = _require_object(document.get("records"), "records")
    if set(record_types) != set(records):
        raise ContractError("record_types and records must describe the same record set")
    for name, record in records.items():
        _validate_schema_record(name, record)


def _validate_schema_record(name: str, record: Any) -> None:
    record = _require_object(record, f"record {name}")
    version = record.get("version")
    if isinstance(version, bool) or not isinstance(version, int) or version != 1:
        raise ContractError(f"record {name}.version must be 1")
    fields = _require_object(record.get("fields"), f"record {name}.fields")
    required = record.get("required")
    if not isinstance(required, list) or any(not isinstance(item, str) for item in required):
        raise ContractError(f"record {name}.required must be a field-name list")
    unknown_required = [field for field in required if field not in fields]
    if unknown_required:
        raise ContractError(f"record {name}.required names undeclared fields: {unknown_required}")
    if "schema_version" not in fields:
        raise ContractError(f"record {name} must declare schema_version")
    for field_name, spec in fields.items():
        spec = _require_object(spec, f"record {name}.{field_name}")
        field_type = spec.get("type")
        if field_type not in _KNOWN_TYPE_KEYS:
            raise ContractError(f"record {name}.{field_name} has unknown type {field_type!r}")
        if field_type == "enum":
            values = spec.get("values")
            if not isinstance(values, list) or not values or any(
                not isinstance(item, str) for item in values
            ):
                raise ContractError(f"record {name}.{field_name} enum needs string values")


# --------------------------------------------------------------------------
# Generic strict record validation
# --------------------------------------------------------------------------


def _validate_scalar(value: Any, spec: Mapping[str, Any], label: str) -> None:
    field_type = spec["type"]
    if field_type == "integer":
        if isinstance(value, bool) or not isinstance(value, int):
            raise ContractError(f"{label} must be an integer")
        if "const" in spec and value != spec["const"]:
            raise ContractError(f"{label} must equal {spec['const']}")
    elif field_type == "boolean":
        if not isinstance(value, bool):
            raise ContractError(f"{label} must be a boolean")
    elif field_type == "string":
        if not isinstance(value, str) or not value:
            raise ContractError(f"{label} must be a non-empty string")
    elif field_type == "id":
        if not isinstance(value, str) or _ID_RE.fullmatch(value) is None:
            raise ContractError(f"{label} must be a restricted ASCII id")
    elif field_type == "sha":
        if not isinstance(value, str) or _SHA_RE.fullmatch(value) is None:
            raise ContractError(f"{label} must be a lowercase 40-character SHA")
    elif field_type == "digest":
        if not isinstance(value, str) or _DIGEST_RE.fullmatch(value) is None:
            raise ContractError(f"{label} must be a lowercase 64-character SHA-256 digest")
    elif field_type == "enum":
        if value not in spec.get("values", []):
            raise ContractError(f"{label} has unsupported value {value!r}")
    elif field_type in {"string_list", "path_list"}:
        if not isinstance(value, list):
            raise ContractError(f"{label} must be a list")
        for index, item in enumerate(value):
            if not isinstance(item, str) or not item:
                raise ContractError(f"{label}[{index}] must be a non-empty string")
            if field_type == "path_list":
                _validate_relative_path(item, f"{label}[{index}]")
    elif field_type == "object":
        if not isinstance(value, dict):
            raise ContractError(f"{label} must be an object")
    elif field_type == "object_list":
        if not isinstance(value, list):
            raise ContractError(f"{label} must be a list")
        for index, item in enumerate(value):
            if not isinstance(item, dict):
                raise ContractError(f"{label}[{index}] must be an object")
    elif field_type.startswith("nullable_"):
        if value is None:
            return
        inner = field_type[len("nullable_"):]
        _validate_scalar(value, {"type": inner}, label)
    else:  # pragma: no cover - schema self-validation prevents this
        raise ContractError(f"{label} has unsupported type {field_type!r}")


def _validate_relative_path(value: str, label: str) -> None:
    if value.startswith("/") or "\\" in value or ":" in value:
        raise ContractError(f"{label} must be a normalized relative POSIX path")
    parts = value.split("/")
    if any(part in {"", ".", ".."} for part in parts):
        raise ContractError(f"{label} must not contain empty, current, or parent segments")


def validate_record(
    record: Any,
    record_name: str,
    schemas: Mapping[str, Any],
    *,
    allow_unknown: bool = False,
) -> dict[str, Any]:
    """Validate a record against a frozen schema with exact-key checking."""

    records = _require_object(schemas.get("records"), "schemas.records")
    if record_name not in records:
        raise ContractError(f"unknown record schema: {record_name}")
    schema = _require_object(records[record_name], f"schema {record_name}")
    record = _require_object(record, f"{record_name} record")
    fields = _require_object(schema.get("fields"), f"schema {record_name}.fields")
    required = schema.get("required", [])

    if not allow_unknown:
        unknown = sorted(set(record) - set(fields))
        if unknown:
            raise ContractError(f"{record_name} has unknown fields: {', '.join(unknown)}")
    for field_name in required:
        if field_name not in record:
            raise ContractError(f"{record_name} is missing required field {field_name}")
    for field_name, value in record.items():
        spec = fields.get(field_name)
        if spec is None:
            continue
        _validate_scalar(value, spec, f"{record_name}.{field_name}")
    return record


def validate_task_contract(record: Any, schemas: Mapping[str, Any]) -> dict[str, Any]:
    return validate_record(record, "task_contract", schemas)


def validate_control_plane_receipt(record: Any, schemas: Mapping[str, Any]) -> dict[str, Any]:
    return validate_record(record, "control_plane_receipt", schemas)


# --------------------------------------------------------------------------
# Adoption lifecycle and fail-closed full-auto eligibility
# --------------------------------------------------------------------------


def validate_adoption_record(record: Any, schemas: Mapping[str, Any]) -> dict[str, Any]:
    """Validate an adoption record and every cross-field legality rule.

    Full-auto eligibility always equals the deterministic predicate, so
    ``architecture_frozen == true`` can never enable autonomous operation.
    """

    record = validate_record(record, "adoption_record", schemas)

    if record["authority_kind"] != "V2_FROZEN_CONTROL_RELEASE":
        raise ContractError("only the frozen V2 control release confers authority")

    architecture_frozen = record["architecture_frozen"]
    amendment_state = record["amendment_state"]
    adopted_release_sha = record["adopted_release_sha"]
    implementation_phase = record["implementation_phase"]
    certification = record["certification"]

    if amendment_state == "ADOPTED":
        if not architecture_frozen:
            raise ContractError("an adopted amendment requires a frozen architecture")
        if adopted_release_sha is None:
            raise ContractError("an adopted amendment requires an operator-pinned release SHA")
        if record["parent_sha"] is None:
            raise ContractError("an adopted amendment requires its pre-adoption parent SHA")
        if record["plan_digest"] is None or record["state_digest"] is None:
            raise ContractError("adoption must record unchanged plan/state digests")
    else:
        if adopted_release_sha is not None:
            raise ContractError("a proposed amendment cannot pin an adopted release SHA")

    if implementation_phase != "NONE" and amendment_state != "ADOPTED":
        raise ContractError("implementation cannot begin before the amendment is adopted")

    if certification == "CERTIFIED":
        if implementation_phase != "M5":
            raise ContractError("certification cannot precede implementation of M5")
        if set(record["completed_phases"]) != set(IMPLEMENTATION_PHASES):
            raise ContractError("certification requires every implementation phase complete")
        if set(record["acceptance_cases_passed"]) != set(ACCEPTANCE_CASE_IDS):
            raise ContractError("certification requires the full control-plane acceptance suite")

    if record["full_auto_eligible"] is not full_auto_eligible(record):
        raise ContractError("full_auto_eligible must equal the deterministic fail-closed predicate")

    return record


def full_auto_eligible(record: Mapping[str, Any]) -> bool:
    """Fail-closed: only a fully implemented and certified adopted release qualifies."""

    try:
        return (
            record["authority_kind"] == "V2_FROZEN_CONTROL_RELEASE"
            and record["architecture_frozen"] is True
            and record["amendment_state"] == "ADOPTED"
            and isinstance(record["adopted_release_sha"], str)
            and record["implementation_phase"] == "M5"
            and set(record["completed_phases"]) == set(IMPLEMENTATION_PHASES)
            and set(record["acceptance_cases_passed"]) == set(ACCEPTANCE_CASE_IDS)
            and record["certification"] == "CERTIFIED"
        )
    except (KeyError, TypeError):
        return False


def proposal_record(
    *,
    architecture_frozen: bool = True,
    architecture_spec_sha: str = ARCHITECTURE_SPEC_SHA,
    completed_phases: Sequence[str] = (),
    acceptance_cases_passed: Sequence[str] = (),
    implementation_phase: str = "NONE",
    certification: str = "NONE",
    parent_sha: str | None = None,
) -> dict[str, Any]:
    """Build a schema-valid adoption record for tests and proposal inspection."""

    return {
        "schema_version": 1,
        "authority_kind": "V2_FROZEN_CONTROL_RELEASE",
        "architecture_frozen": architecture_frozen,
        "architecture_spec_sha": architecture_spec_sha,
        "amendment_state": "PROPOSED",
        "parent_sha": parent_sha,
        "adopted_release_sha": None,
        "implementation_phase": implementation_phase,
        "completed_phases": list(completed_phases),
        "acceptance_cases_passed": list(acceptance_cases_passed),
        "certification": certification,
        "full_auto_eligible": False,
        "changed_paths": [],
        "plan_digest": None,
        "state_digest": None,
    }


# --------------------------------------------------------------------------
# Authority manifest, root resolution, and frozen source closure
# --------------------------------------------------------------------------


def resolve_authority_root(explicit_root: Any) -> Path:
    """Resolve an explicit, absolute trusted control root.

    There is deliberately no module-level default: a caller that cannot name
    the trusted root explicitly cannot load authority.
    """

    if explicit_root is None:
        raise ContractError("an explicit trusted authority root is required")
    if not isinstance(explicit_root, (str, Path)):
        raise ContractError("authority root must be a filesystem path")
    root = Path(explicit_root)
    if not root.is_absolute():
        raise ContractError("authority root must be an absolute path")
    if not root.is_dir():
        raise ContractError(f"authority root is not a directory: {root}")
    return root


def build_manifest(repo_root: Path, relative_paths: Iterable[str]) -> list[dict[str, Any]]:
    """Build a sorted no-follow manifest of regular files with mode and digest."""

    root = resolve_authority_root(repo_root)
    entries: list[dict[str, Any]] = []
    for relative in sorted(set(relative_paths)):
        _validate_relative_path(relative, "manifest path")
        path = root / relative
        try:
            info = path.lstat()
        except OSError as exc:
            raise ContractError(f"frozen path is missing: {relative}") from exc
        if stat.S_ISLNK(info.st_mode):
            raise ContractError(f"frozen path must not be a symlink: {relative}")
        if not stat.S_ISREG(info.st_mode):
            raise ContractError(f"frozen path must be a regular file: {relative}")
        try:
            data = path.read_bytes()
        except OSError as exc:
            raise ContractError(f"cannot read frozen path {relative}: {exc}") from exc
        entries.append(
            {
                "path": relative,
                "mode": "0o" + format(info.st_mode & 0o777, "o"),
                "size": info.st_size,
                "sha256": hashlib.sha256(data).hexdigest(),
                "blob_oid": git_blob_oid(data),
            }
        )
    return entries


def git_blob_oid(data: bytes) -> str:
    """Return the Git blob object id for file bytes (Git's own object naming)."""

    header = b"blob " + str(len(data)).encode("ascii") + b"\x00"
    return hashlib.sha1(header + data).hexdigest()


def load_authority_manifest(
    repo_root: Path,
    *,
    release_oid: str,
    base_oid: str,
    relative_paths: Iterable[str] = FROZEN_CONTROL_PATHS,
    sandbox_digest: str,
    contract_versions: Mapping[str, Any],
) -> dict[str, Any]:
    """Load and digest the complete frozen authority bundle."""

    if _SHA_RE.fullmatch(str(release_oid)) is None:
        raise ContractError("authority release OID must be a lowercase 40-character SHA")
    if _SHA_RE.fullmatch(str(base_oid)) is None:
        raise ContractError("authority base OID must be a lowercase 40-character SHA")
    if _DIGEST_RE.fullmatch(str(sandbox_digest)) is None:
        raise ContractError("sandbox digest must be a lowercase 64-character SHA-256 digest")
    manifest = build_manifest(repo_root, relative_paths)
    payload = {
        "release_oid": release_oid,
        "base_oid": base_oid,
        "manifest": manifest,
        "sandbox_digest": sandbox_digest,
        "contract_versions": dict(contract_versions),
        "schema_version": SCHEMA_VERSION,
    }
    payload["authority_digest"] = canonical_digest(payload)
    return payload


def verify_authority_binding(expected: Mapping[str, Any], current: Mapping[str, Any]) -> None:
    """Refuse any drift between a frozen authority and the live binding."""

    for field in ("authority_digest", "release_oid", "base_oid", "sandbox_digest"):
        if expected.get(field) != current.get(field):
            raise ContractError(f"authority drift detected in {field}")
    if expected.get("manifest") != current.get("manifest"):
        raise ContractError("authority drift detected in the frozen manifest")
    if expected.get("contract_versions") != current.get("contract_versions"):
        raise ContractError("authority drift detected in contract versions")


def frozen_source_closure_violations(
    changed_paths: Iterable[str], frozen_paths: Iterable[str] = FROZEN_CONTROL_PATHS
) -> list[str]:
    """Return every changed path that lies in the complete frozen closure."""

    frozen = set(frozen_paths)
    return sorted({path for path in changed_paths if path in frozen})


# --------------------------------------------------------------------------
# Adoption diff, provenance, and V1 non-authority
# --------------------------------------------------------------------------


def validate_adoption_diff(changed_paths: Iterable[str]) -> list[str]:
    """Require control-only adoption; product/plan/state paths are forbidden."""

    changed = sorted(set(changed_paths))
    violations: list[str] = []
    for path in changed:
        if any(path == forbidden or path.startswith(forbidden) for forbidden in FORBIDDEN_ADOPTION_PATHS):
            violations.append(path)
        elif path not in ADOPTION_ALLOWED_PATHS:
            violations.append(path)
    return violations


def validate_adoption_provenance(
    *,
    architecture_spec_sha: Any,
    external_release_sha: Any,
    marker_contains_own_sha: bool = False,
) -> None:
    """Refuse self-appointed or mismatched adoption markers."""

    if marker_contains_own_sha:
        raise ContractError("an adoption marker must not contain its own commit SHA")
    if architecture_spec_sha != ARCHITECTURE_SPEC_SHA:
        raise ContractError("adoption must cite the reviewed frozen architecture SHA")
    if _SHA_RE.fullmatch(str(external_release_sha)) is None:
        raise ContractError("adoption requires an externally operator-pinned release SHA")


def validate_control_amendment_marker(
    marker: Any,
    schemas: Mapping[str, Any],
    *,
    expected_checkpoint: str | None = None,
) -> dict[str, Any]:
    """Validate the schema-2 adoption marker without self-certifying it.

    This validates the exact marker shape from the amendment proposal. It never
    treats the marker as proof of adoption: the operator pins the resulting
    release SHA externally, and no field may contain the marker's own commit.
    """

    marker = validate_record(marker, "control_amendment_marker", schemas)
    if expected_checkpoint is not None and marker["checkpoint_id"] != expected_checkpoint:
        raise ContractError("adoption marker must target the current NEXT checkpoint")
    if marker["architecture_spec_sha"] != ARCHITECTURE_SPEC_SHA:
        raise ContractError("adoption marker must cite the reviewed frozen architecture SHA")

    legacy = marker["legacy_quality_amendment"]
    if not isinstance(legacy, dict) or set(legacy) != LEGACY_QUALITY_AMENDMENT_FIELDS:
        raise ContractError("legacy_quality_amendment has invalid fields")
    if (
        _SHA_RE.fullmatch(str(legacy["marker_sha"])) is None
        or _SHA_RE.fullmatch(str(legacy["prior_implementation_sha"])) is None
    ):
        raise ContractError("legacy_quality_amendment SHAs are invalid")
    run_id = legacy["prior_failed_run_id"]
    if isinstance(run_id, bool) or not isinstance(run_id, int) or run_id != PRIOR_FAILED_RUN_ID:
        raise ContractError("legacy_quality_amendment failed-run provenance is invalid")
    if legacy["prior_implementation_sha"] != PRIOR_IMPLEMENTATION_SHA:
        raise ContractError("legacy_quality_amendment prior implementation is invalid")

    versions = marker["verified_contract_versions"]
    if not isinstance(versions, dict) or set(versions) != CONTRACT_VERSION_KEYS:
        raise ContractError("verified_contract_versions must contain exactly the three versions")
    for key, value in versions.items():
        if isinstance(value, bool) or not isinstance(value, int) or value < 1:
            raise ContractError(f"verified_contract_versions.{key} must be a positive integer")

    violations = validate_adoption_diff(marker["changed_paths"])
    if violations:
        raise ContractError("adoption marker changes non-control paths: " + ", ".join(violations))
    return marker


def validate_authority_source(authority_kind: Any, reference: str | None = None) -> None:
    """V1/history is audit input only and never V2 execution authority."""

    if authority_kind != "V2_FROZEN_CONTROL_RELEASE":
        raise ContractError("only the frozen V2 control release is authoritative")
    if reference is not None and (
        reference == V1_BRANCH
        or reference == AUDITED_PROTOTYPE_SHA
        or reference == f"refs/heads/{V1_BRANCH}"
    ):
        raise ContractError("V1 prototype history is not V2 authority")


# --------------------------------------------------------------------------
# Legacy evidence boundary and V2 completion receipts
# --------------------------------------------------------------------------


def validate_completion_evidence(
    record: Any,
    schemas: Mapping[str, Any],
    *,
    control_release_active: bool,
) -> dict[str, Any]:
    """Gate new sealed completions on the nested control-plane receipt.

    Historical schema-1/2 records remain valid at their boundary; only when an
    adopted control release is active does a completion require the nested
    receipt. The caller cannot self-declare legacy beyond this boolean, which is
    supplied by the controller, not the record.
    """

    record = _require_object(record, "completion evidence")
    version = record.get("schema_version")
    if version not in (1, 2):
        raise ContractError("completion evidence schema_version must be 1 or 2")
    if not control_release_active:
        return record
    if "control_plane_receipt" not in record:
        raise ContractError("active V2 completion requires a nested control_plane_receipt")
    validate_control_plane_receipt(record["control_plane_receipt"], schemas)
    return record


# --------------------------------------------------------------------------
# Repository contract consistency
# --------------------------------------------------------------------------


def validate_v2_contract_documents(repo_root: Path) -> None:
    """Verify the disabled V2 contract documents are present and consistent.

    This is the read-only check used by the repository control validator. It
    neither activates V2 nor advances any checkpoint.
    """

    root = resolve_authority_root(repo_root)
    schemas = load_protocol_schemas(root)
    contract = load_object(root / V2_CONTRACT_PATH, "V2 contract")
    model_policy = load_object(root / MODEL_POLICY_PATH, "model policy")
    sandbox = load_object(root / SANDBOX_POLICY_PATH, "sandbox policy")
    templates = load_object(root / TASK_TEMPLATES_PATH, "task templates")
    checks = load_object(root / CHECKS_PATH, "checks catalog")

    if contract.get("schema_version") != SCHEMA_VERSION:
        raise ContractError("V2 contract schema_version must be 2")
    if contract.get("activation") != ACTIVATION_DISABLED:
        raise ContractError("V2 contract must remain disabled pending adoption")
    if contract.get("trusted_design_base") != TRUSTED_DESIGN_BASE:
        raise ContractError("V2 contract must cite the authoritative design base")

    adoption = _require_object(contract.get("adoption"), "V2 contract.adoption")
    if adoption.get("lifecycle_state") != "AMENDMENT_PROPOSED":
        raise ContractError("V2 M0 adoption lifecycle state must be AMENDMENT_PROPOSED")
    if adoption.get("full_auto_eligible") is not False:
        raise ContractError("V2 M0 must not permit full-auto operation")
    if adoption.get("certification") != "NONE":
        raise ContractError("V2 M0 must not claim certification")

    roles = _require_object(model_policy.get("role_policy"), "model policy.role_policy")
    for role, policy in roles.items():
        enrolled = policy.get("enrolled_models")
        if enrolled:
            raise ContractError(f"model role {role} must not enroll models before adoption")
    if model_policy.get("unknown_family_is_independent") is not False:
        raise ContractError("unknown model family must not count as independent")
    if model_policy.get("free_suffix_is_qualification") is not False:
        raise ContractError("a free suffix must not qualify a model")

    if sandbox.get("activation") != ACTIVATION_DISABLED:
        raise ContractError("sandbox policy must remain disabled pending adoption")

    if templates.get("model_authored_goal_allowed") is not False:
        raise ContractError("task templates must forbid model-authored goals")

    case_ids = [case.get("id") for case in checks.get("acceptance_cases", [])]
    if set(case_ids) != set(ACCEPTANCE_CASE_IDS) or len(case_ids) != len(ACCEPTANCE_CASE_IDS):
        raise ContractError("checks catalog must contain exactly CP01 through CP48")
    if tuple(checks.get("m0_owned_cases", [])) != M0_OWNED_CASES:
        raise ContractError("checks catalog must assign exactly the M0 acceptance cases")

    record_types = _require_object(schemas.get("record_types"), "record_types")
    if "adoption_record" not in record_types:
        raise ContractError("protocol schemas must define the adoption record")
