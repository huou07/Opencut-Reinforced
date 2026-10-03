"""Repository-scoped shared runtime memory.

Lives under <git-common-dir>/opencut-automation/ so it is shared across
worktrees but never pollutes product commits. Atomic JSON writes, explicit
schema version, resumable after process death. Facts only: no
chain-of-thought, no secrets, no giant raw logs.
"""
from __future__ import annotations

import contextlib
import fcntl
import json
import os
import subprocess
import tempfile
import time
from pathlib import Path

SCHEMA_VERSION = 1
STATE_DIR_NAME = "opencut-automation"


def git_common_dir(repo: str | Path) -> Path:
    out = subprocess.run(
        ["git", "rev-parse", "--git-common-dir"],
        cwd=str(repo),
        capture_output=True,
        text=True,
        check=True,
        timeout=30,
    ).stdout.strip()
    common = Path(out)
    if not common.is_absolute():
        common = Path(repo) / common
    return common.resolve()


def state_dir(repo: str | Path) -> Path:
    root = git_common_dir(repo) / STATE_DIR_NAME
    for sub in ("", "tasks", "runs", "escalations"):
        (root / sub if sub else root).mkdir(parents=True, exist_ok=True)
    return root


def _atomic_write_json(path: Path, value: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    fd, tmp = tempfile.mkstemp(dir=str(path.parent), prefix=path.name + ".", suffix=".tmp")
    try:
        with os.fdopen(fd, "w", encoding="utf-8") as handle:
            json.dump(value, handle, indent=2, sort_keys=True)
            handle.write("\n")
        os.replace(tmp, path)
    except BaseException:
        with contextlib.suppress(OSError):
            os.unlink(tmp)
        raise


def read_json(path: Path, default: dict) -> dict:
    try:
        with open(path, encoding="utf-8") as handle:
            value = json.load(handle)
        return value if isinstance(value, dict) else dict(default)
    except (OSError, ValueError):
        return dict(default)


class LockedState:
    """Exclusive repository-level lock plus atomic state access.

    One writer at a time across Desktop/CLI processes. Blocking with timeout.
    """

    def __init__(self, repo: str | Path, timeout_s: float = 60.0) -> None:
        self.root = state_dir(repo)
        self.lock_path = self.root / "lock"
        self.timeout_s = timeout_s
        self._handle = None

    def acquire(self) -> None:
        self.lock_path.touch(exist_ok=True)
        self._handle = open(self.lock_path, "w", encoding="utf-8")
        deadline = time.monotonic() + self.timeout_s
        while True:
            try:
                fcntl.flock(self._handle.fileno(), fcntl.LOCK_EX | fcntl.LOCK_NB)
                return
            except BlockingIOError:
                if time.monotonic() >= deadline:
                    raise TimeoutError(f"orchestrator lock busy: {self.lock_path}")
                time.sleep(0.1)

    def release(self) -> None:
        if self._handle is not None:
            with contextlib.suppress(OSError):
                fcntl.flock(self._handle.fileno(), fcntl.LOCK_UN)
                self._handle.close()
            self._handle = None

    def __enter__(self) -> "LockedState":
        self.acquire()
        return self

    def __exit__(self, *exc: object) -> None:
        self.release()

    def current(self) -> dict:
        return read_json(self.root / "current.json", {"schema_version": SCHEMA_VERSION})

    def write_current(self, value: dict) -> None:
        value = dict(value)
        value["schema_version"] = SCHEMA_VERSION
        _atomic_write_json(self.root / "current.json", value)

    def task_path(self, task_id: str) -> Path:
        return self.root / "tasks" / f"{task_id}.json"

    def run_path(self, task_id: str) -> Path:
        return self.root / "runs" / f"{task_id}.json"

    def read_task(self, task_id: str) -> dict:
        return read_json(self.task_path(task_id), {})

    def write_task(self, task_id: str, value: dict) -> None:
        value = dict(value)
        value["schema_version"] = SCHEMA_VERSION
        _atomic_write_json(self.task_path(task_id), value)

    def read_run(self, task_id: str) -> dict:
        return read_json(self.run_path(task_id), {})

    def write_run(self, task_id: str, value: dict) -> None:
        value = dict(value)
        value["schema_version"] = SCHEMA_VERSION
        _atomic_write_json(self.run_path(task_id), value)

    def stats(self) -> dict:
        return read_json(self.root / "model-stats.json", {"schema_version": SCHEMA_VERSION, "entries": []})

    def record_stat(self, entry: dict) -> None:
        stats = self.stats()
        entries = stats.get("entries", [])
        entries.append(entry)
        stats["entries"] = entries[-500:]
        _atomic_write_json(self.root / "model-stats.json", stats)
