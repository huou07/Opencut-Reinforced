"""M1 controller-owned POSIX store. No dispatcher or product execution entrypoint."""
from __future__ import annotations

import copy
import fcntl
import os
import re
import stat
import subprocess
import sys
import tempfile
from contextlib import contextmanager, nullcontext
from pathlib import Path
from typing import Callable

from . import contracts as c

MAX_BYTES = 16 * 1024 * 1024
MAX_SEQUENCE = 2**50
OBJECT_KINDS = {'task_contract', 'candidate_identity', 'receipt', 'authorization', 'review', 'attempt', 'diagnostic'}
FAULT_POINTS = ('before_object_create', 'after_object_create', 'after_object_fsync',
                'before_temp_write', 'after_temp_write', 'before_replace',
                'after_replace', 'before_parent_fsync')


class StoreError(c.ContractError):
    pass


def _exact(record, keys):
    if type(record) is not dict or set(record) != set(keys):
        raise StoreError('unsupported record fields')


def _integer(value, minimum=0):
    if type(value) is not int or not minimum <= value <= MAX_SEQUENCE:
        raise StoreError('invalid integer/version/sequence')


def _id(value):
    if type(value) is not str or not re.fullmatch(r'[A-Za-z0-9_-]{1,64}', value):
        raise StoreError('unsafe identifier')


def _digest(value):
    if type(value) is not str or not re.fullmatch('[0-9a-f]{64}', value):
        raise StoreError('invalid object identity')


def _bounded(value, depth=0):
    if depth > 32:
        raise StoreError('nested record bound exceeded')
    if type(value) is dict:
        if len(value) > 4096 or any(type(k) is not str or len(k) > 4096 for k in value):
            raise StoreError('object field bound exceeded')
        for child in value.values():
            _bounded(child, depth + 1)
    elif type(value) is list:
        if len(value) > 4096:
            raise StoreError('list bound exceeded')
        for child in value:
            _bounded(child, depth + 1)
    elif type(value) is str:
        if len(value) > MAX_BYTES:
            raise StoreError('string bound exceeded')
    elif type(value) is int and not -MAX_SEQUENCE <= value <= MAX_SEQUENCE:
        raise StoreError('numeric record bound exceeded')
    elif value is not None and type(value) not in (int, float, bool):
        raise StoreError('unsupported JSON value')


def _bytes(record):
    _bounded(record)
    data = (c.canonical_json(record) + '\n').encode()
    if len(data) > MAX_BYTES:
        raise StoreError('record size bound exceeded')
    return data


def _nofollow(path):
    for part in (path, *path.parents):
        if part.is_symlink():
            raise StoreError('symlink-controlled runtime path')


def _read(path):
    _nofollow(path)
    try:
        fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW)
        with os.fdopen(fd, 'rb') as stream:
            if not stat.S_ISREG(os.fstat(stream.fileno()).st_mode):
                raise StoreError('authoritative record must be regular file')
            data = stream.read(MAX_BYTES + 1)
        if len(data) > MAX_BYTES:
            raise StoreError('record size bound exceeded')
        value = c.load_json_strict(data.decode(), str(path))
        _bounded(value)
        return value
    except (OSError, UnicodeError) as exc:
        raise StoreError('missing or unreadable authoritative record') from exc


def _fsync_dir(path):
    _nofollow(path)
    fd = os.open(path, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW)
    try:
        os.fsync(fd)
    finally:
        os.close(fd)


def filesystem_type(path):
    """Observe actual mount type; unknown/network/native Windows refuse."""
    _nofollow(path)
    if sys.platform == 'darwin':
        mounts = subprocess.check_output(['/sbin/mount'], text=True)
        candidates = []
        for line in mounts.splitlines():
            match = re.match(r'.+ on (.+) \(([^, )]+)', line)
            if match and (str(path) == match[1] or str(path).startswith(match[1].rstrip('/') + '/')):
                candidates.append((len(match[1]), match[2]))
        kind = max(candidates)[1] if candidates else 'unknown'
        supported = {'apfs', 'hfs'}
    elif sys.platform == 'linux':
        kind = subprocess.check_output(['stat', '-f', '-c', '%T', str(path)], text=True).strip()
        supported = {'ext2/ext3', 'ext4', 'xfs', 'btrfs', 'tmpfs'}
    else:
        raise StoreError('UNSUPPORTED_FILESYSTEM_PROFILE')
    if kind not in supported:
        raise StoreError('UNSUPPORTED_FILESYSTEM_PROFILE: ' + kind)
    return kind


def verify_storage(path):
    """Explicit initialization probe: real child contention, replace and fsync."""
    kind = filesystem_type(path)
    with tempfile.TemporaryDirectory(prefix='.store-probe-', dir=path) as directory:
        probe = Path(directory)
        lock = probe / 'lock'
        with lock.open('wb') as stream:
            fcntl.flock(stream, fcntl.LOCK_EX)
            code = ('import fcntl,sys; f=open(sys.argv[1],"rb"); '
                    '\ntry: fcntl.flock(f,fcntl.LOCK_EX|fcntl.LOCK_NB)'
                    '\nexcept BlockingIOError: sys.exit(23)\nsys.exit(1)')
            if subprocess.run([sys.executable, '-c', code, str(lock)], check=False).returncode != 23:
                raise StoreError('UNSUPPORTED_FLOCK_SEMANTICS')
        target = probe / 'snapshot'
        with (probe / 'temp').open('wb') as stream:
            stream.write(b'complete'); stream.flush(); os.fsync(stream.fileno())
        os.replace(probe / 'temp', target)
        _fsync_dir(probe)
        if target.read_bytes() != b'complete':
            raise StoreError('UNSUPPORTED_ATOMIC_PERSISTENCE')
    _fsync_dir(path)
    return kind


def _object(record):
    _exact(record, ('schema_version', 'kind', 'payload'))
    if type(record['schema_version']) is not int or record['schema_version'] != 1 or type(record['kind']) is not str or record['kind'] not in OBJECT_KINDS or type(record['payload']) is not dict:
        raise StoreError('unsupported immutable object schema')
    # Payload is diagnostic/data, never independently confers authorization.
    _bytes(record)
    version = record['payload'].get('schema_version', 1)
    if type(version) is not int or version != 1:
        raise StoreError('unsupported payload record version')
    return record


class RuntimeStore:
    """Trusted host API. The worker never receives this root or Python object.

    Authority stays pinned to the pristine independent M0 provenance source;
    a dirty worker clone is a separate materialization, never re-frozen here.
    """
    def __init__(self, root: Path, authority: c.ValidatedReleaseAuthority):
        self.root = Path(root).absolute()
        _nofollow(self.root)
        self.authority = authority
        self._held = []

    def _authority(self):
        # The store is an M1 capability; it may serve any executing phase at
        # or after M1. Ownership (bootstrap phase 'M1') is distinct from the
        # externally authorized executing phase validated here.
        return c.validate_shared_capability(self.authority, 'M1')

    def initialize(self):
        payload = self._authority()
        _nofollow(self.root)
        for source in c.authority_roots(self.authority):
            if self.root == source or source in self.root.parents or self.root in source.parents:
                raise StoreError('runtime must be independent of authority materializations')
        if self.root.exists():
            raise StoreError('store already exists; no implicit repair')
        filesystem_type(self.root.parent)
        self.root.mkdir(mode=0o700)
        try:
            kind = verify_storage(self.root)
            for name in ('locks', 'objects', 'artifacts', 'telemetry', 'published'):
                (self.root / name).mkdir(mode=0o700)
            for name in ('state.lock', 'integration.lock'):
                self._exclusive(self.root / 'locks' / name, b'')
            bootstrap = dict(schema_version=1, authority_digest=c.canonical_digest(payload),
                             source_sha=self.authority.source_sha, filesystem=kind,
                             phase='M1', execution='OPERATIONAL_PRODUCT_TASK' if payload['git']['purpose'] == 'OPERATIONAL' else 'DISABLED_BUILD_ONLY')
            self._exclusive(self.root / 'bootstrap.json', _bytes(bootstrap))
            snapshot = dict(schema_version=1, sequence=0, epoch=0, pause_generation=0,
                            paused=False, authority_digest=bootstrap['authority_digest'],
                            active_task=None, tasks={}, object_digests=[])
            self._exclusive(self.root / 'state.json', _bytes(snapshot))
            _fsync_dir(self.root / 'locks'); _fsync_dir(self.root)
            _fsync_dir(self.root.parent)
        except BaseException:
            # Partial initialization remains visible and refuses subsequent use.
            # Never auto-heal or erase potentially authoritative diagnostics.
            raise
        return self.inspect()

    @staticmethod
    def _exclusive(path, data):
        _nofollow(path)
        fd = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600)
        with os.fdopen(fd, 'wb') as stream:
            stream.write(data); stream.flush(); os.fsync(stream.fileno())

    def _bootstrap(self):
        payload = self._authority()
        bootstrap = _read(self.root / 'bootstrap.json')
        _exact(bootstrap, ('schema_version', 'authority_digest', 'source_sha', 'filesystem', 'phase', 'execution'))
        if type(bootstrap['schema_version']) is not int or bootstrap != dict(schema_version=1,
                authority_digest=c.canonical_digest(payload), source_sha=self.authority.source_sha,
                filesystem=filesystem_type(self.root), phase='M1', execution='OPERATIONAL_PRODUCT_TASK' if payload['git']['purpose'] == 'OPERATIONAL' else 'DISABLED_BUILD_ONLY'):
            raise StoreError('missing, drifted or malformed active authority')
        return bootstrap

    def _validate(self, state):
        _exact(state, ('schema_version', 'sequence', 'epoch', 'pause_generation', 'paused',
                       'authority_digest', 'active_task', 'tasks', 'object_digests'))
        if type(state['schema_version']) is not int or state['schema_version'] != 1:
            raise StoreError('unsupported state version')
        for name in ('sequence', 'epoch', 'pause_generation'):
            _integer(state[name])
        if state['pause_generation'] > state['sequence'] or state['epoch'] > state['sequence']:
            raise StoreError('epoch/generation exceeds authoritative sequence')
        if type(state['paused']) is not bool or type(state['tasks']) is not dict or len(state['tasks']) > 4096:
            raise StoreError('invalid state types')
        _digest(state['authority_digest'])
        refs = state['object_digests']
        if type(refs) is not list or len(refs) > 4096:
            raise StoreError('invalid object references')
        for digest in refs:
            _digest(digest)
        if len(refs) != len(set(refs)):
            raise StoreError('duplicate object references')
        for digest in refs:
            _digest(digest)
            record = _object(_read(self.root / 'objects' / (digest + '.json')))
            if c.canonical_digest(record) != digest:
                raise StoreError('immutable object identity mismatch')
        for task_id, task in state['tasks'].items():
            _id(task_id)
            _exact(task, ('contract_digest', 'candidate_digest', 'lease_epoch', 'attempt', 'stage', 'status', 'launch'))
            _integer(task['lease_epoch']); _integer(task['attempt'])
            if task['contract_digest'] not in refs or task['status'] not in ('READY', 'CLAIMED', 'SETTLED'):
                raise StoreError('missing task contract or invalid status')
            contract = _object(_read(self.root / 'objects' / (task['contract_digest'] + '.json')))
            if contract['kind'] != 'task_contract' or contract['payload'].get('task_id') != task_id:
                raise StoreError('task pointer is not original immutable contract')
            self._task(contract['payload'], contract['payload'])
            if task['lease_epoch'] > state['epoch'] or task['status'] == 'CLAIMED' and task['attempt'] == 0:
                raise StoreError('task attempt/epoch contradicts snapshot')
            if task['candidate_digest'] is not None:
                _digest(task['candidate_digest'])
                if task['candidate_digest'] not in refs:
                    raise StoreError('candidate identity is not referenced')
                candidate = _object(_read(self.root / 'objects' / (task['candidate_digest'] + '.json')))
                if candidate['kind'] != 'candidate_identity':
                    raise StoreError('invalid original candidate descriptor')
                descriptor = candidate['payload']
                _exact(descriptor, ('schema_version', 'root', 'base_oid', 'authority_digest', 'nonce', 'device', 'inode'))
                _id(descriptor['nonce']); _digest(descriptor['authority_digest'])
                _integer(descriptor['device']); _integer(descriptor['inode'])
                if type(descriptor['root']) is not str or not Path(descriptor['root']).is_absolute() or not re.fullmatch('[0-9a-f]{40}', descriptor['base_oid']):
                    raise StoreError('invalid candidate descriptor')
            if task['launch'] is not None:
                from .workspace import validate_record
                validate_record(task['launch'], task_id, task['lease_epoch'])
                launch = task['launch']
                if task['stage'] is not None and (launch['stage'] != dict(task['stage'], container_id=None)
                        or launch['container_id'] != task['stage']['container_id']):
                    raise StoreError('launch and active stage ownership differ')
                if task['candidate_digest'] is not None:
                    original = {key:value for key,value in descriptor.items() if key != 'schema_version'}
                    if original != {key:value for key,value in launch['input'].items() if key != 'manifest'}:
                        raise StoreError('launch differs from original registered candidate')
                elif launch['input']['authority_digest'] != 'FIXTURE_ONLY':
                    raise StoreError('authority-bound launch requires original candidate registration')
            if task['stage'] is not None:
                stage = task['stage']
                _exact(stage, ('task_id', 'stage_id', 'lease_epoch', 'owner_nonce', 'host_boot_identity', 'stage_nonce', 'container_id'))
                for key in ('task_id', 'stage_id', 'owner_nonce', 'stage_nonce', 'host_boot_identity'):
                    _id(stage[key])
                _integer(stage['lease_epoch'], 1)
                if stage['task_id'] != task_id or stage['lease_epoch'] != task['lease_epoch'] or task['status'] != 'CLAIMED':
                    raise StoreError('contradictory stage ownership')
                if stage['container_id'] is not None:
                    _digest(stage['container_id'])
            elif task['status'] == 'CLAIMED':
                raise StoreError('claim missing durable stage identity')
        active = state['active_task']
        claimed = [key for key, task in state['tasks'].items() if task['status'] == 'CLAIMED']
        if active is not None:
            _id(active)
        if claimed != ([] if active is None else [active]):
            raise StoreError('contradictory active task or multiple writers')
        _bytes(state)
        return state

    def inspect(self):
        """Read only: no locks, directories, repairs or runtime discovery."""
        bootstrap = self._bootstrap()
        state = self._validate(_read(self.root / 'state.json'))
        if state['authority_digest'] != bootstrap['authority_digest']:
            raise StoreError('state authority differs from original bootstrap')
        return state

    @contextmanager
    def lock(self, kind, task_id=None, blocking=True):
        self._bootstrap()
        if kind not in ('task', 'integration', 'state'):
            raise StoreError('unknown lock')
        if kind == 'task':
            _id(task_id)
        rank = {'task': 0, 'integration': 1, 'state': 2}[kind]
        if self._held and rank <= self._held[-1][0]:
            raise StoreError('lock order violation')
        name = 'task-' + task_id + '.lock' if kind == 'task' else kind + '.lock'
        path = self.root / 'locks' / name
        _nofollow(path)
        fd = os.open(path, os.O_RDWR | os.O_NOFOLLOW | (os.O_CREAT if kind == 'task' else 0), 0o600)
        try:
            if not stat.S_ISREG(os.fstat(fd).st_mode):
                raise StoreError('invalid lock file')
            fcntl.flock(fd, fcntl.LOCK_EX | (0 if blocking else fcntl.LOCK_NB))
            self._held.append((rank, task_id))
            try:
                yield
            finally:
                self._held.pop()
                fcntl.flock(fd, fcntl.LOCK_UN)
        finally:
            os.close(fd)

    def transaction(self, expected_sequence, expected_epoch, mutate: Callable, *, objects=(), fault=None):
        _integer(expected_sequence); _integer(expected_epoch)
        hit = fault or (lambda point: None)
        with self.lock('state'):
            previous = self.inspect()
            if (previous['sequence'], previous['epoch']) != (expected_sequence, expected_epoch):
                raise StoreError('stale state sequence/epoch')
            digests = []
            for record in objects:
                _object(record)
                digest = c.canonical_digest(record)
                path = self.root / 'objects' / (digest + '.json')
                _nofollow(path)
                data = _bytes(record)
                hit('before_object_create')
                try:
                    fd = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600)
                except FileExistsError:
                    if _bytes(_object(_read(path))) != data or path.read_bytes() != data:
                        raise StoreError('conflicting immutable object bytes')
                    fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW)
                try:
                    if os.fstat(fd).st_size == 0:
                        with os.fdopen(os.dup(fd), 'wb') as stream:
                            stream.write(data); stream.flush()
                    hit('after_object_create')
                    os.fsync(fd)
                    hit('after_object_fsync')
                finally:
                    os.close(fd)
                if c.canonical_digest(_object(_read(path))) != digest:
                    raise StoreError('object verification failed')
                digests.append(digest)
            _fsync_dir(self.root / 'objects')
            next_state = copy.deepcopy(previous)
            next_state['object_digests'] = list(dict.fromkeys(previous['object_digests'] + digests))
            mutate(next_state)
            if next_state['sequence'] != previous['sequence'] or next_state['authority_digest'] != previous['authority_digest']:
                raise StoreError('caller cannot choose sequence or authority')
            if next_state['epoch'] < previous['epoch'] or next_state['pause_generation'] < previous['pause_generation']:
                raise StoreError('epoch/generation cannot rewind')
            for task_id, original in previous['tasks'].items():
                task = next_state['tasks'].get(task_id)
                if (task is None or task['contract_digest'] != original['contract_digest']
                        or original['candidate_digest'] is not None and task['candidate_digest'] != original['candidate_digest']
                        or task['attempt'] < original['attempt'] or task['lease_epoch'] < original['lease_epoch']):
                    raise StoreError('original task/candidate/attempt history cannot be replaced or rewound')
            next_state['sequence'] += 1
            self._validate(next_state)
            from .workspace import validate_transition
            for task_id, original in previous['tasks'].items():
                validate_transition(original['launch'], next_state['tasks'][task_id]['launch'])
            hit('before_temp_write')
            fd, name = tempfile.mkstemp(prefix='.state-', dir=self.root)
            temp = Path(name)
            try:
                with os.fdopen(fd, 'wb') as stream:
                    stream.write(_bytes(next_state)); stream.flush()
                    hit('after_temp_write'); os.fsync(stream.fileno())
                hit('before_replace')
                os.replace(temp, self.root / 'state.json')
                hit('after_replace'); hit('before_parent_fsync')
                _fsync_dir(self.root)
            finally:
                if temp.exists():
                    temp.unlink()
            return next_state

    def set_paused(self, paused):
        if type(paused) is not bool:
            raise StoreError('pause must be boolean')
        current = self.inspect()
        def update(state):
            state['paused'] = paused
            state['pause_generation'] += 1
        return self.transaction(current['sequence'], current['epoch'], update)

    def _task(self, task, frozen_template):
        # inspect/transaction already revalidated the sealed authority source.
        payload = c.load_json_strict(self.authority.payload_json)
        c.validate_task_contract(task, payload['schemas'], frozen_template=frozen_template)
        c._phase_admission(payload, task, 'M1')

    def register_task(self, task, frozen_template):
        self._authority()
        self._task(task, frozen_template)
        if c.load_json_strict(self.authority.payload_json)['git']['purpose'] == 'OPERATIONAL':
            from .product import verify_product_base
            verify_product_base(self.authority)
        record = dict(schema_version=1, kind='task_contract', payload=task)
        digest = c.canonical_digest(record)
        current = self.inspect()
        def update(state):
            if state['tasks']:
                raise StoreError('one original task contract only; cannot re-freeze')
            state['tasks'][task['task_id']] = dict(contract_digest=digest, candidate_digest=None, lease_epoch=0, attempt=0, stage=None, status='READY', launch=None)
        self.transaction(current['sequence'], current['epoch'], update, objects=[record])
        return digest

    def register_candidate(self, task_id, candidate):
        from .sandbox import Candidate, resume_candidate
        if type(candidate) is not Candidate:
            raise StoreError('owned independent candidate required')
        resume_candidate(candidate, authority=self.authority)
        payload = self._authority()
        if candidate.authority_digest != payload['git']['authority_digest'] or candidate.base_oid != payload['git']['base_oid']:
            raise StoreError('candidate differs from original task authority/base')
        descriptor = dict(schema_version=1, root=str(candidate.root), base_oid=candidate.base_oid,
                authority_digest=candidate.authority_digest, nonce=candidate.nonce,
                device=candidate.filesystem_identity[0], inode=candidate.filesystem_identity[1])
        record = dict(schema_version=1, kind='candidate_identity', payload=descriptor)
        digest = c.canonical_digest(record)
        with nullcontext() if (0, task_id) in self._held else self.lock('task', task_id):
            current = self.inspect()
            def update(state):
                task = state['tasks'][task_id]
                if task['candidate_digest'] is not None or task['status'] != 'READY':
                    raise StoreError('candidate already frozen or stage claimed')
                task['candidate_digest'] = digest
            self.transaction(current['sequence'], current['epoch'], update, objects=[record])
        return digest

    def candidate_descriptor(self, task_id):
        _id(task_id)
        state = self.inspect()
        task = state['tasks'].get(task_id)
        if task is None or task['candidate_digest'] is None:
            raise StoreError('missing original immutable candidate identity')
        return _object(_read(self.root / 'objects' / (task['candidate_digest'] + '.json')))['payload']

    def claim(self, task_id, contract_digest, *, owner_nonce, boot_identity, stage_id, stage_nonce):
        """Reserve one stage opportunity, never launch it. Caller holds task lease."""
        if (0, task_id) not in self._held:
            raise StoreError('task lease required for entire stage/reconciliation')
        for value in (task_id, owner_nonce, boot_identity, stage_id, stage_nonce):
            _id(value)
        current = self.inspect()  # Re-read after actual OS lease acquisition.
        def update(state):
            task = state['tasks'].get(task_id)
            if state['paused'] or state['active_task'] is not None or task is None or task['status'] != 'READY' or task['contract_digest'] != contract_digest:
                raise StoreError('paused, stale, unknown or unreconciled stage')
            state['epoch'] += 1
            task['lease_epoch'] = state['epoch']; task['attempt'] += 1
            task['status'] = 'CLAIMED'; state['active_task'] = task_id
            task['stage'] = dict(task_id=task_id, stage_id=stage_id, lease_epoch=state['epoch'],
                    owner_nonce=owner_nonce, host_boot_identity=boot_identity, stage_nonce=stage_nonce, container_id=None)
        contract = _object(_read(self.root / 'objects' / (contract_digest + '.json')))['payload']
        attempt_objects = []
        if contract.get('checkpoint_id') in ('M3', 'M4', 'M5'):
            from .orchestrator import claim_attempt
            attempt_objects.append(claim_attempt(self, contract, current['epoch'] + 1))
        result = self.transaction(current['sequence'], current['epoch'], update, objects=attempt_objects)
        return result['tasks'][task_id]['stage']

    def bind_container(self, stage, container_id):
        _digest(container_id)
        current = self.inspect()
        if (0, stage['task_id']) not in self._held:
            raise StoreError('task lease required')
        def update(state):
            task = state['tasks'][stage['task_id']]
            if task['stage'] != stage or stage['container_id'] is not None:
                raise StoreError('stale stage or container already bound')
            task['stage']['container_id'] = container_id
            if task['launch'] is not None:
                task['launch']['container_id'] = container_id
        result = self.transaction(current['sequence'], current['epoch'], update)
        return result['tasks'][stage['task_id']]['stage']

    def launch_record(self, stage):
        task = self.inspect()['tasks'].get(stage['task_id'])
        if task is None or task['launch'] is None:
            raise StoreError('missing durable launch workspace locator')
        record = task['launch']
        original = dict(stage, container_id=None)
        if record['stage'] != original or task['lease_epoch'] != stage['lease_epoch'] or record['container_id'] != stage['container_id']:
            raise StoreError('wrong/stale launch task, stage, owner or epoch')
        if task['stage'] is not None and task['stage'] != stage:
            raise StoreError('stale bound container identity')
        return copy.deepcopy(record)

    def update_launch(self, stage, record, *, initial=False):
        if (0, stage['task_id']) not in self._held:
            raise StoreError('task lease required for workspace lifecycle')
        current = self.inspect()
        def update(state):
            task = state['tasks'][stage['task_id']]
            if task['stage'] != stage and task['status'] != 'SETTLED':
                raise StoreError('wrong stage during workspace lifecycle')
            if initial:
                if task['launch'] is not None or task['status'] != 'CLAIMED':
                    raise StoreError('workspace already reserved or stage not claimed')
            else:
                self.launch_record(stage)
            task['launch'] = record
        return self.transaction(current['sequence'], current['epoch'], update)

    def abort_unstarted(self, stage, record):
        if (0, stage['task_id']) not in self._held:
            raise StoreError('task lease required for prelaunch abort')
        current = self.inspect()
        def update(state):
            task = state['tasks'][stage['task_id']]
            if task['launch'] != record or record['phase'] not in ('RESERVED', 'ABORTING') or stage['container_id'] is not None:
                raise StoreError('workspace may contain worker work')
            task['launch']['phase'] = 'ABORTING'
            task['stage'] = None; task['status'] = 'SETTLED'; state['active_task'] = None
        self.transaction(current['sequence'], current['epoch'], update)
        record['phase'] = 'ABORTING'

    def settle(self, stage, reconciliation):
        """Only Docker-observed sealed termination can release a recorded writer."""
        from .sandbox import TerminatedStage
        if type(reconciliation) is not TerminatedStage or reconciliation.fixture_only or not reconciliation.matches(stage):
            raise StoreError('verified whole-stage termination required; unknown liveness blocks')
        if (0, stage['task_id']) not in self._held:
            raise StoreError('task lease required during reconciliation')
        current = self.inspect()
        def update(state):
            task = state['tasks'][stage['task_id']]
            if task['stage'] != stage:
                raise StoreError('stale epoch/owner/stage result')
            launch = task['launch']
            if launch is None or launch['phase'] != 'PRESERVED':
                raise StoreError('launch work must be durably preserved before settlement')
            from .workspace import verify_preserved
            verify_preserved(self, launch)
            task['stage'] = None; task['status'] = 'SETTLED'; state['active_task'] = None
        return self.transaction(current['sequence'], current['epoch'], update)
