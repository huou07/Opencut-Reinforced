#!/usr/bin/env python3
"""Hermetic checks for the shared native UI lock.

The lock exists so the Linux preflight and the Android lane never compile at the
same time. Measured on this host on 2026-10-18, a sibling project's build held
the lock while it ran, and every OR UI lane then died with
`[Errno 11] Resource temporarily unavailable` instead of saying what was wrong.
The lock is per UID and lives outside any one project, so that can happen again.

These checks pin what is true and falsifiable: the lock excludes a second
*process*, a second holder is refused with a named reason, the descriptor is not
inheritable, and the lock is released on exit. They use `OR_NATIVE_LOCK_PATH` so
they never touch the lock this host's other sessions share, and they need no
device, network or compiler.
"""
from __future__ import annotations

import fcntl
import importlib.util
import os
import subprocess
import sys
import tempfile
import time
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
PREFLIGHT = ROOT / "scripts" / "run_linux_ui_preflight.py"
HOLDER = (
    "import fcntl,os,sys,time\n"
    "fd = os.open(sys.argv[1], os.O_CREAT | os.O_RDWR | os.O_CLOEXEC, 0o600)\n"
    "fcntl.flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)\n"
    "sys.stderr.write('locked\\n')\n"
    "sys.stderr.flush()\n"
    "time.sleep(30)\n"
)


def load_preflight():
    spec = importlib.util.spec_from_file_location("or_ui_preflight_under_test", PREFLIGHT)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


class NativeLockCase(unittest.TestCase):
    def setUp(self) -> None:
        self._tmp = tempfile.TemporaryDirectory()
        self.path = Path(self._tmp.name) / "native.lock"
        os.environ["OR_NATIVE_LOCK_PATH"] = str(self.path)
        self.module = load_preflight()
        self.assertEqual(self.module.NATIVE_LOCK_PATH, str(self.path))

    def tearDown(self) -> None:
        os.environ.pop("OR_NATIVE_LOCK_PATH", None)
        self._tmp.cleanup()

    def try_lock(self) -> bool:
        """Can a fresh descriptor take the lock right now?"""
        fd = os.open(str(self.path), os.O_CREAT | os.O_RDWR | os.O_CLOEXEC, 0o600)
        try:
            fcntl.flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
            return True
        except BlockingIOError:
            return False
        finally:
            os.close(fd)

    def test_the_lock_excludes_a_second_process(self) -> None:
        holder = subprocess.Popen([sys.executable, "-c", HOLDER, str(self.path)],
                                  stderr=subprocess.PIPE, text=True)
        try:
            self.assertEqual(holder.stderr.readline().strip(), "locked")
            self.assertFalse(
                self.try_lock(),
                "another process held the lock and this one still acquired it",
            )
        finally:
            if holder.stderr is not None:
                holder.stderr.close()
            holder.kill()
            holder.wait(timeout=10)
        # Once the holder is gone the lock must be free again, or a crashed lane
        # would block every later one forever.
        for _ in range(50):
            if self.try_lock():
                return
            time.sleep(0.1)
        self.fail("the lock stayed held after its holder exited")

    def test_the_lock_descriptor_is_not_inheritable(self) -> None:
        fd = self.module.open_native_lock(str(self.path))
        try:
            self.assertFalse(os.get_inheritable(fd),
                             "the native lock descriptor could be handed to a child")
            self.assertTrue(fcntl.fcntl(fd, fcntl.F_GETFD) & fcntl.FD_CLOEXEC)
        finally:
            os.close(fd)

    def test_a_second_holder_is_refused_with_a_named_reason(self) -> None:
        with self.module.native_lock():
            with self.assertRaises(RuntimeError) as raised:
                with self.module.native_lock():
                    pass
        message = str(raised.exception)
        self.assertIn("BLOCKED", message)
        self.assertIn("native UI lock", message)

    def test_the_lock_is_released_when_the_context_exits(self) -> None:
        with self.module.native_lock():
            self.assertFalse(self.try_lock())
        self.assertTrue(self.try_lock())

    def test_a_directory_is_never_accepted_as_the_lock(self) -> None:
        directory = Path(self._tmp.name) / "a-directory"
        directory.mkdir()
        with self.assertRaises(OSError):
            fd = self.module.open_native_lock(str(directory))
            os.close(fd)


if __name__ == "__main__":
    sys.exit(0 if unittest.main(exit=False).result.wasSuccessful() else 1)
