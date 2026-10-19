#!/usr/bin/env python3
"""Hermetic checks for the emulator start script's serial and sizing rules.

Measured 2026-10-18: the local lane reserved `emulator-5580`, the start script
launched the emulator without `-port`, the guest came up on the default 5554, the
lane's `adb -s emulator-5580 wait-for-device` timed out at exactly its 120 s
bound, and the guest was left running on the host. These checks pin the two rules
that failure needed and need no device, no KVM and no emulator binary:

* the emulator is launched on the console port its reserved serial names;
* a reserved port that is already in use is named as a precondition, not waited
  out;
* a device id that is not an emulator console serial is a usage error.

They also pin the sizing rule: the guest RAM and core count the caller asked for
are written into the AVD configuration.
"""
from __future__ import annotations

import os
import re
import shutil
import stat
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SCRIPT = ROOT / "scripts" / "start-android-emulator.sh"


def write_executable(path: Path, body: str) -> None:
    path.write_text(body, encoding="utf-8")
    path.chmod(path.stat().st_mode | stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH)


class EmulatorStartCase(unittest.TestCase):
    def setUp(self) -> None:
        self._tmp = tempfile.TemporaryDirectory()
        self.tmp = Path(self._tmp.name)
        self.bin = self.tmp / "bin"
        self.bin.mkdir()
        self.capture = self.tmp / "capture.txt"
        self.runner_temp = self.tmp / "runner"
        self.runner_temp.mkdir()

        # A stub emulator that records its argv and exits without starting
        # anything: the checks below are about what the script asks for.
        self.emulator = self.tmp / "emulator-stub"
        write_executable(
            self.emulator,
            "#!/usr/bin/env bash\n"
            'printf \'emulator %s\\n\' "$*" >> "$OR_TEST_CAPTURE"\n'
            "sleep 30\n",
        )

        # A stub adb that answers the exact probes the script makes for the
        # serial it was told to use, and refuses any other serial: the point of
        # the regression is that the two must agree.
        write_executable(
            self.bin / "adb",
            "#!/usr/bin/env bash\n"
            'printf \'adb %s\\n\' "$*" >> "$OR_TEST_CAPTURE"\n'
            'args="$*"\n'
            'case "$args" in\n'
            "  *-s\\ *) serial=\"${args#*-s }\" ; serial=\"${serial%% *}\" ;;\n"
            "  *) serial=\"\" ;;\n"
            "esac\n"
            'if [[ -n "$OR_TEST_EXPECTED_SERIAL" && "$serial" != "$OR_TEST_EXPECTED_SERIAL" ]]; then\n'
            "  exit 1\n"
            "fi\n"
            'case "$args" in\n'
            "  *wait-for-device*) exit 0 ;;\n"
            "  *getprop\\ sys.boot_completed*) printf '1\\n' ;;\n"
            "  *getprop\\ ro.build.version.sdk*) printf '36\\n' ;;\n"
            "  *getprop\\ ro.product.cpu.abi*) printf 'x86_64\\n' ;;\n"
            "  *get-state*) printf 'device\\n' ;;\n"
            "  *) exit 0 ;;\n"
            "esac\n",
        )

        # avdmanager only has to confirm the AVD the test pre-creates.
        write_executable(
            self.bin / "avdmanager",
            "#!/usr/bin/env bash\n"
            'printf \'%s\\n\' "$OR_TEST_AVD"\n',
        )

        self.avd_home = self.tmp / "avd"
        self.avd_home.mkdir()
        self.avd = "or-api36-x86_64"
        (self.avd_home / f"{self.avd}.ini").write_text("", encoding="utf-8")
        (self.avd_home / f"{self.avd}.avd").mkdir()

        self.kvm = self.tmp / "kvm"
        self.kvm.write_text("", encoding="utf-8")
        self.kvm.chmod(0o600)

    def tearDown(self) -> None:
        self._tmp.cleanup()

    def run_script(self, *args: str, **overrides: str) -> subprocess.CompletedProcess[str]:
        environment = dict(
            os.environ,
            PATH=f"{self.bin}{os.pathsep}{os.environ.get('PATH', '')}",
            HOME=str(self.tmp),
            ANDROID_HOME=str(self.tmp / "sdk"),
            ANDROID_AVD_HOME=str(self.avd_home),
            RUNNER_TEMP=str(self.runner_temp),
            OR_TEST_CAPTURE=str(self.capture),
            OR_TEST_AVD=self.avd,
            OR_AVD_NAME=self.avd,
            OR_ANDROID_EMULATOR_BIN=str(self.emulator),
            OR_ANDROID_KVM_DEVICE=str(self.kvm),
        )
        environment.update(overrides)
        return subprocess.run(["bash", str(SCRIPT), *args], cwd=ROOT, env=environment,
                              capture_output=True, text=True, timeout=300)

    def captured(self) -> list[str]:
        if not self.capture.exists():
            return []
        return [line for line in self.capture.read_text(encoding="utf-8").splitlines() if line]

    def emulator_args(self) -> str:
        for line in self.captured():
            if line.startswith("emulator "):
                return line
        return ""

    def test_the_emulator_listens_on_the_port_its_reserved_serial_names(self) -> None:
        result = self.run_script(
            OR_ANDROID_DEVICE_ID="emulator-5580",
            OR_TEST_EXPECTED_SERIAL="emulator-5580",
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("-port 5580", self.emulator_args(), self.captured())
        self.assertIn("emulator-5580", result.stdout)
        self.assertIn("console port 5580", result.stdout)
        # Every probe must name the reserved serial, not a default one.
        adb_lines = [line for line in self.captured() if line.startswith("adb ")]
        self.assertTrue(adb_lines, self.captured())
        for line in adb_lines:
            self.assertIn("-s emulator-5580", line, self.captured())

    def test_a_serial_that_is_not_an_emulator_console_is_a_usage_error(self) -> None:
        result = self.run_script(OR_ANDROID_DEVICE_ID="device-serial")
        self.assertEqual(result.returncode, 2)
        self.assertIn("emulator console serial", result.stderr)
        self.assertEqual(self.emulator_args(), "")

    def test_an_occupied_reserved_port_is_named_not_waited_out(self) -> None:
        import socket
        listener = socket.socket()
        listener.bind(("127.0.0.1", 5599))
        listener.listen(1)
        try:
            result = self.run_script(OR_ANDROID_DEVICE_ID="emulator-5599")
        finally:
            listener.close()
        self.assertEqual(result.returncode, 3, result.stderr)
        self.assertIn("already in use", result.stderr)
        self.assertEqual(self.emulator_args(), "")

    def test_the_requested_guest_size_is_written_to_the_avd(self) -> None:
        result = self.run_script(
            OR_ANDROID_DEVICE_ID="emulator-5580",
            OR_TEST_EXPECTED_SERIAL="emulator-5580",
            OR_ANDROID_GUEST_RAM_MB="3072",
            OR_ANDROID_GUEST_CORES="6",
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        config = (self.avd_home / f"{self.avd}.avd" / "config.ini").read_text(encoding="utf-8")
        self.assertIn("hw.ramSize=3072", config)
        self.assertIn("hw.cpu.ncore=6", config)


if __name__ == "__main__":
    sys.exit(0 if unittest.main(exit=False).result.wasSuccessful() else 1)
