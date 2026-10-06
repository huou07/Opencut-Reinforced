#!/usr/bin/env python3
"""Deterministic cases for the Android disconnect classifier.

Synthetic log text only. These are not acceptance evidence.
"""
import os
import sys
import tempfile
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from classify_android_disconnect import (  # noqa: E402
    CLASSES, classify, collect, main, or_pids)

# Any real OR guest log carries the package name, which is how OR's PID is
# learned. A log without it cannot be attributed to OR and is refused.
OR_MARKER = "I/ImeTracker( 3304): io.github.huou07.or_app:6d6dcbca: onCancelled"
STARTED = "I/flutter ( 3304): The Dart VM service is listening on "

# Real hosted lines from run 37363615433 attempt 3. OR ran as PID 3304; the
# 514/307/144-frame stalls were PID 1042 (Play services) and the ANR was
# com.google.android.gms.persistent. Classifying those as an OR defect would
# blame the product for emulator instability.
REAL_GUEST = "\n".join([
    "10-05 20:40:13.077  1324  1324 I Choreographer: Skipped 72 frames!",
    "10-05 20:40:14.983  1042  1042 I Choreographer: Skipped 514 frames!  The application may be doing too much work on its main thread.",
    "10-05 20:40:20.210  1042  1042 I Choreographer: Skipped 307 frames!",
    "10-05 20:40:31.680   677  2602 E ActivityManager: ANR in com.google.android.gms.persistent",
    "10-05 20:40:58.431  3304  3402 I flutter : The Dart VM service is listening on http://127.0.0.1:34633/",
    "10-05 20:40:45.442  3304  3304 I Choreographer: Skipped 110 frames!  The application may be doing too much work on its main thread.",
    "I/ImeTracker( 3304): io.github.huou07.or_app:6d6dcbca: onCancelled at PHASE_CLIENT_ALREADY_HIDDEN",
])
# The real driver output after the emulator disappeared between Android cases.
REAL_LOST_EMULATOR = (
    "No supported devices found with name or id matching 'emulator-5554'.\n"
    "\nThe following devices were found:\n"
    "Linux (desktop) - linux  - linux-x64\n"
)


def texts(**kwargs):
    base = {"driver": "", "guest": "", "state": "", "health": ""}
    base.update(kwargs)
    return base


class ClassificationTest(unittest.TestCase):
    def test_real_play_services_stall_is_not_attributed_to_or(self):
        result = classify(texts(guest=REAL_GUEST, driver="All tests passed!"))
        self.assertEqual(result["or_pids"], ["3304"])
        self.assertEqual(result["primary_class"], "MAIN_THREAD_STALL")
        for hit in result["evidence"]["MAIN_THREAD_STALL"]:
            self.assertIn(" 3304 ", hit["line"] + " ")

    def test_real_play_services_stall_alone_is_not_an_or_defect(self):
        # Same log without OR's own frames: nothing may be blamed on OR.
        without_or = "\n".join(l for l in REAL_GUEST.splitlines()
                               if " 3304 " not in l)
        result = classify(texts(guest=without_or, driver="All tests passed!"))
        self.assertFalse(result["disconnect_classified"])

    def test_real_lost_emulator_is_classified_as_instability(self):
        result = classify(texts(guest=REAL_GUEST, driver=REAL_LOST_EMULATOR))
        self.assertEqual(result["primary_class"], "EMULATOR_INSTABILITY")
        self.assertIn("No supported devices found", result["evidence"]
                      ["EMULATOR_INSTABILITY"][0]["line"])

    def test_or_pids_is_empty_when_the_package_never_appears(self):
        self.assertEqual(or_pids(texts(guest="I/Choreographer: Skipped 400 frames!")), set())

    def test_clean_pass_is_not_a_disconnect(self):
        result = classify(texts(guest=STARTED, driver="All tests passed!"))
        self.assertEqual(result["primary_class"], "PASS")
        self.assertFalse(result["disconnect_classified"])

    def test_vm_service_never_announced_is_not_reported_as_a_product_failure(self):
        result = classify(texts(driver="Gradle build failed"))
        self.assertFalse(result["vm_service_announced"])
        # The class names the harness stage, and the absent VM service proves
        # product code never ran, so this cannot be read as a product defect.
        self.assertEqual(result["primary_class"], "DRIVER_LIFECYCLE")
        self.assertEqual(result["all_classes"], ["DRIVER_LIFECYCLE"])

    def test_unexplained_disconnect_with_no_start_is_still_distinguished(self):
        result = classify(texts(driver="connection closed unexpectedly"))
        self.assertFalse(result["disconnect_classified"])
        self.assertEqual(result["primary_class"], "NEVER_REACHED_APP")

    def test_native_crash_outranks_following_driver_symptom(self):
        guest = "\n".join([OR_MARKER, STARTED,
                            "10-05 20:41:01.000  3304  3304 F libc : Fatal signal 11 (SIGSEGV), code 1 in tid 1234"]) + "\n"
        result = classify(texts(guest=guest, driver="lost connection to device"))
        self.assertEqual(result["primary_class"], "NATIVE_CRASH")
        self.assertIn("VM_SERVICE_FAILURE", result["all_classes"])
        self.assertTrue(result["evidence"]["NATIVE_CRASH"])

    def test_emulator_instability_outranks_everything(self):
        health = "adb: no devices/emulators found\n"
        guest = "Fatal signal 6 (SIGABRT)\n"
        result = classify(texts(health=health, guest=guest))
        self.assertEqual(result["primary_class"], "EMULATOR_INSTABILITY")

    def test_jni_and_app_crash_are_distinct_from_native(self):
        jni = classify(texts(guest=OR_MARKER + "\n10-05 20:41:02.000  3304  3304 E art : JNI DETECTED ERROR IN APP: reference to bad global\n"))
        app = classify(texts(guest=OR_MARKER + "\n10-05 20:41:03.000  3304  3304 E AndroidRuntime: FATAL EXCEPTION: main java.lang.IllegalStateException\n"))
        self.assertEqual(jni["primary_class"], "JNI_CRASH")
        self.assertEqual(app["primary_class"], "APP_CRASH")

    def test_main_thread_stall_needs_three_digit_skipped_frames(self):
        mild = classify(texts(guest=OR_MARKER + "\n10-05 20:41:01.000  3304  3304 I Choreographer: Skipped 12 frames\n"))
        severe = classify(texts(guest=OR_MARKER + "\n10-05 20:41:01.000  3304  3304 I Choreographer: Skipped 412 frames\n"))
        self.assertFalse(mild["disconnect_classified"])
        self.assertEqual(severe["primary_class"], "MAIN_THREAD_STALL")

    def test_every_class_has_at_least_one_pattern_and_a_known_source(self):
        sources = {"driver", "guest", "state", "health"}
        for name, patterns, used in CLASSES:
            self.assertTrue(patterns, name)
            self.assertLessEqual(set(used), sources, name)

    def test_evidence_lines_name_their_source(self):
        result = classify(texts(state="Guest ANR/tombstone listings:\n"))
        for hits in result["evidence"].values():
            for hit in hits:
                self.assertIn(hit["source"], ("driver", "guest", "state", "health"))
                self.assertTrue(hit["line"])

    def test_an_unattributable_crash_is_not_blamed_on_or(self):
        # Another guest process crashing must not become an OR NATIVE_CRASH.
        result = classify(texts(guest="10-05 20:41:01.000  1042  1042 F libc : Fatal signal 11 (SIGSEGV)\n"))
        self.assertFalse(result["disconnect_classified"])
        self.assertEqual(result["primary_class"], "NEVER_REACHED_APP")

    def test_missing_files_are_tolerated_not_fatal(self):
        with tempfile.TemporaryDirectory() as work:
            missing = os.path.join(work, "absent.log")
            self.assertEqual(collect({"guest": missing})["guest"], "")
            out = os.path.join(work, "nested", "result.json")
            self.assertEqual(main(["--guest", missing, "--output", out]), 0)
            self.assertTrue(os.path.isfile(out))

    def test_missing_all_logs_is_a_usage_error(self):
        with self.assertRaises(SystemExit) as raised:
            main([])
        self.assertNotEqual(raised.exception.code, 0)


if __name__ == "__main__":
    unittest.main()