#!/usr/bin/env python3
"""Exercise install, replacement, launch, and removal of desktop ZIP packages."""

from __future__ import annotations

import os
import shutil
import subprocess
import sys
import tempfile
import zipfile
from pathlib import Path


def create_package(source: Path, work: Path, *, macos: bool) -> Path:
    package_dir = work / "opencut-reinforced-desktop"
    package_dir.mkdir()
    readme = Path(__file__).resolve().parents[1] / "docs/DEVELOPER_PREVIEW_INSTALL.md"
    shutil.copyfile(readme, package_dir / "README-FIRST.md")

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
    if len(sys.argv) != 2:
        raise SystemExit("usage: test_desktop_archive_install.py <built-app-directory>")

    source = Path(sys.argv[1]).resolve()
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
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
