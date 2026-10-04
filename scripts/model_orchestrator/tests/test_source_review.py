#!/usr/bin/env python3
"""CP48 trust/input negative tests; no model launches and no certification claims."""
from __future__ import annotations
import copy
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / 'scripts'))
from model_orchestrator import contracts as c, orchestrator as o
from model_orchestrator.tests.test_contracts import shared_provenance, valid_task
from model_orchestrator.tests.test_lifecycle import reviewer_enrollment


class SourceArchiveTests(unittest.TestCase):
    def test_archive_exact_source_inventory_and_bytes(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            repo = root / 'repo'
            repo.mkdir()
            subprocess.run(['git', 'init', '-q', str(repo)], check=True)
            (repo / 'small.py').write_text('# exact released source\nvalue = 1\n')
            subprocess.run(['git', '-C', str(repo), 'add', 'small.py'], check=True)
            subprocess.run(['git', '-C', str(repo), '-c', 'user.name=Fixture', '-c',
                            'user.email=fixture@example.invalid', 'commit', '-qm', 'fixture'], check=True)
            sha = subprocess.check_output(['git', '-C', str(repo), 'rev-parse', 'HEAD']).decode().strip()
            identity = o._source_review_archive(repo, sha, root / 'source')
            self.assertEqual(identity['release_sha'], sha)
            self.assertEqual(set(identity['source_files']), {'small.py'})
            self.assertEqual((root / 'source/small.py').read_bytes(), (repo / 'small.py').read_bytes())
            self.assertFalse((root / 'source/small.py').stat().st_mode & 0o222)
            with self.assertRaises(o.OrchestratorError):
                o._source_review_archive(repo, 'HEAD', root / 'other')
            with self.assertRaises(o.OrchestratorError):
                o._source_review_archive(repo, sha, root / 'source')

    def test_archive_export_ignore_cannot_hide_released_source(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            repo = root / 'repo'
            repo.mkdir()
            subprocess.run(['git', 'init', '-q', str(repo)], check=True)
            (repo / 'hidden.py').write_text('# source must remain visible\n')
            (repo / '.gitattributes').write_text('hidden.py export-ignore\n')
            subprocess.run(['git', '-C', str(repo), 'add', '-A'], check=True)
            subprocess.run(['git', '-C', str(repo), '-c', 'user.name=Fixture', '-c',
                            'user.email=fixture@example.invalid', 'commit', '-qm', 'fixture'], check=True)
            sha = subprocess.check_output(['git', '-C', str(repo), 'rev-parse', 'HEAD']).decode().strip()
            with self.assertRaises(o.OrchestratorError):
                o._source_review_archive(repo, sha, root / 'source')


class SourceTraceTests(unittest.TestCase):
    def report(self):
        return dict(verdict='PASS', coverage=list(o.SOURCE_REVIEW_STAGES),
                    findings=[dict(id=stage, severity='NONBLOCKING', classification='PROVEN',
                                   citation='source/core.py:2; evidence/receipt.json:1',
                                   claim='Exact source and controller evidence examined')
                              for stage in o.SOURCE_REVIEW_STAGES])

    def test_complete_trace_and_exact_lines_required(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'source').mkdir()
            (root / 'evidence').mkdir()
            (root / 'source/core.py').write_text('# source\nvalue=1\n')
            (root / 'evidence/receipt.json').write_text('{"result":"PASS"}\n')
            report = self.report()
            o._validate_source_review_trace(report, root)
            mutations = [lambda r: r['coverage'].remove('handoff'),
                         lambda r: r['findings'].pop(),
                         lambda r: r['findings'][0].update(citation='source/core.py:200; evidence/receipt.json:1'),
                         lambda r: r['findings'][0].update(citation='source/missing.py:1; evidence/receipt.json:1'),
                         lambda r: r['findings'][0].update(citation='source/core.py:1'),
                         lambda r: r['findings'][0].update(citation='source/../outside.py:1; evidence/receipt.json:1'),
                         lambda r: r['findings'][0].update(classification='UNKNOWN')]
            for mutate in mutations:
                with self.subTest(mutation=mutate):
                    changed = copy.deepcopy(report)
                    mutate(changed)
                    with self.assertRaises(o.OrchestratorError):
                        o._validate_source_review_trace(changed, root)


class SourceToolsTests(unittest.TestCase):
    def test_only_read_glob_grep_may_appear(self):
        for tool in ('read', 'glob', 'grep'):
            o._validate_source_review_tools([{'type': 'tool_use', 'part': {'type': 'tool', 'tool': tool}}])
        for tool in ('bash', 'task', 'edit', 'webfetch', 'unknown'):
            with self.subTest(tool=tool), self.assertRaises(o.OrchestratorError):
                o._validate_source_review_tools([{'type': 'tool_use', 'part': {'type': 'tool', 'tool': tool}}])
        with self.assertRaises(o.OrchestratorError):
            o._validate_source_review_tools([{'type': 'tool_use', 'part': {'tool': 'bash'}}])


class SourceReviewBindingTests(unittest.TestCase):
    def test_other_task_enrollment_cannot_enter_source_review(self):
        fixture = shared_provenance()
        authority = fixture.authorities['M5']
        payload = c._release_authority(authority)
        task = valid_task()
        task.update(task_id=payload['build']['task_id'], checkpoint_id='M5',
                    base_sha=payload['build']['base_sha'], candidate_branch=payload['build']['candidate_branch'],
                    authority_digest=payload['git']['authority_digest'])
        with self.assertRaises(o.OrchestratorError) as caught:
            o.run_source_review(authority=authority, task=task, source_repo=ROOT,
                                release_sha='a' * 40, evidence_root=ROOT,
                                enrollment=reviewer_enrollment().with_reasoning('HIGH'), binary=None,
                                box=None, image='', container_binary='', implementation_family='fixture-a',
                                storage_root=ROOT, observation_dir=ROOT, limits=None)
        self.assertIn('enrollment task binding differs', str(caught.exception))


if __name__ == '__main__':
    unittest.main()
