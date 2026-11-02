#!/usr/bin/env python3
"""Exercise install, replacement, launch, and removal of desktop ZIP packages.

The archive this script builds is exactly the archive a release ships. Pass
``--archive <path>`` to keep a copy of the verified package; the path is then
printed with its SHA-256 so a release can publish the tested bytes.
"""

from __future__ import annotations

import hashlib
import os
import shutil
import subprocess
import sys
import tempfile
import zipfile
from pathlib import Path


README_TEMPLATE = """# Opencut Reinforced {version}

This folder is a self-contained Opencut Reinforced desktop build. It carries its
own FFmpeg runtime, Rust bridge and viewer plugin, so it needs no Visual Studio,
MSYS2, Flutter or system FFmpeg installation.

## Start

1. Keep the folder where it is; the app loads its libraries beside itself.
2. Run `{executable}`.
3. The build is unsigned. If the operating system cannot verify the publisher,
   do not disable its security protections; report the message instead:
   https://github.com/huou07/Opencut-Reinforced/issues

## Projects and data

Projects are `.orproj` files you choose where to keep. The app also stores its
settings, recent-project list, caches and recovery data in the platform
application-support directory, outside this folder, so replacing the folder with
a newer build keeps your work.

## Upgrade and removal

Replace this folder with the newer build to upgrade. Delete the folder to
remove the application; project files and platform data stay where they are.

## What is inside

- `or_app{extension}` — the editor.
- `ffmpeg`/`ffprobe` (`.exe` on Windows) — the packaged media helpers.
- The Rust bridge, the viewer texture plugin and the shared-library runtime.

## Limits

The local speech runtime (Whisper) and the `or` CLI are not part of this
package. Video and audio codecs come from the bundled software FFmpeg build;
no hardware encoder is advertised.
"""


def create_package(source: Path, work: Path, *, macos: bool) -> Path:
    package_dir = work / "opencut-reinforced-desktop"
    package_dir.mkdir()
    repository = Path(__file__).resolve().parents[1]
    pubspec = (repository / "apps/or_app/pubspec.yaml").read_text(encoding="utf-8")
    version = pubspec.split("version:", 1)[1].split("+", 1)[0].strip()
    (package_dir / "README-FIRST.md").write_text(
        README_TEMPLATE.format(
            version=version,
            executable="OpenCut Reinforced.app" if macos else "or_app.exe",
            extension="" if macos else ".exe",
        ),
        encoding="utf-8",
    )

    if macos:
        app_name = "OpenCut Reinforced.app"
        subprocess.run(["ditto", str(source), str(package_dir / app_name)], check=True)
        archive = work / "opencut-reinforced-macos.zip"
        subprocess.run(
            [
                "ditto",
                "-c",
                "-k",
                "--sequesterRsrc",
                "--keepParent",
                str(package_dir),
                str(archive),
            ],
            check=True,
        )
    else:
        app_name = "OpenCut Reinforced"
        shutil.copytree(source, package_dir / app_name)
        archive = work / "opencut-reinforced-windows.zip"
        shutil.make_archive(
            str(archive.with_suffix("")),
            "zip",
            root_dir=package_dir.parent,
            base_dir=package_dir.name,
        )

    if not archive.is_file() or archive.stat().st_size == 0:
        raise SystemExit(f"Desktop package archive is empty: {archive}")
    return archive


def extract_package(archive: Path, destination: Path, *, macos: bool) -> Path:
    destination.mkdir(parents=True)
    if macos:
        subprocess.run(["ditto", "-x", "-k", str(archive), str(destination)], check=True)
    else:
        with zipfile.ZipFile(archive) as package:
            package.extractall(destination)

    package_dir = destination / "opencut-reinforced-desktop"
    app = package_dir / ("OpenCut Reinforced.app" if macos else "OpenCut Reinforced")
    if not app.is_dir():
        raise SystemExit(f"Extracted desktop app directory is missing: {app}")
    executable = (
        app / "Contents/MacOS/or_app"
        if macos
        else app / "or_app.exe"
    )
    if not executable.is_file():
        raise SystemExit(f"Extracted desktop app executable is missing: {executable}")
    return app


def launch_from_install(executable: Path, install_dir: Path, *, windows: bool) -> None:
    env = os.environ.copy()
    for name in (
        "OR_FFMPEG_PATH",
        "OR_FFPROBE_PATH",
        "LD_LIBRARY_PATH",
        "DYLD_LIBRARY_PATH",
        "DYLD_FALLBACK_LIBRARY_PATH",
        "DYLD_FRAMEWORK_PATH",
        "DYLD_ROOT_PATH",
        "DYLD_INSERT_LIBRARIES",
        "FFMPEG_DIR",
        "FFMPEG_INSTALL_PREFIX",
        "PKG_CONFIG_PATH",
        "FRB_DART_LOAD_EXTERNAL_LIBRARY_NATIVE_LIB_DIR",
    ):
        env.pop(name, None)
    if windows:
        system_root = Path(env["SYSTEMROOT"])
        env["PATH"] = os.pathsep.join(
            (str(executable.parent), str(system_root / "System32"), str(system_root))
        )

    log_path = install_dir.parent / "install-launch.log"
    with log_path.open("wb") as log:
        process = subprocess.Popen(
            [str(executable)],
            cwd=install_dir,
            env=env,
            stdout=log,
            stderr=subprocess.STDOUT,
        )
        try:
            process.wait(timeout=12)
        except subprocess.TimeoutExpired:
            process.terminate()
            try:
                process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait(timeout=5)
        else:
            log.flush()
            detail = log_path.read_text(encoding="utf-8", errors="replace")
            raise SystemExit(
                f"Installed app exited before its launch smoke-test timeout "
                f"(status {process.returncode}):\\n{detail[-3000:]}"
            )
    log_path.unlink(missing_ok=True)


def main() -> int:
    arguments = sys.argv[1:]
    keep_archive: Path | None = None
    if "--archive" in arguments:
        index = arguments.index("--archive")
        if index + 1 >= len(arguments):
            raise SystemExit("--archive needs a destination path")
        keep_archive = Path(arguments[index + 1]).resolve()
        del arguments[index : index + 2]
    if len(arguments) != 1:
        raise SystemExit(
            "usage: test_desktop_archive_install.py <built-app-directory> "
            "[--archive <destination.zip>]"
        )

    source = Path(arguments[0]).resolve()
    macos = sys.platform == "darwin"
    windows = sys.platform == "win32"
    if not macos and not windows:
        raise SystemExit(f"Unsupported archive install host: {sys.platform}")
    if not source.is_dir():
        raise SystemExit(f"Built app directory does not exist: {source}")

    home = Path.home()
    if macos:
        install_parent = home / "Applications" / f"OR-install-check-{os.getpid()}"
        app_name = "OpenCut Reinforced.app"
        executable_relative = Path("Contents/MacOS/or_app")
    else:
        local_app_data = Path(os.environ["LOCALAPPDATA"])
        install_parent = local_app_data / "Programs" / f"OR-install-check-{os.getpid()}"
        app_name = "OpenCut Reinforced"
        executable_relative = Path("or_app.exe")

    documents = home / "Documents"
    documents.mkdir(parents=True, exist_ok=True)
    user_data = Path(tempfile.mkdtemp(prefix="or-desktop-install-check-", dir=documents))
    project = user_data / "user-data.txt"
    project.write_text("user data survives app replacement and removal\\n", encoding="utf-8")

    try:
        with tempfile.TemporaryDirectory(prefix="or-desktop-package-") as temporary:
            work = Path(temporary)
            archive = create_package(source, work, macos=macos)
            if keep_archive is not None:
                keep_archive.parent.mkdir(parents=True, exist_ok=True)
                shutil.copyfile(archive, keep_archive)
            initial_extract = work / "initial-extract"
            staged_app = extract_package(archive, initial_extract, macos=macos)
            install_parent.mkdir(parents=True, exist_ok=False)
            installed_app = install_parent / app_name
            shutil.move(str(staged_app), str(installed_app))
            executable = installed_app / executable_relative
            launch_from_install(executable, installed_app, windows=windows)

            update_extract = work / "update-extract"
            replacement = extract_package(archive, update_extract, macos=macos)
            shutil.rmtree(installed_app)
            shutil.move(str(replacement), str(installed_app))
            if project.read_text(encoding="utf-8") != "user data survives app replacement and removal\\n":
                raise SystemExit("User project changed during desktop app replacement.")
            launch_from_install(
                installed_app / executable_relative, installed_app, windows=windows
            )

            shutil.rmtree(installed_app)
            if installed_app.exists():
                raise SystemExit(f"Desktop app remained after uninstall: {installed_app}")
            if not project.is_file():
                raise SystemExit("Uninstall removed user project data.")
    finally:
        shutil.rmtree(install_parent, ignore_errors=True)
        shutil.rmtree(user_data, ignore_errors=True)

    print(
        "Desktop ZIP install, launch from user install location, replacement, "
        "uninstall, and project-data preservation passed."
    )
    if keep_archive is not None:
        digest = hashlib.sha256(keep_archive.read_bytes()).hexdigest()
        print(f"ARCHIVE={keep_archive}")
        print(f"SHA256={digest}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
