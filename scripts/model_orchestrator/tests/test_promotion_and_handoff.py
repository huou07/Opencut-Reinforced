#!/usr/bin/env python3
"""M4 promotion and handoff: real bare remotes, races, crashes; no force, no rebase.

The integration repository and bare remote are disposable fixtures. Product
main is never touched. Crash recovery is proven by exact push counts: one
successful push per intent, never two.
"""
from __future__ import annotations
import copy
import inspect
import json
import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / 'scripts'))
from model_orchestrator import contracts as c, store as s, sandbox as b, promotion as p, push_guard
from model_orchestrator.tests.test_contracts import shared_provenance, valid_task, SCHEMAS
from model_orchestrator.tests.test_lifecycle import passing_report
import agent_supervisor
import execution_evidence
import execution_plan

BASE = c.TRUSTED_DESIGN_BASE


def git(root, *args):
    env = dict(os.environ, GIT_AUTHOR_NAME='Fixture', GIT_AUTHOR_EMAIL='fixture@example.invalid',
               GIT_COMMITTER_NAME='Fixture', GIT_COMMITTER_EMAIL='fixture@example.invalid')
    return subprocess.check_output(['git', *args], cwd=root, env=env, stderr=subprocess.DEVNULL).decode().strip()


def m4_store(parent):
    fixture = shared_provenance()
    authority = fixture.authorities['M4']
    payload = c._release_authority(authority)
    build = payload['build']
    task = valid_task()
    task.update(task_id=build['task_id'], checkpoint_id='M4', base_sha=build['base_sha'],
                candidate_branch=build['candidate_branch'],
                authority_digest=payload['git']['authority_digest'])
    parent = Path(parent).resolve()
    runtime = s.RuntimeStore(parent / 'runtime', authority)
    runtime.initialize()
    digest = runtime.register_task(task, copy.deepcopy(task))
    with runtime.lock('task', task['task_id']):
        runtime.claim(task['task_id'], digest, owner_nonce='owner', boot_identity='boot',
                      stage_id='stage', stage_nonce='nonce')
    return fixture, authority, runtime, task, digest


def settle_test_only(runtime, task_id):
    # Test-only settled setup; the live settle path owns the proof requirement.
    current = runtime.inspect()

    def update(state):
        state['tasks'][task_id]['status'] = 'SETTLED'
        state['tasks'][task_id]['stage'] = None
        state['active_task'] = None

    runtime.transaction(current['sequence'], current['epoch'], update)


def guard_record(head):
    return {'head': head, 'tree_digest': 'e' * 64, 'vetoes': [], 'flags': []}


def verification_receipts(task, head, lease_epoch):
    out = []
    for check in task['check_argv']:
        out.append({'schema_version': 1, 'check_id': check['id'], 'exit_code': 0, 'outcome': 'PASS',
                    'executed_cases': list(check['required_cases']), 'passed_cases': list(check['required_cases']),
                    'failed_cases': [], 'skipped_cases': [], 'artifact_digest': 'd' * 64, 'signal': None,
                    'timed_out': False, 'executable_found': True, 'duration_seconds': 1, 'measurements': [],
                    'task_id': task['task_id'], 'candidate_sha': head, 'authority_digest': task['authority_digest'],
                    'task_contract_digest': c.canonical_digest(task), 'command_digest': c.canonical_digest(check),
                    'environment_digest': check['environment_digest'], 'lease_epoch': lease_epoch,
                    'sequence': 1, 'stage_nonce': 'stage-1'})
    return out


def review_payload(task, head):
    return {'report': passing_report(task, head),
            'metadata': {'adapter': 'test', 'family': 'fixture-b', 'effort': ['HIGH', None, 'DEFAULT_PROVIDER'],
                         'session_id': None, 'argv_digest': 'd' * 64}}


def authorize(runtime, task, head, sequence=1):
    from model_orchestrator import orchestrator as o
    digest = o.persist_review(runtime, task['task_id'], review_payload(task, head)['report'],
                              review_payload(task, head)['metadata'])
    inputs = {'task': task, 'guard': guard_record(head),
              'readiness': {'verification_passed': True, 'unresolved': [], 'pending_hosted_classes': [],
                            'receipts': verification_receipts(task, head, runtime.inspect()['tasks'][task['task_id']]['lease_epoch'])},
              'review': review_payload(task, head),
              'lease_epoch': runtime.inspect()['tasks'][task['task_id']]['lease_epoch']}
    authorization = p.build_authorization(inputs, schemas=SCHEMAS, implementation_family='fixture-a',
                                          issuance_sequence=sequence)
    return p.persist_authorization(runtime, task['task_id'], authorization), authorization


def integration_repo(parent, fixture):
    """Disposable integration repo at the authorized base with a bare origin."""
    parent = Path(parent).resolve()
    bare = parent / 'remote.git'
    subprocess.run(['git', 'init', '--bare', '-q', str(bare)], check=True)
    subprocess.run(['git', '--git-dir', str(bare), 'config', 'receive.denyNonFastForwards', 'true'], check=True)
    repo = parent / 'integration'
    subprocess.run(['git', 'init', '-q', '-b', 'main', str(repo)], check=True)
    git(repo, 'fetch', '-q', str(fixture.candidate), BASE)
    git(repo, 'checkout', '-q', '-b', 'main', 'FETCH_HEAD')
    git(repo, 'remote', 'add', 'origin', str(bare))
    git(repo, 'push', '-q', 'origin', 'main:main')
    return repo, str(bare)


def candidate_commit(repo, name='work.txt', content='candidate work'):
    # The candidate lives off main so the integration branch stays at base.
    git(repo, 'checkout', '-q', '-b', 'candidate')
    (repo / name).write_text(content)
    git(repo, 'add', name)
    git(repo, 'commit', '-qm', 'fixture candidate')
    head = git(repo, 'rev-parse', 'HEAD')
    git(repo, 'checkout', '-q', 'main')
    return head


class CountingRunner:
    """Delegates to the real push while counting invocations exactly."""
    def __init__(self):
        self.calls = []

    def __call__(self, argv, cwd):
        self.calls.append(list(argv))
        return p._run_push(argv, cwd)


class AuthorizationTests(unittest.TestCase):
    """CP36: no caller bypass; forged/stale bindings refuse before any push."""
    def test_no_precheck_or_legacy_bypass_parameters(self):
        parameters = inspect.signature(p.promote).parameters
        self.assertNotIn('precheck', parameters)
        self.assertNotIn('legacy', parameters)
        self.assertNotIn('export', parameters)
        self.assertFalse(any(param.kind == inspect.Parameter.VAR_KEYWORD for param in parameters.values()))
        for name in ('force', 'rebase'):
            self.assertNotIn(name, parameters)

    def test_unknown_task_and_forged_stale_bindings_refuse(self):
        fixture, authority, runtime, task, _ = m4_store(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, runtime.root.parent, True)
        settle_test_only(runtime, task['task_id'])
        head = 'c' * 40
        digest, authorization = authorize(runtime, task, head)
        self.assertEqual(p.load_authorization(runtime, digest), authorization)
        with self.assertRaises(p.PromotionError):
            p.load_authorization(runtime, '0' * 64)
        path = runtime.root / 'objects' / (digest + '.json')
        original = path.read_bytes()
        try:
            forged = json.loads(original.decode())
            forged['payload']['authorization']['candidate_sha'] = 'd' * 40
            path.write_bytes((c.canonical_json(forged) + '\n').encode())
            with self.assertRaises(p.PromotionError):
                p.load_authorization(runtime, digest)
        finally:
            path.write_bytes(original)
        # Stale issuance: a superseded authorization cannot promote.
        digest2, _ = authorize(runtime, task, head, sequence=2)
        with tempfile.TemporaryDirectory() as directory:
            repo, url = integration_repo(directory, fixture)
            hooks = Path(directory) / 'hooks'
            with self.assertRaises(p.PromotionError):
                p.promote(store=runtime, task_id=task['task_id'], authorization_digest=digest,
                          integration_repo=repo, remote='origin', expected_remote_url=url, hooks_dir=hooks)

    def test_defect_review_and_failed_check_cannot_authorize(self):
        fixture, authority, runtime, task, _ = m4_store(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, runtime.root.parent, True)
        settle_test_only(runtime, task['task_id'])
        head = 'c' * 40
        bad_review = review_payload(task, head)
        bad_review['report'] = dict(bad_review['report'], verdict='DEFECT_FOUND',
                                    findings=[dict(id='f1', severity='BLOCKING', classification='PROVEN',
                                                   citation='src/x.py:1', claim='defect', competing_hypotheses=[],
                                                   discriminating_check='measure')])
        from model_orchestrator import orchestrator as o
        o.persist_review(runtime, task['task_id'], bad_review['report'], bad_review['metadata'])
        epoch = runtime.inspect()['tasks'][task['task_id']]['lease_epoch']
        receipts = verification_receipts(task, head, epoch)
        receipts[0] = dict(receipts[0], outcome='FAIL', failed_cases=['case1'], passed_cases=[])
        inputs = {'task': task, 'guard': guard_record(head),
                  'readiness': {'verification_passed': False, 'unresolved': ['unit: FAIL'],
                                'pending_hosted_classes': [], 'receipts': receipts},
                  'review': bad_review, 'lease_epoch': epoch}
        with self.assertRaises(p.PromotionError):
            p.build_authorization(inputs, schemas=SCHEMAS, implementation_family='fixture-a', issuance_sequence=1)

    def test_wrong_base_candidate_and_dirty_tree_refuse_before_intent(self):
        fixture, authority, runtime, task, _ = m4_store(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, runtime.root.parent, True)
        settle_test_only(runtime, task['task_id'])
        head = 'c' * 40
        digest, _ = authorize(runtime, task, head)
        with tempfile.TemporaryDirectory() as directory:
            repo, url = integration_repo(directory, fixture)
            hooks = Path(directory) / 'hooks'
            before = set(runtime.inspect()['object_digests'])
            other_base = '454f1597a74de2703064487518bdbe7a12731bbc'
            git(repo, 'fetch', '-q', str(fixture.candidate), other_base)
            git(repo, 'reset', '-q', '--hard', 'FETCH_HEAD')
            with self.assertRaises(p.PromotionError):
                p.promote(store=runtime, task_id=task['task_id'], authorization_digest=digest,
                          integration_repo=repo, remote='origin', expected_remote_url=url, hooks_dir=hooks)
            git(repo, 'reset', '-q', '--hard', BASE)
            (repo / 'dirty.txt').write_text('dirty')
            with self.assertRaises(p.PromotionError):
                p.promote(store=runtime, task_id=task['task_id'], authorization_digest=digest,
                          integration_repo=repo, remote='origin', expected_remote_url=url, hooks_dir=hooks)
            self.assertEqual(set(runtime.inspect()['object_digests']), before)

    def test_quality_disposition_never_waives_failed_checks_vetoes_or_fidelity(self):
        fixture, authority, runtime, task, _ = m4_store(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, runtime.root.parent, True)
        head = 'c' * 40
        epoch = runtime.inspect()['tasks'][task['task_id']]['lease_epoch']
        flag = 'unresolved material change: work.txt'
        from model_orchestrator import verification as v
        review = review_payload(task, head)
        review['report']['quality_flag_dispositions'] = [
            dict(id=v.quality_flag_id(flag), disposition='NOT_LOWERING', evidence_digest='d' * 64)]
        inputs = dict(task=task, guard=dict(guard_record(head), flags=[flag]),
                      readiness=dict(verification_passed=True, unresolved=[flag],
                                     pending_hosted_classes=[], receipts=verification_receipts(task, head, epoch)),
                      review=review, lease_epoch=epoch)
        self.assertEqual(p.build_authorization(inputs, schemas=SCHEMAS,
                         implementation_family='fixture-a', issuance_sequence=1)['candidate_sha'], head)
        for attack in ('missing', 'lowering', 'stale', 'stale-flag', 'veto', 'acceptance', 'failed', 'incomplete'):
            bad = copy.deepcopy(inputs)
            if attack == 'missing': bad['review']['report']['quality_flag_dispositions'] = []
            elif attack == 'lowering':
                bad['review']['report']['quality_flag_dispositions'][0]['disposition'] = 'DEFECT'
                bad['review']['report']['verdict'] = 'DEFECT_FOUND'
            elif attack == 'stale': bad['review']['report']['candidate_sha'] = 'f' * 40
            elif attack == 'stale-flag': bad['review']['report']['quality_flag_dispositions'][0]['id'] = v.quality_flag_id('other observation')
            elif attack == 'veto': bad['guard']['vetoes'] = ['required case missing: case1']
            elif attack == 'acceptance': bad['readiness']['unresolved'].append('required real class evidence pending: USER_JOURNEY')
            elif attack == 'failed': bad['readiness']['receipts'][0]['outcome'] = 'FAIL'
            else: bad['readiness']['receipts'] = []
            with self.subTest(attack=attack), self.assertRaises(c.ContractError):
                p.build_authorization(bad, schemas=SCHEMAS, implementation_family='fixture-a', issuance_sequence=1)

    def test_collect_inputs_refuses_unknown_or_unsettled_task(self):
        fixture, authority, runtime, task, _ = m4_store(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, runtime.root.parent, True)
        with self.assertRaises(p.PromotionError):
            p.collect_inputs(store=runtime, task_id='unknown-task', guard_root=Path('/nonexistent'),
                             attempt_dir=Path('/nonexistent'), task_path='x', catalog_path='y',
                             implementation_family='fixture-a')


class PushMatrixTests(unittest.TestCase):
    """CP37-40: real bare remotes, races, crashes; exactly one push per intent."""
    def promote_once(self, runtime, task, digest, repo, url, hooks, runner=None, fault=None):
        return p.promote(store=runtime, task_id=task['task_id'], authorization_digest=digest,
                         integration_repo=repo, remote='origin', expected_remote_url=url,
                         hooks_dir=hooks, push_runner=runner, fault=fault)

    def test_success_publishes_once_and_syncs(self):
        fixture, authority, runtime, task, _ = m4_store(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, runtime.root.parent, True)
        with tempfile.TemporaryDirectory() as directory:
            parent = Path(directory).resolve()
            repo, url = integration_repo(parent, fixture)
            head = candidate_commit(repo)
            settle_test_only(runtime, task['task_id'])
            digest, _ = authorize(runtime, task, head)
            runner = CountingRunner()
            result = self.promote_once(runtime, task, digest, repo, url, parent / 'hooks', runner=runner)
            self.assertEqual(result['status'], 'PROMOTED')
            self.assertEqual(len(runner.calls), 1)
            self.assertNotIn('--force', runner.calls[0])
            self.assertNotIn('--force-with-lease', runner.calls[0])
            self.assertEqual(git(repo, 'ls-remote', url, 'refs/heads/main').split()[0], head)
            self.assertEqual(git(repo, 'rev-parse', 'HEAD'), head)
            latest, intent = p._latest_intent(runtime, task['task_id'])
            self.assertEqual(intent['outcome'], 'PROMOTED')
            again = p.reconcile_push(store=runtime, task_id=task['task_id'], intent_digest=latest,
                                     integration_repo=repo, remote='origin', push_observed=False)
            self.assertEqual(again['status'], 'PROMOTED')
            self.assertEqual(len(runner.calls), 1)

    def test_hook_refuses_advanced_remote(self):
        fixture, authority, runtime, task, _ = m4_store(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, runtime.root.parent, True)
        with tempfile.TemporaryDirectory() as directory:
            parent = Path(directory).resolve()
            repo, url = integration_repo(parent, fixture)
            head = candidate_commit(repo)
            settle_test_only(runtime, task['task_id'])
            digest, authorization = authorize(runtime, task, head)
            hooks = parent / 'hooks'
            push_guard.install_push_guard(repo, expected_base=BASE, expected_candidate=head,
                                          expected_remote_url=url, hooks_dir=hooks)
            other = parent / 'other'
            subprocess.run(['git', 'clone', '-q', url, str(other)], check=True)
            git(other, 'checkout', '-q', 'main')
            (other / 'race.txt').write_text('race')
            git(other, 'add', 'race.txt')
            git(other, 'commit', '-qm', 'race')
            git(other, 'push', '-q', 'origin', 'main:main')
            # The server would allow this clobber; only the hook refuses it.
            subprocess.run(['git', '--git-dir', str(parent / 'remote.git'),
                            'config', 'receive.denyNonFastForwards', 'false'], check=True)
            proc = subprocess.run(['git', 'push', 'origin', head + ':refs/heads/main'], cwd=repo,
                                  capture_output=True, text=True, timeout=60)
            self.assertNotEqual(proc.returncode, 0)
            self.assertIn('or-v2 push guard refuses', proc.stderr)
            self.assertNotEqual(git(repo, 'ls-remote', url, 'refs/heads/main').split()[0], head)

    def test_remote_race_between_fetch_and_push_is_rejected(self):
        fixture, authority, runtime, task, _ = m4_store(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, runtime.root.parent, True)
        with tempfile.TemporaryDirectory() as directory:
            parent = Path(directory).resolve()
            repo, url = integration_repo(parent, fixture)
            head = candidate_commit(repo)
            settle_test_only(runtime, task['task_id'])
            digest, _ = authorize(runtime, task, head)
            other = parent / 'other'
            subprocess.run(['git', 'clone', '-q', url, str(other)], check=True)
            git(other, 'fetch', '-q', 'origin')
            git(other, 'checkout', '-q', 'main')
            (other / 'race.txt').write_text('race')
            runner = CountingRunner()

            def advance(point):
                if point == 'before_push':
                    git(other, 'add', 'race.txt')
                    git(other, 'commit', '-qm', 'race')
                    git(other, 'push', '-q', 'origin', 'main:main')

            result = self.promote_once(runtime, task, digest, repo, url, parent / 'hooks',
                                       runner=runner, fault=advance)
            self.assertEqual(result['status'], 'DIAGNOSTIC')
            self.assertEqual(len(runner.calls), 1)
            latest, intent = p._latest_intent(runtime, task['task_id'])
            self.assertEqual(intent['outcome'], 'DIAGNOSTIC')
            self.assertNotEqual(git(repo, 'ls-remote', url, 'refs/heads/main').split()[0], head)
            self.assertEqual(p.load_authorization(runtime, digest)['candidate_sha'], head)

    def test_two_contenders_serialize_and_single_candidate_published(self):
        import multiprocessing
        fixture, authority, runtime, task, _ = m4_store(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, runtime.root.parent, True)
        with tempfile.TemporaryDirectory() as directory:
            parent = Path(directory).resolve()
            repo, url = integration_repo(parent, fixture)
            head = candidate_commit(repo)
            settle_test_only(runtime, task['task_id'])
            digest, _ = authorize(runtime, task, head)
            ctx = multiprocessing.get_context('fork')
            results = ctx.Queue()

            def contend(output):
                try:
                    outcome = p.promote(store=s.RuntimeStore(runtime.root, authority), task_id=task['task_id'],
                                        authorization_digest=digest, integration_repo=repo, remote='origin',
                                        expected_remote_url=url, hooks_dir=parent / 'hooks')
                    output.put(outcome['status'])
                except Exception as exc:  # noqa: BLE001 - record contender failure
                    output.put(type(exc).__name__ + ': ' + str(exc)[:120])

            children = [ctx.Process(target=contend, args=(results,)) for _ in range(2)]
            for child in children:
                child.start()
            for child in children:
                child.join(120)
                self.assertFalse(child.is_alive())
            statuses = sorted(results.get(timeout=10) for _ in range(2))
            self.assertEqual(git(repo, 'ls-remote', url, 'refs/heads/main').split()[0], head)
            self.assertIn('PROMOTED', statuses)
            self.assertTrue(all(status in ('PROMOTED', 'PROMOTION_READY', 'PromotionError', 'DIAGNOSTIC')
                                or 'PromotionError' in status or 'StoreError' in status for status in statuses))

    def test_push_ok_then_dirty_leaves_sync_pending_and_recovers_once(self):
        fixture, authority, runtime, task, _ = m4_store(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, runtime.root.parent, True)
        with tempfile.TemporaryDirectory() as directory:
            parent = Path(directory).resolve()
            repo, url = integration_repo(parent, fixture)
            head = candidate_commit(repo)
            settle_test_only(runtime, task['task_id'])
            digest, _ = authorize(runtime, task, head)
            runner = CountingRunner()

            def dirty_after_push(argv, cwd):
                runner.calls.append(list(argv))
                code, out, err = p._run_push(argv, cwd)
                (Path(cwd) / 'DIRTY').write_text('operator must clean')
                return code, out, err

            result = self.promote_once(runtime, task, digest, repo, url, parent / 'hooks', runner=dirty_after_push)
            self.assertEqual(result['status'], 'LOCAL_SYNC_PENDING')
            self.assertEqual(len(runner.calls), 1)
            self.assertEqual(git(repo, 'ls-remote', url, 'refs/heads/main').split()[0], head)
            (repo / 'DIRTY').unlink()
            latest, _ = p._latest_intent(runtime, task['task_id'])
            recovered = p.reconcile_sync(store=runtime, task_id=task['task_id'], intent_digest=latest,
                                         integration_repo=repo)
            self.assertEqual(recovered['status'], 'PROMOTED')
            self.assertEqual(len(runner.calls), 1)
            self.assertEqual(git(repo, 'rev-parse', 'HEAD'), head)

    def test_crash_matrix_reconciles_exactly_without_second_push(self):
        fixture, authority, runtime, task, _ = m4_store(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, runtime.root.parent, True)
        with tempfile.TemporaryDirectory() as directory:
            parent = Path(directory).resolve()
            repo, url = integration_repo(parent, fixture)
            head = candidate_commit(repo)
            settle_test_only(runtime, task['task_id'])
            digest, _ = authorize(runtime, task, head)
            hooks = parent / 'hooks'
            runner = CountingRunner()
            # Kill before push: intent persisted, nothing pushed.
            def die_before_push(point):
                if point == 'before_push':
                    raise p.PromotionError('simulated controller death', 'UNAVAILABLE')

            with self.assertRaises(p.PromotionError):
                self.promote_once(runtime, task, digest, repo, url, hooks, runner=runner, fault=die_before_push)
            self.assertEqual(len(runner.calls), 0)
            latest, intent = p._latest_intent(runtime, task['task_id'])
            self.assertEqual(intent['outcome'], 'PROMOTING')
            # Unexpected remote advance while the intent is promoting:
            # diagnostic, intent and authorization retained.
            other = parent / 'other'
            subprocess.run(['git', 'clone', '-q', url, str(other)], check=True)
            git(other, 'checkout', '-q', 'main')
            (other / 'race.txt').write_text('unexpected')
            git(other, 'add', 'race.txt')
            git(other, 'commit', '-qm', 'unexpected advance')
            git(other, 'push', '-q', 'origin', 'main:main')
            raced = p.reconcile_push(store=runtime, task_id=task['task_id'], intent_digest=latest,
                                     integration_repo=repo, remote='origin', push_observed=False)
            self.assertEqual(raced['status'], 'DIAGNOSTIC')
            self.assertEqual(p.load_authorization(runtime, digest)['candidate_sha'], head)
            # A settled final outcome is stable: re-reconciling reports it
            # without side effects instead of relitigating new remote facts.
            again_diag = p.reconcile_push(store=runtime, task_id=task['task_id'], intent_digest=latest,
                                          integration_repo=repo, remote='origin', push_observed=False)
            self.assertEqual(again_diag['status'], 'DIAGNOSTIC')
            # The intent is final, so only an explicit re-promotion proceeds.
            # Operator restores the base; re-promotion succeeds exactly once.
            subprocess.run(['git', '--git-dir', str(parent / 'remote.git'), 'update-ref', 'refs/heads/main', BASE], check=True)
            result = self.promote_once(runtime, task, digest, repo, url, hooks, runner=runner)
            self.assertEqual(result['status'], 'PROMOTED')
            self.assertEqual(len(runner.calls), 1)
            latest, _ = p._latest_intent(runtime, task['task_id'])
            again = p.reconcile_push(store=runtime, task_id=task['task_id'], intent_digest=latest,
                                     integration_repo=repo, remote='origin', push_observed=False)
            self.assertEqual(again['status'], 'PROMOTED')
            self.assertEqual(len(runner.calls), 1)

    def test_unreachable_remote_stays_promoting(self):
        fixture, authority, runtime, task, _ = m4_store(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, runtime.root.parent, True)
        with tempfile.TemporaryDirectory() as directory:
            parent = Path(directory).resolve()
            repo, url = integration_repo(parent, fixture)
            head = candidate_commit(repo)
            settle_test_only(runtime, task['task_id'])
            digest, _ = authorize(runtime, task, head)
            runner = CountingRunner()
            result = self.promote_once(runtime, task, digest, repo, url, parent / 'hooks', runner=runner)
            self.assertEqual(result['status'], 'PROMOTED')
            git(repo, 'reset', '-q', '--hard', BASE)
            shutil.rmtree(parent / 'remote.git')
            # Promoting against a missing remote is unavailable, not a push.
            digest2, _ = authorize(runtime, task, head, sequence=2)
            with self.assertRaises(p.PromotionError) as ctx:
                self.promote_once(runtime, task, digest2, repo, url, parent / 'hooks', runner=runner)
            self.assertEqual(ctx.exception.code, 'UNAVAILABLE')
            self.assertEqual(len(runner.calls), 1)
            # A manually persisted intent reconciled against the missing
            # remote stays PROMOTING with availability, pushing nothing.
            intent = {'schema_version': 1, 'kind': 'promotion-intent', 'task_id': task['task_id'],
                      'authorization_digest': digest2, 'base_sha': BASE, 'candidate_sha': head,
                      'destination_ref': 'refs/heads/main', 'remote_url': url,
                      'intent_nonce': 'manual-intent', 'outcome': 'PROMOTING'}
            manual = p.persist_intent(runtime, intent)
            stuck = p.reconcile_push(store=runtime, task_id=task['task_id'], intent_digest=manual,
                                     integration_repo=repo, remote='origin', push_observed=False)
            self.assertEqual((stuck['status'], stuck['availability']), ('PROMOTING', 'remote-unreachable'))
            self.assertEqual(len(runner.calls), 1)


def product_repo(parent):
    """Minimal product-shaped repo: plan/state, origin bare, version sources."""
    parent = Path(parent).resolve()
    bare = parent / 'origin.git'
    subprocess.run(['git', 'init', '--bare', '-q', str(bare)], check=True)
    repo = parent / 'product'
    subprocess.run(['git', 'init', '-q', '-b', 'main', str(repo)], check=True)
    plan = {
        'schema_version': 1, 'plan_id': 'fixture-plan', 'authority_order': [],
        'quality_contract_version': 1,
        'checkpoints': [
            {'id': 'A', 'phase': 9, 'title': 'First', 'spec_document': 'docs/execution/phases/PHASE_9.md',
             'prerequisite_checkpoint_ids': [], 'milestone_membership': ['m'], 'user_visible': False,
             'developer_preview_required': False, 'architecture_gate': False,
             'expected_project_schema_effect_category': 'none', 'expected_ipc_effect_category': 'none',
             'dependency_change_policy': 'No new dependency.', 'next_checkpoint_relation': 'B',
             'evidence_contract_version': 1},
            {'id': 'B', 'phase': 9, 'title': 'Second', 'spec_document': 'docs/execution/phases/PHASE_9.md',
             'prerequisite_checkpoint_ids': ['A'], 'milestone_membership': ['m'], 'user_visible': False,
             'developer_preview_required': False, 'architecture_gate': False,
             'expected_project_schema_effect_category': 'none', 'expected_ipc_effect_category': 'none',
             'dependency_change_policy': 'No new dependency.', 'next_checkpoint_relation': None,
             'evidence_contract_version': 1},
        ],
        'milestones': {'m': {'title': 'M', 'checkpoint_ids': ['A', 'B'], 'completion_checkpoint_id': 'B'}},
        'phases': {'9': {'title': 'Nine', 'spec_document': 'docs/execution/phases/PHASE_9.md'}},
    }
    state = {'schema_version': 1, 'checkpoints': {'A': 'NEXT', 'B': 'PLANNED'}, 'current_next': 'A',
             'phase_status': {'9': 'IN_PROGRESS', '5': 'DONE'},
             'verified_contract_versions': {'project_schema': 7, 'recovery_schema': 1, 'ipc_protocol': 1},
             'repository': 'fixture', 'last_updated': '2026-10-04'}
    policy = {'schema_version': 1, 'enforced_from_checkpoint': 'A',
              'repository': {'owner': 'fixture', 'name': 'fixture', 'branch': 'main'},
              'github_api': {'api_version': '2022-11-28', 'authenticated_poll_seconds': 15,
                             'unauthenticated_poll_seconds': 90, 'timeout_seconds': 7200,
                             'token_environment_variables': ['GH_TOKEN', 'GITHUB_TOKEN']},
              'required_gates': {'repository_hygiene': {'workflow_file': '.github/workflows/repo-hygiene.yml',
                                                       'workflow_name': 'Repository hygiene',
                                                       'required_jobs': ['Repository hygiene']}},
              'developer_preview': {'workflow_file': '.github/workflows/developer-preview.yml',
                                    'workflow_name': 'Developer Preview', 'allowed_events': ['workflow_dispatch'],
                                    'publish_job': 'Verify and publish prerelease', 'tag_template': 'dev-{sha12}',
                                    'required_asset_count': 1, 'required_assets': ['a.zip'],
                                    'checksums_asset': 'a.zip', 'build_info_asset': 'a.zip'},
              'state_transition': {'runner_may_write_state': False, 'completion_status': 'DONE',
                                   'successor_status': 'NEXT',
                                   'allowed_commit_paths': ['docs/execution/STATE.json',
                                                            'docs/execution/evidence/{checkpoint_id}.json'],
                                   'commit_subject_template': 'chore(execution): complete {checkpoint_id}'},
              'evidence_classes': {'enforced_from_checkpoint': 'A', 'allowed': ['STATIC']},
              'evidence_class_proofs': {}}
    files = {
        'docs/execution/PLAN.json': json.dumps(plan),
        'docs/execution/STATE.json': json.dumps(state),
        'docs/execution/EVIDENCE_POLICY.json': json.dumps(policy),
        'docs/execution/architecture-policy.json': json.dumps({
            'schema_version': 1, 'frozen_prototype_sha256': '0' * 64,
            'contract_transition_rules': {
                'project_schema': {'owner_categories': ['explicit-model-gate', 'typed-model-gate'],
                                   'owner_actions': ['retain', 'increment_by_one'], 'maximum_increment': 1,
                                   'non_owner_transition': 'retain_verified'},
                'recovery_schema': {'owner_categories': [], 'owner_actions': [],
                                    'maximum_increment': 0, 'non_owner_transition': 'retain_verified'},
                'ipc_protocol': {'owner_categories': ['explicit-contract-gate'],
                                 'owner_actions': ['retain', 'increment_by_one'], 'maximum_increment': 1,
                                 'non_owner_transition': 'retain_verified'}},
            'or_core_forbidden_direct_dependency_name_patterns': [], 'required_execution_docs': []}),
        'docs/execution/phases/PHASE_9.md': '# Phase 9\n',
        'crates/or_core/src/project_document.rs': 'pub const CURRENT_PROJECT_SCHEMA_VERSION: u32 = 7;\n',
        'crates/or_core/src/project_recovery.rs': 'pub const CURRENT_RECOVERY_SCHEMA_VERSION: u32 = 1;\n',
        'crates/or_ipc/src/protocol.rs': 'pub const OR_LOCAL_IPC_PROTOCOL_VERSION: u32 = 1;\n',
    }
    for name, content in files.items():
        path = repo / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content)
    git(repo, 'add', '-A')
    git(repo, 'commit', '-qm', 'fixture base')
    git(repo, 'remote', 'add', 'origin', str(bare))
    git(repo, 'push', '-q', 'origin', 'main:main')
    return repo, str(bare)


def promote_product_candidate(repo, name='impl.txt', content='implementation'):
    (repo / name).write_text(content)
    git(repo, 'add', name)
    git(repo, 'commit', '-qm', 'fixture implementation')
    head = git(repo, 'rev-parse', 'HEAD')
    git(repo, 'push', '-q', 'origin', 'main:main')
    return head


class FakeGitHubApi:
    """Canned exact-SHA workflow runs; every call is observed, nothing is real."""
    def __init__(self, implementation_sha, gates):
        self.sha = implementation_sha
        self.gates = gates
        self.calls = []

    def get(self, path):
        self.calls.append(path)
        if path.endswith('/actions/runs'):
            runs = []
            for index, gate in enumerate(self.gates):
                runs.append({'id': 100 + index, 'run_attempt': 1, 'head_sha': self.sha, 'head_branch': 'main',
                             'event': 'push', 'status': 'completed', 'conclusion': 'success',
                             'path': gate['workflow_file'], 'name': gate['workflow_name'],
                             'html_url': 'https://example.invalid/runs/%d' % (100 + index),
                             'updated_at': '2026-10-04T00:00:00Z'})
            return {'workflow_runs': runs}
        run_id = int(path.split('/actions/runs/')[1].split('/')[0])
        gate = self.gates[run_id - 100]
        return {'jobs': [{'name': name, 'status': 'completed', 'conclusion': 'success', 'steps': []}
                         for name in gate['required_jobs']]}

    def post(self, path, payload):
        raise AssertionError('no dispatch in handoff tests')


class HandoffTests(unittest.TestCase):
    """CP41: handoff binds task, receipts, and frozen supervisor code or refuses."""
    def context(self, directory):
        fixture, authority, runtime, task, _ = m4_store(directory)
        repo, url = product_repo(Path(directory) / 'product-fixture')
        head = promote_product_candidate(repo)
        settle_test_only(runtime, task['task_id'])
        digest, _ = authorize(runtime, task, head)
        current = runtime.inspect()
        receipt = {'schema_version': 1, 'task_id': task['task_id'], 'authorization_digest': digest,
                   'destination_ref': 'refs/heads/main', 'base_sha': task['base_sha'], 'candidate_sha': head,
                   'observed_remote_sha': head, 'intent_nonce': 'handoff-intent'}
        record = dict(schema_version=1, kind='receipt',
                      payload=dict(schema_version=1, kind='remote-promotion', receipt=receipt))
        runtime.transaction(current['sequence'], current['epoch'], lambda state: None, objects=[record])
        remote_digest = c.canonical_digest(record)
        return fixture, runtime, task, repo, head, digest, remote_digest

    def test_missing_task_id_never_invokes(self):
        with tempfile.TemporaryDirectory() as directory:
            _, runtime, task, repo, head, digest, remote = self.context(directory)
            for bad in ('', None):
                with self.subTest(bad=bad), self.assertRaises(agent_supervisor.SupervisorError):
                    agent_supervisor.validate_task_handoff(repo, task_id=bad, checkpoint_id='A',
                                                           store_root=runtime.root, authorization_digest=digest,
                                                           remote_receipt_digest=remote)

    def test_forged_stale_and_wrong_bindings_refuse(self):
        with tempfile.TemporaryDirectory() as directory:
            _, runtime, task, repo, head, digest, remote = self.context(directory)
            with self.assertRaises(agent_supervisor.SupervisorError):
                agent_supervisor.validate_task_handoff(repo, task_id='other-task', checkpoint_id='A',
                                                       store_root=runtime.root, authorization_digest=digest,
                                                       remote_receipt_digest=remote)
            with self.assertRaises(agent_supervisor.SupervisorError):
                agent_supervisor.validate_task_handoff(repo, task_id=task['task_id'], checkpoint_id='A',
                                                       store_root=runtime.root, authorization_digest='0' * 64,
                                                       remote_receipt_digest=remote)
            path = runtime.root / 'objects' / (digest + '.json')
            original = path.read_bytes()
            try:
                forged = json.loads(original.decode())
                forged['payload']['authorization']['candidate_sha'] = 'd' * 40
                path.write_bytes((c.canonical_json(forged) + '\n').encode())
                with self.assertRaises(agent_supervisor.SupervisorError):
                    agent_supervisor.validate_task_handoff(repo, task_id=task['task_id'], checkpoint_id='A',
                                                           store_root=runtime.root, authorization_digest=digest,
                                                           remote_receipt_digest=remote)
            finally:
                path.write_bytes(original)

    def test_disabled_wrong_sha_dirty_tree_and_paused_store_refuse(self):
        with tempfile.TemporaryDirectory() as directory:
            runtime, task, repo, head, digest, remote = self.disabled_context(directory)
            def check():
                return agent_supervisor.validate_disabled_task_handoff(repo, store=runtime,
                    task_id=task['task_id'], authorization_digest=digest, remote_receipt_digest=remote)
            git(repo, 'commit', '-q', '--allow-empty', '-m', 'drift')
            with self.assertRaisesRegex(agent_supervisor.SupervisorError, 'HEAD'):
                check()
            git(repo, 'reset', '-q', '--hard', head)
            (repo / 'dirty.txt').write_text('dirty')
            with self.assertRaisesRegex(agent_supervisor.SupervisorError, 'clean worktree'):
                check()
            (repo / 'dirty.txt').unlink()
            runtime.set_paused(True)
            with self.assertRaisesRegex(agent_supervisor.SupervisorError, 'unpaused settled'):
                check()

    def disabled_context(self, directory):
        fixture, authority, runtime, task, _ = m4_store(directory)
        repo, url = integration_repo(Path(directory) / 'integration-fixture', fixture)
        head = candidate_commit(repo)
        settle_test_only(runtime, task['task_id'])
        digest, _ = authorize(runtime, task, head)
        result = p.promote(store=runtime, task_id=task['task_id'], authorization_digest=digest,
                           integration_repo=repo, remote='origin', expected_remote_url=url,
                           hooks_dir=Path(directory).resolve() / 'hooks')
        self.assertEqual(result['status'], 'PROMOTED')
        state = runtime.inspect()
        remotes = [(key, p.load_object(runtime, key)) for key in state['object_digests']]
        remote = next(key for key, record in remotes if record['kind'] == 'receipt'
                      and record['payload'].get('kind') == 'remote-promotion')
        return runtime, task, repo, head, digest, remote

    def test_disabled_task_can_never_complete_product_checkpoint(self):
        with tempfile.TemporaryDirectory() as directory:
            _, runtime, task, repo, head, digest, remote = self.context(directory)
            before = (repo / 'docs/execution/STATE.json').read_bytes()
            with self.assertRaisesRegex(agent_supervisor.SupervisorError, 'disabled control-plane'):
                agent_supervisor.validate_task_handoff(repo, task_id=task['task_id'], checkpoint_id='A',
                                                       store_root=runtime.root, authorization_digest=digest,
                                                       remote_receipt_digest=remote)
            self.assertEqual((repo / 'docs/execution/STATE.json').read_bytes(), before)

    def test_disabled_handoff_is_exact_bound_read_only_readiness(self):
        with tempfile.TemporaryDirectory() as directory:
            runtime, task, repo, head, digest, remote = self.disabled_context(directory)
            before = runtime.inspect()
            plan = (repo / 'docs/execution/PLAN.json').read_bytes()
            state = (repo / 'docs/execution/STATE.json').read_bytes()
            refs = git(repo, 'show-ref')
            intent = agent_supervisor.validate_disabled_task_handoff(
                repo, store=runtime, task_id=task['task_id'], authorization_digest=digest,
                remote_receipt_digest=remote)
            self.assertEqual(intent['status'], 'DISABLED_CONTROL_PLANE_READY')
            self.assertEqual(intent['task_checkpoint'], 'M4')
            self.assertEqual(intent['implementation_sha'], head)
            self.assertEqual(intent['task_contract_digest'], c.canonical_digest(task))
            self.assertEqual(intent['verified_contract_versions'],
                             {'project_schema': 7, 'recovery_schema': 1, 'ipc_protocol': 1})
            self.assertEqual(intent['authority_semantics'], 'DISABLED_FACTS_ONLY_NO_PRODUCT_COMPLETION_NO_ADOPTION')
            self.assertEqual(runtime.inspect(), before)
            self.assertEqual((repo / 'docs/execution/PLAN.json').read_bytes(), plan)
            self.assertEqual((repo / 'docs/execution/STATE.json').read_bytes(), state)
            self.assertEqual(git(repo, 'show-ref'), refs)
            self.assertFalse((repo / agent_supervisor.COMPLETION_INTENT_PATH).exists())

    def test_disabled_handoff_refuses_unpublished_wrong_phase_and_stale_remote(self):
        with tempfile.TemporaryDirectory() as directory:
            runtime, task, repo, head, digest, remote = self.disabled_context(directory)
            record = p.load_object(runtime, digest)
            forged = copy.deepcopy(record)
            forged['payload']['authorization']['checkpoint_id'] = 'M5'
            current = runtime.inspect()
            runtime.transaction(current['sequence'], current['epoch'], lambda state: None, objects=[forged])
            with self.assertRaises(agent_supervisor.SupervisorError):
                agent_supervisor.validate_disabled_task_handoff(repo, store=runtime, task_id=task['task_id'],
                    authorization_digest=c.canonical_digest(forged), remote_receipt_digest=remote)
            # An orphan content-addressed record has no authority without snapshot publication.
            orphan = copy.deepcopy(record)
            orphan['payload']['authorization']['issuance_sequence'] += 1
            orphan_digest = c.canonical_digest(orphan)
            (runtime.root / 'objects' / (orphan_digest + '.json')).write_text(c.canonical_json(orphan) + '\n')
            with self.assertRaises(agent_supervisor.SupervisorError):
                agent_supervisor.validate_disabled_task_handoff(repo, store=runtime, task_id=task['task_id'],
                    authorization_digest=orphan_digest, remote_receipt_digest=remote)
            subprocess.run(['git', '--git-dir', str(Path(directory).resolve() / 'integration-fixture/remote.git'),
                            'update-ref', 'refs/heads/main', BASE], check=True)
            with self.assertRaises(agent_supervisor.SupervisorError):
                agent_supervisor.validate_disabled_task_handoff(repo, store=runtime, task_id=task['task_id'],
                    authorization_digest=digest, remote_receipt_digest=remote)


class CompletionRecoveryTests(unittest.TestCase):
    """CP42/43 shape: exact-SHA hosted evidence, single completion, intent recovery."""
    def completion_record(self, implementation_sha='a' * 40):
        return {'schema_version': 1, 'checkpoint_id': 'A', 'implementation_sha': implementation_sha,
                'implementation_subject': 'fixture', 'verified_at_utc': '2026-10-04T00:00:00Z',
                'contract_versions': {'project_schema': 7, 'recovery_schema': 1, 'ipc_protocol': 1},
                'gates': [{'gate_id': 'repository_hygiene', 'workflow_name': 'Repository hygiene',
                           'workflow_file': '.github/workflows/repo-hygiene.yml', 'head_sha': implementation_sha,
                           'head_branch': 'main', 'event': 'push', 'status': 'completed', 'conclusion': 'success',
                           'run_id': 100, 'run_attempt': 1, 'html_url': 'https://example.invalid/runs/100',
                           'jobs': [{'name': 'Repository hygiene', 'status': 'completed', 'conclusion': 'success'}]}],
                'developer_preview': {'required': False}}

    def hosted(self, repo):
        # Hand-built v1 policy: validate_evidence_record reads it directly and
        # never re-validates the shape, so fixtures stay minimal and honest.
        gate = {'gate_id': 'repository_hygiene', 'workflow_file': '.github/workflows/repo-hygiene.yml',
                'workflow_name': 'Repository hygiene', 'required_jobs': ['Repository hygiene']}
        policy = {'repository': {'owner': 'fixture', 'name': 'fixture', 'branch': 'main'},
                  'github_api': {'api_version': '2022-11-28', 'authenticated_poll_seconds': 15,
                                 'unauthenticated_poll_seconds': 90, 'timeout_seconds': 7200,
                                 'token_environment_variables': ['GH_TOKEN', 'GITHUB_TOKEN']},
                  'required_gates': {'repository_hygiene': {'workflow_file': gate['workflow_file'],
                                                            'workflow_name': gate['workflow_name'],
                                                            'required_jobs': gate['required_jobs']}},
                  'developer_preview': {'workflow_file': '.github/workflows/developer-preview.yml'}}
        api = FakeGitHubApi('a' * 40, [gate])
        return policy, api

    def test_nested_receipt_policy_matrix(self):
        with tempfile.TemporaryDirectory() as directory:
            repo, _ = product_repo(Path(directory))
            policy, _ = self.hosted(repo)
            plan = json.loads((repo / 'docs/execution/PLAN.json').read_text())
            checkpoint = next(item for item in plan['checkpoints'] if item['id'] == 'A')
            record = self.completion_record()
            execution_evidence.validate_evidence_record(record, checkpoint_id='A', checkpoint=checkpoint, policy=policy)
            demanding = dict(policy, orchestration_v2={'enforce_control_plane_receipt': True})
            with self.assertRaises(execution_evidence.EvidenceError):
                execution_evidence.validate_evidence_record(record, checkpoint_id='A', checkpoint=checkpoint, policy=demanding)
            receipt = {'schema_version': 1, 'task_id': 'task-M4', 'checkpoint_id': 'A',
                       'task_contract_digest': 'e' * 64, 'authority_digest': 'd' * 64,
                       'adoption_manifest_digest': 'e' * 64, 'base_sha': 'b' * 40,
                       'promoted_implementation_sha': 'a' * 40, 'promotion_authorization_digest': 'f' * 64,
                       'remote_promotion_receipt_digest': 'a' * 64, 'guard_receipt_digest': 'b' * 64,
                       'verification_receipt_digests': ['c' * 64], 'reviewer_receipt_digest': 'd' * 64,
                       'acceptance_contract_digest': 'e' * 64,
                       'production_acceptance_receipts': [{'class_id': 'UNIT', 'case_ids': ['CP01'], 'run_id': 1,
                           'job_id': 2, 'attempt': 1, 'step': 's', 'artifact_digest': 'f' * 64,
                           'authority_digest': 'd' * 64, 'harness_digest': 'e' * 64, 'package_digest': 'f' * 64,
                           'environment_digest': 'e' * 64, 'permission_digest': 'f' * 64,
                           'executed_cases': ['CP01'], 'passed_cases': ['CP01'], 'failed_cases': [],
                           'skipped_cases': [], 'measurements': [], 'result': 'PASS'}]}
            bound = dict(record, control_plane_receipt=receipt)
            execution_evidence.validate_evidence_record(bound, checkpoint_id='A', checkpoint=checkpoint, policy=demanding)
            broken = copy.deepcopy(bound)
            broken['control_plane_receipt']['promoted_implementation_sha'] = 'b' * 40
            with self.assertRaises(execution_evidence.EvidenceError):
                execution_evidence.validate_evidence_record(broken, checkpoint_id='A', checkpoint=checkpoint, policy=demanding)

    def test_completion_intent_single_commit_and_recovery(self):
        with tempfile.TemporaryDirectory() as directory:
            repo, _ = product_repo(Path(directory))
            plan = json.loads((repo / 'docs/execution/PLAN.json').read_text())
            state = json.loads((repo / 'docs/execution/STATE.json').read_text())
            head = promote_product_candidate(repo)
            intent = {'schema_version': 1, 'checkpoint_id': 'A', 'implementation_sha': head,
                      'evidence_digest': 'e' * 64, 'state_path': 'docs/execution/STATE.json',
                      'evidence_path': 'docs/execution/evidence/A.json'}
            execution_plan.validate_completion_intent(intent, plan, state)
            agent_supervisor.write_completion_intent(repo, intent)
            self.assertEqual(agent_supervisor.reconcile_completion_intent(repo, intent), 'PROCEED')
            stored = agent_supervisor.read_completion_intent(repo)
            self.assertEqual(stored['implementation_sha'], head)
            # Crash after the completion commit but before acknowledgement:
            # the recorded commit exists locally and remotely untouched.
            (repo / 'docs/execution/evidence').mkdir(parents=True, exist_ok=True)
            (repo / 'docs/execution/evidence/A.json').write_text(json.dumps({'implementation_sha': head}))
            git(repo, 'add', 'docs/execution/evidence/A.json')
            git(repo, 'commit', '-qm', 'chore(execution): complete A')
            completion = git(repo, 'rev-parse', 'HEAD')
            recorded = dict(intent, recorded_completion_sha=completion)
            agent_supervisor.write_completion_intent(repo, recorded)
            self.assertEqual(agent_supervisor.reconcile_completion_intent(repo, recorded), 'PUSH_RECORDED')
            # The recorded commit is pushed: adopted, never a second commit.
            git(repo, 'push', '-q', 'origin', 'main:main')
            self.assertEqual(agent_supervisor.reconcile_completion_intent(repo, recorded), 'ADOPTED')
            # Back to the implementation with the evidence file present but
            # uncommitted: finalize must refuse before any mutation.
            git(repo, 'reset', '-q', '--hard', head)
            (repo / 'docs/execution/evidence').mkdir(parents=True, exist_ok=True)
            (repo / 'docs/execution/evidence/A.json').write_text(json.dumps({'implementation_sha': head}))
            before = git(repo, 'rev-list', '--count', 'HEAD')
            with self.assertRaisesRegex(agent_supervisor.SupervisorError, 'completion evidence already exists'):
                agent_supervisor.finalize_verified_checkpoint(
                    repo, plan=plan, state=state,
                    checkpoint=next(item for item in plan['checkpoints'] if item['id'] == 'A'),
                    evidence_result={'policy': self.hosted(repo)[0], 'record': self.completion_record(head),
                                     'api': FakeGitHubApi(head, [])},
                    implementation_sha=head, api=FakeGitHubApi(head, []), run_local_checks=False)
            self.assertEqual(git(repo, 'rev-list', '--count', 'HEAD'), before)
            # Dirty evidence binding another SHA is preserved for diagnosis, never blessed.
            (repo / 'docs/execution/evidence/A.json').write_text(json.dumps({'implementation_sha': 'b' * 40}))
            with self.assertRaises(agent_supervisor.SupervisorError):
                agent_supervisor.reconcile_completion_intent(repo, recorded)
            # Unknown remote state diagnoses with intent retained. The force
            # push only simulates an external writer; promotion never forces.
            (repo / 'docs/execution/evidence/A.json').write_text(json.dumps({'implementation_sha': head}))
            git(repo, 'add', 'docs/execution/evidence/A.json')
            git(repo, 'commit', '-qm', 'drift')
            subprocess.run(['git', '-C', str(repo), 'push', '-q', '--force', 'origin', 'main:main'], check=True)
            other = git(repo, 'rev-parse', 'HEAD')
            with self.assertRaises(agent_supervisor.SupervisorError):
                agent_supervisor.reconcile_completion_intent(repo, recorded)
            self.assertEqual(git(repo, 'rev-parse', 'HEAD'), other)

    def test_hosted_mismatch_refuses_and_state_stays_next(self):
        with tempfile.TemporaryDirectory() as directory:
            repo, _ = product_repo(Path(directory))
            plan = json.loads((repo / 'docs/execution/PLAN.json').read_text())
            checkpoint = next(item for item in plan['checkpoints'] if item['id'] == 'A')
            policy, _ = self.hosted(repo)
            gate = {'gate_id': 'repository_hygiene', 'workflow_file': '.github/workflows/repo-hygiene.yml',
                    'workflow_name': 'Repository hygiene', 'required_jobs': ['Repository hygiene']}
            api = FakeGitHubApi('b' * 40, [gate])
            ticks = iter([0.0, 10 ** 9])
            with self.assertRaises(execution_evidence.EvidenceError):
                agent_supervisor.verify_hosted_checkpoint(repo, plan, checkpoint, 'a' * 40, 'fixture subject',
                                                          api=api, clock=lambda: next(ticks), sleep=lambda s: None)
            state = json.loads((repo / 'docs/execution/STATE.json').read_text())
            self.assertEqual(state['current_next'], 'A')
            self.assertFalse((repo / 'docs/execution/evidence/A.json').exists())


if __name__ == '__main__':
    unittest.main(verbosity=2)
