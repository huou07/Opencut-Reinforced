"""M4 trusted pre-push guard: exact advertised-base non-force promotion.

The hook is installed by the controller into a controller-owned hooks
directory with the expected base, candidate, and remote identity baked in at
install time. It verifies the single pushed ref update and refuses anything
else. Force semantics need no hook detection: promotion never emits a force
flag, the hook pins all three SHAs, and the receiving end enforces
non-fast-forward.
"""
from __future__ import annotations

import os
from pathlib import Path
import re
import stat
import subprocess
from typing import Any

from . import contracts as c

HOOK_NAME = 'pre-push'
DESTINATION_REF = 'refs/heads/main'

HOOK_SOURCE = """#!/usr/bin/env python3
import sys
EXPECTED_BASE = %r
EXPECTED_CANDIDATE = %r
EXPECTED_URL = %r

def refuse(message):
    sys.stderr.write('or-v2 push guard refuses: ' + message + '\\n')
    return 1

def main():
    if len(sys.argv) != 3:
        return refuse('hook argv differs')
    if sys.argv[2] != EXPECTED_URL:
        return refuse('remote identity differs')
    try:
        data = sys.stdin.read(4 << 20)
    except OSError:
        return refuse('unreadable push advertisement')
    lines = [line for line in data.splitlines() if line.strip()]
    if len(lines) != 1:
        return refuse('single exact ref update required')
    parts = lines[0].split()
    if len(parts) != 4:
        return refuse('malformed push advertisement')
    local_ref, local_sha, remote_ref, remote_sha = parts
    if remote_ref != 'refs/heads/main':
        return refuse('destination is not main')
    if local_sha != EXPECTED_CANDIDATE:
        return refuse('pushed source is not the authorized candidate')
    if remote_sha != EXPECTED_BASE:
        return refuse('advertised remote is not the authorized base')
    return 0

if __name__ == '__main__':
    sys.exit(main())
"""


class PushGuardError(c.ContractError):
    pass


def _refuse(condition: bool, message: str) -> None:
    if not condition:
        raise PushGuardError(message)


def _sha(value: Any, label: str) -> str:
    _refuse(type(value) is str and re.fullmatch(r'[0-9a-f]{40}', value) is not None, 'invalid ' + label)
    return value


def render_hook(*, expected_base: str, expected_candidate: str, expected_remote_url: str) -> str:
    """Render the hook with baked-in expectations; no environment smuggling."""
    _sha(expected_base, 'expected base')
    _sha(expected_candidate, 'expected candidate')
    _refuse(expected_base != expected_candidate, 'base and candidate must differ')
    _refuse(type(expected_remote_url) is str and expected_remote_url and len(expected_remote_url) <= 4096
            and '\x00' not in expected_remote_url and '\n' not in expected_remote_url, 'invalid remote identity')
    return HOOK_SOURCE % (expected_base, expected_candidate, expected_remote_url)


def install_push_guard(repo: Path, *, expected_base: str, expected_candidate: str,
                       expected_remote_url: str, hooks_dir: Path) -> Path:
    """Install the guard into a controller-owned hooks directory and pin it on the repo."""
    repo = Path(repo)
    _refuse(repo.is_dir() and (repo / '.git').is_dir() and not (repo / '.git').is_symlink(), 'integration repository required')
    hooks = Path(hooks_dir)
    if hooks.exists() or hooks.is_symlink():
        item = hooks.stat()
        _refuse(hooks.is_dir() and not hooks.is_symlink() and item.st_uid == os.geteuid()
                and (item.st_mode & 0o777) == 0o700, 'hooks directory is not controller-private')
    else:
        hooks.mkdir(mode=0o700, parents=True)
    hook = hooks / HOOK_NAME
    content = render_hook(expected_base=expected_base, expected_candidate=expected_candidate,
                          expected_remote_url=expected_remote_url).encode('utf-8')
    fd = os.open(hook, os.O_WRONLY | os.O_CREAT | os.O_TRUNC, 0o700)
    try:
        os.write(fd, content)
        os.fsync(fd)
    finally:
        os.close(fd)
    hook.chmod(0o700)
    try:
        result = subprocess.run(['git', '-C', str(repo), 'config', 'core.hooksPath', str(hooks)],
                                env={'PATH': os.environ.get('PATH', ''), 'LANG': 'C'},
                                capture_output=True, timeout=30, check=False)
    except (OSError, subprocess.SubprocessError) as exc:
        raise PushGuardError('cannot pin hooks path: ' + str(exc)) from exc
    _refuse(result.returncode == 0, 'cannot pin hooks path')
    return hook
