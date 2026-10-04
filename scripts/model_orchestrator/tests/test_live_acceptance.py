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
        run_help = run_binary(binary.path, 'run', '--help')
        self.assertEqual(proc.returncode, 0)
        self.assertEqual(run_help.returncode, 0)
        help_text = (proc.stdout + proc.stderr + run_help.stdout + run_help.stderr).decode()
        for token in ('run', '--model', '--format', '--variant', '--agent'):
            self.assertIn(token, help_text)
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
            import re
            text = proc.stdout.decode()
            match = re.search(r'(?m)^build \(primary\)\s*(\[.*?\])(?=\n[^ \n]|\Z)', text, re.S)
            self.assertIsNotNone(match, 'default build agent missing from installed transport')
            rules = json.loads(match.group(1))
            for tool, expected in (('task', 'deny'), ('read', 'allow'), ('edit', 'allow'), ('bash', 'allow')):
                applicable = [row for row in rules if row['permission'] in ('*', tool) and row['pattern'] == '*']
                self.assertTrue(applicable, 'missing permission for ' + tool)
                self.assertEqual(applicable[-1]['action'], expected, 'installed role permission differs: ' + tool)
            resolved = json.loads(run_binary(binary.path, 'debug', 'config', env=env, cwd=directory).stdout)
            for tool, expected in b.role_policy('IMPLEMENTATION')['permission'].items():
                self.assertEqual(resolved['permission'][tool], expected)


class CodexLiveTests(unittest.TestCase):
    """CP46: minimum real native read-only decision; fault tests never certify a model."""
    def test_no_tool_transport_argv_and_native_model_identity(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            exe = root / 'codex'
            exe.write_text('#!/bin/sh\nprintf "codex-cli 0.158.0\\n"\n')
            exe.chmod(0o700)
            binary = a.CodexBinary(exe, hashlib.sha256(exe.read_bytes()).hexdigest(), a.CODEX_VERSION)
            schema = root / 'decision.schema.json'
            schema.write_text(c.canonical_json(a.architecture_decision_schema()))
            adapter = a.CodexAdapter(binary, model_id='gpt-6-luna', workdir=root, decision_schema_path=schema)
            argv = adapter.build_argv('No tools; emit a decision only.', root / 'last.json')
            for flag in ('--ignore-user-config', '--ignore-rules', '--ephemeral', '--skip-git-repo-check'):
                self.assertIn(flag, argv)
            self.assertEqual(argv[argv.index('--sandbox') + 1], 'read-only')
            self.assertEqual(argv[argv.index('-m') + 1], 'gpt-6-luna')
            self.assertIn('web_search="disabled"', argv)
            disabled = [argv[i + 1] for i, part in enumerate(argv) if part == '--disable']
            self.assertEqual(set(disabled), set(a.CODEX_DISABLED_FEATURES))
            schema.write_text('{"type":"object"}')
            with self.assertRaises(a.AdapterError):
                a.CodexAdapter(binary, model_id='gpt-6-luna', workdir=root, decision_schema_path=schema)

    def test_tool_and_incomplete_streams_refuse(self):
        def stream(item):
            return (c.canonical_json({'type': 'item.completed', 'item': item}) + '\n'
                    + c.canonical_json({'type': 'turn.completed', 'usage': {}}) + '\n').encode()
        for kind in ('command_execution', 'mcp_tool_call', 'web_search', 'file_change', 'collab_tool_call', 'unknown'):
            with self.subTest(kind=kind), self.assertRaises(a.AdapterError):
                a.validate_codex_readonly_stream(stream({'type': kind}))
        a.validate_codex_readonly_stream(stream({'type': 'agent_message', 'text': '{}'}))
        for data in (b'', b'{"type":"turn.started"}\n', b'{"type":"turn.failed"}\n'):
            with self.assertRaises(a.AdapterError):
                a.validate_codex_readonly_stream(data)

    def test_minimum_read_only_invocation(self):
        if os.environ.get('OR_V2_CODEX_LIVE') != '1':
            raise unittest.SkipTest('codex live requires OR_V2_CODEX_LIVE=1')
        for key in ('OR_V2_CODEX_BIN', 'OR_V2_CODEX_SHA', 'OR_V2_CODEX_VERSION', 'OR_V2_CODEX_MODEL'):
            self.assertTrue(os.environ.get(key), 'codex live input missing: ' + key)
        binary = a.CodexBinary(Path(os.environ['OR_V2_CODEX_BIN']), os.environ['OR_V2_CODEX_SHA'],
                               os.environ['OR_V2_CODEX_VERSION'])
        with tempfile.TemporaryDirectory() as directory:
            root = Path(os.environ.get('OR_V2_CODEX_OUTPUT', directory)).resolve()
            root.mkdir(mode=0o700, parents=True, exist_ok=True)
            home = root / 'private-home'
            home.mkdir(mode=0o700)
            schema = root / 'decision.schema.json'
            schema.write_text(c.canonical_json(a.architecture_decision_schema()))
            # Authentication stays in the legitimate host store. No auth bytes
            # are read by the harness, copied to a candidate, or emitted.
            auth_home = Path(os.environ.get('CODEX_HOME', str(Path.home() / '.codex'))).resolve()
            adapter = a.CodexAdapter(binary, model_id=os.environ['OR_V2_CODEX_MODEL'], workdir=home,
                                     decision_schema_path=schema, quota_signatures=[],
                                     env={'PATH': '/usr/bin:/bin', 'LANG': 'C', 'HOME': str(home),
                                          'CODEX_HOME': str(auth_home)})
            packet = {'schema_version': 1, 'task_id': 'cp46-probe', 'checkpoint_id': 'M5',
                      'authority_digest': 'd' * 64, 'base_sha': 'a' * 40, 'kind': 'difficult_root_cause',
                      'failure_facts': {'transport': 'conformance probe'}, 'attempt_digest': 'd' * 64,
                      'invariant_citations': [], 'question': 'Does the read-only transport conform?',
                      'scope': 'transport conformance only'}
            packet['packet_digest'] = c.canonical_digest({k: v for k, v in packet.items() if k != 'packet_digest'})
            prompt = ('Transport conformance only. Do not use tools, read files, edit source, run tests/builds, '
                      'or launch agents. Emit exactly one JSON object: schema_version=1; packet_digest="%s"; '
                      'disposition="DEFER"; decision="Read-only transport probe; no architecture authority"; '
                      'constraints=["read-only","no tools"]; required_verification=[]; '
                      'stop_conditions=["no source or controller writes"]; '
                      'cited_facts=[{"fact":"The supplied packet requests transport conformance only",'
                      '"classification":"PROVEN"}]. No other text.' % packet['packet_digest'])
            outdir = root / 'out'
            outdir.mkdir(mode=0o700)
            identity = {'case_id': 'CP46', 'binary_path': str(binary.path), 'binary_sha256': binary.sha256,
                        'binary_version': binary.version, 'host_sha256': binary.host_sha256, 'model_id': adapter.model_id,
                        'packet_digest': packet['packet_digest'],
                        'argv': adapter.build_argv(prompt, outdir / 'codex-last-message.json')}
            (root / 'identity.json').write_text(c.canonical_json(identity) + '\n')
            before = {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in home.rglob('*') if p.is_file()}
            decision, deferred = adapter.run_architecture(prompt, packet=packet, output_dir=outdir, timeout_seconds=180)
            self.assertIsNone(deferred)
            checked = a.validate_architecture_decision(decision, packet)
            self.assertEqual(checked['packet_digest'], packet['packet_digest'])
            self.assertEqual(checked['disposition'], 'DEFER')
            after = {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in home.rglob('*') if p.is_file()}
            self.assertEqual(after, before, 'read-only model workdir changed')
            (root / 'receipt.json').write_text(c.canonical_json(dict(identity, result='PASS',
                  events_sha256=hashlib.sha256((outdir / 'codex-events.jsonl').read_bytes()).hexdigest(),
                  last_message_sha256=hashlib.sha256((outdir / 'codex-last-message.json').read_bytes()).hexdigest(),
                  no_tools=True, readonly_unchanged=True)) + '\n')


class IndependentReviewTests(unittest.TestCase):
    """CP48: isolated exact release source and complete evidence, never a bundle-only host call."""
    def test_independent_source_review(self):
        cfg = live_config()
        release_sha = os.environ.get('OR_V2_HOSTED_SHA')
        self.assertIsNotNone(release_sha, 'independent source review requires exact final release SHA')
        output = Path(cfg['OR_V2_OUTPUT']).resolve()
        for name in o.SOURCE_REVIEW_EVIDENCE:
            self.assertTrue((output / name).exists(), 'CP48 prerequisite missing: ' + name)
        evidence = c.load_json_strict((output / 'live-evidence.json').read_text())
        self.assertEqual(evidence['review_verdict'], 'PASS')
        self.assertTrue(evidence['verification_passed'])
        self.assertEqual(evidence['promotion'], 'PROMOTED')
        self.assertEqual(c.load_json_strict((output / 'cp46-receipt.json').read_text())['result'], 'PASS')
        self.assertEqual(c.load_json_strict((output / 'cp47-hosted-evidence.json').read_text())['head_sha'], release_sha)
        import uuid
        review_cfg = dict(cfg, OR_V2_OUTPUT=str(output / ('cp48-authority-' + uuid.uuid4().hex[:12])))
        live = LiveM5(review_cfg)
        task = live.task()
        task.update(goal='Independently audit exact released control-plane source and complete execution evidence; no writes.',
                    out_of_scope=['No product adoption', 'No source/controller writes', 'No code execution', 'No nested models'],
                    observable_outcome='A separate readonly source/evidence report bound to the exact release SHA',
                    permission_expectation='Isolated readonly source and evidence; inference credentials only')
        task['resource_limits'].update(output_bytes=8 << 20, wall_seconds=1800)
        records = live.enrollments(task, live.authority, task_class='CONTROL_PLANE_CERTIFICATION_REVIEW')
        task['role_enrollment_ids'] = [c.canonical_digest(record) for record in records]
        authority = live.commit_controller(task)
        enrollments = [a.load_enrollment(record, authority=authority, task=task,
                         expected_digest=c.canonical_digest(record)) for record in records]
        enrollment = o.select_reviewer(task, enrollments, availability={},
                                       implementation_family=evidence['worker']['family'], task_class='CONTROL_PLANE_CERTIFICATION_REVIEW')
        sessions = [evidence['reviewer']['session_id']]
        transcript = a.parse_event_stream((output / 'worker-transcript.bin').read_bytes(),
                                          limits=a.StreamLimits(8 << 20, 4096, 3600),
                                          event_contract=live.binary.event_contract)
        sessions.append(transcript.session_id)
        import uuid
        observations = output / ('cp48-' + release_sha[:12] + '-' + uuid.uuid4().hex[:12])
        outcome = o.run_source_review(authority=authority, task=task, source_repo=ROOT,
                                      release_sha=release_sha, evidence_root=output, enrollment=enrollment,
                                      binary=live.binary, box=live.box(), image=live.image,
                                      container_binary='/usr/local/bin/opencode',
                                      implementation_family=evidence['worker']['family'], storage_root=live.volume,
                                      observation_dir=observations,
                                      limits=b.Limits(1, 1 << 30, 64, 52 << 30, 1 << 30, 8 << 20, 1800),
                                      credential_dir=Path(cfg['OR_V2_CRED_DIR']) if enrollment['provider_id'] == 'opencode-go' else None,
                                      prior_session_ids=sessions, timeout_seconds=1800)
        # A genuine blocking review remains recorded for causal repair.
        (observations / 'review.json').write_text(c.canonical_json(outcome) + '\n')
        (output / 'cp48-independent-review.json').write_text(c.canonical_json(outcome) + '\n')
        self.assertEqual(outcome['report']['candidate_sha'], release_sha)
        self.assertNotIn(outcome['metadata']['session_id'], sessions)
        self.assertEqual(outcome['verdict'], 'PASS', c.canonical_json(outcome['report']))


class HostedHarnessTests(unittest.TestCase):
    """CP47: hosted trusted-harness receipt collection from GitHub provenance."""
    WORKFLOW = '.github/workflows/control-plane-acceptance.yml'

    def collect(self, head_sha, run_id=None):
        from model_orchestrator import hosted
        expected = hosted.expectation(ROOT, head_sha)
        return hosted.collect(expected=expected, run_id=run_id)

    def test_hosted_receipt_collection(self):
        head_sha = os.environ.get('OR_V2_HOSTED_SHA')
        if not head_sha:
            raise unittest.SkipTest('hosted collection requires OR_V2_HOSTED_SHA')
        run_id = os.environ.get('OR_V2_HOSTED_RUN_ID')
        evidence = self.collect(head_sha, run_id=int(run_id) if run_id else None)
        self.assertEqual(evidence['head_sha'], head_sha)
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
        self.root = self.output / 'controller-run'
        self.root.mkdir(mode=0o700)
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
                    observable_outcome='Only the authorized marker is committed, preserved and independently checked before disposable promotion.',
                    expected_result='The real marker check exits zero and reports m5-probe PASS; no other candidate change.',
                    error_cases=['Missing or wrong marker', 'Extra candidate change', 'Uncommitted worker output'],
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

    def box(self):
        return b.ContainerSandbox(self.docker, boot_identity=b.host_boot_identity(), host_platform='linux')

    def enrollments(self, task, authority, *, task_class=None):
        path = os.environ.get('OR_V2_ENROLLMENT_FILE')
        if not path:
            raise ValueError('live acceptance requires externally evaluated qualification observations')
        records = c.load_json_strict(Path(path).read_text())
        enrolled = []
        for role, model in (('IMPLEMENTATION', self.cfg['OR_V2_WORKER_MODEL']),
                            ('INVESTIGATION_REVIEW', self.cfg['OR_V2_REVIEWER_MODEL'])):
            record = next(item for item in records if item['model_id'] == model and role in item['allowed_roles']
                          and (task_class is None or role == 'IMPLEMENTATION' or any(q['task_class'] == task_class for q in item['qualification_evidence'])))
            record = dict(record, operator_adoption_identity=self.build['authorization_id'],
                          adapter_certification_digest=self.binary.certification_digest())
            enrolled.append(record)
        return enrolled


class LiveWorkerTests(unittest.TestCase):
    """CP44: live small task through a real OpenCode worker and independent reviewer."""

    def test_live_small_task_end_to_end(self):
        cfg = live_config()
        live = LiveM5(cfg)
        task = live.task()
        records = live.enrollments(task, live.authority)
        task['role_enrollment_ids'] = [c.canonical_digest(record) for record in records]
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
        enrollments = [a.load_enrollment(record, authority=authority, task=task,
                        expected_digest=c.canonical_digest(record)) for record in records]
        worker_enrollment = o.select_worker(task, enrollments, availability={}, required_reasoning='MEDIUM')
        reviewer_enrollment = o.select_reviewer(task, enrollments, availability={},
                                                 implementation_family=worker_enrollment['family'])
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
            stage = o.claim_stage(runtime, task['task_id'], task_digest, owner_nonce='live-owner',
                                  boot_identity=box.boot_identity, stage_id='live-stage', stage_nonce='live-nonce')
            worker_result = o.run_worker(
                store=runtime, stage=stage, box=box, candidate=candidate, task=task,
                enrollment=worker_enrollment, binary=live.binary, agent=None, prompt=prompt,
                image=live.image, limits=b.Limits(1, 1 << 30, 64, 52 << 30, 1 << 30, 1 << 20, 600),
                network='bridge', container_binary='/usr/local/bin/opencode', storage_root=live.volume,
                timeout_seconds=600,
                credential_dir=Path(cfg['OR_V2_CRED_DIR']) if worker_enrollment['provider_id'] == 'opencode-go' else None,
                observation_dir=live.output)
            (live.output / 'worker-observation.json').write_text(c.canonical_json(worker_result) + '\n')
            stage = dict(stage, container_id=worker_result['container_id'])
            self.assertFalse(worker_result['timed_out'])
            self.assertEqual(worker_result['exit_code'], 0)
            self.assertFalse(worker_result['truncated'])
            self.assertEqual(worker_result['task_status'], 'SETTLED')
        attempt_dir = live.root / 'attempt'
        floor, guarded, readiness = o.import_and_verify(
            store=runtime, task_id=task['task_id'], task_path='controller/task-m5.json',
            catalog_path='controller/catalog-m5.json', guard_root=live.root / 'quarantine', attempt_dir=attempt_dir)
        guard = guarded.verify()
        self.assertFalse(guard['vetoes'])
        self.assertTrue(readiness['verification_passed'])
        self.assertEqual(sorted(readiness['unresolved']), sorted(guard['flags']))
        self.assertEqual(readiness['pending_hosted_classes'], [])
        with runtime.lock('task', task['task_id']):
            review = o.run_reviewer(authority=authority, floor=floor, guarded=guarded,
                                     attempt_dir=attempt_dir, enrollment=reviewer_enrollment, binary=live.binary,
                                     agent=None, box=box, image=live.image, container_binary='/usr/local/bin/opencode',
                                     implementation_family=worker_enrollment['family'],
                                     limits=b.Limits(1, 1 << 30, 64, 52 << 30, 1 << 30, 1 << 20, 600),
                                     credential_dir=Path(cfg['OR_V2_CRED_DIR']) if reviewer_enrollment['provider_id'] == 'opencode-go' else None,
                                     storage_root=live.volume, observation_dir=live.output, store=runtime)
        review_digest = o.persist_review(runtime, task['task_id'], review['report'], review['metadata'])
        inputs = p.collect_inputs(store=runtime, task_id=task['task_id'],
                                  guard_root=live.root / 'quarantine', attempt_dir=attempt_dir,
                                  task_path='controller/task-m5.json', catalog_path='controller/catalog-m5.json',
                                  implementation_family=worker_enrollment['family'])
        authorization = p.build_authorization(inputs, schemas=SCHEMAS,
                                              implementation_family=worker_enrollment['family'], issuance_sequence=1)
        authorization_digest = p.persist_authorization(runtime, task['task_id'], authorization)
        bare = Path(cfg['OR_V2_BARE']).resolve()
        self.assertFalse(bare.exists(), 'live bare target must be fresh')
        subprocess.run(['git', 'init', '--bare', '-q', str(bare)], check=True)
        integration = live.root / 'integration'
        subprocess.run(['git', 'clone', '-q', str(Path(authority.candidate_root)), str(integration)], check=True)
        subprocess.run(['git', '-C', str(integration), 'checkout', '-q', '-b', 'main', task['base_sha']], check=True)
        subprocess.run(['git', '-C', str(integration), 'remote', 'set-url', 'origin', str(bare)], check=True)
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

        def crash_after_push(point):
            if point == 'after_push':
                raise RuntimeError('controlled controller crash after real push')
        with self.assertRaisesRegex(RuntimeError, 'controlled controller crash'):
            p.promote(store=runtime, task_id=task['task_id'], authorization_digest=authorization_digest,
                      integration_repo=integration, remote='origin', expected_remote_url=str(bare),
                      hooks_dir=hooks, push_runner=counting_push, fault=crash_after_push)
        latest, _ = p._latest_intent(runtime, task['task_id'])
        restarted = s.RuntimeStore(runtime.root, authority)
        promoted = p.reconcile_push(store=restarted, task_id=task['task_id'], intent_digest=latest,
                                    integration_repo=integration, remote='origin', push_observed=False)
        latest, _ = p._latest_intent(restarted, task['task_id'])
        replayed = p.reconcile_push(store=restarted, task_id=task['task_id'], intent_digest=latest,
                                   integration_repo=integration, remote='origin', push_observed=False)
        self.assertEqual(replayed['status'], 'PROMOTED')
        (live.output / 'recovery.json').write_text(c.canonical_json(dict(
            crash_point='after_push', actual_push_count=len(pushes), initial=promoted, replay=replayed,
            local_head=p._git_ok(integration, 'rev-parse', 'HEAD'),
            remote_head=p.advertised_main(integration, 'origin'), candidate_sha=guard['head'])) + '\n')
        self.assertEqual(promoted['status'], 'PROMOTED')
        self.assertEqual(len(pushes), 1)
        self.assertNotIn('--force', pushes[0])
        pushes = len(pushes)
        remote_receipts = [d for d in runtime.inspect()['object_digests']
                           if s._object(s._read(runtime.root / 'objects' / (d + '.json')))['payload'].get('kind') == 'remote-promotion']
        self.assertEqual(len(remote_receipts), 1)
        import agent_supervisor
        handoff = agent_supervisor.validate_disabled_task_handoff(
            integration, store=runtime, task_id=task['task_id'], authorization_digest=authorization_digest,
            remote_receipt_digest=remote_receipts[0])
        self.assertEqual(handoff['status'], 'DISABLED_CONTROL_PLANE_READY')
        self.assertEqual(handoff['product_next'], '9B')
        (live.output / 'handoff.json').write_text(c.canonical_json(handoff) + '\n')
        (live.output / 'state-readiness.json').write_text(c.canonical_json(dict(
            status='DISABLED_CONTROL_PLANE_READY', task_id=task['task_id'], candidate_sha=guard['head'],
            product_next=handoff['product_next'], verified_contract_versions=handoff['verified_contract_versions'],
            plan_unchanged=True, state_unchanged=True,
            authority_semantics='NO_PRODUCT_COMPLETION_NO_ADOPTION')) + '\n')
        pins = live.output / 'controller-pins'
        shutil.copytree(Path(authority.controller_root) / 'controller', pins)
        (pins / 'authority.json').write_text(authority.payload_json + '\n')
        (pins / 'source.json').write_text(c.canonical_json(dict(source_sha=authority.source_sha,
            candidate_root=authority.candidate_root, controller_root=authority.controller_root)) + '\n')
        evidence = {'task_id': task['task_id'], 'candidate_head': guard['head'], 'base_sha': task['base_sha'],
                    'marker': marker, 'task': task,
                    'worker': {'model_id': worker_enrollment['model_id'], 'family': worker_enrollment['family'],
                               'container_id': worker_result['container_id'], 'exit_code': worker_result['exit_code'],
                               'timed_out': worker_result['timed_out'], 'truncated': worker_result['truncated'],
                               'effective_digest': worker_result['effective_digest'],
                               'elapsed_seconds': worker_result['elapsed_seconds'], 'metadata': worker_result['metadata']},
                    'reviewer': review['metadata'], 'review_verdict': review['report']['verdict'],
                    'verification_passed': True, 'authorization_digest': authorization_digest,
                    'remote_receipt_digest': remote_receipts[0], 'review_digest': review_digest,
                    'promotion': promoted['status'], 'pushes': pushes, 'bare_remote': str(bare),
                    'image': live.image, 'binary': {'path': str(live.binary.path), 'sha256': live.binary.sha256,
                                                    'version': live.binary.version},
                    'docker': {'digest': live.docker.digest, 'endpoint': live.docker.endpoint},
                    'credential': credential_fingerprint(cfg['OR_V2_CRED_DIR']), 'guard_vetoes': guard['vetoes'], 'guard_flags': guard['flags']}
        self.assertNotIn('key', json.dumps({k: v for k, v in evidence.items() if k != 'credential'}))
        (live.output / 'live-evidence.json').write_text(json.dumps(evidence, indent=2, sort_keys=True) + '\n')
        shutil.copytree(runtime.root, live.output / 'runtime')
        shutil.copytree(live.root / 'quarantine', live.output / 'quarantine')
        shutil.copytree(attempt_dir, live.output / 'attempt')



if __name__ == '__main__':
    unittest.main(verbosity=2)
