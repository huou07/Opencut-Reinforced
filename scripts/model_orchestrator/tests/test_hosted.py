#!/usr/bin/env python3
"""CP47 adversarial protocol fixtures, not substitutes for GitHub acceptance."""
from __future__ import annotations
import base64
import copy
import io
import json
from pathlib import Path
import sys
import unittest
import zipfile

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / 'scripts'))
from model_orchestrator import contracts as c, hosted as h

HEAD = 'a' * 40
TREE = 'b' * 40
RUN = 101
WORKFLOW_BYTES = b'name: exact trusted workflow\njobs: {}\n'


def fixture():
    package = dict(repository=c.REPOSITORY_IDENTITY, head_sha=HEAD, tree_sha=TREE,
                   source_manifest_digest='c' * 64)
    expected = dict(package=package, workflow_digest=h.digest(WORKFLOW_BYTES),
                    case_ids={'unit': ['required-case']}, allowed_skips=[])
    suite = dict(id='unit', result='PASS', duration_seconds=0.1, output_digest='d' * 64,
                 cases=[dict(id='required-case', result='PASS', duration_seconds=0.1)])
    receipt = dict(schema_version=2, head_sha=HEAD, workflow=h.WORKFLOW,
                   workflow_digest=expected['workflow_digest'], package=package,
                   package_digest=c.canonical_digest(package), boundary='hosted-fixture',
                   run_id=RUN, run_attempt=1, suites=[suite],
                   authority_semantics='SUPPORTING_FACTS_ONLY_NO_ADOPTION')
    return expected, receipt


def sealed(receipt):
    receipt = h.seal(receipt, 'receipt_digest')
    collector = h.seal(dict(schema_version=2, head_sha=receipt['head_sha'], run_id=RUN, run_attempt=1,
                           workflow_digest=receipt['workflow_digest'], package_digest=receipt['package_digest'],
                           source_receipt_digest=receipt['receipt_digest'],
                           suite_digest=c.canonical_digest(receipt['suites']),
                           isolation='SEPARATE_HOSTED_JOB_NO_CHECKOUT_NO_PACKAGE_EXECUTION'), 'collector_digest')
    return receipt, collector


def archive(document, kind, extra=False):
    output = io.BytesIO()
    with zipfile.ZipFile(output, 'w') as zipped:
        zipped.writestr(kind + '-receipt.json', json.dumps(document))
        if extra:
            zipped.writestr('../unsafe', 'do not extract')
    return output.getvalue()


def api_fixture(receipt, collector, *, extra=False):
    run = dict(id=RUN, run_number=9, run_attempt=1, head_sha=HEAD, event='push', path=h.WORKFLOW,
               status='completed', conclusion='success', referenced_workflows=[], html_url='https://github.com/example/run',
               repository=dict(full_name=c.REPOSITORY_IDENTITY), head_repository=dict(full_name=c.REPOSITORY_IDENTITY))
    jobs = []
    for index, (name, steps) in enumerate([
        ('Fixture acceptance', ['Measure acceptance cases', 'Upload acceptance receipt']),
        ('Collect receipts', ['Download acceptance receipt', 'Validate and seal collector receipt', 'Upload collector receipt'])]):
        jobs.append(dict(id=index + 1, name=name, head_sha=HEAD, conclusion='success', status='completed',
                         steps=[dict(name=step, conclusion='success') for step in steps]))
    artifacts, data = [], {}
    for index, (kind, document) in enumerate([('acceptance', receipt), ('collector', collector)]):
        body = archive(document, kind, extra=extra)
        data['/actions/artifacts/%d/zip' % (index + 1)] = body
        artifacts.append(dict(id=index + 1, name=kind + '-receipt-' + HEAD, expired=False,
                              digest='sha256:' + h.digest(body), size_in_bytes=len(body), workflow_run=dict(id=RUN, head_sha=HEAD)))
    records = {
        '/actions/workflows/control-plane-acceptance.yml/runs?head_sha=' + HEAD + '&per_page=100': dict(workflow_runs=[run]),
        '/actions/runs/101': run,
        '/contents/' + h.WORKFLOW + '?ref=' + HEAD: dict(content=base64.b64encode(WORKFLOW_BYTES).decode()),
        '/git/commits/' + HEAD: dict(sha=HEAD, tree=dict(sha=TREE)),
        '/actions/runs/101/attempts/1/jobs?per_page=100': dict(jobs=jobs),
        '/actions/runs/101/artifacts?per_page=100': dict(total_count=2, artifacts=artifacts),
    }
    def request(path):
        path = path.removeprefix('/repos/' + c.REPOSITORY_IDENTITY)
        return data[path] if path in data else json.dumps(records[path]).encode()
    return records, data, request


class HostedEvidenceTests(unittest.TestCase):
    def test_downloaded_receipts_bind_exact_candidate_source_cases_and_measurements(self):
        expected, body = fixture()
        receipt, collector = sealed(body)
        _, _, request = api_fixture(receipt, collector)
        result = h.collect(expected=expected, request=request)
        self.assertEqual(result['acceptance_receipt'], receipt)
        self.assertEqual(result['collector_receipt'], collector)
        self.assertEqual(result['head_sha'], HEAD)
        self.assertEqual(len(result['artifacts']), 2)
        self.assertEqual(result['authority_semantics'], 'SUPPORTING_FACTS_ONLY_NO_ADOPTION')

    def test_green_metadata_does_not_waive_bad_receipt_contents(self):
        expected, body = fixture()
        for attack in ('candidate', 'package', 'workflow', 'cases', 'failure', 'skip', 'measurement', 'authority', 'extra-authority', 'run'):
            bad = copy.deepcopy(body)
            if attack == 'candidate': bad['head_sha'] = 'f' * 40
            elif attack == 'package': bad['package']['tree_sha'] = 'f' * 40
            elif attack == 'workflow': bad['workflow_digest'] = 'f' * 64
            elif attack == 'cases': bad['suites'][0]['cases'][0]['id'] = 'invented-case'
            elif attack == 'failure': bad['suites'][0]['cases'][0]['result'] = 'FAIL'
            elif attack == 'skip': bad['suites'][0]['cases'][0]['result'] = 'SKIP'
            elif attack == 'measurement': bad['suites'][0]['cases'][0]['duration_seconds'] = -1
            elif attack == 'authority': bad['authority_semantics'] = 'SELF_CERTIFIED_ADOPTED'
            elif attack == 'extra-authority': bad['adopted'] = True
            else: bad['run_id'] = 999
            receipt, collector = sealed(bad)
            _, _, request = api_fixture(receipt, collector)
            with self.subTest(attack=attack), self.assertRaises(c.ContractError):
                h.collect(expected=expected, request=request)

    def test_exact_api_provenance_uniqueness_and_collector_isolation_required(self):
        expected, body = fixture()
        for attack in ('run-sha', 'fork', 'workflow-blob', 'tree', 'job', 'step', 'duplicate', 'expired', 'oversize', 'digest', 'reusable', 'archive'):
            receipt, collector = sealed(body)
            records, _, request = api_fixture(receipt, collector, extra=attack == 'archive')
            run = records['/actions/runs/101']
            jobs = records['/actions/runs/101/attempts/1/jobs?per_page=100']['jobs']
            artifacts = records['/actions/runs/101/artifacts?per_page=100']
            if attack == 'run-sha': run['head_sha'] = 'f' * 40
            elif attack == 'fork': run['head_repository']['full_name'] = 'attacker/fork'
            elif attack == 'workflow-blob': records['/contents/' + h.WORKFLOW + '?ref=' + HEAD]['content'] = base64.b64encode(b'wrong').decode()
            elif attack == 'tree': records['/git/commits/' + HEAD]['tree']['sha'] = 'f' * 40
            elif attack == 'job': jobs[1]['head_sha'] = 'f' * 40
            elif attack == 'step': jobs[1]['steps'][1]['conclusion'] = 'skipped'
            elif attack == 'duplicate': artifacts['artifacts'].append(copy.deepcopy(artifacts['artifacts'][0])); artifacts['total_count'] = 3
            elif attack == 'expired': artifacts['artifacts'][0]['expired'] = True
            elif attack == 'oversize': artifacts['artifacts'][0]['size_in_bytes'] = h.MAX_ARTIFACT + 1
            elif attack == 'digest': artifacts['artifacts'][0]['digest'] = 'sha256:' + '0' * 64
            elif attack == 'reusable': run['referenced_workflows'] = [dict(path='unpinned.yml')]
            with self.subTest(attack=attack), self.assertRaises(c.ContractError):
                h.collect(expected=expected, run_id=RUN, request=request)
        for attack in ('digest', 'isolation', 'source', 'attempt'):
            receipt, collector = sealed(body)
            if attack == 'digest': collector['collector_digest'] = '0' * 64
            else:
                changed = h.unseal(collector, 'collector_digest')
                if attack == 'isolation': changed['isolation'] = 'SAME_PROCESS_AS_PACKAGE'
                elif attack == 'source': changed['source_receipt_digest'] = '0' * 64
                else: changed['run_attempt'] = 2
                collector = h.seal(changed, 'collector_digest')
            _, _, request = api_fixture(receipt, collector)
            with self.subTest(attack=attack), self.assertRaises(c.ContractError):
                h.collect(expected=expected, run_id=RUN, request=request)

    def test_measured_runner_records_real_failure_skip_and_elapsed_time(self):
        class Probe(unittest.TestCase):
            def test_pass(self): self.assertEqual(2 + 2, 4)
            def test_fail(self): self.fail('actual failure observation')
            @unittest.skip('explicit boundary unavailable')
            def test_skip(self): pass
        result = unittest.TextTestRunner(stream=io.StringIO(), resultclass=h.MeasuredResult).run(
            unittest.defaultTestLoader.loadTestsFromTestCase(Probe))
        self.assertFalse(result.wasSuccessful())
        self.assertEqual({row['result'] for row in result.rows.values()}, {'PASS', 'FAIL', 'SKIP'})
        self.assertTrue(all(row['duration_seconds'] >= 0 for row in result.rows.values()))

    def test_optional_live_skips_remain_skip_facts_and_cannot_be_mandatory_pass(self):
        expected, body = fixture()
        body['suites'][0]['cases'][0]['result'] = 'SKIP'
        body['suites'][0]['cases'][0]['reason'] = 'actual live boundary unavailable'
        expected['allowed_skips'] = ['required-case']
        receipt, collector = sealed(body)
        self.assertEqual(h.validate_receipts(receipt, collector, expected=expected, run_id=RUN, run_attempt=1)['suites'][0]['cases'][0]['result'], 'SKIP')
        expected['allowed_skips'] = []
        with self.assertRaises(c.ContractError):
            h.validate_receipts(receipt, collector, expected=expected, run_id=RUN, run_attempt=1)



class RoadmapHostedProofTests(unittest.TestCase):
    """CP47: a fixture PASS JSON cannot replace exact hosted producer/source facts."""
    def test_collector_freezes_exact_measured_suite_inventory(self):
        import ast,re
        workflow=(h.ROOT/h.WORKFLOW).read_text()
        match=re.search(r"^\s*expected_suites = (.*)$",workflow,re.MULTILINE)
        self.assertIsNotNone(match)
        self.assertEqual(ast.literal_eval(match[1]),set(h.SUITES))

    def inputs(self):
        import io,zipfile,hashlib,json,base64
        from model_orchestrator import hosted as h
        sha='a'*40;candidate='b'*40;task='c'*64
        receipt=dict(schema_version=1,authority_semantics='ISOLATED_ROADMAP_FIXTURE_NOT_PRODUCT_EVIDENCE',
            candidate_sha=candidate,task_digest=task,path='roadmap-1.txt',marker='# ROADMAP_1_PASS',
            content_sha256=hashlib.sha256(b'# ROADMAP_1_PASS\n').hexdigest(),release_sha=sha,run_id=123,attempt=1,result='PASS')
        data=io.BytesIO()
        with zipfile.ZipFile(data,'w') as z:z.writestr('roadmap-fixture-receipt.json',json.dumps(receipt))
        archive=data.getvalue()
        prefix='/repos/huou07/Opencut-Reinforced'
        api={prefix+'/actions/runs/123':dict(repository=dict(full_name='huou07/Opencut-Reinforced'),head_sha=sha,event='workflow_dispatch',conclusion='success',path=h.WORKFLOW,run_attempt=1),
            prefix+'/actions/runs/123/jobs?per_page=100':dict(jobs=[dict(id=456,name='Isolated roadmap hosted marker acceptance',conclusion='success',steps=[dict(name='Independently measure exact immutable fixture marker',conclusion='success')])]),
            prefix+'/actions/runs/123/artifacts?per_page=100':dict(artifacts=[dict(id=789,name='roadmap-fixture-'+task,expired=False,digest='sha256:'+hashlib.sha256(archive).hexdigest(),workflow_run=dict(head_sha=sha))]),
            prefix+'/actions/artifacts/789/zip':archive,
            prefix+'/contents/roadmap-1.txt?ref='+candidate:dict(type='file',encoding='base64',size=17,content=base64.b64encode(b'# ROADMAP_1_PASS\n').decode())}
        def request(path):
            value=api[path];return value if isinstance(value,bytes) else json.dumps(value).encode()
        return api,dict(run_id=123,release_sha=sha,candidate_sha=candidate,task_digest=task,marker_path='roadmap-1.txt',marker='# ROADMAP_1_PASS',request=request)

    def test_fixture_observation_requires_exact_hosted_binding(self):
        from model_orchestrator import hosted as h
        api,kwargs=self.inputs()
        result=h.collect_roadmap_fixture(**kwargs)
        self.assertEqual(result['candidate_sha'],kwargs['candidate_sha'])
        for path,key,value in [
            ('/actions/runs/123','head_sha','d'*40),
            ('/actions/runs/123','conclusion','failure'),
            ('/contents/roadmap-1.txt?ref='+'b'*40,'content','d3Jvbmc='),
        ]:
            api,args=self.inputs();api['/repos/huou07/Opencut-Reinforced'+path][key]=value
            with self.subTest(key=key),self.assertRaises(c.ContractError):h.collect_roadmap_fixture(**args)
        api,args=self.inputs()
        api['/repos/huou07/Opencut-Reinforced/actions/runs/123/jobs?per_page=100']['jobs'][0]['steps'][0]['conclusion']='skipped'
        with self.assertRaises(c.ContractError):h.collect_roadmap_fixture(**args)

if __name__ == '__main__':
    unittest.main(verbosity=2)
