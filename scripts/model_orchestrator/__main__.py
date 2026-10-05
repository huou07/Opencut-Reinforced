#!/usr/bin/env python3
"""M3 operator CLI: one path for admission/claims/recovery/review/escalation.

Typed JSON results on stdout; exit 0 success, 2 contract refusal, 3
unavailable/pending/deferred, 4 conflict/stale. No successful no-ops: a
repeated or stale action refuses instead of reporting success.
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO_ROOT / 'scripts'))

from model_orchestrator import contracts as c
from model_orchestrator import adapters as a
from model_orchestrator import store as s
from model_orchestrator import sandbox as b
from model_orchestrator import workspace as w
from model_orchestrator import guards as g
from model_orchestrator import verification as v
from model_orchestrator import orchestrator as o

EXIT_OK, EXIT_REFUSED, EXIT_UNAVAILABLE, EXIT_CONFLICT = 0, 2, 3, 4


def result(status: str, **fields: object) -> dict:
    return {'status': status, **fields}


def exit_for(status: str) -> int:
    return {'OK': EXIT_OK, 'REFUSED': EXIT_REFUSED, 'UNAVAILABLE': EXIT_UNAVAILABLE,
            'PENDING': EXIT_UNAVAILABLE, 'DEFERRED': EXIT_UNAVAILABLE,
            'CONFLICT': EXIT_CONFLICT}[status]


def emit(payload: dict) -> int:
    sys.stdout.write(c.canonical_json(payload) + '\n')
    return exit_for(payload['status'])


def load_json(path: str) -> object:
    return c.load_json_strict(Path(path).read_text(encoding='utf-8'), path)


def bootstrap_from_dict(raw: object) -> c.ControllerBootstrap:
    _record = c._require_object(raw, 'bootstrap')
    def pin(value: object, label: str) -> c.RecordPin | None:
        if value is None:
            return None
        item = c._require_object(value, label)
        return c.RecordPin(str(item['path']), str(item['digest']))
    return c.ControllerBootstrap(
        source_sha=str(_record['source_sha']), anchor_sha=str(_record['anchor_sha']),
        base_sha=str(_record['base_sha']), release_sha=str(_record['release_sha']),
        candidate_branch=str(_record['candidate_branch']), authorization_id=str(_record['authorization_id']),
        task_id=str(_record['task_id']), sequence=int(_record['sequence']), nonce=str(_record['nonce']),
        sandbox_digest=str(_record['sandbox_digest']), build=pin(_record['build'], 'bootstrap.build'),
        certification=pin(_record.get('certification'), 'bootstrap.certification'),
        adoption=pin(_record.get('adoption'), 'bootstrap.adoption'))


def load_authority(args: argparse.Namespace) -> c.ValidatedReleaseAuthority:
    raw = load_json(args.bootstrap)
    bootstrap = bootstrap_from_dict(raw)
    if raw.get('product_task_pin') is not None:
        if not args.product_root:
            raise c.ContractError('operational bootstrap requires the immutable product base root')
        from .product import load_operational_authority
        pin = raw['product_task_pin']
        return load_operational_authority(Path(args.candidate_root), Path(args.controller_root), Path(args.product_root),
            bootstrap=bootstrap, product_pin=c.RecordPin(pin['path'], pin['digest']))
    if args.product_root:
        raise c.ContractError('product root requires an externally pinned product task')
    return c.load_release_authority(Path(args.candidate_root), Path(args.controller_root), bootstrap=bootstrap)


def open_store(args: argparse.Namespace, authority: c.ValidatedReleaseAuthority) -> s.RuntimeStore:
    return s.RuntimeStore(Path(args.runtime), authority)


def opencode_binary(args: argparse.Namespace) -> a.OpenCodeBinary:
    return a.OpenCodeBinary(Path(args.opencode_bin), args.opencode_sha, args.opencode_version)


def load_role_enrollment(args: argparse.Namespace, authority: c.ValidatedReleaseAuthority,
                         task: dict) -> a.ValidatedEnrollment:
    if not args.enrollment:
        raise c.ContractError('launch requires an externally pinned enrollment')
    record = load_json(args.enrollment)
    digest = args.enrollment_digest or c.canonical_digest(record)
    return a.load_enrollment(record, authority=authority, task=task, expected_digest=digest)


def do_admit(args: argparse.Namespace) -> dict:
    try:
        authority = load_authority(args)
        runtime = open_store(args, authority)
        if not (Path(args.runtime) / 'state.json').exists():
            runtime.initialize()
        task = load_json(args.task)
        template = load_json(args.template)
        digest = o.admit_task(runtime, task, template)
        return result('OK', stage='READY', task_id=task['task_id'], contract_digest=digest)
    except o.OrchestratorError as exc:
        return result('REFUSED' if exc.code == 'REFUSED' else exc.code, reason=str(exc))
    except s.StoreError as exc:
        return result('CONFLICT', reason=str(exc))
    except c.ContractError as exc:
        return result('REFUSED', reason=str(exc))


def do_approve_causal(args: argparse.Namespace) -> dict:
    try:
        authority = load_authority(args)
        runtime = open_store(args, authority)
        diagnosis = c.load_release_authority(Path(args.candidate_root), Path(args.diagnosis_controller),
                    bootstrap=bootstrap_from_dict(load_json(args.diagnosis_bootstrap)))
        with runtime.lock('task', args.task_id):
            digest = o.approve_causal(runtime, args.task_id, diagnosis_authority=diagnosis, subsystem=args.subsystem)
        return result('OK', approval_digest=digest)
    except (c.ContractError, s.StoreError, o.OrchestratorError) as exc:
        return result('REFUSED', reason=str(exc))


def do_claim(args: argparse.Namespace) -> dict:
    try:
        authority = load_authority(args)
        runtime = open_store(args, authority)
        current = runtime.inspect()
        task = current['tasks'].get(args.task_id)
        if task is None:
            return result('REFUSED', reason='unknown task')
        full_task = c.load_json_strict(open_store_args_task(args, runtime, args.task_id))
        enrollment = None
        if args.launch:
            enrollment = load_role_enrollment(args, authority, full_task)
            enrollment = o.select_worker(full_task, [enrollment], availability=c.load_json_strict(args.availability or '{}'),
                                         locked_model=args.locked_model, task_class=args.task_class,
                                         required_reasoning=args.required_reasoning)
        with runtime.lock('task', args.task_id):
            stage = o.claim_stage(runtime, args.task_id, task['contract_digest'], owner_nonce=args.owner,
                                  boot_identity=args.boot, stage_id=args.stage_id, stage_nonce=args.stage_nonce)
            if not args.launch:
                return result('OK', stage='CLAIMED', lease_epoch=stage['lease_epoch'])
            candidate = b.restore_candidate(runtime, args.task_id)
            daemon_box = b.ContainerSandbox(b.DockerCLI(Path(args.docker_bin), args.docker_sha, endpoint=args.endpoint),
                                            boot_identity=args.boot, host_platform='linux')
            outcome = o.run_worker(store=runtime, stage=stage, box=daemon_box, candidate=candidate, task=full_task,
                                   enrollment=enrollment, binary=opencode_binary(args), agent=args.agent,
                                   prompt=Path(args.prompt).read_text(encoding='utf-8'), image=args.image,
                                   limits=b.Limits(*[int(x) for x in args.limits.split(',')]),
                                   network=args.network, container_binary=args.container_binary,
                                   storage_root=Path(args.storage_root), timeout_seconds=args.timeout,
                                   credential_dir=Path(args.credential_dir) if args.credential_dir else None)
            return result('OK', stage=outcome['task_status'], preserved=outcome['preserved'],
                          exit_code=outcome['exit_code'], timed_out=outcome['timed_out'],
                          transcript_digest=outcome['transcript_digest'])
    except o.OrchestratorError as exc:
        code = exc.code if exc.code in ('REFUSED', 'UNAVAILABLE', 'CONFLICT', 'PENDING', 'DEFERRED') else 'REFUSED'
        return result(code, reason=str(exc))
    except s.StoreError as exc:
        return result('CONFLICT', reason=str(exc))
    except (c.ContractError, b.SandboxError, a.AdapterError) as exc:
        code = getattr(exc, 'code', 'REFUSED')
        code = code if code in ('REFUSED', 'UNAVAILABLE', 'CONFLICT', 'PENDING', 'DEFERRED',
                                'ADAPTER_UNAVAILABLE', 'MODEL_UNAVAILABLE', 'TRANSPORT_ERROR') else 'REFUSED'
        if code in ('ADAPTER_UNAVAILABLE', 'MODEL_UNAVAILABLE', 'TRANSPORT_ERROR'):
            code = 'UNAVAILABLE'
        return result(code, reason=str(exc))


def open_store_args_task(args: argparse.Namespace, runtime: s.RuntimeStore, task_id: str) -> str:
    state = runtime.inspect()
    digest = state['tasks'][task_id]['contract_digest']
    record = s._object(s._read(runtime.root / 'objects' / (digest + '.json')))
    return c.canonical_json(record['payload'])


def do_verify(args):
    try:
        authority = load_authority(args)
        runtime = open_store(args, authority)
        with runtime.lock('task', args.task_id):
            _, _, readiness = o.import_and_verify(store=runtime, task_id=args.task_id,
                task_path=args.task_path, catalog_path=args.catalog_path,
                guard_root=Path(args.guard_root), attempt_dir=Path(args.attempt_dir))
        return result('OK' if readiness['verification_passed'] else 'REFUSED', readiness=readiness)
    except c.ContractError as exc:
        return result('REFUSED', reason=str(exc))


def do_review(args: argparse.Namespace) -> dict:
    try:
        authority = load_authority(args)
        runtime = open_store(args, authority)
        floor = g.load_floor(authority, args.task_path, args.catalog_path)
        guarded = g.restore_guarded(floor, Path(args.guard_root))
        full_task = c.load_json_strict(open_store_args_task(args, runtime, args.task_id))
        if full_task != floor.task:
            raise c.ContractError('review floor differs from immutable admitted task')
        enrollment = load_role_enrollment(args, authority, full_task)
        enrollment = o.select_reviewer(full_task, [enrollment], availability=c.load_json_strict(args.availability or '{}'),
                                       implementation_family=args.implementation_family, task_class=args.task_class,
                                       required_reasoning=args.required_reasoning)
        with runtime.lock('task', args.task_id):
            daemon_box = b.ContainerSandbox(b.DockerCLI(Path(args.docker_bin), args.docker_sha, endpoint=args.endpoint),
                                            boot_identity=b.host_boot_identity(), host_platform='linux')
            outcome = o.run_reviewer(authority=authority, floor=floor, guarded=guarded,
                                     attempt_dir=Path(args.attempt_dir), enrollment=enrollment,
                                     binary=opencode_binary(args), agent=args.agent, box=daemon_box,
                                     image=args.image, container_binary=args.container_binary,
                                     implementation_family=args.implementation_family,
                                     limits=b.Limits(*[int(x) for x in args.limits.split(',')]),
                                     timeout_seconds=args.timeout,
                                     credential_dir=Path(args.credential_dir) if args.credential_dir else None,
                                     storage_root=Path(args.storage_root) if args.storage_root else None, store=runtime)
            digest = o.persist_review(runtime, args.task_id, outcome['report'], outcome['metadata'])
            status = 'OK' if outcome['verdict'] == 'PASS' else 'REFUSED'
            return result(status, verdict=outcome['verdict'], review_digest=digest)
    except o.OrchestratorError as exc:
        code = exc.code if exc.code in ('REFUSED', 'UNAVAILABLE', 'CONFLICT', 'PENDING', 'DEFERRED', 'REVIEW_PENDING') else 'REFUSED'
        if code == 'REVIEW_PENDING':
            code = 'PENDING'
        return result(code, reason=str(exc))
    except s.StoreError as exc:
        return result('CONFLICT', reason=str(exc))
    except (c.ContractError, b.SandboxError, a.AdapterError) as exc:
        code = getattr(exc, 'code', 'REFUSED')
        if code in ('REVIEW_PENDING',):
            return result('PENDING', reason=str(exc))
        if code in ('ADAPTER_UNAVAILABLE', 'MODEL_UNAVAILABLE', 'TRANSPORT_ERROR', 'EFFORT_PENDING'):
            return result('UNAVAILABLE', reason=str(exc))
        return result('REFUSED', reason=str(exc))


def do_escalate(args: argparse.Namespace) -> dict:
    try:
        authority = load_authority(args)
        runtime = open_store(args, authority)
        current = runtime.inspect()
        task = current['tasks'].get(args.task_id)
        if task is None:
            return result('REFUSED', reason='unknown task')
        full_task = c.load_json_strict(open_store_args_task(args, runtime, args.task_id))
        payload = c._release_authority(authority)
        ledger = o.AttemptLedger.load(runtime)
        packet = o.build_escalation_packet(task=full_task, authority_payload=payload, kind=args.kind,
                                           failure_facts=load_json(args.failure_facts),
                                           attempt_entries=ledger.entries,
                                           invariant_citations=args.invariants.split(',') if args.invariants else [],
                                           question=args.question, scope=args.scope)
        if args.codex_model is None:
            return result('OK', packet_digest=packet['packet_digest'], transported=False)
        codex = a.CodexAdapter(a.CodexBinary(Path(args.codex_bin), args.codex_sha, args.codex_version),
                               model_id=args.codex_model, workdir=Path(args.codex_workdir),
                               decision_schema_path=Path(args.decision_schema),
                               quota_signatures=load_json(args.quota_signatures) if args.quota_signatures else [])
        output_dir = Path(args.codex_output_dir)
        output_dir.mkdir(mode=0o700, parents=True, exist_ok=True)
        decision, deferred = o.escalate_architecture(packet, codex, Path(args.prompt).read_text(encoding='utf-8'), output_dir)
        if deferred is not None:
            return result('DEFERRED', availability='quota', packet_digest=deferred.packet_digest)
        return result('OK', decision_digest=c.canonical_digest(decision))
    except o.OrchestratorError as exc:
        code = exc.code if exc.code in ('REFUSED', 'UNAVAILABLE', 'CONFLICT', 'PENDING', 'DEFERRED') else 'REFUSED'
        return result(code, reason=str(exc))
    except s.StoreError as exc:
        return result('CONFLICT', reason=str(exc))
    except (c.ContractError, b.SandboxError, a.AdapterError) as exc:
        code = getattr(exc, 'code', 'REFUSED')
        if code in ('ADAPTER_UNAVAILABLE', 'MODEL_UNAVAILABLE', 'TRANSPORT_ERROR', 'QUOTA_DEFERRED', 'EFFORT_PENDING'):
            return result('UNAVAILABLE' if code != 'QUOTA_DEFERRED' else 'DEFERRED', reason=str(exc))
        return result('REFUSED', reason=str(exc))


def do_recover(args: argparse.Namespace) -> dict:
    try:
        authority = load_authority(args)
        runtime = open_store(args, authority)
        with runtime.lock('task', args.task_id):
            current = runtime.inspect()
            task = current['tasks'].get(args.task_id)
            if task is None or task['stage'] is None:
                return result('REFUSED', reason='nothing to reconcile')
            daemon_box = b.ContainerSandbox(b.DockerCLI(Path(args.docker_bin), args.docker_sha, endpoint=args.endpoint),
                                            boot_identity=b.host_boot_identity(), host_platform='linux')
            proof = daemon_box.reconcile_launch(runtime, task['stage'])
            path = w.preserve_launch(runtime, task['stage'], proof)
            settled = runtime.settle(task['stage'], proof)
            recovered = None
            if args.guard_root and args.attempt_dir:
                floor = g.load_floor(authority, args.task_path, args.catalog_path)
                guarded = g.restore_guarded(floor, Path(args.guard_root))
                recovered = v.recover_attempt(guarded, Path(args.attempt_dir))
            return result('OK', preserved=str(path), task_status=settled['tasks'][args.task_id]['status'],
                          attempt_recovered=recovered['attempt_nonce'] if recovered else None)
    except o.OrchestratorError as exc:
        code = exc.code if exc.code in ('REFUSED', 'UNAVAILABLE', 'CONFLICT', 'PENDING', 'DEFERRED') else 'REFUSED'
        return result(code, reason=str(exc))
    except s.StoreError as exc:
        return result('CONFLICT', reason=str(exc))
    except (c.ContractError, b.SandboxError, a.AdapterError) as exc:
        code = getattr(exc, 'code', 'REFUSED')
        if code in ('ADAPTER_UNAVAILABLE', 'MODEL_UNAVAILABLE', 'TRANSPORT_ERROR'):
            return result('UNAVAILABLE', reason=str(exc))
        return result('REFUSED', reason=str(exc))


def do_snapshot(args: argparse.Namespace) -> dict:
    try:
        authority = load_authority(args)
        root = Path(args.runtime)
        present = (root / 'state.json').is_file()
        state = None
        stage = 'NO_TASK'
        task_id = args.task_id
        if present:
            runtime = open_store(args, authority)
            state = runtime.inspect()
            if task_id is None:
                active = state.get('active_task')
                task_id = active
            if task_id is not None and task_id in state['tasks']:
                stage = o.derive_stage(runtime, task_id)
        snapshot = o.publish_snapshot(plan_checkpoint=args.plan_checkpoint, operational_stage=stage,
                                      next_legal_action=args.next_action,
                                      model_availability=c.load_json_strict(args.availability or '{}'),
                                      health=args.health, blockers=args.blockers.split(',') if args.blockers else [],
                                      pending_escalation=args.escalation, runtime_present=present)
        _ = state
        return result('OK', snapshot=snapshot)
    except (c.ContractError, s.StoreError, o.OrchestratorError) as exc:
        return result('REFUSED', reason=str(exc))


def do_candidate(args: argparse.Namespace) -> dict:
    try:
        authority = load_authority(args)
        runtime = open_store(args, authority)
        task = c.load_json_strict(open_store_args_task(args, runtime, args.task_id))
        with runtime.lock('task', args.task_id):
            if runtime.inspect()['tasks'][args.task_id]['candidate_digest'] is not None:
                raise c.ContractError('candidate already registered; restore it without replacement')
            candidate = b.create_candidate(c.base_source_root(authority), Path(args.destination), task['base_sha'], authority=authority)
            digest = runtime.register_candidate(args.task_id, candidate)
        return result('OK', candidate_digest=digest, base_sha=task['base_sha'])
    except (c.ContractError, s.StoreError) as exc:
        return result('REFUSED', reason=str(exc))


def do_authorize(args: argparse.Namespace) -> dict:
    from . import promotion as p
    try:
        authority = load_authority(args)
        runtime = open_store(args, authority)
        with runtime.lock('task', args.task_id):
            inputs = p.collect_inputs(store=runtime, task_id=args.task_id, guard_root=Path(args.guard_root),
                attempt_dir=Path(args.attempt_dir), task_path=args.task_path, catalog_path=args.catalog_path,
                implementation_family=args.implementation_family)
            authorization = p.build_authorization(inputs, schemas=inputs['payload']['schemas'],
                implementation_family=args.implementation_family, issuance_sequence=args.sequence)
            digest = p.persist_authorization(runtime, args.task_id, authorization)
        return result('OK', authorization_digest=digest)
    except (c.ContractError, s.StoreError, p.PromotionError) as exc:
        return result('REFUSED', reason=str(exc))


def do_promote(args: argparse.Namespace) -> dict:
    from . import promotion as p
    try:
        authority = load_authority(args)
        runtime = open_store(args, authority)
        facts = c._release_authority(authority)
        if 'operational' not in facts:
            raise c.ContractError('operational CLI promotion requires external product task authority')
        promoted = p.promote(store=runtime, task_id=args.task_id, authorization_digest=args.authorization_digest,
            integration_repo=Path(args.integration_repo), remote='origin',
            expected_remote_url=facts['operational']['authorization']['destination_url'], hooks_dir=Path(args.hooks_dir))
        return result('OK' if promoted['status'] == 'PROMOTED' else 'PENDING', promotion=promoted)
    except (c.ContractError, s.StoreError, p.PromotionError) as exc:
        return result('REFUSED', reason=str(exc))


def do_handoff(args: argparse.Namespace) -> dict:
    import agent_supervisor
    try:
        authority = load_authority(args)
        runtime = open_store(args, authority)
        handoff = agent_supervisor.validate_task_handoff(Path(args.integration_repo),
            task_id=args.task_id, checkpoint_id=args.checkpoint, store_root=runtime.root,
            authorization_digest=args.authorization_digest, remote_receipt_digest=args.remote_receipt_digest,
            trusted_store=runtime)
        return result('OK', handoff=handoff)
    except (c.ContractError, s.StoreError, agent_supervisor.SupervisorError) as exc:
        return result('REFUSED', reason=str(exc))


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(prog='or-v2', description='V2 model orchestrator control CLI')
    parser.add_argument('--bootstrap', required=True, help='controller bootstrap JSON with exact record pins')
    parser.add_argument('--candidate-root', required=True)
    parser.add_argument('--controller-root', required=True)
    parser.add_argument('--product-root', help='independent clean immutable product base snapshot')
    parser.add_argument('--runtime', required=True, help='trusted runtime store root')
    sub = parser.add_subparsers(dest='verb', required=True)
    admit = sub.add_parser('admit')
    admit.add_argument('--task', required=True)
    admit.add_argument('--template', required=True)
    admit.set_defaults(func=do_admit)
    causal = sub.add_parser('approve-causal')
    causal.add_argument('--task-id', required=True)
    causal.add_argument('--diagnosis-controller', required=True)
    causal.add_argument('--diagnosis-bootstrap', required=True)
    causal.add_argument('--subsystem', choices=('worker', 'reviewer', 'source_reviewer'), default='worker')
    causal.set_defaults(func=do_approve_causal)
    claim = sub.add_parser('claim')
    claim.add_argument('--task-id', required=True)
    claim.add_argument('--owner', required=True)
    claim.add_argument('--boot', required=True)
    claim.add_argument('--stage-id', required=True)
    claim.add_argument('--stage-nonce', required=True)
    claim.add_argument('--launch', action='store_true')
    claim.add_argument('--enrollment')
    claim.add_argument('--enrollment-digest', help='optional exact pin; must match admitted task role_enrollment_ids')
    claim.add_argument('--task-class', default='IMPLEMENTATION')
    claim.add_argument('--required-reasoning', choices=a.REASONING_LEVELS, default='MEDIUM')
    claim.add_argument('--availability', default='{}')
    claim.add_argument('--locked-model')
    claim.add_argument('--opencode-bin')
    claim.add_argument('--opencode-sha')
    claim.add_argument('--opencode-version')
    claim.add_argument('--docker-bin')
    claim.add_argument('--docker-sha')
    claim.add_argument('--endpoint')
    claim.add_argument('--agent')
    claim.add_argument('--credential-dir')
    claim.add_argument('--prompt')
    claim.add_argument('--image')
    claim.add_argument('--limits', help='cpu,memory_bytes,pids,candidate_bytes,scratch_bytes,output_bytes,wall_seconds')
    claim.add_argument('--network', default='none')
    claim.add_argument('--container-binary', default='/usr/local/bin/opencode')
    claim.add_argument('--storage-root')
    claim.add_argument('--timeout', type=int, default=600)
    claim.set_defaults(func=do_claim)
    verify = sub.add_parser('verify')
    for name in ('task-id', 'task-path', 'catalog-path', 'guard-root', 'attempt-dir'):
        verify.add_argument('--' + name, required=True)
    verify.set_defaults(func=do_verify)
    review = sub.add_parser('review')
    review.add_argument('--task-id', required=True)
    review.add_argument('--task-path', required=True)
    review.add_argument('--catalog-path', required=True)
    review.add_argument('--guard-root', required=True)
    review.add_argument('--attempt-dir', required=True)
    review.add_argument('--enrollment', required=True)
    review.add_argument('--enrollment-digest', help='optional exact pin; must match admitted task role_enrollment_ids')
    review.add_argument('--task-class', default='INVESTIGATION_REVIEW')
    review.add_argument('--required-reasoning', choices=('HIGH', 'XHIGH', 'MAX'), default='HIGH')
    review.add_argument('--availability', default='{}')
    review.add_argument('--implementation-family', required=True)
    review.add_argument('--opencode-bin', required=True)
    review.add_argument('--opencode-sha', required=True)
    review.add_argument('--opencode-version', required=True)
    review.add_argument('--docker-bin', required=True)
    review.add_argument('--docker-sha', required=True)
    review.add_argument('--endpoint', required=True)
    review.add_argument('--agent')
    review.add_argument('--credential-dir')
    review.add_argument('--storage-root', required=True)
    review.add_argument('--image', required=True)
    review.add_argument('--limits', required=True)
    review.add_argument('--container-binary', default='/usr/local/bin/opencode')
    review.add_argument('--timeout', type=int, default=600)
    review.set_defaults(func=do_review)
    escalate = sub.add_parser('escalate')
    escalate.add_argument('--task-id', required=True)
    escalate.add_argument('--kind', required=True)
    escalate.add_argument('--failure-facts', required=True)
    escalate.add_argument('--invariants', default='')
    escalate.add_argument('--question', required=True)
    escalate.add_argument('--scope', required=True)
    escalate.add_argument('--codex-model')
    escalate.add_argument('--codex-bin')
    escalate.add_argument('--codex-sha')
    escalate.add_argument('--codex-version')
    escalate.add_argument('--codex-workdir')
    escalate.add_argument('--decision-schema')
    escalate.add_argument('--quota-signatures')
    escalate.add_argument('--codex-output-dir')
    escalate.add_argument('--prompt')
    escalate.set_defaults(func=do_escalate)
    recover = sub.add_parser('recover')
    recover.add_argument('--task-id', required=True)
    recover.add_argument('--docker-bin', required=True)
    recover.add_argument('--docker-sha', required=True)
    recover.add_argument('--endpoint', required=True)
    recover.add_argument('--guard-root')
    recover.add_argument('--attempt-dir')
    recover.add_argument('--task-path')
    recover.add_argument('--catalog-path')
    recover.set_defaults(func=do_recover)
    snapshot = sub.add_parser('snapshot')
    snapshot.add_argument('--task-id')
    snapshot.add_argument('--plan-checkpoint', required=True)
    snapshot.add_argument('--next-action', required=True)
    snapshot.add_argument('--availability', default='{}')
    snapshot.add_argument('--health', required=True)
    snapshot.add_argument('--blockers', default='')
    snapshot.add_argument('--escalation')
    snapshot.set_defaults(func=do_snapshot)
    candidate = sub.add_parser('candidate')
    candidate.add_argument('--task-id', required=True)
    candidate.add_argument('--destination', required=True)
    candidate.set_defaults(func=do_candidate)
    authorize = sub.add_parser('authorize')
    for name in ('task-id', 'guard-root', 'attempt-dir', 'task-path', 'catalog-path', 'implementation-family'):
        authorize.add_argument('--' + name, required=True)
    authorize.add_argument('--sequence', type=int, required=True)
    authorize.set_defaults(func=do_authorize)
    promote = sub.add_parser('promote')
    for name in ('task-id', 'authorization-digest', 'integration-repo', 'hooks-dir'):
        promote.add_argument('--' + name, required=True)
    promote.set_defaults(func=do_promote)
    handoff = sub.add_parser('handoff')
    for name in ('task-id', 'checkpoint', 'integration-repo', 'authorization-digest', 'remote-receipt-digest'):
        handoff.add_argument('--' + name, required=True)
    handoff.set_defaults(func=do_handoff)
    return parser


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    try:
        return emit(args.func(args))
    except BrokenPipeError:
        return EXIT_OK


if __name__ == '__main__':
    raise SystemExit(main())
