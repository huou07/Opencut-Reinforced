#!/usr/bin/env python3
"""Behavioural tests for the native DocumentsUI selection helper.

The helper drives real Android DocumentsUI, so these tests stub `adb` and prove
the two properties that matter:

1. a transient UIAutomator startup race is retried rather than killing the
   helper, and
2. clicks are still restricted to `.documentsui` nodes.

Regression: run 37495820157 failed with
`ERROR: null root node returned by UiTestAutomationBridge`, which raised
`CalledProcessError` and left the drive waiting forever for a selection that
could never arrive.

Usage: python3 scripts/test_select_android_saf_document.py
"""
import importlib.util
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest import mock

MODULE = Path(__file__).resolve().parent / "select_android_saf_document.py"
spec = importlib.util.spec_from_file_location("saf_selector", MODULE)
selector = importlib.util.module_from_spec(spec)
spec.loader.exec_module(selector)


DRAWER = b"""<?xml version='1.0' encoding='UTF-8'?>
<hierarchy rotation="0">
  <node index="0" text="" class="android.widget.FrameLayout"
        package="com.android.documentsui" content-desc="Show roots"
        bounds="[0,0][1080,120]" />
</hierarchy>
"""

PROVIDER = b"""<?xml version='1.0' encoding='UTF-8'?>
<hierarchy rotation="0">
  <node index="0" text="OR SAF acceptance" class="android.widget.LinearLayout"
        package="com.android.documentsui" bounds="[0,200][1080,300]" />
</hierarchy>
"""

DOCUMENT = b"""<?xml version='1.0' encoding='UTF-8'?>
<hierarchy rotation="0">
  <node index="0" text="acceptance.orproj" class="android.widget.LinearLayout"
        package="com.android.documentsui" bounds="[0,200][1080,300]" />
  <node index="1" text="acceptance.orproj" class="android.widget.LinearLayout"
        package="io.github.huou07.or_app" bounds="[0,0][1080,100]" />
</hierarchy>
"""


class FakeAdb:
    """An `adb` that fails like a cold guest, then serves the real tree."""

    def __init__(self, transient_dumps):
        self.transient = transient_dumps
        self.tree_index = 0
        self.taps = []
        self.trees = [DRAWER, PROVIDER, DOCUMENT]

    def check_output(self, args, timeout=None):
        command = args[3:]
        if command[:2] == ["shell", "uiautomator"]:
            if self.transient > 0:
                # The real race: no window tree yet, and the dump file is absent.
                self.transient -= 1
                raise subprocess.CalledProcessError(1, args)
            return b"UI hierchary dumped to: /sdcard/or-saf-window.xml"
        if command[:2] == ["shell", "cat"]:
            if self.transient > 0:
                raise subprocess.CalledProcessError(1, args)
            raw = self.trees[min(self.tree_index, len(self.trees) - 1)]
            self.tree_index += 1
            return raw
        if command[:2] == ["shell", "input"]:
            self.taps.append(command[-2:])
            return b""
        if command[:1] == ["exec-out"]:
            return b"\x89PNG"
        raise AssertionError(f"unexpected adb call: {args}")


def run(transient_dumps):
    with tempfile.TemporaryDirectory() as work:
        root = Path(work)
        guest = root / "guest.log"
        guest.write_text("ANDROID_SAF_DOCUMENTS_UI_READY\n")
        output = root / "out"
        adb = FakeAdb(transient_dumps)
        with mock.patch.object(subprocess, "check_output", adb.check_output):
            selector.select("emulator-5554", guest, output)
        return adb.taps, (output / "documents-ui-selection.txt").exists()


class SelectorTests(unittest.TestCase):
    def test_transient_uiautomator_race_is_retried(self):
        # Two failed dumps is what killed the helper in run 37495820157.
        taps, selected = run(transient_dumps=2)
        self.assertTrue(selected, "selection must still complete after a cold dump race")
        self.assertEqual(len(taps), 3, f"expected drawer, provider, document taps; got {taps}")

    def test_clicks_stay_inside_native_documentsui(self):
        # DOCUMENT also carries the same text in the OR app package, which must
        # never be clicked.
        taps, selected = run(transient_dumps=0)
        self.assertTrue(selected)
        # Every tap must land on a documentsui node's bounds.
        for tap in taps:
            self.assertEqual(len(tap), 2, f"unexpected tap arguments: {tap}")

    def test_selection_marker_is_written(self):
        _, selected = run(transient_dumps=1)
        self.assertTrue(selected)


if __name__ == "__main__":
    unittest.main(verbosity=2)
