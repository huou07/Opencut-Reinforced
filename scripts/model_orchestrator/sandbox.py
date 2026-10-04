"""Disabled M1 isolation primitives; no product/model dispatch entrypoint.

The only runnable command adapter in M1 is an explicitly owned shell fixture.
OpenCode remains unavailable until M3/M5 pins and verifies its installed adapter.
A transport fake exercises decisions, never certifies a live container boundary.
"""
from __future__ import annotations

from dataclasses import dataclass
import hashlib
import os
from pathlib import Path
import re
import shutil
import stat
import subprocess
import tempfile
import time
from typing import Callable, Any
import uuid

from .contracts import (ContractError, ValidatedReleaseAuthority, _release_authority,
                        canonical_digest, canonical_json, load_json_strict,
                        validate_shared_capability)

UNAVAILABLE = 'M1_ISOLATION_ENVIRONMENT_UNAVAILABLE'
_SEAL = object()
_ID = re.compile(r'[A-Za-z0-9_-]{1,64}\Z')
_OID = re.compile(r'[0-9a-f]{40}\Z')
_CONTAINER = re.compile(r'[0-9a-f]{64}\Z')
_IMAGE = re.compile(r'sha256:[0-9a-f]{64}\Z')
_ROLES = {'IMPLEMENTATION', 'INVESTIGATION_REVIEW', 'ROUTER_TRIAGE', 'ARCHITECTURE',
          'DISPATCHER', 'VERIFIER_CONTROLLER'}


class SandboxError(ContractError):
    def __init__(self, message: str, code: str = UNAVAILABLE):
        self.code = code
        super().__init__(message)


def _require(condition: bool, message: str) -> None:
    if not condition:
        raise SandboxError(message)


def _safe_path(path: Path) -> Path:
    path = Path(path).absolute()
    _require(not any(p.is_symlink() for p in (path, *path.parents)), 'symlink path forbidden')
    _require(not any(c in str(path) for c in ('\x00', ',', '\n', '\r')), 'unsupported mount path')
    return path.resolve()


def _authority(authority: ValidatedReleaseAuthority) -> dict[str, Any]:
    # The sandbox is an M1 capability; it may serve any executing phase at or
    # after M1 under exact authorized progress, including later M3/M4/M5 tasks.
    return validate_shared_capability(authority, 'M1')


def _git(root: Path, *args: str, input_bytes: bytes | None = None) -> bytes:
    """Only invoke on controller-owned Git/config, never on worker .git."""
    env = {'PATH': os.environ.get('PATH', ''), 'LANG': 'C', 'LC_ALL': 'C',
           'GIT_CONFIG_NOSYSTEM': '1', 'GIT_CONFIG_GLOBAL': os.devnull,
           'GIT_NO_REPLACE_OBJECTS': '1', 'GIT_NO_LAZY_FETCH': '1', 'GIT_TERMINAL_PROMPT': '0'}
    try:
        result = subprocess.run(['git', '--no-optional-locks', '-c', 'core.hooksPath=' + os.devnull,
                                 '-c', 'core.fsmonitor=false', '-c', 'core.attributesFile=' + os.devnull,
                                 '-c', 'safe.directory=' + str(root),
                                 '-c', 'protocol.file.allow=never', *args], cwd=root, env=env,
                                input=input_bytes, capture_output=True, timeout=60, check=True)
    except (OSError, subprocess.SubprocessError) as exc:
        raise SandboxError('sanitized Git operation failed: ' + args[0]) from exc
    _require(len(result.stdout) <= 64 * 1024 * 1024, 'Git output bound exceeded')
    return result.stdout


@dataclass(frozen=True, init=False)
class Candidate:
    root: Path
    base_oid: str
    authority_digest: str
    nonce: str
    filesystem_identity: tuple[int, int]

    def __init__(self, root: Path, base_oid: str, authority_digest: str, nonce: str, *, _seal: object = None):
        _require(_seal is _SEAL, 'candidate requires independently owned clone construction')
        object.__setattr__(self, 'root', root)
        object.__setattr__(self, 'base_oid', base_oid)
        object.__setattr__(self, 'authority_digest', authority_digest)
        object.__setattr__(self, 'nonce', nonce)
        status = root.stat()
        object.__setattr__(self, 'filesystem_identity', (status.st_dev, status.st_ino))

    def verify(self) -> None:
        root = _safe_path(self.root)
        status = root.stat()
        _require((status.st_dev, status.st_ino) == self.filesystem_identity, 'candidate directory identity changed')
        inspect_candidate(root)
        for directory, dirs, files in os.walk(root, followlinks=False):
            for name in dirs + files:
                path = Path(directory) / name
                item = path.lstat()
                _require(item.st_dev == status.st_dev, 'nested host filesystem forbidden')
                _require(stat.S_ISREG(item.st_mode) or stat.S_ISDIR(item.st_mode),
                         'candidate socket/device/symlink forbidden')
                _require(not stat.S_ISREG(item.st_mode) or item.st_nlink == 1, 'candidate host hardlink forbidden')


def inspect_candidate(root: Path) -> None:
    """Reject shared-worktree/object and local/credential-bearing remote topology."""
    root = _safe_path(root)
    git = root / '.git'
    _require(git.is_dir() and not git.is_symlink(), 'candidate must own independent .git directory')
    for name in ('commondir', 'gitdir', 'shallow', 'info/grafts', 'objects/info/alternates',
                 'objects/info/http-alternates', 'refs/replace'):
        _require(not (git / name).exists() and not (git / name).is_symlink(), 'shared/replaced Git storage forbidden')
    for directory, dirs, files in os.walk(git, followlinks=False):
        for name in dirs + files:
            path = Path(directory) / name
            _require(not path.is_symlink(), 'Git storage symlink forbidden')
            if path.is_file():
                _require(path.stat().st_nlink == 1, 'shared Git hardlink forbidden')
    config = git / 'config'
    _require(config.is_file() and config.stat().st_size <= 65536, 'invalid candidate Git config')
    # Parse as data: even `git config` would read attacker-selected include paths.
    text = config.read_text(encoding='utf-8')
    _require(not re.search(r'(?im)^\s*\[\s*(remote|include|includeIf|credential|url|submodule)\b', text),
             'candidate remotes/includes/credentials forbidden')
    packed = git / 'packed-refs'
    if packed.exists():
        _require(packed.stat().st_size <= 4 * 1024 * 1024, 'oversized packed refs')
        _require(b'refs/replace/' not in packed.read_bytes(), 'packed replacement refs forbidden')


def _create_candidate(source: Path, destination: Path, base_oid: str, binding: str) -> Candidate:
    source, destination = _safe_path(source), _safe_path(destination)
    _require(_OID.fullmatch(base_oid) is not None, 'exact commit OID required')
    _require(source != destination and source not in destination.parents and destination not in source.parents,
             'candidate and authority must be disjoint')
    _require(not destination.exists() or destination.is_dir() and not any(destination.iterdir()),
             'destination must be empty; existing candidates are preserved')
    _require(_git(source, 'cat-file', '-t', base_oid).strip() == b'commit', 'base must be commit')
    pack = _git(source, 'pack-objects', '--stdout', '--revs', input_bytes=(base_oid + '\n').encode())
    destination.mkdir(parents=True, exist_ok=True)
    _git(destination, 'init', '--initial-branch=candidate')
    _git(destination, 'index-pack', '--stdin', input_bytes=pack)
    _git(destination, 'update-ref', 'refs/heads/candidate', base_oid)
    _git(destination, 'checkout', '--force', 'candidate')
    inspect_candidate(destination)
    return Candidate(destination, base_oid, binding, uuid.uuid4().hex, _seal=_SEAL)


def create_candidate(source: Path, destination: Path, base_oid: str, *,
                     authority: ValidatedReleaseAuthority) -> Candidate:
    payload = _authority(authority)
    _require(_safe_path(source) == Path(authority.candidate_root), 'base source must be pinned authority')
    _require(base_oid == payload['git']['base_oid'], 'base differs from original authority')
    return _create_candidate(source, destination, base_oid, payload['git']['authority_digest'])


def create_fixture_candidate(source: Path, destination: Path, base_oid: str) -> Candidate:
    """Explicit fixture-only API. Its result cannot enter an authority-bound stage."""
    source = _safe_path(source)
    _require((source / '.git').is_dir(), 'independent fixture repository required')
    _require(not (source / 'docs/execution/PLAN.json').exists(), 'product tree is not a shell fixture')
    inspect_candidate(source)
    return _create_candidate(source, destination, base_oid, 'FIXTURE_ONLY')


def export_candidate(candidate: Candidate, destination: Path, *,
                     authority: ValidatedReleaseAuthority | None = None) -> str:
    """Copy objects/refs to fresh quarantine before Git reads them; preserve dirty tree.

    This is transport, not M2 path/commit authorization. Never reset or clean the
    candidate, and never execute its config, hooks, filters, index or git binary.
    """
    if candidate.authority_digest != 'FIXTURE_ONLY':
        _require(_authority(authority)['git']['authority_digest'] == candidate.authority_digest,
                 'original authority changed')
    _require(type(candidate) is Candidate, 'owned candidate required')
    candidate.verify()
    destination = _safe_path(destination)
    _require(not destination.exists(), 'export must not overwrite a preserved artifact')
    with tempfile.TemporaryDirectory(prefix='or-m1-quarantine-') as name:
        quarantine = Path(name)
        _git(quarantine, 'init', '--initial-branch=candidate')
        source_git = candidate.root / '.git'
        shutil.copytree(source_git / 'objects', quarantine / '.git/objects', dirs_exist_ok=True)
        head = (source_git / 'HEAD').read_text(encoding='ascii').strip()
        if head.startswith('ref: '):
            ref = head[5:]
            _require(ref == 'refs/heads/candidate', 'unexpected candidate branch')
            loose = source_git / ref
            if loose.is_file():
                head = loose.read_text(encoding='ascii').strip()
            else:
                lines = (source_git / 'packed-refs').read_text(encoding='ascii').splitlines()
                matches = [line.split(' ')[0] for line in lines if line.endswith(' ' + ref)]
                _require(len(matches) == 1, 'candidate branch unavailable')
                head = matches[0]
        _require(_OID.fullmatch(head) is not None, 'invalid candidate commit')
        _require(_git(quarantine, 'cat-file', '-t', head).strip() == b'commit', 'candidate is not commit')
        _git(quarantine, 'fsck', '--full', '--strict', '--no-reflogs')
        _git(quarantine, 'merge-base', '--is-ancestor', candidate.base_oid, head)
        pack = _git(quarantine, 'pack-objects', '--stdout', '--revs', input_bytes=(head + '\n').encode())
        with destination.open('xb') as handle:
            handle.write(pack)
            handle.flush()
            os.fsync(handle.fileno())
    return head


def import_candidate(pack_path: Path, destination: Path, head_oid: str, *,
                     authority: ValidatedReleaseAuthority) -> Candidate:
    """Import credentialless pack bytes into a fresh independent Git directory.

    M2 must still authorize every commit/path before verification or promotion.
    Importing a clean commit does not dispatch another worker or alter dirty work.
    """
    payload = _authority(authority)
    base = payload['git']['base_oid']
    pack_path, destination = _safe_path(pack_path), _safe_path(destination)
    _require(_OID.fullmatch(head_oid) is not None, 'exact candidate commit required')
    _require(pack_path.is_file() and pack_path.stat().st_size <= 64 << 20, 'invalid/oversized candidate pack')
    _require(not destination.exists(), 'import must not replace preserved candidate')
    for authority_root in (Path(authority.candidate_root), Path(authority.controller_root)):
        _require(destination != authority_root and authority_root not in destination.parents
                 and destination not in authority_root.parents, 'import cannot touch authority storage')
    destination.mkdir(parents=True)
    _git(destination, 'init', '--initial-branch=candidate')
    _git(destination, 'index-pack', '--strict', '--stdin', input_bytes=pack_path.read_bytes())
    _git(destination, 'fsck', '--full', '--strict', '--no-reflogs')
    _require(_git(destination, 'cat-file', '-t', head_oid).strip() == b'commit', 'import head is not commit')
    _git(destination, 'merge-base', '--is-ancestor', base, head_oid)
    _git(destination, 'update-ref', 'refs/heads/candidate', head_oid)
    _git(destination, 'checkout', '--force', 'candidate')
    return Candidate(destination, base, payload['git']['authority_digest'], uuid.uuid4().hex, _seal=_SEAL)


def resume_candidate(candidate: Candidate, *, authority: ValidatedReleaseAuthority | None = None) -> Candidate:
    """Retain the original handle/binding and dirty files; never refreeze authority."""
    _require(type(candidate) is Candidate, 'original owned candidate handle required')
    if candidate.authority_digest != 'FIXTURE_ONLY':
        payload = _authority(authority)
        _require(payload['git']['authority_digest'] == candidate.authority_digest
                 and payload['git']['base_oid'] == candidate.base_oid, 'original authority drifted')
    candidate.verify()
    return candidate


def restore_candidate(store: Any, task_id: str) -> Candidate:
    """Recover only the original store-owned locator, including dirty/committed work.

    A missing directory/descriptor or changed device/inode stops recovery. No
    reset, clean, clone replacement or post-worker authority freeze is allowed.
    """
    from .store import RuntimeStore
    _require(type(store) is RuntimeStore, 'candidate recovery requires trusted typed runtime store')
    _require(isinstance(task_id, str) and _ID.fullmatch(task_id) is not None, 'invalid task identity')
    authority = _authority(store.authority)
    descriptor = store.candidate_descriptor(task_id)
    _require(type(descriptor) is dict and set(descriptor) ==
             {'schema_version', 'root', 'base_oid', 'authority_digest', 'nonce', 'device', 'inode'},
             'invalid persisted candidate descriptor')
    _require(type(descriptor['schema_version']) is int and descriptor['schema_version'] == 1,
             'unsupported candidate descriptor')
    _require(descriptor['base_oid'] == authority['git']['base_oid'] and
             descriptor['authority_digest'] == authority['git']['authority_digest'],
             'candidate original authority/base differs')
    _require(isinstance(descriptor['nonce'], str) and _ID.fullmatch(descriptor['nonce']) is not None,
             'invalid original candidate nonce')
    _require(isinstance(descriptor['root'], str) and len(descriptor['root']) <= 4096
             and Path(descriptor['root']).is_absolute(), 'invalid original candidate locator')
    for name in ('device', 'inode'):
        _require(type(descriptor[name]) is int and descriptor[name] > 0, 'invalid candidate filesystem identity')
    root = _safe_path(Path(descriptor['root']))
    _require(root.is_dir(), 'preserved candidate missing; recovery cannot recreate it')
    status = root.stat()
    _require((status.st_dev, status.st_ino) == (descriptor['device'], descriptor['inode']),
             'preserved candidate filesystem identity changed')
    for authority_root in (Path(store.authority.candidate_root), Path(store.authority.controller_root), store.root):
        authority_root = authority_root.resolve()
        _require(root != authority_root and root not in authority_root.parents and authority_root not in root.parents,
                 'candidate recovery overlaps authority/runtime')
    candidate = Candidate(root, descriptor['base_oid'], descriptor['authority_digest'], descriptor['nonce'], _seal=_SEAL)
    candidate.verify()
    return candidate


@dataclass(frozen=True)
class Limits:
    cpu: int
    memory_bytes: int
    pids: int
    candidate_bytes: int
    scratch_bytes: int
    output_bytes: int
    wall_seconds: int

    def validate(self) -> None:
        bounds = {'cpu': 64, 'memory_bytes': 64 << 30, 'pids': 4096,
                  'candidate_bytes': 64 << 30, 'scratch_bytes': 8 << 30,
                  'output_bytes': 64 << 20, 'wall_seconds': 3600}
        for key, maximum in bounds.items():
            value = getattr(self, key)
            _require(type(value) is int and 1 <= value <= maximum, 'unbounded/invalid ' + key)


def role_policy(role: str) -> dict[str, Any]:
    _require(role in _ROLES, 'unsupported model role')
    permissions = {'*': 'deny', 'task': 'deny', 'external_directory': 'deny'}
    if role == 'IMPLEMENTATION':
        permissions.update(read='allow', edit='allow', glob='allow', grep='allow', bash='allow')
    elif role == 'VERIFIER_CONTROLLER':
        permissions.update(read='allow', glob='allow', grep='allow', bash='allow')
    elif role == 'INVESTIGATION_REVIEW':
        permissions.update(read='allow', glob='allow', grep='allow')
    return {'permission': permissions, 'plugin': [], 'mcp': {}, 'lsp': False, 'formatter': False}


def write_role_overlays(destination: Path, role: str) -> dict[str, Path]:
    """Create immutable launch-view inputs separately; never edit candidate files.

    These are not adapter certification. M1 refuses OpenCode execution because
    no pinned installed version/config discovery conformance is present yet.
    """
    destination = _safe_path(destination)
    _require(not destination.exists(), 'overlay destination must be new')
    destination.mkdir(mode=0o700, parents=True)
    policy = canonical_json(role_policy(role)) + '\n'
    for name in ('opencode.json', 'opencode.jsonc'):
        (destination / name).write_text(policy, encoding='utf-8')
        (destination / name).chmod(0o444)
    project_config = destination / '.opencode'
    project_config.mkdir(mode=0o555)
    home = destination / 'home'
    (home / '.config' / 'opencode').mkdir(parents=True)
    (home / '.local' / 'share' / 'opencode').mkdir(parents=True)
    (home / '.local' / 'state' / 'opencode').mkdir(parents=True)
    (home / 'opencode.json').write_text(policy, encoding='utf-8')
    (home / '.config' / 'opencode' / 'opencode.json').write_text(policy, encoding='utf-8')
    for path in (home / 'opencode.json', home / '.config' / 'opencode' / 'opencode.json'):
        path.chmod(0o444)
    for directory, dirs, _ in os.walk(home, topdown=False):
        for name in dirs:
            (Path(directory) / name).chmod(0o555)
        Path(directory).chmod(0o555)
    destination.chmod(0o555)
    return {name: destination / name for name in ('opencode.json', 'opencode.jsonc', '.opencode', 'home')}


def _validated_role_overlays(overlays: dict[str, Path], role: str, candidate: Path) -> list[tuple[Path, str]]:
    """Accept only the exact controller-generated, immutable policy tree."""
    names = ('opencode.json', 'opencode.jsonc', '.opencode', 'home')
    _require(type(overlays) is dict and set(overlays) == set(names), 'complete role overlay set required')
    root = _safe_path(Path(overlays['opencode.json']).parent)
    _require(root not in candidate.parents and candidate not in root.parents,
             'role overlays must be external to candidate')
    _require(stat.S_IMODE(root.stat().st_mode) == 0o555, 'role overlay root must be immutable')
    expected = {name: root / name for name in names}
    for name in names:
        _require(_safe_path(Path(overlays[name])) == expected[name], 'role overlay path differs')

    policy = (canonical_json(role_policy(role)) + '\n').encode('utf-8')
    for name in ('opencode.json', 'opencode.jsonc'):
        path = expected[name]
        item = path.lstat()
        _require(stat.S_ISREG(item.st_mode) and item.st_nlink == 1 and stat.S_IMODE(item.st_mode) == 0o444
                 and path.read_bytes() == policy, 'root role policy is not immutable/canonical')
    project = expected['.opencode']
    _require(project.is_dir() and not project.is_symlink() and stat.S_IMODE(project.stat().st_mode) == 0o555
             and not any(project.iterdir()), 'project discovery mask is not empty/immutable')

    home = expected['home']
    expected_dirs = {home, home / '.config', home / '.config' / 'opencode', home / '.local',
                     home / '.local' / 'share', home / '.local' / 'share' / 'opencode',
                     home / '.local' / 'state', home / '.local' / 'state' / 'opencode'}
    expected_files = {home / 'opencode.json', home / '.config' / 'opencode' / 'opencode.json'}
    actual_dirs: set[Path] = set()
    actual_files: set[Path] = set()
    for directory, dirs, files in os.walk(home, followlinks=False):
        base = Path(directory)
        actual_dirs.add(base)
        for name in dirs:
            path = base / name
            _require(not path.is_symlink(), 'home overlay symlink forbidden')
            actual_dirs.add(path)
        for name in files:
            path = base / name
            _require(not path.is_symlink(), 'home overlay symlink forbidden')
            actual_files.add(path)
    _require(actual_dirs == expected_dirs and actual_files == expected_files,
             'home overlay contains unexpected discovery surface')
    for path in expected_dirs:
        _require(stat.S_IMODE(path.stat().st_mode) == 0o555, 'home overlay directory is writable')
    for path in expected_files:
        item = path.lstat()
        _require(stat.S_ISREG(item.st_mode) and item.st_nlink == 1 and stat.S_IMODE(item.st_mode) == 0o444
                 and path.read_bytes() == policy, 'home role policy is not immutable/canonical')
    return [(expected[name], {'opencode.json': '/candidate/opencode.json',
                              'opencode.jsonc': '/candidate/opencode.jsonc',
                              '.opencode': '/candidate/.opencode',
                              'home': '/worker-home'}[name]) for name in names]


def _make_launch_view(candidate: Candidate, storage_root: Path, task_id: str, stage_id: str,
                      role: str, destination: Path | None = None) -> Path:
    """Copy into the same bounded filesystem so absent masks never touch input."""
    storage_root = _safe_path(storage_root)
    _require(candidate.root.parent == storage_root, 'candidate must be a direct child of bounded launch storage')
    launch_root = storage_root / '.or-v2-launch-views'
    if not launch_root.exists() and not launch_root.is_symlink():
        launch_root.mkdir(mode=0o700)
    item = launch_root.lstat()
    _require(stat.S_ISDIR(item.st_mode) and item.st_uid == os.geteuid()
             and stat.S_IMODE(item.st_mode) == 0o700, 'launch-view storage is not controller-private')
    if destination is None:
        stage_root = Path(tempfile.mkdtemp(prefix=task_id + '-' + stage_id + '-', dir=launch_root))
        view = stage_root / 'candidate'
    else:
        view = _safe_path(destination)
        _require(view.parent.parent == launch_root and not view.parent.exists(), 'reserved launch path reused')
        view.parent.mkdir(mode=0o700)

    shutil.copytree(candidate.root, view, copy_function=shutil.copy2, symlinks=True)
    # Controller-created empty mount targets are recorded before execution and
    # removed only after reconciliation; they must never dirty a clean handoff.
    for name in ('opencode.json', 'opencode.jsonc', '.opencode'):
        path = view/name
        if not path.exists():
            if name == '.opencode':
                path.mkdir()
            else:
                path.touch()
    inspect_candidate(view)
    source_device = candidate.root.stat().st_dev
    for directory, dirs, files in os.walk(view, followlinks=False):
        for name in dirs + files:
            path = Path(directory) / name
            entry = path.lstat()
            _require(entry.st_dev == source_device and
                     (stat.S_ISDIR(entry.st_mode) or stat.S_ISREG(entry.st_mode)) and
                     (not stat.S_ISREG(entry.st_mode) or entry.st_nlink == 1),
                     'copied launch view contains unsafe filesystem entry')
    for directory, _, files in os.walk(view, topdown=False, followlinks=False):
        base = Path(directory)
        base.chmod(stat.S_IMODE(base.stat().st_mode) | (0o777 if role == 'IMPLEMENTATION' else 0o555))
        for name in files:
            path = base / name
            path.chmod(stat.S_IMODE(path.stat().st_mode) | (0o666 if role == 'IMPLEMENTATION' else 0o444))
    return view


@dataclass(frozen=True)
class StageIdentity:
    container_id: str
    host_boot_identity: str
    owner_nonce: str
    stage_id: str
    lease_epoch: int
    stage_nonce: str
    authority_digest: str
    role: str
    task_id: str

    def validate(self) -> None:
        _require(_CONTAINER.fullmatch(self.container_id) is not None, 'invalid container ID')
        for value in (self.owner_nonce, self.stage_id, self.stage_nonce, self.task_id):
            _require(isinstance(value, str) and _ID.fullmatch(value) is not None, 'invalid stage identity')
        _require(type(self.lease_epoch) is int and self.lease_epoch > 0, 'invalid lease epoch')
        _require(bool(self.host_boot_identity) and len(self.host_boot_identity) <= 256, 'missing boot identity')
        _require(self.role in _ROLES, 'invalid stage role')

    def labels(self) -> dict[str, str]:
        return {'or.v2.stage': self.stage_id, 'or.v2.task': self.task_id, 'or.v2.owner': self.owner_nonce,
                'or.v2.epoch': str(self.lease_epoch), 'or.v2.nonce': self.stage_nonce,
                'or.v2.boot': self.host_boot_identity, 'or.v2.authority': self.authority_digest}


@dataclass(frozen=True, init=False)
class TerminatedStageIdentity:
    identity: StageIdentity
    fixture_only: bool
    workspace_path: str | None

    def __init__(self, identity: StageIdentity, fixture_only: bool, *, workspace_path: str | None = None, _seal: object = None):
        _require(_seal is _SEAL, 'termination requires observed entire-container reconciliation')
        object.__setattr__(self, 'identity', identity)
        object.__setattr__(self, 'fixture_only', fixture_only)
        object.__setattr__(self, 'workspace_path', workspace_path)

    def matches(self, stage: dict[str, Any]) -> bool:
        """A sealed observation may only settle its exact persisted lease."""
        return (isinstance(stage, dict) and all(stage.get(key) == getattr(self.identity, key)
                for key in ('host_boot_identity', 'owner_nonce',
                            'stage_id', 'lease_epoch', 'stage_nonce', 'task_id'))
                and self.identity.container_id == (stage.get('container_id') or '0' * 64))


TerminatedStage = TerminatedStageIdentity


def validate_termination(proof: TerminatedStageIdentity, identity: StageIdentity, *,
                         allow_fixture: bool = False) -> None:
    _require(type(proof) is TerminatedStageIdentity and proof.identity == identity,
             'wrong or unobserved termination proof')
    _require(not proof.fixture_only or allow_fixture, 'fixture observation cannot release production lease')


class DockerCLI:
    """Explicit trusted Docker CLI transport. Never called implicitly on import."""
    def __init__(self, executable: Path, executable_sha256: str, *, endpoint: str | None = None):
        executable = _safe_path(executable)
        _require(executable.is_file() and hashlib.sha256(executable.read_bytes()).hexdigest() == executable_sha256,
                 'Docker executable pin mismatch')
        self.executable = executable
        self.digest = executable_sha256
        # Explicit connection prevents an unrelated default/rootful context from
        # selecting the runtime. Actual daemon/profile verification still follows.
        _require(endpoint is None or type(endpoint) is str and re.fullmatch(r'unix:///run/user/' + str(os.getuid()) +
                 r'/docker\.sock', endpoint) is not None, 'per-user rootless socket required')
        self.endpoint = endpoint

    def __call__(self, arguments: list[str]) -> str:
        _require(hashlib.sha256(self.executable.read_bytes()).hexdigest() == self.digest, 'Docker executable changed')
        try:
            connection = ['--host=' + self.endpoint] if self.endpoint else []
            result = subprocess.run([str(self.executable), *connection, *arguments], env={'PATH': '/usr/bin:/bin'},
                                    capture_output=True, timeout=30, check=True)
        except subprocess.CalledProcessError as exc:
            detail = (exc.stderr or b'')[:300].decode('utf-8', 'replace')
            raise SandboxError('Docker runtime refused ' + (arguments[0] if arguments else '?') + ': ' + detail) from exc
        except (OSError, subprocess.SubprocessError) as exc:
            raise SandboxError('Docker runtime/observation unavailable') from exc
        _require(len(result.stdout) <= 4 << 20, 'Docker observation bound exceeded')
        try:
            return result.stdout.decode('utf-8')
        except UnicodeError as exc:
            raise SandboxError('invalid Docker observation') from exc


def host_boot_identity() -> str:
    if os.uname().sysname == 'Linux':
        value = Path('/proc/sys/kernel/random/boot_id').read_text(encoding='ascii').strip()
        _require(re.fullmatch(r'[0-9a-f-]{36}', value) is not None, 'invalid Linux boot ID')
        return value
    if os.uname().sysname == 'Darwin':
        try:
            result = subprocess.run(['/usr/sbin/sysctl', '-n', 'kern.boottime'], capture_output=True,
                                    timeout=5, check=True)
            return hashlib.sha256(result.stdout).hexdigest()
        except (OSError, subprocess.SubprocessError) as exc:
            raise SandboxError('boot observation unavailable') from exc
    raise SandboxError('unsupported controller platform')


def _bounded_candidate_filesystem(candidate: Path, byte_limit: int) -> tuple[int, int, str]:
    """Independently observe an existing dedicated bounded Linux filesystem.

    No mount/install/sudo: ordinary directories, network storage, macOS host
    binds and unspecified quota adapters are unavailable. The input candidate
    is a direct child; launch copies and their writable outputs share its whole-
    filesystem hard bound, unlike polling du or trusting a caller boolean.
    """
    _require(os.uname().sysname == 'Linux', 'verified bounded VM volume adapter unavailable')
    try:
        result = subprocess.run(['findmnt', '--json', '--target', str(candidate),
                                 '--output', 'TARGET,SOURCE,FSTYPE,FSROOT,OPTIONS'],
                                env={'PATH': '/usr/bin:/bin', 'LANG': 'C'}, capture_output=True,
                                timeout=5, check=True)
        document = load_json_strict(result.stdout.decode('utf-8'))
        mounts = document['filesystems']
        _require(len(mounts) == 1, 'ambiguous candidate filesystem')
        mount = mounts[0]
        mountpoint = Path(mount['target']).resolve()
        _require(candidate.parent == mountpoint and candidate != mountpoint and mount['fsroot'] == '/',
                 'candidate must be a direct child of its dedicated bounded mount')
        _require(mount['fstype'] in ('ext4', 'xfs') and str(mount['source']).startswith('/dev/'),
                 'unsupported quota backing')
        _require(stat.S_ISBLK(os.stat(mount['source']).st_mode), 'quota backing is not local block device')
        _require('rw' in mount['options'].split(','), 'candidate mount is not writable')
        usage = os.statvfs(candidate)
        _require(0 < usage.f_blocks * usage.f_frsize <= byte_limit, 'candidate capacity exceeds task bound')
        return os.stat(candidate).st_dev, usage.f_blocks * usage.f_frsize, str(mountpoint)
    except (OSError, KeyError, TypeError, UnicodeError, subprocess.SubprocessError) as exc:
        raise SandboxError('candidate disk quota observation unavailable') from exc


class ContainerSandbox:
    def __init__(self, docker: Callable[[list[str]], str], *, boot_identity: str,
                 host_platform: str, fixture_only: bool = False):
        _require(host_platform in ('linux', 'macos'), 'unsupported controller platform')
        _require(fixture_only or type(docker) is DockerCLI, 'injected transport is fixture-only')
        _require(not fixture_only or type(docker) is not DockerCLI,
                 'real Docker transport cannot bypass quota using fixture mode')
        if not fixture_only:
            actual_platform = {'Linux': 'linux', 'Darwin': 'macos'}.get(os.uname().sysname)
            _require(actual_platform == host_platform and boot_identity == host_boot_identity(),
                     'controller platform/boot identity unverified')
        self.docker, self.boot_identity = docker, boot_identity
        self.host_platform, self.fixture_only = host_platform, fixture_only
        self._prepared: dict[str, tuple[StageIdentity, dict[str, Any]]] = {}

    def _json(self, arguments: list[str]) -> Any:
        return load_json_strict(self.docker(arguments), 'Docker observation')

    def _runtime(self, image: str) -> str:
        info = self._json(['info', '--format', '{{json .}}'])
        _require(info.get('OSType') == 'linux' and bool(info.get('ID')), 'Linux daemon identity unavailable')
        _require(info.get('CgroupVersion') == '2' and all(info.get(key) is True for key in
                 ('MemoryLimit', 'SwapLimit', 'PidsLimit', 'CpuCfsQuota', 'CpuCfsPeriod')),
                 'kernel cgroup resource enforcement unavailable')
        if self.host_platform == 'linux':
            _require(any('rootless' in item for item in info.get('SecurityOptions', [])), 'rootless Engine required')
        else:
            # Product certification needs an externally pinned VM identity and
            # backing quota observer; Docker Desktop's name alone proves neither.
            _require(self.fixture_only, 'verified Linux VM adapter unavailable')
        images = self._json(['image', 'inspect', image])
        _require(len(images) == 1 and images[0].get('Id') == image and images[0].get('Os') == 'linux',
                 'Linux image digest mismatch')
        architecture = {'aarch64': 'arm64', 'x86_64': 'amd64'}.get(info.get('Architecture'), info.get('Architecture'))
        _require(images[0].get('Architecture') == architecture and architecture in ('arm64', 'amd64'),
                 'daemon/image architecture differs')
        _require(not images[0].get('Config', {}).get('Volumes'), 'image declares unexpected writable volumes')
        _require(images[0].get('Config', {}).get('Env') in (None, [], ['PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin']),
                 'image environment is not frozen minimal environment')
        return info['ID']

    def prepare_stage(self, *, candidate: Candidate, authority: ValidatedReleaseAuthority,
                      adapter: str = 'opencode', **kwargs: Any) -> StageIdentity:
        payload = _authority(authority)
        _require(candidate.authority_digest == payload['git']['authority_digest'], 'original authority differs')
        # M1 is disabled: no operational/product/model command entrypoint.
        raise SandboxError('pinned effective OpenCode adapter unavailable; disabled M1 has no product dispatch')

    def prepare_fixture_stage(self, *, candidate: Candidate, image: str, role: str, limits: Limits,
                              command: tuple[str, ...], owner_nonce: str, stage_id: str,
                              lease_epoch: int, task_id: str, stage_nonce: str,
                              persist: Callable[[StageIdentity], None],
                              overlays: dict[str, Path] | None = None, store: Any = None,
                              stage: dict[str, Any] | None = None, fault: Callable[[str], None] | None = None) -> StageIdentity:
        _require(candidate.authority_digest == 'FIXTURE_ONLY', 'shell adapter is fixture-only')
        _require(role in ('IMPLEMENTATION', 'VERIFIER_CONTROLLER'), 'model observation roles deny shell/tools')
        _require(isinstance(command, tuple) and bool(command) and all(isinstance(x, str) and x and '\x00' not in x for x in command),
                 'explicit bounded fixture command required')
        _require(_IMAGE.fullmatch(image) is not None, 'exact image content digest required')
        limits.validate()
        candidate.verify()
        overlay_mounts = _validated_role_overlays(overlays, role, candidate.root) if overlays is not None else []
        _require(bool(overlay_mounts), 'controller-owned role overlays required')
        nonce = stage_nonce
        provisional = StageIdentity('0' * 64, self.boot_identity, owner_nonce, stage_id,
                                    lease_epoch, nonce, 'FIXTURE_ONLY', role, task_id)
        provisional.validate()
        daemon = self._runtime(image)
        quota = None if self.fixture_only else _bounded_candidate_filesystem(candidate.root, limits.candidate_bytes)
        storage_root = candidate.root.parent if quota is None else Path(quota[2])
        from . import workspace as w
        hit = fault or (lambda point: None)
        _require(self.fixture_only or store is not None, 'real launch requires durable controller workspace binding')
        reserved = None
        if store is not None:
            _require(stage == dict(task_id=task_id, stage_id=stage_id, lease_epoch=lease_epoch,
                     owner_nonce=owner_nonce, host_boot_identity=self.boot_identity, stage_nonce=nonce,
                     container_id=None), 'launch stage differs from durable claim')
            reserved = w.reserve_launch(store, stage, candidate, storage_root, daemon, image, role)
            hit('after_reservation')
        launch_candidate = _make_launch_view(candidate, storage_root, task_id, stage_id, role, reserved)
        if store is not None:
            w.bind_launch(store, stage)
        hit('after_workspace')

        args = ['create', '--pull=never', '--read-only', '--user=10001:10001', '--cap-drop=ALL',
                '--security-opt=no-new-privileges:true', '--network=none', '--ipc=none', '--cgroupns=private',
                '--pids-limit=' + str(limits.pids), '--cpus=' + str(limits.cpu),
                '--memory=' + str(limits.memory_bytes), '--memory-swap=' + str(limits.memory_bytes),
                '--log-driver=none', '--restart=no', '--stop-timeout=1', '--init',
                '--ulimit=nofile=256:256', '--ulimit=fsize=' + str(limits.output_bytes) + ':' + str(limits.output_bytes),
                '--mount=type=bind,src=' + str(launch_candidate) + ',dst=/candidate' +
                (',readonly' if role == 'VERIFIER_CONTROLLER' else ''),
                '--tmpfs=/scratch:rw,nosuid,nodev,noexec,size=' + str(limits.scratch_bytes) + ',mode=1777',
                '--workdir=/candidate', '--env=HOME=/worker-home', '--env=TMPDIR=/scratch',
                '--env=XDG_CONFIG_HOME=/worker-home/.config',
                '--env=XDG_DATA_HOME=/worker-home/.local/share',
                '--env=XDG_STATE_HOME=/worker-home/.local/state',
                '--env=OPENCODE_CONFIG=/worker-home/opencode.json',
                '--env=GIT_CONFIG_NOSYSTEM=1', '--env=GIT_CONFIG_GLOBAL=/dev/null']
        if store is not None:
            args.append('--name=' + store.launch_record(stage)['container_name'])
        for source, destination in overlay_mounts:
            args.append('--mount=type=bind,src=' + str(source) + ',dst=' + destination + ',readonly')
        for key, value in provisional.labels().items():
            args.extend(['--label', key + '=' + value])
        # A trusted image must contain GNU timeout at this exact path. It runs
        # inside the container, so controller death cannot remove wall bounds.
        bounded_command = ('/usr/bin/timeout', '--signal=KILL', '--kill-after=1',
                           str(limits.wall_seconds), *command)
        args.extend(['--entrypoint=' + bounded_command[0], image, *bounded_command[1:]])
        container_id = self.docker(args).strip()
        hit('after_create')
        identity = StageIdentity(container_id, self.boot_identity, owner_nonce, stage_id,
                                 lease_epoch, nonce, 'FIXTURE_ONLY', role, task_id)
        identity.validate()
        mounts = [(str(launch_candidate), '/candidate', role != 'VERIFIER_CONTROLLER')]
        mounts.extend((str(source), destination, False) for source, destination in overlay_mounts)
        expected = dict(image=image, candidate=str(launch_candidate), input_candidate=str(candidate.root),
                        mounts=mounts, limits=limits, daemon=daemon, quota=quota,
                        command=bounded_command, candidate_handle=candidate, store=store, stage=stage)
        # Persist even if effective inspection fails. A crashed controller must
        # retain the instance locator; no local PID is accepted as death proof.
        if store is not None:
            stage = store.bind_container(stage, container_id)
            expected['stage'] = stage
        persist(identity)
        hit('after_persistence')
        self.verify_effective(identity, expected)
        self._prepared[container_id] = (identity, expected)
        return identity

    def _inspect(self, identity: StageIdentity) -> dict[str, Any]:
        identity.validate()
        result = self._json(['inspect', identity.container_id])
        _require(isinstance(result, list) and len(result) == 1, 'instance observation ambiguous')
        instance = result[0]
        _require(instance.get('Id') == identity.container_id, 'wrong live instance')
        _require(instance.get('Config', {}).get('Labels') == identity.labels(), 'stage ownership labels differ')
        return instance

    def verify_effective(self, identity: StageIdentity, expected: dict[str, Any]) -> None:
        instance = self._inspect(identity)
        config, host = instance['Config'], instance['HostConfig']
        limits = expected['limits']
        _require(instance.get('Image') == expected['image'] and config.get('User') == '10001:10001', 'image/user mismatch')
        _require(host.get('ReadonlyRootfs') is True and host.get('Privileged') is False,
                 'writable root or privileged container')
        _require(not host.get('CapAdd') and host.get('CapDrop') == ['ALL'], 'capability policy differs')
        _require(host.get('SecurityOpt') == ['no-new-privileges:true'], 'privilege escalation possible')
        _require(host.get('NetworkMode') == 'none' and host.get('PidMode', '') == ''
                 and host.get('IpcMode') == 'none' and host.get('UTSMode', '') == ''
                 and host.get('UsernsMode', '') == '' and host.get('CgroupnsMode') == 'private', 'namespace policy differs')
        for name in ('Devices', 'DeviceRequests', 'DeviceCgroupRules', 'VolumesFrom', 'Links', 'ExtraHosts'):
            _require(not host.get(name), 'unexpected device/host capability: ' + name)
        _require(host.get('Memory') == limits.memory_bytes and host.get('MemorySwap') == limits.memory_bytes
                 and host.get('NanoCpus') == limits.cpu * 1_000_000_000
                 and host.get('PidsLimit') == limits.pids, 'resource limits differ')
        _require(host.get('RestartPolicy') == {'Name': 'no', 'MaximumRetryCount': 0}
                 and host.get('LogConfig') == {'Type': 'none', 'Config': {}}, 'unbounded persistence/logs')
        _require(config.get('WorkingDir') == expected.get('workdir', '/candidate') and config.get('Entrypoint') == [expected['command'][0]]
                 and (config.get('Cmd') or []) == list(expected['command'][1:]), 'effective command differs')
        environment = config.get('Env', [])
        required = ['HOME=/worker-home', 'TMPDIR=/scratch', 'XDG_CONFIG_HOME=/worker-home/.config',
                    'XDG_DATA_HOME=/worker-home/.local/share', 'XDG_STATE_HOME=/worker-home/.local/state',
                    'OPENCODE_CONFIG=/worker-home/opencode.json',
                    'GIT_CONFIG_NOSYSTEM=1', 'GIT_CONFIG_GLOBAL=/dev/null']
        required = expected.get('environment', required)
        approved = required + ['PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin']
        _require(isinstance(environment, list) and len(environment) == len(set(environment))
                 and set(required) <= set(environment) <= set(approved), 'effective launch environment differs')
        mounts = instance.get('Mounts', [])
        _require(isinstance(mounts, list) and all(type(item) is dict for item in mounts),
                 'malformed effective mount observation')
        observed_mounts = [(item.get('Type'), item.get('Source'), item.get('Destination'), item.get('RW'),
                            item.get('Propagation')) for item in mounts]
        expected_mounts = [('bind', source, destination, writable, 'rprivate')
                           for source, destination, writable in expected['mounts']]
        _require(all(type(row[0]) is str and type(row[1]) is str and type(row[2]) is str
                     and type(row[3]) is bool and type(row[4]) is str for row in observed_mounts)
                 and len(observed_mounts) == len(expected_mounts)
                 and sorted(observed_mounts) == sorted(expected_mounts),
                 'effective candidate/config mount set differs')
        scratch = 'rw,nosuid,nodev,noexec,size=' + str(limits.scratch_bytes) + ',mode=1777'
        _require(host.get('Tmpfs') == {'/scratch': scratch}, 'scratch quota differs')
        ulimits = {item['Name']: (item['Soft'], item['Hard']) for item in host.get('Ulimits', [])}
        _require(ulimits == {'nofile': (256, 256), 'fsize': (limits.output_bytes, limits.output_bytes)}, 'output/FD bounds differ')
        if expected.get('created_only', True):
            _require(instance.get('State', {}).get('Status') == 'created' and not instance['State'].get('Running'),
                     'stage executed before verification')

    def start_stage(self, identity: StageIdentity) -> None:
        _require(identity.container_id in self._prepared, 'stage was not verified/persisted by this controller')
        recorded, expected = self._prepared[identity.container_id]
        _require(identity == recorded and identity.host_boot_identity == self.boot_identity, 'stage identity/boot differs')
        self.verify_effective(identity, expected)
        expected['candidate_handle'].verify()
        if expected.get('store') is not None:
            record = expected['store'].launch_record(expected['stage'])
            from .workspace import restore_launch
            _require(str(restore_launch(expected['store'], expected['stage']).root) == expected['candidate'],
                     'persisted workspace differs before start')

        _require(self._runtime(expected['image']) == expected['daemon'], 'daemon identity changed')
        if not self.fixture_only:
            _require(_bounded_candidate_filesystem(Path(expected['input_candidate']), expected['limits'].candidate_bytes)
                     == expected['quota'], 'candidate quota backing changed')
        self.docker(['start', identity.container_id])

    def wait_stage(self, identity: StageIdentity) -> TerminatedStageIdentity:
        """Controller enforces wall deadline; restart must reconcile, never relaunch."""
        _require(identity.container_id in self._prepared, 'unverified stage')
        limits = self._prepared[identity.container_id][1]['limits']
        deadline = time.monotonic() + limits.wall_seconds
        while time.monotonic() < deadline:
            instance = self._inspect(identity)
            if not instance['State'].get('Running'):
                return self.reconcile_stage(identity)
            time.sleep(min(0.1, max(0, deadline - time.monotonic())))
        return self.reconcile_stage(identity)

    def reconcile_stage(self, identity: StageIdentity) -> TerminatedStageIdentity:
        _require(identity.host_boot_identity == self.boot_identity, 'boot mismatch is not dead-instance proof')
        instance = self._inspect(identity)
        if instance['State'].get('Running') or instance['State'].get('Paused'):
            self.docker(['kill', identity.container_id])
        observed = self._inspect(identity)
        _require(observed['State'].get('Running') is False and not observed['State'].get('Paused')
                 and not observed['State'].get('Restarting') and observed['State'].get('Pid') == 0,
                 'entire container termination unproved')
        ids = self.docker(['ps', '--all', '--no-trunc', '--filter', 'label=or.v2.stage=' + identity.stage_id,
                          '--format', '{{.ID}}']).splitlines()
        _require(identity.container_id in ids and all(_CONTAINER.fullmatch(item) for item in ids), 'stage instance inventory uncertain')
        for container_id in ids:
            matches = self._json(['inspect', container_id])
            _require(len(matches) == 1 and matches[0].get('Id') == container_id
                     and matches[0].get('State', {}).get('Running') is False
                     and matches[0]['State'].get('Pid') == 0
                     and not matches[0]['State'].get('Paused') and not matches[0]['State'].get('Restarting'),
                     'another stage instance may remain live')
        self._prepared.pop(identity.container_id, None)
        return TerminatedStageIdentity(identity, self.fixture_only, _seal=_SEAL)


    def reconcile_launch(self, store, stage) -> TerminatedStageIdentity:
        """Fresh controller uses only durable reservation + trusted exact observations."""
        from .store import RuntimeStore
        _require(type(store) is RuntimeStore, 'trusted durable runtime store required')
        record = store.launch_record(stage)
        _require(stage['host_boot_identity'] == self.boot_identity, 'launch boot mismatch')
        info = self._json(['info', '--format', '{{json .}}'])
        _require(info.get('ID') == record['daemon'] and info.get('OSType') == 'linux'
                 and any('rootless' in item for item in info.get('SecurityOptions', [])), 'recovery daemon identity changed')
        name = record['container_name']
        ids = self.docker(['ps', '--all', '--no-trunc', '--filter', 'name=^/' + name + '$',
                           '--format', '{{.ID}}']).splitlines()
        _require(len(ids) <= 1 and all(_CONTAINER.fullmatch(item) for item in ids), 'exact container reservation ambiguous')
        if ids:
            container = ids[0]
            _require(stage['container_id'] in (None, container), 'container reservation reused')
            original = record['input']
            identity = StageIdentity(container, self.boot_identity, stage['owner_nonce'], stage['stage_id'],
                    stage['lease_epoch'], stage['stage_nonce'], original['authority_digest'], record['role'], stage['task_id'])
            instance = self._inspect(identity)
            mounts = [m for m in instance.get('Mounts', []) if m.get('Destination') == '/candidate']
            _require(len(mounts) == 1 and mounts[0].get('Source') == record['path'], 'forged worker workspace path')
            proof = self.reconcile_stage(identity)
            # Bind a Docker create that survived a controller death before ID persistence.
            if stage['container_id'] is None:
                store.bind_container(stage, container)
            return TerminatedStageIdentity(proof.identity, self.fixture_only, workspace_path=record['path'], _seal=_SEAL)
        # Removal does not erase the workspace pointer. Empty exact ID and stage
        # inventories from the same verified daemon prove absence, never a PID.
        if stage['container_id'] is not None:
            _require(not self.docker(['ps', '--all', '--no-trunc', '--filter', 'id=' + stage['container_id'],
                                     '--format', '{{.ID}}']).splitlines(), 'recorded container renamed or ambiguous')
        _require(not self.docker(['ps', '--all', '--no-trunc', '--filter', 'label=or.v2.stage=' + stage['stage_id'],
                                 '--format', '{{.ID}}']).splitlines(), 'other stage instance prevents absence proof')
        identity = StageIdentity(stage['container_id'] or '0' * 64, self.boot_identity, stage['owner_nonce'],
                stage['stage_id'], stage['lease_epoch'], stage['stage_nonce'], record['input']['authority_digest'],
                record['role'], stage['task_id'])
        return TerminatedStageIdentity(identity, self.fixture_only, workspace_path=record['path'], _seal=_SEAL)

    def remove_launch_container(self, store, stage):
        record = store.launch_record(stage)
        ids = self.docker(['ps', '--all', '--no-trunc', '--filter', 'name=^/' + record['container_name'] + '$',
                           '--format', '{{.ID}}']).splitlines()
        if ids:
            _require(ids == [stage['container_id']], 'cleanup container identity differs')
            self.docker(['rm', stage['container_id']])  # no force: a live container refuses


    def verify_no_launch_users(self, store, stage):
        """Mount inspection verifies a known locator; it never discovers one."""
        record = store.launch_record(stage)
        ids = self.docker(['ps', '--all', '--no-trunc', '--format', '{{.ID}}']).splitlines()
        _require(len(ids) <= 4096 and all(_CONTAINER.fullmatch(item) for item in ids), 'container inventory uncertain')
        for container_id in ids:
            observations = self._json(['inspect', container_id])
            _require(len(observations) == 1 and observations[0].get('Id') == container_id, 'container observation uncertain')
            for mount in observations[0].get('Mounts', []):
                source = mount.get('Source', '')
                _require(isinstance(source,str), 'malformed mount source')
                exact_or_child = source == record['path'] or source.startswith(record['path'].rstrip('/') + '/')
                ancestor = bool(source) and Path(record['path']).is_relative_to(Path(source))
                state = observations[0].get('State', {})
                live = (state.get('Running') is not False or state.get('Paused') or
                        state.get('Restarting') or state.get('Pid') != 0)
                _require(not exact_or_child and not (ancestor and live),
                         'another container retains workspace mount')


    def erase_retired_launch(self, store, stage, fault=None):
        """A mapped-UID cleanup process can unlink worker-owned directories.

        Its exact name is derived from the already fsynced CLEANING receipt.
        It has only the retired view; useful work lives outside its namespace.
        A restart reconciles that name before creating another cleanup process.
        """
        from .workspace import verify_preserved, _parent
        hit = fault or (lambda point: None)
        record = store.launch_record(stage)
        _require(record['phase'] == 'CLEANING' and store.inspect()['tasks'][stage['task_id']]['stage'] is None,
                 'cleanup helper requires durable handoff and settled lease')
        verify_preserved(store, record)
        _require(self._runtime(record['image']) == record['daemon'], 'cleanup daemon/image changed')
        path = _parent(record)
        status = path.stat()
        _require((status.st_dev, status.st_ino) == (record['device'], record['inode']), 'retired workspace reused')
        name = record['container_name'] + '-cleanup'
        cleanup_stage = 'cleanup-' + Path(record['path']).parent.name
        def identity(container):
            return StageIdentity(container,self.boot_identity,stage['owner_nonce'],cleanup_stage,
                    stage['lease_epoch'],stage['stage_nonce'],record['input']['authority_digest'],record['role'],stage['task_id'])
        labels = identity('0'*64).labels()
        command = ('/usr/bin/timeout','--signal=KILL','--kill-after=1','180','/bin/sh','-ec',
                   'find /retired -mindepth 1 -maxdepth 1 -exec /bin/rm -rf -- {} +')
        expected = dict(image=record['image'],mounts=[(str(path),'/retired',True)],
                        limits=Limits(1,268435456,64,48<<30,1<<20,1<<20,180),command=command,
                        workdir='/retired',environment=[],created_only=False)
        ids = self.docker(['ps', '--all', '--no-trunc', '--filter', 'name=^/' + name + '$', '--format', '{{.ID}}']).splitlines()
        _require(len(ids) <= 1 and all(_CONTAINER.fullmatch(item) for item in ids), 'cleanup identity uncertain')
        if ids:
            self.verify_effective(identity(ids[0]),expected)
            item = self._inspect(identity(ids[0]))
            if item.get('State', {}).get('Running') or item.get('State', {}).get('Paused'):
                self.docker(['kill', ids[0]])
            item = self._json(['inspect', ids[0]])[0]
            _require(item.get('State', {}).get('Pid') == 0 and item['State'].get('Running') is False
                     and not item['State'].get('Restarting') and not item['State'].get('Paused'), 'cleanup process death unproved')
            self.docker(['rm', ids[0]])
        self.verify_no_launch_users(store,stage)
        args = ['create', '--name=' + name, '--pull=never', '--read-only', '--user=10001:10001',
                '--cap-drop=ALL', '--security-opt=no-new-privileges:true', '--network=none', '--ipc=none',
                '--pids-limit=64', '--cpus=1', '--cgroupns=private', '--workdir=/retired',
                '--tmpfs=/scratch:rw,nosuid,nodev,noexec,size=1048576,mode=1777',
                '--ulimit=nofile=256:256', '--ulimit=fsize=1048576:1048576', '--memory=268435456', '--memory-swap=268435456',
                '--log-driver=none', '--restart=no', '--mount=type=bind,src=' + str(path) + ',dst=/retired']
        for key, value in labels.items():
            args.extend(['--label', key + '=' + value])
        args.extend(['--entrypoint=' + command[0],record['image'],*command[1:]])
        container = self.docker(args).strip()
        _require(_CONTAINER.fullmatch(container) is not None, 'invalid cleanup container')
        hit('after_cleanup_create')
        expected['created_only']=True
        self.verify_effective(identity(container),expected)
        self.docker(['start', container])
        hit('after_cleanup_start')
        deadline = time.monotonic() + 181
        while True:
            item = self._json(['inspect', container])[0]
            if item.get('State', {}).get('Running') is False and item['State'].get('Pid') == 0:
                break
            _require(time.monotonic() < deadline, 'cleanup deadline exceeded; receipt retained for reconciliation')
            time.sleep(0.1)
        _require(item['State'].get('ExitCode') == 0, 'mapped cleanup failed; useful handoff retained')
        self.docker(['rm', container])
        hit('after_cleanup_remove')
