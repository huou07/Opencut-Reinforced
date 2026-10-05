"""Trusted, bounded roadmap delegation; models never own this coordinator.

An immutable operator pin selects the delegation and approved checkpoint floors.
Child controller commits are derivations, not new human authorizations. Each
successor requires the real supervisor's exact state/evidence-only completion.
"""
from dataclasses import dataclass, replace, asdict
from pathlib import Path
import argparse
import copy
import fcntl
import hashlib
import json
import os
import subprocess
import sys
import time

from . import contracts as c, product

BUDGET_KEYS = ('cost_microusd', 'tokens', 'wall_seconds', 'tool_calls')
RUN_PATH = 'controller/roadmap/run.json'


@dataclass(frozen=True)
class RoadmapPin:
    """Out-of-band operator root; never inferred from a child controller commit."""
    source_sha: str
    delegation: c.RecordPin


def _read(root, sha, path):
    c._validate_relative_path(path, 'roadmap record')
    mode = c._git(Path(root), 'ls-tree', sha, '--', path)
    if not mode.startswith((b'100644 blob ', b'100755 blob ')):
        raise c.ContractError('roadmap input must be an immutable regular Git blob')
    return c._git(Path(root), 'show', sha + ':' + path)


def _json(root, sha, path):
    return c.load_json_strict(_read(root, sha, path).decode())


def _digest(data):
    return hashlib.sha256(data).hexdigest()


def protected_manifest(root, sha):
    """Freeze roadmap/spec/policy/control inputs; only STATE/evidence can advance."""
    return [e for e in c.build_manifest(Path(root), sha, _base_input=True)
            if c._authority_path(e['path']) and e['path'] != 'docs/execution/STATE.json'
            and not e['path'].startswith('docs/execution/evidence/')]


def _state(root, sha):
    return _json(root, sha, 'docs/execution/STATE.json')


def _remaining(plan, state):
    import execution_plan
    checkpoint = state['current_next']
    result = []
    while checkpoint is not None:
        if checkpoint in result:
            raise c.ContractError('cyclic roadmap relation')
        result.append(checkpoint)
        checkpoint = execution_plan.checkpoint_for_id(plan, checkpoint)['next_checkpoint_relation']
    return result


def load_delegation(release, product_root, pin):
    if type(pin) is not RoadmapPin or type(pin.delegation) is not c.RecordPin:
        raise c.ContractError('typed operator roadmap root pin required')
    facts = product.release_active(release)
    controller = Path(release.controller_root)
    c._ancestor(controller, pin.source_sha, release.source_sha)
    delegation = _json(controller, pin.source_sha, pin.delegation.path)
    c._shape(delegation, 'roadmap_delegation', facts['schemas'])
    if c.canonical_digest(delegation) != pin.delegation.digest:
        raise c.ContractError('roadmap delegation digest differs from operator root')
    if delegation['release_sha'] != facts['git']['release_oid'] or delegation['adoption_digest'] != c.canonical_digest(facts['adoption']):
        raise c.ContractError('roadmap delegation binds stale release/adoption')
    base = delegation['initial_base_sha']
    root = c._authority_repo(Path(product_root), facts['git']['anchor_oid'])
    for other in c.authority_roots(release):
        if root == other or root in other.parents or other in root.parents:
            raise c.ContractError('roadmap product/controller/control roots must be independent')
    c._ancestor(root, facts['git']['anchor_oid'], base)
    if _digest(_read(root, base, 'docs/execution/PLAN.json')) != delegation['plan_digest'] or _digest(_read(root, base, 'docs/execution/STATE.json')) != delegation['state_digest']:
        raise c.ContractError('delegation initial PLAN/STATE mismatch')
    if c.canonical_digest(protected_manifest(root, base)) != delegation['protected_manifest_digest']:
        raise c.ContractError('delegation protected roadmap manifest mismatch')
    plan = _json(root, base, 'docs/execution/PLAN.json')
    entries = delegation['checkpoints']
    if [e['checkpoint_id'] for e in entries] != _remaining(plan, _state(root, base)):
        raise c.ContractError('delegation must bind exactly the authoritative remaining roadmap')
    if delegation['execution_profile'] == 'PRODUCT':
        if delegation['plan_digest'] != facts['adoption']['plan_digest'] or delegation['destination_url'] not in (
                'https://github.com/' + c.REPOSITORY_IDENTITY + '.git', 'git@github.com:' + c.REPOSITORY_IDENTITY + '.git'):
            raise c.ContractError('production roadmap differs from adopted PLAN/publication')
    else:
        bare = Path(delegation['destination_url'])
        if not bare.is_absolute() or not bare.is_dir() or bare.is_symlink() or c._git(bare, 'rev-parse', '--is-bare-repository').strip() != b'true':
            raise c.ContractError('fixture roadmap requires explicit local bare publication')
    if _digest(_read(controller, pin.source_sha, delegation['executor_path'])) != delegation['executor_digest']:
        raise c.ContractError('trusted roadmap executor differs from operator pin')
    for entry in entries:
        for field in ('task', 'catalog'):
            record = _json(controller, pin.source_sha, entry[field + '_path'])
            if c.canonical_digest(record) != entry[field + '_digest']:
                raise c.ContractError('roadmap task/floor changed after operator delegation')
        task = _json(controller, pin.source_sha, entry['task_path'])
        if task['checkpoint_id'] != entry['checkpoint_id'] or task['task_kind'] != 'product_checkpoint':
            raise c.ContractError('roadmap blueprint checkpoint mismatch')
        c.validate_task_contract(task, facts['schemas'], frozen_template=task)
    return delegation


def derive_task(release, product_root, pin, delegation, sequence, base):
    if not 1 <= sequence <= len(delegation['checkpoints']):
        raise c.ContractError('task exceeds roadmap delegation')
    entry = delegation['checkpoints'][sequence - 1]
    state = _state(product_root, base)
    if state['current_next'] != entry['checkpoint_id']:
        raise c.ContractError('roadmap task must be exact NEXT, no skip')
    if c.canonical_digest(protected_manifest(product_root, base)) != delegation['protected_manifest_digest']:
        raise c.ContractError('roadmap/spec/policy changed outside delegation')
    facts = product.release_active(release)
    manifest = copy.deepcopy(facts['git'])
    manifest.update(base_oid=base, base_manifest=c.build_manifest(Path(product_root), base, _base_input=True),
                    purpose='OPERATIONAL', adoption_digest=c.canonical_digest(facts['adoption']))
    manifest.pop('authority_digest')
    manifest['authority_digest'] = c.canonical_digest(manifest)
    task = copy.deepcopy(_json(release.controller_root, pin.source_sha, entry['task_path']))
    task.update(task_id='roadmap-' + pin.delegation.digest[:20] + '-' + str(sequence),
                base_sha=base, authority_digest=manifest['authority_digest'],
                candidate_branch='candidate/roadmap-' + pin.delegation.digest[:12] + '-' + str(sequence))
    directory = 'controller/roadmap/tasks/' + str(sequence)
    auth = dict(schema_version=1, repository=c.REPOSITORY_IDENTITY, purpose='OPERATOR_PRODUCT_TASK',
        authorization_id=delegation['authorization_id'], nonce=delegation['nonce'], sequence=sequence,
        release_sha=delegation['release_sha'], adoption_digest=delegation['adoption_digest'],
        product_base_sha=base, plan_digest=delegation['plan_digest'],
        state_digest=_digest(_read(product_root, base, 'docs/execution/STATE.json')),
        checkpoint_id=entry['checkpoint_id'], task_path=directory + '/task.json', task_digest=c.canonical_digest(task),
        catalog_path=entry['catalog_path'], catalog_digest=entry['catalog_digest'],
        execution_profile=delegation['execution_profile'], destination_url=delegation['destination_url'],
        roadmap_delegation_digest=pin.delegation.digest, roadmap_sequence=sequence)
    return task, auth


def validate_completion(product_root, delegation, auth, completed_base):
    """Reopen immutable product commits; a controller/model DONE label is insufficient."""
    import execution_plan, execution_evidence, agent_supervisor
    root = Path(product_root)
    implementation = c._git(root, 'rev-parse', completed_base + '^').decode().strip()
    c._ancestor(root, auth['product_base_sha'], implementation)
    paths = c._git(root, 'diff', '--name-only', implementation, completed_base).decode().splitlines()
    if set(paths) != {'docs/execution/STATE.json', 'docs/execution/evidence/' + auth['checkpoint_id'] + '.json'}:
        raise c.ContractError('successor base is not a supervisor evidence/state-only completion')
    if c.canonical_digest(protected_manifest(root, completed_base)) != delegation['protected_manifest_digest']:
        raise c.ContractError('completion changed the delegated roadmap authority')
    plan = _json(root, completed_base, 'docs/execution/PLAN.json')
    before, after = _state(root, auth['product_base_sha']), _state(root, completed_base)
    versions = execution_plan.read_contract_versions(root, completed_base)
    agent_supervisor.assert_state_advanced_once(before, after, plan,
        candidate_contract_versions=versions, policy=_json(root, completed_base, 'docs/execution/architecture-policy.json'), repo_root=root)
    evidence = _json(root, completed_base, 'docs/execution/evidence/' + auth['checkpoint_id'] + '.json')
    checkpoint = execution_plan.checkpoint_for_id(plan, auth['checkpoint_id'])
    if evidence.get('implementation_sha') != implementation:
        raise c.ContractError('supervisor evidence binds a different implementation')
    if delegation['execution_profile'] == 'PRODUCT':
        policy = _json(root, completed_base, 'docs/execution/EVIDENCE_POLICY.json')
        execution_evidence.validate_evidence_record(evidence, checkpoint_id=auth['checkpoint_id'],checkpoint=checkpoint,policy=policy)
        receipt = evidence.get('control_plane_receipt', {})
        expected = dict(task_id='roadmap-' + auth['roadmap_delegation_digest'][:20] + '-' + str(auth['sequence']),
                        task_contract_digest=auth['task_digest'], base_sha=auth['product_base_sha'],
                        adoption_manifest_digest=auth['adoption_digest'], promoted_implementation_sha=implementation)
        c._bind(receipt, expected, expected)
        # Schema validation alone does not establish hosted acceptance. Recollect
        # the exact required workflow/job/class/preview facts independently.
        api = execution_evidence.GitHubApi(*c.REPOSITORY_IDENTITY.split('/'),token=execution_evidence.select_token())
        subject = c._git(root,'show','-s','--format=%s',implementation).decode().strip()
        verified = agent_supervisor.verify_hosted_checkpoint(root,plan,checkpoint,implementation,subject,api=api,control_plane_receipt=receipt)['record']
        if any(verified[k] != evidence[k] for k in ('gates','developer_preview','evidence_classes')):
            raise c.ContractError('completed hosted evidence is stale or differs from recollection')
    else:
        # This separate fixture record cannot pass the production evidence schema.
        if evidence.get('authority_semantics') != 'ISOLATED_ROADMAP_FIXTURE_NOT_PRODUCT_EVIDENCE' or evidence.get('task_digest') != auth['task_digest'] or evidence.get('roadmap_delegation_digest') != auth['roadmap_delegation_digest']:
            raise c.ContractError('fixture completion lacks exact supervisor task/delegation binding')
        hosted_proof = evidence.get('hosted_proof')
        if not isinstance(hosted_proof, dict):
            raise c.ContractError('fixture completion lacks exact hosted observation')
        from .hosted import collect_roadmap_fixture
        actual = collect_roadmap_fixture(run_id=hosted_proof['run_id'],release_sha=delegation['release_sha'],
            candidate_sha=implementation,task_digest=auth['task_digest'],marker_path='roadmap-'+str(auth['sequence'])+'.txt',
            marker='# ROADMAP_'+str(auth['sequence'])+'_PASS')
        if actual != hosted_proof:
            raise c.ContractError('fixture completion hosted observation differs from recollection')
    return implementation


def _run_record(release):
    mode = c._git(Path(release.controller_root), 'ls-tree', release.source_sha, '--', RUN_PATH)
    if not mode:
        return dict(schema_version=1, attempts=[], tasks=[])
    record = c._require_object(_json(release.controller_root, release.source_sha, RUN_PATH),'roadmap ledger')
    if type(record.get('schema_version')) is not int or type(record.get('attempts')) is not list or type(record.get('tasks')) is not list:
        raise c.ContractError('invalid roadmap ledger types')
    if len(record['attempts'])>65536 or len(record['tasks'])>4096:
        raise c.ContractError('roadmap ledger size bound exceeded')
    if set(record) != {'schema_version','attempts','tasks'} or record['schema_version'] != 1:
        raise c.ContractError('unknown roadmap ledger fields/version')
    return record


def validate_budget(delegation, attempts):
    totals = dict.fromkeys(BUDGET_KEYS, 0)
    for item in attempts:
        item=c._require_object(item,'roadmap attempt')
        if set(item) != {'sequence','attempt','reservation','usage','result'} or type(item['sequence']) is not int or type(item['attempt']) is not int:
            raise c.ContractError('invalid roadmap attempt identity')
        reservation=c._require_object(item['reservation'],'roadmap reservation')
        if set(reservation)!=set(BUDGET_KEYS) or any(type(reservation[k]) is not int or reservation[k]<0 for k in BUDGET_KEYS):
            raise c.ContractError('invalid roadmap reservation ceilings')
        charge = c._require_object(item['usage'],'roadmap usage') if item['usage'] is not None else reservation
        for key in BUDGET_KEYS:
            if type(charge.get(key)) is not int or charge[key] < 0:
                raise c.ContractError('unknown/invalid roadmap usage; no further spending')
            totals[key] += max(charge[key], item['reservation'][key])
            if totals[key] > delegation['budget_ceiling'][key]:
                raise c.ContractError('roadmap budget ceiling exceeded: ' + key)
    return totals



def validate_ledger(delegation, run, release, pin):
    validate_budget(delegation,run['attempts'])
    seen={}
    for attempt in run['attempts']:
        seq=attempt['sequence']
        if not 1<=seq<=len(run['tasks']) or attempt['attempt']!=seen.get(seq,0)+1 or attempt['result'] not in ('RESERVED','RETRY','DONE'):
            raise c.ContractError('invalid or duplicate roadmap budget reservation')
        blueprint=_json(release.controller_root,pin.source_sha,delegation['checkpoints'][seq-1]['task_path'])
        if attempt['reservation']!={k:blueprint['budget'][k] for k in BUDGET_KEYS}:
            raise c.ContractError('roadmap budget reservation differs from delegated task ceiling')
        if attempt['result']!='RESERVED' and attempt['usage'] is None:
            raise c.ContractError('completed attempt has unknown resource accounting')
        seen[seq]=attempt['attempt']
    for seq,row in enumerate(run['tasks'],1):
        if seq not in seen or (row['completed_base_sha'] is not None and not any(a['sequence']==seq and a['result']=='DONE' for a in run['attempts'])):
            raise c.ContractError('task/completion lacks a charged execution reservation')


def validate_derivation(release, product_root, pin, auth):
    delegation = load_delegation(release, product_root, pin)
    run = _run_record(release)
    validate_ledger(delegation, run, release, pin)
    base = delegation['initial_base_sha']
    for sequence, row in enumerate(run['tasks'], 1):
        if set(row) != {'authorization','completed_base_sha'}:
            raise c.ContractError('invalid roadmap derivation ledger')
        # Each recorded immutable task must equal deterministic derivation.
        # operational_manifest requires the current immutable materialization;
        # previous task identities are reconstructed against their Git manifests.
        old = row['authorization']
        if old['sequence'] != sequence or old['product_base_sha'] != base or old['roadmap_delegation_digest'] != pin.delegation.digest:
            raise c.ContractError('broken roadmap completion/derivation chain')
        _, expected_old = derive_task(release, product_root, pin, delegation, sequence, base)
        if old != expected_old:
            raise c.ContractError('historical task derivation changed delegated scope/floor/bindings')
        if old == auth:
            if row['completed_base_sha'] is not None or row is not run['tasks'][-1]:
                raise c.ContractError('completed/stale roadmap task cannot execute again')
            task, expected = derive_task(release, product_root, pin, delegation, sequence, base)
            if auth != expected:
                raise c.ContractError('derived task broadened delegation or changed exact bindings')
            actual = _json(release.controller_root, release.source_sha, auth['task_path'])
            if actual != task or _json(release.controller_root, release.source_sha, auth['catalog_path']) != _json(release.controller_root,pin.source_sha,auth['catalog_path']):
                raise c.ContractError('derived task/catalog differs from delegated immutable blueprint')
            return delegation
        if row['completed_base_sha'] is None:
            raise c.ContractError('unfinished predecessor cannot grant successor authority')
        validate_completion(product_root, delegation, old, row['completed_base_sha'])
        base = row['completed_base_sha']
    raise c.ContractError('task absent from external roadmap derivation ledger')


def _commit(controller, files, message):
    for path, value in files.items():
        c._validate_relative_path(path,'controller output')
        target = Path(controller)/path
        for parent in (target,*target.parents):
            if parent==Path(controller):break
            if parent.is_symlink():raise c.ContractError('controller output symlink escape')
        target.parent.mkdir(parents=True,exist_ok=True)
        target.write_text(c.canonical_json(value)+'\n')
    subprocess.run(['git','-C',str(controller),'add','--',*files],check=True)
    subprocess.run(['git','-C',str(controller),'commit','-qm',message],check=True)
    return c._git(Path(controller),'rev-parse','HEAD').decode().strip()


def issue_next(candidate_root, controller_root, product_root, bootstrap, pin):
    release=c.load_release_authority(Path(candidate_root),Path(controller_root),bootstrap=bootstrap)
    delegation=load_delegation(release,product_root,pin)
    run=_run_record(release)
    validate_ledger(delegation,run,release,pin)
    if time.time() >= delegation['expires_at_unix']:
        raise c.ContractError('roadmap deadline budget exhausted')
    sequence=len(run['tasks'])+1
    if run['tasks'] and run['tasks'][-1]['completed_base_sha'] is None:
        raise c.ContractError('unfinished task requires recovery, not new authorization')
    base=run['tasks'][-1]['completed_base_sha'] if run['tasks'] else delegation['initial_base_sha']
    for row in run['tasks']:
        validate_completion(product_root,delegation,row['authorization'],row['completed_base_sha'])
    if sequence>len(delegation['checkpoints']):
        if _state(product_root,base)['current_next'] is not None:
            raise c.ContractError('roadmap scope exhausted while NEXT remains')
        return None
    observed=c._git(Path(product_root),'ls-remote','--exit-code',delegation['destination_url'],'refs/heads/main').decode().split()
    if observed != [base,'refs/heads/main']:
        raise c.ContractError('roadmap publication base is stale')
    task,auth=derive_task(release,product_root,pin,delegation,sequence,base)
    reservation={k:task['budget'][k] for k in BUDGET_KEYS}
    trial=run['attempts']+[dict(sequence=sequence,attempt=1,reservation=reservation,usage=None,result='RESERVED')]
    validate_budget(delegation,trial)
    run['tasks'].append(dict(authorization=auth,completed_base_sha=None))
    run['attempts']=trial
    source=_commit(controller_root,{RUN_PATH:run,auth['task_path']:task,auth['task_path'].replace('task.json','authorization.json'):auth},'feat(controller): derive exact NEXT under operator roadmap delegation')
    bp=replace(bootstrap,source_sha=source)
    task_pin=c.RecordPin(auth['task_path'].replace('task.json','authorization.json'),c.canonical_digest(auth))
    # Mint no raw mapping authority; the real product loader checks everything.
    product.load_operational_authority(candidate_root,controller_root,product_root,bootstrap=bp,product_pin=task_pin,roadmap_pin=pin)
    return bp,task_pin,task


def run_roadmap(candidate_root, controller_root, product_root, bootstrap, pin, runtime_root):
    """Run the pinned trusted executor repeatedly; no model selects checkpoint/base.

    Executor receives a host context file and must use existing V2 stages and
    supervisor. It can recover ordinary failures/escalate within the task, but
    its result alone grants no successor authority. Unknown budget accounting,
    outside-contract decisions and unavailable capabilities stop the run.
    """
    runtime_root=Path(runtime_root).resolve()
    for root in (Path(candidate_root).resolve(),Path(controller_root).resolve(),Path(product_root).resolve()):
        if runtime_root==root or runtime_root in root.parents or root in runtime_root.parents:
            raise c.ContractError('roadmap runtime must be independent of all immutable roots')
    runtime_root.mkdir(parents=True,exist_ok=True,mode=0o700)
    lock=runtime_root/'run.lock'
    with lock.open('a') as handle:
        try: fcntl.flock(handle,fcntl.LOCK_EX|fcntl.LOCK_NB)
        except BlockingIOError as exc: raise c.ContractError('roadmap already owned by another trusted host') from exc
        # Descendant controller snapshots are allowed only through this pinned
        # delegation. Exact release pins still forbid changing build/cert/adoption.
        head=c._git(Path(controller_root),'rev-parse','HEAD').decode().strip()
        c._ancestor(Path(controller_root),bootstrap.source_sha,head)
        bootstrap=replace(bootstrap,source_sha=head)
        while True:
            release=c.load_release_authority(Path(candidate_root),Path(controller_root),bootstrap=bootstrap)
            delegation=load_delegation(release,product_root,pin)
            run=_run_record(release)
            if time.time() >= delegation['expires_at_unix']:
                raise c.ContractError('roadmap deadline budget exhausted')
            if not run['tasks'] or run['tasks'][-1]['completed_base_sha'] is not None:
                issued=issue_next(candidate_root,controller_root,product_root,bootstrap,pin)
                if issued is None:
                    return dict(result='ROADMAP_COMPLETE',completed=len(run['tasks']),controller_sha=bootstrap.source_sha)
                bootstrap,task_pin,task=issued
                release=c.load_release_authority(Path(candidate_root),Path(controller_root),bootstrap=bootstrap)
                run=_run_record(release)
            else:
                auth=run['tasks'][-1]['authorization']
                task_pin=c.RecordPin(auth['task_path'].replace('task.json','authorization.json'),c.canonical_digest(auth))
                task=_json(controller_root,bootstrap.source_sha,auth['task_path'])
                validate_derivation(release,product_root,pin,auth)
            auth=run['tasks'][-1]['authorization']
            sequence=auth['sequence']
            pending=[a for a in run['attempts'] if a['sequence']==sequence and a['result']=='RESERVED']
            if pending:
                # A crashed launch stays charged; resume the same durable task
                # through the executor's receipt reconciliation, no blind replay.
                attempt=pending[-1]
            else:
                prior=[a for a in run['attempts'] if a['sequence']==sequence]
                attempt=dict(sequence=sequence,attempt=len(prior)+1,reservation={k:task['budget'][k] for k in BUDGET_KEYS},usage=None,result='RESERVED')
                run['attempts'].append(attempt)
                validate_budget(delegation,run['attempts'])
                source=_commit(controller_root,{RUN_PATH:run},'chore(controller): reserve bounded roadmap checkpoint execution')
                bootstrap=replace(bootstrap,source_sha=source)
            stage_root=runtime_root/str(sequence)
            stage_root.mkdir(parents=True,exist_ok=True,mode=0o700)
            execution_context=stage_root/'execution-context.json'
            if execution_context.exists():
                fixed=c.load_json_strict(execution_context.read_text())
                if fixed['product_task_pin']!=asdict(task_pin) or fixed['roadmap_pin']!=asdict(pin):
                    raise c.ContractError('crash recovery execution context differs from delegated task')
                execution_controller=Path(fixed['controller_root'])
                execution_product=Path(fixed['product_root'])
                execution_bootstrap=fixed['bootstrap']
            else:
                execution_controller=stage_root/'controller'
                execution_product=stage_root/'product-base'
                for source,target,sha in ((controller_root,execution_controller,bootstrap.source_sha),
                                          (product_root,execution_product,auth['product_base_sha'])):
                    subprocess.run(['git','clone','-q','--no-hardlinks',str(source),str(target)],check=True)
                    subprocess.run(['git','-C',str(target),'checkout','-q','--detach',sha],check=True)
                    subprocess.run(['git','-C',str(target),'remote','set-url','origin','https://github.com/'+c.REPOSITORY_IDENTITY+'.git'],check=True)
                execution_bootstrap=asdict(bootstrap)
                fixed=dict(bootstrap=execution_bootstrap,product_task_pin=asdict(task_pin),roadmap_pin=asdict(pin),
                    controller_root=str(execution_controller),product_root=str(execution_product))
                execution_context.write_text(c.canonical_json(fixed)+'\n');execution_context.chmod(0o600)
            context=dict(bootstrap=execution_bootstrap,product_task_pin=asdict(task_pin),roadmap_pin=asdict(pin),
                         candidate_root=str(candidate_root),controller_root=str(execution_controller),product_root=str(execution_product),next_product_root=str(product_root),
                         task_path=auth['task_path'],catalog_path=auth['catalog_path'],task_id=task['task_id'],
                         checkpoint_id=auth['checkpoint_id'],execution_profile=auth['execution_profile'],
                         runtime_root=str(stage_root/'execution'),attempt=attempt['attempt'])
            context_path=runtime_root/'context.json'
            context_path.write_text(c.canonical_json(context)+'\n');context_path.chmod(0o600)
            result_path=runtime_root/'result.json'
            if result_path.exists(): result_path.unlink()
            executable=runtime_root/'trusted-executor.py'
            executable.write_bytes(_read(controller_root,pin.source_sha,delegation['executor_path']));executable.chmod(0o500)
            start=time.monotonic()
            try:
                proc=subprocess.run([sys.executable,'-B',str(executable),str(context_path),str(result_path)],
                    env=dict(os.environ,PYTHONDONTWRITEBYTECODE='1',PYTHONPATH=str(Path(candidate_root)/'scripts')),
                    timeout=min(task['budget']['wall_seconds'],delegation['expires_at_unix']-time.time()))
            except subprocess.TimeoutExpired as exc:
                raise c.ContractError('checkpoint wall budget exhausted; durable reservation retained') from exc
            if proc.returncode or not result_path.is_file():
                raise c.ContractError('trusted checkpoint executor unavailable; durable task must be reconciled')
            result=c.load_json_strict(result_path.read_text())
            if set(result)!={'result','usage','completed_base_sha'} or result['result'] not in ('DONE','RETRY','OUTSIDE_DELEGATION','CAPABILITY_UNAVAILABLE'):
                raise c.ContractError('invalid trusted checkpoint result')
            if result['result'] in ('OUTSIDE_DELEGATION','CAPABILITY_UNAVAILABLE'):
                raise c.ContractError('roadmap stopped: '+result['result'])
            usage=result['usage']
            if type(usage) is not dict or set(usage)!=set(BUDGET_KEYS) or any(type(usage[k]) is not int or usage[k]<0 for k in BUDGET_KEYS) or usage['wall_seconds']<int(time.monotonic()-start):
                raise c.ContractError('missing/invalid trusted execution usage')
            # Reopen controller after executor: it must not mutate authority/ledger.
            c.load_release_authority(Path(candidate_root),Path(controller_root),bootstrap=bootstrap)
            attempt.update(usage=usage,result=result['result'])
            validate_budget(delegation,run['attempts'])
            if result['result']=='RETRY':
                source=_commit(controller_root,{RUN_PATH:run},'chore(controller): retain checkpoint failure and accounted repair budget')
                bootstrap=replace(bootstrap,source_sha=source)
                continue
            completed=result['completed_base_sha']
            c._commit(Path(product_root),completed)
            validate_completion(product_root,delegation,auth,completed)
            # The publication observation and clean new snapshot both bind DONE.
            if c._git(Path(product_root),'rev-parse','HEAD').decode().strip()!=completed or c._git(Path(product_root),'status','--porcelain').strip():
                raise c.ContractError('executor did not materialize the clean exact completed product base')
            observed=c._git(Path(product_root),'ls-remote','--exit-code',delegation['destination_url'],'refs/heads/main').decode().split()
            if observed != [completed,'refs/heads/main']:
                raise c.ContractError('supervisor completion is not exact publication main')
            run['tasks'][-1]['completed_base_sha']=completed
            source=_commit(controller_root,{RUN_PATH:run},'feat(controller): record independently completed roadmap checkpoint')
            bootstrap=replace(bootstrap,source_sha=source)


def main(argv=None):
    parser=argparse.ArgumentParser(description=__doc__)
    for flag in ('candidate-root','controller-root','product-root','bootstrap','roadmap-pin','runtime'):
        parser.add_argument('--'+flag,required=True)
    args=parser.parse_args(argv)
    from .__main__ import bootstrap_from_dict
    raw=c.load_json_strict(Path(args.roadmap_pin).read_text())
    pin=RoadmapPin(raw['source_sha'],c.RecordPin(**raw['delegation']))
    try:
        result=run_roadmap(Path(args.candidate_root),Path(args.controller_root),Path(args.product_root),
            bootstrap_from_dict(c.load_json_strict(Path(args.bootstrap).read_text())),pin,Path(args.runtime))
        print(c.canonical_json(result));return 0
    except (c.ContractError,OSError,subprocess.SubprocessError) as exc:
        print(c.canonical_json(dict(result='ROADMAP_STOPPED',reason=str(exc))));return 2


if __name__=='__main__':
    raise SystemExit(main())
