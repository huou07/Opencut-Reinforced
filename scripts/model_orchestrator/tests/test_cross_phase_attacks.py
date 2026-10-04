#!/usr/bin/env python3
"""Combined admission attacks against real disposable integration repositories.

Fixture settlement/check receipts establish prerequisites only; these negative
attacks do not certify a live worker, reviewer, product, or adopted release.
"""
from __future__ import annotations
import shutil
import tempfile
import unittest
from pathlib import Path

from model_orchestrator import promotion as p
from model_orchestrator.tests.test_promotion_and_handoff import (
    BASE, CountingRunner, authorize, candidate_commit, git, integration_repo,
    m4_store, settle_test_only,
)


class CrossPhaseAttackTests(unittest.TestCase):
    def test_pause_after_promotion_intent_prevents_real_push(self):
        fixture, _, runtime, task, _ = m4_store(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, runtime.root.parent, True)
        settle_test_only(runtime, task['task_id'])
        with tempfile.TemporaryDirectory() as directory:
            parent = Path(directory)
            repo, url = integration_repo(parent, fixture)
            candidate = candidate_commit(repo)
            authorization_digest, _ = authorize(runtime, task, candidate)
            runner = CountingRunner()

            def pause(point):
                if point == 'before_push':
                    runtime.set_paused(True)

            with self.assertRaises(p.PromotionError):
                p.promote(store=runtime, task_id=task['task_id'],
                          authorization_digest=authorization_digest,
                          integration_repo=repo, remote='origin', expected_remote_url=url,
                          hooks_dir=parent / 'hooks', push_runner=runner, fault=pause)
            self.assertEqual(runner.calls, [])
            self.assertEqual(git(repo, 'ls-remote', url, 'refs/heads/main').split()[0], BASE)
            current = runtime.inspect()
            self.assertTrue(current['paused'])
            _, intent = p._latest_intent(runtime, task['task_id'])
            self.assertEqual(intent['outcome'], 'PROMOTING')
            self.assertEqual(p.load_authorization(runtime, authorization_digest)['candidate_sha'], candidate)
            self.assertEqual(git(repo, 'rev-parse', 'HEAD'), BASE)


if __name__ == '__main__':
    unittest.main(verbosity=2)
