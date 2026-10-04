#!/usr/bin/env python3
"""Opt-in REAL rootless Debian CP07 × CP10 × CP11 crash acceptance.

Requires the already prepared dedicated ext4 volume and content-pinned image.
No mocks, installations, model dispatch, product work or M2 authority. Evidence
and preserved dirty/clean results remain in --output for independent inspection.
"""
from __future__ import annotations
import argparse
import copy
import hashlib
import json
import multiprocessing
import os
from pathlib import Path
import shlex
import shutil
import signal
import subprocess
import sys
import time
import uuid

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / 'scripts'))
from model_orchestrator import contracts as c, sandbox as b, store as s, workspace as w
from model_orchestrator.tests.test_contracts import ProvenanceFixture, valid_task

CTX = multiprocessing.get_context('fork')
POINTS = ('after_reservation', 'after_workspace', 'after_create', 'after_persistence',
          'running_dirty', 'running_clean', 'removed_dirty', 'exited_dirty', 'after_reconciliation', 'after_preservation', 'during_cleanup', 'after_cleanup_create', 'after_cleanup_start', 'after_cleanup_remove')


def runtime(docker_path, image):
    cli = b.DockerCLI(docker_path, hashlib.sha256(docker_path.read_bytes()).hexdigest(),
                      endpoint='unix:///run/user/' + str(os.getuid()) + '/docker.sock')
    box = b.ContainerSandbox(cli, boot_identity=b.host_boot_identity(), host_platform='linux')
    box._runtime(image)
    return box


def git(root, *args):
    return subprocess.check_output(['git', '-c', 'user.name=Worker', '-c', 'user.email=worker@example.invalid', *args], cwd=root).decode().strip()


def controller(root, authority, candidate, image, docker_path, task_id, digest, overlays, point, pipe):
    store = s.RuntimeStore(root, authority)
    box = runtime(docker_path, image)
    with store.lock('task', task_id):
        stage = store.claim(task_id, digest, owner_nonce=uuid.uuid4().hex, boot_identity=box.boot_identity,
                            stage_id=uuid.uuid4().hex, stage_nonce=uuid.uuid4().hex)
        def fault(boundary):
            if point == boundary:
                pipe.send(dict(boundary=boundary, pid=os.getpid(), container_id=store.inspect()['tasks'][task_id]['launch']['container_id'])); signal.pause()
        config_checks = ("test \"$(cat opencode.json)\" = " + shlex.quote(c.canonical_json(b.role_policy('IMPLEMENTATION'))) +
                         "; test ! -e .opencode/agents/hostile; test ! -w /worker-home/opencode.json; "
                         "if printf evil > opencode.json 2>/dev/null; then exit 41; fi; "
                         "if mkdir .opencode/evil 2>/dev/null; then exit 42; fi; ")
        edits = ("printf 'worker committed change\\n' > note.txt; "
                 "git -c safe.directory=/candidate -c core.hooksPath=/dev/null add note.txt; "
                 "git -c safe.directory=/candidate -c core.hooksPath=/dev/null -c user.name=FixtureWorker "
                 "-c user.email=worker@example.invalid commit -qm 'real worker local commit'") if point == 'running_clean' else "printf 'worker dirty change\\n' > note.txt; printf 'worker new file\\n' > new.txt"
        end = 'exit 0' if point.startswith('exited') or point in ('after_reconciliation','after_preservation','during_cleanup') or point.startswith('after_cleanup') else 'sleep 120'
        command = ('/bin/sh', '-ec', config_checks + edits + '; printf ready > .git/worker-ready; ' + end)
        identity = box.prepare_fixture_stage(candidate=candidate, image=image, role='IMPLEMENTATION',
                limits=b.Limits(1, 1 << 30, 64, 48 << 30, 1 << 20, 1 << 20, 180), command=command,
                owner_nonce=stage['owner_nonce'], stage_id=stage['stage_id'], lease_epoch=stage['lease_epoch'],
                task_id=task_id, stage_nonce=stage['stage_nonce'], persist=lambda identity:None,
                overlays=overlays, store=store, stage=stage, fault=fault)
        box.start_stage(identity)
        stage = store.inspect()['tasks'][task_id]['stage']
        view = w.restore_launch(store, stage).root
        deadline=time.monotonic()+20
        while not (view/'.git/worker-ready').exists():
            if time.monotonic()>deadline:raise AssertionError('real worker did not produce useful work')
            time.sleep(0.05)
        if point.startswith('exited') or point in ('after_reconciliation','after_preservation','during_cleanup') or point.startswith('after_cleanup'):
            deadline=time.monotonic()+20
            while box._inspect(identity)['State']['Running']:
                if time.monotonic()>deadline:raise AssertionError('real worker did not exit')
                time.sleep(0.05)
        if point in ('after_reconciliation','after_preservation','during_cleanup') or point.startswith('after_cleanup'):
            proof=box.reconcile_launch(store,stage)
        if point in ('after_preservation','during_cleanup') or point.startswith('after_cleanup'):
            w.preserve_launch(store,stage,proof)
        if point == 'during_cleanup':
            store.settle(stage,proof)
            # Crash after durable cleanup eligibility, before removal.
            record=store.launch_record(stage);record['phase']='CLEANING';store.update_launch(stage,record)
        if point.startswith('after_cleanup'):
            store.settle(stage,proof)
            w.cleanup_launch(store,stage,box,proof,fault=fault)
            raise AssertionError('cleanup fault was not injected')
        worker_commit=(view/'.git/refs/heads/candidate').read_text().strip() if point=='running_clean' else None
        pipe.send(dict(boundary=point,pid=os.getpid(),container_id=identity.container_id,worker_commit=worker_commit));signal.pause()


def recover(root, authority, task_id, image, docker_path, expected_head, point, pipe):
    store=s.RuntimeStore(root,authority);box=runtime(docker_path,image)
    assert box._prepared == {}
    with store.lock('task',task_id):
        state=store.inspect();task=state['tasks'][task_id]
        stage=task['stage'] or dict(task['launch']['stage'],container_id=task['launch']['container_id'])
        record=store.launch_record(stage)
        for key,value in (('task_id','other-task'),('stage_nonce','forged-nonce'),('lease_epoch',stage['lease_epoch']+1),
                          ('owner_nonce','wrong-owner'),('stage_id','wrong-stage')):
            try:w.restore_launch(store,dict(stage,**{key:value}))
            except c.ContractError:pass
            else:raise AssertionError('wrong durable identity admitted: '+key)
        try:
            store.claim(task_id,task['contract_digest'],owner_nonce='other',boot_identity=box.boot_identity,stage_id='duplicate',stage_nonce='duplicate')
        except c.ContractError:pass
        else:raise AssertionError('lease reused before reconciliation')
        proof=box.reconcile_launch(store,stage)
        if record['phase'] in ('RESERVED', 'ABORTING'):
            w.abort_unstarted_launch(store,stage,box,proof)
            final=store.launch_record(stage)
            assert final['phase']=='ABORTED' and not Path(final['path']).exists()
            assert w.restore_preserved(store,stage).root==Path(final['input']['root'])
            pipe.send(dict(task_id=task_id,stage=stage,workspace=final['path'],result=dict(original_input=final['input']),
                           phase=final['phase'],recovery_pid=os.getpid(),prepared_count=0))
            return
        if store.inspect()['tasks'][task_id]['stage'] is not None:
            stage=store.inspect()['tasks'][task_id]['stage']
        view=w.restore_launch(store,stage).root if record['phase']!='CLEANING' else None
        if point == 'running_clean':
            pack=root/'artifacts'/'verified-worker.pack'
            head=b.export_candidate(w.restore_launch(store,stage),pack)
            assert head==expected_head
        if point in ('running_dirty','removed_dirty','exited_dirty','after_reconciliation','after_preservation'):
            assert (view/'note.txt').read_text()=='worker dirty change\n'
            assert (view/'new.txt').read_text()=='worker new file\n'
        if record['phase'] != 'CLEANING':
            try:w.cleanup_launch(store,stage,box,proof)
            except c.ContractError:pass
            else:raise AssertionError('cleanup before useful handoff/settlement')
            w.preserve_launch(store,stage,proof)
            store.settle(stage,proof)
        # Preserve exact container binding after settlement in launch receipt.
        w.cleanup_launch(store,stage,box,proof)
        restored=w.restore_preserved(store,stage)
        if point=='running_clean':
            pack=root/'artifacts'/'after-cleanup.pack';head=b.export_candidate(restored,pack)
            assert head==expected_head
            assert git(restored.root,'rev-parse','HEAD')==expected_head
            assert git(restored.root,'status','--porcelain') == ''
            # Continue candidate verification in a fresh sanitized repository;
            # import the existing worker commit, never create a recovery commit.
            imported = root / 'artifacts' / 'clean-import'
            imported.mkdir()
            b._git(imported, 'init', '--initial-branch=candidate')
            b._git(imported, 'index-pack', '--strict', '--stdin', input_bytes=pack.read_bytes())
            b._git(imported, 'fsck', '--full', '--strict', '--no-reflogs')
            b._git(imported, 'update-ref', 'refs/heads/candidate', expected_head)
            b._git(imported, 'checkout', 'candidate')
            assert git(imported, 'rev-parse', 'HEAD') == expected_head
            assert git(imported, 'status', '--porcelain') == ''
        final=store.launch_record(stage)
        assert final['phase']=='REMOVED' and not Path(final['path']).exists()
        assert not box.docker(['ps','--all','--no-trunc','--filter','name=^/'+final['container_name']+'$','--format','{{.ID}}']).strip()
        pipe.send(dict(task_id=task_id,stage=stage,workspace=final['path'],result=final['result'],
                       phase=final['phase'],recovery_pid=os.getpid(),prepared_count=len(box._prepared)))


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--volume',type=Path,required=True);parser.add_argument('--output',type=Path,required=True)
    parser.add_argument('--docker',type=Path,default=Path('/usr/bin/docker'))
    parser.add_argument('--image',required=True,help='exact pinned Linux image with GNU timeout, find/rm and Git')
    args=parser.parse_args();args.output.mkdir(mode=0o700,parents=True,exist_ok=False)
    box=runtime(args.docker,args.image)
    (args.output/'authority').mkdir()
    fixture=ProvenanceFixture(args.output/'authority');authority=fixture.authorities['M1']
    source=args.output/'source';source.mkdir();git(source,'init','-q','--initial-branch=candidate')
    (source/'note.txt').write_text('original input\n');(source/'opencode.json').write_text('{"permission":{"*":"allow"}}')
    (source/'.opencode/agents').mkdir(parents=True);(source/'.opencode/agents/hostile').write_text('evil')
    git(source,'add','.');git(source,'commit','-qm','independent fixture input');base=git(source,'rev-parse','HEAD')
    rows=[]
    for point in POINTS:
        case=args.output/point;case.mkdir();root=case/'runtime';store=s.RuntimeStore(root,authority);store.initialize()
        task=valid_task();payload=c._release_authority(authority);task.update(task_id=payload['build']['task_id'],checkpoint_id='M1',base_sha=payload['build']['base_sha'],candidate_branch=payload['build']['candidate_branch'],authority_digest=payload['git']['authority_digest'])
        digest=store.register_task(task,copy.deepcopy(task));task_id=task['task_id']
        # The operator owns the bounded mount; create only this fresh input.
        input_name='closure-'+uuid.uuid4().hex;input_path=args.volume/input_name
        (case/'input-locator.json').write_text(json.dumps(dict(path=str(input_path))))
        input_path.mkdir(mode=0o700)
        candidate=b.create_fixture_candidate(source,input_path,base)
        quota=b._bounded_candidate_filesystem(input_path,48<<30);before=w.manifest(input_path)
        overlays=b.write_role_overlays(case/'overlays','IMPLEMENTATION')
        parent,child=CTX.Pipe();proc=CTX.Process(target=controller,args=(root,authority,candidate,args.image,args.docker,task_id,digest,overlays,point,child));proc.start()
        if not parent.poll(50):raise AssertionError('controller failed before fault boundary: '+point)
        event=parent.recv();os.kill(proc.pid,signal.SIGKILL);proc.join(10);assert proc.exitcode == -signal.SIGKILL
        if point == 'removed_dirty':
            box.docker(['kill',event['container_id']]);box.docker(['rm',event['container_id']])
        fresh_parent,fresh_child=CTX.Pipe();fresh=CTX.Process(target=recover,args=(root,authority,task_id,args.image,args.docker,event.get('worker_commit'),point,fresh_child));fresh.start();fresh.join(50)
        if fresh.exitcode!=0:raise AssertionError('fresh recovery failed: '+point+' exit='+str(fresh.exitcode))
        assert fresh_parent.poll(2);result=fresh_parent.recv()
        assert w.manifest(input_path)==before,'input candidate content/modes changed'
        row=dict(case=point,outcome='PASS',controller_event=event,controller_exit=proc.exitcode,
                 filesystem=dict(device=quota[0],capacity=quota[1],mount=quota[2]),original_manifest=before,**result)
        rows.append(row);(case/'evidence.json').write_text(json.dumps(row,indent=2)+'\n')
        print(json.dumps(row),flush=True)
        # Useful work and evidence are durable; remove only the unchanged,
        # controller-owned disposable input, never the bounded mount root.
        assert input_path.parent == args.volume and input_path.name == input_name
        assert input_path.stat().st_uid == os.getuid()
        shutil.rmtree(input_path)
    (args.output/'acceptance.json').write_text(json.dumps(dict(cases=rows,result='PASS',scope='disabled M1 inert fixture; no M2 authority'),indent=2)+'\n')


if __name__=='__main__':main()
