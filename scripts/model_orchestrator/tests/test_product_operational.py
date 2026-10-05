"""CP01–05/33: externally pinned product admission and separate base lineage.

Synthetic certification records here are explicitly unit fixtures, never
production authority. The live acceptance requires real adopted release pins.
"""
import copy
from dataclasses import asdict, replace
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch
from types import SimpleNamespace

from model_orchestrator import contracts as c, product as op, store as s, sandbox as b, guards as g, orchestrator as o
from model_orchestrator.tests.test_contracts import shared_provenance, valid_task, REPO_ROOT, DIGEST
from model_orchestrator.tests.test_promotion_and_handoff import git


def clone(source, target, revision=None):
    subprocess.run(['git', 'clone', '-q', '--no-hardlinks', str(source), str(target)], check=True)
    git(target, 'remote', 'set-url', 'origin', 'https://github.com/' + c.REPOSITORY_IDENTITY + '.git')
    if revision:
        git(target, 'checkout', '-q', '--detach', revision)


class ProductFixture:
    """Independent operator-owned product inputs, never an OR checkpoint implementation."""
    def __init__(self, parent, *, candidate=None, controller=None, bootstrap=None, toy=False,
                 image='sha256:' + 'a' * 64, docker='/usr/bin/docker', docker_digest=DIGEST,
                 endpoint='unix:///run/user/1000/docker.sock', enrollments=()):
        self.root = Path(parent).resolve()
        self.root.mkdir(parents=True, exist_ok=True)
        if candidate is None:
            fixture = shared_provenance()
            candidate, controller, bootstrap = fixture.candidate, fixture.controller, fixture.bootstrap('M5-full', certification=True, adoption=True)
        self.candidate, self.controller, self.product = (self.root / name for name in ('control', 'controller', 'product-base'))
        clone(candidate, self.candidate, bootstrap.release_sha)
        clone(controller, self.controller, bootstrap.source_sha)
        clone(REPO_ROOT, self.product, c.TRUSTED_DESIGN_BASE)
        if toy:
            checkpoint = dict(id='OR-V2-FIXTURE', phase=9, title='Isolated operational acceptance',
                spec_document='docs/execution/phases/OR_V2_FIXTURE.md', prerequisite_checkpoint_ids=[],
                expected_project_schema_effect_category='none', expected_ipc_effect_category='none',
                required_evidence_classes=['UNIT'], runner_allowed_protected_paths=[])
            plan = {'schema_version': 1, 'checkpoints': [checkpoint]}
            state = {'schema_version': 1, 'current_next': checkpoint['id'], 'checkpoints': {checkpoint['id']: 'NEXT'},
                     'verified_contract_versions': dict(project_schema=7, recovery_schema=1, ipc_protocol=1)}
            (self.product / checkpoint['spec_document']).write_text('# Isolated V2 product-checkpoint fixture\n\nWrite only operational-fixture.txt with the exact marker. A real isolated worker, immutable rootless verifier and independent readonly reviewer must pass before a local bare promotion and supervisor handoff. No actual product checkpoint, no 9B work, and no real product main publication. UNIT evidence is required; versions remain 7/1/1.\n')
            for name, record in (('PLAN', plan), ('STATE', state)):
                (self.product / ('docs/execution/' + name + '.json')).write_text(c.canonical_json(record) + '\n')
            git(self.product, 'add', '-A'); git(self.product, 'commit', '-qm', 'fixture: isolated product checkpoint plan')
        self.base = git(self.product, 'rev-parse', 'HEAD')
        self.bare = self.root / 'publication.git'
        subprocess.run(['git', 'init', '-q', '--bare', str(self.bare)], check=True)
        git(self.product, 'push', '-q', str(self.bare), self.base + ':refs/heads/main')
        self.bootstrap = bootstrap
        release = c.load_release_authority(self.candidate, self.controller, bootstrap=bootstrap)
        facts = op.release_active(release)
        manifest = op.operational_manifest(release, self.product, self.base)
        plan = json.loads((self.product / 'docs/execution/PLAN.json').read_text())
        state = json.loads((self.product / 'docs/execution/STATE.json').read_text())
        checkpoint = next(row for row in plan['checkpoints'] if row['id'] == state['current_next'])
        marker = '# OPERATIONAL_PRODUCT_FIXTURE\n'
        self.marker = marker
        self.harness_path = 'controller/product/harness.sh'
        harness = ('#!/bin/sh -e\n'
                   'test "$(cat /candidate/operational-fixture.txt)" = "# OPERATIONAL_PRODUCT_FIXTURE"\n'
                   'printf \'{"cases":[{"id":"case1","result":"PASS"}]}\\n\'\n')
        hdigest = hashlib.sha256(harness.encode()).hexdigest()
        task = valid_task()
        task.update(task_id='operational-product-fixture', task_kind='product_checkpoint',
            checkpoint_id=checkpoint['id'], base_sha=self.base, candidate_branch='fixture/operational-product',
            authority_digest=manifest['authority_digest'], allowed_paths=['operational-fixture.txt'],
            required_documentation=[checkpoint['spec_document']],
            goal='Write exactly the isolated fixture marker, commit only operational-fixture.txt.',
            out_of_scope=['No actual product checkpoint implementation', 'No product main publication'],
            permission_expectation='Rootless worker; no controller/base write authority',
            production_boundary='Explicit isolated product-checkpoint fixture with local bare publication',
            error_cases=['Wrong marker', 'Unauthorized source edit', 'Wrong task pin'],
            observable_outcome='An independent verifier checks the worker edit before local bare promotion',
            persistence_expectation='Durable controller receipts, immutable product input',
            expected_result='Exact marker and read-only independent PASS review; no other files changed',
            evidence_classes=checkpoint['required_evidence_classes'], harness_digest=hdigest,
            role_enrollment_ids=[c.canonical_digest(record) for record in enrollments] or ['unit-fixture'],
            resource_limits=dict(cpu=1, memory_bytes=1 << 30, pids=64, disk_bytes=52 << 30,
                                 output_bytes=1 << 20, wall_seconds=600),
            budget=dict(tokens=200000, cost_microusd=100000, wall_seconds=1800, tool_calls=200,
                        speculative_corrections=2, causal_repairs=1))
        task['check_argv'][0].update(argv=['/bin/sh', '-e', '/verifier/' + self.harness_path],
            harness_digest=hdigest, timeout_seconds=300, resource_limits=copy.deepcopy(task['resource_limits']))
        task['acceptance_cases'] = ['fixture-' + cls for cls in task['evidence_classes']]
        task['acceptance_requirements'] = [dict(class_id=cls, case_ids=['fixture-' + cls],
            production_boundary=task['production_boundary'], environment_digest=task['check_argv'][0]['environment_digest'],
            package_digest=c.canonical_digest(image), permission_digest=c.canonical_digest('rootless readonly verifier'),
            persistence_expectation=task['persistence_expectation'], harness_digest=hdigest) for cls in task['evidence_classes']]
        self.task = task
        self.catalog = dict(schema_version=1, git=dict(executable='/usr/bin/git', digest=hashlib.sha256(Path('/usr/bin/git').read_bytes()).hexdigest()),
            checks={'unit': dict(boundary='rootless', executable='/bin/sh', executable_digest=c.canonical_digest([image, '/bin/sh']),
                harness=self.harness_path, harness_digest=hdigest, image=image, docker=docker,
                docker_digest=docker_digest, endpoint=endpoint)},
            cases={'case1': dict(path='scripts/execution_plan.py', symbol='validate_plan')},
            harnesses=[], executables=[], scratch_paths=['cache'],
            acceptance={req['class_id']: dict(requirement_digest=c.canonical_digest(req), harness=self.harness_path,
                boundary='rootless', check_ids=['unit'], case_bindings={req['case_ids'][0]: 'case1'}) for req in task['acceptance_requirements']},
            performance={})
        self.task_path, self.catalog_path = 'controller/product/task.json', 'controller/product/catalog.json'
        self.auth = dict(schema_version=1, repository=c.REPOSITORY_IDENTITY, purpose='OPERATOR_PRODUCT_TASK',
            authorization_id=bootstrap.authorization_id, nonce='isolated-product-fixture-nonce', sequence=1,
            release_sha=bootstrap.release_sha, adoption_digest=c.canonical_digest(facts['adoption']), product_base_sha=self.base,
            plan_digest=hashlib.sha256((self.product / 'docs/execution/PLAN.json').read_bytes()).hexdigest(),
            state_digest=hashlib.sha256((self.product / 'docs/execution/STATE.json').read_bytes()).hexdigest(),
            checkpoint_id=task['checkpoint_id'], task_path=self.task_path, task_digest=c.canonical_digest(task),
            catalog_path=self.catalog_path, catalog_digest=c.canonical_digest(self.catalog),
            execution_profile='ISOLATED_FIXTURE', destination_url=str(self.bare))
        self.auth_path = 'controller/product/authorization.json'
        for name, record in ((self.task_path, task), (self.catalog_path, self.catalog), (self.auth_path, self.auth)):
            path = self.controller / name; path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(c.canonical_json(record) + '\n')
        (self.controller / self.harness_path).write_text(harness)
        git(self.controller, 'add', '-A'); git(self.controller, 'commit', '-qm', 'fixture: externally pinned isolated product task')
        self.source = git(self.controller, 'rev-parse', 'HEAD')
        self.bootstrap = replace(bootstrap, source_sha=self.source)
        self.pin = c.RecordPin(self.auth_path, c.canonical_digest(self.auth))
        self.authority = self.load()

    def load(self, *, bootstrap=None, pin=None):
        return op.load_operational_authority(self.candidate, self.controller, self.product,
            bootstrap=bootstrap or self.bootstrap, product_pin=pin or self.pin)

    def malformed(self, auth=None, task=None, catalog=None):
        auth = copy.deepcopy(auth or self.auth)
        for path, value, field in ((self.task_path, task, 'task_digest'), (self.catalog_path, catalog, 'catalog_digest')):
            if value is not None:
                (self.controller / path).write_text(c.canonical_json(value) + '\n')
                auth[field] = c.canonical_digest(value)
        (self.controller / self.auth_path).write_text(c.canonical_json(auth) + '\n')
        git(self.controller, 'add', '-A'); git(self.controller, 'commit', '-qm', 'fixture: conflicting operator pin')
        source = git(self.controller, 'rev-parse', 'HEAD')
        try:
            return self.load(bootstrap=replace(self.bootstrap, source_sha=source), pin=c.RecordPin(self.auth_path, c.canonical_digest(auth)))
        finally:
            git(self.controller, 'checkout', '-q', '--detach', self.source)


class ProductAuthorityTests(unittest.TestCase):
    """CP01–05/33: operational trust and exact product admission refusal matrix."""
    @classmethod
    def setUpClass(cls):
        cls.directory = tempfile.TemporaryDirectory(prefix='or-v2-product-authority-')
        cls.fixture = ProductFixture(cls.directory.name)

    @classmethod
    def tearDownClass(cls):
        cls.directory.cleanup()

    def test_real_main_lineage_and_exact_task_admission(self):
        fixture = self.fixture
        self.assertEqual(fixture.base, '915a4a8b951643e475683e4cdf56996118ec7d1e')
        result = subprocess.run(['git', 'merge-base', '--is-ancestor', fixture.bootstrap.release_sha, fixture.base], cwd=fixture.candidate)
        self.assertNotEqual(result.returncode, 0)
        for capability in c.IMPLEMENTATION_PHASES[1:5]:
            facts = c.validate_phase_admission(fixture.authority, fixture.task, capability=capability)
            self.assertEqual(facts['git']['base_oid'], fixture.base)
        with tempfile.TemporaryDirectory() as directory:
            runtime = s.RuntimeStore(Path(directory).resolve() / 'runtime', fixture.authority)
            runtime.initialize(); runtime.register_task(fixture.task, copy.deepcopy(fixture.task))
            candidate = b.create_candidate(fixture.product, Path(directory).resolve() / 'candidate', fixture.base, authority=fixture.authority)
            self.assertEqual(git(candidate.root, 'rev-parse', 'HEAD'), fixture.base)
            self.assertFalse((candidate.root / 'scripts/model_orchestrator/product.py').exists())
            runtime.register_candidate(fixture.task['task_id'], candidate)
            self.assertEqual(runtime.candidate_descriptor(fixture.task['task_id'])['base_oid'], fixture.base)
        floor = g.load_floor(fixture.authority, fixture.task_path, fixture.catalog_path)
        self.assertEqual(floor.task['checkpoint_id'], '9B')

    def test_fresh_cli_uses_real_product_pin_and_cannot_accept_raw_mode(self):
        fixture = self.fixture
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            bootstrap = root / 'bootstrap.json'
            bootstrap.write_text(c.canonical_json(dict(asdict(fixture.bootstrap), product_task_pin=asdict(fixture.pin))))
            argv = [os.sys.executable, '-m', 'model_orchestrator', '--bootstrap', str(bootstrap),
                '--candidate-root', str(fixture.candidate), '--controller-root', str(fixture.controller),
                '--product-root', str(fixture.product), '--runtime', str(root / 'runtime'), 'admit',
                '--task', str(fixture.controller / fixture.task_path), '--template', str(fixture.controller / fixture.task_path)]
            proc = subprocess.run(argv, env=dict(os.environ, PYTHONPATH=str(REPO_ROOT / 'scripts')), capture_output=True)
            self.assertEqual(proc.returncode, 0, proc.stdout.decode() + proc.stderr.decode())
            self.assertEqual(json.loads(proc.stdout)['status'], 'OK')
            candidate_argv = argv[:argv.index('admit')] + ['candidate', '--task-id', fixture.task['task_id'],
                '--destination', str(root / 'candidate')]
            proc = subprocess.run(candidate_argv, env=dict(os.environ, PYTHONPATH=str(REPO_ROOT / 'scripts')),
                                  capture_output=True)
            self.assertEqual(proc.returncode, 0, proc.stdout.decode() + proc.stderr.decode())
            self.assertEqual(json.loads(proc.stdout)['base_sha'], fixture.base)
            proc = subprocess.run(candidate_argv, env=dict(os.environ, PYTHONPATH=str(REPO_ROOT / 'scripts')),
                                  capture_output=True)
            self.assertEqual(proc.returncode, 2, 'duplicate registration must refuse without replacing the candidate')
            forged = dict(asdict(fixture.bootstrap), execution='OPERATIONAL_PRODUCT_TASK')
            bootstrap.write_text(c.canonical_json(forged))
            proc = subprocess.run(argv, env=dict(os.environ, PYTHONPATH=str(REPO_ROOT / 'scripts')), capture_output=True)
            self.assertEqual(proc.returncode, 2)
            self.assertEqual(json.loads(proc.stdout)['status'], 'REFUSED')

    def test_review_admits_exact_product_task_before_verification(self):
        fixture = self.fixture
        floor = g.load_floor(fixture.authority, fixture.task_path, fixture.catalog_path)
        self.assertNotEqual(floor.task['task_id'], c._release_authority(fixture.authority)['build']['task_id'])
        guarded = SimpleNamespace(verify=lambda: {'vetoes': []})
        arguments = dict(authority=fixture.authority, floor=floor, guarded=guarded,
            attempt_dir=fixture.root / 'unexecuted-attempt', enrollment=None, binary=None,
            agent=None, box=None, image=None, container_binary=None,
            implementation_family='muse', limits=None)
        # Stop at the real verifier boundary: this test grants no PASS receipt.
        with patch.object(o.v, 'readiness', side_effect=RuntimeError('verification boundary')) as readiness:
            with self.assertRaisesRegex(RuntimeError, '^verification boundary$'):
                o.run_reviewer(**arguments)
            readiness.assert_called_once_with(guarded, arguments['attempt_dir'])
        wrong = copy.deepcopy(floor.task); wrong['task_id'] = 'unapproved-review-task'
        with patch.object(o.v, 'readiness') as readiness, self.assertRaises(c.ContractError):
            o.run_reviewer(**dict(arguments, floor=SimpleNamespace(task=wrong)))
        readiness.assert_not_called()

    def test_disabled_build_or_unadopted_release_refuses(self):
        for field in ('adoption', 'certification'):
            with self.subTest(field=field), self.assertRaises(c.ContractError):
                self.fixture.load(bootstrap=replace(self.fixture.bootstrap, **{field: None}))
        with self.assertRaises(c.ContractError):
            c.validate_phase_admission(shared_provenance().authorities['M3'], self.fixture.task, capability='M1')

    def test_wrong_release_base_adoption_next_and_digest_refuse(self):
        for key, value in (('release_sha', c.TRUSTED_DESIGN_BASE), ('product_base_sha', self.fixture.bootstrap.release_sha),
                           ('adoption_digest', '0' * 64), ('checkpoint_id', '9B1'), ('task_digest', '0' * 64),
                           ('catalog_digest', '0' * 64), ('plan_digest', '0' * 64), ('state_digest', '0' * 64)):
            auth = copy.deepcopy(self.fixture.auth); auth[key] = value
            with self.subTest(field=key), self.assertRaises(c.ContractError):
                self.fixture.malformed(auth=auth)

    def test_wrong_scope_and_reduced_evidence_refuse_even_with_new_pin(self):
        for key, value in (('allowed_paths', ['docs/execution/STATE.json']), ('allowed_paths', ['scripts/model_orchestrator/product.py']), ('allowed_paths', ['.opencode/agents/orch-worker.md']), ('ipc_effect', 'explicit-contract-gate'),
                           ('evidence_classes', ['UNIT']), ('required_documentation', ['AGENTS.md']),
                           ('task_kind', 'control_plane_phase')):
            task = copy.deepcopy(self.fixture.task); task[key] = value
            with self.subTest(field=key), self.assertRaises(c.ContractError):
                self.fixture.malformed(task=task)

    def test_unpinned_task_cannot_replace_authorized_packet(self):
        for key, value in (('goal', 'Implement another checkpoint'), ('allowed_paths', ['crates/or_core/src/project_document.rs']),
                           ('base_sha', self.fixture.bootstrap.release_sha)):
            task = copy.deepcopy(self.fixture.task); task[key] = value
            with self.subTest(field=key), self.assertRaises(c.ContractError):
                c.validate_phase_admission(self.fixture.authority, task, capability='M1')
        with self.assertRaises(c.ContractError):
            self.fixture.load(pin={'path': self.fixture.pin.path, 'digest': self.fixture.pin.digest})
        with self.assertRaises(c.ContractError):
            g.load_floor(self.fixture.authority, 'controller/other-task.json', self.fixture.catalog_path)

    def test_stale_or_dirty_materialization_refuses(self):
        for root in (self.fixture.candidate, self.fixture.controller, self.fixture.product):
            path = root / 'untrusted.txt'; path.write_text('drift')
            try:
                with self.subTest(root=root), self.assertRaises(c.ContractError):
                    c.validate_shared_capability(self.fixture.authority, 'M1')
            finally:
                path.unlink()
        with self.assertRaises(c.ContractError):
            self.fixture.load(bootstrap=replace(self.fixture.bootstrap, source_sha=shared_provenance().source))

    def test_fixture_cannot_select_github_and_product_cannot_change_plan(self):
        auth = copy.deepcopy(self.fixture.auth); auth['destination_url'] = 'https://github.com/' + c.REPOSITORY_IDENTITY + '.git'
        with self.assertRaises(c.ContractError): self.fixture.malformed(auth=auth)
        auth['execution_profile'] = 'PRODUCT'; auth['destination_url'] = str(self.fixture.bare)
        with self.assertRaises(c.ContractError): self.fixture.malformed(auth=auth)

    def test_product_symlink_root_refuses(self):
        with tempfile.TemporaryDirectory() as directory:
            link = Path(directory).resolve() / 'product-link'; link.symlink_to(self.fixture.product, target_is_directory=True)
            with self.assertRaises(c.ContractError):
                op.load_operational_authority(self.fixture.candidate, self.fixture.controller, link,
                    bootstrap=self.fixture.bootstrap, product_pin=self.fixture.pin)

    def test_candidate_cannot_act_as_external_controller(self):
        with self.assertRaises(c.ContractError):
            op.load_operational_authority(self.fixture.candidate, self.fixture.candidate, self.fixture.product,
                bootstrap=self.fixture.bootstrap, product_pin=self.fixture.pin)

    def test_new_admission_requires_publication_main_exact_base(self):
        fixture = self.fixture
        git(fixture.bare, 'update-ref', 'refs/heads/main', fixture.base + '^')
        try:
            with tempfile.TemporaryDirectory() as directory:
                runtime = s.RuntimeStore(Path(directory).resolve() / 'runtime', fixture.authority); runtime.initialize()
                with self.assertRaises(c.ContractError): runtime.register_task(fixture.task, fixture.task)
        finally:
            git(fixture.bare, 'update-ref', 'refs/heads/main', fixture.base)


class HostedProductReceiptTests(unittest.TestCase):
    """CP05/43: artifact parser fixtures never stand in for real hosted acceptance."""
    def test_downloaded_artifact_and_gate_bindings_are_required(self):
        import io
        import zipfile
        from unittest.mock import patch
        task = valid_task()
        row = dict(class_id='UNIT', case_ids=['journey1'], run_id=10, job_id=20, attempt=1, step='Measured cases',
            artifact_digest=DIGEST, authority_digest=task['authority_digest'], harness_digest=DIGEST,
            package_digest=DIGEST, environment_digest=DIGEST, permission_digest=DIGEST,
            executed_cases=['journey1'], passed_cases=['journey1'], failed_cases=[], skipped_cases=[],
            measurements=[], result='PASS')
        observation = dict(task_id=task['task_id'], task_contract_digest=c.canonical_digest(task), candidate_sha='b' * 40,
            **{key: row[key] for key in ('class_id', 'authority_digest', 'harness_digest', 'package_digest',
                'environment_digest', 'permission_digest', 'executed_cases', 'passed_cases', 'failed_cases',
                'skipped_cases', 'measurements', 'result')})
        data = c.canonical_json(observation).encode()
        row['artifact_digest'] = hashlib.sha256(data).hexdigest()
        buffer = io.BytesIO()
        with zipfile.ZipFile(buffer, 'w') as archive:
            archive.writestr('production-receipt.json', c.canonical_json(row))
            archive.writestr('observation.json', data)
        archive_bytes = buffer.getvalue()
        proof = dict(class_name='unused', **{'class': 'UNIT'}, run_id=10, job_id=20, step_name='Measured cases')
        record = dict(implementation_sha='b' * 40, evidence_classes=[proof])
        responses = {'/actions/runs/10': dict(head_sha='b' * 40, run_attempt=1, repository={'full_name': c.REPOSITORY_IDENTITY}, conclusion='success'),
            '/actions/runs/10/artifacts?per_page=100': {'artifacts': [dict(name='or-v2-product-' + task['task_id'] + '-UNIT', expired=False,
                workflow_run=dict(head_sha='b' * 40), id=30, digest='sha256:' + hashlib.sha256(archive_bytes).hexdigest())]},
            '/actions/artifacts/30/zip': archive_bytes}
        def request(path):
            value = responses[path.removeprefix('/repos/' + c.REPOSITORY_IDENTITY)]
            return value if isinstance(value, bytes) else c.canonical_json(value).encode()
        facts = dict(operational=dict(authorization={'execution_profile': 'PRODUCT'}, task=task))
        # Mock only the preceding loader boundary; real authority is tested in
        # ProductAuthorityTests. No runtime, promotion or state write is possible.
        with patch.object(c, '_release_authority', return_value=facts):
            op.validate_hosted_product_receipts(None, {'production_acceptance_receipts': [row]}, record, request=request)
            for field, value in (('job_id', 21), ('attempt', 2), ('case_ids', ['other']),
                                 ('harness_digest', '0' * 64), ('artifact_digest', '0' * 64)):
                changed = copy.deepcopy(row); changed[field] = value
                with self.subTest(field=field), self.assertRaises(c.ContractError):
                    op.validate_hosted_product_receipts(None, {'production_acceptance_receipts': [changed]}, record, request=request)
            responses['/actions/runs/10']['head_sha'] = 'a' * 40
            with self.assertRaises(c.ContractError):
                op.validate_hosted_product_receipts(None, {'production_acceptance_receipts': [row]}, record, request=request)


if __name__ == '__main__':
    unittest.main()
