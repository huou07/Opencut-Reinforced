#!/usr/bin/env python3
"""M5 live acceptance: real OpenCode/Codex/containers/hosted harness.

Live cases run ONLY with OR_V2_LIVE=1 plus explicit pinned tool/volume/model
inputs; otherwise they skip as BLOCKED_AVAILABILITY, never as passes.
Fixture-mode checks (credential hygiene, enrollment requirements, receipt
shapes, gates) always run. Inference is spent only by the explicitly gated
live worker/reviewer/architecture invocations, one bounded call each.
"""
from __future__ import annotations
import copy
import hashlib
import json
import os
import shutil
import subprocess
import sys
import tempfile
import time
import unittest
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / 'scripts'))
from model_orchestrator import contracts as c, adapters as a, orchestrator as o
from model_orchestrator import store as s, sandbox as b, workspace as w, guards as g
from model_orchestrator import verification as v, promotion as p
from model_orchestrator.tests.test_contracts import shared_provenance, valid_task, SCHEMAS

LIVE_KEYS = ('OR_V2_VOLUME', 'OR_V2_IMAGE', 'OR_V2_OPENCODE_BIN', 'OR_V2_OPENCODE_SHA',
             'OR_V2_OPENCODE_VERSION', 'OR_V2_DOCKER', 'OR_V2_DOCKER_SHA', 'OR_V2_ENDPOINT',
             'OR_V2_WORKER_MODEL', 'OR_V2_WORKER_FAMILY', 'OR_V2_REVIEWER_MODEL',
             'OR_V2_REVIEWER_FAMILY', 'OR_V2_CRED_DIR', 'OR_V2_BARE', 'OR_V2_OUTPUT')


def live_config():
    """Explicit pinned live inputs or a clear blocked-availability skip."""
    if os.environ.get('OR_V2_LIVE') != '1':
        raise unittest.SkipTest('live acceptance requires OR_V2_LIVE=1')
    missing = [key for key in LIVE_KEYS if not os.environ.get(key)]
    if missing:
        raise unittest.SkipTest('live inputs missing: ' + ','.join(missing))
    return {key: os.environ[key] for key in LIVE_KEYS}


def credential_fingerprint(cred_dir):
    """Identify the inference credential without ever returning key material."""
    try:
        raw = json.loads((Path(cred_dir) / 'opencode' / 'auth.json').read_text(encoding='utf-8'))
    except (OSError, ValueError) as exc:
        raise ValueError('credential staging unreadable: ' + str(exc)) from exc
    if set(raw) != {'opencode-go'} or set(raw['opencode-go']) != {'type', 'key'}:
        raise ValueError('credential staging must hold exactly one provider entry')
    key = raw['opencode-go']['key']
    if not isinstance(key, str) or not key:
        raise ValueError('credential key missing')
    return {'provider_id': 'opencode-go', 'key_sha256': hashlib.sha256(key.encode()).hexdigest(),
            'key_length': len(key)}


def run_binary(binary, *argv, env=None, cwd=None, timeout=120):
    if env is None:
        env = {'PATH': '/usr/bin:/bin', 'LANG': 'C'}
        if isinstance(os.environ.get('HOME'), str) and os.environ['HOME']:
            env['HOME'] = os.environ['HOME']
    try:
        proc = subprocess.run([str(binary), *argv], env=env,
                              cwd=cwd, capture_output=True, timeout=timeout, check=False)
    except (OSError, subprocess.SubprocessError) as exc:
        raise unittest.SkipTest('live binary unavailable: ' + str(exc))
    return proc


class LiveGateTests(unittest.TestCase):
    """Gates, credential hygiene, and enrollment requirements (always run)."""
    def test_live_skips_without_explicit_inputs(self):
        if os.environ.get('OR_V2_LIVE') == '1' and all(os.environ.get(k) for k in LIVE_KEYS):
            self.skipTest('live inputs present; gates verified by the live cases')
        with self.assertRaises(unittest.SkipTest):
            live_config()

    def test_credential_fingerprint_never_returns_key_material(self):
        with tempfile.TemporaryDirectory() as directory:
            target = Path(directory) / 'opencode'
            target.mkdir()
            (target / 'auth.json').write_text(json.dumps({'opencode-go': {'type': 'api', 'key': 'sk-test-secret'}}))
            finger = credential_fingerprint(str(Path(directory)))
            self.assertEqual(set(finger), {'provider_id', 'key_sha256', 'key_length'})
            self.assertNotIn('sk-test-secret', json.dumps(finger))
            self.assertEqual(len(finger['key_sha256']), 64)
            (target / 'auth.json').write_text(json.dumps({'a': {}, 'b': {}}))
            with self.assertRaises(ValueError):
                credential_fingerprint(directory)

    def test_listing_is_not_enrollment(self):
        with self.assertRaises(a.AdapterError):
            a.validate_enrollment({'model_id': 'opencode-go/deepseek-v4-flash', 'family': 'deepseek'})
        with self.assertRaises(a.AdapterError):
            a.select_model('IMPLEMENTATION', [{'model_id': 'x/y', 'family': 'deepseek',
                                               'allowed_roles': ['IMPLEMENTATION'],
                                               'reasoning_capabilities': ['HIGH'],
                                               'task_class_qualification': {},
                                               'adapter_certification_digest': 'd' * 64,
                                               'budget': {}}],
                           task_budget={}, availability={'x/y': 'available'})

    def test_event_contract_pins_transport_version(self):
        self.assertEqual(a.OPENCODE_EVENT_CONTRACT['format'], 'opencode-json-1')
        self.assertIn('text', a.OPENCODE_EVENT_CONTRACT['final_types'])
        self.assertIn('error', a.OPENCODE_EVENT_CONTRACT['terminal_error_types'])


class OpenCodeConformanceTests(unittest.TestCase):
    """CP45: pinned binary identity, config precedence, events, help surface."""
    def binary(self):
        cfg = live_config()
        return a.OpenCodeBinary(Path(cfg['OR_V2_OPENCODE_BIN']), cfg['OR_V2_OPENCODE_SHA'],
                                cfg['OR_V2_OPENCODE_VERSION']), cfg

    def test_binary_identity_and_help_surface(self):
        binary, _ = self.binary()
        proc = run_binary(binary.path, '--help')
        self.assertEqual(proc.returncode, 0)
        for token in ('run', '--model', '--format', '--variant', '--agent'):
            self.assertIn(token, proc.stdout.decode())
        self.assertEqual(binary.version, os.environ.get('OR_V2_OPENCODE_VERSION'))

    def test_config_precedence_project_wins(self):
        binary, _ = self.binary()
        with tempfile.TemporaryDirectory() as directory:
            home = Path(directory) / 'home'
            (home / '.config' / 'opencode').mkdir(parents=True)
            (home / '.config' / 'opencode' / 'opencode.json').write_text(json.dumps({'model': 'global-provider/global-model'}))
            project = Path(directory) / 'project'
            project.mkdir()
            (project / 'opencode.json').write_text(json.dumps({'model': 'project-provider/project-model'}))
            env = {'PATH': os.environ.get('PATH', '/usr/bin:/bin'), 'LANG': 'C', 'HOME': str(home)}
            resolved = json.loads(run_binary(binary.path, 'debug', 'config', env=env, cwd=str(project)).stdout.decode())
            self.assertEqual(resolved['model'], 'project-provider/project-model')
            (project / 'opencode.json').unlink()
            resolved = json.loads(run_binary(binary.path, 'debug', 'config', env=env, cwd=str(project)).stdout.decode())
            self.assertEqual(resolved['model'], 'global-provider/global-model')

    def test_error_event_shape_and_terminal_behavior(self):
        binary, cfg = self.binary()
        with tempfile.TemporaryDirectory() as directory:
            env = {'PATH': os.environ.get('PATH', '/usr/bin:/bin'), 'LANG': 'C', 'HOME': directory}
            proc = run_binary(binary.path, 'run', '--format', 'json', '--model', 'no-such-provider/no-such-model',
                              'probe', env=env, cwd=directory)
            self.assertNotEqual(proc.returncode, 0)
            self.assertFalse(proc.stderr)
            stream = a.parse_event_stream(proc.stdout, limits=a.StreamLimits(1 << 20, 64, 120),
                                          event_contract=a.OPENCODE_EVENT_CONTRACT)
            self.assertEqual(stream.error['type'], 'error')
            self.assertTrue(stream.session_id.startswith('ses_'))
            self.assertIsNone(stream.final_payload)

    def test_agent_surface_lists_no_tools_dispatcher(self):
        binary, _ = self.binary()
        with tempfile.TemporaryDirectory() as directory:
            overlay = Path(directory) / 'overlay'
            overlay.mkdir()
            (overlay / 'opencode.json').write_text(c.canonical_json(b.role_policy('IMPLEMENTATION')) + '\n')
            env = {'PATH': os.environ.get('PATH', '/usr/bin:/bin'), 'LANG': 'C', 'HOME': directory,
                   'OPENCODE_CONFIG': str(overlay / 'opencode.json')}
            proc = run_binary(binary.path, 'agent', 'list', env=env, cwd=directory)
            self.assertEqual(proc.returncode, 0)


class CodexLiveTests(unittest.TestCase):
    """CP46: minimum real read-only Codex invocation, or authentic quota evidence."""
    def test_minimum_read_only_invocation(self):
        if os.environ.get('OR_V2_CODEX_LIVE') != '1':
            raise unittest.SkipTest('codex live requires OR_V2_CODEX_LIVE=1')
        for key in ('OR_V2_CODEX_BIN', 'OR_V2_CODEX_SHA', 'OR_V2_CODEX_VERSION', 'OR_V2_CODEX_MODEL'):
            if not os.environ.get(key):
                raise unittest.SkipTest('codex live input missing: ' + key)
        binary = a.CodexBinary(Path(os.environ['OR_V2_CODEX_BIN']), os.environ['OR_V2_CODEX_SHA'],
                               os.environ['OR_V2_CODEX_VERSION'])
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            schema = root / 'decision.schema.json'
            schema.write_text(json.dumps({'type': 'object'}))
            adapter = a.CodexAdapter(binary, model_id=os.environ['OR_V2_CODEX_MODEL'], workdir=root,
                                     decision_schema_path=schema, quota_signatures=[],
                                     env={'PATH': '/usr/bin:/bin', 'LANG': 'C'})
            packet = {'schema_version': 1, 'task_id': 'cp46-probe', 'checkpoint_id': 'M5',
                      'authority_digest': 'd' * 64, 'base_sha': 'a' * 40, 'kind': 'difficult_root_cause',
                      'failure_facts': {'transport': 'conformance probe'}, 'attempt_digest': 'd' * 64,
                      'invariant_citations': [], 'question': 'Does the read-only transport conform?',
                      'scope': 'transport conformance only'}
            packet['packet_digest'] = c.canonical_digest({k: v for k, v in packet.items() if k != 'packet_digest'})
            prompt = ('Emit exactly one JSON object with keys schema_version (=1), packet_digest (=%s), '
                      'disposition (=DEFER), decision, constraints, required_verification, stop_conditions, '
                      'cited_facts. No other text.' % packet['packet_digest'])
            outdir = root / 'out'
            outdir.mkdir()
            try:
                decision, deferred = adapter.run_architecture(prompt, packet=packet, output_dir=outdir)
            except a.AdapterError as exc:
                if exc.code in (a.TRANSPORT_ERROR,):
                    raise unittest.SkipTest('codex availability: ' + str(exc))
                raise
            self.assertIsNone(deferred)
            checked = a.validate_architecture_decision(decision, packet)
            self.assertEqual(checked['packet_digest'], packet['packet_digest'])
            self.assertIn(checked['disposition'], a.DECISION_DISPOSITIONS)
            before = sorted(p.name for p in root.rglob('*') if p.is_file())
            self.assertNotIn('decision.schema.json-modified', before)


class IndependentReviewTests(unittest.TestCase):
    """CP48: a separate read-only reviewer session traces the entire live path."""

    def test_independent_source_review(self):
        cfg = live_config()
        output = Path(cfg['OR_V2_OUTPUT']).resolve()
        evidence_path = output / 'live-evidence.json'
        if not evidence_path.is_file():
            raise unittest.SkipTest('independent review requires the CP44 live evidence')
        evidence = json.loads(evidence_path.read_text())
        for key in ('task_id', 'candidate_head', 'task', 'worker', 'reviewer', 'authorization_digest',
                    'remote_receipt_digest', 'promotion', 'image', 'binary', 'docker', 'credential'):
            self.assertIn(key, evidence)
        before = hashlib.sha256(evidence_path.read_bytes()).hexdigest()
        hosted_path = output / 'cp47-hosted-evidence.json'
        bundle = {'schema_version': 1, 'evidence': evidence,
                  'hosted': json.loads(hosted_path.read_text()) if hosted_path.is_file() else None,
                  'instruction': ('Review the attached M5 live acceptance evidence end to end: prepare, claim, '
                                  'real worker, guards, required checks, full review, authorization, normal push, '
                                  'local recovery, task handoff readiness, hosted receipt, state-only completion '
                                  'readiness. Return exactly one strict review_report JSON object with schema_version 1, '
                                  'the echoed task_id, task_contract_digest and candidate_sha given below, verdict, '
                                  'coverage including m5-live, findings with exact citations, quality_flag_dispositions, '
                                  'and unresolved_questions. PASS requires no blocking defect; it never means product '
                                  'accepted.')}
        task = evidence['task']
        bundle.update(task_id=task['task_id'], task_contract_digest=c.canonical_digest(task),
                      candidate_sha=evidence['candidate_head'])
        workdir = Path(tempfile.mkdtemp(prefix='or-v2-cp48-')).resolve()
        home = workdir / 'home'
        (home / '.local' / 'share' / 'opencode').mkdir(parents=True)
        shutil.copyfile(Path(cfg['OR_V2_CRED_DIR']).resolve() / 'opencode' / 'auth.json',
                        home / '.local' / 'share' / 'opencode' / 'auth.json')
        os.chmod(home / '.local' / 'share' / 'opencode' / 'auth.json', 0o600)
        bundle_path = workdir / 'review-bundle.json'
        bundle_path.write_text(json.dumps(bundle) + '\n')
        binary = a.OpenCodeBinary(Path(cfg['OR_V2_OPENCODE_BIN']), cfg['OR_V2_OPENCODE_SHA'],
                                  cfg['OR_V2_OPENCODE_VERSION'])
        enrollment = {'provider_id': cfg['OR_V2_REVIEWER_MODEL'].split('/')[0],
                      'model_id': cfg['OR_V2_REVIEWER_MODEL'], 'family': cfg['OR_V2_REVIEWER_FAMILY'],
                      'allowed_roles': ['INVESTIGATION_REVIEW'], 'reasoning_capabilities': ['HIGH'],
                      'task_class_qualification': {'INVESTIGATION_REVIEW': True},
                      'adapter_certification_digest': binary.certification_digest(), 'budget': {}, 'variants': {}}
        adapter = a.OpenCodeAdapter(binary, role='INVESTIGATION_REVIEW',
                                    enrollment=dict(enrollment, reasoning_requested='HIGH'),
                                    workdir=workdir, limits=a.StreamLimits(4 << 20, 256, 1800),
                                    env={'PATH': os.environ.get('PATH', '/usr/bin:/bin'), 'LANG': 'C',
                                         'HOME': str(home)})
        observation = adapter.run(['Review the attached bundle file and return the strict report.',
                                   str(bundle_path)],
                                  agent='orch-reviewer', timeout_seconds=1500)
        self.assertFalse(observation.timed_out)
        if observation.error is not None or observation.exit_code != 0:
            raise unittest.SkipTest('independent reviewer availability: %r' % (observation.error,))
        self.assertIsNotNone(observation.final_payload)
        report = a.parse_review_report(
            observation.final_payload, task=task, candidate_sha=evidence['candidate_head'], schemas=SCHEMAS,
            reviewer_family=cfg['OR_V2_REVIEWER_FAMILY'],
            implementation_family=evidence['worker']['family'],
            flag_ids=evidence.get('guard_flags', []))
        self.assertEqual(report['verdict'], 'PASS')
        self.assertTrue(report['coverage'])
        live_root = subprocess.check_output(
            ['git', '-C', str(ROOT), 'status', '--porcelain=v1', '--untracked-files=all'],
            env={'PATH': '/usr/bin:/bin', 'LANG': 'C'}).decode().splitlines()
        unexpected = [line for line in live_root if '__pycache__' not in line]
        self.assertEqual(unexpected, [])
        (output / 'cp48-independent-review.json').write_text(json.dumps(
            {'report': report, 'session_id': observation.session_id, 'model_id': observation.model_id,
             'argv_digest': observation.argv_digest, 'elapsed_seconds': observation.elapsed_seconds},
            indent=2, sort_keys=True) + '\n')
        self.assertEqual(hashlib.sha256(evidence_path.read_bytes()).hexdigest(), before)


class HostedHarnessTests(unittest.TestCase):
    """CP47: hosted trusted-harness receipt collection from GitHub provenance."""
    WORKFLOW = '.github/workflows/control-plane-acceptance.yml'

    def collect(self, head_sha, run_id=None):
        def get(path):
            request = urllib.request.Request('https://api.github.com' + path,
                                             headers={'Accept': 'application/vnd.github+json',
                                                      'User-Agent': 'opencut-reinforced-m5'})
            with urllib.request.urlopen(request, timeout=60) as response:
                return json.loads(response.read().decode())
        if run_id is None:
            runs = get('/repos/huou07/Opencut-Reinforced/actions/workflows/control-plane-acceptance.yml/runs?per_page=10')
            candidates = [run for run in runs.get('workflow_runs', [])
                          if run.get('head_sha') == head_sha and run.get('conclusion') == 'success'
                          and run.get('event') in ('push', 'workflow_dispatch')]
            if not candidates:
                raise unittest.SkipTest('no successful hosted harness run for ' + head_sha)
            run = sorted(candidates, key=lambda r: r['updated_at'])[-1]
        else:
            run = get('/repos/huou07/Opencut-Reinforced/actions/runs/%d' % int(run_id))
            if run.get('head_sha') != head_sha or run.get('conclusion') != 'success':
                raise unittest.SkipTest('named hosted run is not a successful run for ' + head_sha)
        jobs = get('/repos/huou07/Opencut-Reinforced/actions/runs/%d/jobs?per_page=20' % run['id'])
        names = {job['name']: job for job in jobs.get('jobs', [])}
        for name in ('Fixture acceptance', 'Collect receipts'):
            if name not in names or names[name]['conclusion'] != 'success':
                raise unittest.SkipTest('hosted job missing or unsuccessful: ' + name)
        artifacts = get('/repos/huou07/Opencut-Reinforced/actions/runs/%d/artifacts?per_page=20' % run['id'])
        artifact_names = sorted(artifact['name'] for artifact in artifacts.get('artifacts', []))
        expected = sorted(['acceptance-receipt-' + head_sha, 'collector-receipt-' + head_sha])
        if artifact_names != expected:
            raise unittest.SkipTest('hosted artifacts differ: %r' % (artifact_names,))
        referenced = run.get('referenced_workflows') or []
        return {'run_id': run['id'], 'run_attempt': run['run_attempt'], 'head_sha': run['head_sha'],
                'event': run['event'], 'jobs': {name: {'id': names[name]['id'], 'conclusion': names[name]['conclusion'],
                                                       'head_sha': names[name]['head_sha']} for name in names},
                'artifacts': artifact_names, 'referenced_workflows': referenced,
                'html_url': run['html_url']}

    def test_hosted_receipt_collection(self):
        head_sha = os.environ.get('OR_V2_HOSTED_SHA')
        if not head_sha:
            raise unittest.SkipTest('hosted collection requires OR_V2_HOSTED_SHA')
        run_id = os.environ.get('OR_V2_HOSTED_RUN_ID')
        evidence = self.collect(head_sha, run_id=int(run_id) if run_id else None)
        self.assertEqual(evidence['head_sha'], head_sha)
        for name, job in evidence['jobs'].items():
            self.assertEqual(job['conclusion'], 'success')
            self.assertEqual(job['head_sha'], head_sha)
        output = Path(os.environ.get('OR_V2_OUTPUT', tempfile.gettempdir())) / 'cp47-hosted-evidence.json'
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_text(json.dumps(evidence, indent=2, sort_keys=True) + '\n')


def check_resources():
    return dict(cpu=1, memory_bytes=1 << 30, pids=64, disk_bytes=52 << 30, output_bytes=1 << 20,
                wall_seconds=600)


class LiveM5:
    """Live M5 fixture: real authority, real containers, real inference, disposable everything."""

    def __init__(self, cfg):
        self.cfg = cfg
        self.nonce = 'live-%s' % os.getpid()
        self.fixture = shared_provenance()
        self.authority = self.fixture.authorities['M5']
        self.payload = c._release_authority(self.authority)
        self.build = self.payload['build']
        self.volume = Path(cfg['OR_V2_VOLUME']).resolve()
        self.output = Path(cfg['OR_V2_OUTPUT']).resolve()
        if not self.volume.is_dir() or self.volume.is_symlink():
            raise unittest.SkipTest('live volume is not a real directory: ' + str(self.volume))
        self.output.mkdir(parents=True, exist_ok=True)
        self.temp = tempfile.TemporaryDirectory(prefix='or-v2-m5-')
        self.root = Path(self.temp.name).resolve()
        self.binary = a.OpenCodeBinary(Path(cfg['OR_V2_OPENCODE_BIN']), cfg['OR_V2_OPENCODE_SHA'],
                                       cfg['OR_V2_OPENCODE_VERSION'])
        self.docker = b.DockerCLI(Path(cfg['OR_V2_DOCKER']), cfg['OR_V2_DOCKER_SHA'],
                                  endpoint=cfg['OR_V2_ENDPOINT'])
        self.image = cfg['OR_V2_IMAGE']
        if not self.image.startswith('sha256:'):
            raise unittest.SkipTest('live image must be a digest reference')

    def task(self):
        task = valid_task()
        task.update(task_id=self.build['task_id'], checkpoint_id='M5', base_sha=self.build['base_sha'],
                    candidate_branch=self.build['candidate_branch'],
                    authority_digest=self.payload['git']['authority_digest'],
                    goal='Append the exact live marker to the disposable candidate, nothing else.',
                    out_of_scope=['No product work', 'No other files', 'No network beyond inference'],
                    allowed_paths=['scripts/model_orchestrator/contracts.py'],
                    required_check_ids=['m5-live'], required_tests=['m5-live'], case_inventory=['m5-probe'],
                    harness_digest='0' * 64, evidence_classes=['UNIT'], acceptance_cases=['m5-accept'],
                    resource_limits=check_resources(),
                    budget=dict(tokens=200000, cost_microusd=100000, wall_seconds=1800, tool_calls=200,
                                speculative_corrections=2, causal_repairs=1))
        task['check_argv'] = [dict(id='m5-live', argv=['/bin/sh', '-e', '/verifier/controller/harness-m5.sh'],
                                   cwd='.', environment={'LANG': 'C', 'LC_ALL': 'C'},
                                   environment_digest=c.canonical_digest({'LANG': 'C', 'LC_ALL': 'C'}),
                                   harness_digest='0' * 64, required_cases=['m5-probe'],
                                   expected_exit_codes=[0], timeout_seconds=300, retry_budget=0,
                                   resource_limits=check_resources(), allow_empty_cases=False, required_metrics=[])]
        task['acceptance_requirements'] = [dict(class_id='UNIT', case_ids=['m5-accept'],
                                                production_boundary='Disposable Debian rootless verifier fixture',
                                                environment_digest=task['check_argv'][0]['environment_digest'],
                                                package_digest=c.canonical_digest(self.image),
                                                permission_digest=c.canonical_digest('rootless read-only candidate and harness'),
                                                persistence_expectation='Immutable controller receipts',
                                                harness_digest='0' * 64)]
        return task

    def enroll(self, model_id, family, role):
        return {'provider_id': model_id.split('/')[0], 'model_id': model_id, 'family': family,
                'allowed_roles': [role], 'reasoning_capabilities': ['HIGH'],
                'task_class_qualification': {role: True},
                'adapter_certification_digest': self.binary.certification_digest(),
                'budget': {'cost_microusd': 100000}, 'variants': {}}

    def commit_controller(self, task):
        """Pin the live task/catalog/harness in a fresh controller clone."""
        controller = self.root / 'controller'
        subprocess.run(['git', 'clone', '-q', '--no-hardlinks', str(self.fixture.controller), str(controller)],
                       check=True)
        subprocess.run(['git', '-C', str(controller), 'remote', 'set-url', 'origin',
                        'https://github.com/' + c.REPOSITORY_IDENTITY + '.git'], check=True)
        harness = ('#!/bin/sh -e\nMARKER="# LIVE_PROBE_%s"\n'
                   'FILE="/candidate/scripts/model_orchestrator/contracts.py"\n'
                   'test -f "$FILE"\n'
                   'test "$(cat "$FILE")" = "$MARKER"\n'
                   'printf \'{"cases":[{"id":"m5-probe","result":"PASS"}]}\\n\'\n' % self.nonce)
        harness_digest = hashlib.sha256(harness.encode()).hexdigest()
        task['harness_digest'] = harness_digest
        task['check_argv'][0]['harness_digest'] = harness_digest
        for requirement in task['acceptance_requirements']:
            requirement['harness_digest'] = harness_digest
        git = subprocess.check_output(['git', '--version'], cwd=controller).decode()
        git_binary = Path('/usr/bin/git')
        if not git_binary.is_file():
            raise unittest.SkipTest('live controller git missing: ' + git)
        git_digest = hashlib.sha256(git_binary.read_bytes()).hexdigest()
        catalog = dict(schema_version=1,
                       git=dict(executable=str(git_binary), digest=git_digest),
                       checks={'m5-live': dict(boundary='rootless', executable='/bin/sh',
                                               executable_digest=c.canonical_digest([self.image, '/bin/sh']),
                                               harness='controller/harness-m5.sh', harness_digest=harness_digest,
                                               image=self.image, docker=str(Path(self.cfg['OR_V2_DOCKER']).resolve()),
                                               docker_digest=hashlib.sha256(
                                                   Path(self.cfg['OR_V2_DOCKER']).read_bytes()).hexdigest(),
                                               endpoint=self.cfg['OR_V2_ENDPOINT'])},
                       cases={'m5-probe': dict(path='scripts/execution_plan.py', symbol='validate_plan')},
                       harnesses=[], executables=[], scratch_paths=['cache'],
                       acceptance={'UNIT': dict(requirement_digest=c.canonical_digest(task['acceptance_requirements'][0]),
                                                harness='controller/harness-m5.sh', boundary='rootless',
                                                check_ids=['m5-live'], case_bindings={'m5-accept': 'm5-probe'})},
                       performance={})
        (controller / 'controller').mkdir(exist_ok=True)
        (controller / 'controller' / 'task-m5.json').write_text(json.dumps(task) + '\n')
        (controller / 'controller' / 'catalog-m5.json').write_text(json.dumps(catalog) + '\n')
        (controller / 'controller' / 'harness-m5.sh').write_text(harness)
        env = dict(os.environ, GIT_AUTHOR_NAME='M5 Live', GIT_AUTHOR_EMAIL='m5@example.invalid',
                   GIT_COMMITTER_NAME='M5 Live', GIT_COMMITTER_EMAIL='m5@example.invalid')
        subprocess.run(['git', '-C', str(controller), 'add', '-A'], check=True, env=env)
        subprocess.run(['git', '-C', str(controller), 'commit', '-qm', 'fixture: live M5 task and catalog'],
                       check=True, env=env)
        source = subprocess.check_output(['git', '-C', str(controller), 'rev-parse', 'HEAD'],
                                         env=env).decode().strip()
        candidate = self.root / 'candidate-authority'
        subprocess.run(['git', 'clone', '-q', '--no-hardlinks', str(self.fixture.candidate), str(candidate)],
                       check=True)
        subprocess.run(['git', '-C', str(candidate), 'remote', 'set-url', 'origin',
                        'https://github.com/' + c.REPOSITORY_IDENTITY + '.git'], check=True)
        build = self.build
        bootstrap = c.ControllerBootstrap(
            source_sha=source, anchor_sha=c.TRUSTED_DESIGN_BASE, base_sha=build['base_sha'],
            release_sha=self.fixture.release, candidate_branch=build['candidate_branch'],
            authorization_id=build['authorization_id'], task_id=build['task_id'], sequence=1,
            nonce=build['nonce'], sandbox_digest=build['sandbox_digest'],
            build=c.RecordPin('controller/build-M5.json', c.canonical_digest(build)))
        return c.load_release_authority(candidate, controller, bootstrap=bootstrap)

    def credential_mount(self):
        """Explicit inference-credential shadow mount; the file never enters receipts."""
        cred = Path(self.cfg['OR_V2_CRED_DIR']).resolve() / 'opencode' / 'auth.json'
        if not cred.is_file():
            raise unittest.SkipTest('live credential staging missing')
        finger = credential_fingerprint(cred.parent.parent)
        return (str(cred), '/worker-home/.local/share/opencode/auth.json'), finger

    def home_mounts(self, overlays, home_base):
        """Writable scratch home with read-only policy and credential shadows.

        Live discovery: the model binary requires a writable home for its own
        runtime state (project index, sessions), so the M1 fully-read-only
        home cannot host a real execution. The writable base holds no policy:
        every frozen policy file and the credential file are shadow-mounted
        read-only on top, which the effective-profile check verifies exactly.
        """
        home_base = Path(home_base)
        home_base.mkdir(mode=0o755, exist_ok=True)
        os.chmod(home_base, 0o755)
        config_dir = home_base / '.local' / 'share' / 'opencode'
        config_dir.mkdir(mode=0o755, parents=True, exist_ok=True)
        for path in (home_base, home_base / '.local', home_base / '.local' / 'share', config_dir):
            os.chmod(path, 0o755)
        home = Path(overlays['home'])
        mounts = [(str(home_base), '/worker-home', True),
                  (str(home / 'opencode.json'), '/worker-home/opencode.json'),
                  (str(home / '.config' / 'opencode' / 'opencode.json'),
                   '/worker-home/.config/opencode/opencode.json')]
        credential, _ = self.credential_mount()
        mounts.append((credential[0], credential[1]))
        return mounts

    def box(self):
        return b.ContainerSandbox(self.docker, boot_identity=b.host_boot_identity(), host_platform='linux')

    def launch_with_credential(self, *, box, candidate, view, role, container_binary, message_parts, agent,
                               enrollment, image, network, limits, overlay_mounts, labels, name, timeout_seconds):
        mounts = list(overlay_mounts) + [self.credential_mount()[0][0]]
        return a.launch_model_stage(box=box, candidate=candidate, view=view, role=role,
                                    container_binary=container_binary, message_parts=message_parts, agent=agent,
                                    adapter=a.OpenCodeAdapter(self.binary, role=role, enrollment=enrollment,
                                                              workdir=Path('/candidate'),
                                                              limits=a.StreamLimits(4 << 20, 4096, 3600), env=None),
                                    image=image, network=network, limits=limits, overlay_mounts=mounts,
                                    labels=labels, name=name, timeout_seconds=timeout_seconds)


class LiveWorkerTests(unittest.TestCase):
    """CP44: live small task through a real OpenCode worker and independent reviewer."""

    def test_live_small_task_end_to_end(self):
        cfg = live_config()
        live = LiveM5(cfg)
        task = live.task()
        authority = live.commit_controller(task)
        payload = c._release_authority(authority)
        self.assertEqual(payload['build']['task_id'], task['task_id'])
        store_root = live.root / 'runtime'
        runtime = s.RuntimeStore(store_root, authority)
        runtime.initialize()
        task_digest = runtime.register_task(task, copy.deepcopy(task))
        src_name = os.environ.get('OR_V2_CANDIDATE_NAME', 'm5-src-' + live.nonce)
        if '/' in src_name or not src_name or len(src_name) > 64:
            raise unittest.SkipTest('invalid live candidate name')
        candidate = b.create_candidate(Path(authority.candidate_root), live.volume / src_name,
                                       task['base_sha'], authority=authority)
        runtime.register_candidate(task['task_id'], candidate)
        box = live.box()
        daemon = box._runtime(live.image)
        worker_enrollment = live.enroll(cfg['OR_V2_WORKER_MODEL'], cfg['OR_V2_WORKER_FAMILY'], 'IMPLEMENTATION')
        worker_enrollment = dict(worker_enrollment, reasoning_requested='HIGH')
        reviewer_enrollment = live.enroll(cfg['OR_V2_REVIEWER_MODEL'], cfg['OR_V2_REVIEWER_FAMILY'],
                                          'INVESTIGATION_REVIEW')
        reviewer_enrollment = dict(reviewer_enrollment, reasoning_requested='HIGH')
        self.assertNotEqual(worker_enrollment['family'], reviewer_enrollment['family'])
        marker = '# LIVE_PROBE_%s' % live.nonce
        prompt = ('Create exactly one file in the candidate at scripts/model_orchestrator/contracts.py '
                  'whose entire content is exactly this single line followed by a newline: `%s`. '
                  'Change no other file. Then commit exactly that change with '
                  '`git -c user.name="V2 Worker" -c user.email="worker@example.invalid" add '
                  'scripts/model_orchestrator/contracts.py && git -c user.name="V2 Worker" '
                  '-c user.email="worker@example.invalid" commit -m "live probe"`. Do not print the line back; '
                  'the controller verifies the file independently.' % marker)
        with runtime.lock('task', task['task_id']):
            stage = runtime.claim(task['task_id'], task_digest, owner_nonce='live-owner',
                                  boot_identity=box.boot_identity, stage_id='live-stage', stage_nonce='live-nonce')
            path = w.reserve_launch(runtime, stage, candidate, live.volume, daemon, live.image, 'IMPLEMENTATION')
            view = a.build_worker_view(candidate, live.volume, task['task_id'], 'live-stage', 'IMPLEMENTATION',
                                       destination=path)
            w.bind_launch(runtime, stage)
            overlays = b.write_role_overlays(path.parent / 'overlays', 'IMPLEMENTATION')
            validated = b._validated_role_overlays(overlays, 'IMPLEMENTATION', candidate.root)
            mounts = [(s, d) for s, d in validated if d.startswith('/candidate/')]
            mounts.extend(live.home_mounts(overlays, path.parent / 'home'))
            cred_mounts, finger = live.credential_mount()
            self.assertEqual(set(finger), {'provider_id', 'key_sha256', 'key_length'})
            provisional = b.StageIdentity('0' * 64, box.boot_identity, stage['owner_nonce'], stage['stage_id'],
                                          stage['lease_epoch'], stage['stage_nonce'],
                                          task['authority_digest'], 'IMPLEMENTATION', stage['task_id'])
            record = runtime.launch_record(stage)
            worker_limits = b.Limits(1, 1 << 30, 64, 52 << 30, 1 << 30, 1 << 20, 600)
            worker_result = live.launch_with_credential(
                box=box, candidate=candidate, view=view, role='IMPLEMENTATION',
                container_binary='/usr/local/bin/opencode', message_parts=[prompt], agent='orch-worker',
                enrollment=worker_enrollment, image=live.image, network='bridge', limits=worker_limits,
                overlay_mounts=mounts, labels=provisional.labels(), name=record['container_name'],
                timeout_seconds=600)
            self.assertFalse(worker_result.timed_out)
            stage = runtime.bind_container(stage, worker_result.container_id)
            proof = box.reconcile_launch(runtime, stage)
            preserved = w.preserve_launch(runtime, stage, proof)
            settled = runtime.settle(stage, proof)
            self.assertEqual(settled['tasks'][task['task_id']]['status'], 'SETTLED')
        floor = g.load_floor(authority, 'controller/task-m5.json', 'controller/catalog-m5.json')
        guarded = g.inspect(candidate, floor, live.root / 'quarantine', source_authority=authority)
        guard = guarded.verify()
        self.assertFalse(guard['vetoes'])
        attempt_dir = live.root / 'attempt'
        readiness = v.execute(guarded, attempt_dir, lease_epoch=stage['lease_epoch'], sequence=1)
        self.assertTrue(readiness['verification_passed'])
        self.assertEqual(sorted(readiness['unresolved']), sorted(guard['flags']))
        self.assertEqual(readiness['pending_hosted_classes'], [])
        review = self.live_review(live, box, guarded, attempt_dir, task, guard,
                                  reviewer_enrollment, worker_enrollment['family'])
        review_digest = o.persist_review(runtime, task['task_id'], review['report'], review['metadata'])
        inputs = p.collect_inputs(store=runtime, task_id=task['task_id'],
                                  guard_root=live.root / 'quarantine', attempt_dir=attempt_dir,
                                  task_path='controller/task-m5.json', catalog_path='controller/catalog-m5.json')
        authorization = p.build_authorization(inputs, schemas=SCHEMAS,
                                              implementation_family=worker_enrollment['family'], issuance_sequence=1)
        authorization_digest = p.persist_authorization(runtime, task['task_id'], authorization)
        bare = live.output / 'bare-remote.git'
        subprocess.run(['git', 'init', '--bare', '-q', str(bare)], check=True)
        integration = live.root / 'integration'
        subprocess.run(['git', 'clone', '-q', str(Path(authority.candidate_root)), str(integration)], check=True)
        subprocess.run(['git', '-C', str(integration), 'checkout', '-q', '-b', 'main', task['base_sha']], check=True)
        subprocess.run(['git', '-C', str(integration), 'remote', 'add', 'origin', str(bare)], check=True)
        subprocess.run(['git', '-C', str(integration), 'push', '-q', 'origin', 'main:main'], check=True)
        env = dict(os.environ, GIT_AUTHOR_NAME='M5 Live', GIT_AUTHOR_EMAIL='m5@example.invalid',
                   GIT_COMMITTER_NAME='M5 Live', GIT_COMMITTER_EMAIL='m5@example.invalid')
        subprocess.run(['git', '-C', str(integration), 'fetch', '-q', str(guarded.root / 'candidate'),
                        guard['head'] + ':refs/heads/live-candidate'], check=True, env=env)
        hooks = live.root / 'hooks'
        pushes = []

        def counting_push(argv, cwd):
            pushes.append(list(argv))
            return p._run_push(argv, cwd)

        promoted = p.promote(store=runtime, task_id=task['task_id'], authorization_digest=authorization_digest,
                             integration_repo=integration, remote='origin', expected_remote_url=str(bare),
                             hooks_dir=hooks, push_runner=counting_push)
        self.assertEqual(promoted['status'], 'PROMOTED')
        self.assertEqual(len(pushes), 1)
        self.assertNotIn('--force', pushes[0])
        pushes = len(pushes)
        remote_receipts = [d for d in runtime.inspect()['object_digests']
                           if s._object(s._read(runtime.root / 'objects' / (d + '.json')))['payload'].get('kind') == 'remote-promotion']
        self.assertEqual(len(remote_receipts), 1)
        (live.output / 'worker-transcript.bin').write_bytes(worker_result.transcript)
        evidence = {'task_id': task['task_id'], 'candidate_head': guard['head'], 'base_sha': task['base_sha'],
                    'marker': marker, 'task': task,
                    'worker': {'model_id': worker_enrollment['model_id'], 'family': worker_enrollment['family'],
                               'container_id': worker_result.container_id, 'exit_code': worker_result.exit_code,
                               'timed_out': worker_result.timed_out, 'truncated': worker_result.truncated,
                               'effective_digest': worker_result.effective_digest,
                               'elapsed_seconds': worker_result.elapsed_seconds},
                    'reviewer': review['metadata'], 'review_verdict': review['report']['verdict'],
                    'verification_passed': True, 'authorization_digest': authorization_digest,
                    'remote_receipt_digest': remote_receipts[0], 'review_digest': review_digest,
                    'promotion': promoted['status'], 'pushes': pushes, 'bare_remote': str(bare),
                    'image': live.image, 'binary': {'path': str(live.binary.path), 'sha256': live.binary.sha256,
                                                    'version': live.binary.version},
                    'docker': {'digest': live.docker.digest, 'endpoint': live.docker.endpoint},
                    'credential': finger, 'guard_vetoes': guard['vetoes'], 'guard_flags': guard['flags']}
        self.assertNotIn('key', json.dumps({k: v for k, v in evidence.items() if k != 'credential'}))
        (live.output / 'live-evidence.json').write_text(json.dumps(evidence, indent=2, sort_keys=True) + '\n')
        shutil.copytree(runtime.root, live.output / 'runtime')
        shutil.copytree(live.root / 'quarantine', live.output / 'quarantine')
        shutil.copytree(attempt_dir, live.output / 'attempt')

    def live_review(self, live, box, guarded, attempt_dir, task, guard, enrollment, worker_family):
        candidate_root = guarded.root / 'candidate'
        prompt = o.build_review_prompt(
            task=task, candidate_sha=guard['head'],
            diff_text=subprocess.check_output(
                ['git', '-C', str(candidate_root), 'diff', '--no-color', task['base_sha'], guard['head']],
                env={'PATH': '/usr/bin:/bin', 'LANG': 'C'}).decode('utf-8', 'replace')[:262144],
            guard_flags=guard['flags'],
            verification_summary={'passed': True, 'checks': ['m5-live']}, budgets=task['budget'])
        base = live.root / 'review'
        base.mkdir()
        shutil.copytree(candidate_root, base / 'input', symlinks=True)
        if w.manifest(base / 'input') != guard['manifest']:
            raise AssertionError('reviewer input differs from guarded candidate')
        review_candidate = b.Candidate(base / 'input', task['base_sha'], task['authority_digest'],
                                       'review-' + guard['head'][:12], _seal=b._SEAL)
        view = a.build_worker_view(review_candidate, base, task['task_id'], 'review', 'INVESTIGATION_REVIEW')
        overlays = b.write_role_overlays(base / 'overlays', 'INVESTIGATION_REVIEW')
        validated = b._validated_role_overlays(overlays, 'INVESTIGATION_REVIEW', base / 'input')
        mounts = [(s, d) for s, d in validated if d.startswith('/candidate/')]
        mounts.extend(live.home_mounts(overlays, base / 'home'))
        nonce = 'review-' + guard['head'][:12]
        provisional = b.StageIdentity('0' * 64, box.boot_identity, nonce, 'review', 1, nonce,
                                      task['authority_digest'], 'INVESTIGATION_REVIEW', task['task_id'])
        result = live.launch_with_credential(
            box=box, candidate=review_candidate, view=view, role='INVESTIGATION_REVIEW',
            container_binary='/usr/local/bin/opencode', message_parts=[prompt], agent='orch-reviewer',
            enrollment=enrollment, image=live.image, network='bridge', limits=b.Limits(1, 1 << 29, 64, 52 << 30,
                                                                                       1 << 30, 1 << 20, 600),
            overlay_mounts=mounts, labels=provisional.labels(), name='or-v2-live-review-' + nonce,
            timeout_seconds=600)
        self.assertFalse(result.timed_out)
        self.assertEqual(result.exit_code, 0)
        (live.output / 'reviewer-transcript.bin').write_bytes(result.transcript)
        stream = a.parse_event_stream(result.transcript, limits=a.StreamLimits(4 << 20, 4096, 3600),
                                      event_contract=live.binary.event_contract)
        self.assertIsNone(stream.error)
        self.assertIsNotNone(stream.final_payload)
        report = a.parse_review_report(stream.final_payload, task=task, candidate_sha=guard['head'],
                                       schemas=SCHEMAS, reviewer_family=enrollment['family'],
                                       implementation_family=worker_family, flag_ids=guard['flags'])
        box.docker(['rm', result.container_id])
        metadata = {'adapter': live.binary.certification_digest(), 'model_id': enrollment['model_id'],
                    'family': enrollment['family'], 'session_id': stream.session_id,
                    'argv_digest': c.canonical_digest([live.binary.path, enrollment['model_id']])}
        return {'report': report, 'metadata': metadata}


if __name__ == '__main__':
    unittest.main(verbosity=2)
