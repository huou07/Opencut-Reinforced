"""Exact hosted supporting evidence. Never certification, promotion or adoption.

The producer measures actual tests. A separate job without a source checkout
seals its receipt. The controller compares both downloaded artifacts with
GitHub provenance and independently supplied exact source/case expectations.
"""
from __future__ import annotations

import argparse
import base64
import hashlib
import io
import json
import math
import os
from pathlib import Path
import re
import subprocess
import sys
import time
import unittest
import zipfile

from . import contracts as c

WORKFLOW = '.github/workflows/control-plane-acceptance.yml'
ROOT = Path(__file__).resolve().parents[2]
SUITES = {
    name: 'model_orchestrator.tests.' + name for name in
    ('test_contracts', 'test_isolation_and_state', 'test_verification',
     'test_lifecycle', 'test_promotion_and_handoff', 'test_live_acceptance', 'test_hosted', 'test_attempt_execution', 'test_source_review', 'test_cross_phase_attacks', 'test_product_operational', 'test_live_product_operational', 'test_roadmap', 'test_live_roadmap')}
SUITES.update(test_execution_infra='scripts.test_execution_infra',
              test_document_routing='scripts.test_document_routing')
CHECKS = {
    'execution-plan': ['python3', 'scripts/check_execution_plan.py'],
    'architecture-policy': ['python3', 'scripts/check_architecture_policy.py'],
    'document-routing': ['python3', 'scripts/check_document_routing.py'],
    'compile': ['python3', '-m', 'compileall', '-q', 'scripts'],
    'diff': ['git', 'diff', '--check'],
    'repo-hygiene': ['bash', 'scripts/check-repo.sh'],
}
MAX_ARTIFACT = 8 << 20


def require(condition, message):
    if not condition:
        raise c.ContractError('hosted evidence: ' + message)


def digest(data):
    return hashlib.sha256(data).hexdigest()


def seal(record, field):
    return dict(record, **{field: c.canonical_digest(record)})


def unseal(record, field):
    require(type(record) is dict and field in record, 'missing ' + field)
    body = {k: v for k, v in record.items() if k != field}
    require(record[field] == c.canonical_digest(body), field + ' mismatch')
    return body


def cases(suite):
    for test in suite:
        if isinstance(test, unittest.TestSuite):
            yield from cases(test)
        else:
            yield test


def inventory():
    sys.path.insert(0, str(ROOT))
    expected, skips = {}, []
    for name, module in SUITES.items():
        tests = list(cases(unittest.defaultTestLoader.loadTestsFromName(module)))
        expected[name] = sorted(test.id() for test in tests)
        skips.extend(test.id() for test in tests if getattr(test.__class__, '__unittest_skip__', False)
                     or getattr(getattr(test, test._testMethodName), '__unittest_skip__', False))
    # This hosted-fixture suite deliberately cannot collect its own future run.
    skips.extend('model_orchestrator.tests.test_live_acceptance.' + name for name in (
        'HostedHarnessTests.test_hosted_receipt_collection',
        'OpenCodeConformanceTests.test_binary_identity_and_help_surface',
        'OpenCodeConformanceTests.test_config_precedence_project_wins',
        'OpenCodeConformanceTests.test_error_event_shape_and_terminal_behavior',
        'OpenCodeConformanceTests.test_agent_surface_lists_no_tools_dispatcher',
        'CodexLiveTests.test_minimum_read_only_invocation',
        'IndependentReviewTests.test_independent_source_review',
        'LiveWorkerTests.test_live_small_task_end_to_end'))
    expected['control-checks'] = sorted(CHECKS)
    return expected, sorted(set(skips))


def expectation(root, head_sha):
    """Caller must independently trust this exact checked-out candidate first."""
    root = Path(root)
    require(re.fullmatch('[0-9a-f]{40}', head_sha) is not None, 'invalid candidate SHA')
    git = lambda *args: subprocess.check_output(['git', *args], cwd=root)
    require(git('rev-parse', 'HEAD').decode().strip() == head_sha, 'checkout differs from candidate')
    require(not git('diff', head_sha, '--'), 'checkout source differs from exact candidate')
    require(not git('ls-files', '--others', '--exclude-standard'), 'untracked source could shadow exact candidate')
    tree = git('rev-parse', head_sha + '^{tree}').decode().strip()
    source = git('ls-tree', '-r', '--full-tree', head_sha).decode()
    manifest = {line.split('\t', 1)[1]: line.split()[2] for line in source.splitlines()}
    package = dict(repository=c.REPOSITORY_IDENTITY, head_sha=head_sha, tree_sha=tree,
                   source_manifest_digest=c.canonical_digest(manifest))
    workflow = git('show', head_sha + ':' + WORKFLOW)
    case_ids, allowed_skips = inventory()
    return dict(package=package, workflow_digest=digest(workflow), case_ids=case_ids, allowed_skips=allowed_skips)


class MeasuredResult(unittest.TextTestResult):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self.rows = {}
        self.started = {}

    def startTest(self, test):
        self.started[test.id()] = time.monotonic()
        self.rows[test.id()] = dict(id=test.id(), result='RUNNING', duration_seconds=0)
        super().startTest(test)

    def stopTest(self, test):
        self.rows[test.id()]['duration_seconds'] = time.monotonic() - self.started[test.id()]
        super().stopTest(test)

    def addSuccess(self, test):
        self.rows[test.id()]['result'] = 'PASS'
        super().addSuccess(test)

    def addSkip(self, test, reason):
        self.rows[test.id()] = dict(id=test.id(), result='SKIP', reason=reason, duration_seconds=0)
        super().addSkip(test, reason)

    def addFailure(self, test, err):
        self.rows.setdefault(test.id(), dict(id=test.id(), duration_seconds=0))['result'] = 'FAIL'
        super().addFailure(test, err)

    def addError(self, test, err):
        self.rows.setdefault(test.id(), dict(id=test.id(), duration_seconds=0))['result'] = 'ERROR'
        super().addError(test, err)

    def addSubTest(self, test, subtest, err):
        if err is not None:
            self.rows[test.id()]['result'] = 'FAIL'
        super().addSubTest(test, subtest, err)

    def addExpectedFailure(self, test, err):
        self.rows[test.id()]['result'] = 'XFAIL'
        super().addExpectedFailure(test, err)

    def addUnexpectedSuccess(self, test):
        self.rows[test.id()]['result'] = 'XPASS'
        super().addUnexpectedSuccess(test)


def measure_suite(name, destination):
    sys.path.insert(0, str(ROOT))
    suite = unittest.defaultTestLoader.loadTestsFromName(SUITES[name])
    log = io.StringIO()
    started = time.monotonic()
    result = unittest.TextTestRunner(stream=log, verbosity=2, resultclass=MeasuredResult).run(suite)
    row = dict(id=name, cases=sorted(result.rows.values(), key=lambda r: r['id']),
               result='PASS' if result.wasSuccessful() else 'FAIL',
               duration_seconds=time.monotonic() - started, output_digest=digest(log.getvalue().encode()))
    Path(destination).write_text(json.dumps(row, sort_keys=True) + '\n')
    print(log.getvalue())
    return 0 if result.wasSuccessful() else 1


def produce(destination):
    head = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT).decode().strip()
    expected = expectation(ROOT, head)
    rows = []
    destination = Path(destination)
    destination.mkdir(parents=True, exist_ok=False)
    for name in SUITES:
        path = destination / (name + '.json')
        proc = subprocess.run([sys.executable, '-m', 'model_orchestrator.hosted', 'suite', name, str(path)], cwd=ROOT)
        require(path.is_file(), 'test process produced no observations: ' + name)
        row = c.load_json_strict(path.read_text())
        require(proc.returncode == 0 and row['result'] == 'PASS', 'suite failed: ' + name)
        rows.append(row)
    checks = []
    started = time.monotonic()
    for name, argv in CHECKS.items():
        before = time.monotonic()
        proc = subprocess.run(argv, cwd=ROOT, capture_output=True)
        checks.append(dict(id=name, result='PASS' if proc.returncode == 0 else 'FAIL',
                           duration_seconds=time.monotonic() - before, exit_code=proc.returncode,
                           command_digest=c.canonical_digest(argv), output_digest=digest(proc.stdout + proc.stderr)))
        require(proc.returncode == 0, 'control check failed: ' + name)
    rows.append(dict(id='control-checks', result='PASS', cases=checks,
                     duration_seconds=time.monotonic() - started, output_digest=c.canonical_digest(checks)))
    require(expectation(ROOT, head) == expected, 'package/source changed during tests')
    receipt = seal(dict(schema_version=2, head_sha=head, workflow=WORKFLOW,
                        workflow_digest=expected['workflow_digest'], package=expected['package'],
                        package_digest=c.canonical_digest(expected['package']), boundary='hosted-fixture',
                        run_id=int(os.environ['GITHUB_RUN_ID']), run_attempt=int(os.environ['GITHUB_RUN_ATTEMPT']),
                        suites=rows, authority_semantics='SUPPORTING_FACTS_ONLY_NO_ADOPTION'), 'receipt_digest')
    (destination / 'acceptance-receipt.json').write_text(json.dumps(receipt, sort_keys=True, indent=2) + '\n')


def validate_receipts(receipt, collector, *, expected, run_id, run_attempt):
    body = unseal(receipt, 'receipt_digest')
    require(set(body) == {'schema_version', 'head_sha', 'workflow', 'workflow_digest', 'package',
            'package_digest', 'boundary', 'run_id', 'run_attempt', 'suites', 'authority_semantics'}, 'receipt fields differ')
    sealed_collector = unseal(collector, 'collector_digest')
    require(set(sealed_collector) == {'schema_version', 'head_sha', 'run_id', 'run_attempt',
            'workflow_digest', 'package_digest', 'source_receipt_digest', 'suite_digest', 'isolation'}, 'collector fields differ')
    require(body['schema_version'] == 2 and body['boundary'] == 'hosted-fixture', 'wrong receipt boundary/schema')
    require(body['head_sha'] == expected['package']['head_sha'] and body['package'] == expected['package'], 'wrong candidate/source/package')
    require(body['package_digest'] == c.canonical_digest(expected['package']), 'package identity mismatch')
    require(body['workflow'] == WORKFLOW and body['workflow_digest'] == expected['workflow_digest'], 'workflow definition mismatch')
    require(body['run_id'] == run_id and body['run_attempt'] == run_attempt, 'receipt run provenance mismatch')
    require(body['authority_semantics'] == 'SUPPORTING_FACTS_ONLY_NO_ADOPTION', 'candidate receipt claims authority')
    suites = body['suites']
    require(len(suites) == len(expected['case_ids']) and {r['id'] for r in suites} == set(expected['case_ids']), 'suite set differs')
    for suite in suites:
        require(suite['result'] == 'PASS', 'suite did not pass')
        rows = suite['cases']
        require(len(rows) == len(expected['case_ids'][suite['id']]) and
                sorted(r['id'] for r in rows) == expected['case_ids'][suite['id']], 'exact case inventory differs')
        for row in [suite, *rows]:
            value = row['duration_seconds']
            require(type(value) in (float, int) and math.isfinite(value) and value >= 0, 'missing/invalid measurement')
        require(re.fullmatch('[0-9a-f]{64}', suite['output_digest']) is not None, 'missing observed output digest')
        for row in rows:
            require(row['result'] == 'PASS' or row['result'] == 'SKIP' and row['id'] in expected['allowed_skips'], 'required case failed/skipped')
            if suite['id'] == 'control-checks':
                require(row['exit_code'] == 0 and row['command_digest'] == c.canonical_digest(CHECKS[row['id']]), 'control command/result differs')
    require(collector['schema_version'] == 2 and collector['head_sha'] == body['head_sha'] and
            collector['run_id'] == run_id and collector['run_attempt'] == run_attempt and
            collector['source_receipt_digest'] == receipt['receipt_digest'] and
            collector['workflow_digest'] == expected['workflow_digest'] and
            collector['package_digest'] == body['package_digest'] and
            collector['suite_digest'] == c.canonical_digest(suites) and
            collector['isolation'] == 'SEPARATE_HOSTED_JOB_NO_CHECKOUT_NO_PACKAGE_EXECUTION', 'collector binding/isolation mismatch')
    return body


def github(path):
    proc = subprocess.run(['gh', 'api', path], capture_output=True, timeout=90)
    require(proc.returncode == 0, 'GitHub API unavailable: ' + path)
    require(len(proc.stdout) <= MAX_ARTIFACT, 'GitHub response exceeds limit')
    return proc.stdout


def collect(*, expected, run_id=None, request=github):
    repo = '/repos/' + c.REPOSITORY_IDENTITY
    get = lambda path: c.load_json_strict(request(repo + path).decode())
    head = expected['package']['head_sha']
    if run_id is None:
        runs = get('/actions/workflows/control-plane-acceptance.yml/runs?head_sha=' + head + '&per_page=100')['workflow_runs']
        runs = [r for r in runs if r['head_sha'] == head and r['status'] == 'completed' and r['conclusion'] == 'success']
        require(bool(runs), 'no successful exact-candidate hosted run')
        run_id = max(runs, key=lambda r: (r['run_number'], r['run_attempt']))['id']
    run = get('/actions/runs/' + str(int(run_id)))
    require(run['head_sha'] == head and run['status'] == 'completed' and run['conclusion'] == 'success' and
            run['event'] in ('push', 'workflow_dispatch') and run['path'] == WORKFLOW and
            run['repository']['full_name'] == c.REPOSITORY_IDENTITY and
            run['head_repository']['full_name'] == c.REPOSITORY_IDENTITY, 'workflow run provenance differs')
    workflow = get('/contents/' + WORKFLOW + '?ref=' + head)
    workflow_bytes = base64.b64decode(workflow['content'], validate=False)
    require(digest(workflow_bytes) == expected['workflow_digest'], 'GitHub workflow blob differs from trusted definition')
    # This workflow has no reusable calls. Remote actions are pinned in its exact trusted definition.
    require(not run.get('referenced_workflows'), 'unexpected reusable workflow call')
    commit = get('/git/commits/' + head)
    require(commit['sha'] == head and commit['tree']['sha'] == expected['package']['tree_sha'], 'GitHub source tree differs')
    jobs = get('/actions/runs/%d/attempts/%d/jobs?per_page=100' % (run['id'], run['run_attempt']))['jobs']
    require(len(jobs) == 3 and {j['name'] for j in jobs} == {'Fixture acceptance', 'Collect receipts', 'Isolated roadmap hosted marker acceptance'}, 'missing/duplicate hosted job')
    marker_job = next(j for j in jobs if j['name'] == 'Isolated roadmap hosted marker acceptance')
    require(marker_job['head_sha'] == head and marker_job['status'] == 'completed' and marker_job['conclusion'] == 'skipped', 'certification run executed unexpected fixture marker job')
    for job in (j for j in jobs if j is not marker_job):
        require(job['head_sha'] == head and job['conclusion'] == 'success' and job['status'] == 'completed', 'unsuccessful/stale job')
        required = ('Measure acceptance cases', 'Upload acceptance receipt') if job['name'] == 'Fixture acceptance' else ('Download acceptance receipt', 'Validate and seal collector receipt', 'Upload collector receipt')
        for name in required:
            steps = [step for step in job['steps'] if step['name'] == name]
            require(len(steps) == 1 and steps[0]['conclusion'] == 'success', 'required step missing/skipped: ' + name)
    artifacts = get('/actions/runs/%d/artifacts?per_page=100' % run['id'])
    require(artifacts['total_count'] == 2 and len(artifacts['artifacts']) == 2, 'duplicate/missing/extra artifact')
    documents, identities = {}, []
    for artifact in artifacts['artifacts']:
        kind = next((kind for kind in ('acceptance', 'collector') if artifact['name'] == kind + '-receipt-' + head), None)
        require(kind is not None and kind not in documents and not artifact['expired'] and
                type(artifact.get('size_in_bytes')) is int and 0 < artifact['size_in_bytes'] <= MAX_ARTIFACT and
                artifact['workflow_run']['id'] == run['id'] and artifact['workflow_run']['head_sha'] == head, 'artifact provenance/uniqueness differs')
        data = request(repo + '/actions/artifacts/%d/zip' % artifact['id'])
        require(len(data) <= MAX_ARTIFACT and artifact['digest'] == 'sha256:' + digest(data), 'artifact archive digest differs')
        with zipfile.ZipFile(io.BytesIO(data)) as archive:
            require(archive.namelist() == [kind + '-receipt.json'], 'unexpected artifact contents')
            entry = archive.infolist()[0]
            require(entry.file_size <= MAX_ARTIFACT and not (entry.external_attr >> 16 & 0o170000) == 0o120000, 'unsafe artifact entry')
            document = archive.read(entry)
        documents[kind] = c.load_json_strict(document.decode())
        identities.append(dict(id=artifact['id'], name=artifact['name'], archive_digest=digest(data), content_digest=digest(document)))
    validate_receipts(documents['acceptance'], documents['collector'], expected=expected,
                      run_id=run['id'], run_attempt=run['run_attempt'])
    return dict(schema_version=2, head_sha=head, run_id=run['id'], run_attempt=run['run_attempt'],
                html_url=run['html_url'], workflow_digest=expected['workflow_digest'],
                referenced_workflows=[], reusable_workflows='NONE_IN_EXACT_DEFINITION',
                jobs={j['name']: dict(id=j['id'], head_sha=j['head_sha'], conclusion=j['conclusion']) for j in jobs},
                artifacts=identities, acceptance_receipt=documents['acceptance'], collector_receipt=documents['collector'],
                authority_semantics='SUPPORTING_FACTS_ONLY_NO_ADOPTION')


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('mode', choices=('suite', 'produce'))
    parser.add_argument('args', nargs='+')
    args = parser.parse_args()
    if args.mode == 'suite':
        return measure_suite(*args.args)
    produce(*args.args)
    return 0




def collect_roadmap_fixture(*, run_id, release_sha, candidate_sha, task_digest, marker_path, marker, request=None):
    """Actual bounded hosted observation; cannot grant production acceptance."""
    request = request or github
    prefix='/repos/'+c.REPOSITORY_IDENTITY
    get=lambda path:c.load_json_strict(request(prefix+path).decode())
    run=get('/actions/runs/'+str(run_id))
    require(run['repository']['full_name']==c.REPOSITORY_IDENTITY and run['head_sha']==release_sha
            and run['event']=='workflow_dispatch' and run['conclusion']=='success'
            and run['path'].split('@')[0]==WORKFLOW,'wrong/stale fixture hosted run')
    jobs=get('/actions/runs/'+str(run_id)+'/jobs?per_page=100')['jobs']
    matching=[job for job in jobs if job['name']=='Isolated roadmap hosted marker acceptance']
    require(len(matching)==1 and matching[0]['conclusion']=='success','fixture hosted marker job missing/failed')
    steps=[step for step in matching[0]['steps'] if step['name']=='Independently measure exact immutable fixture marker']
    require(len(steps)==1 and steps[0]['conclusion']=='success','fixture measurement step missing/failed')
    artifacts=get('/actions/runs/'+str(run_id)+'/artifacts?per_page=100')['artifacts']
    rows=[a for a in artifacts if a['name']=='roadmap-fixture-'+task_digest and not a['expired']]
    require(len(rows)==1 and rows[0]['workflow_run']['head_sha']==run['head_sha'],'fixture artifact missing/ambiguous/stale')
    data=request(prefix+'/actions/artifacts/'+str(rows[0]['id'])+'/zip')
    require(len(data)<=MAX_ARTIFACT and rows[0]['digest']=='sha256:'+digest(data),'fixture artifact digest mismatch')
    with zipfile.ZipFile(io.BytesIO(data)) as archive:
        require(archive.namelist()==['roadmap-fixture-receipt.json'],'unexpected fixture archive')
        require(archive.infolist()[0].file_size<=MAX_ARTIFACT,'oversized fixture receipt')
        receipt=c.load_json_strict(archive.read(archive.namelist()[0]).decode())
    expected=dict(schema_version=1,authority_semantics='ISOLATED_ROADMAP_FIXTURE_NOT_PRODUCT_EVIDENCE',
        candidate_sha=candidate_sha,task_digest=task_digest,path=marker_path,marker=marker,
        content_sha256=digest((marker+'\n').encode()),release_sha=release_sha,run_id=run_id,attempt=run['run_attempt'],result='PASS')
    require(receipt==expected,'fixture hosted observation differs from immutable task/marker')
    # Recheck raw immutable GitHub bytes rather than trusting the artifact alone.
    blob=get('/contents/'+marker_path+'?ref='+candidate_sha)
    require(blob['type']=='file' and blob['encoding']=='base64' and blob['size']<1024,'invalid fixture Git blob')
    require(base64.b64decode(blob['content'])==(marker+'\n').encode(),'fixture marker source differs')
    return dict(receipt,job_id=matching[0]['id'],artifact_id=rows[0]['id'],archive_digest=digest(data))


if __name__ == '__main__':
    raise SystemExit(main())
