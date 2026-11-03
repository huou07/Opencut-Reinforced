#!/usr/bin/env python3
"""Stage the speech runtime and the `or` CLI into any desktop bundle.

Every desktop package carries the same optional runtime: the pinned Whisper
engine with its shared libraries, the provider adapter the bridge spawns, and
the `or` command the assistant uses. macOS used to stage these in a bash block
while Windows and Linux shipped without them, which is why the Windows build
had no captions and no local speech.

This script owns that copy for all three platforms: one source of truth for the
pinned archives, one set of checks, one place to change when a helper is added.
The platform-specific build scripts only decide *where* the runtime directory is
and then call this.
"""

from __future__ import annotations

import argparse
import hashlib
import os
import platform
import shutil
import stat
import sys
import tarfile
import tempfile
import urllib.request
import zipfile
from pathlib import Path

RELEASE = "https://github.com/ggml-org/whisper.cpp/releases/download/b5454"

# Pinned exactly as apps/or_app/lib/updates/resource_center.dart pins them, so a
# packaged build and an in-app repair install the same engine.
ARCHIVES = {
    ("windows", "amd64"): (
        "whisper-bin-x64.zip",
        8928640,
        "6ba69e3482d7826214f90a6a9c84ca07782aec1e1d0c6a7c30c994fd5d816ccb",
    ),
    ("windows", "arm64"): (
        "whisper-bin-win-cpu-arm64.zip",
        4371193,
        "28c37e7b598c3d9bbfef94f3bd67f2ee6f12b7c86f3da5edcf5e4308d615ff6d",
    ),
    ("linux", "amd64"): (
        "whisper-bin-ubuntu-x64.tar.gz",
        10364195,
        "a72becf15d7917f990f6313867a52638b82b7f9ef237fb0c980dac56a135781c",
    ),
    ("linux", "arm64"): (
        "whisper-bin-ubuntu-arm64.tar.gz",
        4608377,
        "6b95ebfc60447df48e70ef00a73bdc3f41679ed2d01ef465827206a2ff325149",
    ),
}


def host_os() -> str:
    if sys.platform.startswith("win"):
        return "windows"
    if sys.platform == "darwin":
        return "macos"
    return "linux"


def host_machine() -> str:
    machine = platform.machine().lower()
    if machine in {"arm64", "aarch64"}:
        return "arm64"
    return "amd64"


def fail(message: str) -> "NoReturn":  # noqa: F821 - typing only
    print(f"error: {message}", file=sys.stderr)
    raise SystemExit(1)


def suffix_for(target_os: str) -> str:
    return ".exe" if target_os == "windows" else ""


def sha256_of(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def download(url: str, destination: Path, size: int, digest: str) -> Path:
    if destination.is_file() and destination.stat().st_size == size:
        if sha256_of(destination) == digest:
            print(f"using verified archive {destination}")
            return destination
    print(f"downloading {url}")
    request = urllib.request.Request(url, headers={"User-Agent": "Opencut-Reinforced"})
    with urllib.request.urlopen(request, timeout=120) as response, destination.open(
        "wb"
    ) as handle:
        received = 0
        while True:
            chunk = response.read(1024 * 1024)
            if not chunk:
                break
            received += len(chunk)
            if received > size:
                fail("the Whisper archive is larger than its published size")
            handle.write(chunk)
    if received != size:
        fail(f"the Whisper archive was incomplete ({received} of {size} bytes)")
    actual = sha256_of(destination)
    if actual != digest:
        fail(f"the Whisper archive checksum does not match (got {actual})")
    return destination


def unpack(archive: Path, destination: Path) -> None:
    if archive.suffix == ".zip":
        with zipfile.ZipFile(archive) as bundle:
            bundle.extractall(destination)
        return
    with tarfile.open(archive, "r:gz") as bundle:
        bundle.extractall(destination)


def find_file(root: Path, name: str) -> Path | None:
    for candidate in root.rglob(name):
        if candidate.is_file():
            return candidate
    return None


def copy_file(source: Path, target: Path, target_os: str) -> None:
    target.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(source, target)
    if target_os != "windows":
        target.chmod(target.stat().st_mode | stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH)


def python_engine_dir(runtime_dir: Path, target_os: str) -> list[Path]:
    """The shared libraries that sit beside whisper-cli, not the models."""
    if target_os == "windows":
        return sorted(runtime_dir.glob("*.dll"))
    if target_os == "linux":
        return sorted(runtime_dir.glob("lib*.so*"))
    return []


def resolve_whisper_cli(args: argparse.Namespace, target_os: str) -> Path | None:
    explicit = args.whisper_cli or os.environ.get("OR_WHISPER_CLI_PATH")
    if explicit:
        path = Path(explicit)
        if not path.is_file():
            fail(f"the Whisper CLI does not exist: {path}")
        return path
    return None


def stage_whisper(
    args: argparse.Namespace, target_os: str, runtime_dir: Path, required: bool
) -> bool:
    inferred = resolve_whisper_cli(args, target_os)
    if inferred is not None:
        copy_file(inferred, runtime_dir / f"whisper-cli{suffix_for(target_os)}", target_os)
        for library in python_engine_dir(inferred.parent, target_os):
            if library.name == inferred.name:
                continue
            copy_file(library, runtime_dir / library.name, target_os)
        print(f"staged whisper-cli from {inferred}")
        return True

    archive_path = args.whisper_archive or os.environ.get("OR_WHISPER_ARCHIVE")
    key = (target_os, args.arch)
    cached = None
    if key in ARCHIVES:
        name, size, digest = ARCHIVES[key]
        cache = Path(args.cache_dir or Path.home() / ".cache" / "opencut-reinforced")
        candidate = cache / name
        if candidate.is_file() and candidate.stat().st_size == size:
            if sha256_of(candidate) == digest:
                cached = candidate
    if archive_path:
        archive = Path(archive_path)
    elif cached is not None:
        archive = cached
    elif args.download_whisper:
        if key not in ARCHIVES:
            fail(
                f"no pinned Whisper archive for {target_os}/{args.arch}; pass "
                "--whisper-cli or --whisper-archive"
            )
        name, size, digest = ARCHIVES[key]
        cache = Path(args.cache_dir or Path.home() / ".cache" / "opencut-reinforced")
        cache.mkdir(parents=True, exist_ok=True)
        archive = download(f"{RELEASE}/{name}", cache / name, size, digest)
    elif required:
        fail(
            "no Whisper engine was supplied. Pass --whisper-cli, --whisper-archive, "
            "or --download-whisper."
        )
    else:
        print("no Whisper engine source available; skipped")
        return False

    with tempfile.TemporaryDirectory(prefix="or-whisper-") as temporary:
        extracted = Path(temporary)
        unpack(archive, extracted)
        binary = find_file(extracted, f"whisper-cli{suffix_for(target_os)}")
        if binary is None:
            fail(f"the Whisper archive had no whisper-cli: {archive}")
        copy_file(binary, runtime_dir / binary.name, target_os)
        for library in python_engine_dir(binary.parent, target_os):
            if library.name == binary.name:
                continue
            copy_file(library, runtime_dir / library.name, target_os)
    print(f"staged whisper-cli from {archive}")


def resolve_provider(
    args: argparse.Namespace, root: Path, target_os: str, required: bool
) -> Path | None:
    explicit = args.provider or os.environ.get("OR_WHISPER_PROVIDER_PATH")
    if explicit:
        path = Path(explicit)
    else:
        path = root / "target" / "release" / f"or-whisper-provider{suffix_for(target_os)}"
    if not path.is_file():
        if required:
            fail(f"the speech adapter was not built: {path}")
        print("the speech adapter is not built; skipped")
        return None
    return path


def resolve_or_cli(args: argparse.Namespace, root: Path, target_os: str) -> Path | None:
    explicit = args.or_cli or os.environ.get("OR_CLI_PATH")
    if explicit:
        path = Path(explicit)
    else:
        path = root / "target" / "release" / f"or{suffix_for(target_os)}"
    return path if path.is_file() else None


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--os",
        choices=("macos", "windows", "linux"),
        default=host_os(),
        help="target platform (default: this host)",
    )
    parser.add_argument("--arch", choices=("amd64", "arm64"), default=host_machine())
    parser.add_argument(
        "--runtime-dir",
        required=True,
        help="folder beside the application binary, e.g. Foo.app/Contents/MacOS",
    )
    parser.add_argument("--repo-root", default=str(Path(__file__).resolve().parent.parent))
    parser.add_argument("--whisper-cli")
    parser.add_argument("--whisper-archive")
    parser.add_argument("--download-whisper", action="store_true")
    parser.add_argument("--cache-dir")
    parser.add_argument("--provider")
    parser.add_argument("--or-cli")
    parser.add_argument(
        "--require",
        default="whisper,provider",
        help="comma list of whisper,provider,or that must be staged",
    )
    args = parser.parse_args()

    target_os = args.os
    root = Path(args.repo_root)
    runtime_dir = Path(args.runtime_dir)
    runtime_dir.mkdir(parents=True, exist_ok=True)
    required = {part.strip() for part in args.require.split(",") if part.strip()}

    stage_whisper(args, target_os, runtime_dir, "whisper" in required)

    provider = resolve_provider(args, root, target_os, "provider" in required)
    if provider is not None:
        copy_file(provider, runtime_dir / provider.name, target_os)
        print(f"staged {provider.name}")

    or_cli = resolve_or_cli(args, root, target_os)
    if or_cli is None:
        if "or" in required:
            fail("the `or` CLI was not built; run cargo build --release -p or_cli")
        print("warning: the `or` CLI was not found and was not staged")
    else:
        copy_file(or_cli, runtime_dir / or_cli.name, target_os)
        print(f"staged {or_cli.name}")

    staged = {
        "whisper": f"whisper-cli{suffix_for(target_os)}",
        "provider": f"or-whisper-provider{suffix_for(target_os)}",
        "or": f"or{suffix_for(target_os)}",
    }
    for component, name in staged.items():
        if component in required and not (runtime_dir / name).is_file():
            fail(f"the staged runtime is missing {name}")
    print(f"runtime staged in {runtime_dir}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
