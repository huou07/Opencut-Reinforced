"""CP07 × CP10 × CP11 controller-owned launch receipts and preservation.

One exact reservation per task, retained after settlement. No filesystem search,
worker metadata authority, Git reset/clean, synthetic commits or product dispatch.
"""
from __future__ import annotations

import copy
import hashlib
import os
from pathlib import Path
import shutil
import stat
import uuid

from . import contracts as c
from . import sandbox as b
from . import store as s

PHASES = ('RESERVED', 'BOUND', 'RECONCILED', 'PRESERVING', 'PRESERVED', 'CLEANING', 'REMOVED', 'ABORTING', 'ABORTED')


def validate_record(record, task_id, epoch):
    s._exact(record, ('stage', 'path', 'parent_device', 'parent_inode', 'input',
                     'container_name', 'container_id', 'daemon', 'image', 'role', 'device', 'inode', 'phase', 'result', 'mask_targets'))
    stage = record['stage']
    s._exact(stage, ('task_id', 'stage_id', 'lease_epoch', 'owner_nonce', 'host_boot_identity',
                     'stage_nonce', 'container_id'))
    if stage['task_id'] != task_id or stage['lease_epoch'] != epoch or stage['container_id'] is not None:
        raise s.StoreError('workspace stage binding differs')
    s._integer(stage['lease_epoch'], 1)
    for key in ('task_id', 'stage_id', 'owner_nonce', 'host_boot_identity', 'stage_nonce'):
        s._id(stage[key])
    if not isinstance(record['image'], str) or b._IMAGE.fullmatch(record['image']) is None:
        raise s.StoreError('missing exact cleanup image binding')
    if record['role'] not in ('IMPLEMENTATION', 'VERIFIER_CONTROLLER') or not isinstance(record['daemon'], str) or not record['daemon']:
        raise s.StoreError('missing durable runtime binding')
    if record['phase'] not in PHASES or type(record['path']) is not str:
        raise s.StoreError('invalid workspace lifecycle')
    path = Path(record['path'])
    if (not path.is_absolute() or path.name != 'candidate' or
            path.parent.parent.name != '.or-v2-launch-views' or not b._ID.fullmatch(path.parent.name)):
        raise s.StoreError('invalid exact workspace locator')
    if record['container_id'] is not None:
        s._digest(record['container_id'])
    if record['container_name'] != 'or-v2-' + path.parent.name:
        raise s.StoreError('workspace/container reservation differs')
    for key in ('parent_device', 'parent_inode'):
        s._integer(record[key], 1)
    for key in ('device', 'inode'):
        if record[key] is not None:
            s._integer(record[key], 1)
        elif record['phase'] not in ('RESERVED', 'ABORTING', 'ABORTED'):
            raise s.StoreError('missing bound workspace inode')
    s._exact(record['input'], ('root', 'base_oid', 'authority_digest', 'nonce', 'device', 'inode', 'manifest'))
    original = record['input']
    if (type(original['root']) is not str or not Path(original['root']).is_absolute()
            or type(original['base_oid']) is not str or b._OID.fullmatch(original['base_oid']) is None):
        raise s.StoreError('invalid original input binding')
    s._id(original['nonce']);s._digest(original['manifest'])
    for key in ('device', 'inode'):
        s._integer(original[key], 1)
    if original['authority_digest'] != 'FIXTURE_ONLY':
        s._digest(original['authority_digest'])

    if record['phase'] in ('PRESERVING','PRESERVED','CLEANING','REMOVED') and record['result'] is None:
        raise s.StoreError('missing durable preservation locator')
    if type(record['mask_targets']) is not list or len(record['mask_targets']) > 3:
        raise s.StoreError('invalid controller mask targets')
    names = []
    for target in record['mask_targets']:
        s._exact(target, ('name', 'device', 'inode'))
        if target['name'] not in ('opencode.json','opencode.jsonc','.opencode') or target['name'] in names:
            raise s.StoreError('invalid mask target binding')
        names.append(target['name'])
        for key in ('device','inode'):
            if target[key] is not None:
                s._integer(target[key],1)
            elif record['phase'] not in ('RESERVED','ABORTING','ABORTED'):
                raise s.StoreError('missing controller target filesystem binding')
    if record['result'] is not None:
        s._exact(record['result'], ('path', 'manifest', 'head'))
        if record['result']['manifest'] is not None:
            s._digest(record['result']['manifest'])
        if record['result']['head'] is not None and b._OID.fullmatch(record['result']['head']) is None:
            raise s.StoreError('invalid preserved commit identity')


def validate_transition(old, new):
    if old is None:
        if new is not None and new['phase'] != 'RESERVED':
            raise s.StoreError('workspace must begin with durable reservation')
        return
    if new is None:
        raise s.StoreError('durable workspace history cannot be discarded')
    for key in ('stage', 'path', 'parent_device', 'parent_inode', 'input', 'container_name', 'daemon', 'image', 'role'):
        if old[key] != new[key]:
            raise s.StoreError('workspace durable binding cannot change')
    if old['container_id'] is not None and old['container_id'] != new['container_id']:
        raise s.StoreError('bound container history cannot change')
    if [item['name'] for item in old['mask_targets']] != [item['name'] for item in new['mask_targets']]:
        raise s.StoreError('controller mask target inventory cannot change')
    if old['phase'] != 'RESERVED' and old['mask_targets'] != new['mask_targets']:
        raise s.StoreError('controller mask filesystem bindings cannot change')
    if old['device'] is not None and (old['device'],old['inode']) != (new['device'],new['inode']):
        raise s.StoreError('bound filesystem identity cannot change')
    transitions = {'RESERVED':('RESERVED','BOUND','ABORTING'), 'BOUND':('BOUND','RECONCILED'),
                   'RECONCILED':('RECONCILED','PRESERVING'), 'PRESERVING':('PRESERVING','PRESERVED'),
                   'PRESERVED':('PRESERVED','CLEANING'), 'CLEANING':('CLEANING','REMOVED'),
                   'REMOVED':('REMOVED',), 'ABORTING':('ABORTING','ABORTED'), 'ABORTED':('ABORTED',)}
    if new['phase'] not in transitions[old['phase']]:
        raise s.StoreError('workspace lifecycle cannot skip or rewind')
    if old['phase'] in ('PRESERVED','CLEANING','REMOVED') and old['result'] != new['result']:
        raise s.StoreError('verified useful handoff cannot change')


def _parent(record):
    path = b._safe_path(Path(record['path']))
    parent = path.parent.parent
    item = parent.stat()
    b._require((item.st_dev, item.st_ino) == (record['parent_device'], record['parent_inode'])
               and item.st_uid == os.geteuid() and stat.S_IMODE(item.st_mode) == 0o700,
               'workspace controller-private parent changed')
    return path


def restore_launch(store, stage):
    b._require(type(store) is s.RuntimeStore, 'trusted runtime store required')
    record = store.launch_record(stage)
    b._require(record['phase'] in ('BOUND', 'RECONCILED', 'PRESERVING', 'PRESERVED'),
               'workspace is incomplete, stale or consumed')
    path = _parent(record)
    b._require(path.is_dir(), 'durable launch workspace was deleted')
    item = path.stat()
    b._require((item.st_dev, item.st_ino) == (record['device'], record['inode']), 'workspace path reused')
    original = record['input']
    candidate = b.Candidate(path, original['base_oid'], original['authority_digest'], original['nonce'], _seal=b._SEAL)
    candidate.verify()
    return candidate


def reserve_launch(store, stage, candidate, storage_root, daemon, image, role):
    b._require(type(store) is s.RuntimeStore and type(candidate) is b.Candidate, 'owned workspace input required')
    candidate.verify()
    for authority_root in (store.root, Path(store.authority.candidate_root), Path(store.authority.controller_root)):
        root = b._safe_path(authority_root)
        b._require(candidate.root != root and root not in candidate.root.parents and candidate.root not in root.parents,
                   'launch input overlaps controller authority/storage')
    storage_root = b._safe_path(storage_root)
    b._require(candidate.root.parent == storage_root, 'workspace must share bounded candidate storage')
    parent = storage_root / '.or-v2-launch-views'
    if not parent.exists():
        parent.mkdir(mode=0o700)
        s._fsync_dir(storage_root)
    item = parent.stat()
    b._require(item.st_uid == os.geteuid() and stat.S_IMODE(item.st_mode) == 0o700, 'private launch parent required')
    nonce = uuid.uuid4().hex
    path = parent / nonce / 'candidate'
    record = dict(stage=copy.deepcopy(stage), path=str(path), parent_device=item.st_dev,
                  parent_inode=item.st_ino, container_name='or-v2-' + nonce, container_id=None, device=None, inode=None,
                  phase='RESERVED', result=None, mask_targets=[dict(name=name,device=None,inode=None)
                  for name in ('opencode.json','opencode.jsonc','.opencode') if not (candidate.root/name).exists()], daemon=daemon, image=image, role=role, input=dict(root=str(candidate.root), base_oid=candidate.base_oid,
                  authority_digest=candidate.authority_digest, nonce=candidate.nonce,
                  device=candidate.filesystem_identity[0], inode=candidate.filesystem_identity[1], manifest=manifest(candidate.root)))
    b._require(stage['container_id'] is None, 'reserve workspace before container creation')
    store.update_launch(stage, record, initial=True)  # fsynced before mkdir/copy
    return path


def bind_launch(store, stage):
    record = store.launch_record(stage)
    b._require(record['phase'] == 'RESERVED', 'workspace already materialized')
    path = _parent(record)
    item = path.stat()
    for target in record['mask_targets']:
        entry = (path/target['name']).stat()
        target.update(device=entry.st_dev,inode=entry.st_ino)
    b.Candidate(path, record['input']['base_oid'], record['input']['authority_digest'],
                record['input']['nonce'], _seal=b._SEAL).verify()
    flush_tree(path)
    s._fsync_dir(path.parent); s._fsync_dir(path.parent.parent)
    record.update(device=item.st_dev, inode=item.st_ino, phase='BOUND')
    store.update_launch(stage, record)


def file_digest(path):
    digest = hashlib.sha256()
    with path.open('rb') as stream:
        for chunk in iter(lambda:stream.read(1<<20), b''):
            digest.update(chunk)
    return digest.hexdigest()


def manifest(path):
    """Stream regular-file content and framed entries, without loading media."""
    digest = hashlib.sha256()
    device = path.stat().st_dev
    for directory, dirs, files in os.walk(path, followlinks=False):
        dirs.sort()
        for name in sorted(dirs+files):
            item=Path(directory)/name;entry=item.lstat()
            b._require(entry.st_dev==device and (stat.S_ISDIR(entry.st_mode) or
                       stat.S_ISREG(entry.st_mode) and entry.st_nlink==1), 'unsafe preservation entry')
            row=c.canonical_json([str(item.relative_to(path)),stat.S_IMODE(entry.st_mode),
                                  file_digest(item) if stat.S_ISREG(entry.st_mode) else None]).encode()
            digest.update(len(row).to_bytes(8,'big'));digest.update(row)
    return digest.hexdigest()


def is_input_prefix(source, partial):
    with source.open('rb') as original, partial.open('rb') as copied:
        for chunk in iter(lambda:copied.read(1<<20), b''):
            if original.read(len(chunk))!=chunk:
                return False
    return True


def flush_tree(path):
    for directory, _, files in os.walk(path, topdown=False):
        for name in files:
            fd = os.open(Path(directory) / name, os.O_RDONLY | os.O_NOFOLLOW)
            try:
                os.fsync(fd)
            finally:
                os.close(fd)
        s._fsync_dir(Path(directory))


def verify_preserved(store, record):
    result = record['result']
    expected = store.root / 'artifacts' / record['container_name'] / 'candidate'
    b._require(result is not None and result['path'] == str(expected), 'forged preservation path')
    path = b._safe_path(expected)
    b._require(path.is_dir() and result['manifest'] == manifest(path), 'preserved work missing or changed')
    return path


def original_input(record):
    original = record['input']
    path = b._safe_path(Path(original['root']))
    item = path.stat()
    b._require((item.st_dev, item.st_ino) == (original['device'], original['inode']) and
               manifest(path) == original['manifest'], 'original input missing, reused or changed')
    candidate = b.Candidate(path, original['base_oid'], original['authority_digest'], original['nonce'], _seal=b._SEAL)
    candidate.verify()
    return candidate


def abort_unstarted_launch(store, stage, sandbox, proof):
    """Reclaim an interrupted/full-disk copy without needing another full copy.

    Only RESERVED can abort: BOUND is the durable gate before Docker create.
    The original exact snapshot remains the useful result. Extra or divergent
    partial bytes refuse deletion and remain reachable through the receipt.
    """
    b._require(type(proof) is b.TerminatedStage and not proof.fixture_only and proof.matches(stage)
               and stage['container_id'] is None, 'real absence proof required for prelaunch abort')
    record = store.launch_record(stage)
    b._require(proof.workspace_path == record['path'], 'absence proof lacks workspace binding')
    b._require(record['phase'] in ('RESERVED', 'ABORTING'), 'workspace may have been used by a worker')
    original = original_input(record)
    path = _parent(record)
    sandbox.verify_no_launch_users(store, stage)
    if path.exists():
        for item in path.rglob('*'):
            status = item.lstat();source = original.root / item.relative_to(path)
            b._require(status.st_dev == record['parent_device'] and not item.is_symlink(), 'unsafe partial copy')
            if str(item.relative_to(path)) in [target['name'] for target in record['mask_targets']]:
                b._require(item.is_dir() and not any(item.iterdir()) or item.is_file() and item.stat().st_size==0,
                           'unstarted mask target contains useful work')
                continue
            if item.is_dir():
                b._require(source.is_dir(), 'unrecognized partial directory')
            else:
                b._require(item.is_file() and status.st_nlink == 1 and source.is_file()
                           and is_input_prefix(source,item), 'partial copy contains useful divergent work')
    store.abort_unstarted(stage, record)
    if path.exists():
        shutil.rmtree(path)
    if path.parent.exists():
        path.parent.rmdir()
    s._fsync_dir(path.parent.parent)
    record['phase'] = 'ABORTED';store.update_launch(stage, record)


def restore_preserved(store, stage):
    record = store.launch_record(stage)
    if record['phase'] == 'ABORTED':
        return original_input(record)
    b._require(record['phase'] in ('PRESERVED', 'CLEANING', 'REMOVED'), 'handoff not durable')
    path = verify_preserved(store, record)
    original = record['input']
    candidate = b.Candidate(path, original['base_oid'], original['authority_digest'], original['nonce'], _seal=b._SEAL)
    candidate.verify()
    return candidate


def strip_controller_mask_targets(record, candidate):
    # These empty placeholders existed only to prevent Docker's bind-mount
    # creation from altering the supplied input. They were OS read-only while
    # the worker ran. Never remove an original or worker-modified entry.
    for target in record['mask_targets']:
        path = candidate.root/target['name']
        if not path.exists():
            continue  # Resume after a crash between unlink and phase persistence.
        entry = path.lstat()
        b._require((entry.st_dev,entry.st_ino)==(target['device'],target['inode']), 'controller mask target reused')
        if target['name']=='.opencode':
            b._require(path.is_dir() and not any(path.iterdir()), 'mask target contains useful work')
            path.rmdir()
        else:
            b._require(path.is_file() and entry.st_nlink==1 and entry.st_size==0, 'mask target contains useful work')
            path.unlink()
    s._fsync_dir(candidate.root)


def preserve_launch(store, stage, proof):
    b._require(type(proof) is b.TerminatedStage and not proof.fixture_only and proof.matches(stage),
               'real entire-stage reconciliation required before preserving work')
    record = store.launch_record(stage)
    b._require(proof.workspace_path == record['path'], 'termination proof lacks durable workspace binding')
    if record['phase'] == 'PRESERVED':
        return verify_preserved(store, record)
    b._require(record['phase'] in ('BOUND', 'RECONCILED', 'PRESERVING'), 'workspace cannot be preserved in this state')
    candidate = restore_launch(store, stage)
    if record['phase'] == 'BOUND':
        record['phase'] = 'RECONCILED'; store.update_launch(stage, record)
    strip_controller_mask_targets(record, candidate)
    path = store.root / 'artifacts' / record['container_name'] / 'candidate'
    record.update(phase='PRESERVING', result=dict(path=str(path), manifest=None, head=None))
    store.update_launch(stage, record)  # locator before artifact creation
    b._safe_path(path)
    path.parent.mkdir(mode=0o700, exist_ok=True)
    # A partial controller-owned copy may be finished after a crash. The exact
    # reconciled source remains intact until the verified snapshot is durable.
    shutil.copytree(candidate.root, path, dirs_exist_ok=True)
    flush_tree(path); s._fsync_dir(path.parent); s._fsync_dir(path.parent.parent)
    source_hash = manifest(candidate.root)
    b._require(manifest(path) == source_hash, 'preservation copy differs')
    pack = path.parent / 'commit.pack'
    temporary = path.parent / '.commit.pack'
    # Only this unconsumed temporary transport may be replaced: full useful
    # work exists in both exact source and fsynced preservation snapshot.
    if temporary.exists():
        temporary.unlink()
    head = b.export_candidate(candidate, temporary, authority=store.authority if candidate.authority_digest != 'FIXTURE_ONLY' else None)
    os.replace(temporary, pack)
    s._fsync_dir(path.parent)
    record.update(phase='PRESERVED', result=dict(path=str(path), manifest=source_hash, head=head))
    store.update_launch(stage, record)
    return path


def cleanup_launch(store, stage, sandbox, proof, fault=None):
    b._require(type(proof) is b.TerminatedStage and not proof.fixture_only and proof.matches(stage), 'real termination proof required')
    record = store.launch_record(stage)
    b._require(proof.workspace_path == record['path'], 'cleanup proof lacks workspace binding')
    task = store.inspect()['tasks'][stage['task_id']]
    b._require(task['status'] == 'SETTLED' and task['stage'] is None, 'active lease prevents cleanup')
    verify_preserved(store, record)  # dirty files AND clean commits are now consumed by durable handoff
    if record['phase'] == 'REMOVED':
        return
    b._require(record['phase'] in ('PRESERVED', 'CLEANING'), 'cleanup not eligible')
    # Re-observe before removal. Container deletion is AFTER durable handoff.
    sandbox.reconcile_launch(store, stage)
    record['phase'] = 'CLEANING'; store.update_launch(stage, record)
    sandbox.remove_launch_container(store, stage)
    path = _parent(record)
    if path.exists():
        sandbox.erase_retired_launch(store, stage, fault=fault)
    sandbox.verify_no_launch_users(store, stage)
    if path.exists():
        item = path.stat()
        b._require((item.st_dev, item.st_ino) == (record['device'], record['inode']), 'cleanup path reused')
        shutil.rmtree(path)
    if path.parent.exists():
        path.parent.rmdir()
    s._fsync_dir(path.parent.parent)
    record['phase'] = 'REMOVED'; store.update_launch(stage, record)
