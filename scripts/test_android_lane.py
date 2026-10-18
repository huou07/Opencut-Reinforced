#!/usr/bin/env python3
"""Control-flow checks for the local Android lane.

These are hermetic: no device, no emulator, no network, no Gradle. They point
the lane's documented seams at stubs and assert what the lane reports and which
exit code it returns, because the lane's honesty rules (a host precondition is
BLOCKED, a dirty tree is never PASS) are the part that must not rot while no
device is available.
"""
from __future__ import annotations

import json
import os
import stat
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
LANE = ROOT / "scripts" / "run_android_lane.sh"


def write_executable(path: Path, body: str) -> None:
    path.write_text(body, encoding="utf-8")
    path.chmod(path.stat().st_mode | stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH)


class AndroidLaneCase(unittest.TestCase):
    def setUp(self) -> None:
        self._tmp = tempfile.TemporaryDirectory()
        self.tmp = Path(self._tmp.name)

        # A sandbox that satisfies every precondition the case does not remove.
        self.fake_bin = self.tmp / "bin"
        self.fake_bin.mkdir()
        write_executable(self.fake_bin / "adb", "#!/usr/bin/env bash\nprintf 'List of devices attached\\n'\n")
        write_executable(self.fake_bin / "flutter", "#!/usr/bin/env bash\nexit 0\n")

        self.sdk = self.tmp / "sdk"
        (self.sdk / "emulator").mkdir(parents=True)
        write_executable(self.sdk / "emulator" / "emulator", "#!/usr/bin/env bash\nexit 0\n")

        self.avd_home = self.tmp / "avd"
        self.avd_home.mkdir()
        (self.avd_home / "an3-api36.ini").write_text("", encoding="utf-8")

        self.prefix = self.tmp / "ffmpeg"
        (self.prefix / "lib" / "pkgconfig").mkdir(parents=True)

        self.stubs = self.tmp / "stubs"
        self.stubs.mkdir()
        for name in ("start", "preview", "stop"):
            write_executable(self.stubs / f"{name}.sh", "#!/usr/bin/env bash\nexit 0\n")

        self.meminfo = self.tmp / "meminfo"
        self.set_memory_gib(9)
        self.report = self.tmp / "android.json"

    def tearDown(self) -> None:
        self._tmp.cleanup()

    def set_memory_gib(self, gib: int) -> None:
        kib = gib * 1024 * 1024
        self.meminfo.write_text(
            f"MemTotal:       16055732 kB\nMemAvailable:   {kib} kB\n", encoding="utf-8")

    def env(self, **overrides: str) -> dict[str, str]:
        env = dict(os.environ)
        env.update({
            "PATH": f"{self.fake_bin}{os.pathsep}{os.environ.get('PATH', '')}",
            "ANDROID_HOME": str(self.sdk),
            "ANDROID_AVD_HOME": str(self.avd_home),
            "OR_LANE_MEMINFO": str(self.meminfo),
            "OR_ANDROID_FFMPEG_PREFIX": str(self.prefix),
            "OR_ANDROID_START_SCRIPT": str(self.stubs / "start.sh"),
            "OR_ANDROID_PREVIEW_SCRIPT": str(self.stubs / "preview.sh"),
            "OR_ANDROID_STOP_SCRIPT": str(self.stubs / "stop.sh"),
        })
        env.update(overrides)
        return env

    def run_lane(self, *args: str, **overrides: str) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            ["bash", str(LANE), "--report", str(self.report), *args],
            cwd=str(ROOT), env=self.env(**overrides),
            capture_output=True, text=True, timeout=300)

    def payload(self) -> dict:
        return json.loads(self.report.read_text(encoding="utf-8"))

    # --- invocation -------------------------------------------------------
    def test_unknown_step_is_a_usage_error(self) -> None:
        result = self.run_lane("--steps", "apk,nonsense")
        self.assertEqual(result.returncode, 2)
        self.assertIn("unknown step", result.stderr)

    def test_repeated_step_is_a_usage_error(self) -> None:
        result = self.run_lane("--steps", "apk,apk")
        self.assertEqual(result.returncode, 2)
        self.assertIn("named twice", result.stderr)

    # --- host preconditions ----------------------------------------------
    def test_low_memory_is_blocked_not_failed(self) -> None:
        self.set_memory_gib(1)
        result = self.run_lane()
        self.assertEqual(result.returncode, 3)
        payload = self.payload()
        self.assertEqual(payload["result"], "BLOCKED")
        self.assertEqual(payload["lane"], "android")
        self.assertIn("MemAvailable", payload["steps"][0]["reason"] or "")
        self.assertFalse(payload["acceptance"])
        self.assertEqual(payload["evidence_class"], "local-developer")

    def test_missing_emulator_binary_is_blocked(self) -> None:
        empty_sdk = self.tmp / "empty-sdk"
        empty_sdk.mkdir()
        result = self.run_lane(ANDROID_HOME=str(empty_sdk))
        self.assertEqual(result.returncode, 3)
        self.assertIn("emulator binary", self.payload()["steps"][0]["reason"] or "")

    def test_unset_ffmpeg_prefix_is_blocked_with_the_recipe_pointer(self) -> None:
        result = self.run_lane("--steps", "apk", OR_ANDROID_FFMPEG_PREFIX="")
        self.assertEqual(result.returncode, 3)
        reason = self.payload()["steps"][0]["reason"] or ""
        self.assertIn("OR_ANDROID_FFMPEG_PREFIX", reason)
        self.assertIn("OR-BUILD.md", reason)

    # --- the run ----------------------------------------------------------
    def test_all_steps_pass_and_a_dirty_tree_is_never_pass(self) -> None:
        result = self.run_lane()
        payload = self.payload()
        classifications = {step["id"]: step["classification"] for step in payload["steps"]}
        for step in ("apk", "emulator", "preview"):
            self.assertEqual(classifications.get(step), "PASS", payload["steps"])
        dirty = bool(
            subprocess.run(["git", "status", "--porcelain"], cwd=str(ROOT),
                           capture_output=True, text=True).stdout.strip())
        if dirty:
            self.assertEqual(payload["result"], "DIRTY")
            self.assertEqual(result.returncode, 4)
            self.assertTrue(payload["warnings"])
        else:
            self.assertEqual(payload["result"], "PASS")
            self.assertEqual(result.returncode, 0)
        # The report never claims acceptance, whatever the run says.
        self.assertFalse(payload["acceptance"])
        self.assertEqual(payload["guest"]["avd"], "an3-api36")
        self.assertTrue(any("never becomes ACCEPTED" in line for line in payload["limits"]))

    def test_report_records_the_dedicated_guest_and_source(self) -> None:
        self.run_lane()
        payload = self.payload()
        self.assertEqual(payload["guest"]["serial"], "emulator-5580")
        self.assertRegex(payload["source"], r"^[0-9a-f]{40}$")


if __name__ == "__main__":
    sys.exit(0 if unittest.main(exit=False).result.wasSuccessful() else 1)
