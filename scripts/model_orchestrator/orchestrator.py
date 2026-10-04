"""M3 lifecycle controller: legal state/routing/attempt/escalation transitions.

No model output is trusted here. Every action is either a defined transition
with prerequisite receipts or a typed refusal. The store owns durability; the
sandbox owns isolation observations; adapters own transport. This module owns
only the decision order: admit, claim, launch, import, verify, review,
escalate, recover, and the read-only Desktop snapshot.
"""
from __future__ import annotations

import copy
from pathlib import Path
import re
import shutil
import tempfile
from typing import Any, Mapping, Sequence

from . import contracts as c
from . import adapters as a
from . import store as s
from . import sandbox as b
from . import workspace as w
from . import guards as g
from . import verification as v


class OrchestratorError(c.ContractError):
    def __init__(self, message: str, code: str = 'REFUSED'):
        self.code = code
        super().__init__(message)


def _refuse(condition: bool, message: str, code: str = 'REFUSED') -> None:
    if not condition:
        raise OrchestratorError(message, code)


# Every state/action pair is a defined transition or a typed refusal.
LEGAL_ACTIONS = {
    'PLANNED_TASK': ('ADMIT',),
    'READY': ('CLAIM',),
    'CLAIMED': ('LAUNCH',),
    'IMPLEMENTING': ('IMPORT',),
    'CANDIDATE': ('VERIFY',),
    'LOCAL_VERIFY': ('REVIEW',),
    'REVIEWING': ('AUTHORIZE',),
    'REVIEW_PENDING': ('REVIEW',),
    'PROMOTION_READY': (),
    'REJECTED': ('DIAGNOSE',),
    'DIAGNOSTIC': ('REPAIR', 'ESCALATE'),
}
TERMINAL_STAGES = ('PROMOTION_READY',)


def legal_actions(stage: str) -> tuple:
    _refuse(stage in LEGAL_ACTIONS, 'unknown lifecycle stage: ' + str(stage))
    return LEGAL_ACTIONS[stage]


def check_transition(stage: str, action: str) -> None:
    _refuse(action in legal_actions(stage), 'action %s is not legal in stage %s' % (action, stage))


def _store_objects(store: s.RuntimeStore) -> list[dict]:
    state = store.inspect()
    return [s._object(s._read(store.root / 'objects' / (digest + '.json')))
            for digest in state['object_digests']]


def derive_stage(store: s.RuntimeStore, task_id: str) -> str:
    """Derive the lifecycle stage from receipts present; missing receipts fail closed."""
    state = store.inspect()
    task = state['tasks'].get(task_id)
    _refuse(task is not None, 'unknown task', 'REFUSED')
    if task['status'] == 'CLAIMED':
        _refuse(task['stage'] is not None, 'claim missing durable stage identity')
        return 'CLAIMED' if task['launch'] is None else 'IMPLEMENTING'
    _refuse(task['status'] in ('READY', 'SETTLED'), 'contradictory task status')
    if task['status'] == 'READY':
        return 'READY'
    by_kind: dict[str, list] = {}
    for record in _store_objects(store):
        by_kind.setdefault(record['kind'], []).append(record['payload'])
    readiness = [p for p in by_kind.get('receipt', []) if p.get('kind') == 'verification-readiness' and p.get('task_id') == task_id]
    reviews = [p for p in by_kind.get('review', []) if p.get('task_id') == task_id]
    authorizations = [p for p in by_kind.get('authorization', []) if p.get('task_id') == task_id]
    _refuse(not authorizations or reviews, 'authorization without independent review')
    if authorizations:
        return 'PROMOTION_READY'
    if reviews:
        return 'REVIEWING'
    passed = [p for p in readiness if p.get('verification_passed') is True]
    _refuse(len(readiness) == len(passed), 'verification did not pass; review/repair required')
    if readiness:
        return 'REVIEW_PENDING' if state['paused'] else 'LOCAL_VERIFY'
    guard = [p for p in by_kind.get('receipt', []) if p.get('kind') == 'guard-handoff' and p.get('task_id') == task_id]
    if guard:
        return 'CANDIDATE'
    return 'READY'


class AttemptLedger:
    """Two speculative corrections per subsystem/gate episode, then causal evidence."""
    def __init__(self, entries: Sequence[Mapping[str, Any]] = ()):
        self.entries = []
        for entry in entries:
            _refuse(type(entry) is dict, 'invalid attempt entry')
            _refuse(set(entry) == {'episode', 'kind', 'evidence_digest', 'lease_epoch'}, 'invalid attempt entry fields')
            _refuse(entry['kind'] in ('initial', 'speculative', 'causal'), 'invalid attempt kind')
            _refuse(type(entry['episode']) is str and re.fullmatch(r'[A-Za-z0-9_-]{1,64}', entry['episode']) is not None, 'invalid attempt episode')
            _refuse(type(entry['lease_epoch']) is int and entry['lease_epoch'] >= 0, 'invalid attempt epoch')
            _refuse(entry['evidence_digest'] is None and entry['kind'] in ('initial', 'speculative') or type(entry['evidence_digest']) is str and re.fullmatch(r'[0-9a-f]{64}', entry['evidence_digest']) is not None, 'invalid causal evidence')
            self.entries.append(dict(entry))

    @staticmethod
    def episode_id(checkpoint: str, subsystem: str, gate: str) -> str:
        _refuse(all(type(value) is str and value for value in (checkpoint, subsystem, gate)), 'invalid episode identity')
        return c.canonical_digest([checkpoint, subsystem, gate])

    def speculative_used(self, episode: str) -> int:
        return sum(1 for e in self.entries if e['episode'] == episode and e['kind'] == 'speculative')

    def causal_used(self, episode: str) -> int:
        return sum(1 for e in self.entries if e['episode'] == episode and e['kind'] == 'causal')

    def admit(self, episode: str, *, causal_packet: Mapping[str, Any] | None = None) -> str:
        """ADMIT a speculative attempt, REQUIRE_CAUSAL after two, or ESCALATE after a failed causal."""
        _refuse(type(episode) is str and re.fullmatch(r'[A-Za-z0-9_-]{1,64}', episode) is not None, 'invalid episode')
        if self.speculative_used(episode) < 2:
            return 'ADMIT'
        if self.causal_used(episode) >= 1:
            return 'ESCALATE'
        if causal_packet is None:
            return 'REQUIRE_CAUSAL'
        _refuse(type(causal_packet) is dict, 'invalid causal packet')
        for key in ('failure_evidence_digest', 'falsifiable_cause', 'discriminating_result', 'patch_explanation'):
            _refuse(type(causal_packet.get(key)) is str and causal_packet[key], 'causal packet missing ' + key)
        return 'ADMIT'

    def record(self, episode: str, kind: str, evidence_digest: str | None, lease_epoch: int) -> dict:
        _refuse(kind in ('initial', 'speculative', 'causal'), 'invalid attempt kind')
        _refuse((evidence_digest is None and kind in ('initial', 'speculative')) or (type(evidence_digest) is str and re.fullmatch(r'[0-9a-f]{64}', evidence_digest) is not None), 'causal attempts require evidence digest')
        _refuse(type(lease_epoch) is int and lease_epoch >= 0, 'invalid lease epoch')
        entry = {'episode': episode, 'kind': kind, 'evidence_digest': evidence_digest, 'lease_epoch': lease_epoch}
        self.entries.append(entry)
        return entry

    def persist(self, store: s.RuntimeStore, task_id: str, entry: Mapping[str, Any]) -> str:
        _refuse(type(store) is s.RuntimeStore, 'trusted runtime store required')
        current = store.inspect()
        _refuse(task_id in current['tasks'], 'unknown task')
        record = dict(schema_version=1, kind='attempt',
                      payload=dict(schema_version=1, task_id=task_id, **dict(entry)))

        def update(state: dict) -> None:
            pass

        result = store.transaction(current['sequence'], current['epoch'], update, objects=[record])
        _refuse(any(d == c.canonical_digest(record) for d in result['object_digests']), 'attempt object not published')
        return c.canonical_digest(record)

    @staticmethod
    def load(store: s.RuntimeStore) -> 'AttemptLedger':
        entries = []
        for record in _store_objects(store):
            if record['kind'] == 'attempt':
                payload = record['payload']
                entries.append({'episode': payload.get('episode'), 'kind': payload.get('kind'),
                                'evidence_digest': payload.get('evidence_digest'), 'lease_epoch': payload.get('lease_epoch')})
        return AttemptLedger(entries)


def admit_repair(store: s.RuntimeStore, episode: str, causal_packet: Mapping[str, Any] | None = None) -> str:
    """Gate a corrective re-admission on the episode budget; pure decision, caller persists."""
    ledger = AttemptLedger.load(store)
    return ledger.admit(episode, causal_packet=causal_packet)


TRIAGE_LABELS = ('MECHANICAL_FIX', 'INVESTIGATE', 'ESCALATE_ARCHITECTURE', 'NO_MODEL_ACTION')


def triage_label(facts: Mapping[str, Any]) -> str:
    """Deterministic advisory label only; it never overrides a deterministic rule."""
    _refuse(type(facts) is dict, 'triage facts required')
    if facts.get('architecture_question') is True:
        return 'ESCALATE_ARCHITECTURE'
    if type(facts.get('blocking_unknowns')) is int and facts['blocking_unknowns'] > 0:
        return 'INVESTIGATE'
    failed = facts.get('failed_checks', [])
    _refuse(type(failed) is list, 'invalid triage facts')
    if failed and facts.get('speculative_used', 0) >= facts.get('speculative_budget', 2):
        return 'INVESTIGATE'
    if failed:
        return 'MECHANICAL_FIX'
    return 'NO_MODEL_ACTION'


def diagnose_disconnect(summary: Mapping[str, Any], *, hypotheses: Sequence[str] = (),
                        discriminating_observation: str | None = None) -> dict[str, Any]:
    """Historical disconnects are UNKNOWN; PASS requires competing hypotheses plus proof."""
    base = a.classify_disconnect(summary)
    _refuse(type(hypotheses) in (list, tuple) and len(set(hypotheses)) >= 2, 'diagnosis requires competing hypotheses')
    _refuse(type(discriminating_observation) is str and discriminating_observation, 'diagnosis requires a discriminating observation')
    return dict(base, hypotheses=list(hypotheses), discriminating_observation=discriminating_observation,
                verdict='UNKNOWN')


def select_worker(task: Mapping[str, Any], enrollments: Sequence[Mapping[str, Any]], *,
                  availability: Mapping[str, str], locked_model: str | None = None,
                  task_class: str = 'IMPLEMENTATION', required_reasoning: str = 'MEDIUM') -> a.ValidatedEnrollment:
    budget = task.get('budget', {})
    _refuse(type(budget) is dict, 'task budget required')
    try:
        return a.select_model('IMPLEMENTATION', enrollments, task_budget=budget, required_reasoning=required_reasoning,
                              locked_model=locked_model, availability=availability, task_class=task_class)
    except a.AdapterError as exc:
        if exc.code == a.MODEL_UNAVAILABLE:
            raise OrchestratorError('no qualified available implementation model: ' + str(exc), 'UNAVAILABLE') from exc
        raise


def select_reviewer(task: Mapping[str, Any], enrollments: Sequence[Mapping[str, Any]], *,
                    availability: Mapping[str, str], implementation_family: str,
                    task_class: str = 'INVESTIGATION_REVIEW', required_reasoning: str = 'HIGH') -> a.ValidatedEnrollment:
    _refuse(type(implementation_family) is str and implementation_family and implementation_family != 'unknown', 'implementation family required')
    budget = task.get('budget', {})
    _refuse(type(budget) is dict, 'task budget required')
    try:
        return a.select_model('INVESTIGATION_REVIEW', enrollments, task_budget=budget, required_reasoning=required_reasoning,
                              exclude_families=(implementation_family,), availability=availability, task_class=task_class)
    except a.AdapterError as exc:
        if exc.code == a.MODEL_UNAVAILABLE:
            raise OrchestratorError('no qualified independent reviewer; REVIEW_PENDING: ' + str(exc), 'REVIEW_PENDING') from exc
        raise


def admit_task(store: s.RuntimeStore, task: Mapping[str, Any], frozen_template: Mapping[str, Any]) -> str:
    _refuse(type(store) is s.RuntimeStore, 'trusted runtime store required')
    return store.register_task(task, frozen_template)


def claim_attempt(store, task, lease_epoch, *, subsystem='worker'):
    """Atomic claim admission. Frozen gate identity cannot be reset by stage/model names."""
    _refuse(type(store) is s.RuntimeStore, 'trusted runtime store required')
    _refuse(subsystem in ('worker', 'reviewer'), 'unknown attempt subsystem')
    c.validate_phase_admission(store.authority, task, capability='M3')
    episode = AttemptLedger.episode_id(task['checkpoint_id'], subsystem,
                                       c.canonical_json(sorted(task['required_check_ids'])))
    ledger = AttemptLedger.load(store)
    causal = None
    if ledger.admit(episode) == 'REQUIRE_CAUSAL':
        path = 'controller/causal-' + episode + '.json'
        try:
            causal = c.load_json_strict(c._git(Path(store.authority.controller_root), 'show',
                                              store.authority.source_sha + ':' + path).decode())
        except c.ContractError as exc:
            raise OrchestratorError('two speculative attempts exhausted; pin an independent causal diagnosis in controller source', 'REQUIRE_CAUSAL') from exc
        _refuse(causal.get('task_contract_digest') == c.canonical_digest(task)
                 and causal.get('episode') == episode
                 and causal.get('reviewer_family') not in (None, '', 'unknown', causal.get('implementation_family'))
                 and type(causal.get('reviewer_session')) is str and causal['reviewer_session'],
                 'causal diagnosis requires independent exact task/episode binding')
        evidence = c.load_json_strict(c._git(Path(store.authority.controller_root), 'show',
                                           store.authority.source_sha + ':controller/diagnostics/' +
                                           causal['failure_evidence_digest'] + '.json').decode())
        _refuse(c.canonical_digest(evidence) == causal['failure_evidence_digest'], 'causal evidence pin differs')
    decision = ledger.admit(episode, causal_packet=causal)
    _refuse(decision == 'ADMIT', 'attempt discipline requires ' + decision, decision)
    kind = 'causal' if causal is not None else ('speculative' if any(entry['episode'] == episode for entry in ledger.entries) else 'initial')
    entry = ledger.record(episode, kind, c.canonical_digest(causal) if causal is not None else None, lease_epoch)
    return dict(schema_version=1, kind='attempt',
                payload=dict(schema_version=1, task_id=task['task_id'],
                             claim_sequence=store.inspect()['sequence'] + 1, **entry))


def claim_stage(store: s.RuntimeStore, task_id: str, contract_digest: str, *, owner_nonce: str,
                boot_identity: str, stage_id: str, stage_nonce: str) -> dict[str, Any]:
    """Reserve one stage opportunity; pause, budget, and lease are rechecked after acquisition."""
    _refuse((0, task_id) in store._held, 'task lease required for entire stage claim', 'CONFLICT')
    current = store.inspect()
    _refuse(current['paused'] is False, 'paused controller claims no new stage', 'CONFLICT')
    return store.claim(task_id, contract_digest, owner_nonce=owner_nonce, boot_identity=boot_identity,
                       stage_id=stage_id, stage_nonce=stage_nonce)


def claim_review(store: s.RuntimeStore, task_id: str) -> dict[str, Any]:
    """Durably reserve one review opportunity without changing the worker receipt lease."""
    _refuse(type(store) is s.RuntimeStore, 'trusted runtime store required')
    _refuse((0, task_id) in store._held, 'task lease required for review claim', 'CONFLICT')
    current = store.inspect()
    _refuse(current['paused'] is False, 'paused controller starts no new review stage', 'CONFLICT')
    task_state = current['tasks'].get(task_id)
    _refuse(task_state is not None and task_state['status'] == 'SETTLED' and task_state['stage'] is None
             and current['active_task'] is None, 'review requires a settled verification stage', 'CONFLICT')
    _refuse(type(task_state['lease_epoch']) is int and task_state['lease_epoch'] > 0,
             'review requires the original claimed worker lease', 'CONFLICT')
    contract = _store_objects(store)
    contract = next((record for record in contract if c.canonical_digest(record) == task_state['contract_digest']), None)
    _refuse(contract is not None and contract['kind'] == 'task_contract'
             and contract['payload'].get('task_id') == task_id, 'review requires the original immutable task')
    attempt = claim_attempt(store, contract['payload'], task_state['lease_epoch'], subsystem='reviewer')
    def update(state):
        _refuse(state['paused'] is False and state['pause_generation'] == current['pause_generation']
                 and state['active_task'] is None and state['tasks'].get(task_id) == task_state,
                 'paused, stale or unsettled review claim', 'CONFLICT')
    published = store.transaction(current['sequence'], current['epoch'], update, objects=[attempt])
    attempt_digest = c.canonical_digest(attempt)
    _refuse(attempt_digest in published['object_digests'], 'review attempt was not published')
    return dict(task_id=task_id, pause_generation=published['pause_generation'],
                lease_epoch=task_state['lease_epoch'], attempt_digest=attempt_digest,
                episode=attempt['payload']['episode'], kind=attempt['payload']['kind'],
                claim_sequence=attempt['payload']['claim_sequence'])


def build_review_prompt(*, task: Mapping[str, Any], candidate_sha: str, diff_text: str, guard_flags: Sequence[str],
                        verification_summary: Mapping[str, Any], budgets: Mapping[str, Any]) -> str:
    _refuse(type(diff_text) is str, 'candidate diff required')
    bound = 262144
    truncated = len(diff_text) > bound
    parts = [
        'TASK: ' + c.canonical_json(task),
        'CANDIDATE: ' + candidate_sha + ' BASE: ' + task['base_sha'],
        'GUARD_FLAGS: ' + c.canonical_json({v.quality_flag_id(flag): flag for flag in guard_flags}),
        'VERIFICATION: ' + c.canonical_json(verification_summary),
        'BUDGETS: ' + c.canonical_json(budgets),
        'DIFF%s: ' % ('_TRUNCATED_AT_%d_BYTES' % bound if truncated else '') + diff_text[:bound],
        'Output only one JSON object. No Markdown, code fences, explanations or text outside JSON. First character must be { and last character }. Match this schema: ' + c.canonical_json(c.load_json_strict((Path(__file__).resolve().parents[2] / 'docs/execution/automation/PROTOCOL_SCHEMAS.json').read_text())['records']['review_report']),
        'Echo task_id, task_contract_digest=' + c.canonical_digest(task) + ' and candidate_sha. Coverage must contain required_check_ids. Every GUARD_FLAGS key requires a quality_flag_dispositions entry with explicit NOT_LOWERING only if proven by source and checks; evidence_digest=' + c.canonical_digest(verification_summary) + '. PASS requires no blocking defect and never means product accepted.',
    ]
    return '\n---\n'.join(parts)


def run_worker(*, store: s.RuntimeStore, stage: dict, box: b.ContainerSandbox, candidate: b.Candidate,
               task: Mapping[str, Any], enrollment: Mapping[str, Any], binary: a.OpenCodeBinary,
               agent: str, prompt: str, image: str, limits: b.Limits, network: str,
               container_binary: str, storage_root: Path, timeout_seconds: int, credential_dir: Path | None = None,
               observation_dir: Path | None = None) -> dict[str, Any]:
    """Claimed stage to preserved candidate through the durable M1 workspace lifecycle."""
    _refuse(type(store) is s.RuntimeStore and type(box) is b.ContainerSandbox and type(candidate) is b.Candidate, 'trusted controller objects required')
    _refuse((0, stage['task_id']) in store._held, 'task lease required for worker stage', 'CONFLICT')
    _refuse(network == 'none' or network == 'bridge', 'unsupported worker network capability')
    if network == 'bridge':
        _refuse(task.get('budget', {}).get('tool_calls', 0) > 0, 'inference egress requires an explicit tool budget')
    _refuse(type(enrollment) is a.ValidatedEnrollment
             and enrollment.task_contract_digest == c.canonical_digest(task)
             and enrollment.authority_digest == task['authority_digest'], 'worker enrollment binding differs')
    c.validate_phase_admission(store.authority, task, capability='M3')
    daemon = box._runtime(image)
    path = w.reserve_launch(store, stage, candidate, Path(storage_root), daemon, image, 'IMPLEMENTATION')
    view = a.build_worker_view(candidate, Path(storage_root), stage['task_id'], stage['stage_id'], 'IMPLEMENTATION', destination=path)
    w.bind_launch(store, stage)
    overlay_root = path.parent / 'overlays'
    overlays = b.write_role_overlays(overlay_root, 'IMPLEMENTATION')
    mounts = a.model_mounts(overlays=overlays, role='IMPLEMENTATION', candidate=candidate.root,
                            stage_root=path.parent, credential_dir=credential_dir,
                            provider_id=enrollment['provider_id'])
    adapter = a.OpenCodeAdapter(binary, role='IMPLEMENTATION', enrollment=enrollment, workdir=Path('/candidate'),
                                limits=a.StreamLimits(4 << 20, 4096, timeout_seconds), env=None)
    record = store.launch_record(stage)
    provisional = b.StageIdentity('0' * 64, box.boot_identity, stage['owner_nonce'], stage['stage_id'],
                                  stage['lease_epoch'], stage['stage_nonce'],
                                  record['input']['authority_digest'], 'IMPLEMENTATION', stage['task_id'])
    result = a.launch_model_stage(box=box, candidate=candidate, view=view, role='IMPLEMENTATION',
                                  container_binary=container_binary, message_parts=[prompt], agent=agent,
                                  adapter=adapter, image=image, network=network, limits=limits,
                                  overlay_mounts=mounts, labels=provisional.labels(),
                                  name=record['container_name'], timeout_seconds=timeout_seconds,
                                  on_created=lambda cid: store.bind_container(stage, cid))
    stage = store.inspect()['tasks'][stage['task_id']]['stage']
    if observation_dir is not None:
        observation_dir = Path(observation_dir)
        observation_dir.mkdir(parents=True, exist_ok=True)
        (observation_dir / 'worker-transcript.bin').write_bytes(result.transcript)
        (observation_dir / 'worker-stderr.bin').write_bytes(result.transcript_stderr)
    proof = box.reconcile_launch(store, stage)
    preserved = w.preserve_launch(store, stage, proof)
    settled = store.settle(stage, proof)
    # Useful work is preserved before any transport refusal. A clean exit alone
    # cannot certify an inference stream containing a provider error.
    stream = a.parse_event_stream(result.transcript, limits=adapter.limits, event_contract=binary.event_contract)
    _refuse(not result.timed_out and not result.truncated and result.exit_code == 0
             and stream.error is None and stream.final_payload is not None, 'worker transport failed after preservation', 'UNAVAILABLE')
    return {'metadata': {'adapter': binary.certification_digest(), 'model_id': enrollment['model_id'],
            'family': enrollment['family'], 'effort': [adapter.requested, adapter.sent, adapter.confirmed],
            'session_id': stream.session_id, 'argv_digest': c.canonical_digest(adapter.build_argv([prompt], agent=agent))},
            'container_id': result.container_id, 'exit_code': result.exit_code, 'timed_out': result.timed_out,
            'transcript_digest': c.canonical_digest(result.transcript.decode('utf-8', 'replace')),
            'effective_digest': result.effective_digest, 'elapsed_seconds': result.elapsed_seconds, 'truncated': result.truncated,
            'preserved': str(preserved), 'task_status': settled['tasks'][stage['task_id']]['status']}


def import_and_verify(*, store, task_id, task_path, catalog_path, guard_root, attempt_dir):
    """Consume the durable preserved worker output, never the untouched base input."""
    current = store.inspect()
    state = current['tasks'].get(task_id)
    _refuse(state is not None and state['status'] == 'SETTLED' and current['active_task'] is None,
             'verification requires settled whole-stage termination')
    record = state['launch']
    _refuse(record is not None and record['phase'] in ('PRESERVED', 'CLEANING', 'REMOVED'),
             'verification requires durable useful-work handoff')
    path = w.verify_preserved(store, record)
    floor = g.load_floor(store.authority, task_path, catalog_path)
    contract = s._object(s._read(store.root / 'objects' / (state['contract_digest'] + '.json')))['payload']
    _refuse(floor.task == contract, 'verification floor differs from immutable admitted task')
    candidate = b.Candidate(path, record['input']['base_oid'], record['input']['authority_digest'],
                            record['input']['nonce'], _seal=b._SEAL)
    guarded = g.inspect(candidate, floor, Path(guard_root), source_authority=store.authority)
    guard = guarded.verify()
    _refuse(not guard['vetoes'], 'deterministic veto prevents verification')
    readiness = v.execute(guarded, Path(attempt_dir), lease_epoch=state['lease_epoch'], sequence=1)
    return floor, guarded, readiness


def run_reviewer(*, authority: c.ValidatedReleaseAuthority, floor: g.Floor, guarded: g.Guarded,
                 attempt_dir: Path, enrollment: Mapping[str, Any], binary: a.OpenCodeBinary,
                 agent: str, box: b.ContainerSandbox, image: str, container_binary: str,
                 implementation_family: str, limits: b.Limits, timeout_seconds: int = 600,
                 credential_dir: Path | None = None, storage_root: Path | None = None,
                 observation_dir: Path | None = None, store: s.RuntimeStore | None = None) -> dict[str, Any]:
    """Independent read-only review: reverify everything, then transport one strict report."""
    task = floor.task
    payload = c._release_authority(authority)
    _refuse(task['task_id'] == payload['build']['task_id'], 'review task differs from authority')
    guard = guarded.verify()
    _refuse(not guard['vetoes'], 'deterministic guard veto prevents review')
    readiness = v.readiness(guarded, attempt_dir)
    _refuse(readiness['verification_passed'] is True, 'review requires all required checks PASS')
    _refuse(readiness['review_ready'] is True, 'non-disposable verification blocker prevents review')
    _refuse(type(enrollment) is a.ValidatedEnrollment
             and enrollment.task_contract_digest == c.canonical_digest(task)
             and enrollment.authority_digest == task['authority_digest'], 'review enrollment binding differs')
    candidate_root = guarded.root / 'candidate'
    diff = g.git(floor, candidate_root, 'diff', '--no-color', task['base_sha'], guard['head'])
    _refuse(len(diff) <= 1 << 20, 'candidate diff bound exceeded')
    receipts = readiness['receipts']
    prompt = build_review_prompt(task=task, candidate_sha=guard['head'], diff_text=diff.decode('utf-8', 'replace'),
                                 guard_flags=guard['flags'],
                                 verification_summary={'passed': readiness['verification_passed'], 'checks': [r['check_id'] for r in receipts]},
                                 budgets=task['budget'])
    _refuse(enrollment.get('family') != implementation_family, 'same reviewer family leaves REVIEW_PENDING', 'REVIEW_PENDING')
    review_claim = None
    if not box.fixture_only:
        _refuse(type(store) is s.RuntimeStore and store.authority == authority, 'live review requires bound durable runtime')
        review_claim = claim_review(store, task['task_id'])
    adapter = a.OpenCodeAdapter(binary, role='INVESTIGATION_REVIEW', enrollment=enrollment, workdir=Path('/candidate'),
                                limits=a.StreamLimits(4 << 20, 4096, timeout_seconds), env=None)
    with tempfile.TemporaryDirectory(prefix='or-v2-review-', dir=storage_root) as directory:
        base = Path(directory).resolve()
        if not box.fixture_only:
            _refuse(storage_root is not None, 'review requires bounded storage')
        # Byte-identical reviewer input copy; manifest equality keeps the exact
        # guarded bytes while temp mode widening stays outside the quarantine.
        shutil.copytree(candidate_root, base / 'input', symlinks=True)
        _refuse(w.manifest(base / 'input') == guard['manifest'], 'reviewer input differs from guarded candidate')
        review_candidate = b.Candidate(base / 'input', task['base_sha'], task['authority_digest'],
                                       'review-' + guard['head'][:12], _seal=b._SEAL)
        view = a.build_worker_view(review_candidate, base, task['task_id'], 'review', 'INVESTIGATION_REVIEW')
        overlays = b.write_role_overlays(base / 'overlays', 'INVESTIGATION_REVIEW')
        mounts = a.model_mounts(overlays=overlays, role='INVESTIGATION_REVIEW', candidate=base / 'input',
                                stage_root=view.parent, credential_dir=credential_dir,
                                provider_id=enrollment['provider_id'])
        nonce = 'review-' + guard['head'][:12]
        provisional = b.StageIdentity('0' * 64, box.boot_identity, nonce, 'review', 1, nonce,
                                      task['authority_digest'], 'INVESTIGATION_REVIEW', task['task_id'])
        result = a.launch_model_stage(box=box, candidate=review_candidate, view=view, role='INVESTIGATION_REVIEW',
                                      container_binary=container_binary, message_parts=[prompt], agent=agent,
                                      adapter=adapter, image=image, network='bridge', limits=limits,
                                      overlay_mounts=mounts, labels=provisional.labels(),
                                      name='or-v2-review-' + nonce, timeout_seconds=timeout_seconds)
        if observation_dir is not None:
            Path(observation_dir).mkdir(parents=True, exist_ok=True)
            (Path(observation_dir) / 'reviewer-transcript.bin').write_bytes(result.transcript)
        try:
            _refuse(not result.timed_out and not result.truncated and result.exit_code == 0, 'reviewer transport failed')
            stream = a.parse_event_stream(result.transcript, limits=a.StreamLimits(4 << 20, 4096, 3600),
                                          event_contract=binary.event_contract)
            _refuse(stream.error is None and stream.final_payload is not None, 'reviewer produced no final payload')
            report = a.parse_review_report(stream.final_payload, task=task, candidate_sha=guard['head'],
                                           schemas=payload['schemas'], reviewer_family=enrollment['family'],
                                           implementation_family=implementation_family, flag_ids=list(readiness['quality_flag_ids']))
        finally:
            box.docker(['rm', result.container_id])
            a.erase_runtime_home(box, view.parent, image)
    metadata = {'adapter': binary.certification_digest(), 'model_id': adapter.enrollment['model_id'],
                'family': enrollment['family'], 'effort': [adapter.requested, adapter.sent, adapter.confirmed],
                'session_id': stream.session_id, 'attempt_claim': review_claim, 'argv_digest': c.canonical_digest(adapter.build_argv([prompt], agent=agent))}
    return {'report': report, 'metadata': metadata, 'verdict': report['verdict']}


def persist_review(store: s.RuntimeStore, task_id: str, report: Mapping[str, Any], metadata: Mapping[str, Any]) -> str:
    _refuse(type(store) is s.RuntimeStore, 'trusted runtime store required')
    current = store.inspect()
    _refuse(task_id in current['tasks'], 'unknown task')
    record = dict(schema_version=1, kind='review',
                  payload=dict(schema_version=1, task_id=task_id, report=dict(report), metadata=dict(metadata)))

    def update(state: dict) -> None:
        pass

    result = store.transaction(current['sequence'], current['epoch'], update, objects=[record])
    digest = c.canonical_digest(record)
    _refuse(digest in result['object_digests'], 'review object not published')
    return digest


def build_escalation_packet(*, task: Mapping[str, Any], authority_payload: Mapping[str, Any],
                            kind: str, failure_facts: Mapping[str, Any], attempt_entries: Sequence[Mapping[str, Any]],
                            invariant_citations: Sequence[str], question: str, scope: str) -> dict[str, Any]:
    triggers = ('architecture_contradiction', 'security_trust_boundary', 'repeated_causal_repair_failure',
                'platform_semantics_conflict', 'persistence_schema_ownership', 'licensing_packaging',
                'evidence_contract_impossible', 'difficult_root_cause')
    forbidden = ('formatting', 'build_execution', 'test_execution', 'ci_polling', 'waiting',
                 'mechanical_fixes', 'routine_implementation')
    _refuse(kind in triggers, 'escalation kind is not an architecture question')
    _refuse(kind not in forbidden, 'forbidden escalation use')
    _refuse(type(failure_facts) is dict and failure_facts, 'exact failure facts required')
    _refuse(type(question) is str and question and type(scope) is str and scope, 'exact question and decision scope required')
    packet = {'schema_version': 1, 'task_id': task['task_id'], 'checkpoint_id': task['checkpoint_id'],
              'authority_digest': task['authority_digest'], 'base_sha': task['base_sha'],
              'kind': kind, 'failure_facts': dict(failure_facts),
              'attempt_digest': c.canonical_digest(list(attempt_entries)),
              'invariant_citations': list(invariant_citations), 'question': question, 'scope': scope}
    packet['packet_digest'] = c.canonical_digest({k: v for k, v in packet.items() if k != 'packet_digest'})
    return packet


def escalate_architecture(packet: Mapping[str, Any], codex: a.CodexAdapter, prompt: str,
                            output_dir: Path) -> tuple[dict | None, a.EscalationDeferred | None]:
    """Exceptional architecture transport only; quota defers with identical facts, never polls."""
    _refuse(type(packet) is dict and packet.get('packet_digest') == c.canonical_digest({k: v for k, v in packet.items() if k != 'packet_digest'}),
            'escalation packet integrity differs')
    try:
        return codex.run_architecture(prompt, packet=packet, output_dir=output_dir)
    except a.AdapterError as exc:
        if exc.code == a.QUOTA_DEFERRED:
            deferred = codex.defer_quota(packet)
            _refuse(deferred.packet_digest == packet['packet_digest'], 'quota deferral changed packet facts')
            return None, deferred
        raise OrchestratorError('architecture transport failed: ' + str(exc), 'UNAVAILABLE') from exc


def record_telemetry(store: s.RuntimeStore, record: Mapping[str, Any]) -> None:
    _refuse(type(store) is s.RuntimeStore, 'trusted runtime store required')
    a.append_telemetry(store.root / 'telemetry' / 'telemetry.jsonl', record)


SNAPSHOT_KEYS = ('plan_checkpoint', 'operational_stage', 'next_legal_action', 'model_availability',
                 'health', 'blockers', 'pending_escalation', 'runtime')


def publish_snapshot(*, plan_checkpoint: str, operational_stage: str, next_legal_action: str,
                     model_availability: Mapping[str, str], health: str, blockers: Sequence[str],
                     pending_escalation: str | None, runtime_present: bool) -> dict[str, Any]:
    """Read-only published facts for the Desktop viewer. Pure function: no I/O, no discovery."""
    _refuse(type(plan_checkpoint) is str and plan_checkpoint and type(operational_stage) is str and operational_stage, 'checkpoint and stage required')
    _refuse(type(next_legal_action) is str and next_legal_action, 'next legal action required')
    _refuse(type(model_availability) is dict and all(type(k) is str and type(v) is str for k, v in model_availability.items()), 'invalid availability facts')
    _refuse(type(health) is str and health and type(blockers) is list and all(type(b) is str for b in blockers), 'invalid health/blocker facts')
    _refuse(pending_escalation is None or type(pending_escalation) is str, 'invalid escalation fact')
    return {'plan_checkpoint': plan_checkpoint, 'operational_stage': operational_stage,
            'next_legal_action': next_legal_action, 'model_availability': dict(model_availability),
            'health': health, 'blockers': list(blockers), 'pending_escalation': pending_escalation,
            'runtime': 'PRESENT' if runtime_present else 'ABSENT'}


SOURCE_REVIEW_STAGES = ('prepare', 'claim', 'worker', 'guards', 'checks', 'review',
                        'authorization', 'push', 'recovery', 'handoff', 'hosted', 'state_readiness')
SOURCE_REVIEW_EVIDENCE = ('live-evidence.json', 'runtime', 'quarantine', 'attempt', 'controller-pins',
                          'recovery.json', 'handoff.json', 'state-readiness.json',
                          'cp45-conformance.json', 'cp46-receipt.json', 'cp46', 'cp47-hosted-evidence.json')


def _source_review_archive(repo: Path, release_sha: str, destination: Path) -> dict:
    """Materialize exact regular tracked bytes from Git archive, never execute source."""
    import hashlib
    import io
    import tarfile
    from pathlib import PurePosixPath
    _refuse(re.fullmatch(r'[0-9a-f]{40}', release_sha) is not None, 'exact review release SHA required')
    tree = b._git(repo, 'ls-tree', '-r', '-z', '--full-tree', release_sha)
    expected = {}
    for entry in tree.split(b'\0'):
        if not entry:
            continue
        identity, path = entry.split(b'\t', 1)
        mode, kind, oid = identity.decode().split()
        _refuse(mode in ('100644', '100755') and kind == 'blob', 'review source contains unsupported Git entries')
        expected[path.decode()] = oid
    archive = b._git(repo, 'archive', '--format=tar', release_sha)
    _refuse(not destination.exists(), 'review source destination must be fresh')
    destination.mkdir()
    actual = {}
    with tarfile.open(fileobj=io.BytesIO(archive), mode='r:') as reader:
        for member in reader:
            path = PurePosixPath(member.name)
            _refuse(not path.is_absolute() and '..' not in path.parts and (member.isdir() or member.isfile()),
                     'unsafe source archive member')
            target = destination.joinpath(*path.parts)
            if member.isdir():
                target.mkdir(parents=True, exist_ok=True)
                continue
            _refuse(member.name in expected and member.name not in actual, 'archive source inventory differs')
            data = reader.extractfile(member).read()
            oid = hashlib.sha1(b'blob ' + str(len(data)).encode() + b'\0' + data).hexdigest()
            _refuse(oid == expected[member.name], 'archive source bytes differ from release blob')
            actual[member.name] = oid
            target.parent.mkdir(parents=True, exist_ok=True)
            with target.open('xb') as stream:
                stream.write(data)
            target.chmod(0o555 if member.mode & 0o111 else 0o444)
    _refuse(actual == expected, 'Git archive omitted exact released source')
    return {'release_sha': release_sha, 'archive_sha256': hashlib.sha256(archive).hexdigest(),
            'source_manifest_digest': c.canonical_digest(actual), 'source_files': actual}


def _validate_source_review_trace(report: Mapping[str, Any], review_root: Path) -> None:
    """A PASS must cite existing source and evidence lines for every traced stage."""
    if report['verdict'] != 'PASS':
        return
    _refuse(set(SOURCE_REVIEW_STAGES) <= set(report['coverage']), 'source review omitted lifecycle stage')
    findings = {item['id']: item for item in report['findings']}
    for stage in SOURCE_REVIEW_STAGES:
        finding = findings.get(stage)
        _refuse(finding is not None and finding['severity'] == 'NONBLOCKING'
                 and finding['classification'] in ('PROVEN', 'SUPPORTED'), 'source review lacks cited stage evidence: ' + stage)
        categories = set()
        for category, relative, line in re.findall(r'(source|evidence)/([^:;\s]+):([1-9][0-9]*)', finding['citation']):
            path = Path(relative)
            _refuse(not path.is_absolute() and '..' not in path.parts, 'unsafe reviewer citation')
            target = review_root / category / path
            _refuse(target.is_file() and not target.is_symlink(), 'reviewer cited absent source/evidence')
            try:
                lines = target.read_text(encoding='utf-8').splitlines()
            except UnicodeError as exc:
                raise OrchestratorError('reviewer cited non-text input') from exc
            _refuse(int(line) <= len(lines), 'reviewer cited nonexistent input line')
            categories.add(category)
        _refuse(categories == {'source', 'evidence'}, 'stage needs exact source and evidence citations: ' + stage)


def _validate_source_review_tools(events: Sequence[Mapping[str, Any]]) -> None:
    for event in events:
        part = event.get('part', {})
        _refuse(type(part) is dict, 'malformed source review tool envelope')
        if event['type'] == 'tool_use' or part.get('type') == 'tool':
            _refuse(part.get('type') == 'tool' and part.get('tool') in ('read', 'glob', 'grep'),
                     'source reviewer attempted execution/nested tool')


def run_source_review(*, authority: c.ValidatedReleaseAuthority, task: Mapping[str, Any],
                      source_repo: Path, release_sha: str, evidence_root: Path,
                      enrollment: a.ValidatedEnrollment, binary: a.OpenCodeBinary,
                      box: b.ContainerSandbox, image: str, container_binary: str,
                      implementation_family: str, storage_root: Path, observation_dir: Path,
                      limits: b.Limits, credential_dir: Path | None = None,
                      prior_session_ids: Sequence[str] = (), timeout_seconds: int = 1800) -> dict:
    """Independent isolated source/evidence audit; produces supporting facts only."""
    import uuid
    from . import hosted
    payload = c.validate_phase_admission(authority, task, capability='M3')
    _refuse(type(enrollment) is a.ValidatedEnrollment
             and enrollment.task_contract_digest == c.canonical_digest(task)
             and enrollment.authority_digest == task['authority_digest'], 'source reviewer enrollment task binding differs')
    _refuse(enrollment['family'] != implementation_family, 'source reviewer is not independent', 'REVIEW_PENDING')
    resources = task['resource_limits']
    _refuse(limits.cpu <= resources['cpu'] and limits.memory_bytes <= resources['memory_bytes']
             and limits.pids <= resources['pids'] and limits.candidate_bytes <= resources['disk_bytes']
             and limits.output_bytes <= resources['output_bytes']
             and timeout_seconds <= min(resources['wall_seconds'], task['budget']['wall_seconds']),
             'source review exceeds immutable task resource budget')
    _refuse(not box.fixture_only, 'fixture transport cannot certify independent source review')
    evidence_root = b._safe_path(Path(evidence_root))
    for name in SOURCE_REVIEW_EVIDENCE:
        _refuse((evidence_root / name).exists(), 'source review evidence missing: ' + name)
    expected = hosted.expectation(Path(source_repo), release_sha)
    hosted_evidence = c.load_json_strict((evidence_root / 'cp47-hosted-evidence.json').read_text())
    _refuse(hosted_evidence['head_sha'] == release_sha, 'hosted review evidence binds another release')
    hosted.validate_receipts(hosted_evidence['acceptance_receipt'], hosted_evidence['collector_receipt'],
                             expected=expected, run_id=hosted_evidence['run_id'], run_attempt=hosted_evidence['run_attempt'])
    adapter = a.OpenCodeAdapter(binary, role='INVESTIGATION_REVIEW', enrollment=enrollment,
                                workdir=Path('/candidate'), limits=a.StreamLimits(8 << 20, 4096, timeout_seconds))
    nonce = 'source-review-' + uuid.uuid4().hex[:20]
    observation_dir = b._safe_path(Path(observation_dir))
    observation_dir.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='or-v2-source-review-', dir=storage_root) as directory:
        base = Path(directory).resolve()
        review_root = base / 'input'
        review_root.mkdir()
        # Independent inert Git metadata satisfies shared candidate isolation;
        # the source identity comes from actual archive blobs, never this index.
        b._git(review_root, 'init', '--initial-branch=candidate')
        identity = _source_review_archive(Path(source_repo), release_sha, review_root / 'source')
        _refuse(identity['source_manifest_digest'] == expected['package']['source_manifest_digest'],
                 'review archive differs from hosted source package')
        evidence_copy = review_root / 'evidence'
        evidence_copy.mkdir()
        original_manifest = {}
        for name in SOURCE_REVIEW_EVIDENCE:
            source = evidence_root / name
            if source.is_dir():
                original_manifest[name] = w.manifest(source)
                shutil.copytree(source, evidence_copy / name, symlinks=True)
                _refuse(w.manifest(evidence_copy / name) == original_manifest[name], 'review evidence copy differs')
            else:
                _refuse(source.is_file() and not source.is_symlink(), 'unsafe review evidence file')
                shutil.copyfile(source, evidence_copy / name)
                original_manifest[name] = c.canonical_digest(source.read_bytes().hex())
        identity.update(task_id=task['task_id'], task_contract_digest=c.canonical_digest(task),
                        package=expected['package'], evidence_manifest=original_manifest)
        (review_root / 'review-binding.json').write_text(c.canonical_json(identity) + '\n')
        candidate = b.Candidate(review_root, release_sha, task['authority_digest'], nonce, _seal=b._SEAL)
        candidate.verify()
        input_manifest = w.manifest(review_root)
        view = a.build_worker_view(candidate, base, task['task_id'], nonce, 'INVESTIGATION_REVIEW')
        view_source_manifest = w.manifest(view / 'source')
        view_evidence_manifest = w.manifest(view / 'evidence')
        overlays = b.write_role_overlays(base / 'overlays', 'INVESTIGATION_REVIEW')
        mounts = a.model_mounts(overlays=overlays, role='INVESTIGATION_REVIEW', candidate=review_root,
                                stage_root=view.parent, credential_dir=credential_dir,
                                provider_id=enrollment['provider_id'])
        prompt = ('Independently audit this exact released control-plane implementation and complete live evidence. '
                  'Read full files in /candidate/source and /candidate/evidence; review-binding.json pins SHA/package/inputs. '
                  'Never execute code, tests, shell commands, nested agents, promotion or source/controller writes. '
                  'Read/glob/grep only. Do not assume green tests prove architecture invariants. '
                  'Trace prepare→claim→worker→guards→checks→review→authorization→push→recovery→handoff→hosted→state_readiness. '
                  'Distinguish final RELEASE source SHA ' + release_sha + ' from CP44 disposable marker candidate SHA. '
                  'Check budgets/enrollment/effort, durable speculative admission, quality flag dispositions, common execution path, '
                  'push crash recovery, disabled-only handoff/completion, exact hosted receipt identity. '
                  'PASS requires no unresolved blocking defect and never grants product adoption. '
                  'Output only one strict review_report JSON object. No Markdown or code fences. First character {, last character }. For every traced stage, include coverage label and a NONBLOCKING '
                  'PROVEN/SUPPORTED finding with id equal to the stage; citation must contain existing '
                  'source/<full-relative-file>:<line>; evidence/<full-relative-file>:<line> references, and a substantive checked claim. '
                  'If evidence is absent or a defect exists, report it truthfully using BLOCKED/INCONCLUSIVE/DEFECT_FOUND. '
                  'Schema: ' + c.canonical_json(payload['schemas']['records']['review_report']) +
                  ' Exact binding: task_id=' + task['task_id'] + ', task_contract_digest=' + c.canonical_digest(task) +
                  ', candidate_sha=' + release_sha + '. Required coverage also includes: ' + c.canonical_json(task['required_check_ids']))
        labels = b.StageIdentity('0' * 64, box.boot_identity, nonce, 'source-review', 1, nonce,
                                  task['authority_digest'], 'INVESTIGATION_REVIEW', task['task_id']).labels()
        result = a.launch_model_stage(box=box, candidate=candidate, view=view, role='INVESTIGATION_REVIEW',
                                      container_binary=container_binary, message_parts=[prompt], agent=None,
                                      adapter=adapter, image=image, network='bridge', limits=limits,
                                      overlay_mounts=mounts, labels=labels, name='or-v2-' + nonce,
                                      timeout_seconds=timeout_seconds)
        (observation_dir / 'cp48-transcript.bin').write_bytes(result.transcript)
        (observation_dir / 'cp48-stderr.bin').write_bytes(result.transcript_stderr)
        try:
            _refuse(not result.timed_out and not result.truncated and result.exit_code == 0, 'source review transport failed', 'UNAVAILABLE')
            stream = a.parse_event_stream(result.transcript, limits=adapter.limits, event_contract=binary.event_contract)
            _refuse(stream.error is None and stream.final_payload is not None, 'source reviewer returned no final report', 'UNAVAILABLE')
            _refuse(stream.session_id not in prior_session_ids, 'source reviewer reused implementation/review session')
            _validate_source_review_tools(stream.events)
            report = a.parse_review_report(stream.final_payload, task=task, candidate_sha=release_sha,
                                           schemas=payload['schemas'], reviewer_family=enrollment['family'],
                                           implementation_family=implementation_family, flag_ids=[])
            _validate_source_review_trace(report, view)
            _refuse(w.manifest(review_root) == input_manifest and w.manifest(view / 'source') == view_source_manifest
                     and w.manifest(view / 'evidence') == view_evidence_manifest, 'review source/evidence changed')
            for name, before in original_manifest.items():
                source = evidence_root / name
                after = w.manifest(source) if source.is_dir() else c.canonical_digest(source.read_bytes().hex())
                _refuse(after == before, 'controller evidence changed during review')
            _refuse(hosted.expectation(Path(source_repo), release_sha) == expected, 'source changed during review')
            metadata = dict(identity, session_id=stream.session_id, container_id=result.container_id,
                            effective_digest=result.effective_digest, model_id=enrollment['model_id'], family=enrollment['family'],
                            argv_digest=c.canonical_digest(adapter.build_argv([prompt])), readonly_unchanged=True,
                            authority_semantics='SUPPORTING_FACTS_ONLY_NO_ADOPTION')
            return {'report': report, 'metadata': metadata, 'verdict': report['verdict']}
        finally:
            if box._inspect(b.StageIdentity(result.container_id, box.boot_identity, nonce, 'source-review', 1, nonce,
                                            task['authority_digest'], 'INVESTIGATION_REVIEW', task['task_id']))['State']['Running'] is False:
                box.docker(['rm', result.container_id])
                a.erase_runtime_home(box, view.parent, image)
