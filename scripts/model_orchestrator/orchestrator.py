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
            _refuse(entry['kind'] in ('speculative', 'causal'), 'invalid attempt kind')
            self.entries.append(dict(entry))

    @staticmethod
    def episode_id(checkpoint: str, subsystem: str, gate: str) -> str:
        raw = '-'.join((checkpoint, subsystem, gate))
        cleaned = re.sub(r'[^A-Za-z0-9_-]', '-', raw)[:64]
        _refuse(bool(re.fullmatch(r'[A-Za-z0-9_-]{1,64}', cleaned)), 'invalid episode identity')
        return cleaned

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
        _refuse(kind in ('speculative', 'causal'), 'invalid attempt kind')
        _refuse((evidence_digest is None and kind == 'speculative') or (type(evidence_digest) is str and re.fullmatch(r'[0-9a-f]{64}', evidence_digest) is not None), 'causal attempts require evidence digest')
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
                                'evidence_digest': payload.get('evidence_digest'), 'lease_epoch': payload.get('lease_epoch', 0)})
        ledger = AttemptLedger()
        ledger.entries = [e for e in entries if type(e['episode']) is str and e['kind'] in ('speculative', 'causal')]
        return ledger


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
                  availability: Mapping[str, str], locked_model: str | None = None) -> dict[str, Any]:
    budget = task.get('budget', {})
    _refuse(type(budget) is dict, 'task budget required')
    try:
        return a.select_model('IMPLEMENTATION', enrollments, task_budget=budget, required_reasoning='HIGH',
                              locked_model=locked_model, availability=availability)
    except a.AdapterError as exc:
        if exc.code == a.MODEL_UNAVAILABLE:
            raise OrchestratorError('no qualified available implementation model: ' + str(exc), 'UNAVAILABLE') from exc
        raise


def select_reviewer(task: Mapping[str, Any], enrollments: Sequence[Mapping[str, Any]], *,
                    availability: Mapping[str, str], implementation_family: str) -> dict[str, Any]:
    _refuse(type(implementation_family) is str and implementation_family and implementation_family != 'unknown', 'implementation family required')
    budget = task.get('budget', {})
    _refuse(type(budget) is dict, 'task budget required')
    try:
        return a.select_model('INVESTIGATION_REVIEW', enrollments, task_budget=budget, required_reasoning='HIGH',
                              exclude_families=(implementation_family,), availability=availability)
    except a.AdapterError as exc:
        if exc.code == a.MODEL_UNAVAILABLE:
            raise OrchestratorError('no qualified independent reviewer; REVIEW_PENDING: ' + str(exc), 'REVIEW_PENDING') from exc
        raise


def admit_task(store: s.RuntimeStore, task: Mapping[str, Any], frozen_template: Mapping[str, Any]) -> str:
    _refuse(type(store) is s.RuntimeStore, 'trusted runtime store required')
    return store.register_task(task, frozen_template)


def claim_stage(store: s.RuntimeStore, task_id: str, contract_digest: str, *, owner_nonce: str,
                boot_identity: str, stage_id: str, stage_nonce: str) -> dict[str, Any]:
    """Reserve one stage opportunity; pause, budget, and lease are rechecked after acquisition."""
    _refuse((0, task_id) in store._held, 'task lease required for entire stage claim', 'CONFLICT')
    current = store.inspect()
    _refuse(current['paused'] is False, 'paused controller claims no new stage', 'CONFLICT')
    return store.claim(task_id, contract_digest, owner_nonce=owner_nonce, boot_identity=boot_identity,
                       stage_id=stage_id, stage_nonce=stage_nonce)


def claim_review(store: s.RuntimeStore, task_id: str) -> dict[str, Any]:
    """A review claim rechecks pause atomically; pause-after-claim settles but starts nothing new."""
    _refuse((0, task_id) in store._held, 'task lease required for review claim', 'CONFLICT')
    current = store.inspect()
    _refuse(current['paused'] is False, 'paused controller starts no new review stage', 'CONFLICT')
    task = current['tasks'].get(task_id)
    _refuse(task is not None and task['status'] == 'SETTLED', 'review requires a settled verification stage', 'CONFLICT')
    return {'task_id': task_id, 'pause_generation': current['pause_generation'], 'lease_epoch': current['epoch']}


def build_review_prompt(*, task: Mapping[str, Any], candidate_sha: str, diff_text: str, guard_flags: Sequence[str],
                        verification_summary: Mapping[str, Any], budgets: Mapping[str, Any]) -> str:
    _refuse(type(diff_text) is str, 'candidate diff required')
    bound = 262144
    truncated = len(diff_text) > bound
    parts = [
        'TASK: ' + c.canonical_json(task),
        'CANDIDATE: ' + candidate_sha + ' BASE: ' + task['base_sha'],
        'GUARD_FLAGS: ' + c.canonical_json(list(guard_flags)),
        'VERIFICATION: ' + c.canonical_json(verification_summary),
        'BUDGETS: ' + c.canonical_json(budgets),
        'DIFF%s: ' % ('_TRUNCATED_AT_%d_BYTES' % bound if truncated else '') + diff_text[:bound],
        'Respond with exactly one strict review_report JSON object. PASS requires no blocking defect under this contract and never means product accepted.',
    ]
    return '\n---\n'.join(parts)


def run_worker(*, store: s.RuntimeStore, stage: dict, box: b.ContainerSandbox, candidate: b.Candidate,
               task: Mapping[str, Any], enrollment: Mapping[str, Any], binary: a.OpenCodeBinary,
               agent: str, prompt: str, image: str, limits: b.Limits, network: str,
               container_binary: str, storage_root: Path, timeout_seconds: int) -> dict[str, Any]:
    """Claimed stage to preserved candidate through the durable M1 workspace lifecycle."""
    _refuse(type(store) is s.RuntimeStore and type(box) is b.ContainerSandbox and type(candidate) is b.Candidate, 'trusted controller objects required')
    _refuse((0, stage['task_id']) in store._held, 'task lease required for worker stage', 'CONFLICT')
    _refuse(network == 'none' or network == 'bridge', 'unsupported worker network capability')
    if network == 'bridge':
        _refuse(task.get('budget', {}).get('tool_calls', 0) > 0, 'inference egress requires an explicit tool budget')
    daemon = box._runtime(image)
    path = w.reserve_launch(store, stage, candidate, Path(storage_root), daemon, image, 'IMPLEMENTATION')
    view = a.build_worker_view(candidate, Path(storage_root), stage['task_id'], stage['stage_id'], 'IMPLEMENTATION', destination=path)
    w.bind_launch(store, stage)
    overlay_root = path.parent / 'overlays'
    overlays = b.write_role_overlays(overlay_root, 'IMPLEMENTATION')
    mounts = b._validated_role_overlays(overlays, 'IMPLEMENTATION', candidate.root)
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
                                  name=record['container_name'], timeout_seconds=timeout_seconds)
    proof = box.reconcile_launch(store, stage)
    preserved = w.preserve_launch(store, stage, proof)
    settled = store.settle(stage, proof)
    return {'container_id': result.container_id, 'exit_code': result.exit_code, 'timed_out': result.timed_out,
            'transcript_digest': c.canonical_digest(result.transcript.decode('utf-8', 'replace')),
            'effective_digest': result.effective_digest, 'elapsed_seconds': result.elapsed_seconds,
            'preserved': str(preserved), 'task_status': settled['tasks'][stage['task_id']]['status']}


def run_reviewer(*, authority: c.ValidatedReleaseAuthority, floor: g.Floor, guarded: g.Guarded,
                 attempt_dir: Path, enrollment: Mapping[str, Any], binary: a.OpenCodeBinary,
                 agent: str, box: b.ContainerSandbox, image: str, container_binary: str,
                 implementation_family: str, limits: b.Limits, timeout_seconds: int = 600) -> dict[str, Any]:
    """Independent read-only review: reverify everything, then transport one strict report."""
    task = floor.task
    payload = c._release_authority(authority)
    _refuse(task['task_id'] == payload['build']['task_id'], 'review task differs from authority')
    guard = guarded.verify()
    _refuse(not guard['vetoes'], 'deterministic guard veto prevents review')
    readiness = v.readiness(guarded, attempt_dir)
    _refuse(readiness['verification_passed'] is True, 'review requires all required checks PASS')
    _refuse(not readiness['unresolved'], 'unresolved verification prevents review: ' + ';'.join(readiness['unresolved'][:3]))
    candidate_root = guarded.root / 'candidate'
    diff = g.git(floor, candidate_root, 'diff', '--no-color', task['base_sha'], guard['head'])
    _refuse(len(diff) <= 1 << 20, 'candidate diff bound exceeded')
    receipts = readiness['receipts']
    prompt = build_review_prompt(task=task, candidate_sha=guard['head'], diff_text=diff.decode('utf-8', 'replace'),
                                 guard_flags=guard['flags'],
                                 verification_summary={'passed': readiness['verification_passed'], 'checks': [r['check_id'] for r in receipts]},
                                 budgets=task['budget'])
    _refuse(enrollment.get('family') != implementation_family, 'same reviewer family leaves REVIEW_PENDING', 'REVIEW_PENDING')
    adapter = a.OpenCodeAdapter(binary, role='INVESTIGATION_REVIEW', enrollment=enrollment, workdir=Path('/candidate'),
                                limits=a.StreamLimits(4 << 20, 4096, timeout_seconds), env=None)
    with tempfile.TemporaryDirectory(prefix='or-v2-review-') as directory:
        base = Path(directory).resolve()
        # Byte-identical reviewer input copy; manifest equality keeps the exact
        # guarded bytes while temp mode widening stays outside the quarantine.
        shutil.copytree(candidate_root, base / 'input', symlinks=True)
        _refuse(w.manifest(base / 'input') == guard['manifest'], 'reviewer input differs from guarded candidate')
        review_candidate = b.Candidate(base / 'input', task['base_sha'], task['authority_digest'],
                                       'review-' + guard['head'][:12], _seal=b._SEAL)
        view = a.build_worker_view(review_candidate, base, task['task_id'], 'review', 'INVESTIGATION_REVIEW')
        overlays = b.write_role_overlays(base / 'overlays', 'INVESTIGATION_REVIEW')
        mounts = b._validated_role_overlays(overlays, 'INVESTIGATION_REVIEW', base / 'input')
        nonce = 'review-' + guard['head'][:12]
        provisional = b.StageIdentity('0' * 64, box.boot_identity, nonce, 'review', 1, nonce,
                                      task['authority_digest'], 'INVESTIGATION_REVIEW', task['task_id'])
        result = a.launch_model_stage(box=box, candidate=review_candidate, view=view, role='INVESTIGATION_REVIEW',
                                      container_binary=container_binary, message_parts=[prompt], agent=agent,
                                      adapter=adapter, image=image, network='none', limits=limits,
                                      overlay_mounts=mounts, labels=provisional.labels(),
                                      name='or-v2-review-' + nonce, timeout_seconds=timeout_seconds)
        try:
            _refuse(not result.timed_out and result.exit_code == 0, 'reviewer transport failed')
            stream = a.parse_event_stream(result.transcript, limits=a.StreamLimits(4 << 20, 4096, 3600),
                                          event_contract=binary.event_contract)
            _refuse(stream.error is None and stream.final_payload is not None, 'reviewer produced no final payload')
            report = a.parse_review_report(stream.final_payload, task=task, candidate_sha=guard['head'],
                                           schemas=payload['schemas'], reviewer_family=enrollment['family'],
                                           implementation_family=implementation_family, flag_ids=guard['flags'])
        finally:
            box.docker(['rm', result.container_id])
    metadata = {'adapter': binary.certification_digest(), 'model_id': adapter.enrollment['model_id'],
                'family': enrollment['family'], 'effort': [adapter.requested, adapter.sent, adapter.confirmed],
                'session_id': stream.session_id, 'argv_digest': c.canonical_digest(adapter.build_argv([prompt], agent=agent))}
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
