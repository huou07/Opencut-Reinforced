#!/usr/bin/env python3
"""M0-R3 semantic matrices. Real Git fixtures; no runtime/model/product launch."""
from __future__ import annotations
import copy
import hashlib
from dataclasses import replace
import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(REPO_ROOT / 'scripts'))
from model_orchestrator import contracts as c

SCHEMAS = c.load_protocol_schemas(REPO_ROOT)
DIGEST = 'd' * 64
SHA = 'a' * 40


def resources():
    return dict(cpu=1, memory_bytes=2**30, pids=64, disk_bytes=2**30, output_bytes=2**20, wall_seconds=60)


def valid_task(schemas=None):
    check = dict(id='unit', argv=['python3', '-m', 'unittest'], cwd='.', environment={'LANG':'C'}, environment_digest=c.canonical_digest({'LANG':'C'}), harness_digest=DIGEST, required_cases=['case1'], expected_exit_codes=[0], timeout_seconds=30, retry_budget=0, resource_limits=resources(), allow_empty_cases=False, required_metrics=[])
    return dict(schema_version=1, task_id='task-001', task_kind='control_plane_phase', repository=c.REPOSITORY_IDENTITY, checkpoint_id='M0', base_sha=SHA, candidate_branch='control/fixture', authority_digest=DIGEST, template_digest=DIGEST, goal='Verify disabled contract fixtures', out_of_scope=['No product work'], allowed_paths=['scripts/model_orchestrator/contracts.py'], forbidden_paths=['docs/execution/STATE.json'], required_documentation=['docs/execution/AGENT_EXECUTION.md'], required_tests=['unit'], invariant_ids=['INV-NO-SATISFICE'], project_schema_effect='none', recovery_schema_effect='none', ipc_effect='none', user_action='Operator checks the disabled candidate', observable_outcome='Invalid contracts refuse', production_boundary='Disposable controller fixture', error_cases=['Malformed record'], persistence_expectation='Immutable original contract retained', permission_expectation='No controller credentials in candidate', required_check_ids=['unit'], check_argv=[check], harness_digest=DIGEST, case_inventory=['case1'], execution_boundary='controller_verifier', expected_result='Frozen expected exit and case sets', resource_limits=resources(), evidence_classes=['UNIT'], acceptance_cases=['journey1'], acceptance_requirements=[dict(class_id='UNIT', case_ids=['journey1'], production_boundary='Disposable controller fixture', environment_digest=DIGEST, package_digest=DIGEST, permission_digest=DIGEST, persistence_expectation='Immutable receipt', harness_digest=DIGEST)], performance_applicability='not_applicable', performance_budgets=dict(applicability='not_applicable', authorization_digest=DIGEST, rationale='This fixture measures contract rejection, no realtime product path'), role_enrollment_ids=['fixture'], budget=dict(tokens=1000,cost_microusd=0,wall_seconds=60,tool_calls=20,speculative_corrections=2,causal_repairs=1), stop_conditions=['Stop on invalid contract'], resume_stage='NONE')


def receipt_context(task):
    check=task['check_argv'][0]
    return dict(frozen_task=copy.deepcopy(task),task_id=task['task_id'],candidate_sha=SHA,authority_digest=task['authority_digest'],task_contract_digest=c.canonical_digest(task),command_digest=c.canonical_digest(check),environment_digest=check['environment_digest'],lease_epoch=1,sequence=1,stage_nonce='stage-1')


def valid_receipt(task):
    return dict(schema_version=1,check_id='unit',exit_code=0,outcome='PASS',executed_cases=['case1'],passed_cases=['case1'],failed_cases=[],skipped_cases=[],artifact_digest=DIGEST,signal=None,timed_out=False,executable_found=True,duration_seconds=1,measurements=[],**{k:v for k,v in receipt_context(task).items() if k!='frozen_task'})


class TaskMatrixTests(unittest.TestCase):
    """CP03: recursive syntax, semantics and immutable measurement floor."""
    def test_valid_controller_bound_task(self):
        task=valid_task(); c.validate_task_contract(task,SCHEMAS,frozen_template=copy.deepcopy(task))
        # Repeated command arguments are valid; semantic identifiers stay unique.
        task['check_argv'][0]['argv']=['python3','-c','print(1)','same','same']
        c.validate_task_contract(task,SCHEMAS,frozen_template=copy.deepcopy(task))

    def test_all_nested_negatives_fail_before_any_dispatch(self):
        base=valid_task()
        mutations=[]
        for key in resources():
            for value in (-1,0,True,None,10**1000):
                mutations.append((('resource_limits',key),value))
        mutations += [(('check_argv',0,'argv'),v) for v in ([], '', [''], ['python3',3], ['python3',True])]
        mutations += [(('check_argv',0,'environment'),v) for v in ('x',{'PYTHONPATH':'evil'},{'LANG':3},{'LANG':True})]
        mutations += [(('check_argv',0,'cwd'),v) for v in ('../escape','/absolute','a/../b','a\\b','a\x00b')]
        mutations += [(('resource_limits','unknown'),1),(('performance_budgets','waive'),True),(('budget','speculative_corrections'),3),(('budget','causal_repairs'),2),(('check_argv',0,'retry_budget'),-1),(('check_argv',0,'retry_budget'),True),(('check_argv',0,'retry_budget'),3),(('check_argv',0,'timeout_seconds'),0),(('check_argv',0,'timeout_seconds'),61),(('check_argv',0,'unknown'),True),(('required_check_ids',),['unit','unit']),(('acceptance_cases',),['journey1','journey1']),(('acceptance_requirements',0,'case_ids'),['journey1','journey1']),(('schema_version',),True),(('schema_version',),1.0),(('allowed_paths',),['../escape'])]
        for path,value in mutations:
            with self.subTest(path=path,value=value):
                task=copy.deepcopy(base); target=task
                for key in path[:-1]:target=target[key]
                target[path[-1]]=value
                # Intrinsic invalidity must refuse even if the supplied floor
                # contains the same malformed data; drift alone is insufficient.
                with self.assertRaises(c.ContractError):c.validate_task_contract(task,SCHEMAS,frozen_template=copy.deepcopy(task))

    def test_product_floor_cannot_be_reduced(self):
        base=valid_task()
        changes={'goal':'reduced goal','allowed_paths':[], 'required_tests':[], 'acceptance_requirements':[], 'permission_expectation':'disabled', 'production_boundary':'mock', 'performance_applicability':'applicable'}
        for key,value in changes.items():
            with self.subTest(key=key):
                task=copy.deepcopy(base);task[key]=value
                with self.assertRaises(c.ContractError):c.validate_task_contract(task,SCHEMAS,frozen_template=base)
        with self.assertRaises(c.ContractError):c.validate_task_contract(base,SCHEMAS)

    def test_applicable_performance_requires_all_bounds(self):
        task=valid_task();task['performance_applicability']='applicable';task['performance_budgets']=dict(applicability='applicable',authorization_digest=DIGEST,rationale='bounded workload',baseline_digest=DIGEST,method_digest=DIGEST,duration_seconds=10,sample_count=100,bounds=[dict(metric='latency',maximum=20,baseline_maximum=15)],ownership_constraints=['One persistent queue'])
        task['check_argv'][0]['required_metrics']=['latency']
        c.validate_task_contract(task,SCHEMAS,frozen_template=copy.deepcopy(task))
        for key in ('bounds','baseline_digest','sample_count','ownership_constraints','duration_seconds'):
            bad=copy.deepcopy(task);del bad['performance_budgets'][key]
            with self.assertRaises(c.ContractError):c.validate_task_contract(bad,SCHEMAS,frozen_template=copy.deepcopy(bad))

    def test_strict_json_numbers_keys_and_duplicates(self):
        for text in ('{"x":1,"x":2}','{"x":NaN}','{"x":Infinity}','{"x":1e999}'):
            with self.assertRaises(c.ContractError):c.load_json_strict(text)
        with self.assertRaises(c.ContractError):c.canonical_json({1:'not string'})
        self.assertEqual(c.canonical_digest({'b':1,'a':2}),c.canonical_digest({'a':2,'b':1}))


class ReceiptMatrixTests(unittest.TestCase):
    """CP03: PASS requires frozen command, exact attempt and complete case results."""
    def verify(self,receipt,task=None,context=None):
        task=task or valid_task()
        return c.validate_verification_receipt(receipt,SCHEMAS,task=task,candidate_sha=SHA,context=context or receipt_context(task))

    def test_positive_pass_and_explicit_nonzero_success(self):
        task=valid_task();self.verify(valid_receipt(task))
        task['check_argv'][0]['expected_exit_codes']=[23]
        receipt=valid_receipt(task);receipt['exit_code']=23;self.verify(receipt,task)

    def test_receipt_negative_matrix(self):
        task=valid_task();base=valid_receipt(task)
        changes={'exit_code':23,'timed_out':True,'signal':9,'executable_found':False,'skipped_cases':['case1'],'executed_cases':[],'passed_cases':[],'failed_cases':['case1'],'candidate_sha':'b'*40,'command_digest':'b'*64,'environment_digest':'b'*64,'authority_digest':'b'*64,'task_contract_digest':'b'*64,'lease_epoch':2,'sequence':2,'stage_nonce':'old-stage','outcome':'MAYBE','duration_seconds':31}
        for key,value in changes.items():
            with self.subTest(key=key):
                bad=copy.deepcopy(base);bad[key]=value
                with self.assertRaises(c.ContractError):self.verify(bad)
        for key in ('executed_cases','passed_cases','failed_cases','skipped_cases'):
            for value in (['case1','case1'],['unknown']):
                with self.subTest(key=key,value=value):
                    bad=copy.deepcopy(base);bad[key]=value
                    with self.assertRaises(c.ContractError):self.verify(bad)
        with self.assertRaises(c.ContractError):c.validate_record(base,'verification_receipt',SCHEMAS)
        context=receipt_context(task);context['task_contract_digest']='b'*64
        with self.assertRaises(c.ContractError):self.verify(base,context=context)

    def test_empty_inventory_only_when_frozen_empty(self):
        task=valid_task();task['check_argv'][0]['required_cases']=[];task['case_inventory']=[]
        # Empty local check inventory is distinct from required acceptance journey.
        task['check_argv'][0]['allow_empty_cases']=True
        c.validate_task_contract(task,SCHEMAS,frozen_template=copy.deepcopy(task))
        receipt=valid_receipt(task);receipt['executed_cases']=[];receipt['passed_cases']=[]
        self.verify(receipt,task)
        with self.assertRaises(c.ContractError):self.verify(receipt)

    def test_measured_performance_failure_is_not_pass(self):
        task=valid_task();task['performance_applicability']='applicable';task['performance_budgets'].update(applicability='applicable',baseline_digest=DIGEST,method_digest=DIGEST,duration_seconds=10,sample_count=100,bounds=[dict(metric='latency',maximum=20,baseline_maximum=15)],ownership_constraints=['bounded queue']);task['check_argv'][0]['required_metrics']=['latency']
        receipt=valid_receipt(task)
        for measurements in ([],[dict(metric='latency',value=16)],[dict(metric='wrong',value=1)],[dict(metric='latency',value=1)]*2):
            receipt['measurements']=measurements
            with self.assertRaises(c.ContractError):self.verify(receipt,task)
        receipt['measurements']=[dict(metric='latency',value=10)];self.verify(receipt,task)

    def test_other_receipts_need_external_bindings(self):
        candidate=dict(schema_version=1,task_id='task-001',candidate_sha=SHA,base_sha=SHA,tree_digest=DIGEST,imported=True,clean_product_tree=True,guard_receipt_digest=DIGEST,authority_digest=DIGEST,task_contract_digest=DIGEST)
        with self.assertRaises(c.ContractError):c.validate_record(candidate,'candidate_receipt',SCHEMAS)
        c.validate_record(candidate,'candidate_receipt',SCHEMAS,context=candidate.copy())
        candidate['imported']=False
        with self.assertRaises(c.ContractError):c.validate_record(candidate,'candidate_receipt',SCHEMAS,context=candidate.copy())
        report=dict(schema_version=1,task_id='task-001',task_contract_digest=DIGEST,candidate_sha=SHA,verdict='PASS',coverage=['quality'],findings=[],quality_flag_dispositions=[],unresolved_questions=[])
        c.validate_record(report,'review_report',SCHEMAS,context=report.copy())
        for field,value in [('unresolved_questions',['unknown blocking obligation']),('findings',[dict(id='x',severity='BLOCKING',classification='UNKNOWN',citation='source',claim='unknown cause',competing_hypotheses=['app','driver'],discriminating_check='capture exception')]),('quality_flag_dispositions',[dict(id='x',disposition='AMENDMENT_REQUIRED',evidence_digest=DIGEST)])]:
            bad=copy.deepcopy(report);bad[field]=value
            with self.assertRaises(c.ContractError):c.validate_record(bad,'review_report',SCHEMAS,context=report)


class ProvenanceFixture:
    """Real independent Git sources; externally approved pins are fixture-only."""
    def __init__(self, parent):
        self.candidate=Path(parent)/'candidate';self.controller=Path(parent)/'controller'
        for root in (self.candidate,self.controller):
            subprocess.run(['git','clone','-q','--no-hardlinks','--no-checkout',str(REPO_ROOT),str(root)],check=True)
            self.git(root,'checkout','-q','454f1597a74de2703064487518bdbe7a12731bbc')
            self.git(root,'remote','set-url','origin','https://github.com/'+c.REPOSITORY_IDENTITY+'.git')
        for name in ('PROTOCOL_SCHEMAS.json','V2_CONTRACT.json'):
            relative=c.AUTOMATION_DIR+'/'+name
            (self.candidate/relative).write_bytes((REPO_ROOT/relative).read_bytes())
        (self.candidate/'scripts/model_orchestrator/contracts.py').write_bytes((REPO_ROOT/'scripts/model_orchestrator/contracts.py').read_bytes())
        self.release=self.commit(self.candidate,'fixture: R3 contract materialization')
        self.git(self.controller,'fetch','-q',str(self.candidate),self.release)
        self.git(self.controller,'checkout','-q',self.release)
        self.ownership={case['id']:case['phase'] for case in json.loads((REPO_ROOT/c.CHECKS_PATH).read_text())['acceptance_cases']}
        authority=c.load_authority_manifest(self.candidate,release_oid=self.release,base_oid=c.TRUSTED_DESIGN_BASE,purpose='BUILD_AUTHORIZED_DISABLED',sandbox_digest=DIGEST,contract_versions=dict(project_schema=7,recovery_schema=1,ipc_protocol=1))
        self.manifest_digest=c.canonical_digest(authority['manifest'])
        self.builds={};self.certification=None;self.adoption=None
        for phase in (*c.IMPLEMENTATION_PHASES,'M5-full'):
            index=5 if phase=='M5-full' else int(phase[1]);completed=list(c.IMPLEMENTATION_PHASES[:6 if phase=='M5-full' else index])
            common=dict(schema_version=1,repository=c.REPOSITORY_IDENTITY,architecture_spec_sha=c.ARCHITECTURE_SPEC_SHA,base_sha=c.TRUSTED_DESIGN_BASE,release_sha=self.release,candidate_branch='control/fixture',authorization_id='auth-'+phase,task_id='task-'+phase,sequence=1,nonce='nonce-'+phase,sandbox_digest=DIGEST,authority_manifest_digest=self.manifest_digest,checks_digest=c.canonical_digest(json.loads((REPO_ROOT/c.CHECKS_PATH).read_text())))
            build=dict(**common,purpose='DISABLED_BUILD_ONLY',authorized_phases=['M'+str(index)],completed_phases=completed,phase_evidence=[dict(phase=p,case_ids=[case for case,owner in self.ownership.items() if owner==p],receipt_id='phase-'+p,receipt_digest=DIGEST) for p in completed],allowed_paths=['scripts/model_orchestrator/contracts.py'],required_gates=['M0-contracts','execution-infra'])
            build['required_case_ids']=[case for case,owner in self.ownership.items() if owner in build['authorized_phases']]
            build['scope_digest']=c.canonical_digest({key:build[key] for key in ('authorized_phases','allowed_paths','required_gates','required_case_ids')});self.builds[phase]=build
            self.write('build-'+phase,build)
            if phase=='M5-full':
                def evidence(name):return dict(evidence_id=name,digest=c.canonical_digest(name),release_sha=self.release,result='PASS')
                self.certification=dict(**common,purpose='PRE_ADOPTION_CERTIFICATION',completed_phases=list(c.IMPLEMENTATION_PHASES),case_receipts=[dict(case_id=case,phase=owner,receipt_id='case-'+case,receipt_digest=c.canonical_digest(case),result='PASS') for case,owner in self.ownership.items()],live_certification=evidence('live'),independent_review=evidence('review'),qualified_models=dict(**evidence('models'),required_models_available=True),blocking_limitations=[])
                self.write('certification',self.certification)
                self.adoption=dict(**common,purpose='OPERATOR_OPERATIONAL_ADOPTION',authority_kind='V2_FROZEN_CONTROL_RELEASE',certified_release_sha=self.release,certification_digest=c.canonical_digest(self.certification),parent_sha=c.TRUSTED_DESIGN_BASE,plan_digest=hashlib.sha256(subprocess.check_output(['git','show',c.TRUSTED_DESIGN_BASE+':docs/execution/PLAN.json'],cwd=self.candidate)).hexdigest(),state_digest=hashlib.sha256(subprocess.check_output(['git','show',c.TRUSTED_DESIGN_BASE+':docs/execution/STATE.json'],cwd=self.candidate)).hexdigest())
                self.adoption.update({name+'_digest':self.certification[name]['digest'] for name in ('live_certification','independent_review','qualified_models')});self.write('adoption',self.adoption)
        self.source=self.commit(self.controller,'fixture: operator-pinned controller receipts')
        self.authorities={phase:c.load_release_authority(self.candidate,self.controller,bootstrap=self.bootstrap(phase)) for phase in c.IMPLEMENTATION_PHASES}
        self.certified=c.load_release_authority(self.candidate,self.controller,bootstrap=self.bootstrap('M5-full',certification=True))
        self.adopted=c.load_release_authority(self.candidate,self.controller,bootstrap=self.bootstrap('M5-full',certification=True,adoption=True))
    def git(self,root,*args):return subprocess.check_output(['git',*args],cwd=root,stderr=subprocess.DEVNULL).decode().strip()
    def commit(self,root,message):
        self.git(root,'add','-A');self.git(root,'-c','user.name=Contract Fixture','-c','user.email=fixture@example.invalid','commit','-qm',message);return self.git(root,'rev-parse','HEAD')
    def write(self,name,record):
        path=self.controller/('controller/'+name+'.json');path.parent.mkdir(exist_ok=True);path.write_text(json.dumps(record)+'\n')
    def pin(self,name,record):return c.RecordPin('controller/'+name+'.json',c.canonical_digest(record))
    def bootstrap(self,phase='M0',certification=False,adoption=False):
        build=self.builds[phase]
        return c.ControllerBootstrap(source_sha=self.source,anchor_sha=c.TRUSTED_DESIGN_BASE,base_sha=c.TRUSTED_DESIGN_BASE,release_sha=self.release,candidate_branch=build['candidate_branch'],authorization_id=build['authorization_id'],task_id=build['task_id'],sequence=1,nonce=build['nonce'],sandbox_digest=DIGEST,build=self.pin('build-'+phase,build),certification=self.pin('certification',self.certification) if certification else None,adoption=self.pin('adoption',self.adoption) if adoption else None)
    def malformed(self,name,record,bootstrap):
        self.write(name,record);sha=self.commit(self.controller,'fixture: malformed approved record')
        pin=self.pin(name,record)
        field='build' if name.startswith('build-') else name
        try:return c.load_release_authority(self.candidate,self.controller,bootstrap=replace(bootstrap,source_sha=sha,**{field:pin}))
        finally:self.git(self.controller,'checkout','-q',self.source)


class LifecycleMatrixTests(unittest.TestCase):
    """CP33: deterministic lifecycle from independently Git-pinned provenance."""
    @classmethod
    def setUpClass(cls):
        cls.temp=tempfile.TemporaryDirectory();cls.addClassCleanup(cls.temp.cleanup);cls.fixture=ProvenanceFixture(cls.temp.name)
    def build(self,phase='M0'):
        authority=self.fixture.authorities[phase];build=json.loads(authority.payload_json)['build']
        record=c.proposal_record(build_authorization_digest=c.canonical_digest(build),implementation_phase=phase,completed_phases=copy.deepcopy(build['completed_phases']),lifecycle_state='IMPLEMENTATION_'+phase)
        return record,authority
    def certified(self,adopted=False):
        f=self.fixture;authority=f.adopted if adopted else f.certified
        record=c.proposal_record(build_authorization_digest=c.canonical_digest(f.builds['M5-full']),implementation_phase='M5',completed_phases=list(c.IMPLEMENTATION_PHASES),acceptance_cases_passed=list(c.ACCEPTANCE_CASE_IDS),certification='CANDIDATE',certified_release_sha=f.release,lifecycle_state='CERTIFICATION_CANDIDATE')
        record.update({name+'_digest':f.certification[name]['digest'] for name in ('live_certification','independent_review','qualified_models')})
        if adopted:
            record.update(operational_adoption='ADOPTED',authority_kind='V2_FROZEN_CONTROL_RELEASE',adopted_release_sha=f.release,certification='CERTIFIED_ACTIVE',full_auto_eligible=True,lifecycle_state='CERTIFIED_ACTIVE')
            record.update({key:f.adoption[key] for key in ('parent_sha','plan_digest','state_digest')})
        return record,authority
    def test_no_design_or_build_permission_means_no_implementation(self):
        for frozen in (False,True):
            with self.assertRaises(c.ContractError):c.validate_adoption_record(c.proposal_record(architecture_frozen=frozen,implementation_phase='M1'),SCHEMAS)
        c.validate_adoption_record(c.proposal_record(),SCHEMAS)
        self.assertFalse(c.full_auto_eligible(c.proposal_record(),schemas=SCHEMAS))
    def test_all_phases_allowed_disabled_without_adoption(self):
        for phase in c.IMPLEMENTATION_PHASES:
            record,authority=self.build(phase);c.validate_adoption_record(record,SCHEMAS,external=authority)
            self.assertFalse(c.full_auto_eligible(record,schemas=SCHEMAS,external=authority))
            with self.assertRaises(c.ContractError):c.validate_adoption_record(record,SCHEMAS)
        record,authority=self.build();record.update(implementation_phase='M1',lifecycle_state='IMPLEMENTATION_M1')
        with self.assertRaises(c.ContractError):c.validate_adoption_record(record,SCHEMAS,external=authority)
    def test_pre_adoption_certification_not_active(self):
        record,authority=self.certified();c.validate_adoption_record(record,SCHEMAS,external=authority)
        self.assertFalse(c.full_auto_eligible(record,schemas=SCHEMAS,external=authority))
        record.update(adoption_requested=True,lifecycle_state='OPERATIONAL_ADOPTION_PENDING');c.validate_adoption_record(record,SCHEMAS,external=authority)
        record.update(certification='CERTIFIED_ACTIVE',lifecycle_state='CERTIFIED_ACTIVE',full_auto_eligible=True)
        with self.assertRaises(c.ContractError):c.validate_adoption_record(record,SCHEMAS,external=authority)
    def test_operational_adoption_requires_complete_external_evidence(self):
        record,authority=self.certified(True);c.validate_adoption_record(record,SCHEMAS,external=authority)
        self.assertTrue(c.full_auto_eligible(record,schemas=SCHEMAS,external=authority))
        self.assertFalse(c.full_auto_eligible(record))
        for key,value in [('completed_phases',['M0']),('acceptance_cases_passed',['CP01']),('live_certification_digest',None),('qualified_models_digest',None),('blocking_limitations',['missing isolation']),('adopted_release_sha','c'*40),('architecture_spec_sha',c.AUDITED_PROTOTYPE_SHA),('changed_paths',['docs/execution/STATE.json'])]:
            with self.subTest(key=key):
                bad=copy.deepcopy(record);bad[key]=value
                with self.assertRaises(c.ContractError):c.validate_adoption_record(bad,SCHEMAS,external=authority)
        with self.assertRaises(c.ContractError):c.validate_adoption_record(record,SCHEMAS,external=record)
        raw=json.loads(authority.payload_json)
        raw.update(required_models_available=True,live_certification_result='PASS',independent_review_result='PASS',authority_binding_valid=True,case_results=dict.fromkeys(c.ACCEPTANCE_CASE_IDS,'PASS'),authorized_phases=list(c.IMPLEMENTATION_PHASES))
        with self.assertRaises(c.ContractError):c.validate_adoption_record(record,SCHEMAS,external=raw)
        self.assertFalse(c.full_auto_eligible(record,schemas=SCHEMAS,external=raw))
    def test_v1_and_wrong_control_adoption_scope_refused(self):
        for kind,ref in [('V1_PROTOTYPE',None),('V2_FROZEN_CONTROL_RELEASE',c.V1_BRANCH),('V2_FROZEN_CONTROL_RELEASE',c.AUDITED_PROTOTYPE_SHA)]:
            with self.assertRaises(c.ContractError):c.validate_authority_source(kind,ref)
        for path in ('docs/execution/STATE.json','docs/execution/PLAN.json','apps/or_app/lib/main.dart','crates/or_core/src/lib.rs','unknown.py'):
            self.assertEqual(c.validate_adoption_diff([path]),[path])
        self.assertEqual(c.validate_adoption_diff(['scripts/model_orchestrator/contracts.py']),[])
    def test_lifecycle_negative_matrix_and_every_derived_stage(self):
        record,authority=self.build()
        for key,value in [('lifecycle_state','NOT_A_STATE'),('lifecycle_state','CERTIFIED_ACTIVE'),('lifecycle_state','IMPLEMENTATION_M4'),('architecture_frozen',False),('build_authorization_digest',None),('completed_phases',['M1']),('completed_phases',['M1','M0']),('certification','CANDIDATE')]:
            with self.subTest(key=key,value=value):
                bad=copy.deepcopy(record);bad[key]=value
                with self.assertRaises(c.ContractError):c.validate_adoption_record(bad,SCHEMAS,external=authority)
        proposal=c.proposal_record(amendment_proposed=False,lifecycle_state='ARCHITECTURE_FROZEN');c.validate_adoption_record(proposal,SCHEMAS)
        prior,prior_authority=self.build('M1');prior.update(implementation_phase='NONE',lifecycle_state='BUILD_AUTHORIZED_DISABLED')
        with self.assertRaises(c.ContractError):c.validate_adoption_record(prior,SCHEMAS,external=prior_authority)
        record.update(implementation_phase='NONE',lifecycle_state='BUILD_AUTHORIZED_DISABLED');c.validate_adoption_record(record,SCHEMAS,external=authority)
        record,authority=self.certified(True);record.update(certification='CANDIDATE',full_auto_eligible=False,lifecycle_state='OPERATIONALLY_ADOPTED');c.validate_adoption_record(record,SCHEMAS,external=authority)
    def test_acceptance_progress_negative_matrix(self):
        for phase,cases in [('M0',['CP44']),('M1',list(c.ACCEPTANCE_CASE_IDS)),('M1',['CP19']),('M0',['CP01','CP01']),('M0',['UNKNOWN'])]:
            record,authority=self.build(phase);record['acceptance_cases_passed']=cases
            with self.subTest(phase=phase,cases=cases):
                with self.assertRaises(c.ContractError):c.validate_adoption_record(record,SCHEMAS,external=authority)
        record,authority=self.certified();record['acceptance_cases_passed'].remove('CP44')
        with self.assertRaises(c.ContractError):c.validate_adoption_record(record,SCHEMAS,external=authority)
        record,authority=self.build('M3');record['completed_phases']=['M0','M1'];record['acceptance_cases_passed']=['CP19']
        with self.assertRaises(c.ContractError):c.validate_adoption_record(record,SCHEMAS,external=authority)
        record,authority=self.build();record['acceptance_cases_passed']=['CP01','CP33'];c.validate_adoption_record(record,SCHEMAS,external=authority)
    def test_positive_authority_source_and_all_negative_references(self):
        f=self.fixture;c.validate_authority_source('V2_FROZEN_CONTROL_RELEASE',f.release,external=f.adopted)
        for ref in ('arbitrary','control/branch','refs/heads/main','x'*40,c.V1_BRANCH,c.AUDITED_PROTOTYPE_SHA,c.ARCHITECTURE_SPEC_SHA,c.TRUSTED_DESIGN_BASE,'f'*40):
            with self.subTest(reference=ref):
                with self.assertRaises(c.ContractError):c.validate_authority_source('V2_FROZEN_CONTROL_RELEASE',ref,external=f.adopted)
        for authority in (None,{},f.certified,f.authorities['M0']):
            with self.assertRaises(c.ContractError):c.validate_authority_source('V2_FROZEN_CONTROL_RELEASE',f.release,external=authority)
    def test_provenance_pin_binding_negative_matrix(self):
        f=self.fixture;bootstrap=f.bootstrap()
        changes=[dict(architecture_spec_sha=c.AUDITED_PROTOTYPE_SHA),dict(repository='wrong/repository'),dict(base_sha=c.ARCHITECTURE_SPEC_SHA),dict(release_sha=c.TRUSTED_DESIGN_BASE),dict(sequence=2),dict(nonce='old'),dict(candidate_branch='control/other'),dict(authorized_phases=['M1']),dict(scope_digest='b'*64),dict(allowed_paths=['docs/execution/STATE.json'])]
        for change in changes:
            with self.subTest(change=change):
                bad=dict(f.builds['M0'],**change)
                with self.assertRaises(c.ContractError):f.malformed('build-M0',bad,bootstrap)
        bad=copy.deepcopy(f.builds['M0']);bad['required_case_ids']=['CP01'];bad['scope_digest']=c.canonical_digest({key:bad[key] for key in ('authorized_phases','allowed_paths','required_gates','required_case_ids')})
        with self.assertRaises(c.ContractError):f.malformed('build-M0',bad,bootstrap)
        with self.assertRaises(c.ContractError):c.load_release_authority(f.candidate,f.candidate,bootstrap=bootstrap)
        with self.assertRaises(c.ContractError):c.load_release_authority(f.candidate,f.controller,bootstrap={})
        with self.assertRaises(c.ContractError):c.ValidatedReleaseAuthority({},f.candidate,f.controller,f.source)
        bad=replace(bootstrap,build=c.RecordPin(bootstrap.build.path,'b'*64))
        with self.assertRaises(c.ContractError):c.load_release_authority(f.candidate,f.controller,bootstrap=bad)
        with self.assertRaises(c.ContractError):c.validate_record(f.builds['M0'],'build_authorization',SCHEMAS,context=f.builds['M0'])
        record,authority=self.build()
        raw=dict(architecture_spec_sha=c.ARCHITECTURE_SPEC_SHA,build_authorization_digest=record['build_authorization_digest'],authorized_phases=['M0'])
        for external in (raw,f.builds['M0'],record):
            with self.assertRaises(c.ContractError):c.validate_adoption_record(record,SCHEMAS,external=external)

    def test_post_load_repository_identity_and_hidden_drift_refused(self):
        f=self.fixture;record,authority=self.build()
        f.git(f.controller,'remote','set-url','origin','https://github.com/wrong/repository.git')
        try:
            with self.assertRaises(c.ContractError):c.validate_adoption_record(record,SCHEMAS,external=authority)
            with self.assertRaises(c.ContractError):c.load_release_authority(f.candidate,f.controller,bootstrap=f.bootstrap())
        finally:f.git(f.controller,'remote','set-url','origin','https://github.com/'+c.REPOSITORY_IDENTITY+'.git')
        path=f.candidate/'AGENTS.md';original=path.read_bytes()
        f.git(f.candidate,'update-index','--assume-unchanged','AGENTS.md');path.write_text('hidden dirty candidate')
        try:
            with self.assertRaises(c.ContractError):c.validate_adoption_record(record,SCHEMAS,external=authority)
        finally:
            path.write_bytes(original);f.git(f.candidate,'update-index','--no-assume-unchanged','AGENTS.md')
    def test_certification_and_operator_pin_negative_matrix(self):
        f=self.fixture;bootstrap=f.bootstrap('M5-full',certification=True,adoption=True)
        for field,value in [('certified_release_sha',c.ARCHITECTURE_SPEC_SHA),('certification_digest','b'*64),('independent_review_digest','b'*64),('authority_manifest_digest','b'*64),('parent_sha',c.ARCHITECTURE_SPEC_SHA),('state_digest','b'*64)]:
            with self.subTest(field=field):
                with self.assertRaises(c.ContractError):f.malformed('adoption',dict(f.adoption,**{field:value}),bootstrap)
        changes=[('completed_phases',['M0']),('case_receipts',f.certification['case_receipts'][:-1]),('blocking_limitations',['isolation unavailable'])]
        for field,value in changes:
            with self.subTest(field=field):
                with self.assertRaises(c.ContractError):f.malformed('certification',dict(f.certification,**{field:value}),bootstrap)
        for name in ('live_certification','independent_review','qualified_models'):
            bad=copy.deepcopy(f.certification);bad[name]['result']='FAIL'
            with self.assertRaises(c.ContractError):f.malformed('certification',bad,bootstrap)
        bad=copy.deepcopy(f.certification);bad['qualified_models']['required_models_available']=False
        with self.assertRaises(c.ContractError):f.malformed('certification',bad,bootstrap)
        bad=copy.deepcopy(f.certification);bad['case_receipts'][0]['phase']='M5'
        with self.assertRaises(c.ContractError):f.malformed('certification',bad,bootstrap)
        bad=copy.deepcopy(f.certification);bad['case_receipts'][1]=copy.deepcopy(bad['case_receipts'][0])
        with self.assertRaises(c.ContractError):f.malformed('certification',bad,bootstrap)
    def test_stale_controller_snapshot_refused(self):
        f=self.fixture;(f.controller/'controller/new-sequence.txt').write_text('advanced controller sequence')
        f.commit(f.controller,'fixture: later controller approval')
        try:
            record,authority=self.build()
            with self.assertRaises(c.ContractError):c.validate_adoption_record(record,SCHEMAS,external=authority)
            with self.assertRaises(c.ContractError):c.load_release_authority(f.candidate,f.controller,bootstrap=f.bootstrap())
        finally:f.git(f.controller,'checkout','-q',f.source)


class GitFixture:
    def __init__(self,root):
        self.root=root.resolve();self.root.mkdir()
        self.git('init','-q');self.git('config','user.email','fixture@example.invalid');self.git('config','user.name','Fixture');self.git('remote','add','origin','https://github.com/'+c.REPOSITORY_IDENTITY+'.git')
        for name in c.REQUIRED_AUTHORITY_PATHS:
            path=self.root/name;path.parent.mkdir(parents=True,exist_ok=True);path.write_text('fixture authority\n')
        for name in ('docs/execution/PLAN.json','docs/execution/STATE.json','docs/execution/EVIDENCE_POLICY.json','docs/execution/evidence/7A.json'):
            path=self.root/name;path.parent.mkdir(parents=True,exist_ok=True);path.write_bytes((REPO_ROOT/name).read_bytes())
        for name,constant,version in [('crates/or_core/src/project_document.rs','CURRENT_PROJECT_SCHEMA_VERSION',7),('crates/or_core/src/project_recovery.rs','CURRENT_RECOVERY_SCHEMA_VERSION',1),('crates/or_ipc/src/protocol.rs','OR_LOCAL_IPC_PROTOCOL_VERSION',1)]:
            (self.root/name).write_text(f'pub const {constant}: u32 = {version};\n')
        for source in (REPO_ROOT/'docs/execution/phases').glob('*.md'):
            path=self.root/'docs/execution/phases'/source.name;path.parent.mkdir(parents=True,exist_ok=True);path.write_bytes(source.read_bytes())
        self.base=self.commit('base');self.anchor=self.base
    def git(self,*args):
        return subprocess.check_output(['git',*args],cwd=self.root,stderr=subprocess.DEVNULL).decode().strip()
    def commit(self,message):
        self.git('add','-A');self.git('commit','-qm',message);return self.git('rev-parse','HEAD')
    def load(self,**changes):
        args=dict(release_oid=self.git('rev-parse','HEAD'),base_oid=self.base,anchor_oid=self.anchor,purpose='BUILD_AUTHORIZED_DISABLED',sandbox_digest=DIGEST,contract_versions=dict(project_schema=7,recovery_schema=1,ipc_protocol=1));args.update(changes)
        return c.load_authority_manifest(self.root,**args)


class GitAuthorityMatrixTests(unittest.TestCase):
    """CP01/02/04: real object provenance, full closure and resume drift refusal."""
    def setUp(self):
        self.temp=tempfile.TemporaryDirectory();self.addCleanup(self.temp.cleanup);self.fixture=GitFixture(Path(self.temp.name)/'repo')
    def test_positive_full_git_authority(self):
        authority=self.fixture.load();c.verify_authority_binding(authority,copy.deepcopy(authority))
        self.assertEqual(authority['manifest'][0]['mode'],'100644')
        self.assertTrue(any(e['path']=='opencode.json' and e['mode']=='absent' for e in authority['manifest']))
    def test_every_manifest_omission_refused(self):
        f=self.fixture;full=c.build_manifest(f.root,f.base);paths=[e['path'] for e in full]
        for missing in paths:
            with self.subTest(missing=missing):
                with self.assertRaises(c.ContractError):c.build_manifest(f.root,f.base,[p for p in paths if p!=missing])
        self.assertEqual(c.frozen_source_closure_violations(['docs/execution/phases/PHASE_9.md','docs/execution/evidence/7A.json','docs/execution/automation/README.md','docs/TECHNICAL_PLAN.md']),['docs/TECHNICAL_PLAN.md','docs/execution/automation/README.md','docs/execution/evidence/7A.json','docs/execution/phases/PHASE_9.md'])
    def test_non_git_fake_oid_wrong_repo_blob_tag_tree(self):
        f=self.fixture
        with self.assertRaises(c.ContractError):c.load_authority_manifest(Path(self.temp.name),release_oid=SHA,base_oid=SHA,purpose='BUILD_AUTHORIZED_DISABLED',sandbox_digest=DIGEST,contract_versions={})
        for key in ('release_oid','base_oid'):
            for value in ('f'*40,f.git('rev-parse',f.base+':AGENTS.md'),f.git('rev-parse',f.base+'^{tree}')):
                with self.subTest(key=key,value=value):
                    with self.assertRaises(c.ContractError):f.load(**{key:value})
        f.git('tag','-a','tagged','-m','tag',f.base);tag=f.git('rev-parse','refs/tags/tagged')
        for key in ('release_oid','base_oid'):
            with self.assertRaises(c.ContractError):f.load(**{key:tag})
        f.git('remote','set-url','origin','https://github.com/wrong/repository.git')
        with self.assertRaises(c.ContractError):f.load()
    def test_foreign_commit_and_wrong_relationship(self):
        f=self.fixture;foreign=GitFixture(Path(self.temp.name)/'foreign');(foreign.root/'foreign.txt').write_text('foreign');oid=foreign.commit('foreign');f.git('fetch','-q',str(foreign.root),oid)
        with self.assertRaises(c.ContractError):f.load(release_oid=oid)
        (f.root/'product.txt').write_text('new product baseline');new=f.commit('new')
        with self.assertRaises(c.ContractError):f.load(release_oid=f.base,base_oid=new)
        f.git('checkout','-q',f.base)
        f.load(release_oid=f.base,base_oid=new,purpose='OPERATIONAL')
    def test_dirty_mode_untracked_and_assume_unchanged_refused(self):
        f=self.fixture;path=f.root/'AGENTS.md';original=path.read_bytes()
        path.write_text('dirty')
        with self.assertRaises(c.ContractError):f.load()
        f.git('update-index','--assume-unchanged','AGENTS.md')
        with self.assertRaises(c.ContractError):f.load()
        path.write_bytes(original);f.git('update-index','--no-assume-unchanged','AGENTS.md')
        path.chmod(0o755)
        with self.assertRaises(c.ContractError):f.load()
        path.chmod(0o644);(f.root/'unknown.txt').write_text('untracked')
        with self.assertRaises(c.ContractError):f.load()
    def test_configs_resume_and_full_digest_binding(self):
        f=self.fixture;before=f.load();(f.root/'opencode.json').write_text('{}')
        with self.assertRaises(c.ContractError):f.load()
        f.commit('changed config');after=f.load()
        with self.assertRaises(c.ContractError):c.verify_authority_binding(before,after)
        bad=copy.deepcopy(before);bad['manifest'].pop()
        with self.assertRaises(c.ContractError):c.verify_authority_binding(before,bad)
        bad['authority_digest']=c.canonical_digest({k:v for k,v in bad.items() if k!='authority_digest'})
        with self.assertRaises(c.ContractError):c.verify_authority_binding(before,bad)
    def test_base_without_future_m0_files_is_valid_build_input(self):
        f=self.fixture;path=f.root/c.PROTOCOL_SCHEMAS_PATH;data=path.read_bytes();path.unlink();old=f.commit('pre-m0');path.write_bytes(data);release=f.commit('m0');f.load(base_oid=old,release_oid=release)
    def test_symlink_missing_surface_and_version_mismatch(self):
        f=self.fixture
        with self.assertRaises(c.ContractError):f.load(contract_versions=dict(project_schema=6,recovery_schema=1,ipc_protocol=1))
        path=f.root/'AGENTS.md';path.unlink();path.symlink_to('docs/PRODUCT.md');oid=f.commit('symlink')
        with self.assertRaises(c.ContractError):f.load(release_oid=oid)
    def test_wrong_root_and_unsupported_git_indirection(self):
        f=self.fixture
        for root in (None,'relative',f.root/'docs'):
            with self.assertRaises(c.ContractError):c.resolve_authority_root(root) if root in (None,'relative') else c.load_authority_manifest(root,release_oid=f.base,base_oid=f.base,purpose='BUILD_AUTHORIZED_DISABLED',sandbox_digest=DIGEST,contract_versions={})
        f.git('replace',f.base,f.base)
        with self.assertRaises(c.ContractError):f.load()


class CompletionBoundaryTests(unittest.TestCase):
    """CP05: actual immutable evidence + ancestry, no legacy caller boolean."""
    def setUp(self):
        self.temp=tempfile.TemporaryDirectory();self.addCleanup(self.temp.cleanup);self.f=GitFixture(Path(self.temp.name)/'repo');self.path='docs/execution/evidence/7A.json';self.record=json.loads((self.f.root/self.path).read_text());(self.f.root/'adoption.txt').write_text('prepared adoption');self.adoption=self.f.commit('adoption')
    def validate(self,record,revision):
        return c.validate_completion_evidence(record,SCHEMAS,repo_root=self.f.root,revision=revision,evidence_path=self.path,adoption_sha=self.adoption,anchor_oid=self.f.anchor)
    def test_real_historic_record_valid_and_post_adoption_reuse_refused(self):
        self.validate(self.record,self.f.base)
        with self.assertRaises(c.ContractError):self.validate(self.record,self.adoption)
    def test_forged_legacy_record_bool_version_and_missing_receipt_refused(self):
        for change in ({'schema_version':True},{'legacy':True},{'checkpoint_id':'wrong'}):
            bad=dict(self.record,**change)
            with self.assertRaises(c.ContractError):self.validate(bad,self.f.base)
        with self.assertRaises(TypeError):c.validate_completion_evidence(self.record,SCHEMAS,control_release_active=False)
    def test_schema_two_historic_fixture_remains_valid(self):
        # Synthetic offline schema fixture, never a hosted/product PASS claim.
        import execution_evidence as evidence
        from test_execution_infra import valid_gate, contract_versions
        plan=json.loads((REPO_ROOT/'docs/execution/PLAN.json').read_text())
        checkpoint=next(item for item in plan['checkpoints'] if item['id']=='9B')
        policy=evidence.load_policy()
        gates=[valid_gate('repository_hygiene',SHA,101),valid_gate('platform_verification',SHA,102)]
        jobs={101:[],102:[]}
        for sources in evidence.required_class_sources(checkpoint,policy).values():
            for source in sources:
                run=101 if source['gate_id']=='repository_hygiene' else 102
                job=next((j for j in jobs[run] if j['name']==source['job_name']),None)
                if job is None:
                    job=dict(id=run*10+len(jobs[run]),name=source['job_name'],status='completed',conclusion='success',steps=[]);jobs[run].append(job)
                if not any(step['name']==source['step_name'] for step in job['steps']):
                    job['steps'].append(dict(name=source['step_name'],number=len(job['steps'])+1,status='completed',conclusion='success'))
        class Api:
            def get(self,path):return {'jobs':jobs[int(path.split('/actions/runs/')[1].split('/')[0])]}
        proofs=evidence.collect_evidence_class_proofs(Api(),policy,checkpoint,gates)
        record=evidence.build_evidence_record(checkpoint_id='9B',implementation_sha=SHA,implementation_subject='fixture: schema two',gates=gates,developer_preview={'required':False},contract_versions=contract_versions(),evidence_classes=proofs)
        (self.f.root/self.path).write_text(json.dumps(record));historical=self.f.commit('historical schema2 fixture')
        (self.f.root/'later-adoption.txt').write_text('later adoption');self.adoption=self.f.commit('later adoption')
        self.assertEqual(record['schema_version'],2)
        self.validate(record,historical)

    def test_actual_malformed_post_adoption_receipt_refused(self):
        bad=dict(self.record,schema_version=2,control_plane_receipt={'schema_version':1})
        (self.f.root/self.path).write_text(json.dumps(bad));oid=self.f.commit('bad receipt')
        with self.assertRaises(c.ContractError):self.validate(bad,oid)


class RepositoryConsistencyTests(unittest.TestCase):
    def test_required_authority_surfaces_exist_in_actual_repository(self):
        # Minimal fixtures must not invent files the real release does not have.
        tracked=set(subprocess.check_output(['git','ls-files','-z'],cwd=REPO_ROOT).decode().split('\0'))
        self.assertFalse(c.REQUIRED_AUTHORITY_PATHS-tracked)

    def test_corrected_documents_and_disabled_runtime_surface(self):
        c.validate_v2_contract_documents(REPO_ROOT)
        files={p.name for p in (REPO_ROOT/'scripts/model_orchestrator').glob('*.py')}
        self.assertEqual(files,{'contracts.py','__init__.py','store.py','sandbox.py'})
    def test_serialized_document_lifecycle_is_semantically_bound(self):
        import shutil
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory);shutil.copytree(REPO_ROOT/c.AUTOMATION_DIR,root/c.AUTOMATION_DIR)
            path=root/c.V2_CONTRACT_PATH;original=json.loads(path.read_text())
            for label in ('UNKNOWN','CERTIFIED_ACTIVE','IMPLEMENTATION_M4','BUILD_AUTHORIZED_DISABLED'):
                bad=copy.deepcopy(original);bad['adoption']['lifecycle_state']=label;path.write_text(json.dumps(bad))
                with self.assertRaises(c.ContractError):c.validate_v2_contract_documents(root)

    def test_invalid_lifecycle_and_unknown_nested_schema(self):
        bad=copy.deepcopy(SCHEMAS);bad['adoption_lifecycle']['transitions']['AMENDMENT_PROPOSED']=['OPERATIONALLY_ADOPTED']
        with self.assertRaises(c.ContractError):c.validate_protocol_schemas(bad)
        bad=copy.deepcopy(SCHEMAS);bad['records']['task_contract']['fields']['resource_limits']['fields']['cpu']['unexpected']=True
        with self.assertRaises(c.ContractError):c.validate_protocol_schemas(bad)

class ControlAmendmentMarkerTests(unittest.TestCase):
    """The schema-2 adoption marker is defined but never self-certifying."""

    @classmethod
    def setUpClass(cls) -> None:
        cls.schemas = c.load_protocol_schemas(REPO_ROOT)

    def _marker(self) -> dict:
        return {
            "schema_version": 2,
            "kind": "model-orchestrator-v2-adoption",
            "checkpoint_id": "9B",
            "parent_sha": "a" * 40,
            "architecture_spec_sha": c.ARCHITECTURE_SPEC_SHA,
            "authority_manifest_digest": "b" * 64,
            "changed_paths": ["AGENTS.md", "scripts/model_orchestrator/contracts.py"],
            "legacy_quality_amendment": {
                "marker_sha": "c" * 40,
                "prior_implementation_sha": c.PRIOR_IMPLEMENTATION_SHA,
                "prior_failed_run_id": c.PRIOR_FAILED_RUN_ID,
            },
            "state_digest": "d" * 64,
            "plan_digest": "e" * 64,
            "verified_contract_versions": {
                "project_schema": 7,
                "recovery_schema": 1,
                "ipc_protocol": 1,
            },
        }

    def test_valid_marker_passes(self) -> None:
        c.validate_control_amendment_marker(
            self._marker(), self.schemas, expected_checkpoint="9B"
        )

    def test_wrong_checkpoint_is_rejected(self) -> None:
        marker = self._marker()
        marker["checkpoint_id"] = "9C"
        with self.assertRaisesRegex(c.ContractError, "current NEXT"):
            c.validate_control_amendment_marker(
                marker, self.schemas, expected_checkpoint="9B"
            )

    def test_wrong_architecture_sha_is_rejected(self) -> None:
        marker = self._marker()
        marker["architecture_spec_sha"] = "0" * 40
        with self.assertRaisesRegex(c.ContractError, "architecture/control"):
            c.validate_control_amendment_marker(marker, self.schemas)

    def test_product_path_in_marker_is_rejected(self) -> None:
        marker = self._marker()
        marker["changed_paths"] = ["AGENTS.md", "apps/or_app/lib/main.dart"]
        with self.assertRaisesRegex(c.ContractError, "architecture/control"):
            c.validate_control_amendment_marker(marker, self.schemas)

    def test_marker_cannot_hold_its_own_sha(self) -> None:
        marker = self._marker()
        marker["marker_sha"] = "f" * 40
        with self.assertRaisesRegex(c.ContractError, "unknown or missing"):
            c.validate_control_amendment_marker(marker, self.schemas)

    def test_wrong_legacy_provenance_is_rejected(self) -> None:
        marker = self._marker()
        marker["legacy_quality_amendment"]["prior_failed_run_id"] = 1
        with self.assertRaisesRegex(c.ContractError, "legacy provenance"):
            c.validate_control_amendment_marker(marker, self.schemas)

    def test_bool_contract_version_is_rejected(self) -> None:
        marker = self._marker()
        marker["verified_contract_versions"]["project_schema"] = True
        with self.assertRaises(c.ContractError):
            c.validate_control_amendment_marker(marker, self.schemas)



def probe_repository_authority(revision):
    """Actual repository in an independent clone; no runtime/product execution."""
    c._authority_repo(REPO_ROOT,c.TRUSTED_DESIGN_BASE)
    origin=c._git(REPO_ROOT,'config','--local','--get','remote.origin.url').decode().strip()
    with tempfile.TemporaryDirectory(prefix='or-r3-authority-probe-') as directory:
        root=Path(directory)/'repo'
        subprocess.run(['git','clone','-q','--no-hardlinks','--no-checkout',str(REPO_ROOT),str(root)],check=True)
        def git(*args):return subprocess.check_output(['git',*args],cwd=root,stderr=subprocess.DEVNULL).decode().strip()
        git('remote','set-url','origin',origin)
        git('checkout','-q','HEAD' if revision=='staged' else revision)
        if revision=='staged':
            names=subprocess.check_output(['git','diff','--cached','--name-only','-z'],cwd=REPO_ROOT).decode().split('\0')[:-1]
            for name in names:
                path=root/name;path.parent.mkdir(parents=True,exist_ok=True)
                path.write_bytes(subprocess.check_output(['git','show',':'+name],cwd=REPO_ROOT));path.chmod((REPO_ROOT/name).stat().st_mode & 0o777)
            git('add','-A');git('-c','user.name=Contract Fixture','-c','user.email=fixture@example.invalid','commit','-qm','fixture: staged R3 repository probe')
            revision=git('rev-parse','HEAD')
        c._commit(root,c.ARCHITECTURE_SPEC_SHA);c._commit(root,revision)
        c._ancestor(root,c.ARCHITECTURE_SPEC_SHA,revision)
        def load():return c.load_authority_manifest(root,release_oid=revision,base_oid=c.TRUSTED_DESIGN_BASE,purpose='BUILD_AUTHORIZED_DISABLED',sandbox_digest=c.canonical_digest({'diagnostic':'M0 probe, no isolation certification'}),contract_versions=dict(project_schema=7,recovery_schema=1,ipc_protocol=1))
        def refuse(call):
            try:call()
            except c.ContractError:return
            raise AssertionError('authority probe accepted forbidden input')
        authority=load();paths=[e['path'] for e in authority['manifest']]
        absent=[e['path'] for e in authority['manifest'] if e['mode']=='absent']
        for name in c.CONFIG_PATHS:
            if not git('ls-tree',revision,'--',name):assert name in absent
        path=root/'AGENTS.md';original=path.read_bytes();mode=path.stat().st_mode & 0o777
        try:
            path.write_text('dirty probe');refuse(load);path.write_bytes(original)
            untracked=root/'.authority-probe-untracked';untracked.write_text('untracked probe')
            refuse(load);untracked.unlink()
            path.chmod(mode ^ 0o111);refuse(load);path.chmod(mode)
            for omitted in paths:
                refuse(lambda:c.build_manifest(root,revision,[p for p in paths if p!=omitted]))
            git('remote','set-url','origin','https://github.com/foreign/repository.git');refuse(load)
        finally:
            path.write_bytes(original);path.chmod(mode)
        result=dict(repository=c.REPOSITORY_IDENTITY,anchor=c.TRUSTED_DESIGN_BASE,architecture_sha=c.ARCHITECTURE_SPEC_SHA,release_sha=revision,base_sha=c.TRUSTED_DESIGN_BASE,manifest_count=len(paths),optional_absent_configs=absent,results=dict(clean='PASS',dirty_refusal='PASS',untracked_refusal='PASS',executable_mode_refusal='PASS',every_manifest_omission_refusal='PASS',foreign_repository_refusal='PASS'),boundary='M0 headless Git contract diagnostic; no runtime certification')
        print(json.dumps(result,indent=2));return result


if __name__=='__main__':
    if len(sys.argv)==3 and sys.argv[1]=='--probe-repository':probe_repository_authority(sys.argv[2])
    else:unittest.main()
