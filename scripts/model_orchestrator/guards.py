"""M2 credentialless import, all-commit scope and immutable measurement floors.

Only facts: no dispatch, review disposition, promotion or operational adoption.
The controller pins a task and verifier catalog in its independent Git source.
load_floor reads those committed blobs, validates the exact M0 task contract,
and binds checks/executables, baseline case identities, protected harnesses,
ignored scratch paths, class-specific acceptance mappings and resource methods.
The catalog is controller input; no worker-supplied catalog or receipt is read.
Inputs are limited to 256 MiB/100000 entries, Git output/history/facts to 64 MiB,
and history to 2048 single-parent commits; unsupported inputs remain preserved.
Each private quarantine has a fsynced nonce/device/inode reservation before
creation, plus a strict candidate receipt before a durable Guarded handoff.
restore_guarded reloads only that private handoff; cleanup_quarantine reclaims
only an incomplete import while its exact useful M1 input still exists.
"""
from __future__ import annotations

from dataclasses import dataclass
import ast
import hashlib
import os
from pathlib import Path
import re
import shutil
import stat
import subprocess
import time
import uuid

from . import contracts as c, sandbox as b, store as s, workspace as w

_SEAL = object()
MAX_BYTES = 64 << 20
MAX_COMMITS = 2048
MAX_INPUT_BYTES = 256 << 20
CATALOG_KEYS = ('schema_version', 'git', 'checks', 'cases', 'harnesses', 'executables',
                'scratch_paths', 'acceptance', 'performance')


def require(ok, message):
    if not ok:
        raise c.ContractError(message)


def path_bytes(value):
    """A deliberately restricted byte-safe portable subset; never normalize."""
    require(type(value) is bytes and 0 < len(value) <= 4096, 'invalid path bytes')
    require(all(32 <= ch < 127 for ch in value) and b'\\' not in value and b':' not in value,
            'unsupported nonportable path bytes')
    parts = value.split(b'/')
    require(all(p not in (b'', b'.', b'..') and p.lower() != b'.git' and
                not p.endswith((b' ', b'.')) for p in parts), 'ambiguous/traversal path')
    return value.decode('ascii')


def parse_raw(data):
    """Git --raw -z --no-abbrev: status header NUL path [NUL path] NUL."""
    require(type(data) is bytes and len(data) <= MAX_BYTES, 'raw diff bound/type')
    if not data:
        return []
    require(data.endswith(b'\0'), 'truncated NUL diff')
    fields = data[:-1].split(b'\0'); rows = []; index = 0
    header = re.compile(rb':(\d{6}) (\d{6}) ([0-9a-f]{40}) ([0-9a-f]{40}) ([AMD]|[RC]\d{1,3})\Z')
    while index < len(fields):
        match = header.fullmatch(fields[index]); index += 1
        require(match is not None, 'malformed raw status header')
        old_mode, new_mode, old_oid, new_oid, status = (v.decode('ascii') for v in match.groups())
        count = 2 if status[0] in 'RC' else 1
        require(index + count <= len(fields), 'truncated rename/copy pair')
        paths = [path_bytes(v) for v in fields[index:index+count]]; index += count
        if count == 2:
            require(0 <= int(status[1:]) <= 100 and paths[0] != paths[1], 'malformed rename/copy pair')
        require(old_mode in ('000000','100644','100755') and new_mode in ('000000','100644','100755'),
                'unsupported mode/type/gitlink')
        require((old_mode == '000000') == (old_oid == '0'*40) and
                (new_mode == '000000') == (new_oid == '0'*40), 'mode/object mismatch')
        require((status == 'A') == (old_mode == '000000') and
                (status == 'D') == (new_mode == '000000'), 'status/mode mismatch')
        rows.append(dict(status=status, paths=paths, old_mode=old_mode, new_mode=new_mode,
                         old_oid=old_oid, new_oid=new_oid))
    return rows


def digest_file(path):
    return w.file_digest(Path(path))


def write_json(path, value):
    """Controller-owned bounded atomic record; path comes from its reservation."""
    data = (c.canonical_json(value)+'\n').encode()
    require(len(data) <= MAX_BYTES, 'controller artifact bound')
    temporary = path.with_name('.'+path.name+'.tmp')
    fd=os.open(temporary,os.O_WRONLY|os.O_CREAT|os.O_TRUNC|os.O_NOFOLLOW,0o600)
    with os.fdopen(fd,'wb') as handle:
        item=os.fstat(handle.fileno())
        require(stat.S_ISREG(item.st_mode) and item.st_nlink==1 and item.st_uid==os.geteuid(),'unsafe controller temporary')
        handle.write(data); handle.flush(); os.fsync(handle.fileno())
    os.replace(temporary, path); s._fsync_dir(path.parent)


@dataclass(frozen=True, init=False)
class Floor:
    authority: c.ValidatedReleaseAuthority
    task_json: str
    catalog_json: str
    digest: str

    def __init__(self, authority, task, catalog, *, seal=None):
        require(seal is _SEAL, 'floor requires independent pinned controller source')
        object.__setattr__(self, 'authority', authority)
        object.__setattr__(self, 'task_json', c.canonical_json(task))
        object.__setattr__(self, 'catalog_json', c.canonical_json(catalog))
        object.__setattr__(self, 'digest', c.canonical_digest([task,catalog]))

    @property
    def task(self): return c.load_json_strict(self.task_json)

    @property
    def catalog(self): return c.load_json_strict(self.catalog_json)

    def verify(self):
        # The floor is an M2 guard capability; it admits the exact externally
        # executing phase task at or after M2, never a rewritten checkpoint.
        payload = c._release_authority(self.authority)
        return c._phase_admission(payload, self.task, 'M2')


def controller_blob(authority, name):
    c._validate_relative_path(name, 'controller blob')
    root = Path(authority.controller_root)
    mode = c._git(root, 'ls-tree', authority.source_sha, '--', name)
    require(mode.startswith(b'100644 blob ') or mode.startswith(b'100755 blob '), 'missing regular frozen harness/blob')
    return c._git(root, 'show', authority.source_sha+':'+name)


def load_floor(authority, task_path, catalog_path):
    payload = c._release_authority(authority)
    if 'operational' in payload:
        auth = payload['operational']['authorization']
        if task_path != auth['task_path'] or catalog_path != auth['catalog_path']:
            raise c.ContractError('product floor paths differ from exact operator pins')
    task = c.load_json_strict(controller_blob(authority,task_path).decode())
    catalog = c.load_json_strict(controller_blob(authority,catalog_path).decode())
    s._exact(catalog, CATALOG_KEYS)
    require(catalog['schema_version'] == 1 and type(catalog['schema_version']) is int, 'unknown catalog version')
    floor = Floor(authority,task,catalog,seal=_SEAL); payload = floor.verify()
    c.validate_task_contract(task,payload['schemas'],frozen_template=task)
    require(set(catalog['checks']) == set(task['required_check_ids']), 'frozen check bindings incomplete')
    require(set(catalog['cases']) == set(task['case_inventory']), 'baseline case identities incomplete')
    require(type(catalog['checks']) is dict and type(catalog['cases']) is dict and
            type(catalog['acceptance']) is dict and type(catalog['performance']) is dict,
            'invalid frozen catalog maps')
    for name in catalog['scratch_paths'] + catalog['executables'] + catalog['harnesses']:
        path_bytes(name.encode('ascii'))
    for case,binding in catalog['cases'].items():
        s._exact(binding,('path','symbol'));path_bytes(binding['path'].encode('ascii'))
        names=symbols(c._git(c.base_source_root(authority),'show',task['base_sha']+':'+binding['path']))
        require(names is not None and binding['symbol'] in names,'required baseline case identity missing: '+case)
    for check in task['check_argv']:
        binding = catalog['checks'][check['id']]
        s._exact(binding, ('boundary','executable','executable_digest','harness','harness_digest','image','docker','docker_digest','endpoint'))
        require(binding['boundary'] in ('controller-data','rootless','hosted-only'), 'unsupported verifier boundary')
        require(check['argv'][0] == binding['executable'], 'executable argv binding differs')
        require(binding['executable'].startswith(('/bin/','/usr/bin/')) and
                binding['executable_digest']==c.canonical_digest([binding['image'],binding['executable']]),
                'executable/image identity not frozen')
        harness = controller_blob(authority,binding['harness'])
        require(hashlib.sha256(harness).hexdigest() == binding['harness_digest'] == check['harness_digest'], 'wrong harness digest')
        require('/verifier/'+binding['harness'] in check['argv'] or binding['boundary'] != 'rootless', 'rootless harness argv missing')
        require(not set(check['argv'])&{'-c','-m','--eval','--command'},'inline/module execution cannot replace frozen harness')
    return floor


def git(floor, root, *args, input_bytes=None):
    binding = floor.catalog['git']; s._exact(binding,('executable','digest'))
    executable = Path(binding['executable'])
    require(executable.is_absolute() and executable.is_file() and digest_file(executable)==binding['digest'], 'trusted Git executable missing/changed')
    env={'PATH':str(executable.parent),'LANG':'C','LC_ALL':'C','GIT_CONFIG_NOSYSTEM':'1',
         'GIT_CONFIG_GLOBAL':os.devnull,'GIT_NO_REPLACE_OBJECTS':'1','GIT_NO_LAZY_FETCH':'1','GIT_TERMINAL_PROMPT':'0'}
    try:
        result=subprocess.run([str(executable),'--no-optional-locks','-c','core.hooksPath='+os.devnull,
            '-c','core.fsmonitor=false','-c','core.attributesFile='+os.devnull,'-c','protocol.file.allow=never',
            *args],cwd=root,env=env,input=input_bytes,capture_output=True,timeout=60,check=True)
    except (OSError,subprocess.SubprocessError) as exc:
        raise c.ContractError('trusted Git rejected '+args[0]) from exc
    require(len(result.stdout)<=MAX_BYTES,'Git output bound');return result.stdout


def refs(candidate):
    """Read as bytes; never ask Git to read the worker's config/index."""
    candidate.verify(); root=candidate.root/'.git'
    require((root/'HEAD').read_bytes()==b'ref: refs/heads/candidate\n', 'branch mismatch/detached HEAD')
    seen={}
    for directory, _, files in os.walk(root/'refs'):
        for name in files:
            p=Path(directory)/name; ref=str(p.relative_to(root))
            require(ref=='refs/heads/candidate','unexpected refs')
            require(p.stat().st_size==41,'invalid ref bound')
            value=p.read_bytes().strip();require(re.fullmatch(rb'[0-9a-f]{40}',value) is not None,'invalid ref')
            seen[ref]=value.decode('ascii')
    packed=root/'packed-refs'
    if packed.exists():
        for line in packed.read_bytes().splitlines():
            if line.startswith(b'# pack-refs with:'):continue
            require(re.fullmatch(rb'[0-9a-f]{40} refs/heads/candidate',line) is not None,'unexpected/malformed packed refs')
            ref='refs/heads/candidate'
            require(ref not in seen or seen[ref]==line[:40].decode(),'ambiguous ref identity')
            seen[ref]=line[:40].decode()
    require(set(seen)=={'refs/heads/candidate'},'candidate branch missing')
    return seen['refs/heads/candidate']


def tree(floor, root, oid):
    output=git(floor,root,'ls-tree','-rz','--full-tree',oid)
    require(output.endswith(b'\0') or not output,'truncated tree')
    result={}; portable=set()
    for row in output[:-1].split(b'\0') if output else []:
        require(row.count(b'\t')==1,'malformed tree row')
        header,name=row.split(b'\t');parts=header.split(b' ')
        require(len(parts)==3 and parts[0] in (b'100644',b'100755') and parts[1]==b'blob' and
                re.fullmatch(rb'[0-9a-f]{40}',parts[2]),'unsupported tree type/mode')
        path=path_bytes(name);require(path.lower() not in portable,'portable path collision')
        portable.add(path.lower());result[path]=(parts[0].decode(),parts[2].decode())
    return result


def workspace_tree(floor, root):
    result={};device=root.stat().st_dev
    for directory, dirs, files in os.walk(root,followlinks=False):
        if Path(directory)==root:dirs[:]=[d for d in dirs if d!='.git']
        for name in dirs+files:
            p=Path(directory)/name;entry=p.lstat();relative=str(p.relative_to(root));path_bytes(os.fsencode(relative))
            require(entry.st_dev==device and (stat.S_ISDIR(entry.st_mode) or stat.S_ISREG(entry.st_mode) and entry.st_nlink==1),'unsafe worker entry')
            if stat.S_ISREG(entry.st_mode):
                if any(relative==scratch or relative.startswith(scratch+'/') for scratch in floor.catalog['scratch_paths']):continue
                mode='100755' if entry.st_mode&0o111 else '100644'
                digest=hashlib.sha1(b'blob '+str(entry.st_size).encode()+b'\0')
                with p.open('rb') as handle:
                    for chunk in iter(lambda:handle.read(1<<20),b''):digest.update(chunk)
                result[relative]=(mode,digest.hexdigest())
    return result


def bounded_input(root, limit):
    """M1's in-memory pack transport is used only for bounded M2 inputs."""
    total=0;count=0
    for directory,dirs,files in os.walk(root,followlinks=False):
        for name in dirs+files:
            path=Path(directory)/name;item=path.lstat();count+=1
            relative=path.relative_to(root)
            if relative.parts[0]!='.git':path_bytes(os.fsencode(str(relative)))
            require(stat.S_ISDIR(item.st_mode) or stat.S_ISREG(item.st_mode) and item.st_nlink==1,
                    'unsupported input entry')
            total+=item.st_size
            require(count<=100000 and total<=min(MAX_INPUT_BYTES,limit),'quarantine input size/entry bound')


def matches(path, patterns, *, subtree=False):
    return any(path==p or subtree and path.startswith(p+'/') for p in patterns)


def symbols(data):
    try:
        module=ast.parse(data)
    except (SyntaxError,UnicodeError):return None
    return {node.name for node in ast.walk(module) if isinstance(node,(ast.FunctionDef,ast.AsyncFunctionDef))}


def floor_changes(floor, root, old, new, paths):
    """Exact frozen identities plus conservative flags, never semantic clearance."""
    veto=[];flags=[];catalog=floor.catalog
    old_tree=tree(floor,root,old);new_tree=tree(floor,root,new)
    for case,binding in catalog['cases'].items():
        s._exact(binding,('path','symbol'));path=binding['path']
        if path not in new_tree:
            veto.append('required case missing: '+case);continue
        if path in paths:
            data=git(floor,root,'show',new+':'+path);names=symbols(data)
            if names is None or binding['symbol'] not in names:veto.append('required case identity missing: '+case)
            elif new_tree[path]!=old_tree.get(path):flags.append('required case/assertion changed: '+case)
    for path in catalog['harnesses']:
        if path not in new_tree:veto.append('required harness deleted: '+path)
        elif path in paths and new_tree[path]!=old_tree.get(path):veto.append('required harness/binding changed: '+path)
    for path in paths:
        before=git(floor,root,'show',old+':'+path) if path in old_tree else b''
        after=git(floor,root,'show',new+':'+path) if path in new_tree else b''
        if before==after:continue
        if Path(path).name in ('Cargo.toml','Cargo.lock','package.json','package-lock.json','pubspec.yaml','pubspec.lock','requirements.txt','pyproject.toml'):
            veto.append('unauthorized dependency/capability change: '+path)
        # JSON floor fields compare structurally when present, including deletions.
        if path.endswith('.json') or before.lstrip().startswith(b'{') and after.lstrip().startswith(b'{'):
            try:
                left=c.load_json_strict(before.decode());right=c.load_json_strict(after.decode())
                protected=('required_cases','required_checks','required_check_ids','check_argv','case_inventory','expected_exit_codes','required_tests','resource_limits','command','binding',
                           'evidence_classes','acceptance_requirements','failure_tolerance','capabilities','dependencies')
                def fields(value,prefix=''):
                    if type(value) is list:
                        out={}
                        for index,item in enumerate(value):out.update(fields(item,prefix+str(index)+'.'))
                        return out
                    if type(value) is not dict:return {}
                    out={}
                    for key,v in value.items():
                        if key in protected:out[prefix+key]=v
                        out.update(fields(v,prefix+key+'.'))
                    return out
                if fields(left)!=fields(right):veto.append('measurement floor/command/evidence reduction or change: '+path)
            except (c.ContractError,UnicodeError):
                flags.append('unresolved structured floor change: '+path)
        text=before+b'\n'+after
        patterns={'assertion':rb'\bassert\b|assert[A-Z]', 'retry/timeout':rb'retr(?:y|ies)|timeout',
                  'ignored failure':rb'continue-on-error|\|\|\s*true|except\b|ignore.?error',
                  'workflow condition':rb'(?m)^\s*if:', 'mock boundary':rb'mock|test.?double|fixture_only',
                  'host executable':rb'\bPATH\b|which\(|shutil.which|/usr/(?:local/)?bin',
                  'environment/permission':rb'environment|permission|privileg|credential',
                  'feature/fallback/geometry':rb'feature|fallback|geometry|width|height|opacity',
                  'resource ownership/reuse':rb'queue|memory|descriptor|lease|reuse|ownership'}
        for label,pattern in patterns.items():
            if re.search(pattern,text,re.I):flags.append(label+': '+path)
        flags.append('unresolved material change: '+path)
    return veto,flags


def private_record(root, name):
    """Hashes bind facts only after the controller-private boundary is proven."""
    root = b._safe_path(root)
    entry = root.stat()
    require(stat.S_ISDIR(entry.st_mode) and entry.st_uid == os.geteuid() and
            stat.S_IMODE(entry.st_mode) == 0o700, 'controller-private quarantine required')
    path = root / name
    s._nofollow(path)
    fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
    with os.fdopen(fd, 'rb') as stream:
        entry = os.fstat(stream.fileno())
        require(stat.S_ISREG(entry.st_mode) and entry.st_uid == os.geteuid() and
                entry.st_nlink == 1 and stat.S_IMODE(entry.st_mode) == 0o600,
                'controller-private regular record required')
        data = stream.read(MAX_BYTES + 1)
    require(len(data) <= MAX_BYTES, 'controller record bound exceeded')
    return c.load_json_strict(data.decode())


def guarded_records(root):
    record = private_record(root, 'guard.json')
    reservation = private_record(root, 'reservation.json')
    entry = root.stat()
    identity = [entry.st_dev, entry.st_ino]
    require(record['directory_identity'] == reservation['directory_identity'] == identity,
            'quarantine reservation directory differs')
    require(all(record[key] == reservation[key] for key in
                ('nonce', 'task_id', 'floor_digest', 'base', 'head', 'source',
                 'source_device', 'source_inode')), 'guard reservation binding differs')
    return record, reservation


@dataclass(frozen=True, init=False)
class Guarded:
    floor: Floor
    root: Path
    record_json: str

    def __init__(self,floor,root,record,*,seal=None):
        require(seal is _SEAL,'guard facts must be controller observed')
        object.__setattr__(self,'floor',floor);object.__setattr__(self,'root',root)
        object.__setattr__(self,'record_json',c.canonical_json(record))

    @property
    def record(self):return c.load_json_strict(self.record_json)

    def verify(self):
        self.floor.verify();record,_=guarded_records(self.root)
        require(c.canonical_json(record)==self.record_json,'guard record drift')
        require(record['floor_digest']==self.floor.digest and
                w.manifest(self.root/'candidate')==record['manifest'],'candidate changed after guard')
        entry=self.root.stat()
        require([entry.st_dev,entry.st_ino]==record['directory_identity'],'quarantine directory replaced')
        return record


def restore_guarded(floor,destination):
    """Reload controller-private facts for a new process; no worker JSON input."""
    destination=b._safe_path(destination);floor.verify()
    record,reservation=guarded_records(destination)
    require(record['nonce']==reservation['nonce'] and record['floor_digest']==floor.digest and
            record['task_id']==floor.task['task_id'] and record['base']==floor.task['base_sha'],
            'foreign/stale guard reservation')
    receipt=private_record(destination,'candidate-receipt.json')
    require(receipt['guard_receipt_digest']==c.canonical_digest(record),'incomplete guard handoff')
    c.validate_record(receipt,'candidate_receipt',floor.verify()['schemas'],context=dict(
        task_id=floor.task['task_id'],candidate_sha=record['head'],base_sha=record['base'],
        tree_digest=record['tree_digest'],guard_receipt_digest=c.canonical_digest(record),
        authority_digest=floor.task['authority_digest'],task_contract_digest=c.canonical_digest(floor.task)))
    result=Guarded(floor,destination,record,seal=_SEAL);result.verify();return result


def inspect(candidate, floor, destination, *, source_authority=None, fault=None):
    """Consume M1's preserved object via its existing safe export/import API."""
    require(type(candidate) is b.Candidate and type(floor) is Floor,'typed M1 candidate/frozen floor required')
    payload=floor.verify();task=floor.task;started=time.monotonic()
    require(candidate.base_oid==task['base_sha'],'branch/base mismatch')
    require(candidate.authority_digest!='FIXTURE_ONLY','fixture candidate cannot certify M2 import')
    require(source_authority is not None,'original M1 import authority required')
    require(c._release_authority(source_authority)['git']['authority_digest']==candidate.authority_digest,'M1 authority differs')
    head=refs(candidate);require(head!=task['base_sha'],'empty unauthorized candidate')
    bounded_input(candidate.root,task['resource_limits']['disk_bytes'])
    destination=b._safe_path(destination)
    require(not destination.exists(),'stale quarantine reservation cannot be reused')
    for root in (candidate.root,*c.authority_roots(floor.authority)):
        require(root!=destination and root not in destination.parents and destination not in root.parents,'quarantine overlaps input/authority')
    destination.mkdir(mode=0o700,parents=True)
    record=dict(schema_version=1,nonce=uuid.uuid4().hex,task_id=task['task_id'],floor_digest=floor.digest,
                source=str(candidate.root),source_device=candidate.filesystem_identity[0],source_inode=candidate.filesystem_identity[1],
                base=task['base_sha'],head=head,phase='RESERVED',vetoes=[],flags=[])
    entry=destination.stat();record['directory_identity']=[entry.st_dev,entry.st_ino]
    write_json(destination/'reservation.json',record);s._fsync_dir(destination.parent)
    hit=fault or (lambda point:None);hit('reserved')
    original=w.manifest(candidate.root)
    # M1 transport's executable must resolve to the separately pinned trusted Git.
    require(shutil.which('git') is not None and Path(shutil.which('git')).resolve()==Path(floor.catalog['git']['executable']).resolve(), 'M1 Git transport resolution differs')
    require(b.export_candidate(candidate,destination/'candidate.pack',authority=source_authority)==head,'export identity changed')
    imported=b.import_candidate(destination/'candidate.pack',destination/'candidate',head,authority=source_authority)
    require(original==w.manifest(candidate.root),'source mutation during import')
    require(workspace_tree(floor,candidate.root)==tree(floor,imported.root,head),'dirty work preserved; immutable verification refused')
    commits=git(floor,imported.root,'rev-list','--reverse',task['base_sha']+'..'+head).splitlines()
    require(0<len(commits)<=MAX_COMMITS,'history bound/empty')
    record['commits']=[];history_bytes=0
    for raw in commits:
        oid=raw.decode('ascii');require(b._OID.fullmatch(oid),'malformed commit')
        parents=git(floor,imported.root,'rev-list','--parents','-n','1',oid).split()
        require(len(parents)==2,'unsupported merge/root history')
        parent=parents[1].decode()
        require(time.monotonic()-started<=task['resource_limits']['wall_seconds'],'quarantine wall budget exhausted')
        raw_diff=git(floor,imported.root,'diff-tree','-r','--no-commit-id','--raw','-z','--no-abbrev','-M','-C','--find-copies-harder',parent,oid)
        history_bytes+=len(raw_diff);require(history_bytes<=MAX_BYTES,'aggregate history artifact bound')
        rows=parse_raw(raw_diff)
        record['commits'].append(dict(sha=oid,parent=parent,changes=rows))
    aggregate=parse_raw(git(floor,imported.root,'diff','--raw','-z','--no-abbrev','-M','-C','--find-copies-harder',task['base_sha'],head))
    require(aggregate,'empty aggregate diff')
    for comparison in record['commits']+[dict(sha=head,parent=task['base_sha'],changes=aggregate)]:
        require(time.monotonic()-started<=task['resource_limits']['wall_seconds'],'quarantine wall budget exhausted')
        paths={p for row in comparison['changes'] for p in row['paths']}
        for row in comparison['changes']:
            for path in row['paths']:
                if matches(path,task['forbidden_paths'],subtree=True) or not matches(path,task['allowed_paths']):record['vetoes'].append('scope/protected path: '+path+' @ '+comparison['sha'])
                if row['new_mode']=='100755' and row['old_mode']!='100755' and path not in floor.catalog['executables']:
                    record['vetoes'].append('unauthorized executable mode: '+path)
        veto,flags=floor_changes(floor,imported.root,comparison['parent'],comparison['sha'],paths)
        record['vetoes'].extend(veto);record['flags'].extend(flags)
    record['vetoes']=sorted(set(record['vetoes']));record['flags']=sorted(set(record['flags']))
    require(time.monotonic()-started<=task['resource_limits']['wall_seconds'],'quarantine wall budget exhausted')
    record['duration_seconds']=time.monotonic()-started
    record.update(phase='GUARDED',tree_digest=c.canonical_digest(tree(floor,imported.root,head)),manifest=w.manifest(imported.root))
    write_json(destination/'guard.json',record);hit('guarded')
    receipt=dict(schema_version=1,task_id=task['task_id'],candidate_sha=head,base_sha=task['base_sha'],
        tree_digest=record['tree_digest'],imported=True,clean_product_tree=True,guard_receipt_digest=c.canonical_digest(record),
        authority_digest=task['authority_digest'],task_contract_digest=c.canonical_digest(task))
    c.validate_record(receipt,'candidate_receipt',payload['schemas'],context=receipt)
    write_json(destination/'candidate-receipt.json',receipt)
    return Guarded(floor,destination,record,seal=_SEAL)


def inspect_preserved(store, stage, floor, destination, *, fault=None):
    require(type(store) is s.RuntimeStore,'trusted M1 store required')
    candidate=w.restore_preserved(store,stage)
    return inspect(candidate,floor,destination,source_authority=store.authority,fault=fault)


def cleanup_quarantine(floor, destination):
    """Only exact failed/unconsumed import; original useful M1 work is retained."""
    destination=b._safe_path(destination);record=private_record(destination,'reservation.json')
    floor.verify();require(record['floor_digest']==floor.digest,'foreign quarantine')
    entry=destination.stat();require([entry.st_dev,entry.st_ino]==record['directory_identity'],'quarantine directory replaced')
    require(not (destination/'guard.json').exists(),'guarded handoff remains useful; explicit retention required')
    original=b._safe_path(Path(record['source']));entry=original.stat()
    require((entry.st_dev,entry.st_ino)==(record['source_device'],record['source_inode']),'original result missing/reused')
    # No verifier can launch without a completed sealed Guarded handle.
    shutil.rmtree(destination);s._fsync_dir(destination.parent)
