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
import math
import os
import subprocess
from dataclasses import dataclass
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

ARCHITECTURE_SPEC_SHA = "8f41e0b5f03a9da0797ad5ddf6a65da40f447d09"
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

# Minimum mandatory surfaces. The actual manifest covers the complete pinned Git
# tree; this list is not a substitute for full-tree inventory validation.
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
        "docs/execution/automation/README.md",
        "docs/execution/automation/IMPLEMENTATION_PLAN.md",
        "docs/execution/automation/AMENDMENT_PROPOSAL.md",
        "docs/execution/automation/V1_AUDIT.md",
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

REPOSITORY_IDENTITY = "huou07/Opencut-Reinforced"
AUTHORITY_PREFIXES = ("scripts/", "docs/execution/", "docs/adr/", ".github/workflows/", ".opencode/")
AUTHORITY_EXACT_PATHS = frozenset({
    "AGENTS.md", "DESIGN.md", "README.md", "CONTRIBUTING.md", "Cargo.toml", "Cargo.lock", "rust-toolchain.toml", "rust-toolchain",
    "docs/UX_ACCEPTANCE.md", "docs/ARCHITECTURE.md", "docs/TECHNICAL_PLAN.md", "docs/PRODUCT.md",
    "docs/SECURITY_LICENSING.md", "docs/TESTING.md", "docs/TOOLING.md", "docs/DEVELOPMENT_WORKFLOW.md",
    "crates/or_core/src/project_document.rs", "crates/or_core/src/project_recovery.rs", "crates/or_ipc/src/protocol.rs",
    "opencode.json", "opencode.jsonc",
})
CONFIG_PATHS = ("opencode.json", "opencode.jsonc", "rust-toolchain.toml", "rust-toolchain")
REQUIRED_AUTHORITY_PATHS = frozenset(FROZEN_CONTROL_PATHS) | (AUTHORITY_EXACT_PATHS - set(CONFIG_PATHS)) | {
    "docs/execution/automation/README.md", "docs/execution/automation/IMPLEMENTATION_PLAN.md",
    "docs/execution/automation/AMENDMENT_PROPOSAL.md", "docs/execution/automation/V1_AUDIT.md",
    "docs/adr/0009-model-orchestrator-v2.md",
}

class ContractError(ValueError):
    """Malformed, contradictory or unbound frozen contract."""


def _json_safe(value: Any) -> None:
    if isinstance(value, dict):
        if any(not isinstance(key, str) for key in value):
            raise ContractError('JSON object keys must be strings')
        for item in value.values():
            _json_safe(item)
    elif isinstance(value, list):
        for item in value:
            _json_safe(item)
    elif isinstance(value, float) and not math.isfinite(value):
        raise ContractError('non-finite number')


def _pairs(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result = {}
    for key, value in pairs:
        if key in result:
            raise ContractError('duplicate JSON key: ' + key)
        result[key] = value
    return result


def load_json_strict(text: str, label: str = 'JSON') -> Any:
    if len(text.encode('utf-8')) > 16 * 1024 * 1024:
        raise ContractError('JSON size bound exceeded')
    try:
        value = json.loads(text, object_pairs_hook=_pairs)
        _json_safe(value)
        return value
    except (ValueError, RecursionError) as exc:
        raise ContractError(f'{label}: {exc}') from exc


def load_json_file(path: Path) -> Any:
    try:
        return load_json_strict(path.read_text(encoding='utf-8'), str(path))
    except (OSError, UnicodeError) as exc:
        raise ContractError(str(exc)) from exc


def load_object(path: Path, label: str) -> dict[str, Any]:
    return _require_object(load_json_file(path), label)


def canonical_json(value: Any) -> str:
    _json_safe(value)
    try:
        return json.dumps(value, sort_keys=True, separators=(',', ':'), allow_nan=False)
    except (TypeError, ValueError, RecursionError) as exc:
        raise ContractError(str(exc)) from exc


def canonical_digest(value: Any) -> str:
    return hashlib.sha256(canonical_json(value).encode()).hexdigest()


def _require_object(value: Any, label: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise ContractError(label + ' must be an object')
    _json_safe(value)
    return value


def _validate_relative_path(value: str, label: str) -> None:
    if not isinstance(value, str) or not value or len(value) > 4096:
        raise ContractError(label + ' invalid path')
    if value.startswith('/') or any(c in value for c in '\\:\x00*?[]') or any(ord(c) < 32 for c in value):
        raise ContractError(label + ' invalid path')
    if any(part in ('', '.', '..') for part in value.split('/')):
        raise ContractError(label + ' path traversal or non-normalized path')


def _unique(values: list[Any], label: str) -> None:
    if len({canonical_json(v) for v in values}) != len(values):
        raise ContractError(label + ' duplicate identifiers')


def _value(value: Any, spec: Mapping[str, Any], label: str) -> None:
    kind = spec['type']
    if kind.startswith('nullable_'):
        if value is None:
            return
        return _value(value, dict(spec, type=kind[9:]), label)
    if kind in ('integer', 'number'):
        valid = type(value) is int if kind == 'integer' else type(value) in (int, float)
        if not valid or type(value) is float and not math.isfinite(value):
            raise ContractError(label + ' invalid numeric type')
        if value < spec.get('minimum', -2**50) or value > spec.get('maximum', 2**50):
            raise ContractError(label + ' numeric bound violated')
    elif kind == 'boolean':
        if type(value) is not bool:
            raise ContractError(label + ' must be boolean')
    elif kind in ('string', 'id', 'sha', 'digest', 'path', 'enum'):
        if not isinstance(value, str) or not value or len(value) > spec.get('max_length', 4096) or '\x00' in value:
            raise ContractError(label + ' invalid string')
        pattern = {'id': _ID_RE, 'sha': _SHA_RE, 'digest': _DIGEST_RE}.get(kind)
        if pattern and not pattern.fullmatch(value):
            raise ContractError(label + ' invalid ' + kind)
        if kind == 'enum' and value not in spec['values']:
            raise ContractError(label + ' unsupported enum')
        if kind == 'path' and not (value == '.' and spec.get('root_allowed') is True):
            _validate_relative_path(value, label)
    elif kind == 'object':
        value = _require_object(value, label)
        fields = spec['fields']
        if set(value) - set(fields) or set(spec['required']) - set(value):
            raise ContractError(label + ' unknown or missing nested fields')
        for key, item in value.items():
            _value(item, fields[key], label + '.' + key)
    elif kind in ('list', 'string_list', 'path_list'):
        if not isinstance(value, list) or not spec.get('min_items', 0) <= len(value) <= spec.get('max_items', 4096):
            raise ContractError(label + ' invalid list size/type')
        if spec.get('unique', True):
            _unique(value, label)
        inner = spec.get('items', {'type': 'path' if kind == 'path_list' else 'string'})
        for item in value:
            _value(item, inner, label)
    else:
        raise ContractError(label + ' unsupported schema type')
    if 'const' in spec and (type(value) is not type(spec['const']) or value != spec['const']):
        raise ContractError(label + ' unsupported version/constant')


def _schema(spec: Any) -> None:
    spec = _require_object(spec, 'schema')
    allowed = {'type', 'const', 'minimum', 'maximum', 'values', 'fields', 'required', 'items', 'min_items', 'max_items', 'unique', 'max_length', 'root_allowed'}
    if set(spec) - allowed:
        raise ContractError('unknown schema fields')
    kind = spec.get('type', '').removeprefix('nullable_')
    if kind == 'object':
        fields = _require_object(spec.get('fields'), 'schema fields')
        if not isinstance(spec.get('required'), list) or not set(spec['required']) <= set(fields):
            raise ContractError('invalid required fields')
        for field in fields.values():
            _schema(field)
    elif kind == 'list':
        _schema(spec['items'])
    elif kind == 'enum':
        _value(spec['values'], {'type': 'string_list', 'min_items': 1}, 'enum values')
    elif kind not in ('string', 'integer', 'number', 'boolean', 'id', 'sha', 'digest', 'path', 'string_list', 'path_list'):
        raise ContractError('unsupported schema type')


def load_protocol_schemas(repo_root: Path) -> dict[str, Any]:
    document = load_object(repo_root / PROTOCOL_SCHEMAS_PATH, 'protocol schemas')
    validate_protocol_schemas(document)
    return document


def validate_protocol_schemas(document: Mapping[str, Any]) -> None:
    if type(document.get('schema_version')) is not int or document['schema_version'] != 2 or document.get('activation') != ACTIVATION_DISABLED:
        raise ContractError('schema version/activation')
    lifecycle = document['adoption_lifecycle']
    states = ['ARCHITECTURE_FROZEN', 'AMENDMENT_PROPOSED', 'BUILD_AUTHORIZED_DISABLED', *[f'IMPLEMENTATION_M{i}' for i in range(6)], 'CERTIFICATION_CANDIDATE', 'OPERATIONAL_ADOPTION_PENDING', 'OPERATIONALLY_ADOPTED', 'CERTIFIED_ACTIVE']
    transitions = {s: [states[i+1]] if i+1 < len(states) else [] for i, s in enumerate(states)}
    if lifecycle.get('states') != states or lifecycle.get('transitions') != transitions or lifecycle.get('architecture_freeze_confers_authority') is not False or lifecycle.get('build_grants_operational_authority') is not False:
        raise ContractError('contradictory release lifecycle')
    if set(document['record_types']) != set(document['records']):
        raise ContractError('record inventory mismatch')
    for record in document['records'].values():
        if type(record['version']) is not int or record['version'] != 1:
            raise ContractError('unsupported record version')
        _schema(dict(type='object', fields=record['fields'], required=record['required']))
    for spec in document['nested_records'].values():
        _schema(spec)


def _shape(record: Any, name: str, schemas: Mapping[str, Any]) -> dict[str, Any]:
    if name not in schemas['records']:
        raise ContractError('unknown record schema')
    schema = schemas['records'][name]
    _value(record, dict(type='object', fields=schema['fields'], required=schema['required']), name)
    return record


def validate_task_contract(record: Any, schemas: Mapping[str, Any], *, frozen_template: Mapping[str, Any] | None = None) -> dict[str, Any]:
    record = _shape(record, 'task_contract', schemas)
    checks = record['check_argv']
    ids = [c['id'] for c in checks]
    _unique(ids, 'checks')
    if ids != record['required_check_ids'] or ids != record['required_tests']:
        raise ContractError('required check inventory differs')
    cases = [case for c in checks for case in c['required_cases']]
    _unique(cases, 'check cases')
    if set(cases) != set(record['case_inventory']):
        raise ContractError('case inventory differs')
    for check in checks:
        if not check['required_cases'] and not check['allow_empty_cases']:
            raise ContractError('empty case inventory was not explicitly authorized')
        if check['environment_digest'] != canonical_digest(check['environment']):
            raise ContractError('environment digest mismatch')
        if check['timeout_seconds'] > check['resource_limits']['wall_seconds'] or check['timeout_seconds'] > record['budget']['wall_seconds']:
            raise ContractError('timeout exceeds budget')
        for key, bound in record['resource_limits'].items():
            if check['resource_limits'][key] > bound:
                raise ContractError('check resource limit exceeds task')
    requirements = record['acceptance_requirements']
    _unique([a['class_id'] for a in requirements], 'acceptance classes')
    accept_cases = [case for a in requirements for case in a['case_ids']]
    if set(a['class_id'] for a in requirements) != set(record['evidence_classes']) or set(accept_cases) != set(record['acceptance_cases']):
        raise ContractError('acceptance inventory mismatch')
    perf = record['performance_budgets']
    if perf['applicability'] != record['performance_applicability']:
        raise ContractError('performance applicability mismatch')
    applicable = perf['applicability'] == 'applicable'
    required = {'baseline_digest', 'method_digest', 'duration_seconds', 'sample_count', 'bounds', 'ownership_constraints'}
    if applicable and not required <= set(perf) or not applicable and set(perf) != {'applicability', 'authorization_digest', 'rationale'}:
        raise ContractError('performance contract incomplete or unauthorized N/A fields')
    if applicable:
        _unique([b['metric'] for b in perf['bounds']], 'performance metrics')
    if {m for check in checks for m in check['required_metrics']} != {b['metric'] for b in perf.get('bounds', [])}:
        raise ContractError('required measurement inventory differs from performance floor')
    if frozen_template is None or canonical_digest(record) != canonical_digest(frozen_template):
        raise ContractError('frozen task template/floor binding required or differs')
    return record


def _bind(record: Mapping[str, Any], expected: Mapping[str, Any] | None, keys: Iterable[str]) -> None:
    if expected is None:
        raise ContractError('independent frozen receipt context required')
    for key in keys:
        if key not in expected or type(record.get(key)) is not type(expected[key]) or record.get(key) != expected[key]:
            raise ContractError('receipt context mismatch: ' + key)


def validate_verification_receipt(record: Any, schemas: Mapping[str, Any], *, task: Mapping[str, Any], candidate_sha: str, context: Mapping[str, Any]) -> dict[str, Any]:
    record = _shape(record, 'verification_receipt', schemas)
    validate_task_contract(task, schemas, frozen_template=context.get('frozen_task'))
    check = next((c for c in task['check_argv'] if c['id'] == record['check_id']), None)
    if check is None:
        raise ContractError('unknown check')
    expected = dict(context, task_id=task['task_id'], candidate_sha=candidate_sha, authority_digest=task['authority_digest'], task_contract_digest=canonical_digest(task), command_digest=canonical_digest(check), environment_digest=check['environment_digest'])
    _bind(expected, context, ['task_id', 'candidate_sha', 'authority_digest', 'task_contract_digest', 'command_digest', 'environment_digest'])
    _bind(record, expected, ['task_id', 'candidate_sha', 'authority_digest', 'task_contract_digest', 'command_digest', 'environment_digest', 'lease_epoch', 'sequence', 'stage_nonce'])
    required = set(check['required_cases'])
    executed, passed, failed, skipped = (set(record[k]) for k in ('executed_cases', 'passed_cases', 'failed_cases', 'skipped_cases'))
    if not (executed | passed | failed | skipped) <= required or passed & failed or executed & skipped or passed | failed != executed:
        raise ContractError('contradictory/unknown case results')
    if record['duration_seconds'] > check['timeout_seconds'] and not record['timed_out']:
        raise ContractError('elapsed duration contradicts timeout')
    if record['signal'] is not None and (type(record['signal']) is not int or record['signal'] <= 0):
        raise ContractError('invalid signal')
    if record['outcome'] == 'PASS':
        if record['exit_code'] not in check['expected_exit_codes'] or record['timed_out'] or record['signal'] is not None or not record['executable_found'] or passed != required or executed != required or failed or skipped:
            raise ContractError('PASS contradicts command/case outcome')
        measurements = {m['metric']: m['value'] for m in record['measurements']}
        _unique([m['metric'] for m in record['measurements']], 'measurements')
        perf = task['performance_budgets']
        bounds = [b for b in perf.get('bounds', []) if b['metric'] in check['required_metrics']]
        if set(measurements) != {b['metric'] for b in bounds}:
            raise ContractError('missing/unknown measurements')
        if any(measurements[b['metric']] > min(b['maximum'], b['baseline_maximum']) for b in bounds):
            raise ContractError('performance/resource measurement failed')
    return record


def validate_record(record: Any, record_name: str, schemas: Mapping[str, Any], *, context: Mapping[str, Any] | ValidatedReleaseAuthority | None = None) -> dict[str, Any]:
    if record_name == 'task_contract':
        return validate_task_contract(record, schemas, frozen_template=context)
    if record_name == 'adoption_record':
        return validate_adoption_record(record, schemas, external=context)
    if record_name == 'control_amendment_marker':
        return validate_control_amendment_marker(record, schemas)
    if record_name in ('build_authorization', 'certification_bundle', 'operational_adoption_pin'):
        raise ContractError('external authority records require the Git pin loader')
    record = _shape(record, record_name, schemas)
    if record_name == 'verification_receipt':
        if context is None:
            raise ContractError('verification needs frozen task/check/attempt context')
        return validate_verification_receipt(record, schemas, **context)
    identity = {'task_id', 'candidate_sha', 'base_sha', 'checkpoint_id', 'authority_digest', 'task_contract_digest', 'template_digest', 'lease_epoch', 'issuance_sequence'} & set(record)
    # Receipt references are controller-owned bindings, not self-authored hashes.
    identity |= {key for key in record if key.endswith('_digest') or key.endswith('_digests')}
    _bind(record, context, identity)
    if record_name == 'candidate_receipt' and (not record['imported'] or not record['clean_product_tree']):
        raise ContractError('candidate import/cleanliness not established')
    if record_name == 'remote_promotion_receipt' and (record['destination_ref'] != 'refs/heads/main' or record['observed_remote_sha'] != record['candidate_sha']):
        raise ContractError('remote promotion not observed')
    if record_name == 'review_report':
        _unique([f['id'] for f in record['findings']], 'findings')
        _unique([f['id'] for f in record['quality_flag_dispositions']], 'quality flags')
        _bind(record, context, ['coverage'])
        if record['verdict'] == 'PASS' and (record['unresolved_questions'] or any(f['severity'] == 'BLOCKING' for f in record['findings']) or any(f['disposition'] != 'NOT_LOWERING' for f in record['quality_flag_dispositions'])):
            raise ContractError('review PASS contradicts unresolved quality obligations')
    if record_name == 'control_plane_receipt':
        _bind(record, context, ['production_acceptance_receipts'])
        for receipt in record['production_acceptance_receipts']:
            if receipt['result'] != 'PASS' or receipt['failed_cases'] or receipt['skipped_cases'] or set(receipt['case_ids']) != set(receipt['passed_cases']) or set(receipt['passed_cases']) != set(receipt['executed_cases']):
                raise ContractError('production acceptance missing/failed cases')
    return record


def validate_control_plane_receipt(record: Any, schemas: Mapping[str, Any], *, context: Mapping[str, Any] | None = None) -> dict[str, Any]:
    return validate_record(record, 'control_plane_receipt', schemas, context=context)


@dataclass(frozen=True)
class RecordPin:
    """Exact controller Git blob identity selected by operator bootstrap."""
    path: str
    digest: str


@dataclass(frozen=True)
class ControllerBootstrap:
    """Trusted host input, never deserialized from a candidate/model record.

    M0 defines the pin contract; later runtime owns and protects its source.
    A pin is approval supplied out of band, not approval inferred from Git authors.
    """
    source_sha: str
    anchor_sha: str
    base_sha: str
    release_sha: str
    candidate_branch: str
    authorization_id: str
    task_id: str
    sequence: int
    nonce: str
    sandbox_digest: str
    build: RecordPin
    certification: RecordPin | None = None
    adoption: RecordPin | None = None


_PROVENANCE_SEAL = object()


@dataclass(frozen=True, init=False)
class ValidatedReleaseAuthority:
    """Immutable validated snapshot; ordinary mappings cannot construct it.

    This is a host API type barrier, not a sandbox for arbitrary Python execution.
    Only load_release_authority reads operator-pinned records and mints snapshots.
    """
    payload_json: str
    candidate_root: str
    controller_root: str
    source_sha: str

    def __init__(self, payload: Mapping[str, Any], candidate_root: Path, controller_root: Path, source_sha: str, *, _seal: object = None):
        if _seal is not _PROVENANCE_SEAL:
            raise ContractError('validated provenance requires the Git pin loader')
        object.__setattr__(self, 'payload_json', canonical_json(payload))
        object.__setattr__(self, 'candidate_root', str(candidate_root))
        object.__setattr__(self, 'controller_root', str(controller_root))
        object.__setattr__(self, 'source_sha', source_sha)


def _release_authority(external: Any) -> dict[str, Any]:
    if type(external) is not ValidatedReleaseAuthority:
        raise ContractError('validated external Git provenance required; raw mappings confer no authority')
    payload = load_json_strict(external.payload_json)
    for root, sha in ((Path(external.controller_root), external.source_sha), (Path(external.candidate_root), payload['git']['release_oid'])):
        _authority_repo(root, payload['git']['anchor_oid'])
        if _git(root, 'rev-parse', 'HEAD').decode().strip() != sha or _git(root, 'status', '--porcelain=v1', '--untracked-files=all').strip():
            raise ContractError('stale or dirty provenance materialization')
    _verify_materialization(Path(external.candidate_root), payload['git']['manifest'])
    return payload


def executing_build_phase(build: Mapping[str, Any]) -> str:
    """The single externally authorized executing phase derived from build progress.

    `completed_phases` must be the exact ordered prefix immediately preceding
    the executing phase. A future, skipped, reordered, duplicated, or replayed
    phase refuses here; worker-controlled data never determines the result.
    """
    progress = build.get('completed_phases')
    if (not isinstance(progress, list) or progress != list(IMPLEMENTATION_PHASES[:len(progress)])
            or len(progress) >= len(IMPLEMENTATION_PHASES)):
        raise ContractError('build authorization is not executing an implementation phase')
    executing = IMPLEMENTATION_PHASES[len(progress)]
    if executing not in build.get('authorized_phases', []):
        raise ContractError('executing phase is outside the external build authorization')
    return executing


def validate_shared_capability(external: Any, capability: str) -> dict[str, Any]:
    """A shared primitive requires only its own prerequisite capability phase.

    Implementation ownership never pins execution: the store and sandbox (M1)
    may serve any executing phase at or after M1; guards/floor (M2) may serve
    any executing phase at or after M2, always under exact authorized progress.
    This is capability reuse, not authority widening.
    """
    payload = _release_authority(external)
    if capability not in IMPLEMENTATION_PHASES:
        raise ContractError('unknown shared capability phase')
    executing = executing_build_phase(payload['build'])
    if IMPLEMENTATION_PHASES.index(executing) < IMPLEMENTATION_PHASES.index(capability):
        raise ContractError('shared control primitive does not exist yet: ' + capability)
    return payload


def _phase_admission(payload: Mapping[str, Any], task: Mapping[str, Any], capability: str | None = None) -> dict[str, Any]:
    """ONE phase-aware admission rule for shared control-plane reuse.

    The task is admitted against the current externally authorized executing
    phase; a shared primitive additionally requires its own prerequisite
    capability. Exact task binding (task ID, base SHA, candidate branch,
    authority digest, allowed-paths subset) remains mandatory. A past phase
    replay, a future/skipped phase, a rewritten checkpoint, a product task, or
    any binding mismatch refuses.
    """
    build = payload['build']
    if task.get('task_kind') != 'control_plane_phase' or task.get('checkpoint_id') not in IMPLEMENTATION_PHASES:
        raise ContractError('task is not an authorized control-plane phase task')
    executing = executing_build_phase(build)
    if task['checkpoint_id'] != executing:
        raise ContractError('task phase is not the externally executing phase')
    if capability is not None:
        if capability not in IMPLEMENTATION_PHASES:
            raise ContractError('unknown shared capability phase')
        if IMPLEMENTATION_PHASES.index(executing) < IMPLEMENTATION_PHASES.index(capability):
            raise ContractError('shared control primitive does not exist yet: ' + capability)
    if (task['task_id'] != build['task_id'] or task['base_sha'] != build['base_sha']
            or task['candidate_branch'] != build['candidate_branch']
            or task['authority_digest'] != payload['git']['authority_digest']
            or not set(task['allowed_paths']) <= set(build['allowed_paths'])):
        raise ContractError('task differs from external authority/base/scope')
    return payload


def validate_phase_admission(external: Any, task: Mapping[str, Any], *, capability: str | None = None) -> dict[str, Any]:
    """Admit the exact externally executing phase task against validated provenance."""
    return _phase_admission(_release_authority(external), task, capability)


def load_release_authority(candidate_root: Path, controller_root: Path, *, bootstrap: ControllerBootstrap) -> ValidatedReleaseAuthority:
    """Read independent controller records at externally approved exact pins.

    No worker can supply bootstrap or controller_root. Later host bootstrap must
    enforce that ownership; this contract neither implements runtime ownership
    nor treats a well-shaped candidate file as operator approval.
    """
    if type(bootstrap) is not ControllerBootstrap:
        raise ContractError('controller-owned typed bootstrap required')
    candidate = _authority_repo(candidate_root, bootstrap.anchor_sha)
    controller = _authority_repo(controller_root, bootstrap.anchor_sha)
    if candidate == controller or candidate in controller.parents or controller in candidate.parents:
        raise ContractError('candidate cannot serve as its own controller provenance')
    for root in (candidate, controller):
        _commit(root, ARCHITECTURE_SPEC_SHA)
        _ancestor(root, bootstrap.anchor_sha, ARCHITECTURE_SPEC_SHA)
        _ancestor(root, ARCHITECTURE_SPEC_SHA, bootstrap.release_sha if root == candidate else bootstrap.source_sha)
    _commit(controller, bootstrap.source_sha)
    if _git(controller, 'rev-parse', 'HEAD').decode().strip() != bootstrap.source_sha or _git(controller, 'status', '--porcelain=v1', '--untracked-files=all').strip():
        raise ContractError('stale or dirty controller provenance')
    git = load_authority_manifest(candidate, release_oid=bootstrap.release_sha, base_oid=bootstrap.base_sha, anchor_oid=bootstrap.anchor_sha, purpose='BUILD_AUTHORIZED_DISABLED', sandbox_digest=bootstrap.sandbox_digest, contract_versions=dict(project_schema=7, recovery_schema=1, ipc_protocol=1))
    schemas = load_json_strict(_git(candidate, 'show', bootstrap.release_sha + ':' + PROTOCOL_SCHEMAS_PATH).decode())
    validate_protocol_schemas(schemas)
    checks = load_json_strict(_git(candidate, 'show', bootstrap.release_sha + ':' + CHECKS_PATH).decode())
    ownership = {item['id']: item['phase'] for item in checks['acceptance_cases']}
    if len(checks['acceptance_cases']) != 48 or set(ownership) != set(ACCEPTANCE_CASE_IDS) or set(ownership.values()) != set(IMPLEMENTATION_PHASES) or {case for case, phase in ownership.items() if phase == 'M0'} != set(M0_OWNED_CASES):
        raise ContractError('invalid frozen phase/case ownership')
    expected = dict(repository=REPOSITORY_IDENTITY, architecture_spec_sha=ARCHITECTURE_SPEC_SHA, base_sha=bootstrap.base_sha, release_sha=bootstrap.release_sha, candidate_branch=bootstrap.candidate_branch, authorization_id=bootstrap.authorization_id, task_id=bootstrap.task_id, sequence=bootstrap.sequence, nonce=bootstrap.nonce, sandbox_digest=bootstrap.sandbox_digest, authority_manifest_digest=canonical_digest(git['manifest']), checks_digest=canonical_digest(checks))
    def read(pin: RecordPin | None, name: str) -> dict[str, Any] | None:
        if pin is None:
            return None
        if type(pin) is not RecordPin:
            raise ContractError('typed exact record pin required')
        _validate_relative_path(pin.path, 'controller record path')
        _value(pin.digest, {'type': 'digest'}, 'controller record digest')
        record = load_json_strict(_git(controller, 'show', bootstrap.source_sha + ':' + pin.path).decode())
        _shape(record, name, schemas)
        if canonical_digest(record) != pin.digest:
            raise ContractError('external record pin differs from Git blob')
        _bind(record, expected, expected)
        return record
    build = read(bootstrap.build, 'build_authorization')
    if build is None:
        raise ContractError('build authorization provenance missing')
    scope = {key: build[key] for key in ('authorized_phases', 'allowed_paths', 'required_gates', 'required_case_ids')}
    if build['scope_digest'] != canonical_digest(scope) or validate_adoption_diff(build['allowed_paths']):
        raise ContractError('disabled build scope/gates mismatch')
    if set(build['required_case_ids']) != {case for case, owner in ownership.items() if owner in build['authorized_phases']}:
        raise ContractError('build authorization omits or substitutes phase-owned required cases')
    indices = [IMPLEMENTATION_PHASES.index(phase) for phase in build['authorized_phases']]
    if indices != list(range(indices[0], indices[-1]+1)):
        raise ContractError('build phase scope must be explicit ordered contiguous bounds')
    progress = build['completed_phases']
    if len(progress) < indices[0] or len(progress) > indices[-1]+1:
        raise ContractError('authorized phase scope lacks prerequisite progress evidence')
    if progress != list(IMPLEMENTATION_PHASES[:len(progress)]) or [e['phase'] for e in build['phase_evidence']] != progress:
        raise ContractError('build progress lacks ordered controller phase evidence')
    for evidence in build['phase_evidence']:
        if set(evidence['case_ids']) != {case for case, phase in ownership.items() if phase == evidence['phase']}:
            raise ContractError('completed phase evidence omits owned cases')
    certification = read(bootstrap.certification, 'certification_bundle')
    if certification is not None:
        receipts = certification['case_receipts']
        _unique([r['case_id'] for r in receipts], 'certification cases')
        if certification['completed_phases'] != list(IMPLEMENTATION_PHASES) or progress != list(IMPLEMENTATION_PHASES) or set(r['case_id'] for r in receipts) != set(ACCEPTANCE_CASE_IDS) or certification['blocking_limitations']:
            raise ContractError('certification requires full phase/case evidence without blockers')
        if any(r['result'] != 'PASS' or r['phase'] != ownership[r['case_id']] for r in receipts):
            raise ContractError('certification case failed or phase ownership differs')
        for name in ('live_certification', 'independent_review', 'qualified_models'):
            evidence = certification[name]
            if evidence['result'] != 'PASS' or evidence['release_sha'] != bootstrap.release_sha:
                raise ContractError('certification evidence failed or wrong release')
        if certification['qualified_models']['required_models_available'] is not True:
            raise ContractError('required model qualification unavailable')
    adoption = read(bootstrap.adoption, 'operational_adoption_pin')
    if adoption is not None:
        if certification is None:
            raise ContractError('operator adoption requires certification provenance')
        expected_adoption = dict(certified_release_sha=bootstrap.release_sha, certification_digest=canonical_digest(certification), parent_sha=bootstrap.base_sha, plan_digest=hashlib.sha256(_git(candidate, 'show', bootstrap.base_sha + ':docs/execution/PLAN.json')).hexdigest(), state_digest=hashlib.sha256(_git(candidate, 'show', bootstrap.base_sha + ':docs/execution/STATE.json')).hexdigest())
        expected_adoption.update({name + '_digest': certification[name]['digest'] for name in ('live_certification', 'independent_review', 'qualified_models')})
        _bind(adoption, expected_adoption, expected_adoption)
    payload = dict(git=git, schemas=schemas, ownership=ownership, build=build, certification=certification, adoption=adoption)
    return ValidatedReleaseAuthority(payload, candidate, controller, bootstrap.source_sha, _seal=_PROVENANCE_SEAL)


def proposal_record(*, architecture_frozen: bool = True, **changes: Any) -> dict[str, Any]:
    record = dict(schema_version=1, authority_kind='V2_DESIGN_CANDIDATE', architecture_frozen=architecture_frozen, architecture_spec_sha=ARCHITECTURE_SPEC_SHA if architecture_frozen else None, build_authorization_digest=None, operational_adoption='PENDING', parent_sha=None, adopted_release_sha=None, implementation_phase='NONE', completed_phases=[], acceptance_cases_passed=[], certification='NONE', certified_release_sha=None, live_certification_digest=None, independent_review_digest=None, qualified_models_digest=None, blocking_limitations=[], full_auto_eligible=False, changed_paths=[], plan_digest=None, state_digest=None, amendment_proposed=True, adoption_requested=False, lifecycle_state='AMENDMENT_PROPOSED')
    record.update(changes)
    return record


def derive_release_lifecycle_state(record: Any, schemas: Mapping[str, Any], *, external: ValidatedReleaseAuthority | None = None) -> str:
    """One deterministic derivation. Serialized state/eligibility are checked later."""
    record = _shape(record, 'adoption_record', schemas)
    if record['architecture_frozen'] is not True or record['architecture_spec_sha'] != ARCHITECTURE_SPEC_SHA:
        raise ContractError('release lifecycle requires the exact frozen architecture')
    if validate_adoption_diff(record['changed_paths']):
        raise ContractError('adoption changed forbidden product/control paths')
    progress, phase = record['completed_phases'], record['implementation_phase']
    if progress != list(IMPLEMENTATION_PHASES[:len(progress)]):
        raise ContractError('implementation progress must be ordered prefix')
    if phase == 'NONE' and progress:
        raise ContractError('completed implementation cannot rewind to the initial build state')
    authorized = record['build_authorization_digest'] is not None
    facts = _release_authority(external) if external is not None else None
    if facts is not None and canonical_digest(schemas) != canonical_digest(facts['schemas']):
        raise ContractError('protocol differs from pinned authority')
    if authorized:
        if facts is None or not record['amendment_proposed'] or record['build_authorization_digest'] != canonical_digest(facts['build']):
            raise ContractError('implementation requires exact validated build provenance')
        if progress != facts['build']['completed_phases']:
            raise ContractError('progress differs from controller phase evidence')
        if phase != 'NONE' and (phase not in facts['build']['authorized_phases'] or len(progress) not in (IMPLEMENTATION_PHASES.index(phase), IMPLEMENTATION_PHASES.index(phase)+1)):
            raise ContractError('phase outside build permission or inconsistent with progress')
        owned = {case for case, owner in facts['ownership'].items() if owner in progress or owner == phase}
        if not set(record['acceptance_cases_passed']) <= owned:
            raise ContractError('acceptance case claimed before its owning phase')
    elif phase != 'NONE' or progress or record['acceptance_cases_passed'] or record['certification'] != 'NONE':
        raise ContractError('implementation/certification requires build authorization')
    certified = record['certification'] != 'NONE'
    certification = facts['certification'] if facts else None
    if certified:
        if not authorized or certification is None or phase != 'M5' or progress != list(IMPLEMENTATION_PHASES) or set(record['acceptance_cases_passed']) != set(ACCEPTANCE_CASE_IDS):
            raise ContractError('certification requires complete validated phase/case evidence')
        expected = dict(certified_release_sha=certification['release_sha'], blocking_limitations=certification['blocking_limitations'])
        expected.update({name + '_digest': certification[name]['digest'] for name in ('live_certification', 'independent_review', 'qualified_models')})
        _bind(record, expected, expected)
    elif any(record[k] is not None for k in ('certified_release_sha', 'live_certification_digest', 'independent_review_digest', 'qualified_models_digest')):
        raise ContractError('uncertified release cannot claim certification identities')
    adopted = record['operational_adoption'] == 'ADOPTED'
    if adopted:
        adoption = facts['adoption'] if facts else None
        if not certified or adoption is None or record['authority_kind'] != 'V2_FROZEN_CONTROL_RELEASE':
            raise ContractError('operational adoption requires exact validated operator pin')
        expected = {key: adoption[key] for key in ('parent_sha', 'plan_digest', 'state_digest')}
        expected['adopted_release_sha'] = adoption['certified_release_sha']
        _bind(record, expected, expected)
        if record['adopted_release_sha'] != record['certified_release_sha']:
            raise ContractError('adopted release differs from certified release')
    elif record['adopted_release_sha'] is not None or record['authority_kind'] != 'V2_DESIGN_CANDIDATE' or any(record[k] is not None for k in ('parent_sha', 'plan_digest', 'state_digest')):
        raise ContractError('disabled candidate cannot claim runtime authority/provenance')
    if record['certification'] == 'CERTIFIED_ACTIVE' and not adopted:
        raise ContractError('active certification requires external operational adoption')
    if record['adoption_requested'] and not certified:
        raise ContractError('adoption request precedes complete certification')
    if record['blocking_limitations']:
        raise ContractError('release has unresolved blocking limitations')
    if adopted:
        return 'CERTIFIED_ACTIVE' if record['certification'] == 'CERTIFIED_ACTIVE' else 'OPERATIONALLY_ADOPTED'
    if certified:
        return 'OPERATIONAL_ADOPTION_PENDING' if record['adoption_requested'] else 'CERTIFICATION_CANDIDATE'
    if phase != 'NONE':
        return 'IMPLEMENTATION_' + phase
    if authorized:
        return 'BUILD_AUTHORIZED_DISABLED'
    return 'AMENDMENT_PROPOSED' if record['amendment_proposed'] else 'ARCHITECTURE_FROZEN'


def validate_adoption_record(record: Any, schemas: Mapping[str, Any], *, external: ValidatedReleaseAuthority | None = None) -> dict[str, Any]:
    state = derive_release_lifecycle_state(record, schemas, external=external)
    if record['lifecycle_state'] != state or record['full_auto_eligible'] is not (state == 'CERTIFIED_ACTIVE'):
        raise ContractError('serialized lifecycle/eligibility contradicts deterministic derivation')
    return record


def full_auto_eligible(record: Mapping[str, Any], *, schemas: Mapping[str, Any] | None = None, external: ValidatedReleaseAuthority | None = None) -> bool:
    if schemas is None or external is None:
        return False
    try:
        validate_adoption_record(record, schemas, external=external)
        return record['full_auto_eligible'] is True
    except (ContractError, KeyError, TypeError):
        return False


def resolve_authority_root(explicit_root: Any) -> Path:
    if not isinstance(explicit_root, (str, Path)):
        raise ContractError('explicit authority root required')
    root = Path(explicit_root)
    if not root.is_absolute() or not root.is_dir() or root.is_symlink():
        raise ContractError('authority root must be absolute existing non-symlink directory')
    return root.resolve()


def _git(root: Path, *args: str) -> bytes:
    env = {'PATH': os.environ.get('PATH', ''), 'LANG': 'C', 'LC_ALL': 'C', 'GIT_CONFIG_NOSYSTEM': '1', 'GIT_CONFIG_GLOBAL': os.devnull, 'GIT_NO_REPLACE_OBJECTS': '1', 'GIT_TERMINAL_PROMPT': '0', 'GIT_NO_LAZY_FETCH': '1'}
    try:
        result = subprocess.run(['git', '--no-optional-locks', '-c', 'core.hooksPath=' + os.devnull, '-c', 'core.fsmonitor=false', '-c', 'core.attributesFile=' + os.devnull, *args], cwd=root, env=env, capture_output=True, timeout=20, check=True)
    except (OSError, subprocess.SubprocessError) as exc:
        raise ContractError('Git authority operation failed: ' + args[0]) from exc
    if len(result.stdout) > 64 * 1024 * 1024:
        raise ContractError('Git authority output bound exceeded')
    return result.stdout


def _commit(root: Path, oid: Any) -> None:
    _value(oid, {'type': 'sha'}, 'Git revision')
    if _git(root, 'cat-file', '-t', oid).strip() != b'commit':
        raise ContractError('authority revision must be exact commit object, not tag/blob/tree')


def _ancestor(root: Path, first: str, second: str) -> None:
    _git(root, 'merge-base', '--is-ancestor', first, second)


def _authority_repo(repo_root: Path, anchor_oid: str) -> Path:
    root = resolve_authority_root(repo_root)
    if not (root / '.git').is_dir() or (root / '.git').is_symlink():
        raise ContractError('trusted authority must be independent Git repository root')
    top = Path(_git(root, 'rev-parse', '--show-toplevel').decode().strip()).resolve()
    if top != root:
        raise ContractError('wrong repository root')
    urls = _git(root, 'config', '--local', '--get-all', 'remote.origin.url').decode().splitlines()
    accepted = {f'https://github.com/{REPOSITORY_IDENTITY}.git', f'https://github.com/{REPOSITORY_IDENTITY}', f'git@github.com:{REPOSITORY_IDENTITY}.git', f'ssh://git@github.com/{REPOSITORY_IDENTITY}.git'}
    if len(urls) != 1 or urls[0] not in accepted:
        raise ContractError('wrong repository identity')
    if _git(root, 'rev-parse', '--is-shallow-repository').strip() != b'false':
        raise ContractError('shallow authority forbidden')
    if _git(root, 'for-each-ref', '--format=%(refname)', 'refs/replace/').strip():
        raise ContractError('replace refs forbidden')
    for relative in ('info/grafts', 'objects/info/alternates'):
        if (root / '.git' / relative).exists():
            raise ContractError('grafts/alternates forbidden')
    _commit(root, anchor_oid)
    return root


def build_manifest(repo_root: Path, revision: str, relative_paths: Iterable[str] | None = None, *, _base_input: bool = False) -> list[dict[str, Any]]:
    """Full pinned tree; filesystem bytes never supply authority."""
    root = resolve_authority_root(repo_root)
    _commit(root, revision)
    output = _git(root, 'ls-tree', '-r', '-z', '--full-tree', revision)
    if not output or not output.endswith(b'\x00'):
        raise ContractError('empty/truncated tree inventory')
    inventory = {raw.split(b'\t', 1)[1].decode('utf-8') for raw in output[:-1].split(b'\x00')}
    if relative_paths is not None:
        requested = list(relative_paths)
        _unique(requested, 'manifest inventory')
        if set(requested) != inventory | set(CONFIG_PATHS):
            raise ContractError('manifest inventory omission/addition')
    entries = []
    for raw in output[:-1].split(b'\x00'):
        try:
            metadata, name = raw.split(b'\t', 1)
            mode, kind, oid = metadata.decode('ascii').split(' ')
            path = name.decode('utf-8')
        except (ValueError, UnicodeError) as exc:
            raise ContractError('malformed tree entry') from exc
        _validate_relative_path(path, 'Git tree path')
        if mode not in ('100644', '100755') or kind != 'blob':
            raise ContractError('unsupported Git tree mode/type')
        data = _git(root, 'cat-file', 'blob', oid)
        entries.append(dict(path=path, mode=mode, blob_oid=oid, size=len(data), sha256=hashlib.sha256(data).hexdigest()))
    paths = {entry['path'] for entry in entries}
    if not _base_input:
        plan = load_json_strict(_git(root, 'show', revision + ':docs/execution/PLAN.json').decode())
        specifications = {c['spec_document'] for c in plan['checkpoints']} | {p['spec_document'] for p in plan['phases'].values()}
        if not specifications <= paths:
            raise ContractError('locked specification omitted from authority tree')
    if not _base_input and not REQUIRED_AUTHORITY_PATHS <= paths:
        raise ContractError('required authority surfaces omitted: ' + ', '.join(sorted(REQUIRED_AUTHORITY_PATHS - paths)))
    for path in CONFIG_PATHS:
        if path not in paths:
            entries.append(dict(path=path, mode='absent', blob_oid=None, size=0, sha256=None))
    entries.sort(key=lambda item: item['path'])
    if relative_paths is not None:
        requested = list(relative_paths)
        _unique(requested, 'manifest inventory')
        if set(requested) != {entry['path'] for entry in entries}:
            raise ContractError('manifest inventory omission/addition')
    return entries


def _authority_path(path: str) -> bool:
    return path in AUTHORITY_EXACT_PATHS or path.startswith(AUTHORITY_PREFIXES)


def _verify_materialization(root: Path, manifest: list[dict[str, Any]]) -> None:
    # Read filesystem only to reject drift hidden by assume-unchanged/index flags.
    for entry in manifest:
        path = root / entry['path']
        if entry['mode'] == 'absent':
            if path.exists() or path.is_symlink():
                raise ContractError('absent frozen config appeared')
            continue
        for parent in (path, *path.parents):
            if parent == root:
                break
            if parent.is_symlink():
                raise ContractError('authority symlink escape')
        if not path.is_file() or hashlib.sha256(path.read_bytes()).hexdigest() != entry['sha256'] or bool(path.stat().st_mode & 0o111) != (entry['mode'] == '100755'):
            raise ContractError('working file differs from pinned release: ' + entry['path'])


def load_authority_manifest(repo_root: Path, *, release_oid: str, base_oid: str, sandbox_digest: str, contract_versions: Mapping[str, Any], anchor_oid: str = TRUSTED_DESIGN_BASE, purpose: str, relative_paths: Iterable[str] | None = None) -> dict[str, Any]:
    root = _authority_repo(repo_root, anchor_oid)
    for oid in (release_oid, base_oid):
        _commit(root, oid)
        _ancestor(root, anchor_oid, oid)
    if purpose not in ('BUILD_AUTHORIZED_DISABLED', 'OPERATIONAL'):
        raise ContractError('explicit authority purpose required')
    _ancestor(root, base_oid if purpose == 'BUILD_AUTHORIZED_DISABLED' else release_oid, release_oid if purpose == 'BUILD_AUTHORIZED_DISABLED' else base_oid)
    _value(sandbox_digest, {'type': 'digest'}, 'sandbox digest')
    version_spec = {'type': 'object', 'fields': {k: {'type': 'integer', 'minimum': 1} for k in CONTRACT_VERSION_KEYS}, 'required': sorted(CONTRACT_VERSION_KEYS)}
    _value(dict(contract_versions), version_spec, 'versions')
    if _git(root, 'rev-parse', 'HEAD').decode().strip() != release_oid or _git(root, 'status', '--porcelain=v1', '--untracked-files=all').strip():
        raise ContractError('dirty or mixed-revision authority materialization')
    manifest = build_manifest(root, release_oid, relative_paths)
    base_manifest = build_manifest(root, base_oid, _base_input=True)
    if not {e['path'] for e in base_manifest if _authority_path(e['path']) and e['mode'] != 'absent'} <= {e['path'] for e in manifest if e['mode'] != 'absent'}:
        raise ContractError('baseline authority surface removed from release')
    _verify_materialization(root, manifest)
    if purpose == 'OPERATIONAL':
        base_entries = {e['path']: e for e in base_manifest}
        for entry in manifest:
            path = entry['path']
            if _authority_path(path) and path != 'docs/execution/STATE.json' and not path.startswith(('docs/execution/evidence/', '.github/workflows/')) and base_entries.get(path) != entry:
                raise ContractError('control release/base mismatch')
    version_sources = {'project_schema': ('crates/or_core/src/project_document.rs', 'CURRENT_PROJECT_SCHEMA_VERSION'), 'recovery_schema': ('crates/or_core/src/project_recovery.rs', 'CURRENT_RECOVERY_SCHEMA_VERSION'), 'ipc_protocol': ('crates/or_ipc/src/protocol.rs', 'OR_LOCAL_IPC_PROTOCOL_VERSION')}
    for key, (path, constant) in version_sources.items():
        source = _git(root, 'show', f'{base_oid}:{path}').decode('utf-8')
        match = re.search(r'\bpub\s+const\s+' + constant + r'\s*:\s*u\d+\s*=\s*(\d+)\s*;', source)
        if not match or int(match[1]) != contract_versions[key]:
            raise ContractError('contract version differs from pinned base')
    payload = dict(schema_version=2, repository=REPOSITORY_IDENTITY, anchor_oid=anchor_oid, release_oid=release_oid, base_oid=base_oid, purpose=purpose, manifest=manifest, base_manifest=base_manifest, sandbox_digest=sandbox_digest, contract_versions=dict(contract_versions))
    payload['authority_digest'] = canonical_digest(payload)
    return payload


def verify_authority_binding(expected: Mapping[str, Any], current: Mapping[str, Any]) -> None:
    for payload in (expected, current):
        if 'authority_digest' not in payload or payload['authority_digest'] != canonical_digest({k: v for k, v in payload.items() if k != 'authority_digest'}):
            raise ContractError('corrupt authority digest')
    if canonical_digest(expected) != canonical_digest(current):
        raise ContractError('frozen authority drift')


def frozen_source_closure_violations(changed_paths: Iterable[str], frozen_paths: Iterable[str] | None = None) -> list[str]:
    return sorted({path for path in changed_paths if _authority_path(path) or frozen_paths is not None and path in set(frozen_paths)})


def validate_adoption_diff(changed_paths: Iterable[str]) -> list[str]:
    return sorted({p for p in changed_paths if p not in ADOPTION_ALLOWED_PATHS or any(p == f or p.startswith(f) for f in FORBIDDEN_ADOPTION_PATHS)})


def validate_adoption_provenance(*, architecture_spec_sha: Any, external_release_sha: Any, marker_contains_own_sha: bool = False) -> None:
    if marker_contains_own_sha or architecture_spec_sha != ARCHITECTURE_SPEC_SHA:
        raise ContractError('self-issued/wrong architecture provenance')
    _value(external_release_sha, {'type': 'sha'}, 'external release pin')


def validate_control_amendment_marker(marker: Any, schemas: Mapping[str, Any], *, expected_checkpoint: str | None = None) -> dict[str, Any]:
    marker = _shape(marker, 'control_amendment_marker', schemas)
    if expected_checkpoint is not None and marker['checkpoint_id'] != expected_checkpoint:
        raise ContractError('marker is not current NEXT')
    if marker['architecture_spec_sha'] != ARCHITECTURE_SPEC_SHA or validate_adoption_diff(marker['changed_paths']):
        raise ContractError('marker architecture/control scope mismatch')
    legacy = marker['legacy_quality_amendment']
    if legacy['prior_failed_run_id'] != PRIOR_FAILED_RUN_ID or legacy['prior_implementation_sha'] != PRIOR_IMPLEMENTATION_SHA:
        raise ContractError('legacy provenance changed')
    return marker


def validate_authority_source(authority_kind: Any, reference: str | None = None, *, external: ValidatedReleaseAuthority | None = None) -> None:
    facts = _release_authority(external)
    _value(reference, {'type': 'sha'}, 'operational authority reference')
    adoption = facts['adoption']
    if authority_kind != 'V2_FROZEN_CONTROL_RELEASE' or adoption is None or reference != adoption['certified_release_sha'] or reference != facts['git']['release_oid']:
        raise ContractError('exact positively verified operational adoption/Git authority required')


def validate_completion_evidence(record: Any, schemas: Mapping[str, Any], *, repo_root: Path, revision: str, evidence_path: str, adoption_sha: str, anchor_oid: str = TRUSTED_DESIGN_BASE, receipt_context: Mapping[str, Any] | None = None) -> dict[str, Any]:
    root = _authority_repo(repo_root, anchor_oid)
    for oid in (revision, adoption_sha):
        _commit(root, oid)
        _ancestor(root, anchor_oid, oid)
    _validate_relative_path(evidence_path, 'evidence path')
    actual = load_json_strict(_git(root, 'show', revision + ':' + evidence_path).decode('utf-8'))
    if canonical_digest(actual) != canonical_digest(record):
        raise ContractError('evidence differs from immutable Git blob')
    if type(record.get('schema_version')) is not int or record['schema_version'] not in (1, 2):
        raise ContractError('unsupported evidence version')
    try:
        _ancestor(root, adoption_sha, revision)
        post_adoption = True
    except ContractError:
        _ancestor(root, revision, adoption_sha)
        post_adoption = False
    if post_adoption:
        if record['schema_version'] != 2 or 'control_plane_receipt' not in record:
            raise ContractError('post-adoption completion requires nested receipt')
        validate_control_plane_receipt(record['control_plane_receipt'], schemas, context=receipt_context)
    # Reuse the existing evidence verifier, imported from trusted controller code.
    import execution_evidence
    plan = load_json_strict(_git(root, 'show', adoption_sha + ':docs/execution/PLAN.json').decode())
    policy = load_json_strict(_git(root, 'show', adoption_sha + ':docs/execution/EVIDENCE_POLICY.json').decode())
    checkpoint = next((c for c in plan['checkpoints'] if c['id'] == record.get('checkpoint_id')), None)
    if checkpoint is None:
        raise ContractError('evidence checkpoint absent')
    try:
        execution_evidence.validate_evidence_record(record, checkpoint_id=checkpoint['id'], checkpoint=checkpoint, policy=policy)
    except execution_evidence.EvidenceError as exc:
        raise ContractError(str(exc)) from exc
    return record


def validate_v2_contract_documents(repo_root: Path) -> None:
    root = resolve_authority_root(repo_root)
    schemas = load_protocol_schemas(root)
    contract = load_object(root / V2_CONTRACT_PATH, 'V2 contract')
    model = load_object(root / MODEL_POLICY_PATH, 'model policy')
    sandbox = load_object(root / SANDBOX_POLICY_PATH, 'sandbox')
    templates = load_object(root / TASK_TEMPLATES_PATH, 'templates')
    checks = load_object(root / CHECKS_PATH, 'checks')
    for document in (contract, model, sandbox, templates, checks):
        if type(document.get('schema_version')) is not int or document['schema_version'] != 2 or document.get('activation') != ACTIVATION_DISABLED:
            raise ContractError('candidate contract version/activation differs')
    adoption = contract['adoption']
    validate_adoption_record(adoption, schemas)
    if adoption['lifecycle_state'] != 'AMENDMENT_PROPOSED' or adoption != proposal_record():
        raise ContractError('M0 must remain proposed/disabled and uncertified')
    if contract['release_lifecycle']['states'] != schemas['adoption_lifecycle']['states'] or contract['release_lifecycle']['transitions'] != schemas['adoption_lifecycle']['transitions']:
        raise ContractError('documents disagree about release lifecycle')
    if contract.get('authority_provenance') != schemas.get('authority_provenance') or schemas.get('authority_provenance', {}).get('raw_mapping_confers_authority') is not False:
        raise ContractError('documents disagree about external provenance authority')
    if any(p.get('enrolled_models') for p in model['role_policy'].values()) or model['unknown_family_is_independent'] is not False or model['free_suffix_is_qualification'] is not False:
        raise ContractError('M0 may not enroll/qualify models')
    if templates['model_authored_goal_allowed'] is not False or templates['measurement_floor'] != 'exact_external_frozen_template_comparison':
        raise ContractError('untrusted template/floor')
    case_ids = [c['id'] for c in checks['acceptance_cases']]
    if set(case_ids) != set(ACCEPTANCE_CASE_IDS) or len(case_ids) != 48 or tuple(checks['m0_owned_cases']) != M0_OWNED_CASES:
        raise ContractError('acceptance case inventory differs')
