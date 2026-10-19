#!/usr/bin/env python3
"""One bounded Linux developer journey; never packaged acceptance.

The default mode runs the native editor journey under Xvfb. A second, narrow
mode -- ``--exec`` -- runs one caller-supplied command inside the same prepared
environment (project-local Cargo/pub caches, a writable Flutter SDK mirror, the
host-scoped Rust toolchain manifest, and optionally a pinned local FFmpeg
prefix). ``scripts/run_local_lane.sh --packaged`` uses it to build and package
the Linux bundle; that mode never installs, never claims acceptance and never
changes the working tree outside the ignored build caches.
"""
from __future__ import annotations

from contextlib import contextmanager
import argparse
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
# One shared native UI lock so two lanes cannot compile at once. The path is
# overridable so a hermetic check can exercise the lock without touching the
# real one this host's other sessions share.
NATIVE_LOCK_PATH = os.environ.get("OR_NATIVE_LOCK_PATH") or f"/tmp/aeh-native-ui-{os.getuid()}.lock"
FLUTTER_OVERLAY_NAME = "flutter-overlay"
MANIFEST_BACKUP_NAME = "rust-toolchain.toml.original"
NATIVE_BUDGET_SECONDS = 600
PUB_GET_BUDGET_SECONDS = 300
REQUIRED_TOOLS = ("flutter", "xvfb-run", "clang", "clang++", "cmake", "ninja", "ffprobe")
EXEC_USAGE = (
    "usage: run_linux_ui_preflight.py --exec [--local-home DIR] "
    "[--ffmpeg-prefix DIR] [--codec-prefix DIR] -- <command> [args...]"
)
EXEC_VALUE_OPTIONS = ("--local-home", "--ffmpeg-prefix", "--codec-prefix")


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


def artifact_directory() -> Path:
    return APP / "build/ui-preflight"


def git_output(*args: str) -> str:
    return subprocess.check_output(
        ["git", "-c", f"safe.directory={ROOT}", *args], cwd=ROOT, text=True).strip()


def working_tree_status() -> str:
    """Porcelain status: empty means the preflight's rewrite stays attributable."""
    return git_output("status", "--porcelain")


def manifest_backup_path() -> Path:
    return artifact_directory() / MANIFEST_BACKUP_NAME


def recover_interrupted_manifest(manifest: Path, backup: Path) -> str | None:
    """Restore a manifest that a killed run left scoped to this host.

    SIGKILL cannot be caught, so `finally` never runs and the tracked manifest
    stays edited. The next run repairs the tree from the backup before it does
    anything else, so an interrupted run cannot leave an unattributed dirty file
    that blocks every later run.
    """
    if not backup.is_file():
        return None
    original = backup.read_bytes()
    if manifest.is_file() and manifest.read_bytes() == original:
        backup.unlink()
        return None
    manifest.write_bytes(original)
    backup.unlink()
    return f"recovered {manifest} from the interrupted-run backup at {backup}"


@contextmanager
def linux_toolchain_target(path: Path, target: str, backup: Path | None = None):
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
    if backup is not None:
        backup.parent.mkdir(parents=True, exist_ok=True)
        backup.write_bytes(original)
    try:
        path.write_text(scoped, encoding="utf-8")
        yield
    finally:
        path.write_bytes(original)
        if backup is not None and backup.is_file():
            backup.unlink()


def available_memory() -> int:
    for line in Path("/proc/meminfo").read_text().splitlines():
        if line.startswith("MemAvailable:"):
            return int(line.split()[1]) * 1024
    raise RuntimeError("MemAvailable is unavailable")


def cache_is_writable(path: Path) -> bool:
    """Probe for a real write.

    `os.access` reports the shared owner caches as writable because their ACLs
    grant this worker `rw`, but a sandboxed run still denies the write. Only an
    actual create proves the cache can be used in place.
    """
    if not os.access(path, os.W_OK):
        return False
    try:
        with tempfile.NamedTemporaryFile(dir=path, prefix=".or-write-probe-"):
            pass
    except OSError:
        return False
    return True


def writable_cargo_cache(environment: dict[str, str]) -> None:
    """Copy readable public registry/Git entries; never copy credentials or config."""
    seed = Path(environment.get("CARGO_HOME", ""))
    if not seed.is_absolute() or not seed.is_dir() or cache_is_writable(seed):
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
    """Point PUB_CACHE at the project-local staged cache, staging it once.

    The decision is deliberately not a writability probe. Measured 2026-10-18: a
    `flutter pub get` run against the owner's shared cache could *create* inside
    it and delete a fresh directory, yet failed to replace a populated package
    (`Deletion failed, path = '.../hosted/pub.dev/path_provider-2.1.6' (OS Error:
    Operation not permitted)`), leaving 20 package directories without their
    `pubspec.yaml`. Every later offline resolution then failed with "no versions
    of path_provider match 2.1.6", and the compile step saw undefined getters in
    `vector_math` and `clock`. A probe cannot distinguish that state, so the lane
    prefers the project-local cache whenever it exists and stages one (copying
    only readable public pub.dev entries) when it does not. A stale local cache
    fails loudly against `--enforce-lockfile`; delete it to re-stage.
    """
    home = Path(environment.get("HOME", str(Path.home())))
    seed = Path(environment.get("PUB_CACHE", home / ".pub-cache"))
    target = APP / ".dart_tool/aeh-pub-cache"
    if target.is_symlink():
        raise RuntimeError("BLOCKED: redirected project pub cache")
    if (target / "hosted" / "pub.dev").is_dir():
        environment["PUB_CACHE"] = str(target)
        return
    public_cache = seed / "hosted" / "pub.dev"
    if not seed.is_dir() or not public_cache.is_dir():
        return
    if target.exists():
        if target.stat().st_uid != os.getuid():
            raise RuntimeError("BLOCKED: project pub cache belongs to another worker")
        if not (target / "hosted" / "pub.dev").is_dir():
            raise RuntimeError("BLOCKED: project pub cache is incomplete")
    target.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="or-pub-seed-", dir=target.parent) as staging:
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


def flutter_root_from(environment: dict[str, str]) -> Path | None:
    """Resolve the shared Flutter SDK root the way `bin/flutter` resolves itself."""
    candidates = []
    configured = environment.get("FLUTTER_ROOT")
    if configured:
        candidates.append(Path(configured) / "bin" / "flutter")
    found = shutil.which("flutter", path=environment.get("PATH"))
    if found:
        candidates.append(Path(found))
    for candidate in candidates:
        try:
            resolved = candidate.resolve(strict=True)
        except OSError:
            continue
        root = resolved.parent.parent
        if (root / "bin" / "internal" / "shared.sh").is_file():
            return root
    return None


def materialize_flutter_overlay(real_root: Path, target: Path) -> None:
    """Mirror a read-only Flutter SDK so its `bin/cache` becomes writable.

    `bin/flutter` derives `FLUTTER_ROOT` from its own location and rewrites
    `bin/cache/engine.stamp` on every invocation, so a worker that cannot write
    the shared SDK needs a real `bin` tree. Everything else keeps a symlink to
    the shared SDK: only the small tool snapshot and metadata are duplicated,
    not the engine artifacts.
    """
    target.mkdir(parents=True)
    for entry in sorted(real_root.iterdir()):
        if entry.name == "bin":
            continue
        (target / entry.name).symlink_to(entry)
    bin_source = real_root / "bin"
    bin_target = target / "bin"
    bin_target.mkdir()
    for entry in sorted(bin_source.iterdir()):
        if entry.name == "cache":
            continue
        if entry.is_dir():
            shutil.copytree(entry, bin_target / entry.name, symlinks=True)
        else:
            shutil.copy2(entry, bin_target / entry.name)
    cache_source = bin_source / "cache"
    cache_target = bin_target / "cache"
    cache_target.mkdir()
    for entry in sorted(cache_source.iterdir()):
        if entry.is_dir() and not entry.is_symlink():
            (cache_target / entry.name).symlink_to(entry)
        else:
            shutil.copy2(entry, cache_target / entry.name, follow_symlinks=False)


def writable_flutter_root(environment: dict[str, str]) -> None:
    """Point PATH at a writable SDK mirror when the shared cache is read-only."""
    root = flutter_root_from(environment)
    if root is None:
        raise RuntimeError("BLOCKED: the Flutter SDK root could not be located")
    if cache_is_writable(root / "bin" / "cache"):
        return
    target = APP / ".dart_tool" / FLUTTER_OVERLAY_NAME
    if target.is_symlink():
        raise RuntimeError("BLOCKED: redirected project Flutter overlay")
    if target.exists():
        if target.stat().st_uid != os.getuid():
            raise RuntimeError("BLOCKED: project Flutter overlay belongs to another worker")
        if not (target / "bin" / "flutter").is_file():
            raise RuntimeError("BLOCKED: project Flutter overlay is incomplete")
    else:
        target.parent.mkdir(parents=True, exist_ok=True)
        with tempfile.TemporaryDirectory(prefix="or-flutter-overlay-", dir=target.parent) as staging:
            prepared = Path(staging) / "flutter"
            materialize_flutter_overlay(root, prepared)
            prepared.rename(target)
    environment["PATH"] = f"{target / 'bin'}{os.pathsep}{environment.get('PATH', '')}"


def open_native_lock(path: str | None = None) -> int:
    """Open the shared native UI lock as a non-inheritable descriptor.

    The contention this names was measured on 2026-10-18: while a sibling
    project's build (`bash work/phaseA/build-e2e.sh` and `npm run tauri build`
    under ~/Projects/VCE) was running, this preflight failed with
    `[Errno 11] Resource temporarily unavailable` on its own lock, and both of
    those processes held an open descriptor on
    /tmp/aeh-native-ui-<uid>.lock. The lock is per UID and outside any one
    project, so an unrelated build can take it; the mechanism by which it did was
    not established, and is not claimed here.

    Two consequences are handled rather than one guessed: the contention is
    reported as a named BLOCKED reason with the settle command, and the
    descriptor is created non-inheritable (PEP 446 does this by default; the flag
    is explicit so a future rewrite that reaches for a raw fd cannot silently
    remove it) so this process cannot itself be the path by which the lock is
    handed to a child.
    """
    path = path or NATIVE_LOCK_PATH
    return os.open(path, os.O_CREAT | os.O_RDWR | os.O_NOFOLLOW | os.O_CLOEXEC, 0o600)


@contextmanager
def native_lock():
    """The one shared native UI lock, so two lanes cannot compile at once.

    The lock file is per UID, outside the tree, and must be owned by this worker;
    a lock owned by anyone else is a host misconfiguration, not something to take
    over. A second holder fails immediately instead of queueing behind it.
    """
    fd = open_native_lock()
    try:
        if os.fstat(fd).st_uid != os.getuid():
            raise RuntimeError("BLOCKED: lock is not owned by this worker")
        try:
            fcntl.flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError as error:
            raise RuntimeError(
                "BLOCKED: another OR lane or process holds the shared native UI "
                f"lock {NATIVE_LOCK_PATH}; serialise the lanes and retry"
            ) from error
        yield
    finally:
        # Closed on every path, including the refusal: a descriptor left open
        # here is exactly the state this lock must never leave behind.
        os.close(fd)


def redirect_home(environment: dict[str, str], home: Path) -> None:
    """Point HOME at a project-local home for the semi-hermetic build hook.

    Flutter's native-assets hook runs cargo in a narrowed environment: it keeps
    HOME but drops CARGO_HOME, FFMPEG_DIR, PKG_CONFIG_PATH and RUSTFLAGS. A
    project-local HOME whose `.cargo` is the real Cargo home is therefore how the
    hook can read the pinned-prefix config the caller asked for.
    """
    home = home.resolve()
    home.mkdir(parents=True, exist_ok=True)
    shared = Path(environment.get("HOME", str(Path.home())))
    cargo_home = Path(environment.get("CARGO_HOME") or (shared / ".cargo")).resolve()
    rustup_home = Path(environment.get("RUSTUP_HOME") or (shared / ".rustup")).resolve()
    for name, target in ((".cargo", cargo_home), (".rustup", rustup_home)):
        link = home / name
        if link.is_symlink():
            if link.resolve() != target:
                raise RuntimeError(f"BLOCKED: {link} points at {link.resolve()}, not {target}")
        elif link.exists():
            raise RuntimeError(f"BLOCKED: {link} exists and is not a symlink")
        elif target.is_dir():
            link.symlink_to(target)
    environment["HOME"] = str(home)


@contextmanager
def pinned_cargo_config(cargo_home: Path, ffmpeg_prefix: Path, codec_prefix: Path | None):
    """Add a Cargo config that pins one FFmpeg prefix, for the duration of a build.

    Two host facts make this necessary and temporary. The hook cannot be handed
    FFMPEG_DIR, and Debian 13 also ships FFmpeg 7 development libraries, so the
    pinned prefix has to come first in the native search path or the bridge links
    `libavutil.so.59` instead of the packaged `libavutil.so.60`. The file is
    removed afterwards so it cannot silently change later Cargo runs.
    """
    config = cargo_home / "config.toml"
    previous = config.read_bytes() if config.is_file() else None
    pkg_config_path = f"{ffmpeg_prefix}/lib/pkgconfig"
    if codec_prefix is not None:
        pkg_config_path += f":{codec_prefix}/lib/pkgconfig"
    cargo_home.mkdir(parents=True, exist_ok=True)
    config.write_text(
        "[build]\n"
        f'rustflags = ["-L", "native={ffmpeg_prefix}/lib", '
        '"-C", "link-arg=-Wl,-rpath,$ORIGIN"]\n'
        "\n[env]\n"
        f'FFMPEG_DIR = {{ value = "{ffmpeg_prefix}", force = true }}\n'
        f'PKG_CONFIG_PATH = {{ value = "{pkg_config_path}", force = true }}\n',
        encoding="utf-8",
    )
    try:
        yield config
    finally:
        if previous is None:
            config.unlink(missing_ok=True)
        else:
            config.write_bytes(previous)


def parse_exec_arguments(
    argv: list[str],
) -> tuple[list[str], Path | None, Path | None, Path | None]:
    """Split `--exec [options] -- command...` without argparse ambiguity."""
    options: dict[str, Path | None] = {"local-home": None, "ffmpeg-prefix": None,
                                       "codec-prefix": None}
    index = 0
    while index < len(argv) and argv[index] != "--":
        token = argv[index]
        if token not in EXEC_VALUE_OPTIONS:
            raise SystemExit(f"{EXEC_USAGE}\nunknown option: {token}")
        if index + 1 >= len(argv):
            raise SystemExit(f"{EXEC_USAGE}\n{token} needs a value")
        options[token[2:]] = Path(argv[index + 1])
        index += 2
    if index < len(argv):
        index += 1
    command = argv[index:]
    if not command:
        raise SystemExit(f"{EXEC_USAGE}\nno command was given")
    return command, options["local-home"], options["ffmpeg-prefix"], options["codec-prefix"]


def exec_in_environment(argv: list[str]) -> int:
    """Run one command in the prepared developer environment; return its status.

    This deliberately has no packaging, install or acceptance step of its own: it
    prepares the same caches, Flutter mirror and host-scoped manifest the bounded
    journey uses, then hands control to the caller's command.
    """
    command, local_home, ffmpeg_prefix, codec_prefix = parse_exec_arguments(argv)
    manifest = ROOT / "crates/or_app_bridge/rust-toolchain.toml"
    backup = manifest_backup_path()
    recovered = recover_interrupted_manifest(manifest, backup)
    if recovered:
        print(recovered, flush=True)
    if available_memory() < 6 * GIB or shutil.disk_usage(APP).free < 6 * GIB:
        raise RuntimeError("BLOCKED: require 6 GiB available RAM and disk before starting")
    for tool in ("cargo", "flutter"):
        if not shutil.which(tool):
            raise RuntimeError(f"BLOCKED: missing {tool}; do not change host privileges")
    env = dict(os.environ, RUSTUP_TOOLCHAIN="stable", RUSTUP_AUTO_UPDATE="disable")
    writable_cargo_cache(env)
    writable_pub_cache(env)
    writable_flutter_root(env)
    common = dict(cwd=APP, env=env, check=False)
    with native_lock():
        with linux_toolchain_target(manifest, linux_rust_target(), backup=backup):
            if ffmpeg_prefix is None:
                return subprocess.run(command, **common).returncode
            cargo_home = Path(
                env.get("CARGO_HOME")
                or (Path(env.get("HOME", str(Path.home()))) / ".cargo")
            )
            if local_home is not None:
                redirect_home(env, local_home)
            with pinned_cargo_config(cargo_home, ffmpeg_prefix.resolve(),
                                     codec_prefix.resolve() if codec_prefix else None):
                return subprocess.run(command, **common).returncode


def plan_lines() -> tuple[list[str], list[str], list[list[str]]]:
    """Describe what a real run would do, without changing anything.

    Returns the report lines, the reasons the run would be blocked, and the
    commands with their own budgets. Creating the project-local caches and the
    Flutter mirror is a side effect, so the plan reports the decision instead of
    performing it.
    """
    environment = dict(os.environ)
    lines = [f"work tree: {ROOT}", f"HEAD: {git_output('rev-parse', 'HEAD')}"]
    blocked: list[str] = []

    status = working_tree_status()
    lines.append(f"working tree: {'clean' if not status else 'DIRTY'}")
    if status:
        lines.append(status)
        blocked.append("the working tree is not clean; commit or stash before a real run")

    memory = available_memory()
    free = shutil.disk_usage(APP).free
    lines.append(f"MemAvailable: {memory} bytes (need {6 * GIB})")
    lines.append(f"free disk: {free} bytes (need {6 * GIB})")
    if memory < 6 * GIB or free < 6 * GIB:
        blocked.append("insufficient RAM or disk headroom")

    missing = [tool for tool in REQUIRED_TOOLS if not shutil.which(tool)]
    lines.append(f"required tools: {'all present' if not missing else 'MISSING ' + ', '.join(missing)}")
    if missing:
        blocked.append("missing required tools")

    backup = manifest_backup_path()
    lines.append(f"toolchain manifest: {ROOT / 'crates/or_app_bridge/rust-toolchain.toml'}")
    lines.append(f"manifest backup: {backup}{' (pending restore)' if backup.is_file() else ''}")
    if backup.is_file():
        lines.append("  a real run restores this backup before its clean-tree check")
    lines.append(f"rust target: {linux_rust_target()}")

    cargo = Path(environment.get("CARGO_HOME", ""))
    if not cargo.is_absolute() or not cargo.is_dir() or cache_is_writable(cargo):
        lines.append(f"CARGO_HOME: use {cargo or '<unset>'}")
    else:
        lines.append(f"CARGO_HOME: stage readable public entries into {APP / '.dart_tool/aeh-cargo'} and go offline")
    home = Path(environment.get("HOME", str(Path.home())))
    pub_seed = Path(environment.get("PUB_CACHE", home / ".pub-cache"))
    public = pub_seed / "hosted" / "pub.dev"
    if not pub_seed.is_dir() or not public.is_dir() or cache_is_writable(public):
        lines.append(f"PUB_CACHE: use {pub_seed}")
    else:
        lines.append(f"PUB_CACHE: stage public pub.dev entries into {APP / '.dart_tool/aeh-pub-cache'}")
    flutter_root = flutter_root_from(environment)
    if flutter_root is None:
        lines.append("FLUTTER_ROOT: NOT FOUND")
        blocked.append("the Flutter SDK root could not be located")
    elif cache_is_writable(flutter_root / "bin" / "cache"):
        lines.append(f"Flutter SDK: use the shared tree at {flutter_root}")
    else:
        overlay = APP / ".dart_tool" / FLUTTER_OVERLAY_NAME
        lines.append(
            f"Flutter SDK: shared cache is not writable; mirror into {overlay}"
            f"{' (exists)' if overlay.exists() else ' (would be created)'}")
        lines.append(f"  PATH head becomes {overlay / 'bin'} for the run only")

    commands = [
        ["flutter", "--no-version-check", "pub", "get", "--offline", "--enforce-lockfile"],
        ["xvfb-run", "-a", "-s", "-screen 0 1440x900x24",
         "flutter", "--no-version-check", "test", "--no-pub", "-d", "linux",
         "integration_test/linux_editor_preflight_test.dart"],
    ]
    lines.append(f"lock: /tmp/aeh-native-ui-{os.getuid()}.lock (exclusive for the duration)")
    lines.append(f"budgets: pub get {PUB_GET_BUDGET_SECONDS}s, native journey {NATIVE_BUDGET_SECONDS}s, "
                 f"stop below {2 * GIB} bytes available RAM")
    lines.append("artifacts: " + str(artifact_directory()))
    lines.append("commands:")
    for command in commands:
        lines.append("  " + " ".join(command))
    return lines, blocked, commands


def supervise(command: list[str], budget: int,
              environment: dict[str, str]) -> tuple[int, str | None, float, int]:
    """Run one command with its own budget; return code, reason, elapsed, min RAM."""
    started = time.monotonic()
    minimum = available_memory()
    reason = None
    child = subprocess.Popen(command, cwd=APP, env=environment, start_new_session=True)
    try:
        while child.poll() is None:
            minimum = min(minimum, available_memory())
            if minimum < 2 * GIB:
                reason = "BLOCKED: available RAM fell below 2 GiB"
                break
            if time.monotonic() - started > budget:
                reason = f"TIMEOUT: {budget} second budget"
                break
            time.sleep(1)
        code = child.poll() if reason is None else 124
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
    return (code if code is not None else 1), reason, round(time.monotonic() - started, 2), minimum


def main(argv: list[str] | None = None) -> int:
    argv = list(sys.argv[1:] if argv is None else argv)
    if "--exec" in argv:
        # Parsed by hand: the command after `--` must reach the child untouched.
        return exec_in_environment(argv[argv.index("--exec") + 1:])

    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--plan", action="store_true",
                        help="report what a run would do and exit without changing anything")
    args = parser.parse_args(argv)

    manifest = ROOT / "crates/or_app_bridge/rust-toolchain.toml"
    backup = manifest_backup_path()

    if args.plan:
        lines, blocked, _ = plan_lines()
        for line in lines:
            print(line)
        if blocked:
            for entry in blocked:
                print(f"BLOCKED: {entry}")
            return 3
        print("PLAN: ready (nothing was changed)")
        return 0

    # Repair a killed run before asking whether the tree is clean: a leftover
    # scoped manifest is exactly the dirtiness this preflight would refuse.
    recovered = recover_interrupted_manifest(manifest, backup)
    if recovered:
        print(recovered, flush=True)

    if available_memory() < 6 * GIB or shutil.disk_usage(APP).free < 6 * GIB:
        raise RuntimeError("BLOCKED: require 6 GiB available RAM and disk before starting")

    status = working_tree_status()
    if status:
        raise RuntimeError(
            "BLOCKED: the working tree must be clean so a toolchain rewrite stays "
            f"attributable; git status --porcelain reports:\n{status}")

    for tool in REQUIRED_TOOLS:
        if not shutil.which(tool):
            raise RuntimeError(f"BLOCKED: missing {tool}; do not change host privileges")
    lock_path = NATIVE_LOCK_PATH
    fd = open_native_lock(lock_path)
    with os.fdopen(fd, "w") as lock:
        if os.fstat(lock.fileno()).st_uid != os.getuid():
            raise RuntimeError("BLOCKED: lock is not owned by this worker")
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        artifacts = artifact_directory()
        artifacts.mkdir(parents=True, exist_ok=True)
        env = dict(os.environ, CARGO_BUILD_JOBS="1", CMAKE_BUILD_PARALLEL_LEVEL="1",
                   RUSTUP_TOOLCHAIN="stable", RUSTUP_AUTO_UPDATE="disable",
                   LIBGL_ALWAYS_SOFTWARE="1", GDK_BACKEND="x11",
                   OR_UI_ARTIFACT_DIR=str(artifacts))
        writable_cargo_cache(env)
        writable_pub_cache(env)
        writable_flutter_root(env)
        commands = [
            ["flutter", "--no-version-check", "pub", "get", "--offline", "--enforce-lockfile"],
            ["xvfb-run", "-a", "-s", "-screen 0 1440x900x24",
             "flutter", "--no-version-check", "test", "--no-pub", "-d", "linux",
             "integration_test/linux_editor_preflight_test.dart"],
        ]
        budgets = [PUB_GET_BUDGET_SECONDS, NATIVE_BUDGET_SECONDS]
        started = time.monotonic()
        minimum = available_memory()
        result = None
        reason = None
        timings: list[dict[str, object]] = []
        host_target = linux_rust_target()
        with tempfile.TemporaryDirectory(prefix="or-ui-config-") as config:
            for name in ("XDG_CONFIG_HOME", "XDG_CACHE_HOME", "XDG_DATA_HOME"):
                env[name] = str(Path(config) / name)
            with linux_toolchain_target(manifest, host_target, backup=backup):
                for command, budget in zip(commands, budgets):
                    code, failed_reason, elapsed, window_minimum = supervise(
                        command, budget, env)
                    minimum = min(minimum, window_minimum)
                    timings.append({
                        "command": command[0] if command[0] != "xvfb-run" else command[3],
                        "budget_seconds": budget,
                        "elapsed_seconds": elapsed,
                    })
                    result = code
                    reason = failed_reason
                    if reason or code != 0:
                        break
        evidence = {
            "source": git_output("rev-parse", "HEAD"),
            "dirty": working_tree_status() + "\n",
            "rust_target": host_target,
            "commands": commands, "exit_code": result, "reason": reason,
            "elapsed_seconds": round(time.monotonic() - started, 2),
            "stage_timings": timings,
            "minimum_available_ram_bytes": minimum,
            "limits": (f"one shared native lock; one compiler worker; "
                       f"{PUB_GET_BUDGET_SECONDS}s dependency resolution then "
                       f"{NATIVE_BUDGET_SECONDS}s for the native journey; "
                       f"stop below 2 GiB available RAM; refuse a dirty tree"),
            "toolchain_backup": str(backup),
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
