#!/usr/bin/env python3
"""One bounded Linux developer journey; never packaged acceptance."""
from __future__ import annotations

from contextlib import contextmanager
import fcntl
import json
import os
from pathlib import Path
import platform
import re
import shutil
import signal
import subprocess
import sys
import tempfile
import time
import tomllib

ROOT = Path(__file__).resolve().parents[1]
APP = ROOT / "apps/or_app"
GIB = 1024 ** 3
TOOLCHAIN_TARGETS = re.compile(r"(?ms)^(?P<indent>[ \t]*)targets[ \t]*=[ \t]*\[.*?^(?P=indent)\]")


def linux_rust_target() -> str:
    if sys.platform != "linux":
        raise RuntimeError("BLOCKED: this preflight only supports a Linux host")
    machine = platform.machine().lower()
    targets = {
        "x86_64": "x86_64-unknown-linux-gnu",
        "amd64": "x86_64-unknown-linux-gnu",
        "aarch64": "aarch64-unknown-linux-gnu",
        "arm64": "aarch64-unknown-linux-gnu",
    }
    if machine not in targets:
        raise RuntimeError(f"BLOCKED: unsupported Linux architecture {machine}")
    return targets[machine]


@contextmanager
def linux_toolchain_target(path: Path, target: str):
    """Limit Rustup's temporary manifest to this host and restore exact bytes."""
    if path.is_symlink() or not path.is_file():
        raise RuntimeError("BLOCKED: Rust toolchain manifest is missing or redirected")
    original = path.read_bytes()
    text = original.decode("utf-8")
    configured = tomllib.loads(text).get("toolchain", {}).get("targets")
    if not isinstance(configured, list) or target not in configured:
        raise RuntimeError(f"BLOCKED: Rust manifest does not declare host target {target}")
    matches = list(TOOLCHAIN_TARGETS.finditer(text))
    if len(matches) != 1:
        raise RuntimeError("BLOCKED: Rust manifest must contain one targets array")
    match = matches[0]
    indent = match.group("indent")
    replacement = f'{indent}targets = [\n{indent}    "{target}",\n{indent}]'
    scoped = text[:match.start()] + replacement + text[match.end():]
    try:
        path.write_text(scoped, encoding="utf-8")
        yield
    finally:
        path.write_bytes(original)


def available_memory() -> int:
    for line in Path("/proc/meminfo").read_text().splitlines():
        if line.startswith("MemAvailable:"):
            return int(line.split()[1]) * 1024
    raise RuntimeError("MemAvailable is unavailable")


def writable_cargo_cache(environment: dict[str, str]) -> None:
    """Copy readable public registry/Git entries; never copy credentials or config."""
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

            def ignore_unreadable(directory: str, names: list[str]) -> set[str]:
                ignored = set()
                for name in names:
                    entry = Path(directory) / name
                    required = os.R_OK
                    if entry.is_dir() and not entry.is_symlink():
                        required |= os.X_OK
                    if not os.access(entry, required):
                        ignored.add(name)
                return ignored

            for name in ("registry", "git"):
                if (seed / name).is_dir():
                    shutil.copytree(
                        seed / name, prepared / name, symlinks=True,
                        ignore=ignore_unreadable,
                    )
            (prepared / "AEH_PROVENANCE.json").write_text(json.dumps({
                "seed": str(seed), "copied": ["registry", "git"],
                "excluded": "credentials, config and installed executables",
            }, indent=2) + "\n")
            prepared.rename(target)
    environment["CARGO_HOME"] = str(target)
    environment["CARGO_NET_OFFLINE"] = "true"



def writable_pub_cache(environment: dict[str, str]) -> None:
    """Seed a project-local cache from readable public pub.dev packages only."""
    home = Path(environment.get("HOME", str(Path.home())))
    seed = Path(environment.get("PUB_CACHE", home / ".pub-cache"))
    public_cache = seed / "hosted" / "pub.dev"
    if not seed.is_dir() or (
        not public_cache.is_dir() or os.access(public_cache, os.W_OK)
    ):
        return
    target = APP / ".dart_tool/aeh-pub-cache"
    if target.is_symlink():
        raise RuntimeError("BLOCKED: redirected project pub cache")
    if not target.exists():
        target.parent.mkdir(parents=True, exist_ok=True)
        with tempfile.TemporaryDirectory(prefix="or-pub-seed-", dir=APP / ".dart_tool") as staging:
            prepared = Path(staging) / "pub-cache"
            prepared.mkdir()

            def ignore_unreadable(directory: str, names: list[str]) -> set[str]:
                ignored = set()
                for name in names:
                    entry = Path(directory) / name
                    required = os.R_OK
                    if entry.is_dir() and not entry.is_symlink():
                        required |= os.X_OK
                    if not os.access(entry, required):
                        ignored.add(name)
                return ignored

            for name in ("hosted", "hosted-hashes"):
                public = seed / name / "pub.dev"
                if public.is_dir():
                    destination = prepared / name / "pub.dev"
                    destination.parent.mkdir(parents=True, exist_ok=True)
                    shutil.copytree(
                        public, destination, symlinks=True,
                        ignore=ignore_unreadable,
                    )
            prepared.rename(target)
    environment["PUB_CACHE"] = str(target)

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
                   RUSTUP_TOOLCHAIN="stable", RUSTUP_AUTO_UPDATE="disable",
                   LIBGL_ALWAYS_SOFTWARE="1", GDK_BACKEND="x11",
                   OR_UI_ARTIFACT_DIR=str(artifacts))
        writable_cargo_cache(env)
        writable_pub_cache(env)
        commands = [
            ["flutter", "--no-version-check", "pub", "get", "--offline", "--enforce-lockfile"],
            ["xvfb-run", "-a", "-s", "-screen 0 1440x900x24",
             "flutter", "--no-version-check", "test", "--no-pub", "-d", "linux",
             "integration_test/linux_editor_preflight_test.dart"],
        ]
        started = time.monotonic()
        minimum = available_memory()
        result = None
        reason = None
        host_target = linux_rust_target()
        toolchain_manifest = ROOT / "crates/or_app_bridge/rust-toolchain.toml"
        with tempfile.TemporaryDirectory(prefix="or-ui-config-") as config:
            for name in ("XDG_CONFIG_HOME", "XDG_CACHE_HOME", "XDG_DATA_HOME"):
                env[name] = str(Path(config) / name)
            with linux_toolchain_target(toolchain_manifest, host_target):
                for command in commands:
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
                    if reason or result != 0:
                        break
        evidence = {
            "source": subprocess.check_output(["git", "-c", f"safe.directory={ROOT}", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip(),
            "dirty": subprocess.check_output(["git", "-c", f"safe.directory={ROOT}", "status", "--porcelain"], cwd=ROOT, text=True),
            "rust_target": host_target,
            "commands": commands, "exit_code": result, "reason": reason,
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
