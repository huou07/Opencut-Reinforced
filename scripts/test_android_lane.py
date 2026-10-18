#!/usr/bin/env python3
"""Control-flow checks for the local Android lane.

These are hermetic: no device, no emulator, no network, no Gradle. They point
the lane's documented seams at stubs and assert what the lane reports and which
exit code it returns, because the lane's honesty rules (a host precondition is
BLOCKED, a dirty tree is never PASS) are the part that must not rot while no
device is available.

They also pin the wiring that made the lane unusable even with a staged FFmpeg
prefix: the apk step must call the case-APK builder, the preview and saf steps
must pass the four positional arguments and the dedicated serial the real
preview script requires, the saf step must boot its own wiped guest, and no
artifact may land outside the directory the report was written to.
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
DEFAULT_LANE_DIR = ROOT / "apps" / "or_app" / "build" / "local-lanes"


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

        # A Flutter SDK shaped like the real one: the lane derives the SDK root
        # from the `flutter` executable on PATH and checks that the Gradle plugin
        # directory it includes is writable.
        self.flutter_sdk = self.tmp / "flutter-sdk"
        (self.flutter_sdk / "bin").mkdir(parents=True)
        write_executable(self.flutter_sdk / "bin" / "flutter", "#!/usr/bin/env bash\nexit 0\n")
        self.flutter_gradle_plugin = self.flutter_sdk / "packages" / "flutter_tools" / "gradle"
        self.flutter_gradle_plugin.mkdir(parents=True)

        self.sdk = self.tmp / "sdk"
        (self.sdk / "emulator").mkdir(parents=True)
        write_executable(self.sdk / "emulator" / "emulator", "#!/usr/bin/env bash\nexit 0\n")

        self.avd_home = self.tmp / "avd"
        self.avd_home.mkdir()
        (self.avd_home / "an3-api36.ini").write_text("", encoding="utf-8")

        self.prefix = self.tmp / "ffmpeg"
        (self.prefix / "lib" / "pkgconfig").mkdir(parents=True)

        # Every stub records its argv, so the test can assert what the lane
        # actually asked the repository's own scripts to do.
        self.capture = self.tmp / "capture"
        self.capture.mkdir()

        self.stubs = self.tmp / "stubs"
        self.stubs.mkdir()
        recorder = (
            "#!/usr/bin/env bash\n"
            'printf \'%s\\n\' "$(basename "$0") $*" >> "$OR_ANDROID_TEST_CAPTURE"\n'
            'printf \'%s\\n\' "$(basename "$0")" >> "$OR_ANDROID_TEST_ENV"\n'
            'printf \'RUNNER_TEMP=%s\\n\' "${RUNNER_TEMP:-unset}" >> "$OR_ANDROID_TEST_ENV"\n'
            'printf \'PUB_CACHE=%s\\n\' "${PUB_CACHE:-unset}" >> "$OR_ANDROID_TEST_ENV"\n'
        )
        for name in ("start", "preview", "stop"):
            write_executable(self.stubs / f"{name}.sh", recorder + "exit 0\n")
        write_executable(
            self.stubs / "case_apks.sh",
            recorder
            + 'mkdir -p "$1"\n'
            + 'for apk in diagnostics surface saf saf-recovery saf_fixture; do\n'
            + '  printf \'apk\' > "$1/$apk.apk"\n'
            + "done\n",
        )

        # The lane runs `./gradlew --stop` after its apk step; point that at a
        # stub so a control-flow check never starts a real JVM or a network
        # dependency.
        self.gradlew_dir = self.tmp / "gradle"
        self.gradlew_dir.mkdir()
        write_executable(
            self.gradlew_dir / "gradlew",
            recorder := (
                "#!/usr/bin/env bash\n"
                'printf \'%s\\n\' "$(basename "$0") $*" >> "$OR_ANDROID_TEST_CAPTURE"\n'
                "exit 0\n"
            ),
        )

        self.meminfo = self.tmp / "meminfo"
        self.set_memory_gib(9)
        # A readable and writable stand-in for /dev/kvm. The real device exists
        # on the developer workstation and does not exist on a hosted runner, so
        # the checks must not depend on it.
        self.kvm = self.tmp / "kvm"
        self.kvm.write_text("", encoding="utf-8")
        self.kvm.chmod(0o600)
        self.report = self.tmp / "report" / "android.json"

    def tearDown(self) -> None:
        self._tmp.cleanup()

    def set_memory_gib(self, gib: int) -> None:
        kib = gib * 1024 * 1024
        self.meminfo.write_text(
            f"MemTotal:       16055732 kB\nMemAvailable:   {kib} kB\n", encoding="utf-8")

    def env(self, **overrides: str) -> dict[str, str]:
        env = dict(os.environ)
        env.update({
            "PATH": f"{self.fake_bin}{os.pathsep}{self.flutter_sdk / 'bin'}{os.pathsep}{os.environ.get('PATH', '')}",
            "ANDROID_HOME": str(self.sdk),
            "ANDROID_AVD_HOME": str(self.avd_home),
            "OR_LANE_MEMINFO": str(self.meminfo),
            "OR_LANE_KVM_DEVICE": str(self.kvm),
            "OR_ANDROID_FFMPEG_PREFIX": str(self.prefix),
            "OR_ANDROID_START_SCRIPT": str(self.stubs / "start.sh"),
            "OR_ANDROID_PREVIEW_SCRIPT": str(self.stubs / "preview.sh"),
            "OR_ANDROID_STOP_SCRIPT": str(self.stubs / "stop.sh"),
            "OR_ANDROID_CASE_APK_SCRIPT": str(self.stubs / "case_apks.sh"),
            "OR_ANDROID_GRADLEW_DIR": str(self.gradlew_dir),
            "OR_ANDROID_TEST_CAPTURE": str(self.capture / "argv.txt"),
            "OR_ANDROID_TEST_ENV": str(self.capture / "env.txt"),
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

    def captured(self) -> list[str]:
        path = self.capture / "argv.txt"
        if not path.exists():
            return []
        return [line for line in path.read_text(encoding="utf-8").splitlines() if line]

    def captured_env(self) -> dict[str, str]:
        """The environment the lane handed each driven script, per script."""
        path = self.capture / "env.txt"
        if not path.exists():
            return {}
        observed: dict[str, str] = {}
        pending: str | None = None
        for line in path.read_text(encoding="utf-8").splitlines():
            if "=" not in line:
                pending = line
                continue
            key, _, value = line.partition("=")
            if pending:
                observed[f"{pending}.{key}"] = value
        return observed

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

    def test_a_kvm_device_that_is_absent_or_unwritable_is_blocked(self) -> None:
        missing = self.tmp / "no-such-kvm"
        result = self.run_lane(OR_LANE_KVM_DEVICE=str(missing))
        self.assertEqual(result.returncode, 3)
        self.assertIn("not readable and writable", self.payload()["steps"][0]["reason"] or "")

    def test_missing_emulator_binary_is_blocked(self) -> None:
        empty_sdk = self.tmp / "empty-sdk"
        empty_sdk.mkdir()
        result = self.run_lane(ANDROID_HOME=str(empty_sdk))
        self.assertEqual(result.returncode, 3)
        self.assertIn("emulator binary", self.payload()["steps"][0]["reason"] or "")

    def test_unset_ffmpeg_prefix_is_blocked_with_the_staging_recipe(self) -> None:
        result = self.run_lane("--steps", "apk", OR_ANDROID_FFMPEG_PREFIX="")
        self.assertEqual(result.returncode, 3)
        reason = self.payload()["steps"][0]["reason"] or ""
        self.assertIn("OR_ANDROID_FFMPEG_PREFIX", reason)
        self.assertIn("stage_android_ffmpeg.sh", reason)

    def test_a_read_only_flutter_gradle_plugin_is_blocked_by_name(self) -> None:
        # Gradle 9 must create a project cache inside the included Flutter Gradle
        # plugin build. When it cannot, the APK step dies with "The settings are
        # not yet available for build."; the lane must name the real cause.
        os.chmod(self.flutter_gradle_plugin, 0o555)
        try:
            result = self.run_lane("--steps", "apk")
        finally:
            os.chmod(self.flutter_gradle_plugin, 0o755)
        self.assertEqual(result.returncode, 3)
        reason = self.payload()["steps"][0]["reason"] or ""
        self.assertIn("flutter_tools/gradle", reason)
        self.assertIn("not writable", reason)
        self.assertIn("The settings are not yet available for build", reason)

    # --- wiring the previous lane never had -------------------------------
    def test_apk_step_calls_the_case_apk_builder(self) -> None:
        self.run_lane("--steps", "apk")
        lines = self.captured()
        self.assertTrue(any(line.startswith("case_apks.sh ") for line in lines), lines)
        target = Path(self.captured()[0].split(" ", 1)[1])
        self.assertTrue(str(target).startswith(str(self.report.parent)), target)
        self.assertTrue(target.name == "android-case-apks", target)

    def test_preview_step_passes_the_case_arguments_and_the_dedicated_serial(self) -> None:
        self.run_lane("--steps", "apk,emulator,preview")
        preview_calls = [line for line in self.captured() if line.startswith("preview.sh ")]
        self.assertEqual(len(preview_calls), 2, preview_calls)
        diagnostics, surface = preview_calls
        self.assertIn("diagnostics integration_test/native_bridge_diagnostics_test.dart "
                      "test_driver/integration_test.dart", diagnostics)
        self.assertIn("diagnostics.apk", diagnostics)
        self.assertIn("--keep-app-running", diagnostics)
        self.assertIn("surface integration_test/android_preview_surface_test.dart "
                      "test_driver/integration_test.dart", surface)
        self.assertIn("surface.apk", surface)
        # The real preview script now reads the serial from OR_ANDROID_DEVICE_ID;
        # the lane must export the same dedicated serial it reserves.
        payload = self.payload()
        self.assertEqual(payload["guest"]["serial"], "emulator-5580")

    def test_saf_step_boots_its_own_wiped_guest_and_installs_the_fixture(self) -> None:
        self.run_lane("--steps", "apk,emulator,saf")
        start_calls = [line for line in self.captured() if line.startswith("start.sh")]
        self.assertIn("start.sh wipe-data", start_calls, start_calls)
        saf_calls = [line for line in self.captured() if line.startswith("preview.sh saf ")]
        self.assertEqual(len(saf_calls), 1, saf_calls)
        self.assertIn("android_saf_preview_test.dart test_driver/android_saf_preview.dart", saf_calls[0])
        self.assertIn("saf.apk", saf_calls[0])
        self.assertIn("saf-recovery.apk", saf_calls[0])

    def test_artifacts_follow_the_report_path(self) -> None:
        before = set(DEFAULT_LANE_DIR.rglob("*")) if DEFAULT_LANE_DIR.exists() else set()
        self.run_lane("--steps", "apk")
        logs = list(self.report.parent.glob("android-logs/*.log"))
        self.assertTrue(logs, "the lane wrote no logs beside its report")
        after = set(DEFAULT_LANE_DIR.rglob("*")) if DEFAULT_LANE_DIR.exists() else set()
        self.assertEqual(before, after, "a redirected run wrote into the real lane directory")

    def test_the_lane_hands_the_driven_scripts_an_absolute_runner_temp(self) -> None:
        # `run-android-preview-check.sh` changes directory to apps/or_app, so a
        # relative RUNNER_TEMP stopped resolving and the case reported a driver
        # failure instead of its real cause.
        self.run_lane("--steps", "apk,emulator,preview")
        observed = self.captured_env()
        self.assertTrue(observed, self.captured_env())
        runner_temps = {value for key, value in observed.items() if key.endswith(".RUNNER_TEMP")}
        self.assertTrue(runner_temps, observed)
        for runner_temp in runner_temps:
            self.assertTrue(runner_temp.startswith("/"), runner_temp)
            self.assertTrue(Path(runner_temp).is_dir(), runner_temp)
            self.assertTrue(runner_temp.startswith(str(self.report.parent)), runner_temp)

    def test_the_lane_exports_a_writable_pub_cache_for_the_driven_cases(self) -> None:
        # `flutter drive` runs `pub get`; against a shared cache this UID cannot
        # mutate, the case died with `Deletion failed ... Operation not permitted`.
        shared = self.tmp / "shared-pub-cache"
        (shared / "hosted" / "pub.dev").mkdir(parents=True)
        (shared / "hosted" / "pub.dev").chmod(0o555)
        local_cache = ROOT / "apps" / "or_app" / ".dart_tool" / "aeh-pub-cache"
        staged = (local_cache / "hosted" / "pub.dev").is_dir()
        try:
            self.run_lane("--steps", "apk,emulator,preview", PUB_CACHE=str(shared))
            observed = self.captured_env()
            pub_caches = {value for key, value in observed.items() if key.endswith(".PUB_CACHE")}
            self.assertTrue(pub_caches, observed)
            pub_cache = sorted(pub_caches)[0]
            if staged:
                self.assertNotEqual(pub_cache, str(shared), pub_cache)
                self.assertIn("aeh-pub-cache", pub_cache, pub_cache)
            else:
                # Without a staged local cache the lane must say so instead of
                # letting the case fail inside `pub get`.
                warnings = self.payload()["warnings"]
                self.assertTrue(any("pub cache" in entry for entry in warnings), warnings)
        finally:
            (shared / "hosted" / "pub.dev").chmod(0o755)

    def test_report_hashes_the_driven_apks_and_names_the_missing_probe(self) -> None:
        self.run_lane("--steps", "apk")
        payload = self.payload()
        names = {Path(artifact["path"]).name for artifact in payload["artifacts"]}
        self.assertIn("diagnostics.apk", names)
        for artifact in payload["artifacts"]:
            self.assertRegex(artifact["sha256"], r"^[0-9a-f]{64}$")
        self.assertTrue(any("link probe" in entry for entry in payload["not_run"]), payload["not_run"])
        self.assertTrue(any("link probe" in warning for warning in payload["warnings"]), payload["warnings"])

    def test_a_staged_probe_runs_before_the_diagnostics_case(self) -> None:
        probe = self.tmp / "ffmpeg-link-probe"
        write_executable(probe, "#!/usr/bin/env bash\nexit 0\n")
        self.run_lane("--steps", "apk,emulator,preview", OR_ANDROID_FFMPEG_PROBE=str(probe))
        payload = self.payload()
        preview = next(step for step in payload["steps"] if step["id"] == "preview")
        self.assertIn("run_android_ffmpeg_probe.sh", preview["command"])
        self.assertEqual(payload["not_run"], [])

    # --- the run ----------------------------------------------------------
    def test_all_steps_pass_and_a_dirty_tree_is_never_pass(self) -> None:
        result = self.run_lane()
        payload = self.payload()
        classifications = {step["id"]: step["classification"] for step in payload["steps"]}
        for step in ("apk", "emulator", "preview", "saf"):
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

    # --- the memory floor is derived, not flat ---------------------------
    def test_a_guest_run_derives_its_floor_from_the_guest_it_boots(self) -> None:
        # 3072 MB guest + 1024 MB emulator + 1024 MB host margin = 5120 MiB.
        # What matters is not the constant but that the floor follows the guest
        # the operator asked for instead of a flat 7 GiB no default guest fits
        # under on this host.
        self.set_memory_gib(9)
        result = self.run_lane("--steps", "apk,emulator", OR_ANDROID_GUEST_RAM_MB="3072")
        self.assertIn(result.returncode, (0, 4), result.stderr)  # 4 == dirty tree
        floor = self.payload()["memory_floor"]
        self.assertEqual(floor["mib"], 3072 + 1024 + 1024)
        self.assertIn("guest 3072 MB", floor["derivation"])

    def test_an_apk_only_run_does_not_demand_guest_memory(self) -> None:
        # 6 GiB builds the APK but is below the old flat 7 GiB floor and far below
        # what a 6 GiB guest needs: an apk-only lane that demanded the guest's RAM
        # would report BLOCKED for work it can actually do.
        self.set_memory_gib(6)
        result = self.run_lane("--steps", "apk")
        self.assertIn(result.returncode, (0, 4), result.stderr)
        floor = self.payload()["memory_floor"]
        self.assertEqual(floor["mib"], 5120)
        self.assertIn("apk-only", floor["derivation"])
        self.assertLess(floor["mib"], 7 * 1024)

    def test_the_same_headroom_blocks_the_guest_run_that_the_apk_run_allowed(self) -> None:
        self.set_memory_gib(6)
        result = self.run_lane("--steps", "apk,emulator")
        self.assertEqual(result.returncode, 3)
        reason = self.payload()["steps"][0]["reason"] or ""
        self.assertIn("below the", reason)
        self.assertIn("lower OR_ANDROID_GUEST_RAM_MB", reason)

    def test_the_explicit_override_wins_and_is_named_in_the_report(self) -> None:
        self.set_memory_gib(3)
        result = self.run_lane("--steps", "apk", OR_ANDROID_LANE_MIN_MEM_GIB="2")
        self.assertIn(result.returncode, (0, 4), result.stderr)
        floor = self.payload()["memory_floor"]
        self.assertEqual(floor["mib"], 2048)
        self.assertIn("override", floor["derivation"])

    # --- the lane reaps the daemons it started, and only those -----------
    def fake_daemon_command(self, marker: str, version: str) -> str:
        """Return a shell line that leaves a Gradle-daemon-shaped process alive.

        `bash -c 'sleep 120; :'` keeps its argv: with a single simple command
        bash execs it and the extra arguments disappear, which would make the
        stand-in invisible to `pgrep -f` and to the lane's attribution.
        """
        return (
            f'bash -c "sleep 120; :" GradleDaemon "gradle-{version}-all" {marker} '
            f'>/dev/null 2>&1 & echo $! >> "$OR_ANDROID_TEST_DAEMON_PIDS"\n'
        )

    def test_the_lane_stops_its_own_daemon_and_leaves_a_foreign_one_alone(self) -> None:
        version = self.wrapper_version()
        pids_file = self.tmp / "daemon-pids"
        # The stub builder starts what the real build starts: one daemon on the
        # wrapper's Gradle version, which is the lane's, and one on another
        # version, standing in for a sibling project's build.
        write_executable(
            self.stubs / "case_apks.sh",
            "#!/usr/bin/env bash\n"
            'printf \'%s\\n\' "$(basename "$0") $*" >> "$OR_ANDROID_TEST_CAPTURE"\n'
            'mkdir -p "$1"\n'
            'for apk in diagnostics surface saf saf-recovery saf_fixture; do\n'
            '  printf \'apk\' > "$1/$apk.apk"\n'
            "done\n"
            + self.fake_daemon_command("OURS", version)
            + self.fake_daemon_command("FOREIGN", "8.14.3")
            + "sleep 0.5\nexit 0\n",
        )
        result = self.run_lane("--steps", "apk",
                               OR_ANDROID_TEST_DAEMON_PIDS=str(pids_file))
        self.assertIn(result.returncode, (0, 4), result.stderr)
        ours, foreign = (int(line) for line in pids_file.read_text().split())
        try:
            self.assertFalse(self.pid_alive(ours),
                             "the lane left the Gradle daemon its own apk step started running")
            self.assertTrue(self.pid_alive(foreign),
                            "the lane stopped a daemon it did not start")
            payload = self.payload()
            self.assertTrue(payload["daemons_stopped"], payload["warnings"])
            self.assertIn(f"gradle-{version}", payload["daemons_stopped"][0])
            self.assertTrue(
                any("does not belong to this lane" in entry for entry in payload["warnings"]),
                payload["warnings"])
        finally:
            for pid in (ours, foreign):
                try:
                    os.kill(pid, 9)
                except OSError:
                    pass

    def wrapper_version(self) -> str:
        text = (ROOT / "apps/or_app/android/gradle/wrapper/gradle-wrapper.properties").read_text()
        match = __import__("re").search(r"gradle-([0-9][0-9.]*)-all\.zip", text)
        self.assertIsNotNone(match, "the wrapper properties name no Gradle version")
        return match.group(1)

    @staticmethod
    def pid_alive(pid: int) -> bool:
        """A zombie is dead: the stub's orphan is reparented and reaped by init."""
        try:
            os.kill(pid, 0)
        except OSError:
            return False
        try:
            state = Path(f"/proc/{pid}/stat").read_text().split(") ", 1)[1].split()[0]
        except (OSError, IndexError):
            return False
        return state != "Z"


if __name__ == "__main__":
    sys.exit(0 if unittest.main(exit=False).result.wasSuccessful() else 1)
