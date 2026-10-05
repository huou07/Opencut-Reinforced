"""CP44: opt-in post-adoption REAL product fixture, all CLI stages fresh processes.

Requires real externally adopted control/controller/bootstrap roots; unit
certification fixtures cannot satisfy this live prerequisite. The product
checkpoint is OR-V2-FIXTURE on a disposable descendant of actual main. Its
local bare target prevents publication or implementation of actual 9B.
"""
import copy
from dataclasses import asdict
import json
import os
from pathlib import Path
import subprocess
import sys
import unittest

from model_orchestrator import contracts as c, adapters as a, store as s, promotion as p, sandbox as b
from model_orchestrator.__main__ import bootstrap_from_dict
from model_orchestrator.tests.test_live_acceptance import live_config, LiveM5
from model_orchestrator.tests.test_product_operational import ProductFixture, clone, git, REPO_ROOT


@unittest.skipUnless(os.environ.get('OR_V2_PRODUCT_LIVE') == '1', 'post-adoption live product inputs required')
class OperationalProductLiveTests(unittest.TestCase):
    """CP44: adopted authority → real product task → worker/verifier/review/promotion/handoff."""
    def test_post_adoption_product_cli_journey(self):
        cfg = live_config()
        required = ('OR_V2_PRODUCT_CONTROL', 'OR_V2_PRODUCT_CONTROLLER', 'OR_V2_PRODUCT_BOOTSTRAP')
        self.assertTrue(all(os.environ.get(name) for name in required), 'exact real adopted roots/bootstrap required')
        bootstrap = bootstrap_from_dict(c.load_json_strict(Path(os.environ['OR_V2_PRODUCT_BOOTSTRAP']).read_text()))
        release = c.load_release_authority(Path(os.environ['OR_V2_PRODUCT_CONTROL']),
            Path(os.environ['OR_V2_PRODUCT_CONTROLLER']), bootstrap=bootstrap)
        from model_orchestrator.product import release_active
        facts = release_active(release)
        self.assertEqual(facts['certification']['release_sha'], os.environ['OR_V2_HOSTED_SHA'])
        # The real release must contain source identical to this executing host.
        for path in ('scripts/model_orchestrator/product.py', 'scripts/model_orchestrator/__main__.py',
                     'scripts/agent_supervisor.py', c.PROTOCOL_SCHEMAS_PATH):
            self.assertEqual(c._git(Path(release.candidate_root), 'show', bootstrap.release_sha + ':' + path),
                             (REPO_ROOT / path).read_bytes())
        output = Path(cfg['OR_V2_OUTPUT']).resolve()
        output.mkdir(parents=True, exist_ok=True)
        records = c.load_json_strict(Path(os.environ['OR_V2_ENROLLMENT_FILE']).read_text())
        binary = a.OpenCodeBinary(Path(cfg['OR_V2_OPENCODE_BIN']), cfg['OR_V2_OPENCODE_SHA'], cfg['OR_V2_OPENCODE_VERSION'])
        chosen = []
        for role, model in (('IMPLEMENTATION', cfg['OR_V2_WORKER_MODEL']), ('INVESTIGATION_REVIEW', cfg['OR_V2_REVIEWER_MODEL'])):
            record = next(item for item in records if item['model_id'] == model and role in item['allowed_roles'])
            chosen.append(dict(record, operator_adoption_identity=facts['build']['authorization_id'],
                               adapter_certification_digest=binary.certification_digest()))
        fixture = ProductFixture(output / 'product-run', candidate=release.candidate_root, controller=release.controller_root,
            bootstrap=bootstrap, toy=True, image=cfg['OR_V2_IMAGE'], docker=cfg['OR_V2_DOCKER'],
            docker_digest=cfg['OR_V2_DOCKER_SHA'], endpoint=cfg['OR_V2_ENDPOINT'], enrollments=chosen)
        task = fixture.task
        runtime_path = fixture.root / 'runtime'
        host_bootstrap = fixture.root / 'host-bootstrap.json'
        host_bootstrap.write_text(c.canonical_json(dict(asdict(fixture.bootstrap), product_task_pin=asdict(fixture.pin))) + '\n')
        base = [sys.executable, '-m', 'model_orchestrator', '--bootstrap', str(host_bootstrap),
            '--candidate-root', str(fixture.candidate), '--controller-root', str(fixture.controller),
            '--product-root', str(fixture.product), '--runtime', str(runtime_path)]
        env = dict(os.environ, PYTHONPATH=str(REPO_ROOT / 'scripts'), PYTHONDONTWRITEBYTECODE='1')
        stages = []
        def call(name, *args):
            proc = subprocess.run([*base, name, *map(str, args)], env=env, capture_output=True, timeout=1800)
            (output / (name + '.stdout')).write_bytes(proc.stdout)
            (output / (name + '.stderr')).write_bytes(proc.stderr)
            self.assertEqual(proc.returncode, 0, proc.stdout.decode(errors='replace') + proc.stderr.decode(errors='replace'))
            result = c.load_json_strict(proc.stdout.decode())
            self.assertEqual(result['status'], 'OK', result)
            stages.append(dict(stage=name, result=result))
            return result
        call('admit', '--task', fixture.controller / fixture.task_path, '--template', fixture.controller / fixture.task_path)
        destination = Path(cfg['OR_V2_VOLUME']) / ('product-' + fixture.bootstrap.release_sha[:10] + '-' + str(os.getpid()))
        call('candidate', '--task-id', task['task_id'], '--destination', destination)
        enrollments = [a.load_enrollment(record, authority=fixture.authority, task=task,
                       expected_digest=c.canonical_digest(record)) for record in chosen]
        worker, reviewer = enrollments
        self.assertNotEqual(worker['family'], reviewer['family'])
        for role, record in zip(('worker', 'reviewer'), chosen):
            (fixture.root / (role + '-enrollment.json')).write_text(c.canonical_json(record) + '\n')
        prompt = fixture.root / 'worker-prompt.txt'
        prompt.write_text('This is an isolated product-checkpoint fixture, not 9B. Create only operational-fixture.txt '
            'containing exactly `# OPERATIONAL_PRODUCT_FIXTURE` followed by a newline. Do not change any other file. '
            'Commit only that file using git -c user.name="Fixture Worker" -c user.email="worker@example.invalid". '
            'Do not run product work or models. The controller independently verifies the exact bytes.')
        transport = ['--opencode-bin', cfg['OR_V2_OPENCODE_BIN'], '--opencode-sha', cfg['OR_V2_OPENCODE_SHA'],
            '--opencode-version', cfg['OR_V2_OPENCODE_VERSION'], '--docker-bin', cfg['OR_V2_DOCKER'],
            '--docker-sha', cfg['OR_V2_DOCKER_SHA'], '--endpoint', cfg['OR_V2_ENDPOINT'],
            '--image', cfg['OR_V2_IMAGE'], '--limits', '1,1073741824,64,55834574848,1073741824,1048576,600',
            '--storage-root', cfg['OR_V2_VOLUME'], '--timeout', '600']
        credentials = ['--credential-dir', cfg['OR_V2_CRED_DIR']]
        call('claim', '--task-id', task['task_id'], '--owner', 'product-live-owner', '--boot', b.host_boot_identity(),
            '--stage-id', 'product-worker', '--stage-nonce', 'product-stage-nonce', '--launch',
            '--enrollment', fixture.root / 'worker-enrollment.json', '--prompt', prompt, '--network', 'bridge',
            *transport, *(credentials if worker['provider_id'] == 'opencode-go' else []))
        guard_root, attempt = fixture.root / 'quarantine', fixture.root / 'attempt'
        floor_args = ['--task-id', task['task_id'], '--task-path', fixture.task_path, '--catalog-path', fixture.catalog_path,
                      '--guard-root', guard_root, '--attempt-dir', attempt]
        verification = call('verify', *floor_args)
        self.assertTrue(verification['readiness']['verification_passed'])
        review = call('review', *floor_args, '--enrollment', fixture.root / 'reviewer-enrollment.json',
            '--implementation-family', worker['family'], *transport,
            *(credentials if reviewer['provider_id'] == 'opencode-go' else []))
        self.assertEqual(review['verdict'], 'PASS')
        authorization = call('authorize', *floor_args, '--implementation-family', worker['family'], '--sequence', '1')
        integration = fixture.root / 'integration'
        clone(fixture.product, integration, fixture.base)
        git(integration, 'checkout', '-q', '-B', 'main', fixture.base)
        git(integration, 'remote', 'set-url', 'origin', str(fixture.bare))
        runtime = s.RuntimeStore(runtime_path, fixture.authority)
        authorized = p.load_authorization(runtime, authorization['authorization_digest'])
        git(integration, 'fetch', '-q', str(guard_root / 'candidate'), authorized['candidate_sha'] + ':refs/heads/product-candidate')
        promoted = call('promote', '--task-id', task['task_id'], '--authorization-digest', authorization['authorization_digest'],
            '--integration-repo', integration, '--hooks-dir', fixture.root / 'hooks')
        self.assertEqual(promoted['promotion']['status'], 'PROMOTED')
        remotes = [digest for digest in runtime.inspect()['object_digests']
                   if p.load_object(runtime, digest)['payload'].get('kind') == 'remote-promotion']
        self.assertEqual(len(remotes), 1)
        handoff = call('handoff', '--task-id', task['task_id'], '--checkpoint', task['checkpoint_id'],
            '--integration-repo', integration, '--authorization-digest', authorization['authorization_digest'],
            '--remote-receipt-digest', remotes[0])
        self.assertEqual(handoff['handoff']['implementation_sha'], authorized['candidate_sha'])
        self.assertEqual(git(fixture.product, 'rev-parse', 'HEAD'), fixture.base)
        self.assertEqual(json.loads((integration / 'docs/execution/STATE.json').read_text())['current_next'], 'OR-V2-FIXTURE')
        evidence = dict(release_sha=bootstrap.release_sha, adopted_controller_sha=bootstrap.source_sha,
            fixture_controller_sha=fixture.source, product_task_pin=asdict(fixture.pin), product_base_sha=fixture.base,
            product_candidate_sha=authorized['candidate_sha'], publication=str(fixture.bare), stages=stages,
            result='PASS', authority_semantics='REAL_POST_ADOPTION_PRODUCT_PATH_ISOLATED_FIXTURE_NOT_9B_EVIDENCE')
        (output / 'product-operational-evidence.json').write_text(c.canonical_json(evidence) + '\n')


if __name__ == '__main__':
    unittest.main(verbosity=2)
