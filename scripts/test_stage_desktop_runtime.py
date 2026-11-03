#!/usr/bin/env python3
"""Checks that the shared runtime stager stages every desktop platform alike."""

from __future__ import annotations

import os
import stat
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
STAGER = ROOT / "scripts" / "stage_desktop_runtime.py"


def write(path: Path, text: str = "#!/bin/sh\n") -> Path:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text)
    path.chmod(path.stat().st_mode | stat.S_IXUSR)
    return path


class StageDesktopRuntimeTest(unittest.TestCase):
    def run_stager(self, *arguments: str) -> subprocess.CompletedProcess:
        return subprocess.run(
            [sys.executable, str(STAGER), *arguments],
            capture_output=True,
            text=True,
            check=False,
        )

    def test_linux_bundle_carries_engine_libraries_and_cli(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            engine = write(root / "engine" / "whisper-cli")
            write(root / "engine" / "libwhisper.so.1")
            write(root / "engine" / "main")
            provider = write(root / "target" / "release" / "or-whisper-provider")
            cli = write(root / "target" / "release" / "or")
            bundle = root / "bundle"
            bundle.mkdir()

            result = self.run_stager(
                "--os",
                "linux",
                "--runtime-dir",
                str(bundle),
                "--repo-root",
                str(root),
                "--whisper-cli",
                str(engine),
                "--provider",
                str(provider),
                "--or-cli",
                str(cli),
                "--require",
                "whisper,provider,or",
            )

            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertTrue((bundle / "whisper-cli").is_file())
            self.assertTrue((bundle / "libwhisper.so.1").is_file())
            self.assertTrue((bundle / "or-whisper-provider").is_file())
            self.assertTrue((bundle / "or").is_file())
            self.assertFalse((bundle / "main").exists())
            self.assertTrue(os.access(bundle / "or", os.X_OK))

    def test_windows_bundle_uses_exe_names(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            engine = write(root / "engine" / "whisper-cli.exe")
            write(root / "engine" / "ggml.dll")
            provider = write(root / "target" / "release" / "or-whisper-provider.exe")
            cli = write(root / "target" / "release" / "or.exe")
            bundle = root / "bundle"

            result = self.run_stager(
                "--os",
                "windows",
                "--runtime-dir",
                str(bundle),
                "--repo-root",
                str(root),
                "--whisper-cli",
                str(engine),
                "--provider",
                str(provider),
                "--or-cli",
                str(cli),
                "--require",
                "whisper,provider,or",
            )

            self.assertEqual(result.returncode, 0, result.stderr)
            for name in ("whisper-cli.exe", "ggml.dll", "or-whisper-provider.exe", "or.exe"):
                self.assertTrue((bundle / name).is_file(), name)

    def test_a_missing_adapter_fails_when_it_is_required(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            bundle = root / "bundle"
            bundle.mkdir()

            result = self.run_stager(
                "--os",
                "linux",
                "--runtime-dir",
                str(bundle),
                "--repo-root",
                str(root),
                "--require",
                "provider",
            )

            self.assertNotEqual(result.returncode, 0)
            self.assertIn("speech adapter", result.stderr)

    def test_a_bundle_without_a_speech_source_only_warns(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            cli = write(root / "target" / "release" / "or")
            bundle = root / "bundle"

            result = self.run_stager(
                "--os",
                "linux",
                "--runtime-dir",
                str(bundle),
                "--repo-root",
                str(root),
                "--or-cli",
                str(cli),
                "--require",
                "or",
            )

            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertTrue((bundle / "or").is_file())


if __name__ == "__main__":
    unittest.main()
