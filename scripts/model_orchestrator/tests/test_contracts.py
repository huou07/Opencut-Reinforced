#!/usr/bin/env python3
"""M0-R2 semantic matrices. Real Git fixtures; no runtime/model/product launch."""
from __future__ import annotations
import copy
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


class LifecycleMatrixTests(unittest.TestCase):
    """CP33: build permission and runtime authority never share ADOPTED."""
    def build(self,phase='M0'):
        record=c.proposal_record(build_authorization_digest=DIGEST,implementation_phase=phase,completed_phases=list(c.IMPLEMENTATION_PHASES[:int(phase[1])]))
        external=dict(architecture_spec_sha=c.ARCHITECTURE_SPEC_SHA,build_authorization_digest=DIGEST,authorized_phases=list(c.IMPLEMENTATION_PHASES))
        return record,external

    def certified(self):
        record,external=self.build('M5');record.update(completed_phases=list(c.IMPLEMENTATION_PHASES),acceptance_cases_passed=list(c.ACCEPTANCE_CASE_IDS),certification='CANDIDATE',certified_release_sha=SHA,live_certification_digest=DIGEST,independent_review_digest=DIGEST,qualified_models_digest=DIGEST)
        external.update({k:copy.deepcopy(record[k]) for k in ('completed_phases','acceptance_cases_passed','certified_release_sha','live_certification_digest','independent_review_digest','qualified_models_digest','blocking_limitations')})
        external.update(live_certification_result='PASS',independent_review_result='PASS',required_models_available=True,case_results=dict.fromkeys(c.ACCEPTANCE_CASE_IDS,'PASS'))
        return record,external

    def test_no_design_or_build_permission_means_no_implementation(self):
        for frozen in (False,True):
            record=c.proposal_record(architecture_frozen=frozen,implementation_phase='M1')
            with self.assertRaises(c.ContractError):c.validate_adoption_record(record,SCHEMAS)
        c.validate_adoption_record(c.proposal_record(),SCHEMAS)

    def test_all_phases_allowed_disabled_without_adoption(self):
        for phase in c.IMPLEMENTATION_PHASES:
            with self.subTest(phase=phase):
                record,external=self.build(phase);c.validate_adoption_record(record,SCHEMAS,external=external)
                self.assertFalse(c.full_auto_eligible(record,schemas=SCHEMAS,external=external))
                with self.assertRaises(c.ContractError):c.validate_adoption_record(record,SCHEMAS)
        record,external=self.build('M1');external['authorized_phases']=['M0']
        with self.assertRaises(c.ContractError):c.validate_adoption_record(record,SCHEMAS,external=external)

    def test_pre_adoption_certification_not_active(self):
        record,external=self.certified();c.validate_adoption_record(record,SCHEMAS,external=external)
        self.assertFalse(c.full_auto_eligible(record,schemas=SCHEMAS,external=external))
        record['full_auto_eligible']=True
        with self.assertRaises(c.ContractError):c.validate_adoption_record(record,SCHEMAS,external=external)

    def test_operational_adoption_requires_complete_external_evidence(self):
        record,external=self.certified();record.update(operational_adoption='ADOPTED',authority_kind='V2_FROZEN_CONTROL_RELEASE',adopted_release_sha=SHA,parent_sha='b'*40,plan_digest=DIGEST,state_digest=DIGEST,certification='CERTIFIED_ACTIVE',full_auto_eligible=True)
        external.update({k:record[k] for k in ('adopted_release_sha','parent_sha','plan_digest','state_digest')})
        external['authority_binding_valid']=True
        c.validate_adoption_record(record,SCHEMAS,external=external)
        self.assertTrue(c.full_auto_eligible(record,schemas=SCHEMAS,external=external))
        self.assertFalse(c.full_auto_eligible(record))
        for key,value in [('completed_phases',['M0']),('acceptance_cases_passed',['CP01']),('live_certification_digest',None),('qualified_models_digest',None),('blocking_limitations',['missing isolation']),('adopted_release_sha','c'*40),('architecture_spec_sha',c.AUDITED_PROTOTYPE_SHA),('changed_paths',['docs/execution/STATE.json'])]:
            with self.subTest(key=key):
                bad=copy.deepcopy(record);bad[key]=value
                with self.assertRaises(c.ContractError):c.validate_adoption_record(bad,SCHEMAS,external=external)
        self.assertFalse(c.full_auto_eligible(record,schemas=SCHEMAS,external={}))
        for key,value in [('required_models_available',False),('live_certification_result','SKIPPED'),('independent_review_result','UNKNOWN'),('case_results',{'CP01':'PASS'}),('authority_binding_valid',False),('authority_binding_valid',1)]:
            bad=copy.deepcopy(external);bad[key]=value
            self.assertFalse(c.full_auto_eligible(record,schemas=SCHEMAS,external=bad))

    def test_v1_and_wrong_control_adoption_scope_refused(self):
        for kind,ref in [('V1_PROTOTYPE',None),('V2_FROZEN_CONTROL_RELEASE',c.V1_BRANCH),('V2_FROZEN_CONTROL_RELEASE',c.AUDITED_PROTOTYPE_SHA)]:
            with self.assertRaises(c.ContractError):c.validate_authority_source(kind,ref)
        for path in ('docs/execution/STATE.json','docs/execution/PLAN.json','apps/or_app/lib/main.dart','crates/or_core/src/lib.rs','unknown.py'):
            self.assertEqual(c.validate_adoption_diff([path]),[path])
        self.assertEqual(c.validate_adoption_diff(['scripts/model_orchestrator/contracts.py']),[])


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
        self.assertEqual(files,{'contracts.py','__init__.py'})
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



if __name__=='__main__':unittest.main()
