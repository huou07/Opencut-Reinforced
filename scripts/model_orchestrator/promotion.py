"""M4 durable promotion authorization and exact-base remote/local recovery.

No caller assertion confers authority: every binding is revalidated against
controller-owned store objects, and the remote is queried freshly,
independently of stale tracking refs. No force, no rebase, no optional task
identity, no `{ok:true}` bypass. A single non-force push per intent; recovery
never pushes twice.
"""
from __future__ import annotations

import os
from pathlib import Path
import re
import subprocess
import uuid
from typing import Any, Callable, Mapping, Sequence

from . import contracts as c
from . import store as s
from . import push_guard


class PromotionError(c.ContractError):
    def __init__(self, message: str, code: str = 'REFUSED'):
        self.code = code
        super().__init__(message)


def _refuse(condition: bool, message: str, code: str = 'REFUSED') -> None:
    if not condition:
        raise PromotionError(message, code)


def load_object(store: s.RuntimeStore, digest: str) -> dict:
    _refuse(type(digest) is str and re.fullmatch(r'[0-9a-f]{64}', digest) is not None, 'invalid object digest')
    try:
        record = s._object(s._read(store.root / 'objects' / (digest + '.json')))
    except s.StoreError as exc:
        raise PromotionError('missing or unreadable controller object: ' + str(exc)) from exc
    _refuse(c.canonical_digest(record) == digest, 'immutable object identity mismatch')
    return record


def collect_inputs(*, store: s.RuntimeStore, task_id: str, guard_root: Path,
                   attempt_dir: Path, task_path: str, catalog_path: str,
                   implementation_family: str) -> dict[str, Any]:
    """Load every authorization input from controller-owned locations only."""
    from . import guards as g
    from . import verification as v
    _refuse(type(store) is s.RuntimeStore, 'trusted runtime store required')
    state = store.inspect()
    task_state = state['tasks'].get(task_id)
    _refuse(task_state is not None and task_state['status'] == 'SETTLED', 'promotion requires a settled task')
    contract = load_object(store, task_state['contract_digest'])
    _refuse(contract['kind'] == 'task_contract' and contract['payload'].get('task_id') == task_id, 'task pointer is not the original contract')
    task = contract['payload']
    authority = store.authority
    payload = c.validate_phase_admission(authority, task, capability='M4')
    floor = g.load_floor(authority, task_path, catalog_path)
    _refuse(c.canonical_digest(floor.task) == c.canonical_digest(task), 'floor task differs from admitted task')
    guarded = g.restore_guarded(floor, Path(guard_root))
    guard = guarded.verify()
    _refuse(not guard['vetoes'], 'deterministic guard veto prevents promotion')
    readiness = v.readiness(guarded, Path(attempt_dir))
    _refuse(readiness['review_ready'] is True, 'promotion requires passed verification with no non-semantic blockers')
    reviews = [load_object(store, digest) for digest in state['object_digests']]
    reviews = [r['payload'] for r in reviews if r['kind'] == 'review' and r['payload'].get('task_id') == task_id
               and r['payload'].get('report',{}).get('candidate_sha') == guard['head']]
    _refuse(len(reviews) == 1, 'promotion requires exactly one persisted independent review for the exact candidate')
    parsed = _validated_review(reviews[0], task=task, guard=guard, schemas=floor.verify()['schemas'],
                               implementation_family=implementation_family)
    readiness = dict(readiness, acceptance_ready=True, unresolved=[],
                     quality_flag_dispositions=parsed['quality_flag_dispositions'])
    return {'task': task, 'payload': payload, 'guard': guard, 'readiness': readiness,
            'review': reviews[0], 'lease_epoch': task_state['lease_epoch']}


def _validated_review(review, *, task, guard, schemas, implementation_family):
    _refuse(type(implementation_family) is str and implementation_family and
            implementation_family != 'unknown', 'implementation family required')
    report = review.get('report', review)
    metadata = review.get('metadata', {})
    reviewer_family = metadata.get('family', review.get('family', 'unknown'))
    from . import adapters as adapters_module, verification as verification_module
    parsed = adapters_module.parse_review_report(report, task=task, candidate_sha=guard['head'], schemas=schemas,
                                                 reviewer_family=reviewer_family,
                                                 implementation_family=implementation_family,
                                                 flag_ids=[verification_module.quality_flag_id(flag) for flag in guard['flags']])
    _refuse(parsed['verdict'] == 'PASS', 'a non-PASS review cannot issue promotion authority')
    dispositions = {item['id']: item['disposition'] for item in parsed['quality_flag_dispositions']}
    undisposed = [flag for flag in guard['flags'] if dispositions.get(verification_module.quality_flag_id(flag)) != 'NOT_LOWERING']
    _refuse(not undisposed, 'guard flags lack reviewer disposition: ' + ', '.join(undisposed[:5]))
    return parsed


def build_authorization(inputs: Mapping[str, Any], *, schemas: Mapping[str, Any],
                        implementation_family: str, issuance_sequence: int) -> dict[str, Any]:
    """Assemble and fully revalidate the durable promotion authorization."""
    task = inputs['task']
    guard = inputs['guard']
    readiness = inputs['readiness']
    review = inputs['review']
    _refuse(type(implementation_family) is str and implementation_family and implementation_family != 'unknown', 'implementation family required')
    _refuse(type(issuance_sequence) is int and issuance_sequence >= 1, 'invalid issuance sequence')
    head = guard['head']
    _refuse(head != task['base_sha'], 'empty candidate cannot be promoted')
    candidate_receipt = {'schema_version': 1, 'task_id': task['task_id'], 'candidate_sha': head,
                         'base_sha': task['base_sha'], 'tree_digest': guard['tree_digest'], 'imported': True,
                         'clean_product_tree': True, 'guard_receipt_digest': c.canonical_digest(guard),
                         'authority_digest': task['authority_digest'],
                         'task_contract_digest': c.canonical_digest(task)}
    c.validate_record(candidate_receipt, 'candidate_receipt', schemas,
                      context={'task_id': task['task_id'], 'candidate_sha': head, 'base_sha': task['base_sha'],
                               'tree_digest': guard['tree_digest'], 'guard_receipt_digest': c.canonical_digest(guard),
                               'authority_digest': task['authority_digest'],
                               'task_contract_digest': c.canonical_digest(task)})
    verified = []
    for receipt in readiness['receipts']:
        check = next((x for x in task['check_argv'] if x['id'] == receipt['check_id']), None)
        _refuse(check is not None, 'receipt references an unknown check')
        context = {'frozen_task': dict(task), 'task_id': task['task_id'], 'candidate_sha': head,
                   'authority_digest': task['authority_digest'],
                   'task_contract_digest': c.canonical_digest(task), 'command_digest': c.canonical_digest(check),
                   'environment_digest': check['environment_digest'], 'lease_epoch': receipt['lease_epoch'],
                   'sequence': receipt['sequence'], 'stage_nonce': receipt['stage_nonce']}
        _refuse(receipt['lease_epoch'] == inputs['lease_epoch'], 'receipt lease epoch differs from claim')
        c.validate_verification_receipt(receipt, schemas, task=task, candidate_sha=head, context=context)
        _refuse(receipt['outcome'] == 'PASS', 'a failed required check cannot authorize promotion')
        verified.append(receipt)
    _refuse(set(r['check_id'] for r in verified) == set(task['required_check_ids']) and
            len(verified) == len(task['required_check_ids']), 'incomplete or duplicate required check receipts')
    _refuse(not guard['vetoes'], 'deterministic guard veto prevents promotion')
    _refuse(readiness['verification_passed'] is True and
            not [reason for reason in readiness['unresolved'] if reason not in guard['flags']],
            'promotion requires passed verification with no non-semantic blockers')
    _validated_review(review, task=task, guard=guard, schemas=schemas,
                      implementation_family=implementation_family)
    authorization = {'schema_version': 1, 'task_id': task['task_id'], 'checkpoint_id': task['checkpoint_id'],
                     'base_sha': task['base_sha'], 'candidate_sha': head,
                     'authority_digest': task['authority_digest'], 'template_digest': task['template_digest'],
                     'candidate_receipt_digest': c.canonical_digest(candidate_receipt),
                     'verification_receipt_digests': sorted(c.canonical_digest(r) for r in verified),
                     'reviewer_receipt_digest': c.canonical_digest(review),
                     'pending_hosted_classes': list(readiness['pending_hosted_classes']),
                     'lease_epoch': inputs['lease_epoch'], 'issuance_sequence': issuance_sequence}
    context = {'task_id': authorization['task_id'], 'checkpoint_id': authorization['checkpoint_id'],
               'base_sha': authorization['base_sha'], 'candidate_sha': authorization['candidate_sha'],
               'authority_digest': authorization['authority_digest'], 'template_digest': authorization['template_digest'],
               'candidate_receipt_digest': authorization['candidate_receipt_digest'],
               'verification_receipt_digests': sorted(authorization['verification_receipt_digests']),
               'reviewer_receipt_digest': authorization['reviewer_receipt_digest'],
               'lease_epoch': authorization['lease_epoch'], 'issuance_sequence': authorization['issuance_sequence']}
    c.validate_record(authorization, 'promotion_authorization', schemas, context=context)
    return authorization


def persist_authorization(store: s.RuntimeStore, task_id: str, authorization: Mapping[str, Any]) -> str:
    _refuse(type(store) is s.RuntimeStore, 'trusted runtime store required')
    current = store.inspect()
    _refuse(task_id in current['tasks'], 'unknown task')
    record = dict(schema_version=1, kind='authorization', payload=dict(schema_version=1, task_id=task_id, authorization=dict(authorization)))
    digest = c.canonical_digest(record)
    prior = [d for d in current['object_digests']
             if s._object(s._read(store.root / 'objects' / (d + '.json')))['kind'] == 'authorization']

    def update(state: dict) -> None:
        pass

    result = store.transaction(current['sequence'], current['epoch'], update, objects=[record])
    _refuse(digest in result['object_digests'], 'authorization object not published')
    _refuse(len(prior) + 1 == sum(1 for d in result['object_digests']
                                  if s._object(s._read(store.root / 'objects' / (d + '.json')))['kind'] == 'authorization'),
            'authorization issuance sequence differs')
    return digest


def _latest_authorization_sequence(store: s.RuntimeStore, task_id: str) -> int:
    current = store.inspect()
    sequences = []
    for digest in current['object_digests']:
        record = load_object(store, digest)
        if record['kind'] == 'authorization' and record['payload'].get('task_id') == task_id:
            sequences.append(record['payload']['authorization'].get('issuance_sequence', 0))
    _refuse(sequences, 'no promotion authorization for task')
    return max(sequences)


def load_authorization(store: s.RuntimeStore, digest: str) -> dict[str, Any]:
    record = load_object(store, digest)
    _refuse(record['kind'] == 'authorization', 'object is not a promotion authorization')
    authorization = record['payload']['authorization']
    _refuse(type(authorization) is dict and authorization.get('schema_version') == 1, 'invalid authorization payload')
    return authorization


def _git(repo: Path, *args: str, input_bytes: bytes | None = None) -> bytes:
    env = {'PATH': os.environ.get('PATH', ''), 'LANG': 'C', 'LC_ALL': 'C', 'GIT_CONFIG_NOSYSTEM': '1',
           'GIT_CONFIG_GLOBAL': os.devnull, 'GIT_NO_REPLACE_OBJECTS': '1', 'GIT_TERMINAL_PROMPT': '0'}
    try:
        result = subprocess.run(['git', '--no-optional-locks', '-c', 'core.hooksPath=' + os.devnull, *args],
                                cwd=repo, env=env, input=input_bytes, capture_output=True, timeout=120, check=False)
    except (OSError, subprocess.SubprocessError) as exc:
        raise PromotionError('integration git failed: ' + args[0], 'UNAVAILABLE') from exc
    return result


def _git_ok(repo: Path, *args: str) -> str:
    result = _git(repo, *args)
    _refuse(result.returncode == 0, 'integration git rejected ' + args[0], 'UNAVAILABLE')
    _refuse(len(result.stdout) <= 4 << 20, 'integration git output bound exceeded')
    try:
        return result.stdout.decode('utf-8').strip()
    except UnicodeError as exc:
        raise PromotionError('invalid integration git output') from exc


def advertised_main(repo: Path, remote: str) -> str | None:
    """Fresh remote advertisement, independent of stale tracking refs."""
    result = _git(repo, 'ls-remote', remote, 'refs/heads/main')
    _refuse(result.returncode == 0, 'remote advertisement unavailable', 'UNAVAILABLE')
    lines = result.stdout.decode('utf-8', 'replace').splitlines()
    _refuse(len(lines) <= 1, 'ambiguous remote advertisement')
    if not lines:
        return None
    oid, _, ref = lines[0].partition('\t')
    _refuse(ref.strip() == 'refs/heads/main' and re.fullmatch(r'[0-9a-f]{40}', oid.strip()) is not None, 'malformed remote advertisement')
    return oid.strip()


def check_integration_repo(repo: Path, remote: str, expected_url: str, base_sha: str) -> dict[str, str]:
    """Clean main at the authorized base, pinned origin identity, no candidate code executed."""
    repo = Path(repo)
    _refuse(repo.is_dir() and (repo / '.git').is_dir() and not (repo / '.git').is_symlink(), 'integration repository required')
    _refuse(not _git_ok(repo, 'status', '--porcelain=v1', '--untracked-files=all'), 'dirty integration tree')
    url = _git_ok(repo, 'config', '--get', 'remote.' + remote + '.url')
    _refuse(url == expected_url, 'integration remote identity differs')
    head = _git_ok(repo, 'rev-parse', 'HEAD')
    branch = _git_ok(repo, 'rev-parse', '--abbrev-ref', 'HEAD')
    _refuse(head == base_sha and branch == 'main', 'integration main is not the clean authorized base')
    _refuse(_git_ok(repo, 'cat-file', '-t', base_sha) == 'commit', 'authorized base is not a commit')
    return {'head': head, 'branch': branch, 'url': url}


def check_candidate(repo: Path, base_sha: str, candidate_sha: str) -> None:
    _refuse(candidate_sha != base_sha, 'empty candidate cannot be promoted')
    _refuse(_git_ok(repo, 'cat-file', '-t', candidate_sha) == 'commit', 'candidate is not a commit')
    result = _git(repo, 'merge-base', '--is-ancestor', base_sha, candidate_sha)
    _refuse(result.returncode == 0, 'candidate does not descend from the authorized base')


INTENT_OUTCOMES = ('PROMOTING', 'REMOTE_PROMOTED', 'LOCAL_SYNC_PENDING', 'PROMOTED',
                   'UNCOMMITTED', 'DIAGNOSTIC')
INTENT_TRANSITIONS = {'PROMOTING': ('REMOTE_PROMOTED', 'UNCOMMITTED', 'DIAGNOSTIC'),
                      'REMOTE_PROMOTED': ('PROMOTED', 'LOCAL_SYNC_PENDING'),
                      'LOCAL_SYNC_PENDING': ('PROMOTED', 'LOCAL_SYNC_PENDING')}
FINAL_OUTCOMES = ('PROMOTED', 'UNCOMMITTED', 'DIAGNOSTIC')


def persist_intent(store: s.RuntimeStore, intent: Mapping[str, Any]) -> str:
    _refuse(set(intent) == {'schema_version', 'kind', 'task_id', 'authorization_digest', 'base_sha', 'candidate_sha',
                            'destination_ref', 'remote_url', 'intent_nonce', 'outcome'},
            'invalid promotion intent fields')
    _refuse(intent.get('schema_version') == 1 and intent.get('kind') == 'promotion-intent'
            and intent.get('outcome') == 'PROMOTING', 'invalid promotion intent')
    current = store.inspect()
    record = dict(schema_version=1, kind='receipt', payload=dict(intent))

    def update(state: dict) -> None:
        pass

    result = store.transaction(current['sequence'], current['epoch'], update, objects=[record])
    digest = c.canonical_digest(record)
    _refuse(digest in result['object_digests'], 'promotion intent not published')
    return digest


def load_intent(store: s.RuntimeStore, digest: str) -> dict[str, Any]:
    record = load_object(store, digest)
    _refuse(record['kind'] == 'receipt' and record['payload'].get('kind') == 'promotion-intent', 'object is not a promotion intent')
    return record['payload']


def _intent_groups(store: s.RuntimeStore, task_id: str) -> list:
    """Intent objects grouped by nonce in publication order; outcomes append."""
    current = store.inspect()
    groups: list = []
    order: dict = {}
    for digest in current['object_digests']:
        record = load_object(store, digest)
        if record['kind'] == 'receipt' and record['payload'].get('kind') == 'promotion-intent' \
                and record['payload'].get('task_id') == task_id:
            nonce = record['payload']['intent_nonce']
            if nonce not in order:
                order[nonce] = len(groups)
                groups.append((nonce, []))
            groups[order[nonce]][1].append((digest, record['payload']))
    return groups


def _latest_intent(store: s.RuntimeStore, task_id: str) -> tuple[str, dict[str, Any]]:
    """Latest intent object overall, for inspection only."""
    groups = _intent_groups(store, task_id)
    _refuse(groups, 'no promotion intent for task')
    return groups[-1][1][-1]


def _resolve_intent(store: s.RuntimeStore, task_id: str, intent_digest: str) -> tuple[str, dict[str, Any]]:
    """Resolve the caller's intent to its group's latest outcome.

    A newer explicit promotion (different nonce, later publication) supersedes
    the caller's intent and refuses. Outcome updates within the same nonce
    group never supersede.
    """
    groups = _intent_groups(store, task_id)
    _refuse(groups, 'no promotion intent for task')
    ours = None
    for index, (nonce, members) in enumerate(groups):
        if any(digest == intent_digest for digest, _ in members):
            ours = (index, members[-1])
    _refuse(ours is not None, 'unknown promotion intent')
    index, (digest, payload) = ours
    _refuse(index == len(groups) - 1, 'intent superseded by a newer explicit promotion')
    return digest, payload


def promote(*, store: s.RuntimeStore, task_id: str, authorization_digest: str, integration_repo: Path,
            remote: str, expected_remote_url: str, hooks_dir: Path,
            push_runner: Callable[[list[str], Path], tuple[int, str, str]] | None = None,
            fault: Callable[[str], None] | None = None) -> dict[str, Any]:
    """Durable exact-base non-force promotion with crash-safe reconciliation."""
    _refuse(type(store) is s.RuntimeStore, 'trusted runtime store required')
    _refuse(type(task_id) is str and re.fullmatch(r'[A-Za-z0-9_-]{1,64}', task_id) is not None, 'task identity required; no optional task path')
    _refuse(type(remote) is str and re.fullmatch(r'[A-Za-z0-9_.-]{1,64}', remote) is not None, 'invalid remote name')
    hit = fault or (lambda point: None)
    with store.lock('task', task_id):
        with store.lock('integration'):
            current = store.inspect()
            _refuse(current['paused'] is False, 'paused controller promotes nothing', 'CONFLICT')
            task_state = current['tasks'].get(task_id)
            _refuse(task_state is not None and task_state['status'] == 'SETTLED', 'promotion requires a settled task')
            authorization = load_authorization(store, authorization_digest)
            contract = load_object(store, task_state['contract_digest'])
            task = contract['payload']
            c.validate_phase_admission(store.authority, task, capability='M4')
            _refuse(authorization['task_id'] == task_id and authorization['base_sha'] == task['base_sha']
                    and authorization['authority_digest'] == task['authority_digest'], 'authorization binds a different task')
            _refuse(authorization['lease_epoch'] == task_state['lease_epoch'], 'stale or future authorization epoch')
            _refuse(authorization['issuance_sequence'] == _latest_authorization_sequence(store, task_id),
                    'superseded authorization issuance')
            authority = c._release_authority(store.authority)
            if 'operational' in authority and expected_remote_url != authority['operational']['authorization']['destination_url']:
                raise c.ContractError('promotion destination differs from operator product authorization')
            repo = Path(integration_repo)
            topology = check_integration_repo(repo, remote, expected_remote_url, authorization['base_sha'])
            _refuse(topology['head'] == authorization['base_sha'], 'integration main moved')
            check_candidate(repo, authorization['base_sha'], authorization['candidate_sha'])
            fresh = advertised_main(repo, remote)
            _refuse(fresh == authorization['base_sha'], 'remote is not the authorized base')
            nonce = uuid.uuid4().hex
            intent = {'schema_version': 1, 'kind': 'promotion-intent', 'task_id': task_id,
                      'authorization_digest': authorization_digest, 'base_sha': authorization['base_sha'],
                      'candidate_sha': authorization['candidate_sha'], 'destination_ref': 'refs/heads/main',
                      'remote_url': expected_remote_url, 'intent_nonce': nonce, 'outcome': 'PROMOTING'}
            intent_digest = persist_intent(store, intent)
            hit('intent_persisted')
            push_guard.install_push_guard(repo, expected_base=authorization['base_sha'],
                                          expected_candidate=authorization['candidate_sha'],
                                          expected_remote_url=expected_remote_url, hooks_dir=Path(hooks_dir))
            argv = ['push', remote, authorization['candidate_sha'] + ':refs/heads/main']
            _refuse('--force' not in argv and '--force-with-lease' not in argv, 'force push is never promotion')
            hit('before_push')
            runner = push_runner or (lambda command, cwd: _run_push(command, cwd))
            # Serialize the irreversible dispatch with pause publication. An
            # earlier pause (including pause/resume) invalidates this intent.
            with store.lock('state'):
                fresh_state = store.inspect()
                _refuse(fresh_state['paused'] is False
                        and fresh_state['pause_generation'] == current['pause_generation']
                        and fresh_state['tasks'].get(task_id) == task_state,
                        'paused or stale promotion before push', 'CONFLICT')
                code, stdout, stderr = runner(argv, repo)
            hit('after_push')
            return _reconcile_push_inner(store=store, task_id=task_id, intent_digest=intent_digest,
                                         integration_repo=repo, remote=remote, push_observed=(code == 0),
                                         push_alive=False, push_output=stdout + stderr, fault=fault)


def _run_push(argv: list[str], cwd: Path) -> tuple[int, str, str]:
    env = {'PATH': os.environ.get('PATH', ''), 'LANG': 'C', 'LC_ALL': 'C', 'GIT_CONFIG_NOSYSTEM': '1',
           'GIT_CONFIG_GLOBAL': os.devnull, 'GIT_NO_REPLACE_OBJECTS': '1', 'GIT_TERMINAL_PROMPT': '0'}
    try:
        result = subprocess.run(['git', '--no-optional-locks', *argv], cwd=cwd, env=env,
                                capture_output=True, timeout=300, check=False)
    except (OSError, subprocess.SubprocessError) as exc:
        raise PromotionError('promotion push unavailable: ' + str(exc), 'UNAVAILABLE') from exc
    _refuse(len(result.stdout) + len(result.stderr) <= 4 << 20, 'push output bound exceeded')
    return result.returncode, result.stdout.decode('utf-8', 'replace'), result.stderr.decode('utf-8', 'replace')


def reconcile_push(*, store: s.RuntimeStore, task_id: str, intent_digest: str, integration_repo: Path,
                   remote: str, push_observed: bool, push_alive: bool = False, push_output: str = '',
                   fault: Callable[[str], None] | None = None) -> dict[str, Any]:
    """Reconcile the durable intent against the real remote. Never pushes twice."""
    with store.lock('task', task_id):
        with store.lock('integration'):
            return _reconcile_push_inner(store=store, task_id=task_id, intent_digest=intent_digest,
                                         integration_repo=integration_repo, remote=remote,
                                         push_observed=push_observed, push_alive=push_alive,
                                         push_output=push_output, fault=fault)


def _reconcile_push_inner(*, store: s.RuntimeStore, task_id: str, intent_digest: str, integration_repo: Path,
                          remote: str, push_observed: bool, push_alive: bool = False, push_output: str = '',
                          fault: Callable[[str], None] | None = None) -> dict[str, Any]:
    """Inner reconciliation; the caller holds the task and integration locks.

    Operates on the latest intent for the task: a superseded digest refuses,
    and an already-final outcome returns its status without side effects.
    `push_alive` must be true only when the push process may still live; a live
    push with a base-observed remote stays PROMOTING instead of risking a
    second push.
    """
    hit = fault or (lambda point: None)
    latest_digest, intent = _resolve_intent(store, task_id, intent_digest)
    repo = Path(integration_repo)
    if intent['outcome'] in FINAL_OUTCOMES:
        return {'status': intent['outcome'] if intent['outcome'] != 'UNCOMMITTED' else 'PROMOTION_READY',
                'intent_digest': latest_digest, 'note': 'already reconciled; no second push'}
    if intent['outcome'] in ('REMOTE_PROMOTED', 'LOCAL_SYNC_PENDING'):
        return _reconcile_sync_inner(store=store, task_id=task_id, intent_digest=intent_digest,
        integration_repo=repo, fault=fault)
    _refuse(intent['outcome'] == 'PROMOTING', 'intent is not awaiting push reconciliation')
    repo = Path(integration_repo)
    try:
        observed = advertised_main(repo, remote)
    except PromotionError as exc:
        if exc.code == 'UNAVAILABLE':
            return {'status': 'PROMOTING', 'availability': 'remote-unreachable', 'intent_digest': latest_digest}
        raise
    hit('observed')
    if observed == intent['candidate_sha']:
        receipt = {'schema_version': 1, 'task_id': task_id, 'authorization_digest': intent['authorization_digest'],
                   'destination_ref': 'refs/heads/main', 'base_sha': intent['base_sha'],
                   'candidate_sha': intent['candidate_sha'], 'observed_remote_sha': observed,
                   'intent_nonce': intent['intent_nonce']}
        payload = c._release_authority(store.authority)
        c.validate_record(receipt, 'remote_promotion_receipt', payload['schemas'],
                          context={'task_id': task_id, 'authorization_digest': intent['authorization_digest'],
                                   'base_sha': intent['base_sha'], 'candidate_sha': intent['candidate_sha']})
        record = dict(schema_version=1, kind='receipt',
                      payload=dict(schema_version=1, kind='remote-promotion', receipt=receipt))
        current = store.inspect()

        def update(state: dict) -> None:
            pass

        result = store.transaction(current['sequence'], current['epoch'], update, objects=[record])
        receipt_digest = c.canonical_digest(record)
        _refuse(receipt_digest in result['object_digests'], 'remote receipt not published')
        _set_intent_outcome(store, latest_digest, 'REMOTE_PROMOTED')
        hit('receipted')
        return _reconcile_sync_inner(store=store, task_id=task_id, intent_digest=intent_digest,
                                     integration_repo=repo, fault=fault)
    if observed == intent['base_sha']:
        _refuse(push_observed is False, 'push claimed success but remote is still the base')
        _refuse(push_alive is False, 'push process may still live; outcome unknown while remote is base')
        _set_intent_outcome(store, latest_digest, 'UNCOMMITTED')
        return {'status': 'PROMOTION_READY', 'intent_digest': latest_digest,
                'note': 'intent not observed committed; another push needs explicit authorization'}
    _set_intent_outcome(store, latest_digest, 'DIAGNOSTIC')
    return {'status': 'DIAGNOSTIC', 'intent_digest': latest_digest,
            'observed_remote_sha': observed, 'note': 'remote advanced or conflicted; no retry/rebase/force'}


def _set_intent_outcome(store: s.RuntimeStore, intent_digest: str, outcome: str) -> None:
    _refuse(outcome in INTENT_OUTCOMES, 'invalid intent outcome')
    current = store.inspect()
    record = load_object(store, intent_digest)
    payload = dict(record['payload'])
    previous = payload.get('outcome')
    _refuse(previous in INTENT_TRANSITIONS and outcome in INTENT_TRANSITIONS[previous],
            'illegal intent outcome transition: %s -> %s' % (previous, outcome))
    payload['outcome'] = outcome

    def update(state: dict) -> None:
        pass

    updated = dict(record, payload=payload)
    store.transaction(current['sequence'], current['epoch'], update, objects=[updated])
    _refuse(c.canonical_digest(updated) in store.inspect()['object_digests'], 'intent outcome not published')


def reconcile_sync(*, store: s.RuntimeStore, task_id: str, intent_digest: str, integration_repo: Path,
                   fault: Callable[[str], None] | None = None) -> dict[str, Any]:
    """Align local main ff-only; dirty/diverged local state blocks without reset."""
    with store.lock('task', task_id):
        with store.lock('integration'):
            return _reconcile_sync_inner(store=store, task_id=task_id, intent_digest=intent_digest,
                                         integration_repo=integration_repo, fault=fault)


def _reconcile_sync_inner(*, store: s.RuntimeStore, task_id: str, intent_digest: str, integration_repo: Path,
                          fault: Callable[[str], None] | None = None) -> dict[str, Any]:
    """Inner sync; the caller holds the task and integration locks."""
    hit = fault or (lambda point: None)
    latest_digest, intent = _resolve_intent(store, task_id, intent_digest)
    if intent['outcome'] == 'PROMOTED':
        return {'status': 'PROMOTED', 'intent_digest': latest_digest, 'note': 'already reconciled'}
    _refuse(intent['outcome'] in ('REMOTE_PROMOTED', 'LOCAL_SYNC_PENDING'), 'sync requires a promoted remote')
    repo = Path(integration_repo)
    dirty = _git_ok(repo, 'status', '--porcelain=v1', '--untracked-files=all')
    local = _git_ok(repo, 'rev-parse', 'HEAD')
    branch = _git_ok(repo, 'rev-parse', '--abbrev-ref', 'HEAD')
    _refuse(branch == 'main', 'integration branch moved')
    hit('inspected')
    if dirty:
        _set_intent_outcome(store, latest_digest, 'LOCAL_SYNC_PENDING')
        return {'status': 'LOCAL_SYNC_PENDING', 'intent_digest': latest_digest,
                'local_head': local, 'note': 'operator resolves local state; no reset/clean/discard'}
    if local == intent['candidate_sha']:
        _set_intent_outcome(store, latest_digest, 'PROMOTED')
        return {'status': 'PROMOTED', 'intent_digest': latest_digest}
    _refuse(local == intent['base_sha'], 'local main diverged; reconcile without reset')
    moved = _git(repo, 'merge', '--ff-only', intent['candidate_sha'])
    if moved.returncode != 0:
        _set_intent_outcome(store, latest_digest, 'LOCAL_SYNC_PENDING')
        return {'status': 'LOCAL_SYNC_PENDING', 'intent_digest': latest_digest,
                'local_head': local, 'note': 'fast-forward refused; operator resolves without reset'}
    hit('fast_forwarded')
    _refuse(_git_ok(repo, 'rev-parse', 'HEAD') == intent['candidate_sha'], 'local alignment unverified')
    _set_intent_outcome(store, latest_digest, 'PROMOTED')
    return {'status': 'PROMOTED', 'intent_digest': latest_digest}


def build_control_plane_receipt(*, store: s.RuntimeStore, task_id: str, authorization_digest: str,
                                remote_receipt_digest: str, guard_digest: str,
                                adoption_manifest_digest: str, acceptance_contract_digest: str,
                                production_receipts: Sequence[Mapping[str, Any]]) -> dict[str, Any]:
    """Assemble the nested schema-2 completion receipt from controller objects.

    Production receipts come from hosted verification, never from local
    fixtures: at least one is required and each must show full passing cases.
    """
    _refuse(type(production_receipts) is list and bool(production_receipts), 'completion requires hosted production receipts')
    current = store.inspect()
    task_state = current['tasks'].get(task_id)
    _refuse(task_state is not None, 'unknown task')
    contract = load_object(store, task_state['contract_digest'])
    task = contract['payload']
    authorization = load_authorization(store, authorization_digest)
    _refuse(authorization['task_id'] == task_id, 'authorization binds a different task')
    remote_record = load_object(store, remote_receipt_digest)
    _refuse(remote_record['kind'] == 'receipt' and remote_record['payload'].get('kind') == 'remote-promotion', 'object is not a remote promotion receipt')
    remote = remote_record['payload']['receipt']
    _refuse(remote['authorization_digest'] == authorization_digest, 'remote receipt binds a different authorization')
    reviews = [load_object(store, digest) for digest in current['object_digests']]
    reviews = [r['payload'] for r in reviews if r['kind'] == 'review' and r['payload'].get('task_id') == task_id
               and r['payload'].get('report',{}).get('candidate_sha') == authorization['candidate_sha']]
    _refuse(len(reviews) == 1, 'completion requires exactly one persisted review')
    payload = c._release_authority(store.authority)
    receipt = {'schema_version': 1, 'task_id': task_id, 'checkpoint_id': task['checkpoint_id'],
               'task_contract_digest': task_state['contract_digest'], 'authority_digest': task['authority_digest'],
               'adoption_manifest_digest': adoption_manifest_digest, 'base_sha': task['base_sha'],
               'promoted_implementation_sha': authorization['candidate_sha'],
               'promotion_authorization_digest': authorization_digest,
               'remote_promotion_receipt_digest': remote_receipt_digest, 'guard_receipt_digest': guard_digest,
               'verification_receipt_digests': sorted(authorization['verification_receipt_digests']),
               'reviewer_receipt_digest': authorization['reviewer_receipt_digest'],
               'acceptance_contract_digest': acceptance_contract_digest,
               'production_acceptance_receipts': list(production_receipts)}
    context = {'task_id': task_id, 'checkpoint_id': task['checkpoint_id'],
               'task_contract_digest': task_state['contract_digest'], 'authority_digest': task['authority_digest'],
               'adoption_manifest_digest': adoption_manifest_digest, 'base_sha': task['base_sha'],
               'promotion_authorization_digest': authorization_digest,
               'remote_promotion_receipt_digest': remote_receipt_digest, 'guard_receipt_digest': guard_digest,
               'verification_receipt_digests': sorted(authorization['verification_receipt_digests']),
               'reviewer_receipt_digest': authorization['reviewer_receipt_digest'],
               'acceptance_contract_digest': acceptance_contract_digest,
               'production_acceptance_receipts': list(production_receipts)}
    c.validate_control_plane_receipt(receipt, payload['schemas'], context=context)
    return receipt
