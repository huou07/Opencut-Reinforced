#!/usr/bin/env python3
"""One bounded Linux developer journey; never packaged acceptance."""
from __future__ import annotations

import fcntl
import json
import os
from pathlib import Path
import shutil
import signal
import subprocess
import tempfile
import time

ROOT = Path(__file__).resolve().parents[1]
APP = ROOT / "apps/or_app"
GIB = 1024 ** 3


def available_memory() -> int:
    for line in Path("/proc/meminfo").read_text().splitlines():
        if line.startswith("MemAvailable:"):
            return int(line.split()[1]) * 1024
    raise RuntimeError("MemAvailable is unavailable")


def writable_cargo_cache(environment: dict[str, str]) -> None:
    """Copy only public registry/Git sources; never copy credentials or config."""
    seed = Path(environment.get("CARGO_HOME", ""))
    if not seed.is_absolute() or not seed.is_dir() or os.access(seed, os.W_OK):
        return
    target = APP / ".dart_tool/aeh-cargo"
    if target.is_symlink():
        raise RuntimeError("BLOCKED: redirected project Cargo cache")
    if not target.exists():
        target.parent.mkdir(parents=True, exist_ok=True)
        with tempfile.TemporaryDirectory(prefix="or-cargo-seed-", dir=APP / ".dart_tool") as staging:
            prepared = Path(staging) / "cargo"
            prepared.mkdir()
            for name in ("registry", "git"):
                if (seed / name).is_dir():
                    shutil.copytree(seed / name, prepared / name, symlinks=True)
            (prepared / "AEH_PROVENANCE.json").write_text(json.dumps({
                "seed": str(seed), "copied": ["registry", "git"],
                "excluded": "credentials, config and installed executables",
            }, indent=2) + "\n")
            prepared.rename(target)
    environment["CARGO_HOME"] = str(target)
    environment["CARGO_NET_OFFLINE"] = "true"


def main() -> int:
    if available_memory() < 6 * GIB or shutil.disk_usage(APP).free < 6 * GIB:
        raise RuntimeError("BLOCKED: require 6 GiB available RAM and disk before starting")
    for tool in ("flutter", "xvfb-run", "clang", "clang++", "cmake", "ninja", "ffprobe"):
        if not shutil.which(tool):
            raise RuntimeError(f"BLOCKED: missing {tool}; do not change host privileges")
    lock_path = f"/tmp/aeh-native-ui-{os.getuid()}.lock"
    fd = os.open(lock_path, os.O_CREAT | os.O_RDWR | os.O_NOFOLLOW, 0o600)
    with os.fdopen(fd, "w") as lock:
        if os.fstat(lock.fileno()).st_uid != os.getuid():
            raise RuntimeError("BLOCKED: lock is not owned by this worker")
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        artifacts = APP / "build/ui-preflight"
        artifacts.mkdir(parents=True, exist_ok=True)
        env = dict(os.environ, CARGO_BUILD_JOBS="1", CMAKE_BUILD_PARALLEL_LEVEL="1",
                   LIBGL_ALWAYS_SOFTWARE="1", GDK_BACKEND="x11",
                   OR_UI_ARTIFACT_DIR=str(artifacts))
        writable_cargo_cache(env)
        command = ["xvfb-run", "-a", "-s", "-screen 0 1440x900x24",
                   "flutter", "test", "--no-pub", "-d", "linux",
                   "integration_test/linux_editor_preflight_test.dart"]
        started = time.monotonic()
        minimum = available_memory()
        result = None
        reason = None
        with tempfile.TemporaryDirectory(prefix="or-ui-config-") as config:
            for name in ("XDG_CONFIG_HOME", "XDG_CACHE_HOME", "XDG_DATA_HOME"):
                env[name] = str(Path(config) / name)
            child = subprocess.Popen(command, cwd=APP, env=env, start_new_session=True)
            try:
                while child.poll() is None:
                    minimum = min(minimum, available_memory())
                    if minimum < 2 * GIB:
                        reason = "BLOCKED: available RAM fell below 2 GiB"
                        break
                    if time.monotonic() - started > 600:
                        reason = "TIMEOUT: 600 second native budget"
                        break
                    time.sleep(1)
                result = child.poll() if reason is None else 124
            finally:
                # Stop the complete process group, including display/server descendants.
                try:
                    os.killpg(child.pid, signal.SIGTERM)
                except ProcessLookupError:
                    pass
                try:
                    child.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    pass
                try:
                    os.killpg(child.pid, signal.SIGKILL)
                except ProcessLookupError:
                    pass
        evidence = {
            "source": subprocess.check_output(["git", "-c", f"safe.directory={ROOT}", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip(),
            "dirty": subprocess.check_output(["git", "-c", f"safe.directory={ROOT}", "status", "--porcelain"], cwd=ROOT, text=True),
            "command": command, "exit_code": result, "reason": reason,
            "elapsed_seconds": round(time.monotonic() - started, 2),
            "minimum_available_ram_bytes": minimum,
            "limits": "one shared native lock; one compiler worker; 600s; stop below 2 GiB available RAM",
            "limitations": "Linux debug app; fixture picker; system media libraries; no installed package, OS picker or cross-platform acceptance",
        }
        (artifacts / "run.json").write_text(json.dumps(evidence, indent=2) + "\n")
        if reason:
            print(reason, flush=True)
        return result if result is not None else 1


if __name__ == "__main__":
    def terminate(_signum, _frame):
        raise KeyboardInterrupt
    signal.signal(signal.SIGTERM, terminate)
    try:
        raise SystemExit(main())
    except (RuntimeError, BlockingIOError) as error:
        print(error, flush=True)
        raise SystemExit(3)
