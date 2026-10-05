"""CP01–05/33: bounded roadmap root and deterministic task derivation attacks.

Synthetic release pins here are unit fixtures only, never adoption evidence.
The opt-in live fixture must use a real separately adopted release.
"""
import copy
from dataclasses import replace
import hashlib
import json
from pathlib import Path
import tempfile
import time
import unittest
from unittest.mock import patch

from model_orchestrator import contracts as c, product, roadmap as r
from model_orchestrator.tests.test_product_operational import ProductFixture, git
import execution_plan


class RoadmapFixture:
    def __init__(self,root,**kwargs):
        f=self.fixture=ProductFixture(root,**dict(kwargs,toy=True))
        self.__dict__.update(f.__dict__)
        git(self.controller,'config','user.name','Roadmap Unit Fixture')
        git(self.controller,'config','user.email','fixture@example.invalid')
        plan=json.loads(c._git(self.product,'show',c.TRUSTED_DESIGN_BASE+':docs/execution/PLAN.json').decode())
        original=next(row for row in plan['checkpoints'] if row['id']=='9B')
        rows=[]
        for index,name in enumerate(('OR-A','OR-B')):
            row=copy.deepcopy(original)
            row.update(id=name,title='Isolated roadmap '+name,prerequisite_checkpoint_ids=[] if index==0 else ['OR-A'],
                next_checkpoint_relation='OR-B' if index==0 else None,required_evidence_classes=['UNIT'],
                expected_project_schema_effect_category='none',expected_ipc_effect_category='none',
                architecture_gate=False,developer_preview_required=False,milestone_membership=[],
                spec_document='docs/execution/phases/OR_ROADMAP_FIXTURE.md')
            row.pop('runner_allowed_protected_paths',None)
            rows.append(row)
        plan=dict(schema_version=1,checkpoints=rows,phases={'9':plan['phases']['9']})
        state=dict(schema_version=1,current_next='OR-A',checkpoints={'OR-A':'NEXT','OR-B':'PLANNED'},
            phase_status={'5':'DONE','9':'IN_PROGRESS'},last_updated='2026-10-05',verified_contract_versions=dict(project_schema=7,recovery_schema=1,ipc_protocol=1))
        (self.product/rows[0]['spec_document']).write_text('# Isolated two-checkpoint roadmap\n\nEach checkpoint writes its own marker only. No 9B or product publication. The frozen plan is immutable; only the independently validating supervisor advances STATE after review, verification, guarded local promotion and actual hosted marker acceptance.\n')
        for name,record in (('PLAN',plan),('STATE',state)):
            (self.product/('docs/execution/'+name+'.json')).write_text(c.canonical_json(record)+'\n')
        git(self.product,'add','-A');git(self.product,'commit','-qm','fixture: immutable two-checkpoint product roadmap')
        self.base=git(self.product,'rev-parse','HEAD')
        git(self.product,'push','-q',str(self.bare),self.base+':refs/heads/main')
        release=c.load_release_authority(self.candidate,self.controller,bootstrap=self.bootstrap)
        facts=product.release_active(release)
        entries=[]
        for index,row in enumerate(rows,1):
            task=copy.deepcopy(self.task)
            task.update(checkpoint_id=row['id'],base_sha=self.base,evidence_classes=['UNIT'],required_documentation=[row['spec_document']],
                allowed_paths=['roadmap-'+str(index)+'.txt'],goal='Write the exact isolated roadmap marker '+str(index))
            task['acceptance_requirements']=[a for a in task['acceptance_requirements'] if a['class_id']=='UNIT']
            task['acceptance_cases']=[case for a in task['acceptance_requirements'] for case in a['case_ids']]
            harness_path='controller/roadmap/harnesses/'+str(index)+'.sh'
            harness=('#!/bin/sh -e\n'
                     'test "$(cat /candidate/roadmap-'+str(index)+'.txt)" = "# ROADMAP_'+str(index)+'_PASS"\n'
                     'printf \'{"cases":[{"id":"case1","result":"PASS"}]}\\n\'\n')
            hdigest=hashlib.sha256(harness.encode()).hexdigest()
            (self.controller/harness_path).parent.mkdir(parents=True,exist_ok=True)
            (self.controller/harness_path).write_text(harness)
            task['harness_digest']=hdigest
            task['check_argv'][0].update(argv=['/bin/sh','-e','/verifier/'+harness_path],harness_digest=hdigest)
            for req in task['acceptance_requirements']:req['harness_digest']=hdigest
            catalog=copy.deepcopy(self.catalog)
            catalog['checks']['unit'].update(harness=harness_path,harness_digest=hdigest)
            catalog['acceptance']={a['class_id']:dict(requirement_digest=c.canonical_digest(a),harness=harness_path,
                boundary='rootless',check_ids=['unit'],case_bindings={a['case_ids'][0]:'case1'}) for a in task['acceptance_requirements']}
            catalog_path='controller/roadmap/catalogs/'+str(index)+'.json'
            (self.controller/catalog_path).parent.mkdir(parents=True,exist_ok=True)
            (self.controller/catalog_path).write_text(c.canonical_json(catalog)+'\n')
            task_path='controller/roadmap/blueprints/'+str(index)+'.json'
            target=self.controller/task_path;target.parent.mkdir(parents=True,exist_ok=True);target.write_text(c.canonical_json(task)+'\n')
            entries.append(dict(checkpoint_id=row['id'],task_path=task_path,task_digest=c.canonical_digest(task),catalog_path=catalog_path,catalog_digest=c.canonical_digest(catalog)))
        executor_path='controller/roadmap/executor.py'
        code=b'raise RuntimeError("Unit-only fixture: no inference/execution/adoption")\n'
        (self.controller/executor_path).write_bytes(code)
        self.delegation=dict(schema_version=1,repository=c.REPOSITORY_IDENTITY,purpose='OPERATOR_ROADMAP_DELEGATION',
            authorization_id='unit-operator-roadmap',nonce='unit-roadmap-nonce',expires_at_unix=int(time.time())+9000,release_sha=self.bootstrap.release_sha,
            adoption_digest=c.canonical_digest(facts['adoption']),initial_base_sha=self.base,
            plan_digest=hashlib.sha256((self.product/'docs/execution/PLAN.json').read_bytes()).hexdigest(),
            state_digest=hashlib.sha256((self.product/'docs/execution/STATE.json').read_bytes()).hexdigest(),
            protected_manifest_digest=c.canonical_digest(r.protected_manifest(self.product,self.base)),
            execution_profile='ISOLATED_FIXTURE',destination_url=str(self.bare),checkpoints=entries,
            budget_ceiling=dict(tokens=1000000,cost_microusd=500000,wall_seconds=9000,tool_calls=1000),
            executor_path=executor_path,executor_digest=hashlib.sha256(code).hexdigest())
        self.delegation_path='controller/roadmap/delegation.json'
        (self.controller/self.delegation_path).write_text(c.canonical_json(self.delegation)+'\n')
        git(self.controller,'add','-A');git(self.controller,'commit','-qm','fixture: one bounded operator roadmap delegation')
        self.source=git(self.controller,'rev-parse','HEAD')
        self.bootstrap=replace(self.bootstrap,source_sha=self.source)
        self.roadmap_pin=r.RoadmapPin(self.source,c.RecordPin(self.delegation_path,c.canonical_digest(self.delegation)))


class RoadmapAuthorityTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.directory=tempfile.TemporaryDirectory(prefix='v2-roadmap-unit-')
        cls.fixture=RoadmapFixture(cls.directory.name)

    @classmethod
    def tearDownClass(cls):cls.directory.cleanup()

    def setUp(self):
        f=self.fixture
        git(f.controller,'checkout','-q','--detach',f.source)
        git(f.controller,'clean','-fd')
        git(f.product,'checkout','-q','--detach',f.base)
        git(f.product,'clean','-fd')

    def test_one_operator_root_issues_exact_NEXT(self):
        f=self.fixture
        bp,pin,task=r.issue_next(f.candidate,f.controller,f.product,f.bootstrap,f.roadmap_pin)
        a=product.load_operational_authority(f.candidate,f.controller,f.product,bootstrap=bp,product_pin=pin,roadmap_pin=f.roadmap_pin)
        self.assertEqual(c._release_authority(a)['operational']['task'],task)
        self.assertEqual(task['checkpoint_id'],'OR-A')
        with self.assertRaisesRegex(c.ContractError,'typed operator roadmap'):
            product.load_operational_authority(f.candidate,f.controller,f.product,bootstrap=bp,product_pin=pin)
        with self.assertRaisesRegex(c.ContractError,'unfinished task'):
            r.issue_next(f.candidate,f.controller,f.product,bp,f.roadmap_pin)

    def test_skip_broaden_and_exceed_delegation_refuse(self):
        f=self.fixture;release=c.load_release_authority(f.candidate,f.controller,bootstrap=f.bootstrap)
        for seq in (0,2,3):
            with self.subTest(sequence=seq),self.assertRaises(c.ContractError):
                r.derive_task(release,f.product,f.roadmap_pin,f.delegation,seq,f.base)
        bp,pin,task=r.issue_next(f.candidate,f.controller,f.product,f.bootstrap,f.roadmap_pin)
        path=f.controller/'controller/roadmap/tasks/1/task.json'
        record=json.loads(path.read_text());record['allowed_paths'].append('docs/execution/PLAN.json');path.write_text(c.canonical_json(record)+'\n')
        git(f.controller,'add','-A');git(f.controller,'commit','-qm','fixture: attack broaden delegated task')
        with self.assertRaisesRegex(c.ContractError,'derived task/catalog'):
            product.load_operational_authority(f.candidate,f.controller,f.product,bootstrap=replace(bp,source_sha=git(f.controller,'rev-parse','HEAD')),product_pin=pin,roadmap_pin=f.roadmap_pin)

    def test_stale_base_self_modified_plan_and_unadopted_release_refuse(self):
        f=self.fixture
        with self.assertRaises(c.ContractError):
            r.issue_next(f.candidate,f.controller,f.product,replace(f.bootstrap,adoption=None),f.roadmap_pin)
        release=c.load_release_authority(f.candidate,f.controller,bootstrap=f.bootstrap)
        (f.product/'docs/execution/PLAN.json').write_text('{}\n');git(f.product,'add','-A');git(f.product,'commit','-qm','fixture: attack self modify roadmap')
        wrong=git(f.product,'rev-parse','HEAD')
        with self.assertRaisesRegex(c.ContractError,'roadmap/spec/policy'):
            r.derive_task(release,f.product,f.roadmap_pin,f.delegation,1,wrong)
        with self.assertRaises(c.ContractError):
            r.issue_next(f.candidate,f.controller,f.product,f.bootstrap,f.roadmap_pin)

    def test_run_budget_reservations_unknown_and_excess_usage_fail_closed(self):
        f=self.fixture;d=copy.deepcopy(f.delegation)
        reservation=dict(cost_microusd=100000,tokens=200000,wall_seconds=1800,tool_calls=200)
        attempt=dict(sequence=1,attempt=1,reservation=reservation,usage=None,result='RESERVED')
        self.assertEqual(r.validate_budget(d,[attempt]),reservation)
        d['budget_ceiling']['cost_microusd']=99999
        with self.assertRaisesRegex(c.ContractError,'cost_microusd'):r.validate_budget(d,[attempt])
        d['budget_ceiling']['cost_microusd']=500000
        attempt['usage']=dict(reservation,tokens=None)
        with self.assertRaisesRegex(c.ContractError,'unknown/invalid'):r.validate_budget(d,[attempt])
        attempt['usage']=dict(reservation,cost_microusd=500001)
        with self.assertRaises(c.ContractError):r.validate_budget(d,[attempt])

    def test_supervisor_transition_derives_B_without_new_operator_identity(self):
        # Unit-only mocked hosted boundary; the live acceptance repeats this
        # with actual OpenCode roles, guarded promotion and GitHub artifacts.
        import agent_supervisor
        f=self.fixture
        bp,pin,task=r.issue_next(f.candidate,f.controller,f.product,f.bootstrap,f.roadmap_pin)
        auth=r._json(f.controller,bp.source_sha,pin.path)
        marker=f.product/'roadmap-1.txt';marker.write_text('# ROADMAP_1_PASS\n')
        git(f.product,'add','roadmap-1.txt');git(f.product,'commit','-qm','fixture: unit-only marker implementation')
        implementation=git(f.product,'rev-parse','HEAD')
        plan,state=execution_plan.load_plan_state(f.product)
        after=agent_supervisor.advance_state_once(state,plan,'OR-A',repo_root=f.product)
        proof=dict(result='PASS',candidate_sha=implementation,run_id=123)
        evidence=dict(authority_semantics='ISOLATED_ROADMAP_FIXTURE_NOT_PRODUCT_EVIDENCE',implementation_sha=implementation,
            task_digest=auth['task_digest'],roadmap_delegation_digest=f.roadmap_pin.delegation.digest,hosted_proof=proof)
        ep=f.product/'docs/execution/evidence/OR-A.json';ep.parent.mkdir(exist_ok=True);ep.write_text(c.canonical_json(evidence)+'\n')
        (f.product/'docs/execution/STATE.json').write_text(c.canonical_json(after)+'\n')
        git(f.product,'add','-A');git(f.product,'commit','-qm','fixture: unit-only supervisor state/evidence completion')
        completed=git(f.product,'rev-parse','HEAD')
        git(f.product,'push','-q',str(f.bare),completed+':refs/heads/main')
        run=r._run_record(c.load_release_authority(f.candidate,f.controller,bootstrap=bp))
        run['tasks'][0]['completed_base_sha']=completed
        run['attempts'][0].update(result='DONE',usage=run['attempts'][0]['reservation'])
        source=r._commit(f.controller,{r.RUN_PATH:run},'fixture: unit-only accounted completion')
        with patch('model_orchestrator.hosted.collect_roadmap_fixture',return_value=proof):
            bp2,pin2,task2=r.issue_next(f.candidate,f.controller,f.product,replace(bp,source_sha=source),f.roadmap_pin)
        self.assertEqual((task2['checkpoint_id'],task2['base_sha']),('OR-B',completed))
        auth2=r._json(f.controller,bp2.source_sha,pin2.path)
        self.assertEqual((auth2['authorization_id'],auth2['nonce']), (auth['authorization_id'],auth['nonce']))
        self.assertEqual(auth2['roadmap_sequence'],2)
        # Restore only this disposable fixture's bare remote for sibling tests.
        other=f.root/'unit-remote-replacement.git'
        git(f.root,'init','-q','--bare',str(other))
        git(f.product,'push','-q',str(other),f.base+':refs/heads/main')
        import shutil
        shutil.rmtree(f.bare);other.rename(f.bare)

    def test_product_claims_keep_attempt_history_and_refuse_unsettled_repair(self):
        # Unit fixture: no process launched; READY resets exercise only the
        # durable claim ledger. Real repair/preservation is the live boundary.
        from model_orchestrator import store as st,orchestrator as o
        from model_orchestrator.tests.test_attempt_execution import reset_ready_test_only
        f=self.fixture
        bp,pin,task=r.issue_next(f.candidate,f.controller,f.product,f.bootstrap,f.roadmap_pin)
        authority=product.load_operational_authority(f.candidate,f.controller,f.product,bootstrap=bp,product_pin=pin,roadmap_pin=f.roadmap_pin)
        runtime=st.RuntimeStore(f.root/'claim-runtime',authority);runtime.initialize()
        digest=runtime.register_task(task,task)
        for attempt in range(3):
            with runtime.lock('task',task['task_id']):
                runtime.claim(task['task_id'],digest,owner_nonce='unit-owner',boot_identity='unit-boot',stage_id='unit-stage-'+str(attempt),stage_nonce='unit-nonce-'+str(attempt))
                with self.assertRaisesRegex(o.OrchestratorError,'unreconciled'):
                    o.prepare_worker_repair(store=runtime,task_id=task['task_id'],task_path='unused',catalog_path='unused',guard_root='unused',attempt_dir='unused',destination='unused')
            reset_ready_test_only(runtime,task['task_id'])
        episode=o.AttemptLedger.episode_id(task['checkpoint_id'],'worker',c.canonical_json(sorted(task['required_check_ids'])))
        self.assertEqual(o.AttemptLedger.load(runtime).speculative_used(episode),2)
        with runtime.lock('task',task['task_id']),self.assertRaisesRegex(o.OrchestratorError,'two speculative'):
            runtime.claim(task['task_id'],digest,owner_nonce='unit-owner',boot_identity='unit-boot',stage_id='unit-fourth',stage_nonce='unit-fourth')

    def test_failed_worker_repair_archives_work_and_keeps_original_binding(self):
        # Unit-only no-process lifecycle fixture. Real Docker termination and
        # failed verification/repair are required separately by live acceptance.
        import shutil
        from types import SimpleNamespace
        from model_orchestrator import store as st,orchestrator as o,sandbox as b,workspace as w
        f=self.fixture
        bp,pin,task=r.issue_next(f.candidate,f.controller,f.product,f.bootstrap,f.roadmap_pin)
        authority=product.load_operational_authority(f.candidate,f.controller,f.product,bootstrap=bp,product_pin=pin,roadmap_pin=f.roadmap_pin)
        runtime=st.RuntimeStore(f.root/'repair-runtime',authority);runtime.initialize();digest=runtime.register_task(task,task)
        storage=f.root/'repair-storage';storage.mkdir()
        candidate=b.create_candidate(f.product,storage/'original',task['base_sha'],authority=authority)
        runtime.register_candidate(task['task_id'],candidate)
        with runtime.lock('task',task['task_id']):
            stage=runtime.claim(task['task_id'],digest,owner_nonce='unit-owner',boot_identity='unit-boot',stage_id='unit-stage',stage_nonce='unit-nonce')
            path=w.reserve_launch(runtime,stage,candidate,storage,'unit-daemon','sha256:'+'b'*64,'IMPLEMENTATION')
            b._make_launch_view(candidate,storage,task['task_id'],'unit-stage','IMPLEMENTATION',path)
            w.bind_launch(runtime,stage)
            (path/'roadmap-1.txt').write_text('WRONG\n');git(path,'add','roadmap-1.txt');git(path,'commit','-qm','unit: retained failed marker')
            preserved=runtime.root/'artifacts'/runtime.launch_record(stage)['container_name']/'candidate'
            preserved.parent.mkdir(parents=True);shutil.copytree(path,preserved)
            for phase in ('RECONCILED','PRESERVING','PRESERVED'):
                launch=runtime.launch_record(stage);launch['phase']=phase
                if phase in ('PRESERVING','PRESERVED'):launch['result']=dict(path=str(preserved),manifest=w.manifest(preserved),head=git(preserved,'rev-parse','HEAD'))
                runtime.update_launch(stage,launch)
            current=runtime.inspect()
            def unit_settled(state):
                state['tasks'][task['task_id']].update(status='SETTLED',stage=None);state['active_task']=None
            runtime.transaction(current['sequence'],current['epoch'],unit_settled)
            guard=SimpleNamespace(verify=lambda:dict(vetoes=[],head=git(preserved,'rev-parse','HEAD')))
            with patch('model_orchestrator.guards.restore_guarded',return_value=guard),patch('model_orchestrator.verification.readiness',return_value=dict(verification_passed=False)):
                repair=o.prepare_worker_repair(store=runtime,task_id=task['task_id'],task_path=r._json(f.controller,bp.source_sha,pin.path)['task_path'],
                    catalog_path=r._json(f.controller,bp.source_sha,pin.path)['catalog_path'],guard_root='unused',attempt_dir='unused',destination=storage/'repaired-input')
            self.assertIn(repair,runtime.inspect()['object_digests'])
            self.assertEqual(runtime.candidate_descriptor(task['task_id'])['root'],str(candidate.root))
            retained=o.worker_input(runtime,task['task_id'])
            self.assertEqual((retained.root/'roadmap-1.txt').read_text(),'WRONG\n')
            new=runtime.claim(task['task_id'],digest,owner_nonce='unit-owner',boot_identity='unit-boot',stage_id='unit-repair',stage_nonce='unit-repair')
            w.reserve_launch(runtime,new,retained,storage,'unit-daemon','sha256:'+'b'*64,'IMPLEMENTATION')
            self.assertEqual(runtime.inspect()['tasks'][task['task_id']]['attempt'],2)
            (retained.root/'roadmap-1.txt').write_text('tampered\n')
            with self.assertRaisesRegex(o.OrchestratorError,'changed or disappeared'):o.worker_input(runtime,task['task_id'])

    def test_model_raw_delegation_has_zero_authority(self):
        f=self.fixture;release=c.load_release_authority(f.candidate,f.controller,bootstrap=f.bootstrap)
        with self.assertRaises(c.ContractError):r.load_delegation(release,f.product,f.delegation)
        wrong=r.RoadmapPin(f.source,c.RecordPin(f.delegation_path,'0'*64))
        with self.assertRaises(c.ContractError):r.load_delegation(release,f.product,wrong)


class RecordedAttemptTests(unittest.TestCase):
    def test_recollects_pinned_attempt_and_refuses_dispatch(self):
        from types import SimpleNamespace
        from unittest.mock import Mock
        api = SimpleNamespace(authenticated=True, get=Mock(return_value=dict(id=12,run_attempt=1)))
        pinned = r.RecordedAttemptApi(api, dict(gates=[dict(run_id=12,run_attempt=1,workflow_file='.github/workflows/test.yml')],developer_preview=dict(required=False)))
        prefix='/repos/'+c.REPOSITORY_IDENTITY
        self.assertEqual(pinned.get(prefix+'/actions/workflows/test.yml/runs?head_sha=abc')['workflow_runs'][0]['run_attempt'],1)
        self.assertEqual(api.get.call_args.args[0],prefix+'/actions/runs/12/attempts/1')
        pinned.get(prefix+'/actions/runs/12/jobs?per_page=100')
        self.assertEqual(api.get.call_args.args[0],prefix+'/actions/runs/12/attempts/1/jobs?per_page=100')
        for action in (lambda:pinned.post('anything'),lambda:pinned.get(prefix+'/actions/runs/13/jobs')):
            with self.assertRaises(c.ContractError):action()
        api.get.return_value=dict(id=12,run_attempt=2)
        with self.assertRaises(c.ContractError):pinned.get(prefix+'/actions/workflows/test.yml/runs?head_sha=abc')

    def test_protected_manifest_excludes_product_version_source(self):
        entries=[dict(path=path) for path in ('crates/or_core/src/project_document.rs','crates/or_ipc/src/protocol.rs',
            'README.md','docs/execution/PLAN.json','scripts/model_orchestrator/roadmap.py','docs/execution/phases/PHASE_9.md',
            'docs/execution/STATE.json','docs/execution/evidence/OR-A.json')]
        with patch.object(c,'build_manifest',return_value=entries):
            self.assertEqual([row['path'] for row in r.protected_manifest('.', 'unused')],
                ['docs/execution/PLAN.json','scripts/model_orchestrator/roadmap.py','docs/execution/phases/PHASE_9.md'])


if __name__=='__main__':unittest.main(verbosity=2)
