"""Focused strict routing, exclusion, provenance and read-only checks."""
import copy
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

from scripts import document_routing as routing
from scripts.execution_plan import checkpoint_for_id, load_plan_state

ROOT = Path(__file__).resolve().parents[1]


class RoutingTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        (self.root / 'docs/features').mkdir(parents=True)
        (self.root / 'docs/execution').mkdir()
        (self.root / 'AGENTS.md').write_text('# Rules\nSafety\n')
        (self.root / 'docs/feature.md').write_text('# Feature\n## Relevant\nexact\n### Detail\nrequired\n```md\n## Decoy\n```\n## Unrelated\nsecret feature\n')
        (self.root / 'docs/execution/PLAN.json').write_text('{"checkpoints": []}')
        (self.root / 'docs/execution/STATE.json').write_text('{}')
        self.manifest = {'schema_version': 1, 'core': ['rules'],
                         'documents': {'rules': {'path': 'AGENTS.md'}, 'feature': {'path': 'docs/feature.md', 'heading': 'Relevant'}},
                         'features': {'viewer': ['feature']}, 'phase_features': {}, 'checkpoint_features': {},
                         'on_demand': [], 'budgets': {'core': 20000, 'feature': 30000, 'context': 50000}}

    def write(self, value=None):
        (self.root / routing.MANIFEST).write_text(json.dumps(self.manifest if value is None else value))

    def test_strict_schema_and_references(self):
        mutations = [lambda m: m.update(schema_version=True), lambda m: m.update(schema_version=2),
                     lambda m: m.update(extra=1), lambda m: m['features'].update(viewer=['missing']),
                     lambda m: m['features'].update(viewer=['feature', 'feature']),
                     lambda m: m['phase_features'].update({'7': ['unknown']}),
                     lambda m: m['checkpoint_features'].update({'7F': ['unknown']}),
                     lambda m: m['budgets'].update(core=True), lambda m: m['budgets'].update(context=0),
                     lambda m: m.update(on_demand=[{}]), lambda m: m.update(on_demand=['AGENTS.md', 'AGENTS.md']),
                     lambda m: m['documents']['feature'].update(heading=3),
                     lambda m: m['documents']['feature'].update(extra='bad'),
                     lambda m: m['documents'].update(duplicate={'path': 'AGENTS.md'}),
                     lambda m: m['features'].update(core=['feature'])]
        for mutate in mutations:
            with self.subTest(mutate=mutate):
                m = copy.deepcopy(self.manifest); mutate(m); self.write(m)
                with self.assertRaises(routing.RoutingError): routing.load(self.root)

    def test_duplicate_json_nonfinite_and_malformed(self):
        for text in ['{"schema_version":1,"schema_version":1}', '{"x": NaN}', '{"x": Infinity}', '{broken']:
            with self.subTest(text=text):
                (self.root / routing.MANIFEST).write_text(text)
                with self.assertRaises(routing.RoutingError): routing.load(self.root)

    def test_path_boundary(self):
        for path in ['../AGENTS.md', '/AGENTS.md', 'docs/../AGENTS.md', 'docs//feature.md', 'docs\\feature.md', 'C:/doc.md', 'docs/missing.md']:
            with self.subTest(path=path):
                m = copy.deepcopy(self.manifest); m['documents']['feature']['path'] = path; self.write(m)
                with self.assertRaises(routing.RoutingError): routing.load(self.root)
        (self.root / 'docs/link.md').symlink_to(self.root / 'AGENTS.md')
        self.manifest['documents']['feature'] = {'path': 'docs/link.md'}; self.write()
        with self.assertRaises(routing.RoutingError): routing.load(self.root)

    def test_exact_heading_and_fenced_decoy(self):
        self.write(); manifest = routing.load(self.root)
        bundle = routing.resolve(self.root, manifest, ['viewer'])
        text = bundle['files'][1]['excerpts'][0]['text']
        self.assertIn('required', text); self.assertIn('## Decoy', text)
        self.assertNotIn('secret feature', text)
        self.assertEqual(bundle['bytes'], sum(len(e['text'].encode()) for f in bundle['files'] for e in f['excerpts']))
        self.manifest['documents']['feature']['heading'] = 'Missing'; self.write()
        with self.assertRaises(routing.RoutingError): routing.load(self.root)
        self.manifest['documents']['feature']['heading'] = 'Relevant'; self.write()
        with (self.root / 'docs/feature.md').open('a') as f: f.write('## Relevant\ncollision\n')
        with self.assertRaises(routing.RoutingError): routing.load(self.root)

    def test_overlap_merges_authoritative_bytes(self):
        self.manifest['documents']['whole'] = {'path': 'docs/feature.md'}
        self.manifest['features']['full'] = ['whole']; self.write()
        bundle = routing.resolve(self.root, routing.load(self.root), ['viewer', 'full', 'viewer'])
        self.assertEqual(bundle['file_count'], 2)
        self.assertEqual(bundle['files'][1]['bytes'], (self.root / 'docs/feature.md').stat().st_size)

    def test_unknown_bundle_refuses(self):
        self.write()
        with self.assertRaisesRegex(routing.RoutingError, 'unknown feature'): routing.resolve(self.root, routing.load(self.root), ['typo'])

    def test_links_orphans_and_status(self):
        self.write()
        (self.root / 'docs/features/orphan.md').write_text('[missing](../missing.md)\n')
        (self.root / 'AGENTS.md').write_text('Status: NEXT\n')
        errors, _, _ = routing.validate(self.root, routing.load(self.root))
        self.assertTrue(any('missing.md' in e for e in errors))
        self.assertTrue(any('orphan' in e for e in errors))
        self.assertTrue(any('mutable execution status' in e for e in errors))

    def test_observed_semantic_regressions_in_global_entrypoints(self):
        for claim in ('export is not implemented', 'Android SAF is unavailable',
                      'current project schema is v5', 'viewer is not implemented'):
            with self.subTest(claim=claim):
                (self.root / 'AGENTS.md').write_text(claim)
                self.assertTrue(routing.narrative_errors(self.root))
        (self.root / 'AGENTS.md').write_text('Safety rules')
        (self.root / 'docs/RELEASE.md').write_text('export is not implemented')
        self.assertTrue(routing.narrative_errors(self.root))
        (self.root / 'docs/RELEASE.md').unlink()
        (self.root / 'AGENTS.md').write_text('Consult STATE and evidence for capabilities.')
        self.assertEqual(routing.narrative_errors(self.root), [])

    def test_readme_inventory_and_dropped_historical_provenance_refuse(self):
        (self.root / 'README.md').write_text('# Project\n## Implemented today\nInventory\n')
        self.assertTrue(routing.narrative_errors(self.root))
        (self.root / 'README.md').unlink()
        (self.root / 'docs/TESTING.md').write_text('# Tests\n## Coverage\nUnbound snapshot\n')
        self.assertTrue(routing.narrative_errors(self.root))
        (self.root / 'docs/TESTING.md').write_text('# Tests\n## Status\nUnbound snapshot\n')
        self.assertTrue(routing.narrative_errors(self.root))

    def test_budgets_warn_without_truncation(self):
        self.manifest['budgets'] = {'core': 1, 'feature': 1, 'context': 1}; self.write()
        errors, warnings, _ = routing.validate(self.root, routing.load(self.root))
        self.assertEqual(errors, []); self.assertEqual(len(warnings), 2)
        bundle = routing.resolve(self.root, routing.load(self.root), ['viewer'])
        self.assertTrue(bundle['over_budget']); self.assertIn('required', bundle['files'][1]['excerpts'][0]['text'])


class RepositoryRoutingTests(unittest.TestCase):
    def setUp(self):
        self.manifest = routing.load(ROOT)
        self.plan, self.state = load_plan_state(ROOT)

    def test_repository_routes_links_and_budgets(self):
        errors, _, reports = routing.validate(ROOT, self.manifest)
        self.assertEqual(errors, [])
        self.assertTrue(all(v['bytes'] > 0 and v['file_count'] > 0 for v in reports.values()))

    def test_representative_context_exclusions(self):
        cases = [('7F', ['viewer', 'media-runtime']), ('9C', ['ui']), (None, ['release']),
                 (None, ['model-orchestrator', 'model-orchestrator-m1'])]
        for checkpoint_id, features in cases:
            with self.subTest(case=features):
                cp = checkpoint_for_id(self.plan, checkpoint_id) if checkpoint_id else None
                bundle = routing.resolve(ROOT, self.manifest, features, cp)
                self.assertEqual(bundle, routing.resolve(ROOT, self.manifest, reversed(features), cp))
                text = ''.join(e['text'] for f in bundle['files'] for e in f['excerpts'])
                paths = [f['path'] for f in bundle['files']]
                if checkpoint_id: self.assertIn(cp['spec_document'], paths)
                if features == ['viewer', 'media-runtime']:
                    self.assertNotIn('AI providers and local inference', text)
                    self.assertNotIn('Declarative MotionScene', text)
                    self.assertNotIn('docs/RELEASE.md', paths)
                if features == ['ui']: self.assertNotIn('docs/TECHNICAL_PLAN.md', paths)
                if features[0] == 'model-orchestrator':
                    self.assertNotIn('docs/TECHNICAL_PLAN.md', paths); self.assertNotIn('DESIGN.md', paths)

    def test_defaults_exact_contract_and_unknown_ids(self):
        for checkpoint in self.plan['checkpoints']:
            bundle = routing.resolve(ROOT, self.manifest, checkpoint=checkpoint)
            self.assertTrue(bundle['features'], checkpoint['id'])
            contract = next(f for f in bundle['files'] if f['path'] == checkpoint['spec_document'])
            text = ''.join(e['text'] for e in contract['excerpts'])
            self.assertIn(checkpoint['id'] + ' — ', text)
            self.assertIn('## Stop conditions', text)
            self.assertIn('## Handoff', text)
            lines = (ROOT / checkpoint['spec_document']).read_text().splitlines(keepends=True)
            heading = next(title for _, _, title in routing._headings(lines) if title.startswith(checkpoint['id'] + ' — '))
            source, (start, end) = routing.section(ROOT, {'path': checkpoint['spec_document'], 'heading': heading})
            self.assertIn(''.join(source[start:end]), text)
            for peer in self.plan['checkpoints']:
                if peer['spec_document'] == checkpoint['spec_document'] and peer['id'] != checkpoint['id']:
                    self.assertNotIn(peer['id'] + ' — ', text)
        with self.assertRaises(ValueError): checkpoint_for_id(self.plan, 'UNKNOWN')

    def test_cli_read_only_and_explicit_unknown_refusal(self):
        protected = ['docs/execution/PLAN.json', 'docs/execution/STATE.json', routing.MANIFEST]
        before = {p: hashlib.sha256((ROOT / p).read_bytes()).hexdigest() for p in protected}
        status = subprocess.check_output(['git', 'status', '--porcelain'], cwd=ROOT)
        command = [sys.executable, str(ROOT / 'scripts/execution_plan.py')]
        result = subprocess.run(command + ['context', '7F', '--json'], capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn('documentation', json.loads(result.stdout))
        result = subprocess.run(command + ['docs', '--features', 'typo'], capture_output=True, text=True)
        self.assertNotEqual(result.returncode, 0); self.assertIn('unknown feature', result.stderr)
        result = subprocess.run(command + ['docs', 'guessed task'], capture_output=True, text=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(status, subprocess.check_output(['git', 'status', '--porcelain'], cwd=ROOT))
        self.assertEqual(before, {p: hashlib.sha256((ROOT / p).read_bytes()).hexdigest() for p in protected})


if __name__ == '__main__':
    unittest.main()
