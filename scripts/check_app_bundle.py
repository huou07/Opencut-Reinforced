#!/usr/bin/env python3
"""Refuse to package a Flutter app bundle built from a test entrypoint.

`flutter test integration_test/<case>.dart -d <device>` builds the *test* app
into the platform bundle directory. A packaging step that copies that directory
afterwards ships a bundle whose Dart entrypoint is the integration-test listener
instead of `lib/main.dart`: the application window is created but never draws a
frame, so the user sees a black screen. That is exactly how the 2026-10-19
Developer Preview desktop packages were produced.

This check is the regression guard for the packaging order. It inspects the
Dart kernel every debug bundle carries and rejects the test harness markers.
Run it on the staged copy an archive is built from, never on the live build
directory (which the test run is allowed to overwrite).

Usage:
    check_app_bundle.py --platform linux  <bundle-directory>
    check_app_bundle.py --platform windows <bundle-directory>
    check_app_bundle.py --platform macos  <or_app.app>
"""
from __future__ import annotations

import argparse
import sys
from pathlib import Path

# The test listener that `flutter test -d <device>` injects, and the case files
# that only the integration-test build reaches. A product build reaches neither.
FORBIDDEN_MARKERS = (
    b"flutter_test_listener",
    b"package:integration_test",
    b"integration_test/core_bridge_test.dart",
    b"or_ui_scenario_test.dart",
)

# Relative launcher paths inside each platform bundle.
LAUNCHERS = {
    "linux": "or_app",
    "windows": "or_app.exe",
    "macos": "Contents/MacOS/or_app",
}


class BundleError(Exception):
    """The bundle cannot be distributed as the application."""


def kernel_files(bundle: Path) -> list[Path]:
    """Every Dart kernel blob in a debug Flutter bundle.

    Debug bundles carry `data/flutter_assets/kernel_blob.bin` (Linux, Windows)
    or `Contents/Frameworks/App.framework/Resources/flutter_assets/kernel_blob.bin`
    (macOS). Searching instead of hard-coding the layout keeps the guard honest
    if the embedder moves the file.
    """
    return sorted(path for path in bundle.rglob("kernel_blob.bin") if path.is_file())


def find_test_entrypoint(bundle: Path) -> list[str]:
    """Return one diagnostic per test-harness marker found in a kernel."""
    problems: list[str] = []
    for kernel in kernel_files(bundle):
        contents = kernel.read_bytes()
        for marker in FORBIDDEN_MARKERS:
            if marker in contents:
                problems.append(
                    f"{kernel}: contains {marker.decode('utf-8')!r}; "
                    "this is an integration-test build, not the application"
                )
    return problems


def verify(bundle: Path, platform: str) -> None:
    if not bundle.is_dir():
        raise BundleError(f"bundle directory does not exist: {bundle}")

    launcher = bundle / LAUNCHERS[platform]
    if not launcher.is_file():
        raise BundleError(f"bundle has no {LAUNCHERS[platform]} launcher: {launcher}")

    kernels = kernel_files(bundle)
    if not kernels:
        raise BundleError(
            f"no kernel_blob.bin under {bundle}: the guard cannot tell which "
            "entrypoint this bundle carries, so it must not be packaged"
        )

    problems = find_test_entrypoint(bundle)
    if problems:
        raise BundleError("\n".join(problems))


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--platform", required=True, choices=sorted(LAUNCHERS))
    parser.add_argument("bundle", type=Path)
    args = parser.parse_args(argv)
    try:
        verify(args.bundle, args.platform)
    except BundleError as error:
        print(f"APP BUNDLE INVALID: {error}", file=sys.stderr)
        return 1
    kernels = kernel_files(args.bundle)
    sizes = ", ".join(f"{kernel.name} {kernel.stat().st_size} bytes" for kernel in kernels)
    print(f"APP BUNDLE OK: {args.platform} {args.bundle} ({sizes})")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
