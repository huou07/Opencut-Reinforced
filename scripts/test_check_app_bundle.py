#!/usr/bin/env python3
"""Hermetic checks for the packaging entrypoint guard.

The 2026-10-19 Developer Preview shipped desktop bundles whose Dart entrypoint
was the integration-test listener, so the app opened to a black screen. These
checks pin the guard that makes that packaging mistake fail loudly, without a
Flutter SDK or a device: a fake bundle is enough to prove the classification.
"""
from __future__ import annotations

import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SCRIPT = ROOT / "scripts" / "check_app_bundle.py"

PRODUCT_KERNEL = (
    b"\xd9\x01\x00\x00kernel package:or_app/main.dart package:flutter/material.dart"
)
TEST_KERNEL = (
    b"\xd9\x01\x00\x00kernel package:or_app/main.dart "
    b"file:///tmp/flutter_tools.X/flutter_test_listener.Y/listener.dart "
    b"package:integration_test/integration_test.dart "
    b"integration_test/core_bridge_test.dart"
)


def run(platform: str, bundle: Path) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [sys.executable, str(SCRIPT), "--platform", platform, str(bundle)],
        capture_output=True,
        text=True,
        timeout=60,
    )


def make_bundle(root: Path, platform: str, kernel: bytes) -> Path:
    if platform == "macos":
        bundle = root / "or_app.app"
        (bundle / "Contents/MacOS").mkdir(parents=True)
        (bundle / "Contents/MacOS/or_app").write_bytes(b"launcher")
        kernel_path = (
            bundle
            / "Contents/Frameworks/App.framework/Resources/flutter_assets"
            / "kernel_blob.bin"
        )
    else:
        bundle = root / "bundle"
        (bundle / "data/flutter_assets").mkdir(parents=True)
        launcher = "or_app.exe" if platform == "windows" else "or_app"
        (bundle / launcher).write_bytes(b"launcher")
        kernel_path = bundle / "data/flutter_assets/kernel_blob.bin"
    kernel_path.parent.mkdir(parents=True, exist_ok=True)
    kernel_path.write_bytes(kernel)
    return bundle


class AppBundleGuardCase(unittest.TestCase):
    def test_every_platform_accepts_the_product_entrypoint(self) -> None:
        for platform in ("linux", "windows", "macos"):
            with self.subTest(platform=platform), tempfile.TemporaryDirectory() as directory:
                bundle = make_bundle(Path(directory), platform, PRODUCT_KERNEL)
                completed = run(platform, bundle)
                self.assertEqual(completed.returncode, 0, completed.stderr)
                self.assertIn("APP BUNDLE OK", completed.stdout)

    def test_every_platform_rejects_the_integration_test_entrypoint(self) -> None:
        for platform in ("linux", "windows", "macos"):
            with self.subTest(platform=platform), tempfile.TemporaryDirectory() as directory:
                bundle = make_bundle(Path(directory), platform, TEST_KERNEL)
                completed = run(platform, bundle)
                self.assertEqual(completed.returncode, 1, completed.stdout)
                self.assertIn("integration-test build", completed.stderr)
                self.assertIn("flutter_test_listener", completed.stderr)

    def test_the_marker_is_not_reported_for_a_product_bundle(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            bundle = make_bundle(Path(directory), "linux", PRODUCT_KERNEL)
            completed = run("linux", bundle)
            self.assertNotIn("integration-test build", completed.stderr)

    def test_a_missing_kernel_is_refused_instead_of_assumed_good(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            bundle = make_bundle(Path(directory), "linux", PRODUCT_KERNEL)
            (bundle / "data/flutter_assets/kernel_blob.bin").unlink()
            completed = run("linux", bundle)
            self.assertEqual(completed.returncode, 1)
            self.assertIn("no kernel_blob.bin", completed.stderr)

    def test_a_missing_launcher_is_refused(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            bundle = make_bundle(Path(directory), "linux", PRODUCT_KERNEL)
            (bundle / "or_app").unlink()
            completed = run("linux", bundle)
            self.assertEqual(completed.returncode, 1)
            self.assertIn("no or_app launcher", completed.stderr)

    def test_the_platform_launcher_name_is_enforced(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            bundle = make_bundle(Path(directory), "windows", PRODUCT_KERNEL)
            completed = run("macos", bundle)
            self.assertEqual(completed.returncode, 1)
            self.assertIn("no Contents/MacOS/or_app launcher", completed.stderr)


if __name__ == "__main__":
    unittest.main()
