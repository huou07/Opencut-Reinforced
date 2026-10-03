#!/usr/bin/env python3
"""M1: real POSIX/process/Git checks, explicit injected crashes, no live PASS fiction."""
from __future__ import annotations
import copy
import multiprocessing
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

REPO_ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(REPO_ROOT / 'scripts'))
from model_orchestrator import contracts as c, store as s, sandbox as b
from model_orchestrator.tests.test_contracts import ProvenanceFixture, valid_task

CTX = multiprocessing.get_context('fork')


def object_record(label='diagnostic'):
    return dict(schema_version=1, kind='diagnostic', payload=dict(label=label))


def crash_transaction(root, authority, point):
    runtime = s.RuntimeStore(root, authority)
    current = runtime.inspect()
    def fault(at):
        if at == point:
            os._exit(77)
    runtime.transaction(current['sequence'], current['epoch'], lambda state: state.update(paused=True, pause_generation=1), objects=[object_record(point)], fault=fault)
    os._exit(1)


def contend(root, authority, task_id, digest, ready, gate, results, index):
    runtime = s.RuntimeStore(root, authority)
    stale = runtime.inspect()  # Deliberately before acquisition.
    ready.put(stale['sequence'])
    gate.wait(20)
    try:
        with runtime.lock('task', task_id):
            stage = runtime.claim(task_id, digest, owner_nonce='owner-'+str(index), boot_identity='boot', stage_id='fixture', stage_nonce='nonce-'+str(index))
            results.put(('CLAIMED', stage['lease_epoch']))
    except c.ContractError:
        results.put(('REFUSED', None))


def lock_contender(root, authority, kind, task_id, results):
    runtime = s.RuntimeStore(root, authority)
    try:
        with runtime.lock(kind, task_id, blocking=False):
            results.put('ACQUIRED')
    except BlockingIOError:
        results.put('CONTENDED')


def restore_child(root, authority, task_id, results):
    restored = b.restore_candidate(s.RuntimeStore(root, authority), task_id)
    results.put((restored.nonce, (restored.root/'interrupted.txt').read_text(),
                 subprocess.check_output(['git','rev-parse','HEAD'],cwd=restored.root).decode().strip()))


class StoreTests(unittest.TestCase):
    """CP27/28/29/34 and state part of CP10: real APFS/POSIX + child processes."""
    @classmethod
    def setUpClass(cls):
        cls.fixture_dir = tempfile.TemporaryDirectory()
        cls.addClassCleanup(cls.fixture_dir.cleanup)
        cls.fixture = ProvenanceFixture(Path(cls.fixture_dir.name).resolve())
        cls.authority = cls.fixture.authorities['M1']
        print('REAL_ENVIRONMENT OS='+sys.platform+' filesystem='+s.filesystem_type(REPO_ROOT)+' cases=CP27,CP28,CP29,CP34 observation=real-process/filesystem; CP29=fault-injection', flush=True)

    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name).resolve() / 'runtime'
        self.runtime = s.RuntimeStore(self.root, self.authority)

    def initialize_task(self):
        self.runtime.initialize()
        payload = c._release_authority(self.authority)
        task = valid_task()
        task.update(task_id=payload['build']['task_id'], checkpoint_id='M1', base_sha=payload['build']['base_sha'], candidate_branch=payload['build']['candidate_branch'], authority_digest=payload['git']['authority_digest'])
        return task['task_id'], self.runtime.register_task(task, copy.deepcopy(task))

    def test_inspection_missing_store_is_read_only(self):
        before = list(self.root.parent.iterdir())
        with self.assertRaises(c.ContractError):
            self.runtime.inspect()
        self.assertEqual(before, list(self.root.parent.iterdir()))
        self.assertFalse(self.root.exists())

    def test_explicit_initialize_and_no_reinitialization(self):
        state = self.runtime.initialize()
        self.assertEqual((state['sequence'], state['epoch']), (0, 0))
        for path in ('bootstrap.json', 'state.json', 'locks/state.lock', 'locks/integration.lock', 'objects', 'artifacts', 'telemetry', 'published'):
            self.assertTrue((self.root/path).exists())
        before = {p: p.stat().st_mtime_ns for p in self.root.rglob('*')}
        self.runtime.inspect()
        self.assertEqual(before, {p: p.stat().st_mtime_ns for p in self.root.rglob('*')})
        with self.assertRaises(c.ContractError):
            self.runtime.initialize()

    def test_phase_authority_raw_mapping_and_M0_refuse(self):
        for authority in ({'authorized_phases':['M1']}, self.fixture.authorities['M0']):
            with self.assertRaises(c.ContractError):
                s.RuntimeStore(self.root, authority).initialize()
            self.assertFalse(self.root.exists())

    def test_strict_state_negative_matrix_without_repair(self):
        self.runtime.initialize()
        path = self.root/'state.json'; original = path.read_bytes()
        changes = dict(schema_version=True, sequence=True, epoch=-1, pause_generation=2, paused=1, tasks=[], active_task='../bad', object_digests=['0'*64], unknown='bad')
        for key, value in changes.items():
            with self.subTest(key=key):
                bad=c.load_json_strict(original.decode()); bad[key]=value
                path.write_text(c.canonical_json(bad))
                before=path.read_bytes()
                with self.assertRaises(c.ContractError):self.runtime.inspect()
                self.assertEqual(path.read_bytes(), before)
        for raw in ('{"sequence":1,"sequence":2}', '{"x":NaN}', '{invalid}', '{"schema_version":2}'):
            path.write_text(raw)
            with self.assertRaises(c.ContractError):self.runtime.inspect()
            self.assertEqual(path.read_text(),raw)
        path.write_bytes(original)

    def test_symlink_and_oversize_authority_refusal(self):
        self.runtime.initialize()
        for name in ('state.json','bootstrap.json'):
            path=self.root/name; backup=self.root/(name+'.backup'); path.rename(backup);path.symlink_to(backup)
            with self.assertRaises(c.ContractError):self.runtime.inspect()
            path.unlink();backup.rename(path)
        path=self.root/'state.json'; original=path.read_bytes();path.write_bytes(b' '*(s.MAX_BYTES+1))
        with self.assertRaises(c.ContractError):self.runtime.inspect()
        path.write_bytes(original)
        with self.assertRaises(c.ContractError):
            self.runtime.transaction(0,0,lambda state:None,objects=[dict(schema_version=True,kind='diagnostic',payload={})])
        deep={};nested=deep
        for _ in range(34):nested['x']={};nested=nested['x']
        with self.assertRaises(c.ContractError):s._bytes(deep)

    def test_sequence_epoch_and_lock_order(self):
        self.runtime.initialize()
        self.assertEqual(self.runtime.set_paused(True)['sequence'],1)
        with self.assertRaises(c.ContractError):self.runtime.transaction(0,0,lambda state:None)
        for mutation in (lambda state:state.update(sequence=50),lambda state:state.update(authority_digest='0'*64),lambda state:state.update(pause_generation=0),lambda state:state.update(epoch=True)):
            with self.assertRaises(c.ContractError):self.runtime.transaction(1,0,mutation)
        with self.runtime.lock('state'):
            with self.assertRaises(c.ContractError):
                with self.runtime.lock('integration'):pass
        for name in ('../bad','a/b','/tmp','a\\b',''):
            with self.assertRaises(c.ContractError):
                with self.runtime.lock('task',name):pass

    def test_immutable_object_creation_conflict_and_corruption(self):
        self.runtime.initialize();record=object_record();digest=c.canonical_digest(record)
        state=self.runtime.transaction(0,0,lambda state:None,objects=[record])
        path=self.root/'objects'/(digest+'.json')
        self.assertEqual(state['object_digests'],[digest]);self.assertEqual(path.read_bytes(),s._bytes(record))
        self.runtime.transaction(1,0,lambda state:None,objects=[record])
        path.write_text('{"schema_version":1,"kind":"diagnostic","payload":{"label":"forged"}}')
        with self.assertRaises(c.ContractError):self.runtime.inspect()
        with self.assertRaises(c.ContractError):self.runtime.transaction(2,0,lambda state:None,objects=[record])

    def test_orphan_object_has_no_authority_or_automatic_collection(self):
        self.runtime.initialize();record=object_record();digest=c.canonical_digest(record)
        path=self.root/'objects'/(digest+'.json');path.write_bytes(s._bytes(record))
        self.assertEqual(self.runtime.inspect()['object_digests'],[])
        self.assertTrue(path.exists())
        path.write_bytes(b'corrupt orphan')
        self.assertEqual(self.runtime.inspect()['object_digests'],[])
        with self.assertRaises(c.ContractError):self.runtime.transaction(0,0,lambda state:None,objects=[record])
        self.assertEqual(self.runtime.inspect()['sequence'],0)

    def test_object_symlink_refused(self):
        self.runtime.initialize();record=object_record();digest=c.canonical_digest(record)
        elsewhere=self.root/'diagnostic';elsewhere.write_bytes(s._bytes(record))
        (self.root/'objects'/(digest+'.json')).symlink_to(elsewhere)
        with self.assertRaises(c.ContractError):self.runtime.transaction(0,0,lambda state:None,objects=[record])
        (self.root/'objects'/(digest+'.json')).unlink()
        saved=self.root/'objects-saved';(self.root/'objects').rename(saved)
        outside=self.root.parent/'outside';outside.mkdir();(self.root/'objects').symlink_to(outside)
        with self.assertRaises(c.ContractError):self.runtime.transaction(0,0,lambda state:None,objects=[record])
        self.assertEqual(list(outside.iterdir()),[])

    def test_CP29_real_process_crashes_at_all_boundaries(self):
        for point in s.FAULT_POINTS:
            with self.subTest(point=point), tempfile.TemporaryDirectory() as directory:
                root=Path(directory).resolve()/'runtime';runtime=s.RuntimeStore(root,self.authority);previous=runtime.initialize()
                child=CTX.Process(target=crash_transaction,args=(root,self.authority,point));child.start();child.join(30)
                self.assertFalse(child.is_alive());self.assertEqual(child.exitcode,77)
                restarted=s.RuntimeStore(root,self.authority).inspect()
                if point in ('after_replace','before_parent_fsync'):
                    self.assertEqual(restarted['sequence'],1);self.assertTrue(restarted['paused']);self.assertEqual(len(restarted['object_digests']),1)
                else:self.assertEqual(restarted,previous)
                for digest in restarted['object_digests']:self.assertTrue((root/'objects'/(digest+'.json')).is_file())

    def test_parent_directory_fsync_failure_does_not_publish_partial_state(self):
        self.runtime.initialize();real=s._fsync_dir
        def fail(path):
            if path==self.root:raise OSError('injected directory fsync failure')
            real(path)
        with patch.object(s,'_fsync_dir',side_effect=fail),self.assertRaises(OSError):
            self.runtime.transaction(0,0,lambda state:state.update(paused=True,pause_generation=1),objects=[object_record()])
        state=self.runtime.inspect();self.assertEqual(state['sequence'],1);self.assertEqual(len(state['object_digests']),1)

    def test_real_cross_process_each_separate_lock(self):
        self.runtime.initialize()
        for kind,task_id in (('state',None),('task','task-1'),('integration',None)):
            with self.subTest(kind=kind),self.runtime.lock(kind,task_id):
                results=CTX.Queue();child=CTX.Process(target=lock_contender,args=(self.root,self.authority,kind,task_id,results));child.start();child.join(30)
                self.assertFalse(child.is_alive());self.assertEqual(results.get(timeout=2),'CONTENDED')
        with self.runtime.lock('task','task-1'):
            results=CTX.Queue();child=CTX.Process(target=lock_contender,args=(self.root,self.authority,'integration',None,results));child.start();child.join(30)
            self.assertEqual(results.get(timeout=2),'ACQUIRED')

    def test_CP27_two_real_controllers_one_claim_and_stale_reader_refuses(self):
        task_id,digest=self.initialize_task();ready=CTX.Queue();results=CTX.Queue();gate=CTX.Event()
        children=[CTX.Process(target=contend,args=(self.root,self.authority,task_id,digest,ready,gate,results,index)) for index in (1,2)]
        for child in children:child.start()
        self.assertEqual([ready.get(timeout=30),ready.get(timeout=30)],[1,1]);gate.set()
        for child in children:child.join(30);self.assertFalse(child.is_alive());self.assertEqual(child.exitcode,0)
        self.assertEqual(sorted([results.get(timeout=2)[0],results.get(timeout=2)[0]]),['CLAIMED','REFUSED'])
        state=self.runtime.inspect();self.assertEqual(state['tasks'][task_id]['attempt'],1);self.assertEqual(state['epoch'],1)

    def test_CP28_pause_before_claim_zero_opportunities(self):
        task_id,digest=self.initialize_task();self.runtime.set_paused(True)
        with self.runtime.lock('task',task_id),self.assertRaises(c.ContractError):
            self.runtime.claim(task_id,digest,owner_nonce='owner',boot_identity='boot',stage_id='stage',stage_nonce='nonce')
        self.assertEqual(self.runtime.inspect()['tasks'][task_id]['attempt'],0)

    def test_CP28_pause_after_claim_persists_stage_and_blocks_next(self):
        task_id,digest=self.initialize_task()
        with self.runtime.lock('task',task_id):
            stage=self.runtime.claim(task_id,digest,owner_nonce='owner',boot_identity='boot',stage_id='stage',stage_nonce='nonce')
            self.runtime.set_paused(True)
            self.assertEqual(self.runtime.inspect()['tasks'][task_id]['stage'],stage)
        restarted=s.RuntimeStore(self.root,self.authority);state=restarted.inspect()
        self.assertTrue(state['paused']);self.assertEqual(state['pause_generation'],1)
        with restarted.lock('task',task_id),self.assertRaises(c.ContractError):
            restarted.claim(task_id,digest,owner_nonce='new',boot_identity='new-boot',stage_id='stage',stage_nonce='new')

    def test_CP10_unknown_liveness_boot_mismatch_PID_loss_never_reuses_lease(self):
        task_id,digest=self.initialize_task()
        with self.runtime.lock('task',task_id):
            stage=self.runtime.claim(task_id,digest,owner_nonce='owner',boot_identity='old-boot',stage_id='stage',stage_nonce='nonce')
        restarted=s.RuntimeStore(self.root,self.authority)
        with restarted.lock('task',task_id),self.assertRaises(c.ContractError):
            restarted.claim(task_id,digest,owner_nonce='new-owner',boot_identity='new-boot',stage_id='stage',stage_nonce='new-nonce')
        self.assertEqual(restarted.inspect()['tasks'][task_id]['stage'],stage)

    def test_CP34_real_supported_storage_and_unsupported_refusal(self):
        self.assertIn(s.verify_storage(self.root.parent),('apfs','hfs','ext2/ext3','ext4','xfs','btrfs','tmpfs'))
        with patch.object(s.sys,'platform','win32'),self.assertRaises(c.ContractError):s.filesystem_type(self.root.parent)
        with patch.object(s,'filesystem_type',side_effect=s.StoreError('UNSUPPORTED_FILESYSTEM_PROFILE: nfs')),self.assertRaises(c.ContractError):self.runtime.initialize()
        self.assertFalse(self.root.exists())

    def test_task_authority_drift_and_re_freeze_refuse(self):
        task_id,digest=self.initialize_task()
        with self.runtime.lock('task',task_id),self.assertRaises(c.ContractError):
            self.runtime.claim(task_id,'0'*64,owner_nonce='owner',boot_identity='boot',stage_id='stage',stage_nonce='nonce')
        path=self.fixture.candidate/'AGENTS.md';original=path.read_bytes();path.write_bytes(original+b'\ndrift\n')
        try:
            with self.assertRaises(c.ContractError):self.runtime.inspect()
        finally:path.write_bytes(original)

    def test_CP11_new_controller_restores_original_candidate_dirty_work_and_commit(self):
        task_id, _ = self.initialize_task()
        candidate = b.create_candidate(self.fixture.candidate, self.root.parent/'worker',
                c.TRUSTED_DESIGN_BASE, authority=self.authority)
        self.runtime.register_candidate(task_id, candidate)
        path=candidate.root/'interrupted.txt';path.write_text('clean worker commit')
        subprocess.run(['git','add','interrupted.txt'],cwd=candidate.root,check=True)
        subprocess.run(['git','-c','user.name=Fixture','-c','user.email=fixture@example.invalid','commit','-qm','fixture worker commit'],cwd=candidate.root,check=True)
        commit=subprocess.check_output(['git','rev-parse','HEAD'],cwd=candidate.root).decode().strip()
        path.write_text('dirty interrupted work');(candidate.root/'untracked.txt').write_text('diagnostics')
        results=CTX.Queue();child=CTX.Process(target=restore_child,args=(self.root,self.authority,task_id,results));child.start();child.join(30)
        self.assertEqual(child.exitcode,0);self.assertEqual(results.get(timeout=2),(candidate.nonce,'dirty interrupted work',commit))
        self.assertEqual((candidate.root/'untracked.txt').read_text(),'diagnostics')
        with self.assertRaises(c.ContractError):self.runtime.register_candidate(task_id,candidate)
        original=candidate.root;renamed=original.with_name('saved-worker');original.rename(renamed);original.mkdir()
        with self.assertRaises(c.ContractError):b.restore_candidate(self.runtime,task_id)
        original.rmdir();renamed.rename(original)
        self.assertEqual(b.restore_candidate(self.runtime,task_id).nonce,candidate.nonce)
        pack=self.root.parent/'worker.pack'
        self.assertEqual(b.export_candidate(candidate,pack,authority=self.authority),commit)
        imported=b.import_candidate(pack,self.root.parent/'imported',commit,authority=self.authority)
        b.inspect_candidate(imported.root)
        self.assertEqual(subprocess.check_output(['git','rev-parse','HEAD'],cwd=imported.root).decode().strip(),commit)
        self.assertEqual(path.read_text(),'dirty interrupted work')

    def test_task_pointer_history_and_forged_completion_cannot_release_writer(self):
        task_id,digest=self.initialize_task()
        with self.runtime.lock('task',task_id):
            stage=self.runtime.claim(task_id,digest,owner_nonce='owner',boot_identity='boot',stage_id='stage',stage_nonce='nonce')
            stage=self.runtime.bind_container(stage,'a'*64)
            for proof in ({'terminated':True},None,{'completion':'PASS','container_id':'a'*64}):
                with self.assertRaises(c.ContractError):self.runtime.settle(stage,proof)
            current=self.runtime.inspect()
            for mutate in (lambda state:state['tasks'][task_id].update(attempt=0),lambda state:state['tasks'][task_id].update(contract_digest='0'*64),lambda state:state.update(tasks={},active_task=None)):
                with self.assertRaises(c.ContractError):self.runtime.transaction(current['sequence'],current['epoch'],mutate)
            self.assertEqual(self.runtime.inspect(),current)


class GitIsolationTests(unittest.TestCase):
    """CP09/11 real independent Git/OS observations, no worker/model launch."""
    def setUp(self):
        self.temp=tempfile.TemporaryDirectory();self.addCleanup(self.temp.cleanup)
        self.root=Path(self.temp.name).resolve();self.source=self.root/'source';self.source.mkdir()
        self.git(self.source,'init','-q','--initial-branch=candidate')
        (self.source/'note.txt').write_text('original')
        self.git(self.source,'add','-A');self.git(self.source,'-c','user.name=Fixture','-c','user.email=fixture@example.invalid','commit','-qm','fixture base')
        self.base=self.git(self.source,'rev-parse','HEAD')
        self.candidate=b.create_fixture_candidate(self.source,self.root/'candidate',self.base)
    def git(self,root,*args):
        return subprocess.check_output(['git',*args],cwd=root,stderr=subprocess.DEVNULL).decode().strip()
    def test_CP09_real_independent_clone_no_shared_objects_refs_or_origin(self):
        b.inspect_candidate(self.candidate.root)
        source_git=self.source/'.git';candidate_git=self.candidate.root/'.git'
        self.assertTrue(candidate_git.is_dir());self.assertNotEqual(source_git.stat().st_ino,candidate_git.stat().st_ino)
        self.assertEqual(self.git(self.candidate.root,'remote'),'')
        self.assertEqual(self.git(self.candidate.root,'rev-parse','HEAD'),self.base)
        for path in candidate_git.rglob('*'):
            if path.is_file():self.assertEqual(path.stat().st_nlink,1)
        self.git(self.source,'update-ref','refs/heads/controller-only',self.base)
        self.assertNotIn('controller-only',self.git(self.candidate.root,'show-ref'))
    def test_worktree_alternate_common_dir_and_unsafe_origin_refuse(self):
        worktree=self.root/'worktree';self.git(self.source,'worktree','add','--detach',str(worktree),self.base)
        with self.assertRaises(c.ContractError):b.inspect_candidate(worktree)
        for name,contents in [('commondir',str(self.source/'.git')),('objects/info/alternates',str(self.source/'.git/objects')),('objects/info/http-alternates','https://wrong.example/objects')]:
            path=self.candidate.root/'.git'/name;path.parent.mkdir(parents=True,exist_ok=True);path.write_text(contents)
            with self.assertRaises(c.ContractError):b.inspect_candidate(self.candidate.root)
            path.unlink()
        for origin in ('https://github.com/meomeo/Opencut-Reinforced.git',str(self.source),'https://credential@example.invalid/repo'):
            self.git(self.candidate.root,'remote','add','origin',origin)
            with self.assertRaises(c.ContractError):b.inspect_candidate(self.candidate.root)
            self.git(self.candidate.root,'remote','remove','origin')
    def test_shared_hardlink_symlink_git_storage_refuse(self):
        path=self.candidate.root/'.git/objects/shared';os.link(self.source/'.git/HEAD',path)
        with self.assertRaises(c.ContractError):b.inspect_candidate(self.candidate.root)
        path.unlink();path.symlink_to(self.source/'.git/objects')
        with self.assertRaises(c.ContractError):b.inspect_candidate(self.candidate.root)
    def test_CP11_dirty_diagnostics_and_clean_commit_survive_sanitized_export(self):
        root=self.candidate.root
        (root/'note.txt').write_text('worker committed');self.git(root,'add','note.txt');self.git(root,'-c','user.name=Fixture','-c','user.email=fixture@example.invalid','commit','-qm','worker commit')
        worker_commit=self.git(root,'rev-parse','HEAD');(root/'note.txt').write_text('dirty interrupted');(root/'diagnostic.txt').write_text('untracked diagnostic')
        hook=root/'.git/hooks/pre-commit';hook.write_text('#!/bin/sh\ntouch '+str(self.root/'HOOK_EXECUTED')+'\n');hook.chmod(0o755)
        self.git(root,'config','core.hooksPath',str(hook.parent))
        exported=self.root/'export.pack';self.assertEqual(b.export_candidate(self.candidate,exported),worker_commit)
        self.assertEqual((root/'note.txt').read_text(),'dirty interrupted');self.assertEqual((root/'diagnostic.txt').read_text(),'untracked diagnostic');self.assertEqual(self.git(root,'rev-parse','HEAD'),worker_commit)
        self.assertFalse((self.root/'HOOK_EXECUTED').exists())
        imported=self.root/'imported';imported.mkdir();self.git(imported,'init','-q')
        subprocess.run(['git','index-pack','--stdin'],cwd=imported,input=exported.read_bytes(),check=True,stdout=subprocess.DEVNULL)
        self.assertEqual(self.git(imported,'show',worker_commit+':note.txt'),'worker committed')
        with self.assertRaises(c.ContractError):b.create_fixture_candidate(self.source,root,self.base)
    def test_role_config_overlay_is_external_and_candidate_preserved(self):
        root=self.candidate.root;(root/'opencode.json').write_text('malicious config')
        (root/'.opencode').mkdir();(root/'.opencode/agent').mkdir();(root/'.opencode/agent/evil.md').write_text('allow all')
        overlay=b.write_role_overlays(self.root/'overlay','INVESTIGATION_REVIEW')
        self.assertEqual((root/'opencode.json').read_text(),'malicious config')
        self.assertFalse((overlay['.opencode']/'agent/evil.md').exists())
        policy=c.load_json_strict(overlay['opencode.json'].read_text());self.assertEqual(policy['permission']['*'],'deny');self.assertEqual(policy['permission']['task'],'deny');self.assertEqual(policy['plugin'],[]);self.assertEqual(policy['mcp'],{})
        self.assertEqual(overlay['opencode.json'].stat().st_mode&0o222,0)


class DockerFixture:
    """Injected inspect responses: NEVER evidence of actual container isolation."""
    def __init__(self,mutate=None,missing=False):
        self.calls=[];self.instance=None;self.mutate=mutate;self.missing=missing
    def __call__(self,args):
        self.calls.append(args)
        if self.missing:raise b.SandboxError('observed missing-runtime fixture')
        if args[0]=='info':return c.canonical_json(dict(OSType='linux',ID='daemon-fixture',SecurityOptions=['name=rootless'],CgroupVersion='2',MemoryLimit=True,SwapLimit=True,PidsLimit=True,CpuCfsQuota=True,CpuCfsPeriod=True,Architecture='arm64'))
        if args[:2]==['image','inspect']:return c.canonical_json([dict(Id=args[2],Os='linux',Architecture='arm64',Config=dict(Volumes=None))])
        if args[0]=='create':
            value=lambda key:next(item.split('=',1)[1] for item in args if item.startswith('--'+key+'='))
            labels={args[index+1].split('=',1)[0]:args[index+1].split('=',1)[1] for index,item in enumerate(args) if item=='--label'}
            image=next(item for item in args if item.startswith('sha256:'));image_index=args.index(image)
            mount=value('mount');source=mount.split(',src=')[1].split(',dst=')[0]
            self.instance=dict(Id='a'*64,Image=image,Config=dict(User='10001:10001',Labels=labels,WorkingDir='/candidate',Entrypoint=[value('entrypoint')],Cmd=args[image_index+1:],Env=['HOME=/scratch','TMPDIR=/scratch','GIT_CONFIG_NOSYSTEM=1','GIT_CONFIG_GLOBAL=/dev/null']),HostConfig=dict(ReadonlyRootfs=True,Privileged=False,CapAdd=None,CapDrop=['ALL'],SecurityOpt=['no-new-privileges:true'],NetworkMode='none',PidMode='',IpcMode='none',UTSMode='',UsernsMode='',CgroupnsMode='private',Devices=[],DeviceRequests=[],DeviceCgroupRules=[],VolumesFrom=[],Links=[],ExtraHosts=[],Memory=int(value('memory')),MemorySwap=int(value('memory-swap')),NanoCpus=int(value('cpus'))*1000000000,PidsLimit=int(value('pids-limit')),RestartPolicy=dict(Name='no',MaximumRetryCount=0),LogConfig=dict(Type='none',Config={}),Tmpfs={'/scratch':value('tmpfs').split(':',1)[1]},Ulimits=[dict(Name='nofile',Soft=256,Hard=256),dict(Name='fsize',Soft=1048576,Hard=1048576)]),Mounts=[dict(Type='bind',Source=source,Destination='/candidate',RW=True,Propagation='rprivate')],State=dict(Status='created',Running=False,Paused=False,Restarting=False,Pid=0))
            if self.mutate:self.mutate(self.instance)
            return 'a'*64
        if args[0]=='inspect':return c.canonical_json([self.instance])
        if args[0]=='start':self.instance['State'].update(Status='running',Running=True,Pid=123);return 'a'*64
        if args[0]=='kill':self.instance['State'].update(Status='exited',Running=False,Pid=0);return 'a'*64
        if args[0]=='ps':return 'a'*64
        raise AssertionError(args)


class SandboxFixtureTests(GitIsolationTests):
    """CP06/07/08/10 deterministic boundary decisions; no live certification."""
    # Do not re-run inherited Git tests: this class shares only fixture setup.
    test_CP09_real_independent_clone_no_shared_objects_refs_or_origin=None
    test_worktree_alternate_common_dir_and_unsafe_origin_refuse=None
    test_shared_hardlink_symlink_git_storage_refuse=None
    test_CP11_dirty_diagnostics_and_clean_commit_survive_sanitized_export=None
    test_role_config_overlay_is_external_and_candidate_preserved=None
    def prepare(self,transport=None):
        transport=transport or DockerFixture()
        runtime=b.ContainerSandbox(transport,boot_identity='boot',host_platform='linux',fixture_only=True)
        persisted=[]
        identity=runtime.prepare_fixture_stage(candidate=self.candidate,image='sha256:'+'b'*64,role='IMPLEMENTATION',limits=b.Limits(1,2**30,64,2**30,2**20,2**20,60),command=('/bin/sh','-c','exit 0'),owner_nonce='owner',task_id='task',stage_id='stage',stage_nonce='nonce',lease_epoch=1,persist=persisted.append)
        return runtime,transport,identity,persisted
    def test_CP08_missing_runtime_exact_classification_zero_creates_starts(self):
        transport=DockerFixture(missing=True)
        with self.assertRaises(b.SandboxError) as failure:self.prepare(transport)
        self.assertEqual(failure.exception.code,'M1_ISOLATION_ENVIRONMENT_UNAVAILABLE')
        self.assertEqual([call[0] for call in transport.calls],['info'])
    def test_CP07_six_role_capabilities_and_nonworker_shell_refused(self):
        for role in ('INVESTIGATION_REVIEW','ROUTER_TRIAGE','ARCHITECTURE','DISPATCHER'):
            policy=b.role_policy(role);self.assertEqual(policy['permission']['task'],'deny')
            if role!='VERIFIER_CONTROLLER':self.assertNotEqual(policy['permission'].get('bash'),'allow');self.assertNotEqual(policy['permission'].get('edit'),'allow')
            transport=DockerFixture();runtime=b.ContainerSandbox(transport,boot_identity='boot',host_platform='linux',fixture_only=True)
            with self.assertRaises(c.ContractError):runtime.prepare_fixture_stage(candidate=self.candidate,image='sha256:'+'b'*64,role=role,limits=b.Limits(1,2**30,64,2**30,2**20,2**20,60),command=('/bin/sh',),owner_nonce='owner',task_id='task',stage_id='stage',stage_nonce='nonce',lease_epoch=1,persist=lambda identity:None)
            self.assertEqual(transport.calls,[])
    def test_effective_profile_negative_matrix_refuses_before_start(self):
        mutations=[('Config','User','0'),('HostConfig','Privileged',True),('HostConfig','ReadonlyRootfs',False),('HostConfig','CapAdd',['SYS_ADMIN']),('HostConfig','CapDrop',[]),('HostConfig','PidMode','host'),('HostConfig','NetworkMode','host'),('HostConfig','IpcMode','host'),('HostConfig','Devices',[dict(PathOnHost='/dev/sda')]),('HostConfig','Memory',0),('HostConfig','NanoCpus',0),('HostConfig','PidsLimit',0),('HostConfig','Tmpfs',{}),('Config','Cmd',['unbounded']),('HostConfig','SecurityOpt',[]),('HostConfig','CgroupnsMode','host')]
        for section,key,value in mutations:
            transport=DockerFixture(mutate=lambda instance,section=section,key=key,value=value:instance[section].update({key:value}))
            with self.subTest(section=section,key=key),self.assertRaises(c.ContractError):self.prepare(transport)
            self.assertIn('create',[call[0] for call in transport.calls]);self.assertNotIn('start',[call[0] for call in transport.calls])
        for mount in ('/controller','/host-home','/var/run/docker.sock','/host-git','/host-ssh'):
            transport=DockerFixture(mutate=lambda instance,mount=mount:instance['Mounts'].append(dict(Type='bind',Source=mount,Destination=mount,RW=True)))
            with self.subTest(mount=mount),self.assertRaises(c.ContractError):self.prepare(transport)
            self.assertNotIn('start',[call[0] for call in transport.calls])
    def test_verified_fixture_persists_before_start_and_rechecks_inspection(self):
        runtime,transport,identity,persisted=self.prepare();self.assertEqual(persisted,[identity]);self.assertNotIn('start',[call[0] for call in transport.calls])
        transport.instance['HostConfig']['Privileged']=True
        with self.assertRaises(c.ContractError):runtime.start_stage(identity)
        self.assertNotIn('start',[call[0] for call in transport.calls])
    def test_CP10_reconciliation_kills_entire_recorded_fixture_stage_no_raw_proof(self):
        runtime,transport,identity,_=self.prepare();runtime.start_stage(identity);proof=runtime.reconcile_stage(identity)
        self.assertTrue(proof.fixture_only);self.assertIn('kill',[call[0] for call in transport.calls]);self.assertFalse(transport.instance['State']['Running'])
        with self.assertRaises(c.ContractError):b.validate_termination(proof,identity)
        b.validate_termination(proof,identity,allow_fixture=True)
        with self.assertRaises(c.ContractError):b.TerminatedStage(identity,False)
        runtime.boot_identity='other-boot'
        with self.assertRaises(c.ContractError):runtime.reconcile_stage(identity)
    def test_unknown_liveness_and_wrong_owner_refuse(self):
        runtime,transport,identity,_=self.prepare();transport.instance['Config']['Labels']['or.v2.owner']='forged'
        with self.assertRaises(c.ContractError):runtime.reconcile_stage(identity)
        self.assertNotIn('kill',[call[0] for call in transport.calls])
        runtime,transport,identity,_=self.prepare();transport.instance['State']['Pid']=123
        with self.assertRaises(c.ContractError):runtime.reconcile_stage(identity)
    def test_resource_limits_bool_zero_unbounded_refuse(self):
        for value in (True,0,-1,2**100):
            with self.subTest(value=value),self.assertRaises(c.ContractError):b.Limits(value,2**30,64,2**30,2**20,2**20,60).validate()
    def test_actual_bounded_filesystem_not_inferred_from_plain_directory(self):
        with self.assertRaises(b.SandboxError):b._bounded_candidate_filesystem(self.candidate.root,2**30)
    def test_untrusted_transport_and_raw_authority_cannot_launch(self):
        with self.assertRaises(c.ContractError):b.ContainerSandbox(DockerFixture(),boot_identity='boot',host_platform='linux')
        runtime=b.ContainerSandbox(DockerFixture(),boot_identity='boot',host_platform='linux',fixture_only=True)
        with self.assertRaises(c.ContractError):runtime.prepare_stage(candidate=self.candidate,authority={'authorized_phases':['M1']})

    def test_candidate_seal_host_socket_and_symlink_escape_refuse(self):
        with self.assertRaises(c.ContractError):b.Candidate(self.source,self.base,'FIXTURE_ONLY','nonce')
        escape=self.candidate.root/'escape';escape.symlink_to(Path.home())
        transport=DockerFixture()
        with self.assertRaises(c.ContractError):self.prepare(transport)
        self.assertEqual(transport.calls,[]);escape.unlink()
        import socket
        endpoint=socket.socket(socket.AF_UNIX,socket.SOCK_STREAM)
        self.addCleanup(endpoint.close);endpoint.bind(str(self.candidate.root/'host.sock'))
        with self.assertRaises(c.ContractError):self.prepare(transport)
        self.assertEqual(transport.calls,[])

    def test_image_runtime_credential_namespace_and_termination_refusal(self):
        for section,key,value in [('Config','Env',['GH_TOKEN=fixture-marker']),('HostConfig','DeviceRequests',[{'Count':-1}]),('HostConfig','VolumesFrom',['controller']),('HostConfig','UsernsMode','host'),('HostConfig','LogConfig',dict(Type='json-file',Config={})),('State','Running',True)]:
            transport=DockerFixture(mutate=lambda instance,section=section,key=key,value=value:instance[section].update({key:value}))
            with self.subTest(key=key),self.assertRaises(c.ContractError):self.prepare(transport)
            self.assertIn('create',[call[0] for call in transport.calls]);self.assertNotIn('start',[call[0] for call in transport.calls])
        for field,value in [('OSType','windows'),('SecurityOptions',[]),('CgroupVersion','1'),('MemoryLimit',False)]:
            baseline=DockerFixture()
            def transport(args,field=field,value=value):
                result=baseline(args)
                if args[0]=='info':document=c.load_json_strict(result);document[field]=value;return c.canonical_json(document)
                return result
            with self.subTest(field=field),self.assertRaises(c.ContractError):self.prepare(transport)
            self.assertNotIn('create',[call[0] for call in baseline.calls])

    def test_other_live_stage_instance_refuses_termination_proof(self):
        runtime,transport,identity,_=self.prepare()
        original=transport.__call__
        def another(args):
            if args[0]=='ps':return 'a'*64+'\n'+'c'*64
            if args[0]=='inspect' and args[1]=='c'*64:
                instance=copy.deepcopy(transport.instance);instance['Id']='c'*64;instance['State'].update(Running=True,Pid=999)
                return c.canonical_json([instance])
            return original(args)
        runtime.docker=another
        with self.assertRaises(c.ContractError):runtime.reconcile_stage(identity)


if __name__=='__main__':unittest.main(verbosity=2)
