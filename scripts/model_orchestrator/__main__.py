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
    bootstrap = bootstrap_from_dict(load_json(args.bootstrap))
    return c.load_release_authority(Path(args.candidate_root), Path(args.controller_root), bootstrap=bootstrap)


def open_store(args: argparse.Namespace, authority: c.ValidatedReleaseAuthority) -> s.RuntimeStore:
    return s.RuntimeStore(Path(args.runtime), authority)


def opencode_binary(args: argparse.Namespace) -> a.OpenCodeBinary:
    return a.OpenCodeBinary(Path(args.opencode_bin), args.opencode_sha, args.opencode_version)


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
            enrollment = a.validate_enrollment(load_json(args.enrollment))
            worker = o.select_worker(full_task, [enrollment], availability=json.loads(args.availability or '{}'),
                                     locked_model=args.locked_model)
            enrollment = dict(enrollment, reasoning_requested=worker['reasoning_requested'])
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
                                   storage_root=Path(args.storage_root), timeout_seconds=args.timeout)
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
                                'ADAPTER_UNAVAILABLE', 'MODEL_UNAVAILABLE', 'TRANSPORT_ERROR', 'PROTOCOL_ERROR') else 'REFUSED'
        if code in ('ADAPTER_UNAVAILABLE', 'MODEL_UNAVAILABLE', 'TRANSPORT_ERROR'):
            code = 'UNAVAILABLE'
        return result(code, reason=str(exc))


def open_store_args_task(args: argparse.Namespace, runtime: s.RuntimeStore, task_id: str) -> str:
    state = runtime.inspect()
    digest = state['tasks'][task_id]['contract_digest']
    record = s._object(s._read(runtime.root / 'objects' / (digest + '.json')))
    return c.canonical_json(record['payload'])


def do_review(args: argparse.Namespace) -> dict:
    try:
        authority = load_authority(args)
        runtime = open_store(args, authority)
        floor = g.load_floor(authority, args.task_path, args.catalog_path)
        guarded = g.restore_guarded(floor, Path(args.guard_root))
        with runtime.lock('task', args.task_id):
            o.claim_review(runtime, args.task_id)
            enrollment = a.validate_enrollment(load_json(args.enrollment))
            payload = c._release_authority(authority)
            full_task = floor.task
            reviewer = o.select_reviewer(full_task, [enrollment], availability=json.loads(args.availability or '{}'),
                                         implementation_family=args.implementation_family)
            enrollment = dict(enrollment, reasoning_requested=reviewer['reasoning_requested'])
            daemon_box = b.ContainerSandbox(b.DockerCLI(Path(args.docker_bin), args.docker_sha, endpoint=args.endpoint),
                                            boot_identity=b.host_boot_identity(), host_platform='linux')
            outcome = o.run_reviewer(authority=authority, floor=floor, guarded=guarded,
                                     attempt_dir=Path(args.attempt_dir), enrollment=enrollment,
                                     binary=opencode_binary(args), agent=args.agent, box=daemon_box,
                                     image=args.image, container_binary=args.container_binary,
                                     implementation_family=args.implementation_family,
                                     limits=b.Limits(*[int(x) for x in args.limits.split(',')]),
                                     timeout_seconds=args.timeout)
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
                                      model_availability=json.loads(args.availability or '{}'),
                                      health=args.health, blockers=args.blockers.split(',') if args.blockers else [],
                                      pending_escalation=args.escalation, runtime_present=present)
        _ = state
        return result('OK', snapshot=snapshot)
    except (c.ContractError, s.StoreError, o.OrchestratorError) as exc:
        return result('REFUSED', reason=str(exc))


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(prog='or-v2', description='V2 model orchestrator control CLI')
    parser.add_argument('--bootstrap', required=True, help='controller bootstrap JSON with exact record pins')
    parser.add_argument('--candidate-root', required=True)
    parser.add_argument('--controller-root', required=True)
    parser.add_argument('--runtime', required=True, help='trusted runtime store root')
    sub = parser.add_subparsers(dest='verb', required=True)
    admit = sub.add_parser('admit')
    admit.add_argument('--task', required=True)
    admit.add_argument('--template', required=True)
    admit.set_defaults(func=do_admit)
    claim = sub.add_parser('claim')
    claim.add_argument('--task-id', required=True)
    claim.add_argument('--owner', required=True)
    claim.add_argument('--boot', required=True)
    claim.add_argument('--stage-id', required=True)
    claim.add_argument('--stage-nonce', required=True)
    claim.add_argument('--launch', action='store_true')
    claim.add_argument('--enrollment')
    claim.add_argument('--availability', default='{}')
    claim.add_argument('--locked-model')
    claim.add_argument('--opencode-bin')
    claim.add_argument('--opencode-sha')
    claim.add_argument('--opencode-version')
    claim.add_argument('--docker-bin')
    claim.add_argument('--docker-sha')
    claim.add_argument('--endpoint')
    claim.add_argument('--agent', default='orch-worker')
    claim.add_argument('--prompt')
    claim.add_argument('--image')
    claim.add_argument('--limits', help='cpu,memory_bytes,pids,candidate_bytes,scratch_bytes,output_bytes,wall_seconds')
    claim.add_argument('--network', default='none')
    claim.add_argument('--container-binary', default='/usr/local/bin/opencode')
    claim.add_argument('--storage-root')
    claim.add_argument('--timeout', type=int, default=600)
    claim.set_defaults(func=do_claim)
    review = sub.add_parser('review')
    review.add_argument('--task-id', required=True)
    review.add_argument('--task-path', required=True)
    review.add_argument('--catalog-path', required=True)
    review.add_argument('--guard-root', required=True)
    review.add_argument('--attempt-dir', required=True)
    review.add_argument('--enrollment', required=True)
    review.add_argument('--availability', default='{}')
    review.add_argument('--implementation-family', required=True)
    review.add_argument('--opencode-bin', required=True)
    review.add_argument('--opencode-sha', required=True)
    review.add_argument('--opencode-version', required=True)
    review.add_argument('--docker-bin', required=True)
    review.add_argument('--docker-sha', required=True)
    review.add_argument('--endpoint', required=True)
    review.add_argument('--agent', default='orch-reviewer')
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
    return parser


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    try:
        return emit(args.func(args))
    except BrokenPipeError:
        return EXIT_OK


if __name__ == '__main__':
    raise SystemExit(main())
