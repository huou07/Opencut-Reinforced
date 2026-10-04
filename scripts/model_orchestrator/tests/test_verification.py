#!/usr/bin/env python3
"""M2 CP12–18/30: real Git; opt-in prepared rootless verifier acceptance.

OR_M2_LIVE=1 enables actual Linux containers, never mocks. OR_M2_IMAGE pins the
already prepared image. OR_M2_OUTPUT names a NEW controller evidence directory.
No native product, tool installation, promotion, review or adoption is invoked.
"""
from __future__ import annotations

import copy
import hashlib
import json
import multiprocessing
import os
from pathlib import Path
import shutil
import signal
import subprocess
import sys
import tempfile
import time
import unittest
import uuid

ROOT=Path(__file__).resolve().parents[3]
sys.path.insert(0,str(ROOT/'scripts'))
from model_orchestrator import contracts as c, sandbox as b, store as s, workspace as w, guards as g, verification as v
from model_orchestrator.tests.test_contracts import valid_task

COMBINED_BASE='8ec46c069f0f7946be29716eccb3586676217c28'
IMAGE=os.environ.get('OR_M2_IMAGE','sha256:ab8e1e306576feb6fd8217aaf519ac3e96a76028c7bc8936fe4a4d74342c5bcd')
LIVE=os.environ.get('OR_M2_LIVE')=='1'
VALUE='scripts/model_orchestrator/guards.py'
ALLOWED_COPY='scripts/model_orchestrator/verification.py'
FLOOR_PATH='scripts/model_orchestrator/tests/test_verification.py'
CASE_PATH='scripts/model_orchestrator/tests/test_contracts.py'
CASE_SYMBOL='test_required_authority_surfaces_exist_in_actual_repository'


def git(root,*args):
    return subprocess.check_output(['git','-c','core.hooksPath='+os.devnull,'-c','user.name=M2 Fixture',
        '-c','user.email=fixture@example.invalid',*args],cwd=root,stderr=subprocess.DEVNULL).decode().strip()


def commit(root):
    git(root,'add','-A');git(root,'commit','-qm','fixture: candidate change');return git(root,'rev-parse','HEAD')


class ExactFixture:
    """Independent exact combined/release trees, externally pinned inert records."""
    def __init__(self,parent,script=None,*,timeout=10,classes=('UNIT',),metrics=False,case_ids=('case1',),hosted=False):
        self.parent=Path(parent).resolve();self.parent.mkdir(parents=True,exist_ok=True)
        self.candidate=self.parent/'authority';self.controller=self.parent/'controller'
        self.release=git(ROOT,'rev-parse','HEAD')
        for root in (self.candidate,self.controller):
            subprocess.run(['git','clone','-q','--no-hardlinks','--no-checkout',str(ROOT),str(root)],check=True)
            git(root,'checkout','-q',self.release);git(root,'remote','set-url','origin','https://github.com/'+c.REPOSITORY_IDENTITY+'.git')
        self.sandbox_digest=c.canonical_digest({'fixture_image':IMAGE,'boundary':'disabled M2 rootless checks, no adoption'})
        manifest=c.load_authority_manifest(self.candidate,release_oid=self.release,base_oid=COMBINED_BASE,
            purpose='BUILD_AUTHORIZED_DISABLED',sandbox_digest=self.sandbox_digest,
            contract_versions=dict(project_schema=7,recovery_schema=1,ipc_protocol=1))
        self.task=valid_task();self.task.update(task_id='task-M2',checkpoint_id='M2',base_sha=COMBINED_BASE,
            candidate_branch='control/model-orchestrator-v2-m2',authority_digest=manifest['authority_digest'],
            allowed_paths=[VALUE,ALLOWED_COPY,FLOOR_PATH,CASE_PATH],
            forbidden_paths=['docs/execution','AGENTS.md','scripts/model_orchestrator/contracts.py'],
            case_inventory=list(case_ids),resource_limits=dict(cpu=1,memory_bytes=1<<28,pids=64,disk_bytes=48<<30,output_bytes=1<<20,wall_seconds=max(30,timeout)),
            budget=dict(tokens=1000,cost_microusd=0,wall_seconds=max(30,timeout),tool_calls=100,speculative_corrections=2,causal_repairs=1))
        harness_name='controller/harness.sh'
        if script is None:script='test "$(cat '+VALUE+')" = "allowed change"\nprintf \'{"cases":['+','.join('{"id":"'+case+'","result":"PASS"}' for case in case_ids)+']}\\n\'\n'
        self.script=script;target=self.controller/harness_name;target.parent.mkdir(exist_ok=True);target.write_text(script)
        harness_digest=hashlib.sha256(script.encode()).hexdigest()
        self.task['harness_digest']=harness_digest
        check=self.task['check_argv'][0];check.update(argv=['/bin/sh','-e','/verifier/'+harness_name],harness_digest=harness_digest,
             required_cases=list(case_ids),timeout_seconds=timeout,resource_limits=copy.deepcopy(self.task['resource_limits']))
        self.task.update(evidence_classes=list(classes),acceptance_cases=['accept-'+cl for cl in classes],
            acceptance_requirements=[dict(class_id=cl,case_ids=['accept-'+cl],production_boundary='Prepared credentialless verifier fixture',
                environment_digest=c.canonical_digest(check['environment']),package_digest=c.canonical_digest(IMAGE),
                permission_digest=c.canonical_digest('rootless read-only candidate and harness'),persistence_expectation='Immutable controller receipts',
                harness_digest=harness_digest) for cl in classes])
        executable=Path(shutil.which('git')).resolve()
        self.catalog=dict(schema_version=1,git=dict(executable=str(executable),digest=g.digest_file(executable)),
            checks={'unit':dict(boundary='rootless',executable='/bin/sh',executable_digest=c.canonical_digest([IMAGE,'/bin/sh']),
                harness=harness_name,harness_digest=harness_digest,image=IMAGE,docker='/usr/bin/docker',
                docker_digest=g.digest_file('/usr/bin/docker') if Path('/usr/bin/docker').is_file() else 'd'*64,
                endpoint='unix:///run/user/'+str(os.getuid())+'/docker.sock')},
            cases={case:dict(path=CASE_PATH,symbol=CASE_SYMBOL) for case in case_ids},harnesses=[],executables=[],scratch_paths=['cache'],
            acceptance={cl:dict(requirement_digest=c.canonical_digest(req),harness=harness_name,
                boundary='hosted-only' if hosted else 'rootless',check_ids=['unit'],case_bindings={'accept-'+cl:case_ids[0]}) for cl,req in zip(classes,self.task['acceptance_requirements'])},performance={})
        if metrics:
            self.catalog['performance']=dict(baseline={'memory_bytes':131072,'fixture':'frozen negative sustained shell workload'},
                method={'observer':'rootless Docker cgroup MemUsage','sampling':'repeated no-stream observations','exclusions':[]})
            self.task.update(performance_applicability='applicable',performance_budgets=dict(applicability='applicable',
                authorization_digest='d'*64,rationale='Frozen resource negative fixture',baseline_digest=c.canonical_digest(self.catalog['performance']['baseline']),
                method_digest=c.canonical_digest(self.catalog['performance']['method']),duration_seconds=3,sample_count=2,
                bounds=[dict(metric='memory_bytes',maximum=262144,baseline_maximum=131072)],ownership_constraints=['one bounded rootless verifier; no stale reuse']))
            check['required_metrics']=['memory_bytes']
        self.records={};checks=json.loads((ROOT/c.CHECKS_PATH).read_text());ownership={row['id']:row['phase'] for row in checks['acceptance_cases']}
        for phase in ('M1','M2'):
            completed=['M0'] if phase=='M1' else ['M0','M1']
            record=dict(schema_version=1,repository=c.REPOSITORY_IDENTITY,architecture_spec_sha=c.ARCHITECTURE_SPEC_SHA,
                base_sha=COMBINED_BASE,release_sha=self.release,candidate_branch='control/model-orchestrator-v2-m2',
                authorization_id='fixture-auth-'+phase,task_id='task-'+phase,sequence=1,nonce='fixture-nonce-'+phase,
                sandbox_digest=self.sandbox_digest,authority_manifest_digest=c.canonical_digest(manifest['manifest']),checks_digest=c.canonical_digest(checks),
                purpose='DISABLED_BUILD_ONLY',authorized_phases=[phase],completed_phases=completed,
                phase_evidence=[dict(phase=p,case_ids=[case for case,owner in ownership.items() if owner==p],
                    receipt_id='fixture-'+p,receipt_digest=c.canonical_digest(['inert prerequisite fixture',COMBINED_BASE,p])) for p in completed],
                allowed_paths=['scripts/model_orchestrator/guards.py','scripts/model_orchestrator/verification.py',
                    'scripts/model_orchestrator/tests/test_verification.py','scripts/model_orchestrator/tests/test_contracts.py'],
                required_gates=['M0-contracts','M1-isolation','M2-verification'],required_case_ids=[case for case,owner in ownership.items() if owner==phase])
            record['scope_digest']=c.canonical_digest({k:record[k] for k in ('authorized_phases','allowed_paths','required_gates','required_case_ids')})
            self.records[phase]=record
        self.pin()

    def pin(self):
        for name,value in [('task',self.task),('catalog',self.catalog),*[('build-'+p,r) for p,r in self.records.items()]]:
            (self.controller/'controller'/ (name+'.json')).write_text(json.dumps(value)+'\n')
        self.source=commit(self.controller);self.authorities={}
        for phase,record in self.records.items():
            bootstrap=c.ControllerBootstrap(source_sha=self.source,anchor_sha=c.TRUSTED_DESIGN_BASE,base_sha=COMBINED_BASE,
                release_sha=self.release,candidate_branch=record['candidate_branch'],authorization_id=record['authorization_id'],
                task_id=record['task_id'],sequence=1,nonce=record['nonce'],sandbox_digest=self.sandbox_digest,
                build=c.RecordPin('controller/build-'+phase+'.json',c.canonical_digest(record)))
            self.authorities[phase]=c.load_release_authority(self.candidate,self.controller,bootstrap=bootstrap)
        self.floor=g.load_floor(self.authorities['M2'],'controller/task.json','controller/catalog.json')

    def work(self,name='worker'):
        return b.create_candidate(self.candidate,self.parent/name,COMBINED_BASE,authority=self.authorities['M1'])

    def changed(self,name='worker'):
        candidate=self.work(name);(candidate.root/VALUE).write_text('allowed change\n');commit(candidate.root);return candidate

    def inspect(self,candidate,name='quarantine',**kwargs):
        return g.inspect(candidate,self.floor,self.parent/name,source_authority=self.authorities['M1'],**kwargs)


class ParserTests(unittest.TestCase):
    def test_CP12_byte_safe_raw_status_pairs_and_malformed_matrix(self):
        header=b':100644 100644 '+b'a'*40+b' '+b'b'*40+b' R100\0'
        self.assertEqual(g.parse_raw(header+b'old name\0new name\0')[0]['paths'],['old name','new name'])
        for raw in (header+b'old\0',header+b'old\0new',header+b'old\0old\0',header+b'old\0../escape\0',
                    header+b'old\0bad\xff\0',header+b'old\0a//b\0',header+b'old\0./a\0',header+b'old\0a\\b\0',
                    header.replace(b'R100',b'R101')+b'old\0new\0',header.replace(b'100644',b'120000',1)+b'old\0new\0',
                    header.replace(b'R100',b'U')+b'old\0',b'garbage\0',b'\0',header+b'old\0new\0trailing\0'):
            with self.subTest(raw=raw),self.assertRaises(c.ContractError):g.parse_raw(raw)

    def test_CP14_CP16_case_count_never_replaces_identity(self):
        required=['old-case']
        for data in (b'{"status":"PASS"}',b'{"cases":[]}',b'{"cases":[{"id":"new-case","result":"PASS"}]}',
                     b'{"cases":[{"id":"old-case","result":"PASS"},{"id":"old-case","result":"PASS"}]}',
                     b'{"cases":[],"cases":[]}',b'bad',b'\xff'):
            with self.subTest(data=data),self.assertRaises(c.ContractError):v.case_results(data,required)
        row=v.case_results(b'{"cases":[{"id":"old-case","result":"SKIP"}]}',required)
        self.assertEqual(row['skipped_cases'],required)


class GitGuardsTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.temp=tempfile.TemporaryDirectory(prefix='or-m2-git-');cls.addClassCleanup(cls.temp.cleanup)
        cls.fixture=ExactFixture(Path(cls.temp.name)/'fixture');cls.counter=0

    def setUp(self):
        type(self).counter+=1;self.n=str(self.counter);self.f=self.fixture
        self.candidate=self.f.work('worker-'+self.n)

    def inspect(self):return self.f.inspect(self.candidate,'quarantine-'+self.n)

    def change(self):
        (self.candidate.root/VALUE).write_text('allowed change\n');return commit(self.candidate.root)

    def test_CP12_positive_exact_history_and_controller_receipt(self):
        head=self.change();result=self.inspect();self.assertEqual(result.record['head'],head)
        self.assertEqual(result.record['base'],COMBINED_BASE);self.assertEqual(result.record['vetoes'],[])
        self.assertTrue(result.record['flags']);result.verify()
        receipt=json.loads((result.root/'candidate-receipt.json').read_text());self.assertTrue(receipt['imported'])

    def test_CP12_A_hidden_protected_edit_reverted_is_retained(self):
        protected=self.candidate.root/'docs/execution/STATE.json';original=protected.read_bytes()
        protected.write_bytes(original+b'\n');bad=commit(self.candidate.root);protected.write_bytes(original);self.change()
        result=self.inspect();self.assertTrue(any(bad in item for item in result.record['vetoes']))

    def test_CP12_B_rename_laundering_through_protected_path(self):
        self.change();middle='docs/execution/renamed.txt';git(self.candidate.root,'mv',VALUE,middle);commit(self.candidate.root)
        git(self.candidate.root,'mv',middle,VALUE);commit(self.candidate.root)
        result=self.inspect();self.assertTrue(any(middle in item for item in result.record['vetoes']))

    def test_CP12_copy_protected_to_allowed_checks_source(self):
        shutil.copy2(self.candidate.root/'AGENTS.md',self.candidate.root/ALLOWED_COPY);commit(self.candidate.root)
        result=self.inspect();self.assertTrue(any('AGENTS.md' in item for item in result.record['vetoes']))

    def test_CP12_copy_allowed_to_protected_checks_destination(self):
        self.change();shutil.copy2(self.candidate.root/VALUE,self.candidate.root/'docs/execution/copied.txt');commit(self.candidate.root)
        self.assertTrue(self.inspect().record['vetoes'])

    def test_CP12_rename_protected_to_allowed_and_delete_add(self):
        git(self.candidate.root,'mv','AGENTS.md',ALLOWED_COPY);commit(self.candidate.root)
        result=self.inspect();self.assertTrue(any('AGENTS.md' in item for item in result.record['vetoes']))

    def test_CP12_intermediate_executable_bit_is_not_laundered(self):
        self.change();path=self.candidate.root/VALUE;path.chmod(0o755);commit(self.candidate.root)
        path.chmod(0o644);commit(self.candidate.root);self.assertTrue(any('executable mode' in x for x in self.inspect().record['vetoes']))

    def test_CP12_CP17_unsafe_topology_refs_types_fail_closed(self):
        self.change();root=self.candidate.root
        mutations=[lambda:(root/'.git/shallow').write_text(COMBINED_BASE+'\n'),
                   lambda:(root/'.git/info/grafts').write_text(COMBINED_BASE+'\n'),
                   lambda:(root/'.git/objects/info/alternates').write_text('/foreign\n'),
                   lambda:(root/'.git/refs/heads/foreign').write_text(COMBINED_BASE+'\n'),
                   lambda:(root/'symlink').symlink_to('/etc/passwd'),
                   lambda:os.link(root/VALUE,root/'hardlink')]
        for index,mutate in enumerate(mutations):
            before={str(p.relative_to(root)) for p in root.rglob('*')};mutate()
            with self.subTest(index=index),self.assertRaises(c.ContractError):self.f.inspect(self.candidate,'reject-'+self.n+'-'+str(index))
            for p in root.rglob('*'):
                if str(p.relative_to(root)) not in before and not p.is_dir():p.unlink()

    def test_CP12_replace_ref_gitlink_nonportable_and_branch_base(self):
        self.change();root=self.candidate.root
        replace=root/'.git/refs/replace';replace.mkdir();(replace/COMBINED_BASE).write_text(COMBINED_BASE+'\n')
        with self.assertRaises(c.ContractError):self.inspect()
        shutil.rmtree(replace)
        (root/'.git/HEAD').write_text(COMBINED_BASE+'\n')
        with self.assertRaises(c.ContractError):self.inspect()
        (root/'.git/HEAD').write_text('ref: refs/heads/candidate\n')
        git(root,'update-index','--add','--cacheinfo','160000,'+COMBINED_BASE+','+ALLOWED_COPY);git(root,'commit','-qm','fixture: gitlink')
        with self.assertRaises(c.ContractError):self.inspect()

    def test_CP12_empty_and_dirty_are_preserved_and_refused(self):
        original=w.manifest(self.candidate.root)
        with self.assertRaises(c.ContractError):self.inspect()
        self.assertEqual(w.manifest(self.candidate.root),original)
        self.change();(self.candidate.root/VALUE).write_text('uncommitted useful work')
        with self.assertRaises(c.ContractError):self.inspect()
        self.assertEqual((self.candidate.root/VALUE).read_text(),'uncommitted useful work')
        g.cleanup_quarantine(self.f.floor,self.f.parent/('quarantine-'+self.n))

    def test_CP12_protected_delete_and_distinct_add_replacement(self):
        (self.candidate.root/'AGENTS.md').unlink();self.change()
        result=self.inspect()
        self.assertTrue(any(row['status']=='D' and row['paths']==['AGENTS.md'] for item in result.record['commits'] for row in item['changes']))
        self.assertTrue(any('AGENTS.md' in row for row in result.record['vetoes']))

    def test_CP15_frozen_harness_change_is_nonwaivable_veto(self):
        fixture=ExactFixture(self.f.parent/('harness-floor-'+self.n))
        fixture.catalog['harnesses']=[CASE_PATH];fixture.pin();candidate=fixture.changed()
        path=candidate.root/CASE_PATH;path.write_text(path.read_text()+'\n# changed trusted case harness\n');commit(candidate.root)
        result=fixture.inspect(candidate)
        self.assertTrue(any('required harness/binding changed' in row for row in result.record['vetoes']))
        with self.assertRaises(c.ContractError):v.execute(result,fixture.parent/'attempt',lease_epoch=1,sequence=1)

    def test_CP15_CP16_D_required_case_deleted_fake_count_equal(self):
        path=self.candidate.root/CASE_PATH
        content=path.read_text().replace('def '+CASE_SYMBOL+'(', 'def test_fake_replacement(')
        path.write_text(content);self.change()
        self.assertTrue(any('required case identity missing' in item for item in self.inspect().record['vetoes']))

    def test_CP15_assertion_and_mock_timeout_flags_not_disposition(self):
        path=self.candidate.root/CASE_PATH;path.write_text(path.read_text().replace('self.assertFalse(c.REQUIRED_AUTHORITY_PATHS-tracked)','self.assertTrue(True) # mock timeout retry permission fallback geometry queue'))
        self.change();result=self.inspect()
        self.assertTrue(any('assertion' in x for x in result.record['flags']))
        self.assertTrue(any('mock boundary' in x for x in result.record['flags']))

    def test_CP15_structured_floor_bound_command_class_removal_veto(self):
        path=self.candidate.root/FLOOR_PATH
        path.write_text(json.dumps({'resource_limits':{'memory':10},'required_cases':['original'],'evidence_classes':['PACKAGED_RUNTIME']}));commit(self.candidate.root)
        path.write_text(json.dumps({'resource_limits':{},'required_cases':['fake'],'evidence_classes':['UNIT']}));self.change()
        self.assertTrue(any('floor/command/evidence' in x for x in self.inspect().record['vetoes']))

    def test_CP17_E_worker_config_index_hooks_cache_cannot_execute(self):
        head=self.change();root=self.candidate.root;marker=self.f.parent/'poison-ran'
        (root/'.git/config').write_text((root/'.git/config').read_text()+'\n[alias]\n status = !touch '+str(marker)+'\n[core]\n fsmonitor = '+str(marker)+'\n')
        (root/'.git/hooks/post-checkout').write_text('#!/bin/sh\ntouch '+str(marker)+'\n');(root/'.git/hooks/post-checkout').chmod(0o755)
        (root/'.git/index').write_bytes(b'forged worker index')
        (root/'cache').mkdir();(root/'cache/git').write_text('fake PASS');(root/'cache/PASS.json').write_text('{"status":"PASS"}')
        result=self.inspect();self.assertEqual(result.record['head'],head);self.assertFalse(marker.exists())
        self.assertFalse((result.root/'candidate/cache').exists())

    def test_CP17_mutation_after_guard_and_missing_floor_authority(self):
        self.change();result=self.inspect();(result.root/'candidate'/VALUE).write_text('later mutation')
        with self.assertRaises(c.ContractError):result.verify()
        with self.assertRaises(c.ContractError):g.Floor(self.f.authorities['M2'],self.f.task,self.f.catalog)


    def test_CP12_real_nonUTF_FIFO_packed_refs_and_wrong_base(self):
        self.change();root=self.candidate.root
        bad=os.fsencode(root)+b'/bad\xff'
        if os.uname().sysname=='Linux':
            fd=os.open(bad,os.O_CREAT|os.O_WRONLY,0o644);os.write(fd,b'bytes');os.close(fd)
            with self.assertRaises(c.ContractError):self.inspect()
            os.unlink(bad)
        else:
            # APFS rejects invalid UTF filenames at creation. The byte parser
            # runs here; the actual nonUTF filesystem attack runs on Debian.
            with self.assertRaises(OSError):os.open(bad,os.O_CREAT|os.O_WRONLY,0o644)
        os.mkfifo(root/'fifo')
        with self.assertRaises(c.ContractError):self.inspect()
        (root/'fifo').unlink()
        (root/'.git/packed-refs').write_text(COMBINED_BASE+' refs/tags/unexpected\n')
        with self.assertRaises(c.ContractError):self.inspect()
        (root/'.git/packed-refs').unlink()
        (root/'.git/refs/heads/candidate').write_bytes(b'\xff'*40+b'\n')
        with self.assertRaises(c.ContractError):self.inspect()
        wrong=b.create_candidate(self.f.candidate,self.f.parent/('wrong-'+self.n),COMBINED_BASE,authority=self.f.authorities['M1'])
        object.__setattr__(wrong,'base_oid','a'*40)
        with self.assertRaises(c.ContractError):self.f.inspect(wrong,'wrong-base-'+self.n)

    def test_CP15_required_test_file_deletion_and_all_flag_families(self):
        (self.candidate.root/CASE_PATH).unlink();self.change()
        result=self.inspect();self.assertTrue(any('required case missing' in item for item in result.record['vetoes']))
        flags='assert retry timeout continue-on-error || true except mock PATH environment permission feature fallback geometry width queue reuse ownership'
        (self.candidate.root/FLOOR_PATH).write_text(flags);commit(self.candidate.root)
        result=self.f.inspect(self.candidate,'all-flags-'+self.n)
        for label in ('assertion','retry/timeout','ignored failure','mock boundary','host executable','environment/permission','feature/fallback/geometry','resource ownership/reuse'):
            self.assertTrue(any(item.startswith(label+':') for item in result.record['flags']),label)

    def test_CP17_reserved_quarantine_crash_cleanup_and_durable_reload(self):
        self.change();dest=self.f.parent/('fault-'+self.n)
        def fault(point):
            if point=='reserved':raise RuntimeError('injected controller interruption')
        with self.assertRaises(RuntimeError):self.f.inspect(self.candidate,dest.name,fault=fault)
        before=w.manifest(self.candidate.root);dest.chmod(0o777)
        with self.assertRaises(c.ContractError):g.cleanup_quarantine(self.f.floor,dest)
        self.assertTrue(dest.exists());dest.chmod(0o700)
        g.cleanup_quarantine(self.f.floor,dest)
        self.assertFalse(dest.exists());self.assertEqual(w.manifest(self.candidate.root),before)
        result=self.inspect();self.assertEqual(g.restore_guarded(self.f.floor,result.root).record,result.record)
        with self.assertRaises(c.ContractError):g.cleanup_quarantine(self.f.floor,result.root)

    def test_CP17_copied_writable_handoff_and_rehashed_relocation_refused(self):
        self.change();result=self.inspect()
        destination=self.f.parent/('relocated-'+self.n)
        shutil.copytree(result.root,destination)
        record=json.loads((destination/'guard.json').read_text())
        entry=destination.stat();record['directory_identity']=[entry.st_dev,entry.st_ino]
        g.write_json(destination/'guard.json',record)
        receipt=json.loads((destination/'candidate-receipt.json').read_text())
        receipt['guard_receipt_digest']=c.canonical_digest(record)
        g.write_json(destination/'candidate-receipt.json',receipt)
        destination.chmod(0o777)
        with self.assertRaises(c.ContractError):g.restore_guarded(self.f.floor,destination)
        destination.chmod(0o700)
        with self.assertRaises(c.ContractError):g.restore_guarded(self.f.floor,destination)
        self.assertEqual(g.restore_guarded(self.f.floor,result.root).record,result.record)

    def test_CP17_writable_hardlinked_symlink_and_fifo_records_refused(self):
        self.change();result=self.inspect()
        for name in ('guard.json','reservation.json','candidate-receipt.json'):
            path=result.root/name;original=path.read_bytes()
            path.chmod(0o666)
            with self.subTest(name=name),self.assertRaises(c.ContractError):
                g.restore_guarded(self.f.floor,result.root)
            path.chmod(0o600)
            link=result.root/(name+'.link');os.link(path,link)
            with self.assertRaises(c.ContractError):g.restore_guarded(self.f.floor,result.root)
            link.unlink()
            path.rename(link);path.symlink_to(link)
            with self.assertRaises(c.ContractError):g.restore_guarded(self.f.floor,result.root)
            path.unlink();link.rename(path)
            path.unlink();os.mkfifo(path,0o600)
            with self.assertRaises(c.ContractError):g.restore_guarded(self.f.floor,result.root)
            path.unlink();path.write_bytes(original);path.chmod(0o600)
        self.assertEqual(g.restore_guarded(self.f.floor,result.root).record,result.record)
        result.root.chmod(0o777)
        try:
            with self.assertRaises(c.ContractError):result.verify()
        finally:result.root.chmod(0o700)


@unittest.skipUnless(LIVE,'required real Linux/rootless cases run with OR_M2_LIVE=1; no local fixture substitutes')
class LiveVerificationTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.output=Path(os.environ['OR_M2_OUTPUT']);cls.output.mkdir(mode=0o700,parents=True,exist_ok=False)
        cls.rows=[]

    @classmethod
    def tearDownClass(cls):
        g.write_json(cls.output/'acceptance.json',dict(cases=cls.rows,source_sha=git(ROOT,'rev-parse','HEAD'),
             image=IMAGE,scope='real disabled M2 inert fixtures; no product/review/promotion/adoption'))

    def fixture(self,script=None,**kwargs):
        self.case=self.output/self._testMethodName;self.case.mkdir(mode=0o700)
        self.f=ExactFixture(self.case/'fixture',script,**kwargs);self.candidate=self.f.changed()
        self.guard=self.f.inspect(self.candidate)
        return self.f

    def execute(self):
        result=v.execute(self.guard,self.case/'attempt',lease_epoch=1,sequence=1)
        self.rows.append(dict(case=self._testMethodName,result=result));return result

    def test_CP13_C_real_exit_failure_overrides_worker_PASS(self):
        self.fixture('printf \'{"status":"PASS"}\' > /scratch/PASS.json\nprintf \'{"cases":[{"id":"case1","result":"PASS"}]}\\n\'\nexit 7\n')
        result=self.execute();receipt=result['receipts'][0]
        self.assertEqual(receipt['exit_code'],7);self.assertEqual(receipt['outcome'],'FAIL');self.assertFalse(result['verification_passed'])

    def test_CP14_timeout(self):
        self.fixture('sleep 20\n',timeout=0.2);result=self.execute()
        self.assertNotEqual(result['receipts'][0]['outcome'],'PASS');self.assertFalse(result['verification_passed'])

    def test_CP14_signal(self):
        self.fixture('kill -TERM $$\n');receipt=self.execute()['receipts'][0]
        self.assertEqual(receipt['signal'],15);self.assertEqual(receipt['outcome'],'FAIL')

    def test_CP14_skip_and_empty(self):
        self.fixture('printf \'{"cases":[{"id":"case1","result":"SKIP"}]}\\n\'\n')
        result=self.execute();self.assertEqual(result['receipts'][0]['skipped_cases'],['case1']);self.assertFalse(result['verification_passed'])

    def test_CP14_malformed_and_fake_receipt(self):
        self.fixture('printf \'{"status":"PASS","exit_code":0}\\n\'\n');result=self.execute()
        self.assertEqual(result['receipts'][0]['outcome'],'FAIL')

    def test_CP14_unavailable_image_is_availability(self):
        self.fixture();self.f.catalog['checks']['unit']['image']='sha256:'+'f'*64
        self.f.catalog['checks']['unit']['executable_digest']=c.canonical_digest([self.f.catalog['checks']['unit']['image'],'/bin/sh']);self.f.pin()
        self.guard=self.f.inspect(self.candidate,'quarantine-unavailable')
        receipt=self.execute()['receipts'][0];self.assertEqual(receipt['outcome'],'LOCAL ENVIRONMENT BLOCKED')

    def test_CP14_readonly_candidate_harness_and_CP06_CP08_profile(self):
        script=('test "$(id -u)" = 10001\n'
                'test ! -e /home/huou07/or-m2-integrity-base8ec46/source/.git\n'
                'test ! -e /run/user/1000/docker.sock\n'
                'if printf poison > '+VALUE+' 2>/dev/null; then exit 31; fi\n'
                'if printf poison > /verifier/controller/harness.sh 2>/dev/null; then exit 32; fi\n'
                'printf \'{"cases":[{"id":"case1","result":"PASS"}]}\\n\'\n')
        self.fixture(script);result=self.execute();self.assertTrue(result['verification_passed'])

    def test_CP17_fresh_exact_verifier_ignores_worker_poison(self):
        script=('test "$(cat '+VALUE+')" = "allowed change"\n'
                'test ! -e cache/PASS.json\n'
                'test "$(command -v git)" = /usr/bin/git\n'
                'printf \'{"cases":[{"id":"case1","result":"PASS"}]}\\n\'\n')
        self.fixture(script);root=self.candidate.root
        (root/'.git/index').write_bytes(b'fake index');(root/'cache').mkdir();(root/'cache/PASS.json').write_text('{"status":"PASS"}')
        (root/'cache/git').write_text('fake shim');(root/'cache/git').chmod(0o755)
        (root/'.git/config').write_text((root/'.git/config').read_text()+'\n[alias]\n status = !echo PASS\n')
        self.guard=self.f.inspect(self.candidate,'quarantine-poisoned')
        result=self.execute();self.assertTrue(result['verification_passed'])
        self.assertFalse(result['acceptance_ready']) # unresolved material flag is not M3 disposition

    def test_CP18_F_UNIT_does_not_satisfy_packaged_or_persistence(self):
        self.fixture(classes=('UNIT','PACKAGED_RUNTIME','PERSISTENCE_RELAUNCH'))
        result=self.execute();self.assertTrue(result['verification_passed']);self.assertFalse(result['acceptance_ready'])
        self.assertTrue(any('PACKAGED_RUNTIME' in item for item in result['unresolved']))

    def test_CP18_missing_binding_and_hosted_pending(self):
        self.fixture(classes=('PACKAGED_RUNTIME',),hosted=True)
        result=self.execute();self.assertEqual(result['pending_hosted_classes'],['PACKAGED_RUNTIME'])
        self.assertFalse(any('required real class evidence pending' in item for item in result['unresolved']))

    def test_CP30_G_functional_PASS_resource_bound_failure(self):
        self.case=self.output/self._testMethodName;self.case.mkdir(mode=0o700)
        functional='test "$(cat '+VALUE+')" = "allowed change"\n'
        baseline=ExactFixture(self.case/'baseline',functional+'sleep 8\n'+self.frozen_success(),timeout=20,metrics=True)
        baseline.catalog['performance']['baseline']['memory_bytes']=1<<28
        perf=baseline.task['performance_budgets'];perf['bounds'][0].update(maximum=1<<28,baseline_maximum=1<<28)
        perf['baseline_digest']=c.canonical_digest(baseline.catalog['performance']['baseline']);baseline.pin()
        base_guard=baseline.inspect(baseline.changed());base_result=v.execute(base_guard,self.case/'baseline-attempt',lease_epoch=1,sequence=1)
        self.assertTrue(base_result['verification_passed'])
        base_receipt=base_result['receipts'][0];measured=base_receipt['measurements'][0]['value']
        self.assertGreater(measured,0)
        # A measured same-image baseline plus explicit noise allowance is
        # frozen in independent controller Git before the negative workload.
        bound=measured*1.5+(1<<20)
        script=functional+'data=$(head -c 8388608 /dev/zero | tr "\\000" x)\nsleep 8\n'+self.frozen_success()
        self.f=ExactFixture(self.case/'fixture',script,timeout=20,metrics=True)
        self.f.catalog['performance']['baseline']=dict(memory_bytes=bound,measured_peak=measured,noise_allowance='peak*1.5+1MiB',
            receipt_digest=c.canonical_digest(base_receipt),artifact_digest=base_receipt['artifact_digest'],image=IMAGE,
            environment_digest=base_receipt['environment_digest'],duration_seconds=base_receipt['duration_seconds'])
        perf=self.f.task['performance_budgets'];perf['bounds'][0].update(maximum=bound*2,baseline_maximum=bound)
        perf['baseline_digest']=c.canonical_digest(self.f.catalog['performance']['baseline']);self.f.pin()
        self.candidate=self.f.changed();self.guard=self.f.inspect(self.candidate)
        result=self.execute();receipt=result['receipts'][0]
        self.assertEqual(receipt['exit_code'],0);self.assertEqual(receipt['passed_cases'],['case1'])
        self.assertEqual(receipt['outcome'],'FAIL');self.assertFalse(result['verification_passed'])
        self.assertTrue(receipt['measurements']);self.assertGreater(receipt['measurements'][0]['value'],bound)
        self.rows.append(dict(case=self._testMethodName+'-measured-baseline',result=base_result,measured_peak=measured,frozen_bound=bound))

    def test_CP14_empty_required_result_and_missing_executable(self):
        self.fixture('printf \'{"cases":[]}\\n\'\n')
        self.assertEqual(self.execute()['receipts'][0]['outcome'],'FAIL')
        self.f.task['check_argv'][0]['argv'][0]='/bin/does-not-exist'
        binding=self.f.catalog['checks']['unit'];binding['executable']='/bin/does-not-exist'
        binding['executable_digest']=c.canonical_digest([IMAGE,binding['executable']]);self.f.pin()
        self.guard=self.f.inspect(self.candidate,'quarantine-missing-executable')
        result=v.execute(self.guard,self.case/'attempt-missing',lease_epoch=2,sequence=2)
        receipt=result['receipts'][0];self.assertEqual(receipt['outcome'],'LOCAL ENVIRONMENT BLOCKED')
        self.assertFalse(receipt['executable_found']);self.rows.append(dict(case=self._testMethodName+'-missing',result=result))

    def test_CP14_missing_wrong_harness_and_image_identity(self):
        self.fixture()
        self.f.catalog['checks']['unit']['harness_digest']='f'*64
        with self.assertRaises(c.ContractError):self.f.pin()
        self.f.catalog['checks']['unit']['harness_digest']=self.f.task['check_argv'][0]['harness_digest']
        self.f.catalog['checks']['unit']['harness']='controller/absent.sh'
        with self.assertRaises(c.ContractError):self.f.pin()
        self.f.catalog['checks']['unit']['harness']='controller/harness.sh'
        self.f.catalog['checks']['unit']['executable_digest']='f'*64
        with self.assertRaises(c.ContractError):self.f.pin()
        self.rows.append(dict(case=self._testMethodName,result='PASS: trusted admission refused all wrong bindings'))

    def test_CP14_mutation_during_verification_and_replay(self):
        self.fixture('sleep 1\n'+self.frozen_success())
        changed=False
        def fault(point):
            nonlocal changed
            if point=='running':
                (self.guard.root/'candidate'/VALUE).write_text('controller-side adversarial mutation');changed=True
        with self.assertRaises(c.ContractError):v.execute(self.guard,self.case/'attempt',lease_epoch=1,sequence=1,fault=fault)
        self.assertTrue(changed)
        record=json.loads((self.case/'attempt/attempt.json').read_text())
        self.assertEqual(record['checks']['unit']['phase'],'REMOVED')
        receipt=json.loads((self.case/'attempt/unit/receipt.json').read_text());self.assertNotEqual(receipt['outcome'],'PASS')
        self.rows.append(dict(case=self._testMethodName,receipt=receipt,result='PASS: mutation refused, exact stage removed'))

    @staticmethod
    def frozen_success():return 'printf \'{"cases":[{"id":"case1","result":"PASS"}]}\\n\'\n'

    def test_CP14_output_bound_and_incomplete_checks(self):
        self.fixture('head -c 2097152 /dev/zero\n');result=self.execute()
        self.assertFalse(result['verification_passed'])
        path=self.case/'attempt';record=json.loads((path/'attempt.json').read_text())
        record['receipts']=[];g.write_json(path/'attempt.json',record)
        self.assertTrue(any('incomplete required check set' in item for item in v.readiness(self.guard,path)['unresolved']))
        with self.assertRaises(c.ContractError):v.execute(self.guard,path,lease_epoch=1,sequence=1)
        record['task_id']='foreign';g.write_json(path/'attempt.json',record)
        with self.assertRaises(c.ContractError):v.recover_attempt(self.guard,path)

    def test_CP18_missing_actual_class_binding_budget_and_performance_waiver(self):
        self.fixture(classes=('UNIT','USER_JOURNEY','CLEAN_ENVIRONMENT','PERFORMANCE','RESOURCE_STRESS','CROSS_PLATFORM'))
        self.f.catalog['acceptance'].pop('USER_JOURNEY');self.f.pin()
        self.guard=self.f.inspect(self.candidate,'quarantine-missing-class');result=self.execute()
        self.assertFalse(result['acceptance_ready']);self.assertTrue(any('missing acceptance harness/binding: USER_JOURNEY' in r for r in result['unresolved']))
        self.f.task['performance_applicability']='not_applicable'
        self.f.task['performance_budgets']['rationale']='worker says no measurements'
        with self.assertRaises(c.ContractError):g.Floor(self.f.authorities['M2'],self.f.task,self.f.catalog)

    def test_CP17_H_M1_preserved_clean_and_dirty_handoff(self):
        self.case=self.output/self._testMethodName;self.case.mkdir(mode=0o700)
        self.f=ExactFixture(self.case/'fixture');ctx=multiprocessing.get_context('fork')
        for dirty in (False,True):
            row=self.case/('dirty' if dirty else 'clean');row.mkdir()
            box,_=v._runtime(self.f.catalog['checks']['unit'])
            volume=Path('/srv/opencut-v2/candidate');input_name='m2-handoff-'+uuid.uuid4().hex
            g.write_json(row/'input-locator.json',dict(path=str(volume/input_name),image=IMAGE))
            box.docker(['run','--rm','--network=none','--user=10001:10001','--mount=type=bind,src='+str(volume)+',dst=/volume',IMAGE,
                '/bin/sh','-ec','mkdir /volume/'+input_name+'; chmod 777 /volume/'+input_name])
            storage=volume/input_name
            source=b.create_candidate(self.f.candidate,storage,COMBINED_BASE,authority=self.f.authorities['M1'])
            b._bounded_candidate_filesystem(source.root,48<<30);before=w.manifest(source.root)
            store=s.RuntimeStore(row/'runtime',self.f.authorities['M1']);store.initialize()
            task=copy.deepcopy(self.f.task);task.update(task_id='task-M1',checkpoint_id='M1')
            digest=store.register_task(task,copy.deepcopy(task));store.register_candidate(task['task_id'],source)
            parent,child=ctx.Pipe()
            def produce():
                owned=s.RuntimeStore(row/'runtime',self.f.authorities['M1']);box,daemon=v._runtime(self.f.catalog['checks']['unit'])
                with owned.lock('task',task['task_id']):
                    stage=owned.claim(task['task_id'],digest,owner_nonce=uuid.uuid4().hex,boot_identity=box.boot_identity,
                        stage_id=uuid.uuid4().hex,stage_nonce=uuid.uuid4().hex)
                    reserved=w.reserve_launch(owned,stage,source,source.root.parent,daemon,IMAGE,'IMPLEMENTATION')
                    view=b._make_launch_view(source,source.root.parent,task['task_id'],stage['stage_id'],'IMPLEMENTATION',reserved)
                    w.bind_launch(owned,stage)
                    record=owned.launch_record(stage)
                    # This fixed inert shell/Git fixture is not an operational
                    # worker adapter. Only the scoped M2 text file is changed.
                    command='printf "allowed change\\n" > '+VALUE+'; '
                    if not dirty:
                        command+='git -c safe.directory=/candidate -c core.hooksPath=/dev/null add '+VALUE+'; git -c safe.directory=/candidate -c core.hooksPath=/dev/null -c user.name=Fixture -c user.email=fixture@example.invalid commit -qm "inert M2 handoff fixture"; '
                    command+='printf "\\n[alias]\\n status = !echo PASS\\n" >> .git/config; printf ready > .git/m2-ready; sleep 120'
                    identity=b.StageIdentity('0'*64,box.boot_identity,stage['owner_nonce'],stage['stage_id'],stage['lease_epoch'],
                        stage['stage_nonce'],source.authority_digest,'IMPLEMENTATION',task['task_id'])
                    args=['create','--name='+record['container_name'],'--pull=never','--read-only','--user=10001:10001',
                        '--cap-drop=ALL','--security-opt=no-new-privileges:true','--network=none','--ipc=none','--cgroupns=private',
                        '--pids-limit=64','--cpus=1','--memory=268435456','--memory-swap=268435456','--log-driver=none','--restart=no',
                        '--ulimit=nofile=256:256','--ulimit=fsize=1048576:1048576','--workdir=/candidate',
                        '--mount=type=bind,src='+str(view)+',dst=/candidate','--tmpfs=/scratch:rw,nosuid,nodev,noexec,size=1048576,mode=1777']
                    for key,value in identity.labels().items():args.extend(['--label',key+'='+value])
                    container=box.docker([*args,'--entrypoint=/usr/bin/timeout',IMAGE,'--signal=KILL','180','/bin/sh','-ec',command]).strip()
                    owned.bind_container(stage,container)
                    identity=b.StageIdentity(container,box.boot_identity,stage['owner_nonce'],stage['stage_id'],stage['lease_epoch'],
                        stage['stage_nonce'],source.authority_digest,'IMPLEMENTATION',task['task_id'])
                    expected=dict(image=IMAGE,mounts=[(str(view),'/candidate',True)],workdir='/candidate',environment=[],created_only=True,
                        limits=b.Limits(1,1<<28,64,48<<30,1<<20,1<<20,180),
                        command=('/usr/bin/timeout','--signal=KILL','180','/bin/sh','-ec',command))
                    box.verify_effective(identity,expected);g.write_json(row/'worker-effective.json',box._inspect(identity))
                    box.docker(['start',container])
                    deadline=time.monotonic()+25
                    while not (view/'.git/m2-ready').exists():
                        if time.monotonic()>deadline:raise AssertionError('inert M1 handoff fixture did not create useful result')
                        time.sleep(0.05)
                    (view/'.git/m2-ready').unlink()
                    stage=owned.inspect()['tasks'][task['task_id']]['stage']
                    head=(view/'.git/refs/heads/candidate').read_text().strip()
                    child.send(dict(stage=stage,head=head));signal.pause()
            proc=ctx.Process(target=produce);proc.start();self.assertTrue(parent.poll(40));event=parent.recv()
            os.kill(proc.pid,signal.SIGKILL);proc.join(10);self.assertEqual(proc.exitcode,-9)
            fresh_parent,fresh_child=ctx.Pipe()
            def recover():
                owned=s.RuntimeStore(row/'runtime',self.f.authorities['M1']);box,_=v._runtime(self.f.catalog['checks']['unit'])
                self.assertEqual(box._prepared,{})
                with owned.lock('task',task['task_id']):
                    # Fresh rootless reconciliation kills the real fixture and
                    # proves entire-stage death before useful handoff/cleanup.
                    proof=box.reconcile_launch(owned,event['stage']);w.preserve_launch(owned,event['stage'],proof)
                    owned.settle(event['stage'],proof);w.cleanup_launch(owned,event['stage'],box,proof)
                fresh_child.send(owned.launch_record(event['stage']))
            fresh=ctx.Process(target=recover);fresh.start();fresh.join(50)
            self.assertEqual(fresh.exitcode,0);self.assertTrue(fresh_parent.poll(1));record=fresh_parent.recv()
            preserved=w.restore_preserved(store,event['stage']);self.assertEqual(w.manifest(source.root),before)
            self.assertEqual((preserved.root/VALUE).read_text(),'allowed change\n');self.assertEqual(record['phase'],'REMOVED')
            if dirty:
                with self.assertRaises(c.ContractError):g.inspect_preserved(store,event['stage'],self.f.floor,row/'quarantine')
                self.assertEqual((preserved.root/VALUE).read_text(),'allowed change\n')
            else:
                guard=g.inspect_preserved(store,event['stage'],self.f.floor,row/'quarantine');self.assertEqual(guard.record['head'],event['head'])
                result=v.execute(guard,row/'attempt',lease_epoch=1,sequence=1);self.assertTrue(result['verification_passed'])
                self.rows.append(dict(case=self._testMethodName+'-clean',stage=event['stage'],result=result,head=event['head']))
            self.rows.append(dict(case=self._testMethodName+'-'+str(dirty),phase=record['phase'],controller_exit=-9,
                boundary='real M1 rootless fixture/controller SIGKILL, fresh reconciliation, preservation, settlement, cleanup; M2 real verifier',
                preserved_manifest=w.manifest(preserved.root)))
            # Original disposable input is unchanged. Useful work is outside
            # this exact input in M1's durably indexed preservation artifact.
            for directory,_,_ in os.walk(storage,topdown=False):
                if Path(directory)!=storage:Path(directory).chmod(0o777)
            box.docker(['run','--rm','--network=none','--user=10001:10001','--mount=type=bind,src='+str(volume)+',dst=/volume',IMAGE,
                '/bin/rm','-rf','/volume/'+input_name])

    def test_CP17_verifier_durable_creation_receipt_fault_boundaries(self):
        self.fixture();ctx=multiprocessing.get_context('fork')
        for point in ('reserved','materialized','created','receipted'):
            destination=self.case/point;parent,child=ctx.Pipe()
            def work():
                def fault(boundary):
                    if boundary==point:child.send(os.getpid());signal.pause()
                v.execute(self.guard,destination,lease_epoch=1,sequence=1,fault=fault)
            proc=ctx.Process(target=work);proc.start();self.assertTrue(parent.poll(40),point);pid=parent.recv()
            os.kill(pid,signal.SIGKILL);proc.join(10);self.assertEqual(proc.exitcode,-9)
            fresh_parent,fresh_child=ctx.Pipe()
            def recover():fresh_child.send(v.recover_attempt(g.restore_guarded(self.f.floor,self.guard.root),destination))
            fresh=ctx.Process(target=recover);fresh.start();fresh.join(45)
            self.assertEqual(fresh.exitcode,0,point);self.assertTrue(fresh_parent.poll(1));result=fresh_parent.recv()
            self.assertEqual(result['verification_passed'],point=='receipted')
            self.assertFalse(result['acceptance_ready'])
            record=json.loads((destination/'attempt.json').read_text())
            self.assertTrue(all(row['phase']=='REMOVED' for row in record['checks'].values()))
            self.assertFalse((destination/'unit/candidate').exists());self.assertFalse((destination/'unit/harness').exists())
            self.rows.append(dict(case=self._testMethodName+'-'+point,controller_exit=-9,result=result))

    def test_CP17_verifier_crash_exact_recovery_no_rerun(self):
        self.fixture('sleep 20\n',timeout=30)
        ctx=multiprocessing.get_context('fork');parent,child=ctx.Pipe();destination=self.case/'attempt'
        def work():
            def fault(point):
                if point=='running':child.send(os.getpid());signal.pause()
            v.execute(self.guard,destination,lease_epoch=1,sequence=1,fault=fault)
        proc=ctx.Process(target=work);proc.start();self.assertTrue(parent.poll(30));pid=parent.recv()
        os.kill(pid,signal.SIGKILL);proc.join(10);self.assertEqual(proc.exitcode,-9)
        fresh_parent,fresh_child=ctx.Pipe()
        def recover():fresh_child.send(v.recover_attempt(g.restore_guarded(self.f.floor,self.guard.root),destination))
        fresh=ctx.Process(target=recover);fresh.start();fresh.join(40)
        self.assertEqual(fresh.exitcode,0);self.assertTrue(fresh_parent.poll(1));result=fresh_parent.recv()
        self.assertFalse(result['verification_passed'])
        record=json.loads((destination/'attempt.json').read_text());self.assertEqual(record['checks']['unit']['phase'],'REMOVED')
        self.rows.append(dict(case=self._testMethodName,controller_pid=pid,controller_exit=-9,result=result))


if __name__=='__main__':unittest.main(verbosity=2)
