"""M3 model transport: pinned OpenCode/Codex adapters, selection, review parsing.

No dispatch happens on import. Every launch goes through explicit adapter calls
bound to a validated M3 build authority, task, store lease, and sandbox
capability. Model identity is policy data: enrollment records are explicit
controller inputs validated here, never inferred from listings or suffixes. A
model appearing in a discovery listing is not enrollment, and a free suffix is
not qualification.
"""
from __future__ import annotations

from dataclasses import dataclass
import hashlib
import os
from pathlib import Path
import re
import subprocess
import time
from typing import Any, Callable, Mapping, Sequence

from . import contracts as c

ADAPTER_UNAVAILABLE = 'ADAPTER_UNAVAILABLE'
MODEL_UNAVAILABLE = 'MODEL_UNAVAILABLE'
EFFORT_PENDING = 'EFFORT_PENDING'
QUOTA_DEFERRED = 'QUOTA_DEFERRED'
TRANSPORT_ERROR = 'TRANSPORT_ERROR'
PROTOCOL_ERROR = 'PROTOCOL_ERROR'
REVIEW_PENDING = 'REVIEW_PENDING'

KNOWN_ROLES = ('ROUTER_TRIAGE', 'MECHANICAL', 'IMPLEMENTATION', 'INVESTIGATION_REVIEW', 'ARCHITECTURE')
REASONING_LEVELS = ('NONE', 'LOW', 'DEFAULT', 'MEDIUM', 'HIGH', 'XHIGH', 'MAX')
# DEFAULT is an opaque provider setting, never evidence for a higher floor.
EFFORT_RANK = {effort: rank for rank, effort in enumerate(('DEFAULT', 'NONE', 'LOW', 'MEDIUM', 'HIGH', 'XHIGH', 'MAX'))}
OPENCODE_VERSION = '1.18.31'
OPENCODE_SHA256 = '16c960ba77421da11b53e785f359b73f328a86118b48feb4af143db5d9afb198'
CODEX_VERSION = 'codex-cli 0.158.0'

# Live-certified envelope (M5): every event carries top-level type/sessionID
# with an optional timestamp; text content lives at part.text; error events
# carry a top-level error object. Any CLI replacement or unlisted event
# version renders the adapter unavailable until reverified.
OPENCODE_EVENT_CONTRACT = {
    'format': 'opencode-json-1',
    'required_envelope': ('type', 'sessionID'),
    'terminal_error_types': ('error',),
    'final_types': ('text',),
    'confirm_fields': (),
}
TELEMETRY_FIELDS = ('task_class', 'model_id', 'provider_id', 'family', 'adapter_version',
                    'reasoning_requested', 'reasoning_sent', 'reasoning_confirmed',
                    'candidate_outcome', 'reviewer_defects', 'repair_count', 'wall_time',
                    'tool_loops', 'acceptance_outcome', 'tokens_if_available', 'cost_if_available')


class AdapterError(c.ContractError):
    def __init__(self, message: str, code: str = ADAPTER_UNAVAILABLE):
        self.code = code
        super().__init__(message)


def _require(condition: bool, message: str, code: str = ADAPTER_UNAVAILABLE) -> None:
    if not condition:
        raise AdapterError(message, code)


def _probe_env() -> dict[str, str]:
    """Minimal probe environment; passes the controller HOME through when present.

    Some hosts restrict the default home directory. Probes never invent a
    home, they only inherit the controller's own.
    """
    env = {'PATH': '/usr/bin:/bin', 'LANG': 'C'}
    if isinstance(os.environ.get('HOME'), str) and os.environ['HOME']:
        env['HOME'] = os.environ['HOME']
    return env


def _scrubbed_env(explicit: Mapping[str, str] | None) -> dict[str, str]:
    if explicit is None:
        return {'PATH': '/usr/bin:/bin', 'LANG': 'C', 'LC_ALL': 'C'}
    env = {}
    for key, value in explicit.items():
        _require(type(key) is str and type(value) is str and key and value, 'invalid adapter environment', PROTOCOL_ERROR)
        upper = key.upper()
        _require(not any(token in upper for token in ('TOKEN', 'SECRET', 'CREDENTIAL', 'API_KEY', 'PRIVATE')), 'credential-bearing adapter environment refused', PROTOCOL_ERROR)
        env[key] = value
    env.setdefault('LANG', 'C')
    return env


@dataclass(frozen=True, init=False)
class OpenCodeBinary:
    """Pinned installed OpenCode: exact path bytes, version, and event contract."""
    path: Path
    sha256: str
    version: str
    event_contract: dict

    def __init__(self, path: Path, sha256: str, version: str, event_contract: Mapping[str, Any] | None = None):
        # Resolve install shims (e.g. Homebrew Cellar links) to the exact executed file.
        path = Path(path).absolute()
        resolved = path.resolve()
        _require(resolved.is_file() and not resolved.is_symlink(), 'OpenCode executable must resolve to a pinned regular file')
        digest = hashlib.sha256(resolved.read_bytes()).hexdigest()
        _require(digest == sha256, 'OpenCode executable pin mismatch')
        try:
            observed = subprocess.run([str(path), '--version'], env=_probe_env(),
                                      capture_output=True, timeout=30, check=True).stdout.decode('utf-8').strip()
        except (OSError, subprocess.SubprocessError, UnicodeError) as exc:
            raise AdapterError('OpenCode version probe failed: ' + str(exc), ADAPTER_UNAVAILABLE) from exc
        _require(observed == version, 'OpenCode version differs from pinned adapter')
        contract = dict(event_contract) if event_contract is not None else dict(OPENCODE_EVENT_CONTRACT)
        _require(set(contract) == set(OPENCODE_EVENT_CONTRACT), 'event contract fields differ')
        object.__setattr__(self, 'path', resolved)
        object.__setattr__(self, 'sha256', digest)
        object.__setattr__(self, 'version', observed)
        object.__setattr__(self, 'event_contract', contract)

    def certification_digest(self) -> str:
        return c.canonical_digest({'path': str(self.path), 'sha256': self.sha256, 'version': self.version, 'event_contract': self.event_contract})


@dataclass(frozen=True, init=False)
class CodexBinary:
    """Pinned installed Codex CLI: exact path bytes and version string."""
    path: Path
    sha256: str
    version: str
    host_sha256: str | None

    def __init__(self, path: Path, sha256: str, version: str):
        _require(version == CODEX_VERSION, 'Codex adapter requires the live-certified CLI version')
        path = Path(path).absolute()
        resolved = path.resolve()
        _require(resolved.is_file() and not resolved.is_symlink(), 'Codex executable must resolve to a pinned regular file')
        digest = hashlib.sha256(resolved.read_bytes()).hexdigest()
        _require(digest == sha256, 'Codex executable pin mismatch')
        try:
            observed = subprocess.run([str(path), '--version'], env=_probe_env(),
                                      capture_output=True, timeout=30, check=True).stdout.decode('utf-8').strip()
        except (OSError, subprocess.SubprocessError, UnicodeError) as exc:
            raise AdapterError('Codex version probe failed: ' + str(exc), ADAPTER_UNAVAILABLE) from exc
        _require(observed == version, 'Codex version differs from pinned adapter')
        object.__setattr__(self, 'path', resolved)
        object.__setattr__(self, 'sha256', digest)
        object.__setattr__(self, 'version', observed)
        host = resolved.with_name('codex-code-mode-host')
        object.__setattr__(self, 'host_sha256', hashlib.sha256(host.read_bytes()).hexdigest()
                           if host.is_file() and not host.is_symlink() else None)


_ENROLLMENT_SEAL = object()


@dataclass(frozen=True, init=False)
class ValidatedEnrollment(Mapping):
    """Immutable host-pinned enrollment; raw catalog/model mappings confer no trust."""
    payload_json: str
    enrollment_digest: str
    task_contract_digest: str
    authority_digest: str
    reasoning_requested: str | None

    def __init__(self, record, *, task_contract_digest, authority_digest,
                 reasoning_requested=None, _seal=None):
        _require(_seal is _ENROLLMENT_SEAL, 'enrollment requires the controller pin loader')
        object.__setattr__(self, 'payload_json', c.canonical_json(record))
        object.__setattr__(self, 'enrollment_digest', c.canonical_digest(record))
        object.__setattr__(self, 'task_contract_digest', task_contract_digest)
        object.__setattr__(self, 'authority_digest', authority_digest)
        object.__setattr__(self, 'reasoning_requested', reasoning_requested)

    def __getitem__(self, key):
        if key == 'reasoning_requested' and self.reasoning_requested is not None:
            return self.reasoning_requested
        return c.load_json_strict(self.payload_json)[key]

    def __iter__(self):
        return iter((*c.load_json_strict(self.payload_json),
                     *(('reasoning_requested',) if self.reasoning_requested is not None else ())))

    def __len__(self):
        return len(c.load_json_strict(self.payload_json)) + (self.reasoning_requested is not None)

    def with_reasoning(self, effort):
        _require(effort in self['reasoning_capabilities'], 'unsupported requested reasoning effort', EFFORT_PENDING)
        return ValidatedEnrollment(c.load_json_strict(self.payload_json),
                                   task_contract_digest=self.task_contract_digest,
                                   authority_digest=self.authority_digest,
                                   reasoning_requested=effort, _seal=_ENROLLMENT_SEAL)


def validate_enrollment(record: Any) -> dict[str, Any]:
    """Validate data only; qualification/price authority requires load_enrollment."""
    if type(record) is ValidatedEnrollment:
        record = dict(record)
    _require(type(record) is dict, 'enrollment must be an explicit record')
    required = ('provider_id', 'model_id', 'family', 'allowed_roles', 'reasoning_capabilities',
                'task_class_qualification', 'adapter_certification_digest', 'budget',
                'qualification_evidence', 'availability_observation', 'pricing_observation',
                'operator_adoption_identity')
    _require(all(key in record for key in required), 'enrollment missing required fields')
    _require(type(record['provider_id']) is str and record['provider_id'] and '/' not in record['provider_id'], 'invalid provider id')
    _require(type(record['model_id']) is str and record['model_id'].startswith(record['provider_id'] + '/')
             and ' ' not in record['model_id'], 'model/provider binding differs')
    _require(type(record['family']) is str and record['family'] and record['family'] != 'unknown', 'unknown family is not independent')
    _require(type(record['allowed_roles']) is list and bool(record['allowed_roles']) and set(record['allowed_roles']) <= set(KNOWN_ROLES), 'invalid allowed roles')
    _require(type(record['reasoning_capabilities']) is list and bool(record['reasoning_capabilities']) and set(record['reasoning_capabilities']) <= set(REASONING_LEVELS), 'invalid reasoning capabilities')
    qualification = record['task_class_qualification']
    _require(type(qualification) is dict and set(qualification) <= set(record['allowed_roles'])
             and all(value is True for value in qualification.values()) and bool(qualification), 'task-class qualification must explicitly qualify roles')
    _require(type(record['adapter_certification_digest']) is str and re.fullmatch(r'[0-9a-f]{64}', record['adapter_certification_digest']) is not None, 'invalid adapter certification digest')
    _require(type(record['budget']) is dict, 'invalid enrollment budget')
    variants = record.get('variants', {})
    _require(type(variants) is dict and set(variants) <= set(record['reasoning_capabilities'])
             and all(type(v) is str and v for v in variants.values()), 'invalid effort variants')
    _require(type(record['operator_adoption_identity']) is str and record['operator_adoption_identity'], 'operator enrollment approval identity required')
    evidence = record['qualification_evidence']
    _require(type(evidence) is list and bool(evidence), 'task-class evidence required')
    for item in evidence:
        _require(type(item) is dict and set(item) == {'role', 'task_class', 'reasoning_efforts',
                 'quality_passed', 'scope_compliance', 'tool_use_correct', 'evidence_digest'}, 'invalid qualification evidence fields')
        _require(item['role'] in record['allowed_roles'] and type(item['task_class']) is str and item['task_class'], 'invalid qualification role/task class')
        _require(type(item['reasoning_efforts']) is list and bool(item['reasoning_efforts'])
                 and set(item['reasoning_efforts']) <= set(record['reasoning_capabilities']), 'unsupported qualification effort')
        _require(all(type(item[k]) is bool for k in ('quality_passed', 'scope_compliance', 'tool_use_correct')), 'invalid qualification results')
        _require(type(item['evidence_digest']) is str and re.fullmatch(r'[0-9a-f]{64}', item['evidence_digest']) is not None, 'invalid qualification evidence digest')
    available = record['availability_observation']
    _require(type(available) is dict and set(available) == {'state', 'quota_remaining', 'quota_scarce'}, 'invalid availability observation fields')
    _require(available['state'] in ('available', 'unavailable', 'quota_exhausted', 'rate_limited'), 'unknown availability')
    _require(available['quota_remaining'] is None or type(available['quota_remaining']) is int and available['quota_remaining'] >= 0, 'invalid remaining quota')
    _require(type(available['quota_scarce']) is bool, 'invalid scarce-quota observation')
    price = record['pricing_observation']
    _require(type(price) is dict and set(price) == {'kind', 'effort_cost_microusd', 'retry_cost_microusd', 'budget_pressure_microusd'}, 'invalid pricing observation fields')
    _require(price['kind'] in ('free', 'prepaid', 'metered', 'unknown'), 'unknown pricing kind')
    costs = price['effort_cost_microusd']
    _require(type(costs) is dict and set(costs) <= set(record['reasoning_capabilities'])
             and all(type(value) is int and value >= 0 for value in costs.values()), 'invalid effort cost')
    _require(all(type(price[k]) is int and price[k] >= 0 for k in ('retry_cost_microusd', 'budget_pressure_microusd')), 'invalid effective cost')
    _require(price['kind'] not in ('free', 'prepaid') or all(value == 0 for value in costs.values()), 'free/prepaid observation has marginal monetary charge')
    _require(price['kind'] != 'unknown' or not costs, 'unknown pricing cannot claim zero cost')
    return record


def load_enrollment(record: Any, *, authority: c.ValidatedReleaseAuthority,
                    task: Mapping[str, Any], expected_digest: str) -> ValidatedEnrollment:
    """Host pin must belong to the admitted immutable task, never candidate approval."""
    payload = c.validate_phase_admission(authority, task, capability='M3')
    record = validate_enrollment(record)
    _require(record['operator_adoption_identity'] == payload['build']['authorization_id'],
             'enrollment approval differs from executing operator authorization', PROTOCOL_ERROR)
    _require('reasoning_requested' not in record, 'pin base enrollment before selecting effort')
    _require(c.canonical_digest(record) == expected_digest
             and expected_digest in task.get('role_enrollment_ids', []), 'enrollment differs from operator-pinned task', PROTOCOL_ERROR)
    return ValidatedEnrollment(record, task_contract_digest=c.canonical_digest(task),
                               authority_digest=task['authority_digest'], _seal=_ENROLLMENT_SEAL)


def select_model(role: str, enrollments: Sequence[Mapping[str, Any]], *, task_budget: Mapping[str, Any],
                 required_reasoning: str = 'LOW', exclude_families: Sequence[str] = (),
                 locked_model: str | None = None, availability: Mapping[str, str] | None = None,
                 task_class: str | None = None) -> ValidatedEnrollment:
    """Deterministic cheapest sufficient model/effort from exact trusted observations."""
    _require(role in KNOWN_ROLES, 'unknown role')
    _require(required_reasoning in REASONING_LEVELS, 'unknown reasoning requirement')
    _require(type(task_budget) is dict and type(task_budget.get('cost_microusd')) is int
             and task_budget['cost_microusd'] >= 0, 'explicit task monetary budget required')
    _require(type(enrollments) is list and all(type(e) is ValidatedEnrollment for e in enrollments),
             'controller-pinned enrollment records required; listings confer nothing')
    _require(availability is None or type(availability) is dict and all(
             type(key) is str and value in ('available', 'unavailable', 'quota_exhausted', 'rate_limited')
             for key, value in availability.items()), 'invalid external availability veto', PROTOCOL_ERROR)
    _require(task_class is None or type(task_class) is str and bool(task_class), 'invalid task class', PROTOCOL_ERROR)
    task_class = task_class or role
    choices = []
    for enrolled in enrollments:
        record = validate_enrollment(enrolled)
        if locked_model is not None and record['model_id'] != locked_model:
            continue
        if role not in record['allowed_roles'] or record['task_class_qualification'].get(role) is not True:
            continue
        if record['family'] in set(exclude_families):
            continue
        observed = record['availability_observation']
        # External availability can remove eligibility, never grant it.
        if observed['state'] != 'available' or observed['quota_remaining'] == 0:
            continue
        if availability is not None and availability.get(record['model_id'], 'available') != 'available':
            continue
        price = record['pricing_observation']
        if price['kind'] == 'unknown':
            continue
        qualified_efforts = set()
        for evidence in record['qualification_evidence']:
            if evidence['role'] == role and evidence['task_class'] == task_class and all(
                    evidence[k] is True for k in ('quality_passed', 'scope_compliance', 'tool_use_correct')):
                qualified_efforts.update(evidence['reasoning_efforts'])
        efforts = [effort for effort in qualified_efforts
                   if EFFORT_RANK[effort] >= EFFORT_RANK[required_reasoning]
                   and effort in price['effort_cost_microusd']]
        if not efforts:
            continue
        prefer_high = price['kind'] == 'free' and not observed['quota_scarce']
        effort = sorted(efforts, key=lambda e: EFFORT_RANK[e], reverse=prefer_high)[0]
        effective = (price['effort_cost_microusd'][effort] + price['retry_cost_microusd']
                     + price['budget_pressure_microusd'])
        if effective > task_budget['cost_microusd']:
            continue
        ceiling = record['budget'].get('cost_microusd')
        if ceiling is not None:
            _require(type(ceiling) is int and ceiling >= 0, 'invalid enrollment cost ceiling')
            if effective > ceiling:
                continue
        choices.append((effective, record['model_id'], enrolled.with_reasoning(effort)))
    _require(bool(choices), 'no qualified available enrolled model/effort within task budget; no substitution', MODEL_UNAVAILABLE)
    return min(choices, key=lambda choice: choice[:2])[2]


@dataclass(frozen=True)
class StreamLimits:
    max_bytes: int
    max_events: int
    max_elapsed_seconds: int

    def validate(self) -> None:
        _require(type(self.max_bytes) is int and 1 <= self.max_bytes <= 64 << 20, 'invalid stream byte bound')
        _require(type(self.max_events) is int and 1 <= self.max_events <= 4096, 'invalid stream event bound')
        _require(type(self.max_elapsed_seconds) is int and 1 <= self.max_elapsed_seconds <= 3600, 'invalid stream elapsed bound')


@dataclass(frozen=True)
class ParsedStream:
    events: tuple
    session_id: str
    error: dict | None
    final_payload: Any


def parse_event_stream(data: bytes, *, limits: StreamLimits, event_contract: Mapping[str, Any]) -> ParsedStream:
    """Strict pinned-envelope parsing; earlier text/tools/reasoning never become final JSON."""
    limits.validate()
    _require(type(data) is bytes and len(data) <= limits.max_bytes, 'event stream bound exceeded', PROTOCOL_ERROR)
    try:
        text = data.decode('utf-8')
    except UnicodeError as exc:
        raise AdapterError('event stream is not valid UTF-8', PROTOCOL_ERROR) from exc
    _require(not text or text.endswith('\n'), 'truncated terminal stream', PROTOCOL_ERROR)
    _require(set(event_contract) == set(OPENCODE_EVENT_CONTRACT), 'event contract fields differ', PROTOCOL_ERROR)
    final_types = tuple(event_contract['final_types'])
    terminal_errors = tuple(event_contract['terminal_error_types'])
    events: list[dict] = []
    session: str | None = None
    for raw in text.splitlines():
        if not raw.strip():
            continue
        _require(len(events) < limits.max_events, 'event count bound exceeded', PROTOCOL_ERROR)
        try:
            event = c.load_json_strict(raw)
        except c.ContractError as exc:
            raise AdapterError('malformed event JSON: ' + str(exc), PROTOCOL_ERROR) from exc
        _require(type(event) is dict and type(event.get('type')) is str and event['type']
                 and type(event.get('sessionID')) is str and event['sessionID'], 'unknown event envelope', PROTOCOL_ERROR)
        if 'timestamp' in event:
            _require(type(event['timestamp']) is int, 'invalid event timestamp', PROTOCOL_ERROR)
        if session is None:
            session = event['sessionID']
        _require(event['sessionID'] == session, 'multiple sessions in one stream', PROTOCOL_ERROR)
        events.append(event)
    _require(session is not None, 'empty event stream', PROTOCOL_ERROR)
    error = next((event for event in events if event['type'] in terminal_errors), None)
    finals = []
    for event in events:
        if event['type'] in final_types:
            part = event.get('part')
            _require(type(part) is dict and type(part.get('text')) is str and part['text'],
                     'final text event carries no content', PROTOCOL_ERROR)
            finals.append(part['text'])
    _require(len(finals) <= 1, 'ambiguous final payload', PROTOCOL_ERROR)
    return ParsedStream(events=tuple(events), session_id=session, error=error, final_payload=finals[0] if finals else None)


@dataclass(frozen=True)
class RunObservation:
    argv: tuple
    argv_digest: str
    exit_code: int | None
    timed_out: bool
    session_id: str | None
    events_digest: str | None
    final_payload: Any
    error: dict | None
    elapsed_seconds: float
    model_id: str
    provider_id: str
    family: str
    reasoning_requested: str
    reasoning_sent: str | None
    reasoning_confirmed: str
    transcript_digest: str


class OpenCodeAdapter:
    """One pinned role transport. The model ID reaches the actual process argv."""
    def __init__(self, binary: OpenCodeBinary, *, role: str, enrollment: Mapping[str, Any],
                 workdir: Path, limits: StreamLimits, env: Mapping[str, str] | None = None,
                 require_confirmed_effort: bool = False):
        _require(type(binary) is OpenCodeBinary, 'pinned OpenCode binary required')
        _require(role in KNOWN_ROLES, 'unknown role')
        _require(type(enrollment) is ValidatedEnrollment, 'controller-pinned enrollment required')
        self.enrollment = validate_enrollment(enrollment)
        _require(self.enrollment['adapter_certification_digest'] == binary.certification_digest(),
                 'enrollment certification differs from actual pinned OpenCode runtime')
        _require(role in self.enrollment['allowed_roles'] and self.enrollment['task_class_qualification'].get(role) is True,
                 'enrollment does not qualify this role')
        _require(self.enrollment.get('reasoning_requested') in self.enrollment['reasoning_capabilities'],
                 'adapter requires the selected requested effort')
        self.requested = self.enrollment['reasoning_requested']
        variant = self.enrollment.get('variants', {}).get(self.requested)
        self.sent = variant
        if self.requested == 'HIGH' and role == 'INVESTIGATION_REVIEW' and variant is None:
            self.confirmed = 'DEFAULT_PROVIDER'
        elif variant is None:
            self.confirmed = 'DEFAULT_PROVIDER'
        else:
            self.confirmed = 'UNCONFIRMED'
        if require_confirmed_effort and self.confirmed != self.sent:
            raise AdapterError('task requires confirmed ' + self.requested + ' effort the adapter cannot supply; stage stays pending', EFFORT_PENDING)
        self.binary, self.role, self.limits = binary, role, limits
        self.workdir = Path(workdir)
        _require(self.workdir.is_absolute(), 'adapter workdir must be absolute')
        self.env = _scrubbed_env(env)

    def build_argv(self, message_parts: Sequence[str], *, agent: str | None = None, session: str | None = None) -> list[str]:
        _require(type(message_parts) in (list, tuple) and bool(message_parts) and all(type(p) is str and p and len(p) <= 65536 for p in message_parts), 'bounded explicit message required', PROTOCOL_ERROR)
        argv = [str(self.binary.path), 'run', '--format', 'json', '--model', self.enrollment['model_id']]
        if self.sent is not None:
            argv.extend(['--variant', self.sent])
        if agent is not None:
            _require(type(agent) is str and re.fullmatch(r'[A-Za-z0-9_.-]{1,64}', agent) is not None, 'invalid agent name', PROTOCOL_ERROR)
            argv.extend(['--agent', agent])
        if session is not None:
            _require(type(session) is str and re.fullmatch(r'[A-Za-z0-9_.-]{1,128}', session) is not None, 'invalid session identity', PROTOCOL_ERROR)
            argv.extend(['--session', session])
        argv.extend(['--dir', str(self.workdir), *message_parts])
        return argv

    def run(self, message_parts: Sequence[str], *, agent: str | None = None, session: str | None = None,
            timeout_seconds: int = 600) -> RunObservation:
        """Launch the real pinned process; observations are controller facts, model text is not."""
        _require(type(timeout_seconds) is int and 1 <= timeout_seconds <= 3600, 'invalid run timeout')
        argv = self.build_argv(message_parts, agent=agent, session=session)
        started = time.monotonic()
        try:
            proc = subprocess.Popen(argv, env=self.env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, start_new_session=True)
        except OSError as exc:
            raise AdapterError('model process launch failed: ' + str(exc), TRANSPORT_ERROR) from exc
        try:
            output, _ = proc.communicate(timeout=timeout_seconds)
            timed_out = False
        except subprocess.TimeoutExpired:
            try:
                os.killpg(proc.pid, 9)
            except (OSError, ProcessLookupError):
                pass
            output, _ = proc.communicate()
            timed_out = True
        elapsed = time.monotonic() - started
        stream: ParsedStream | None = None
        error: dict | None = None
        final: Any = None
        session_id: str | None = None
        if output:
            stream = parse_event_stream(output, limits=self.limits, event_contract=self.binary.event_contract)
            error, final, session_id = stream.error, stream.final_payload, stream.session_id
        provider, _, _ = self.enrollment['model_id'].partition('/')
        confirmed = self.confirmed
        if self.sent is not None and final is not None:
            for field in self.binary.event_contract['confirm_fields']:
                node: Any = final
                for part in field.split('.'):
                    node = node.get(part) if type(node) is dict else None
                if node == self.sent:
                    confirmed = self.sent
                    break
        return RunObservation(argv=tuple(argv), argv_digest=c.canonical_digest(argv), exit_code=proc.returncode,
                              timed_out=timed_out, session_id=session_id,
                              events_digest=c.canonical_digest([dict(e) for e in stream.events]) if stream else None,
                              final_payload=final, error=error, elapsed_seconds=elapsed,
                              model_id=self.enrollment['model_id'], provider_id=provider,
                              family=self.enrollment['family'], reasoning_requested=self.requested,
                              reasoning_sent=self.sent, reasoning_confirmed=confirmed,
                              transcript_digest=c.canonical_digest(output.decode('utf-8', 'replace')))


def parse_review_report(payload: Any, *, task: Mapping[str, Any], candidate_sha: str, schemas: Mapping[str, Any],
                        reviewer_family: str, implementation_family: str, flag_ids: Sequence[str] = ()) -> dict[str, Any]:
    """Strict reviewer report bound to the exact task/candidate; prose or stale data refuses."""
    if type(payload) is str:
        try:
            payload = c.load_json_strict(payload)
        except c.ContractError as exc:
            raise AdapterError('reviewer response is not strict JSON: ' + str(exc), PROTOCOL_ERROR) from exc
    _require(type(payload) is dict, 'reviewer response must be a strict report object', PROTOCOL_ERROR)
    binding = {'task_id': task['task_id'], 'candidate_sha': candidate_sha,
               'task_contract_digest': c.canonical_digest(task), 'coverage': list(payload.get('coverage', []))}
    try:
        report = c.validate_record(payload, 'review_report', schemas, context=binding)
    except c.ContractError as exc:
        raise AdapterError('reviewer report fails independent binding: ' + str(exc), PROTOCOL_ERROR) from exc
    _require(report['task_id'] == task['task_id'] and report['task_contract_digest'] == c.canonical_digest(task)
             and report['candidate_sha'] == candidate_sha, 'reviewer report echoes wrong/stale task/candidate', PROTOCOL_ERROR)
    _require(type(reviewer_family) is str and reviewer_family and reviewer_family != 'unknown'
             and reviewer_family != implementation_family, 'same or unknown reviewer family leaves REVIEW_PENDING', REVIEW_PENDING)
    _require(set(report['coverage']) >= set(task['required_check_ids']), 'reviewer coverage omits required checks', PROTOCOL_ERROR)
    current_flags = set(flag_ids)
    for disposition in report['quality_flag_dispositions']:
        _require(disposition['id'] in current_flags, 'reviewer disposition cites a stale/unknown flag', PROTOCOL_ERROR)
    if report['verdict'] == 'PASS':
        _require(not report['unresolved_questions']
                 and not any(f['severity'] == 'BLOCKING' for f in report['findings'])
                 and all(d['disposition'] == 'NOT_LOWERING' for d in report['quality_flag_dispositions']),
                 'review PASS contradicts unresolved quality obligations', PROTOCOL_ERROR)
    return report


@dataclass(frozen=True)
class EscalationDeferred:
    packet_digest: str
    availability: str
    packet: dict


# Pinned 0.158 surfaces verified by CP46; absent flags require recertification.
CODEX_DISABLED_FEATURES = ('shell_tool', 'unified_exec', 'multi_agent', 'apps', 'hooks',
                           'plugins', 'remote_plugin', 'browser_use', 'browser_use_external',
                           'computer_use', 'image_generation', 'view_image', 'goals',
                           'code_mode', 'code_mode_only', 'workspace_dependencies', 'skill_search',
                           'tool_suggest', 'shell_snapshot', 'sleep_tool', 'memories')


def architecture_decision_schema():
    """Exact structural output contract, including closed nested cited-fact objects."""
    strings = {'type': 'array', 'items': {'type': 'string'}}
    properties = {
        'schema_version': {'type': 'integer', 'enum': [1]},
        'packet_digest': {'type': 'string'},
        'disposition': {'type': 'string', 'enum': list(DECISION_DISPOSITIONS)},
        'decision': {'type': 'string'},
        'constraints': strings, 'required_verification': strings, 'stop_conditions': strings,
        'cited_facts': {'type': 'array', 'items': {'type': 'object',
            'properties': {'fact': {'type': 'string'}, 'classification': {'type': 'string',
                'enum': ['PROVEN', 'SUPPORTED', 'PLAUSIBLE', 'UNKNOWN']}},
            'required': ['fact', 'classification'], 'additionalProperties': False}},
    }
    return {'type': 'object', 'properties': properties,
            'required': list(properties), 'additionalProperties': False}


def validate_codex_readonly_stream(data: bytes) -> None:
    """Any model tool item, unknown event, or incomplete turn refuses certification."""
    _require(type(data) is bytes and len(data) <= 4 << 20 and data.endswith(b'\n'),
             'invalid Codex event stream', PROTOCOL_ERROR)
    completed = False
    for line in data.splitlines():
        try:
            event = c.load_json_strict(line.decode('utf-8'))
        except (c.ContractError, UnicodeError) as exc:
            raise AdapterError('invalid Codex event JSON', PROTOCOL_ERROR) from exc
        _require(type(event) is dict and event.get('type') in (
                 'thread.started', 'turn.started', 'item.started', 'item.updated', 'item.completed', 'turn.completed'),
                 'unknown/error Codex event', PROTOCOL_ERROR)
        if event['type'].startswith('item.'):
            item = event.get('item')
            _require(type(item) is dict and item.get('type') in ('agent_message', 'reasoning'),
                     'Codex read-only decision attempted a tool or nested launch', PROTOCOL_ERROR)
        if event['type'] == 'turn.completed':
            _require(not completed, 'multiple Codex terminal turns', PROTOCOL_ERROR)
            completed = True
    _require(completed, 'Codex decision stream has no terminal completion', PROTOCOL_ERROR)


class CodexAdapter:
    """Read-only architecture transport. Quota defers with identical facts; anything else is a tool error."""
    def __init__(self, binary: CodexBinary, *, model_id: str, workdir: Path, decision_schema_path: Path,
                 quota_signatures: Sequence[Mapping[str, Any]] = (), env: Mapping[str, str] | None = None):
        _require(type(binary) is CodexBinary, 'pinned Codex binary required')
        _require(type(model_id) is str and re.fullmatch(r'[A-Za-z0-9][A-Za-z0-9_.:/-]{0,127}', model_id) is not None, 'invalid Codex model id')
        self.workdir = Path(workdir)
        _require(self.workdir.is_absolute(), 'adapter workdir must be absolute')
        schema_path = Path(decision_schema_path)
        _require(schema_path.is_absolute() and schema_path.is_file() and not schema_path.is_symlink(), 'strict decision schema file required')
        _require(c.load_json_strict(schema_path.read_text()) == architecture_decision_schema(),
                 'decision schema differs from strict architecture decision contract', PROTOCOL_ERROR)
        for signature in quota_signatures:
            _require(type(signature) is dict and type(signature.get('version')) is str
                     and type(signature.get('stderr_contains')) is str and signature['stderr_contains'], 'invalid quota signature')
        self.binary, self.model_id = binary, model_id
        self.decision_schema_path = schema_path
        self.quota_signatures = list(quota_signatures)
        self.env = _scrubbed_env(env)

    def build_argv(self, prompt: str, last_message_path: Path) -> list[str]:
        _require(type(prompt) is str and prompt and len(prompt) <= 65536, 'bounded explicit architecture prompt required', PROTOCOL_ERROR)
        _require(isinstance(last_message_path, Path) and last_message_path.is_absolute(), 'controller-owned last-message path required', PROTOCOL_ERROR)
        argv = [str(self.binary.path), 'exec', '--json', '--sandbox', 'read-only',
                '--ignore-user-config', '--ignore-rules', '--ephemeral', '--skip-git-repo-check',
                '-c', 'web_search="disabled"', '-c', 'model_reasoning_effort="low"',
                '-c', 'features.code_mode_host=true']
        for feature in CODEX_DISABLED_FEATURES:
            argv.extend(['--disable', feature])
        return [*argv, '-C', str(self.workdir), '--output-schema', str(self.decision_schema_path),
                '-o', str(last_message_path), '-m', self.model_id, prompt]


    def run_architecture(self, prompt: str, *, packet: Mapping[str, Any], output_dir: Path,
                         timeout_seconds: int = 600) -> tuple[dict, EscalationDeferred | None]:
        """Returns (decision, None) or raises AdapterError; quota raises QUOTA_DEFERRED preserving packet facts.

        The strict decision is the controller-owned last-message file, never
        stdout noise. Malformed decisions, auth errors, and tool errors refuse
        as transport/protocol failures; only exact pinned versioned quota
        evidence defers.
        """
        _require(type(packet) is dict, 'frozen escalation packet required')
        output_dir = Path(output_dir)
        _require(output_dir.is_absolute() and output_dir.is_dir() and not output_dir.is_symlink(), 'controller-owned output directory required')
        last_message = output_dir / 'codex-last-message.json'
        _require(not last_message.exists(), 'decision artifact must not pre-exist')
        _require(hashlib.sha256(self.binary.path.read_bytes()).hexdigest() == self.binary.sha256,
                 'Codex runtime changed after certification')
        if self.binary.host_sha256 is not None:
            _require(hashlib.sha256(self.binary.path.with_name('codex-code-mode-host').read_bytes()).hexdigest()
                     == self.binary.host_sha256, 'Codex runtime host changed after certification')
        argv = self.build_argv(prompt, last_message)
        try:
            proc = subprocess.run(argv, env=self.env, capture_output=True, timeout=timeout_seconds)
        except subprocess.TimeoutExpired as exc:
            raise AdapterError('architecture transport timed out', TRANSPORT_ERROR) from exc
        except OSError as exc:
            raise AdapterError('architecture transport launch failed: ' + str(exc), TRANSPORT_ERROR) from exc
        for name, data in (('codex-events.jsonl', proc.stdout), ('codex-stderr.txt', proc.stderr)):
            _require(len(data) <= 4 << 20, 'Codex transcript bound exceeded', PROTOCOL_ERROR)
            target = output_dir / name
            _require(not target.exists(), 'Codex evidence artifact must not pre-exist', PROTOCOL_ERROR)
            with target.open('xb') as stream:
                stream.write(data)
        if proc.returncode != 0:
            stderr = proc.stderr.decode('utf-8', 'replace')
            for signature in self.quota_signatures:
                if signature.get('version') == self.binary.version and signature['stderr_contains'] in stderr:
                    raise AdapterError('Codex quota exhausted; packet preserved', QUOTA_DEFERRED)
            raise AdapterError('Codex transport error; not a decision and not quota', TRANSPORT_ERROR)
        validate_codex_readonly_stream(proc.stdout)
        try:
            raw = last_message.read_bytes()
        except OSError as exc:
            raise AdapterError('Codex produced no decision artifact', PROTOCOL_ERROR) from exc
        _require(len(raw) <= 1 << 20, 'decision artifact bound exceeded', PROTOCOL_ERROR)
        try:
            decision = c.load_json_strict(raw.decode('utf-8'))
        except (c.ContractError, UnicodeError) as exc:
            raise AdapterError('Codex decision is not strict JSON: ' + str(exc), PROTOCOL_ERROR) from exc
        return validate_architecture_decision(decision, packet), None

    def defer_quota(self, packet: Mapping[str, Any]) -> EscalationDeferred:
        """Deterministic quota deferral preserving identical packet/attempt facts."""
        _require(type(packet) is dict, 'frozen escalation packet required')
        digest = c.canonical_digest({k: v for k, v in packet.items() if k != 'packet_digest'})
        _require(packet.get('packet_digest', digest) == digest, 'quota deferral changed packet facts')
        return EscalationDeferred(packet_digest=digest, availability='quota', packet=dict(packet))


DECISION_DISPOSITIONS = ('WITHIN_CONTRACT', 'AMENDMENT_REQUIRED', 'NEEDS_DISCRIMINATING_EVIDENCE', 'DEFER')


def validate_architecture_decision(decision: Any, packet: Mapping[str, Any]) -> dict[str, Any]:
    """Strict architecture decision bound to the exact frozen packet; prose refuses."""
    _require(type(decision) is dict, 'architecture decision must be a strict object', PROTOCOL_ERROR)
    required = ('schema_version', 'packet_digest', 'disposition', 'decision', 'constraints',
                'required_verification', 'stop_conditions', 'cited_facts')
    _require(set(decision) == set(required), 'architecture decision fields differ', PROTOCOL_ERROR)
    _require(decision['schema_version'] == 1 and type(decision['schema_version']) is int, 'unsupported decision version', PROTOCOL_ERROR)
    _require(decision['packet_digest'] == c.canonical_digest({k: v for k, v in packet.items() if k != 'packet_digest'}),
             'decision binds a different packet', PROTOCOL_ERROR)
    _require(decision['disposition'] in DECISION_DISPOSITIONS, 'unsupported decision disposition', PROTOCOL_ERROR)
    for key in ('decision',):
        _require(type(decision[key]) is str and decision[key], 'decision requires ' + key, PROTOCOL_ERROR)
    for key in ('constraints', 'required_verification', 'stop_conditions'):
        _require(type(decision[key]) is list and all(type(item) is str and item for item in decision[key]), 'decision requires ' + key, PROTOCOL_ERROR)
    _require(type(decision['cited_facts']) is list and bool(decision['cited_facts']), 'decision requires cited facts', PROTOCOL_ERROR)
    for fact in decision['cited_facts']:
        _require(type(fact) is dict and type(fact.get('fact')) is str and fact['fact']
                 and fact.get('classification') in ('PROVEN', 'SUPPORTED', 'PLAUSIBLE', 'UNKNOWN'),
                 'decision cited fact invalid', PROTOCOL_ERROR)
    return decision

def classify_disconnect(summary: Mapping[str, Any]) -> dict[str, Any]:
    """A historical disconnect alone is UNKNOWN/inconclusive, never infrastructure PASS."""
    _require(type(summary) is dict and summary.get('kind') == 'disconnect', 'only disconnect summaries classify here')
    return {'classification': 'UNKNOWN', 'requires': ['competing_hypotheses', 'discriminating_observation'],
            'never': ['infrastructure_PASS', 'automatic_rerun_verdict']}


def discover_models(binary: OpenCodeBinary) -> list[str]:
    """Raw provider listing for operator visibility only; never enrollment."""
    _require(type(binary) is OpenCodeBinary, 'pinned OpenCode binary required')
    try:
        output = subprocess.run([str(binary.path), 'models'], env=_probe_env(),
                                capture_output=True, timeout=60, check=True).stdout.decode('utf-8')
    except (OSError, subprocess.SubprocessError, UnicodeError) as exc:
        raise AdapterError('model discovery failed: ' + str(exc), TRANSPORT_ERROR) from exc
    names = [line.strip() for line in output.splitlines() if line.strip()]
    _require(all(' ' not in name for name in names), 'malformed discovery listing')
    return names


def append_telemetry(path: Path, record: Mapping[str, Any]) -> None:
    """Append-only factual telemetry; unknown cost/tokens stay null, never inferred."""
    _require(set(record) == set(TELEMETRY_FIELDS), 'telemetry record fields differ')
    _require((record['tokens_if_available'] is None or type(record['tokens_if_available']) is int)
             and (record['cost_if_available'] is None or type(record['cost_if_available']) in (int, float)),
             'telemetry cost/tokens must be numeric or null')
    path = Path(path)
    data = (c.canonical_json(dict(record)) + '\n').encode('utf-8')
    _require(len(data) <= 1 << 20, 'telemetry record bound exceeded')
    fd = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_APPEND, 0o600)
    try:
        os.write(fd, data)
        os.fsync(fd)
    finally:
        os.close(fd)


def adapter_version() -> str:
    return 'm3-adapters-1'


@dataclass(frozen=True)
class WorkerStageResult:
    container_id: str
    exit_code: int | None
    timed_out: bool
    transcript: bytes
    transcript_stderr: bytes
    truncated: bool
    effective_digest: str
    elapsed_seconds: float


def build_worker_view(candidate: Any, storage_root: Path, task_id: str, stage_id: str,
                      role: str, destination: Path | None = None) -> Path:
    """M3 launch-view copy with controller mask targets; mirrors M1 view semantics.

    Copies the candidate into controller-private bounded storage, creates empty
    mask targets for absent frozen config names so Docker cannot create them
    inside the input, and widens modes only for the IMPLEMENTATION role. The
    input candidate is never modified.
    """
    from . import sandbox as b
    _require(hasattr(candidate, 'root') and hasattr(candidate, 'verify'), 'owned candidate required')
    _require(role in ('IMPLEMENTATION', 'INVESTIGATION_REVIEW'), 'unsupported worker role')
    storage_root = b._safe_path(Path(storage_root))
    _require(Path(candidate.root).parent == storage_root, 'candidate must be a direct child of bounded launch storage')
    launch_root = storage_root / '.or-v2-launch-views'
    if not launch_root.exists() and not launch_root.is_symlink():
        launch_root.mkdir(mode=0o700)
    item = launch_root.lstat()
    _require(item.st_uid == os.geteuid() and (item.st_mode & 0o777) == 0o700, 'launch-view storage is not controller-private')
    import shutil
    import stat as stat_module
    import tempfile
    if destination is None:
        stage_root = Path(tempfile.mkdtemp(prefix=task_id + '-' + stage_id + '-', dir=launch_root))
        view = stage_root / 'candidate'
    else:
        view = b._safe_path(Path(destination))
        _require(view.parent.parent == launch_root and not view.parent.exists(), 'reserved launch path reused')
        view.parent.mkdir(mode=0o700)
    shutil.copytree(candidate.root, view, copy_function=shutil.copy2, symlinks=True)
    for name in ('opencode.json', 'opencode.jsonc', '.opencode'):
        path = view / name
        if not path.exists():
            if name == '.opencode':
                path.mkdir()
            else:
                path.touch()
    b.inspect_candidate(view)
    source_device = Path(candidate.root).stat().st_dev
    for directory, dirs, files in os.walk(view, followlinks=False):
        for name in dirs + files:
            path = Path(directory) / name
            entry = path.lstat()
            _require(entry.st_dev == source_device and (stat_module.S_ISDIR(entry.st_mode) or stat_module.S_ISREG(entry.st_mode))
                     and (not stat_module.S_ISREG(entry.st_mode) or entry.st_nlink == 1),
                     'copied launch view contains unsafe filesystem entry')
    for directory, _, files in os.walk(view, topdown=False, followlinks=False):
        base = Path(directory)
        base.chmod((base.stat().st_mode & 0o777) | (0o777 if role == 'IMPLEMENTATION' else 0o555))
        for name in files:
            path = base / name
            path.chmod((path.stat().st_mode & 0o777) | (0o666 if role == 'IMPLEMENTATION' else 0o444))
    return view


def model_mounts(*, overlays, role, candidate, stage_root, credential_dir, provider_id):
    """Shared live profile: bounded writable runtime, immutable policy/credential shadows."""
    from . import sandbox as b
    import json
    validated = b._validated_role_overlays(overlays, role, candidate)
    stage_root = b._safe_path(Path(stage_root))
    _require(stage_root.is_dir() and stage_root.stat().st_uid == os.geteuid()
             and stage_root.stat().st_mode & 0o777 == 0o700, 'private model stage root required')
    mask = stage_root / 'config-mask'
    mask.mkdir(mode=0o700)
    project = mask / '.opencode'
    project.mkdir(mode=0o755)
    (project / '.gitignore').write_bytes(b'')
    (project / '.gitignore').chmod(0o444)
    project.chmod(0o555)
    mask.chmod(0o555)
    home = stage_root / 'runtime-home'
    home.mkdir(mode=0o777)
    home.chmod(0o777)
    for sub in ('.local/share/opencode', '.config/opencode'):
        node = home
        for part in Path(sub).parts:
            node = node / part
            node.mkdir(mode=0o777, exist_ok=True)
            node.chmod(0o777)
    frozen_home = Path(overlays['home'])
    mounts = [(source, target) for source, target in validated
              if target not in ('/candidate/.opencode', '/worker-home')]
    mounts.extend([(project, '/candidate/.opencode'), (home, '/worker-home', True),
                   (frozen_home / 'opencode.json', '/worker-home/opencode.json'),
                   (frozen_home / '.config/opencode/opencode.json',
                    '/worker-home/.config/opencode/opencode.json')])
    if credential_dir is not None:
        credential = b._safe_path(Path(credential_dir)) / 'opencode/auth.json'
        item = credential.lstat()
        import stat
        _require(stat.S_ISREG(item.st_mode) and item.st_nlink == 1 and item.st_size <= 65536,
                 'regular bounded inference credential required')
        raw = json.loads(credential.read_text())
        _require(set(raw) == {provider_id} and set(raw[provider_id]) == {'type', 'key'}
                 and raw[provider_id]['type'] == 'api' and type(raw[provider_id]['key']) is str
                 and bool(raw[provider_id]['key']), 'credential must match exactly the selected provider')
        mounts.append((credential, '/worker-home/.local/share/opencode/auth.json'))
    return mounts


def erase_runtime_home(box, stage_root, image):
    """Erase only the controller-created disposable model runtime after stage death."""
    from . import sandbox as b
    stage_root = b._safe_path(Path(stage_root))
    home = stage_root / 'runtime-home'
    _require(stage_root.stat().st_uid == os.geteuid() and stage_root.stat().st_mode & 0o777 == 0o700
             and home.is_dir() and not home.is_symlink(), 'private runtime cleanup locator differs')
    if not box.fixture_only:
        box.docker(['run', '--rm', '--pull=never', '--network=none', '--read-only',
                    '--user=10001:10001', '--cap-drop=ALL', '--security-opt=no-new-privileges:true',
                    '--pids-limit=32', '--memory=268435456', '--log-driver=none',
                    '--mount=type=bind,src=' + str(home) + ',dst=/runtime',
                    '--entrypoint=/bin/sh', image, '-ec',
                    'find /runtime -mindepth 1 -maxdepth 1 -exec rm -rf -- {} +'])


def _bounded_capture(proc, *, stdout_limit, stderr_limit, timeout_seconds, kill):
    """Drain both pipes with fixed memory bounds; overflow kills the whole stage."""
    import selectors
    selector = selectors.DefaultSelector()
    buffers = {proc.stdout: bytearray(), proc.stderr: bytearray()}
    bounds = {proc.stdout: stdout_limit, proc.stderr: stderr_limit}
    for pipe in buffers:
        selector.register(pipe, selectors.EVENT_READ)
    deadline = time.monotonic() + timeout_seconds
    timed_out = truncated = False
    killed = False
    try:
        while selector.get_map():
            if time.monotonic() >= deadline and not killed:
                timed_out = True
                kill()
                killed = True
                deadline = time.monotonic() + 5
            if killed and time.monotonic() >= deadline:
                break
            for key, _ in selector.select(0.2):
                chunk = os.read(key.fileobj.fileno(), 65536)
                if not chunk:
                    selector.unregister(key.fileobj)
                    continue
                buf = buffers[key.fileobj]
                room = bounds[key.fileobj] - len(buf)
                buf.extend(chunk[:max(0, room)])
                if len(chunk) > room and not killed:
                    truncated = True
                    kill()
                    killed = True
                    deadline = time.monotonic() + 5
        proc.wait(timeout=5)
    finally:
        selector.close()
        for pipe in buffers:
            pipe.close()
    return bytes(buffers[proc.stdout]), bytes(buffers[proc.stderr]), timed_out, truncated


def launch_model_stage(*, box: Any, candidate: Any, view: Path, role: str, container_binary: str,
                       message_parts: Sequence[str], agent: str | None, adapter: OpenCodeAdapter,
                       image: str, network: str, limits: Any, overlay_mounts: Sequence[tuple],
                       labels: Mapping[str, str], name: str | None = None,
                       timeout_seconds: int = 600, workdir: str = '/candidate', on_created=None) -> WorkerStageResult:
    """Run the pinned model command in an isolated container; return observations only.

    No receipt, seal, or authority is minted here. The orchestrator binds the
    store launch record, reconciles through the sandbox, and parses results.
    `network` is an explicit capability: 'none' always, or 'bridge' for model
    roles that require inference egress (IMPLEMENTATION worker and
    INVESTIGATION_REVIEW reviewer), recorded by the caller. Reviewer egress
    never grants shell, candidate writes, or credentials beyond inference.
    """
    from . import sandbox as b
    _require(hasattr(box, 'docker') and hasattr(box, 'verify_effective'), 'trusted container sandbox required')
    _require(role in ('IMPLEMENTATION', 'INVESTIGATION_REVIEW'), 'unsupported worker role')
    _require(network == 'none' or (network == 'bridge' and role in ('IMPLEMENTATION', 'INVESTIGATION_REVIEW')), 'network is an explicit model-inference capability')
    _require(type(container_binary) is str and container_binary.startswith('/') and ' ' not in container_binary, 'invalid container model binary path')
    _require(type(timeout_seconds) is int and 1 <= timeout_seconds <= 3600, 'invalid stage timeout')
    _require(type(name) is None or (type(name) is str and re.fullmatch(r'[A-Za-z0-9_.-]{1,128}', name) is not None), 'invalid container name')
    _require(type(labels) is dict and labels and all(type(k) is str and type(v) is str for k, v in labels.items()), 'stage ownership labels required')
    limits.validate()
    if not box.fixture_only:
        b._bounded_candidate_filesystem(candidate.root if role == 'IMPLEMENTATION' else candidate.root.parent, limits.candidate_bytes)
        observed = box.docker(['run', '--rm', '--pull=never', '--network=none', '--read-only',
                               '--cap-drop=ALL', '--security-opt=no-new-privileges:true',
                               '--entrypoint=/usr/bin/sha256sum', image, container_binary]).split()[0]
        _require(observed == adapter.binary.sha256, 'image model executable differs from certified binary')
    file_bytes = max(limits.scratch_bytes, limits.output_bytes)
    view = b._safe_path(Path(view))
    inner = [container_binary, 'run', '--format', 'json', '--model', adapter.enrollment['model_id']]
    if adapter.sent is not None:
        inner.extend(['--variant', adapter.sent])
    if agent is not None:
        _require(type(agent) is str and re.fullmatch(r'[A-Za-z0-9_.-]{1,64}', agent) is not None, 'invalid agent name', PROTOCOL_ERROR)
        inner.extend(['--agent', agent])
    inner.extend(['--dir', '/candidate'])
    _require(type(message_parts) in (list, tuple) and bool(message_parts) and all(type(p) is str and p and len(p) <= 65536 for p in message_parts), 'bounded explicit message required', PROTOCOL_ERROR)
    inner.extend(list(message_parts))
    bounded = ('/usr/bin/timeout', '--signal=KILL', '--kill-after=1', str(timeout_seconds), *inner)
    mounts = [(str(view), '/candidate', role == 'IMPLEMENTATION')]
    for entry in overlay_mounts:
        _require(type(entry) is tuple and len(entry) in (2, 3), 'invalid overlay mount entry', PROTOCOL_ERROR)
        source, destination = str(entry[0]), entry[1]
        writable = bool(entry[2]) if len(entry) == 3 else False
        _require(type(entry[1]) is str and destination.startswith('/'),
                 'invalid overlay mount paths', PROTOCOL_ERROR)
        _require(not writable or (destination == '/worker-home'
                 and Path(source).parent == view.parent and Path(source).name == 'runtime-home'),
                 'only stage-private runtime home may be writable', PROTOCOL_ERROR)
        mounts.append((source, destination, writable))
    expected = dict(image=image, mounts=mounts, limits=limits, file_bytes=file_bytes, network=network, command=bounded,
                    environment=['HOME=/worker-home', 'TMPDIR=/scratch',
                                 'XDG_CONFIG_HOME=/worker-home/.config', 'XDG_DATA_HOME=/worker-home/.local/share',
                                 'XDG_STATE_HOME=/worker-home/.local/state', 'OPENCODE_CONFIG=/worker-home/opencode.json',
                                 'GIT_CONFIG_NOSYSTEM=1', 'GIT_CONFIG_GLOBAL=/dev/null'])
    args = ['create', '--pull=never', '--read-only', '--user=10001:10001', '--cap-drop=ALL',
            '--security-opt=no-new-privileges:true', '--network=' + network, '--ipc=none', '--cgroupns=private',
            '--pids-limit=' + str(limits.pids), '--cpus=' + str(limits.cpu),
            '--memory=' + str(limits.memory_bytes), '--memory-swap=' + str(limits.memory_bytes),
            '--log-driver=none', '--restart=no', '--stop-timeout=1', '--init',
            '--ulimit=nofile=256:256', '--ulimit=fsize=' + str(file_bytes) + ':' + str(file_bytes),
            '--mount=type=bind,src=' + str(view) + ',dst=/candidate' + ('' if role == 'IMPLEMENTATION' else ',readonly'),
            '--tmpfs=/scratch:rw,nosuid,nodev,noexec,size=' + str(limits.scratch_bytes) + ',mode=1777',
            '--workdir=' + workdir]
    if name is not None:
        args.append('--name=' + name)
    for source, destination, writable in mounts[1:]:
        args.append('--mount=type=bind,src=' + str(source) + ',dst=' + destination + ('' if writable else ',readonly'))
    for key, value in labels.items():
        args.extend(['--label', key + '=' + value])
    for variable in expected['environment']:
        args.append('--env=' + variable)
    args.extend(['--entrypoint=' + bounded[0], image, *bounded[1:]])
    container_id = box.docker(args).strip()
    _require(re.fullmatch(r'[0-9a-f]{64}', container_id) is not None, 'invalid worker container identity')
    identity = _stage_identity(container_id, labels, role)
    if on_created is not None:
        on_created(container_id)
    _verify_bridge_effective(box, identity, expected)
    effective_digest = c.canonical_digest({'image': image, 'mounts': mounts, 'command': list(bounded),
                                           'environment': expected['environment'], 'labels': dict(labels),
                                           'network': network})
    import subprocess as subprocess_module
    started = time.monotonic()
    proc = subprocess_module.Popen(_attach_argv(box, container_id),
                                   env={'PATH': '/usr/bin:/bin', 'LANG': 'C'},
                                   stdout=subprocess_module.PIPE, stderr=subprocess_module.PIPE, start_new_session=True)
    def kill_stage():
        try:
            box.docker(['kill', container_id])
        finally:
            try:
                os.killpg(proc.pid, 9)
            except OSError:
                pass
    output, errors, timed_out, truncated = _bounded_capture(
        proc, stdout_limit=limits.output_bytes + 4096, stderr_limit=65536,
        timeout_seconds=timeout_seconds + 30, kill=kill_stage)
    elapsed = time.monotonic() - started
    instance = box._inspect(identity) if hasattr(box, '_inspect') else None
    exit_code = None
    if type(instance) is dict:
        exit_code = instance.get('State', {}).get('ExitCode')
        _require(instance.get('State', {}).get('Running') is False, 'worker container still running')
    running = bool(instance) and bool(instance.get('State', {}).get('Running'))
    if instance is None or running:
        try:
            box.docker(['kill', container_id])
        except Exception:
            pass  # Best effort only: attach already returned, and the later
            # controller reconciliation independently proves whole-container death.
    return WorkerStageResult(container_id=container_id, exit_code=exit_code, timed_out=timed_out,
                             transcript=output[:limits.output_bytes + 4096],
                             transcript_stderr=errors[:65536], truncated=truncated,
                             effective_digest=effective_digest, elapsed_seconds=elapsed)


def _verify_bridge_effective(box: Any, identity: Any, expected: Mapping[str, Any]) -> None:
    """Explicit inference-egress profile: the M1 least-privilege profile with bridge network.

    Every other assertion matches the frozen M1 effective check exactly; only
    the network mode differs, and it is an explicit recorded capability, never
    a default. Used only for the IMPLEMENTATION worker with inference egress.
    """
    instance = box._inspect(identity)
    config, host = instance['Config'], instance['HostConfig']
    limits = expected['limits']
    _require(instance.get('Image') == expected['image'] and config.get('User') == '10001:10001', 'image/user mismatch')
    _require(host.get('ReadonlyRootfs') is True and host.get('Privileged') is False, 'writable root or privileged container')
    _require(not host.get('CapAdd') and host.get('CapDrop') == ['ALL'], 'capability policy differs')
    _require(host.get('SecurityOpt') == ['no-new-privileges:true'], 'privilege escalation possible')
    _require(host.get('NetworkMode') == expected.get('network', 'bridge') and host.get('PidMode', '') == ''
             and host.get('IpcMode') == 'none' and host.get('UTSMode', '') == ''
             and host.get('UsernsMode', '') == '' and host.get('CgroupnsMode') == 'private', 'namespace policy differs')
    for name in ('Devices', 'DeviceRequests', 'DeviceCgroupRules', 'VolumesFrom', 'Links', 'ExtraHosts'):
        _require(not host.get(name), 'unexpected device/host capability: ' + name)
    _require(host.get('Memory') == limits.memory_bytes and host.get('MemorySwap') == limits.memory_bytes
             and host.get('NanoCpus') == limits.cpu * 1_000_000_000
             and host.get('PidsLimit') == limits.pids, 'resource limits differ')
    _require(host.get('RestartPolicy') == {'Name': 'no', 'MaximumRetryCount': 0}
             and host.get('LogConfig') == {'Type': 'none', 'Config': {}}, 'unbounded persistence/logs')
    _require(config.get('WorkingDir') == expected.get('workdir', '/candidate') and config.get('Entrypoint') == [expected['command'][0]]
             and (config.get('Cmd') or []) == list(expected['command'][1:]), 'effective command differs')
    environment = config.get('Env', [])
    required = expected.get('environment', [])
    approved = required + ['PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin']
    _require(isinstance(environment, list) and len(environment) == len(set(environment))
             and set(required) <= set(environment) <= set(approved), 'effective launch environment differs')
    observed_mounts = [(item.get('Type'), item.get('Source'), item.get('Destination'), item.get('RW'), item.get('Propagation'))
                       for item in instance.get('Mounts', [])]
    expected_mounts = [('bind', source, destination, writable, 'rprivate') for source, destination, writable in expected['mounts']]
    _require(len(observed_mounts) == len(expected_mounts) and sorted(observed_mounts) == sorted(expected_mounts),
             'effective candidate/config mount set differs')
    scratch = 'rw,nosuid,nodev,noexec,size=' + str(limits.scratch_bytes) + ',mode=1777'
    _require(host.get('Tmpfs') == {'/scratch': scratch}, 'scratch quota differs')
    ulimits = {item['Name']: (item['Soft'], item['Hard']) for item in host.get('Ulimits', [])}
    _require(ulimits == {'nofile': (256, 256), 'fsize': (expected.get('file_bytes', limits.output_bytes), expected.get('file_bytes', limits.output_bytes))}, 'output/FD bounds differ')
    if expected.get('created_only', True):
        _require(instance.get('State', {}).get('Status') == 'created' and not instance['State'].get('Running'),
                 'stage executed before verification')


def _stage_identity(container_id: str, labels: Mapping[str, str], role: str) -> Any:
    from . import sandbox as b
    return b.StageIdentity(container_id, labels.get('or.v2.boot', ''), labels.get('or.v2.owner', ''),
                           labels.get('or.v2.stage', ''), int(labels.get('or.v2.epoch', '0')),
                           labels.get('or.v2.nonce', ''), labels.get('or.v2.authority', ''),
                           role, labels.get('or.v2.task', ''))


def _attach_argv(box: Any, container_id: str) -> list[str]:
    docker = box.docker
    base = [str(docker.executable)]
    if docker.endpoint:
        base.append('--host=' + docker.endpoint)
    return base + ['start', '--attach', container_id]
