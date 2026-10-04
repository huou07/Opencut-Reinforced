#!/usr/bin/env python3
"""Real claim/CLI durability tests; READY resets are fixtures, never reconciliation."""
from __future__ import annotations
import copy
from dataclasses import replace
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / 'scripts'))
from model_orchestrator import contracts as c, store as s, orchestrator as o
from model_orchestrator.tests.test_contracts import shared_provenance, valid_task
from model_orchestrator.tests.test_lifecycle import bootstrap_dict, cli


def task_for(authority):
    payload = c._release_authority(authority)
    build = payload['build']
    task = valid_task()
    task.update(task_id=build['task_id'], checkpoint_id='M3', base_sha=build['base_sha'],
                candidate_branch=build['candidate_branch'], authority_digest=payload['git']['authority_digest'])
    return task


def reset_ready_test_only(runtime, task_id):
    """No process is ever launched here. Production requires real reconciliation."""
    current = runtime.inspect()
    def reset(state):
        state['tasks'][task_id]['status'] = 'READY'
        state['tasks'][task_id]['stage'] = None
        state['active_task'] = None
    runtime.transaction(current['sequence'], current['epoch'], reset)


def settle_for_review_test_only(runtime, task_id):
    """The execution tests launch no process; live verification proves real settlement."""
    current = runtime.inspect()
    def settle(state):
        state['tasks'][task_id]['status'] = 'SETTLED'
        state['tasks'][task_id]['stage'] = None
        state['active_task'] = None
    runtime.transaction(current['sequence'], current['epoch'], settle)


class ClaimAttemptExecutionTests(unittest.TestCase):
    def context(self, directory, authority=None):
        self.fixture = shared_provenance()
        authority = authority or self.fixture.authorities['M3']
        task = task_for(authority)
        runtime = s.RuntimeStore(Path(directory).resolve() / 'runtime', authority)
        runtime.initialize()
        contract_digest = runtime.register_task(task, copy.deepcopy(task))
        return authority, task, runtime, contract_digest

    def claim(self, runtime, task, contract_digest, name):
        with runtime.lock('task', task['task_id']):
            return runtime.claim(task['task_id'], contract_digest, owner_nonce='owner-' + name,
                                 boot_identity='boot', stage_id=name, stage_nonce='nonce-' + name)

    def test_actual_cli_claim_and_direct_restart_share_one_durable_episode(self):
        with tempfile.TemporaryDirectory() as directory:
            authority, task, runtime, contract_digest = self.context(directory)
            parent = Path(directory).resolve()
            bootstrap = parent / 'bootstrap.json'
            bootstrap.write_text(json.dumps(bootstrap_dict(self.fixture.bootstrap('M3'))))
            base = ['--bootstrap', str(bootstrap), '--candidate-root', str(self.fixture.candidate),
                    '--controller-root', str(self.fixture.controller), '--runtime', str(runtime.root)]
            code, payload = cli(*base, 'claim', '--task-id', task['task_id'], '--owner', 'owner-one',
                                '--boot', 'boot', '--stage-id', 'different-model-one', '--stage-nonce', 'nonce-one')
            self.assertEqual((code, payload['status']), (0, 'OK'))
            first = o.AttemptLedger.load(runtime).entries
            self.assertEqual(len(first), 1)
            self.assertEqual(first[0]['kind'], 'initial')
            self.assertEqual(first[0]['lease_epoch'], runtime.inspect()['tasks'][task['task_id']]['lease_epoch'])
            reset_ready_test_only(runtime, task['task_id'])
            restarted = s.RuntimeStore(runtime.root, authority)
            second_stage = self.claim(restarted, task, contract_digest, 'different-model-two')
            ledger = o.AttemptLedger.load(restarted)
            self.assertEqual(len(ledger.entries), 2)
            self.assertEqual({entry['episode'] for entry in ledger.entries}, {first[0]['episode']})
            self.assertEqual(ledger.entries[-1]['lease_epoch'], second_stage['lease_epoch'])
            reset_ready_test_only(restarted, task['task_id'])
            self.claim(restarted, task, contract_digest, 'renamed-stage-third')
            self.assertEqual([entry['kind'] for entry in o.AttemptLedger.load(restarted).entries],
                             ['initial', 'speculative', 'speculative'])
            reset_ready_test_only(restarted, task['task_id'])
            before = restarted.inspect()
            with self.assertRaises(o.OrchestratorError) as caught:
                self.claim(restarted, task, contract_digest, 'renamed-stage-four')
            self.assertEqual(caught.exception.code, 'REQUIRE_CAUSAL')
            self.assertEqual(restarted.inspect(), before)
            code, payload = cli(*base, 'claim', '--task-id', task['task_id'], '--owner', 'owner-four',
                                '--boot', 'boot', '--stage-id', 'renamed-stage-four', '--stage-nonce', 'nonce-four')
            self.assertEqual((code, payload['status']), (2, 'REFUSED'))
            self.assertIn('causal diagnosis', payload['reason'])
            self.assertEqual(restarted.inspect(), before)

    def test_malformed_persisted_attempt_refuses_actual_claim(self):
        with tempfile.TemporaryDirectory() as directory:
            _, task, runtime, contract_digest = self.context(directory)
            current = runtime.inspect()
            record = dict(schema_version=1, kind='attempt', payload=dict(schema_version=1,
                          task_id=task['task_id'], episode='gate', kind='speculative',
                          evidence_digest=None, lease_epoch='malformed'))
            runtime.transaction(current['sequence'], current['epoch'], lambda state: None, objects=[record])
            before = runtime.inspect()
            with self.assertRaises(c.ContractError):
                self.claim(runtime, task, contract_digest, 'no-bypass')
            self.assertEqual(runtime.inspect(), before)

    def test_pause_race_and_object_crash_publish_no_illegal_claim(self):
        with tempfile.TemporaryDirectory() as directory:
            _, task, runtime, contract_digest = self.context(directory)
            original = o.claim_attempt
            def paused_after_admission(store, admitted_task, lease_epoch):
                attempt = original(store, admitted_task, lease_epoch)
                store.set_paused(True)
                return attempt
            with patch.object(o, 'claim_attempt', paused_after_admission):
                with self.assertRaises(s.StoreError):
                    self.claim(runtime, task, contract_digest, 'pause-race')
            self.assertTrue(runtime.inspect()['paused'])
            self.assertEqual(runtime.inspect()['tasks'][task['task_id']]['status'], 'READY')
            self.assertEqual(o.AttemptLedger.load(runtime).entries, [])
            runtime.set_paused(False)
            original_transaction = runtime.transaction
            def crash_transaction(*args, **kwargs):
                def fault(point):
                    if point == 'after_object_fsync':
                        raise RuntimeError('injected claim object crash')
                return original_transaction(*args, **kwargs, fault=fault)
            with patch.object(runtime, 'transaction', crash_transaction):
                with self.assertRaises(RuntimeError):
                    self.claim(runtime, task, contract_digest, 'crash')
            self.assertEqual(runtime.inspect()['tasks'][task['task_id']]['status'], 'READY')
            self.assertEqual(o.AttemptLedger.load(runtime).entries, [])
            restarted = s.RuntimeStore(runtime.root, runtime.authority)
            stage = self.claim(restarted, task, contract_digest, 'recovered')
            self.assertEqual(len(o.AttemptLedger.load(restarted).entries), 1)
            self.assertEqual(stage['lease_epoch'], 1)

    def test_review_gate_is_durable_distinct_and_does_not_change_worker_lease(self):
        with tempfile.TemporaryDirectory() as directory:
            authority, task, runtime, contract_digest = self.context(directory)
            worker = self.claim(runtime, task, contract_digest, 'worker')
            settle_for_review_test_only(runtime, task['task_id'])
            before = runtime.inspect()
            task_state = copy.deepcopy(before['tasks'][task['task_id']])
            claims = []
            for number in (1, 2, 3):
                restarted = s.RuntimeStore(runtime.root, authority)
                with restarted.lock('task', task['task_id']):
                    claims.append(o.claim_review(restarted, task['task_id']))
            self.assertEqual([claim['kind'] for claim in claims], ['initial', 'speculative', 'speculative'])
            self.assertEqual(len({claim['attempt_digest'] for claim in claims}), 3)
            self.assertEqual(len({claim['claim_sequence'] for claim in claims}), 3)
            self.assertEqual({claim['lease_epoch'] for claim in claims}, {worker['lease_epoch']})
            self.assertEqual(runtime.inspect()['epoch'], before['epoch'])
            self.assertEqual(runtime.inspect()['tasks'][task['task_id']], task_state)
            ledger = o.AttemptLedger.load(runtime)
            self.assertEqual(ledger.speculative_used(claims[0]['episode']), 2)
            self.assertNotEqual(ledger.entries[0]['episode'], claims[0]['episode'])
            before_refusal = runtime.inspect()
            with runtime.lock('task', task['task_id']), self.assertRaises(o.OrchestratorError) as caught:
                o.claim_review(runtime, task['task_id'])
            self.assertEqual(caught.exception.code, 'REQUIRE_CAUSAL')
            self.assertEqual(runtime.inspect(), before_refusal)

    def test_review_pause_race_publishes_no_opportunity(self):
        with tempfile.TemporaryDirectory() as directory:
            _, task, runtime, contract_digest = self.context(directory)
            worker = self.claim(runtime, task, contract_digest, 'worker')
            settle_for_review_test_only(runtime, task['task_id'])
            before = o.AttemptLedger.load(runtime).entries
            original = o.claim_attempt
            def paused_after_admission(store, admitted_task, lease_epoch, **kwargs):
                attempt = original(store, admitted_task, lease_epoch, **kwargs)
                store.set_paused(True)
                return attempt
            with patch.object(o, 'claim_attempt', paused_after_admission), runtime.lock('task', task['task_id']):
                with self.assertRaises(s.StoreError):
                    o.claim_review(runtime, task['task_id'])
            self.assertTrue(runtime.inspect()['paused'])
            self.assertEqual(o.AttemptLedger.load(runtime).entries, before)
            self.assertEqual(runtime.inspect()['tasks'][task['task_id']]['lease_epoch'], worker['lease_epoch'])
            self.assertEqual(runtime.inspect()['tasks'][task['task_id']]['status'], 'SETTLED')

    def test_actual_cli_review_bad_prerequisites_spend_no_review_opportunity(self):
        with tempfile.TemporaryDirectory() as directory:
            _, task, runtime, contract_digest = self.context(directory)
            self.claim(runtime, task, contract_digest, 'worker')
            settle_for_review_test_only(runtime, task['task_id'])
            parent = Path(directory).resolve()
            bootstrap = parent / 'bootstrap.json'
            bootstrap.write_text(json.dumps(bootstrap_dict(self.fixture.bootstrap('M3'))))
            before = runtime.inspect()
            code, payload = cli('--bootstrap', str(bootstrap), '--candidate-root', str(self.fixture.candidate),
                '--controller-root', str(self.fixture.controller), '--runtime', str(runtime.root),
                'review', '--task-id', task['task_id'], '--task-path', 'controller/missing-task.json',
                '--catalog-path', 'controller/missing-catalog.json', '--guard-root', str(parent / 'missing-guard'),
                '--attempt-dir', str(parent / 'missing-attempt'), '--enrollment', str(parent / 'missing-enrollment'),
                '--implementation-family', 'fixture-worker', '--opencode-bin', '/nonexistent/opencode',
                '--opencode-sha', '0' * 64, '--opencode-version', '0.0.0', '--docker-bin', '/nonexistent/docker',
                '--docker-sha', '0' * 64, '--endpoint', 'unix:///nonexistent/docker.sock',
                '--storage-root', str(parent / 'storage'), '--image', 'sha256:' + '0' * 64,
                '--limits', '1,536870912,64,1073741824,1073741824,1048576,60')
            self.assertEqual((code, payload['status']), (2, 'REFUSED'))
            self.assertEqual(runtime.inspect(), before)

    def test_only_independently_pinned_exact_causal_diagnosis_permits_causal_claim_after_two_corrections(self):
        fixture = shared_provenance()
        task = task_for(fixture.authorities['M3'])
        episode = o.AttemptLedger.episode_id('M3', 'worker', c.canonical_json(sorted(task['required_check_ids'])))
        with tempfile.TemporaryDirectory() as directory:
            parent = Path(directory).resolve()
            controller = parent / 'controller'
            subprocess.run(['git', 'clone', '-q', str(fixture.controller), str(controller)], check=True)
            fixture.git(controller, 'remote', 'set-url', 'origin', 'https://github.com/' + c.REPOSITORY_IDENTITY + '.git')
            diagnostic_process = subprocess.run(['/bin/sh', '-c', 'exit 7'])
            evidence = dict(argv=['/bin/sh', '-c', 'exit 7'], exit_code=diagnostic_process.returncode,
                            boundary='test-only independent controller diagnosis')
            diagnostic_digest = c.canonical_digest(evidence)
            diagnostic_path = controller / 'controller/diagnostics' / (diagnostic_digest + '.json')
            diagnostic_path.parent.mkdir(parents=True)
            diagnostic_path.write_text(json.dumps(evidence))
            causal = dict(task_contract_digest=c.canonical_digest(task), episode=episode,
                          reviewer_family='fixture-independent', implementation_family='fixture-worker',
                          reviewer_session='independent-test-session', failure_evidence_digest=diagnostic_digest,
                          falsifiable_cause='Explicit shell exit causes this diagnostic failure',
                          discriminating_result='Separate diagnostic subprocess exited 7',
                          patch_explanation='Test admission only; no worker is launched')
            (controller / ('controller/causal-' + episode + '.json')).write_text(json.dumps(causal))
            reviewer_episode = o.AttemptLedger.episode_id('M3', 'reviewer', c.canonical_json(sorted(task['required_check_ids'])))
            (controller / ('controller/causal-' + reviewer_episode + '.json')).write_text(json.dumps(dict(causal, episode=reviewer_episode)))
            source = fixture.commit(controller, 'fixture: independently pinned causal diagnosis')
            bootstrap = replace(fixture.bootstrap('M3'), source_sha=source)
            authority = c.load_release_authority(fixture.candidate, controller, bootstrap=bootstrap)
            _, admitted, runtime, contract_digest = self.context(parent, authority)
            self.assertEqual(c.canonical_digest(admitted), causal['task_contract_digest'])
            for number in (1, 2, 3, 4):
                self.claim(runtime, admitted, contract_digest, 'stage-' + str(number))
                reset_ready_test_only(runtime, admitted['task_id'])
            self.assertEqual([entry['kind'] for entry in o.AttemptLedger.load(runtime).entries],
                             ['initial', 'speculative', 'speculative', 'causal'])
            before = runtime.inspect()
            with self.assertRaises(o.OrchestratorError) as caught:
                self.claim(runtime, admitted, contract_digest, 'stage-five')
            self.assertEqual(caught.exception.code, 'ESCALATE')
            self.assertEqual(runtime.inspect(), before)
            settle_for_review_test_only(runtime, admitted['task_id'])
            review_claims = []
            for number in (1, 2, 3, 4):
                with runtime.lock('task', admitted['task_id']):
                    review_claims.append(o.claim_review(runtime, admitted['task_id']))
            self.assertEqual([claim['kind'] for claim in review_claims], ['initial', 'speculative', 'speculative', 'causal'])
            with runtime.lock('task', admitted['task_id']), self.assertRaises(o.OrchestratorError) as review_refusal:
                o.claim_review(runtime, admitted['task_id'])
            self.assertEqual(review_refusal.exception.code, 'ESCALATE')
            # A later writable/uncommitted diagnosis cannot replace the exact source pin.
            (controller / ('controller/causal-' + episode + '.json')).write_text(json.dumps(dict(causal, reviewer_family='fixture-worker')))
            with self.assertRaises(c.ContractError):
                runtime.inspect()


if __name__ == '__main__':
    unittest.main(verbosity=2)
