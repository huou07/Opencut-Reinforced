"""Disabled M2 factual verification in a fresh credentialless rootless verifier.

Approved Git-pinned harnesses and controller observations, never worker PASS
files. Attempts retain bounded receipts/logs; exact-name recovery fences crashes.
There is deliberately no review, promotion, model transport or supervisor path.
execute requires sealed M2 guard facts and explicit lease/sequence fencing. It
persists each exact stage locator before materialization/create, mounts fresh
candidate/harness copies read-only, checks the effective M1 isolation profile,
and stores actual process/cgroup observations bound by strict M0 receipt hashes.
Receipts precede whole-stage reconciliation and copy removal. An unresolved
cleanup stops as RECOVERY_REQUIRED; recover_attempt reopens only the recorded
name/ID on the same daemon/boot and never reruns an interrupted check. Required
checks with no result become NOT RUN, never PASS. Logs/samples obey frozen
output/time/sample limits; retained bounded facts never confer later authority.
readiness revalidates floor/candidate/receipt/log identities. Semantic flags
remain unresolved, and missing real acceptance classes cannot be replaced by
unit results. Frozen hosted-only obligations are explicitly pending.
"""
from __future__ import annotations

import copy
import hashlib
import math
import os
from pathlib import Path
import re
import selectors
import shutil
import signal
import subprocess
import time
import uuid

from . import contracts as c, sandbox as b, store as s, workspace as w, guards as g


def case_results(data, required):
    """Only the frozen harness's complete protocol, not a candidate PASS file."""
    try:
        record=c.load_json_strict(data.decode('utf-8'))
    except (UnicodeError,c.ContractError) as exc:
        raise c.ContractError('malformed frozen harness result') from exc
    s._exact(record,('cases',))
    g.require(type(record['cases']) is list,'case result list required')
    seen={}
    for row in record['cases']:
        s._exact(row,('id','result'))
        g.require(type(row['id']) is str and row['id'] in required and row['id'] not in seen and
                  row['result'] in ('PASS','FAIL','SKIP'),'unknown/duplicate case result')
        seen[row['id']]=row['result']
    g.require(set(seen)==set(required),'missing baseline-required cases; count is not identity')
    return dict(executed_cases=[k for k,v in seen.items() if v!='SKIP'],
                passed_cases=[k for k,v in seen.items() if v=='PASS'],
                failed_cases=[k for k,v in seen.items() if v=='FAIL'],
                skipped_cases=[k for k,v in seen.items() if v=='SKIP'])


def memory_bytes(value):
    match=re.fullmatch(r'([0-9]+(?:\.[0-9]+)?)\s*(B|KiB|MiB|GiB|kB|MB|GB)\s*/\s*.+',value)
    g.require(match is not None,'unsupported cgroup memory observation')
    units={'B':1,'KiB':1024,'MiB':1024**2,'GiB':1024**3,'kB':1000,'MB':1000**2,'GB':1000**3}
    return float(match[1])*units[match[2]]


def _context(floor, head, check, attempt):
    task=floor.task
    return dict(frozen_task=task,task_id=task['task_id'],candidate_sha=head,
                authority_digest=task['authority_digest'],task_contract_digest=c.canonical_digest(task),
                command_digest=c.canonical_digest(check),environment_digest=check['environment_digest'],
                lease_epoch=attempt['lease_epoch'],sequence=attempt['sequence'],stage_nonce=attempt['nonce'])


def _receipt(floor,head,check,attempt):
    return dict(schema_version=1,check_id=check['id'],exit_code=None,outcome='NOT RUN',
                executed_cases=[],passed_cases=[],failed_cases=[],skipped_cases=[],artifact_digest=None,
                signal=None,timed_out=False,executable_found=False,duration_seconds=0,measurements=[],
                **{k:v for k,v in _context(floor,head,check,attempt).items() if k!='frozen_task'})


def _runtime(binding):
    g.require(binding['boundary']=='rootless','required local environment unavailable; no hosted substitution')
    g.require(os.uname().sysname=='Linux','LOCAL ENVIRONMENT BLOCKED: verified Linux rootless engine required')
    cli=b.DockerCLI(Path(binding['docker']),binding['docker_digest'],endpoint=binding['endpoint'])
    box=b.ContainerSandbox(cli,boot_identity=b.host_boot_identity(),host_platform='linux')
    daemon=box._runtime(binding['image'])
    return box,daemon


def _identity(record, container):
    return b.StageIdentity(container,record['boot'],record['nonce'],record['stage_id'],record['lease_epoch'],
                           record['nonce'],record['authority_digest'],'VERIFIER_CONTROLLER',record['task_id'])


def _expected(check,binding,path):
    limits=check['resource_limits']
    g.require(set(check['required_metrics'])<={'memory_bytes','duration_seconds'},'required resource observer unavailable')
    g.require(type(limits['cpu']) is int and 1<=limits['cpu']<=64 and limits['pids']<=4096,
              'required resource profile unavailable')
    command=('/usr/bin/timeout','--signal=KILL','--kill-after=1',str(check['timeout_seconds']),*check['argv'])
    return dict(image=binding['image'],mounts=[(str(path/'candidate'),'/candidate',False),
                    (str(path/'harness'),'/verifier',False)],workdir='/candidate'+('' if check['cwd']=='.' else '/'+check['cwd']),
                limits=b.Limits(limits['cpu'],limits['memory_bytes'],limits['pids'],limits['disk_bytes'],
                                min(limits['disk_bytes'],1<<20),limits['output_bytes'],math.ceil(limits['wall_seconds'])),
                command=command,environment=[k+'='+v for k,v in sorted(check['environment'].items())],created_only=True)


def _readonly(path):
    for directory,_,files in os.walk(path,topdown=False):
        for name in files:
            p=Path(directory)/name;p.chmod(0o555 if p.stat().st_mode&0o111 else 0o444)
        Path(directory).chmod(0o555)


def _remove_copy(path):
    if path.exists():
        for directory,_,_ in os.walk(path):Path(directory).chmod(0o700)
        shutil.rmtree(path)


def _persist_failure(path,floor,head,check,attempt,message,outcome='LOCAL ENVIRONMENT BLOCKED'):
    receipt=_receipt(floor,head,check,attempt);receipt['outcome']=outcome
    artifact=dict(error=message,check_id=check['id'],attempt_nonce=attempt['nonce'])
    g.write_json(path/'observation.json',artifact);receipt['artifact_digest']=c.canonical_digest(artifact)
    c.validate_verification_receipt(receipt,floor.verify()['schemas'],task=floor.task,candidate_sha=head,
                                    context=_context(floor,head,check,attempt))
    g.write_json(path/'receipt.json',receipt);return receipt


def execute(guarded,destination,*,lease_epoch,sequence,fault=None):
    """One new bounded attempt; a restart recovers it, never silently reruns it."""
    g.require(type(guarded) is g.Guarded,'sealed controller guard facts required')
    guard=guarded.verify();floor=guarded.floor;task=floor.task
    g.require(not guard['vetoes'],'deterministic guard veto prevents verification')
    g.require(type(lease_epoch) is int and lease_epoch>0 and type(sequence) is int and sequence>0,'attempt fencing required')
    destination=b._safe_path(destination);g.require(not destination.exists(),'existing/stale verifier attempt cannot be reused')
    for root in (guarded.root,*c.authority_roots(floor.authority)):
        g.require(root!=destination and root not in destination.parents and destination not in root.parents,'attempt overlaps authority/input')
    destination.mkdir(mode=0o700,parents=True);s._fsync_dir(destination.parent)
    attempt=dict(schema_version=1,nonce=uuid.uuid4().hex,task_id=task['task_id'],floor_digest=floor.digest,
                 guard_digest=c.canonical_digest(guard),head=guard['head'],lease_epoch=lease_epoch,
                 sequence=sequence,phase='RESERVED',checks={},receipts=[])
    entry=destination.stat();attempt['directory_identity']=[entry.st_dev,entry.st_ino]
    g.write_json(destination/'attempt.json',attempt)
    hit=fault or (lambda point:None);hit('reserved')
    started=time.monotonic()
    for check in task['check_argv']:
        check_path=destination/check['id'];check_path.mkdir(mode=0o700)
        if time.monotonic()-started>=task['budget']['wall_seconds']:
            receipt=_persist_failure(check_path,floor,guard['head'],check,attempt,'frozen total wall budget exhausted','NOT RUN')
        else:
            receipt=_execute_one(guarded,check,check_path,attempt,destination,hit)
        attempt['receipts'].append(receipt);attempt['phase']='VERIFYING';g.write_json(destination/'attempt.json',attempt)
    attempt['phase']='COMPLETE';g.write_json(destination/'attempt.json',attempt);hit('complete')
    return readiness(guarded,destination)


def _execute_one(guarded,check,path,attempt,destination,hit):
    floor=guarded.floor;guard=guarded.verify();binding=floor.catalog['checks'][check['id']]
    head=guard['head'];box=None;identity=None
    try:
        harness=g.controller_blob(floor.authority,binding['harness'])
        g.require(hashlib.sha256(harness).hexdigest()==check['harness_digest']==binding['harness_digest'],'missing/wrong frozen harness identity')
        box,daemon=_runtime(binding)
        expected=_expected(check,binding,path)
        # Persist all locators BEFORE materializing mutable copies or Docker create.
        record=dict(nonce=attempt['nonce'],task_id=floor.task['task_id'],authority_digest=floor.task['authority_digest'],
                    lease_epoch=attempt['lease_epoch'],stage_id=uuid.uuid4().hex,boot=box.boot_identity,
                    daemon=daemon,name='or-m2-'+uuid.uuid4().hex,container_id=None,phase='RESERVED',binding=binding,
                    manifest=None,harness_manifest=None)
        attempt['checks'][check['id']]=record;g.write_json(destination/'attempt.json',attempt)
        shutil.copytree(guarded.root/'candidate',path/'candidate')
        target=path/'harness'/binding['harness'];target.parent.mkdir(mode=0o700,parents=True);target.write_bytes(harness)
        _readonly(path/'candidate');_readonly(path/'harness');w.flush_tree(path/'candidate');w.flush_tree(path/'harness')
        record['manifest']=w.manifest(path/'candidate');record['harness_manifest']=w.manifest(path/'harness')
        record['phase']='BOUND';g.write_json(destination/'attempt.json',attempt);hit('materialized')
        limits=expected['limits']
        args=['create','--name='+record['name'],'--pull=never','--read-only','--user=10001:10001','--cap-drop=ALL',
              '--security-opt=no-new-privileges:true','--network=none','--ipc=none','--cgroupns=private',
              '--pids-limit='+str(limits.pids),'--cpus='+str(limits.cpu),'--memory='+str(limits.memory_bytes),
              '--memory-swap='+str(limits.memory_bytes),'--workdir='+expected['workdir'],'--log-driver=none','--restart=no',
              '--tmpfs=/scratch:rw,nosuid,nodev,noexec,size='+str(limits.scratch_bytes)+',mode=1777',
              '--ulimit=nofile=256:256','--ulimit=fsize='+str(limits.output_bytes)+':'+str(limits.output_bytes)]
        for source,target,rw in expected['mounts']:args.append('--mount=type=bind,src='+source+',dst='+target+',readonly')
        for env in expected['environment']:args.append('--env='+env)
        for key,value in _identity(record,'0'*64).labels().items():args.extend(['--label',key+'='+value])
        args.extend(['--entrypoint='+expected['command'][0],binding['image'],*expected['command'][1:]])
        container=box.docker(args).strip();g.require(b._CONTAINER.fullmatch(container),'invalid verifier container identity')
        hit('created')
        record['container_id']=container;identity=_identity(record,container)
        record['phase']='CREATED';g.write_json(destination/'attempt.json',attempt)
        box.verify_effective(identity,expected)
        before=(w.manifest(path/'candidate'),w.manifest(path/'harness'))
        record['phase']='RUNNING';g.write_json(destination/'attempt.json',attempt)
        receipt=_receipt(floor,head,check,attempt);receipt['executable_found']=True
        observation=_observe(box,identity,check,binding,path,hit)
        receipt.update(exit_code=observation['exit_code'],signal=observation['signal'],timed_out=observation['timed_out'],
                       duration_seconds=observation['duration_seconds'],outcome='FAIL')
        if observation['exit_code']==127:
            receipt.update(executable_found=False,outcome='LOCAL ENVIRONMENT BLOCKED')
        try:
            receipt.update(case_results((path/'stdout.log').read_bytes(),check['required_cases']))
            after=(w.manifest(path/'candidate'),w.manifest(path/'harness'))
            g.require(before==after,'candidate/harness mutation during verification')
            guarded.verify();floor.verify()
            metrics=_measurements(floor,check,observation)
            receipt['measurements']=metrics
            if (receipt['exit_code'] in check['expected_exit_codes'] and not receipt['signal'] and not receipt['timed_out']
                    and not observation['output_overflow'] and not receipt['failed_cases'] and not receipt['skipped_cases']):
                receipt['outcome']='PASS'
                try:
                    c.validate_verification_receipt(receipt,floor.verify()['schemas'],task=floor.task,candidate_sha=head,
                                                   context=_context(floor,head,check,attempt))
                except c.ContractError as exc:
                    receipt['outcome']='FAIL';observation['failure']=str(exc)
        except (c.ContractError,OSError) as exc:
            observation['failure']=str(exc)
        observation.update(candidate_manifest=record['manifest'],harness_manifest=record['harness_manifest'],
                           argv=check['argv'],cwd=check['cwd'],environment=check['environment'],
                           executable_identity=dict(path=binding['executable'],image=binding['image']),
                           harness_digest=check['harness_digest'],candidate_sha=head,
                           stdout_digest=g.digest_file(path/'stdout.log'),stderr_digest=g.digest_file(path/'stderr.log'))
        if floor.task['performance_applicability']=='applicable':
            observation['performance_floor']=dict(baseline=floor.catalog['performance']['baseline'],method=floor.catalog['performance']['method'],
                budgets=floor.task['performance_budgets'],environment_digest=check['environment_digest'],package_identity=binding['image'])
        g.write_json(path/'observation.json',observation);receipt['artifact_digest']=c.canonical_digest(observation)
        c.validate_verification_receipt(receipt,floor.verify()['schemas'],task=floor.task,candidate_sha=head,
                                        context=_context(floor,head,check,attempt))
        # Receipt precedes cleanup. A killed controller retains exact name/ID.
        g.write_json(path/'receipt.json',receipt);record['phase']='RECEIPTED';g.write_json(destination/'attempt.json',attempt)
        hit('receipted')
        box.reconcile_stage(identity);box.docker(['rm',container])
        _remove_copy(path/'candidate');_remove_copy(path/'harness')
        record['phase']='REMOVED';g.write_json(destination/'attempt.json',attempt);return receipt
    except (c.ContractError,OSError,subprocess.SubprocessError) as exc:
        # Record availability/failure before exact reconciliation. A failure to
        # prove death keeps the reservation and stops; it cannot become COMPLETE.
        receipt=_persist_failure(path,floor,head,check,attempt,str(exc))
        record=attempt['checks'].get(check['id'])
        if record is not None and record['phase']!='REMOVED':
            try:
                _cleanup_check(floor,check,path,record)
                _remove_copy(path/'candidate');_remove_copy(path/'harness')
                record['phase']='REMOVED';g.write_json(destination/'attempt.json',attempt)
            except (c.ContractError,OSError,subprocess.SubprocessError) as cleanup_error:
                attempt['phase']='RECOVERY_REQUIRED';g.write_json(destination/'attempt.json',attempt)
                raise c.ContractError('exact verifier reconciliation required: '+str(cleanup_error)) from cleanup_error
        return receipt


def _observe(box,identity,check,binding,path,hit):
    """Actual attach process, cgroup samples and daemon-observed termination."""
    cli=box.docker
    args=[str(cli.executable),'--host',cli.endpoint,'start','--attach',identity.container_id]
    start=time.monotonic();wall_start=time.time();output=0;overflow=False;timed_out=False;samples=[]
    proc=subprocess.Popen(args,env={'PATH':'/usr/bin:/bin','LANG':'C'},stdout=subprocess.PIPE,stderr=subprocess.PIPE,start_new_session=True)
    hit('running')
    selector=selectors.DefaultSelector()
    for stream,name in ((proc.stdout,'stdout.log'),(proc.stderr,'stderr.log')):
        os.set_blocking(stream.fileno(),False);selector.register(stream,selectors.EVENT_READ,name)
    handles={name:(path/name).open('wb') for name in ('stdout.log','stderr.log')};last_sample=0
    try:
        while selector.get_map() or proc.poll() is None:
            elapsed=time.monotonic()-start
            if elapsed>check['timeout_seconds'] or overflow:
                timed_out=not overflow
                box.reconcile_stage(identity)
                if proc.poll() is None:os.killpg(proc.pid,signal.SIGKILL)
            for key,_ in selector.select(0.02):
                data=os.read(key.fileobj.fileno(),65536)
                if not data:selector.unregister(key.fileobj);continue
                remaining=max(0,check['resource_limits']['output_bytes']-output)
                handles[key.data].write(data[:remaining]);output+=len(data)
                if output>check['resource_limits']['output_bytes']:overflow=True
            if check['required_metrics'] and elapsed-last_sample>=0.1 and proc.poll() is None and not timed_out:
                result=box._json(['stats','--no-stream','--format','{{json .}}',identity.container_id])
                samples.append(dict(elapsed=time.monotonic()-start,memory_bytes=memory_bytes(result['MemUsage'])))
                g.require(len(samples)<=4096,'measurement sample bound');last_sample=time.monotonic()-start
        proc.wait(timeout=5)
    finally:
        selector.close()
        for handle in handles.values():handle.flush();os.fsync(handle.fileno());handle.close()
        for stream in (proc.stdout,proc.stderr):stream.close()
        if proc.poll() is None:
            box.reconcile_stage(identity);os.killpg(proc.pid,signal.SIGKILL);proc.wait(timeout=5)
    instance=box._inspect(identity);state=instance['State']
    g.require(state.get('Running') is False and state.get('Pid')==0 and not state.get('Paused') and
              not state.get('Restarting'),'verifier entire-process death unproved')
    code=state['ExitCode'];signum=code-128 if 128<code<193 else None
    elapsed=time.monotonic()-start
    timed_out=timed_out or elapsed>check['timeout_seconds'] or code==124
    return dict(process_pid=proc.pid,container_id=identity.container_id,boot=box.boot_identity,
                start_unix=wall_start,end_unix=time.time(),duration_seconds=elapsed,
                exit_code=code,attach_exit=proc.returncode,signal=signum,timed_out=timed_out,
                output_overflow=overflow,samples=samples,image=binding['image'],daemon=box._json(['info','--format','{{json .}}'])['ID'])


def _cleanup_check(floor,check,path,record):
    binding=floor.catalog['checks'][check['id']]
    g.require(binding==record['binding'],'verifier binding drift')
    box,daemon=_runtime(binding)
    g.require(daemon==record['daemon'] and box.boot_identity==record['boot'],'verifier daemon/boot changed')
    ids=box.docker(['ps','--all','--no-trunc','--filter','name=^/'+record['name']+'$','--format','{{.ID}}']).splitlines()
    g.require(len(ids)<=1 and all(b._CONTAINER.fullmatch(x) for x in ids),'ambiguous verifier reservation')
    if ids:
        g.require(record['container_id'] in (None,ids[0]),'verifier name reused')
        identity=_identity(record,ids[0]);expected=_expected(check,binding,path);expected['created_only']=False
        box.verify_effective(identity,expected);box.reconcile_stage(identity);box.docker(['rm',ids[0]])
    elif record['container_id']:
        g.require(not box.docker(['ps','--all','--no-trunc','--filter','id='+record['container_id'],'--format','{{.ID}}']).strip(),
                  'verifier renamed; cleanup refused')


def _attempt(guarded,destination):
    guard=guarded.verify();floor=guarded.floor;destination=b._safe_path(destination)
    attempt=c.load_json_strict((destination/'attempt.json').read_text())
    entry=destination.stat()
    g.require(attempt['directory_identity']==[entry.st_dev,entry.st_ino] and
              attempt['floor_digest']==floor.digest and attempt['guard_digest']==c.canonical_digest(guard) and
              attempt['head']==guard['head'] and attempt['task_id']==floor.task['task_id'],
              'stale/foreign verifier attempt')
    g.require(type(attempt['lease_epoch']) is int and attempt['lease_epoch']>0 and
              type(attempt['sequence']) is int and attempt['sequence']>0 and
              re.fullmatch('[0-9a-f]{32}',attempt['nonce']) and
              set(attempt['checks'])<=set(floor.task['required_check_ids']),'invalid attempt fencing')
    for record in attempt['checks'].values():
        g.require(record['nonce']==attempt['nonce'] and record['task_id']==attempt['task_id'] and
                  record['lease_epoch']==attempt['lease_epoch'] and record['authority_digest']==floor.task['authority_digest'] and
                  re.fullmatch('or-m2-[0-9a-f]{32}',record['name']) and
                  re.fullmatch('[0-9a-f]{32}',record['stage_id']), 'check reservation fencing differs')
    return attempt


def _measurements(floor,check,observation):
    task=floor.task;perf=task['performance_budgets'];catalog=floor.catalog['performance']
    if not check['required_metrics']:return []
    g.require(perf['applicability']=='applicable','worker cannot waive applicable performance')
    s._exact(catalog,('baseline','method'))
    g.require(c.canonical_digest(catalog['baseline'])==perf['baseline_digest'] and
              c.canonical_digest(catalog['method'])==perf['method_digest'],'performance baseline/method mismatch')
    g.require(observation['duration_seconds']>=perf['duration_seconds'] and len(observation['samples'])>=perf['sample_count'],
              'insufficient sustained measurement duration/samples')
    for bound in perf['bounds']:
        g.require(catalog['baseline'].get(bound['metric'])==bound['baseline_maximum'],'baseline bound identity differs')
    observed={'memory_bytes':max(row['memory_bytes'] for row in observation['samples']),
              'duration_seconds':observation['duration_seconds']}
    g.require(set(check['required_metrics'])<=set(observed),'required resource observer unavailable')
    return [dict(metric=metric,value=observed[metric]) for metric in check['required_metrics']]


def recover_attempt(guarded,destination):
    """No process-local memory, PID liveness guesses, scan or automatic rerun."""
    guard=guarded.verify();floor=guarded.floor;destination=b._safe_path(destination)
    attempt=_attempt(guarded,destination)
    for check in floor.task['check_argv']:
        path=destination/check['id'];record=attempt['checks'].get(check['id'])
        path.mkdir(mode=0o700,exist_ok=True)
        if record is not None and record['phase']!='REMOVED':_cleanup_check(floor,check,path,record)
        # Missing receipt never becomes PASS. Retain interruption facts and copies
        # until the exact entire stage is absent and an immutable receipt exists.
        if not (path/'receipt.json').exists():
            receipt=_persist_failure(path,floor,guard['head'],check,attempt,'controller interrupted; exact stage reconciled','NOT RUN')
        else:receipt=c.load_json_strict((path/'receipt.json').read_text())
        attempt['receipts']=[r for r in attempt['receipts'] if r['check_id']!=check['id']]+[receipt]
        g.write_json(destination/'attempt.json',attempt)
        _remove_copy(path/'candidate');_remove_copy(path/'harness')
        if record is not None:record['phase']='REMOVED'
        g.write_json(destination/'attempt.json',attempt)
    attempt['phase']='COMPLETE';g.write_json(destination/'attempt.json',attempt)
    return readiness(guarded,destination)


def quality_flag_id(flag):
    """Bind prose guard observations to the frozen review schema's ID type."""
    return c.canonical_digest(flag)


def readiness(guarded,destination):
    """Supporting verification facts and pending obligations, never promotion."""
    guard=guarded.verify();floor=guarded.floor;task=floor.task
    attempt=_attempt(guarded,destination)
    receipts=attempt['receipts'];g.require(len({r['check_id'] for r in receipts})==len(receipts),'duplicate check receipt')
    reasons=list(guard['vetoes']);pending=[]
    if attempt['phase']!='COMPLETE' or any(r['phase']!='REMOVED' for r in attempt['checks'].values()):
        reasons.append('verifier lifecycle incomplete')
    if set(r['check_id'] for r in receipts)!=set(task['required_check_ids']):reasons.append('incomplete required check set')
    for receipt in receipts:
        check=next(x for x in task['check_argv'] if x['id']==receipt['check_id'])
        c.validate_verification_receipt(receipt,floor.verify()['schemas'],task=task,candidate_sha=guard['head'],
                                      context=_context(floor,guard['head'],check,attempt))
        path=Path(destination)/check['id'];stored=c.load_json_strict((path/'receipt.json').read_text())
        g.require(stored==receipt,'receipt artifact drift')
        observation=c.load_json_strict((path/'observation.json').read_text())
        g.require(c.canonical_digest(observation)==receipt['artifact_digest'],'observed artifact digest differs')
        for stream in ('stdout','stderr'):
            if stream+'_digest' in observation:
                g.require(g.digest_file(path/(stream+'.log'))==observation[stream+'_digest'],'observed log digest differs')
        if receipt['outcome']!='PASS':reasons.append(check['id']+': '+receipt['outcome'])
    for requirement in task['acceptance_requirements']:
        binding=floor.catalog['acceptance'].get(requirement['class_id'])
        if not binding or binding.get('requirement_digest')!=c.canonical_digest(requirement) or not binding.get('harness'):
            reasons.append('missing acceptance harness/binding: '+requirement['class_id']);continue
        data=g.controller_blob(floor.authority,binding['harness'])
        if hashlib.sha256(data).hexdigest()!=requirement['harness_digest']:
            reasons.append('acceptance harness digest mismatch: '+requirement['class_id']);continue
        mapping=binding.get('case_bindings',{})
        passed={case for receipt in receipts if receipt['outcome']=='PASS' and receipt['check_id'] in binding.get('check_ids',[])
                for case in receipt['passed_cases']}
        if set(mapping)!=set(requirement['case_ids']) or not set(mapping.values())<=passed:
            reasons.append('acceptance case binding/execution missing: '+requirement['class_id']);continue
        if binding.get('boundary')=='hosted-only':pending.append(requirement['class_id'])
        elif requirement['class_id'] not in ('STATIC','UNIT','INTEGRATION'):
            reasons.append('required real class evidence pending: '+requirement['class_id'])
        elif not binding.get('check_ids') or not set(binding['check_ids'])<=set(r['check_id'] for r in receipts if r['outcome']=='PASS'):
            reasons.append('acceptance checks missing: '+requirement['class_id'])
    result=dict(schema_version=1,task_id=task['task_id'],candidate_sha=guard['head'],floor_digest=floor.digest,
                attempt_nonce=attempt['nonce'],verification_passed=bool(receipts) and all(r['outcome']=='PASS' for r in receipts),
                acceptance_ready=not reasons and not guard['flags'],
                review_ready=bool(receipts) and all(r['outcome']=='PASS' for r in receipts) and not reasons,
                quality_flags=list(guard['flags']),
                quality_flag_ids={quality_flag_id(flag):flag for flag in guard['flags']},blocking_reasons=reasons,
                pending_hosted_classes=pending,unresolved=reasons+list(guard['flags']),
                receipts=receipts,authority_semantics='FACTS_ONLY_NO_REVIEW_PROMOTION_ADOPTION')
    g.write_json(Path(destination)/'readiness.json',result);return result
