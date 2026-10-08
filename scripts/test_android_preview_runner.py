#!/usr/bin/env python3
"""The SAF runner fails promptly when its native picker selector fails."""

import os
from pathlib import Path
import subprocess
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[1]


class AndroidPreviewRunnerTests(unittest.TestCase):
    def test_picker_failure_stops_a_stalled_flutter_drive(self) -> None:
        with tempfile.TemporaryDirectory(prefix="or-android-runner-test-") as temp:
            root = Path(temp)
            binaries = root / "bin"
            binaries.mkdir()
            runner_temp = root / "runner-temp"
            runner_temp.mkdir()
            output = root / "acceptance"
            output.mkdir()
            events = root / "events.log"
            emulator_pid = runner_temp / "android-emulator.pid"
            emulator_pid.write_text(f"{os.getpid()}\n")

            adb = binaries / "adb"
            adb.write_text(
                "#!/usr/bin/env bash\n"
                f'echo adb:"$*" >> "{events}"\n'
                "if [[ \"$*\" == *' logcat -b '* ]]; then sleep 60; exit; fi\n"
                "if [[ \"$*\" == *' shell echo or-ready'* ]]; then echo or-ready; exit; fi\n"
                "if [[ \"$*\" == *' get-state'* ]]; then echo device; exit; fi\n"
                "exit 0\n"
            )
            flutter = binaries / "flutter"
            flutter.write_text(f"#!/usr/bin/env bash\necho flutter >> {events}\nexec sleep 60\n")
            timeout = binaries / "timeout"
            timeout.write_text("#!/usr/bin/env bash\nshift\nexec \"$@\"\n")
            python = binaries / "python3"
            python.write_text(
                "#!/usr/bin/env bash\n"
                f'echo python:"$*" >> "{events}"\n'
                "case \"$1\" in\n"
                "  */select_android_saf_document.py) echo 'picker diagnostic'; exit 7;;\n"
                "  *) exit 0;;\n"
                "esac\n"
            )
            for executable in (adb, flutter, timeout, python):
                executable.chmod(0o755)

            env = os.environ | {
                "PATH": f"{binaries}{os.pathsep}{os.environ['PATH']}",
                "RUNNER_TEMP": str(runner_temp),
                "GITHUB_WORKSPACE": str(ROOT),
                "OR_ANDROID_ACCEPTANCE_OUTPUT": str(output),
                "OR_ANDROID_SAF_FLOW": "export",
            }
            try:
                result = subprocess.run(
                    [
                        "bash",
                        str(ROOT / "scripts/run-android-preview-check.sh"),
                        "saf",
                        "integration_test.dart",
                        "driver.dart",
                        "fixture.apk",
                        "recovery.apk",
                    ],
                    cwd=ROOT,
                    env=env,
                    text=True,
                    capture_output=True,
                    timeout=20,
                    check=False,
                )
            except subprocess.TimeoutExpired as error:
                self.fail(
                    f"runner did not stop promptly: {error.stdout!r} {error.stderr!r}; "
                    f"events={events.read_text() if events.exists() else 'none'}"
                )
            event_output = events.read_text() if events.exists() else "none"

        self.assertNotEqual(result.returncode, 0)
        self.assertIn(
            "picker diagnostic",
            result.stderr,
            f"code={result.returncode} stdout={result.stdout!r} stderr={result.stderr!r} events={event_output!r}",
        )

    def test_background_resume_uses_the_emulator_launcher(self) -> None:
        with tempfile.TemporaryDirectory(prefix="or-android-resume-test-") as temp:
            root = Path(temp)
            binaries = root / "bin"
            binaries.mkdir()
            runner_temp = root / "runner-temp"
            runner_temp.mkdir()
            output = root / "acceptance"
            output.mkdir()
            events = root / "events.log"
            (runner_temp / "android-emulator.pid").write_text(f"{os.getpid()}\n")

            adb = binaries / "adb"
            signal_queries = root / "signal-queries"
            adb.write_text(
                "#!/usr/bin/env bash\n"
                f'echo adb:"$*" >> "{events}"\n'
                "if [[ \"$*\" == *' shell run-as io.github.huou07.or_app rm -f files/or-saf-background-requested'* ]]; then exit 0; fi\n"
                "if [[ \"$*\" == *' shell run-as io.github.huou07.or_app cat files/or-saf-background-requested'* ]]; then\n"
                f'  query_count=0; [[ -f "{signal_queries}" ]] && read -r query_count < "{signal_queries}"\n'
                "  query_count=$((query_count + 1))\n"
                f'  printf "%s\\n" "$query_count" > "{signal_queries}"\n'
                "  if ((query_count >= 2)); then echo -n ANDROID_SAF_BACKGROUND_RELAUNCH_REQUESTED; fi\n"
                "  exit 0\n"
                "fi\n"
                "if [[ \"$*\" == *' logcat -b '* ]]; then sleep 60; exit; fi\n"
                "if [[ \"$*\" == *' shell echo or-ready'* ]]; then echo or-ready; exit; fi\n"
                "if [[ \"$*\" == *' get-state'* ]]; then echo device; exit; fi\n"
                "exit 0\n"
            )
            flutter = binaries / "flutter"
            flutter.write_text(
                "#!/usr/bin/env bash\n"
                f'cat >> "$OR_ANDROID_ACCEPTANCE_OUTPUT/android-guest-saf.log" <<\'LOG\'\n'
                "ANDROID_SAF_DOCUMENTS_UI_READY\n"
                "LOG\n"
                "sleep 3\n"
            )
            timeout = binaries / "timeout"
            timeout.write_text("#!/usr/bin/env bash\nshift\nexec \"$@\"\n")
            python = binaries / "python3"
            python.write_text("#!/usr/bin/env bash\nexit 0\n")
            for executable in (adb, flutter, timeout, python):
                executable.chmod(0o755)

            env = os.environ | {
                "PATH": f"{binaries}{os.pathsep}{os.environ['PATH']}",
                "RUNNER_TEMP": str(runner_temp),
                "GITHUB_WORKSPACE": str(ROOT),
                "OR_ANDROID_ACCEPTANCE_OUTPUT": str(output),
            }
            result = subprocess.run(
                [
                    "bash",
                    str(ROOT / "scripts/run-android-preview-check.sh"),
                    "saf",
                    "integration_test.dart",
                    "driver.dart",
                    "fixture.apk",
                    "recovery.apk",
                ],
                cwd=ROOT,
                env=env,
                text=True,
                capture_output=True,
                timeout=20,
                check=False,
            )
            event_output = events.read_text() if events.exists() else "none"
            guest_output = (
                output / "android-guest-saf.log"
            ).read_text() if (output / "android-guest-saf.log").exists() else "none"
            driver_output = (
                output / "android-driver-saf.log"
            ).read_text() if (output / "android-driver-saf.log").exists() else "none"
            lifecycle_status = (
                output / "android-lifecycle-saf.status"
            ).read_text() if (output / "android-lifecycle-saf.status").exists() else "none"
            lifecycle_signal_log = (
                output / "android-lifecycle-saf-signal.log"
            ).read_text() if (output / "android-lifecycle-saf-signal.log").exists() else "none"

        self.assertNotEqual(result.returncode, 0)
        self.assertIn(
            "process recovery APK is missing",
            result.stderr,
            f"code={result.returncode} stdout={result.stdout!r} stderr={result.stderr!r} events={event_output!r}",
        )
        clear_event = "shell run-as io.github.huou07.or_app rm -f files/or-saf-background-requested"
        read_event = "shell run-as io.github.huou07.or_app cat files/or-saf-background-requested"
        launch_event = "shell monkey -p io.github.huou07.or_app 1"
        self.assertIn(clear_event, event_output)
        self.assertIn(read_event, event_output)
        self.assertIn(launch_event, event_output)
        self.assertLess(event_output.index(read_event), event_output.index(launch_event))
        self.assertNotIn("ANDROID_SAF_BACKGROUND_RELAUNCH_REQUESTED", guest_output)
        self.assertNotIn("ANDROID_SAF_BACKGROUND_CONTROL_COMPLETE", driver_output)
        self.assertIn("pending", lifecycle_signal_log)
        self.assertIn("ANDROID_SAF_BACKGROUND_RELAUNCH_REQUESTED", lifecycle_signal_log)
        self.assertEqual(lifecycle_status.strip(), "0")


if __name__ == "__main__":
    unittest.main()
