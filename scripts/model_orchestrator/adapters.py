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
REASONING_LEVELS = ('LOW', 'DEFAULT', 'HIGH')
OPENCODE_VERSION = '1.18.31'
OPENCODE_SHA256 = '16c960ba77421da11b53e785f359b73f328a86118b48feb4af143db5d9afb198'
CODEX_VERSION = 'codex-cli 0.158.0'

# Provisional success shapes: only error envelopes are live-observed in M3.
# M5 live certification confirms or revises these; any CLI replacement or
# unlisted event version renders the adapter unavailable until reverified.
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
            observed = subprocess.run([str(path), '--version'], env={'PATH': '/usr/bin:/bin', 'LANG': 'C'},
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

    def __init__(self, path: Path, sha256: str, version: str):
        path = Path(path).absolute()
        resolved = path.resolve()
        _require(resolved.is_file() and not resolved.is_symlink(), 'Codex executable must resolve to a pinned regular file')
        digest = hashlib.sha256(resolved.read_bytes()).hexdigest()
        _require(digest == sha256, 'Codex executable pin mismatch')
        try:
            observed = subprocess.run([str(path), '--version'], env={'PATH': '/usr/bin:/bin', 'LANG': 'C'},
                                      capture_output=True, timeout=30, check=True).stdout.decode('utf-8').strip()
        except (OSError, subprocess.SubprocessError, UnicodeError) as exc:
            raise AdapterError('Codex version probe failed: ' + str(exc), ADAPTER_UNAVAILABLE) from exc
        _require(observed == version, 'Codex version differs from pinned adapter')
        object.__setattr__(self, 'path', resolved)
        object.__setattr__(self, 'sha256', digest)
        object.__setattr__(self, 'version', observed)


def validate_enrollment(record: Any) -> dict[str, Any]:
    """Explicit controller enrollment: qualification is per-role data, never a suffix."""
    _require(type(record) is dict, 'enrollment must be an explicit record')
    required = ('provider_id', 'model_id', 'family', 'allowed_roles', 'reasoning_capabilities',
                'task_class_qualification', 'adapter_certification_digest', 'budget')
    _require(all(key in record for key in required), 'enrollment missing required fields')
    _require(type(record['provider_id']) is str and record['provider_id'] and '/' not in record['provider_id'], 'invalid provider id')
    _require(type(record['model_id']) is str and '/' in record['model_id'] and ' ' not in record['model_id'], 'invalid model id')
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
    return record


def select_model(role: str, enrollments: Sequence[Mapping[str, Any]], *, task_budget: Mapping[str, Any],
                 required_reasoning: str = 'LOW', exclude_families: Sequence[str] = (),
                 locked_model: str | None = None, availability: Mapping[str, str] | None = None) -> dict[str, Any]:
    """Cheapest qualified enrolled model; absence or lock mismatch is unavailable, never substituted."""
    _require(role in KNOWN_ROLES, 'unknown role')
    _require(required_reasoning in REASONING_LEVELS, 'unknown reasoning requirement')
    _require(type(enrollments) is list and all(type(e) is dict for e in enrollments), 'enrollment records required; listings confer nothing')
    qualified = []
    for raw in enrollments:
        record = validate_enrollment(raw)
        if role not in record['allowed_roles'] or record['task_class_qualification'].get(role) is not True:
            continue
        if record['family'] in set(exclude_families):
            continue
        if required_reasoning not in record['reasoning_capabilities']:
            continue
        state = (availability or {}).get(record['model_id'], 'unavailable')
        if state != 'available':
            continue
        qualified.append(record)
    if locked_model is not None:
        match = [record for record in qualified if record['model_id'] == locked_model]
        _require(bool(match), 'locked model is not enrolled, qualified, and available for this role; no substitution', MODEL_UNAVAILABLE)
        return dict(match[0], reasoning_requested=required_reasoning)
    _require(bool(qualified), 'no qualified available enrolled model for this role', MODEL_UNAVAILABLE)
    def cost(record: Mapping[str, Any]) -> tuple:
        budget = record['budget']
        price = budget.get('cost_microusd', 0)
        _require(type(price) is int and price >= 0, 'invalid enrollment price')
        return (price, record['model_id'])
    return dict(sorted(qualified, key=cost)[0], reasoning_requested=required_reasoning)


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
            _require(type(event.get('text')) is str and event['text'], 'final text event carries no content', PROTOCOL_ERROR)
            finals.append(event['text'])
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
        self.enrollment = validate_enrollment(enrollment)
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
        if require_confirmed_effort and self.requested == 'HIGH' and self.confirmed != self.sent:
            raise AdapterError('task requires confirmed HIGH effort the adapter cannot supply; stage stays pending', EFFORT_PENDING)
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


class CodexAdapter:
    """Read-only architecture transport. Quota defers with identical facts; anything else is a tool error."""
    def __init__(self, binary: CodexBinary, *, model_id: str, workdir: Path, decision_schema_path: Path,
                 quota_signatures: Sequence[Mapping[str, Any]] = (), env: Mapping[str, str] | None = None):
        _require(type(binary) is CodexBinary, 'pinned Codex binary required')
        _require(type(model_id) is str and '/' in model_id and ' ' not in model_id, 'invalid Codex model id')
        self.workdir = Path(workdir)
        _require(self.workdir.is_absolute(), 'adapter workdir must be absolute')
        schema_path = Path(decision_schema_path)
        _require(schema_path.is_file(), 'strict decision schema file required')
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
        return [str(self.binary.path), 'exec', '--json', '--sandbox', 'read-only',
                '--ignore-user-config', '--ignore-rules', '--ephemeral',
                '-C', str(self.workdir), '--output-schema', str(self.decision_schema_path),
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
        argv = self.build_argv(prompt, last_message)
        try:
            proc = subprocess.run(argv, env=self.env, capture_output=True, timeout=timeout_seconds)
        except subprocess.TimeoutExpired as exc:
            raise AdapterError('architecture transport timed out', TRANSPORT_ERROR) from exc
        except OSError as exc:
            raise AdapterError('architecture transport launch failed: ' + str(exc), TRANSPORT_ERROR) from exc
        if proc.returncode != 0:
            stderr = proc.stderr.decode('utf-8', 'replace')
            for signature in self.quota_signatures:
                if signature.get('version') == self.binary.version and signature['stderr_contains'] in stderr:
                    raise AdapterError('Codex quota exhausted; packet preserved', QUOTA_DEFERRED)
            raise AdapterError('Codex transport error; not a decision and not quota', TRANSPORT_ERROR)
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
        output = subprocess.run([str(binary.path), 'models'], env={'PATH': '/usr/bin:/bin', 'LANG': 'C'},
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


def launch_model_stage(*, box: Any, candidate: Any, view: Path, role: str, container_binary: str,
                       message_parts: Sequence[str], agent: str | None, adapter: OpenCodeAdapter,
                       image: str, network: str, limits: Any, overlay_mounts: Sequence[tuple],
                       labels: Mapping[str, str], name: str | None = None,
                       timeout_seconds: int = 600, workdir: str = '/candidate') -> WorkerStageResult:
    """Run the pinned model command in an isolated container; return observations only.

    No receipt, seal, or authority is minted here. The orchestrator binds the
    store launch record, reconciles through the sandbox, and parses results.
    `network` is an explicit capability: 'none' always, or 'bridge' only for
    the IMPLEMENTATION worker with inference egress recorded by the caller.
    """
    from . import sandbox as b
    _require(hasattr(box, 'docker') and hasattr(box, 'verify_effective'), 'trusted container sandbox required')
    _require(role in ('IMPLEMENTATION', 'INVESTIGATION_REVIEW'), 'unsupported worker role')
    _require(network == 'none' or (network == 'bridge' and role == 'IMPLEMENTATION'), 'network is an explicit worker-only inference capability')
    _require(type(container_binary) is str and container_binary.startswith('/') and ' ' not in container_binary, 'invalid container model binary path')
    _require(type(timeout_seconds) is int and 1 <= timeout_seconds <= 3600, 'invalid stage timeout')
    _require(type(name) is None or (type(name) is str and re.fullmatch(r'[A-Za-z0-9_.-]{1,128}', name) is not None), 'invalid container name')
    _require(type(labels) is dict and labels and all(type(k) is str and type(v) is str for k, v in labels.items()), 'stage ownership labels required')
    limits.validate()
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
    mounts.extend((str(source), destination, False) for source, destination in overlay_mounts)
    expected = dict(image=image, mounts=mounts, limits=limits, command=bounded,
                    environment=['HOME=/worker-home', 'TMPDIR=/scratch',
                                 'XDG_CONFIG_HOME=/worker-home/.config', 'XDG_DATA_HOME=/worker-home/.local/share',
                                 'XDG_STATE_HOME=/worker-home/.local/state', 'OPENCODE_CONFIG=/worker-home/opencode.json',
                                 'GIT_CONFIG_NOSYSTEM=1', 'GIT_CONFIG_GLOBAL=/dev/null'])
    args = ['create', '--pull=never', '--read-only', '--user=10001:10001', '--cap-drop=ALL',
            '--security-opt=no-new-privileges:true', '--network=' + network, '--ipc=none', '--cgroupns=private',
            '--pids-limit=' + str(limits.pids), '--cpus=' + str(limits.cpu),
            '--memory=' + str(limits.memory_bytes), '--memory-swap=' + str(limits.memory_bytes),
            '--log-driver=none', '--restart=no', '--stop-timeout=1', '--init',
            '--ulimit=nofile=256:256', '--ulimit=fsize=' + str(limits.output_bytes) + ':' + str(limits.output_bytes),
            '--mount=type=bind,src=' + str(view) + ',dst=/candidate' + ('' if role == 'IMPLEMENTATION' else ',readonly'),
            '--tmpfs=/scratch:rw,nosuid,nodev,noexec,size=' + str(limits.scratch_bytes) + ',mode=1777',
            '--workdir=' + workdir]
    if name is not None:
        args.append('--name=' + name)
    for source, destination in overlay_mounts:
        args.append('--mount=type=bind,src=' + str(source) + ',dst=' + destination + ',readonly')
    for key, value in labels.items():
        args.extend(['--label', key + '=' + value])
    for variable in expected['environment']:
        args.append('--env=' + variable)
    args.extend(['--entrypoint=' + bounded[0], image, *bounded[1:]])
    container_id = box.docker(args).strip()
    _require(re.fullmatch(r'[0-9a-f]{64}', container_id) is not None, 'invalid worker container identity')
    identity = _stage_identity(container_id, labels, role)
    if network == 'none':
        box.verify_effective(identity, expected)
    else:
        _verify_bridge_effective(box, identity, expected)
    effective_digest = c.canonical_digest({'image': image, 'mounts': mounts, 'command': list(bounded),
                                           'environment': expected['environment'], 'labels': dict(labels),
                                           'network': network})
    import subprocess as subprocess_module
    started = time.monotonic()
    proc = subprocess_module.Popen(_attach_argv(box, container_id),
                                   env={'PATH': '/usr/bin:/bin', 'LANG': 'C'},
                                   stdout=subprocess_module.PIPE, stderr=subprocess_module.PIPE, start_new_session=True)
    try:
        output, _ = proc.communicate(timeout=timeout_seconds + 30)
        timed_out = False
    except subprocess_module.TimeoutExpired:
        try:
            box.docker(['kill', container_id])
        except AdapterError:
            pass
        try:
            os.killpg(proc.pid, 9)
        except (OSError, ProcessLookupError):
            pass
        output, _ = proc.communicate()
        timed_out = True
    elapsed = time.monotonic() - started
    instance = box._inspect(identity) if hasattr(box, '_inspect') else None
    exit_code = None
    if type(instance) is dict:
        exit_code = instance.get('State', {}).get('ExitCode')
        _require(instance.get('State', {}).get('Running') is False, 'worker container still running')
    try:
        box.docker(['kill', container_id])
    except AdapterError:
        pass
    truncated = len(output) > limits.output_bytes + 4096
    return WorkerStageResult(container_id=container_id, exit_code=exit_code, timed_out=timed_out,
                             transcript=output[:limits.output_bytes + 4096], truncated=truncated,
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
    _require(host.get('NetworkMode') == 'bridge' and host.get('PidMode', '') == ''
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
    _require(ulimits == {'nofile': (256, 256), 'fsize': (limits.output_bytes, limits.output_bytes)}, 'output/FD bounds differ')
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
