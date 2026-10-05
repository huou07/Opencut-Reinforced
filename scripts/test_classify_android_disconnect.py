#!/usr/bin/env python3
"""Deterministic cases for the Android disconnect classifier.

Synthetic log text only. These are not acceptance evidence.
"""
import os
import sys
import tempfile
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from classify_android_disconnect import CLASSES, classify, collect, main  # noqa: E402

STARTED = "I/flutter: VM service is listening on "


def texts(**kwargs):
    base = {"driver": "", "guest": "", "state": "", "health": ""}
    base.update(kwargs)
    return base


class ClassificationTest(unittest.TestCase):
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
        guest = "I/flutter: VM service is listening on\n" \
                "Fatal signal 11 (SIGSEGV), code 1 in tid 1234\n"
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
        jni = classify(texts(guest="JNI DETECTED ERROR IN APP: reference to bad global\n"))
        app = classify(texts(guest="FATAL EXCEPTION: main java.lang.IllegalStateException\n"))
        self.assertEqual(jni["primary_class"], "JNI_CRASH")
        self.assertEqual(app["primary_class"], "APP_CRASH")

    def test_main_thread_stall_needs_three_digit_skipped_frames(self):
        mild = classify(texts(guest="I/Choreographer: Skipped 12 frames\n"))
        severe = classify(texts(guest="I/Choreographer: Skipped 412 frames\n"))
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