#!/usr/bin/env python3
"""Behavioural tests for the native DocumentsUI selection helper.

The helper drives real Android DocumentsUI, so these tests stub `adb` and prove
the picker stays within DocumentsUI and selects only after its ready marker.

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
  <node index="0" text="" class="android.widget.FrameLayout"
        package="com.google.android.documentsui" resource-id="com.google.android.documentsui:id/drawer_layout">
    <node index="0" text="" class="android.widget.LinearLayout"
          package="com.google.android.documentsui" resource-id="com.google.android.documentsui:id/apps_row">
      <node index="0" text="OR SAF acceptance" class="android.widget.TextView"
            package="com.google.android.documentsui" resource-id="android:id/title"
            bounds="[107,216][212,233]" />
    </node>
    <node index="1" text="" class="android.widget.LinearLayout"
          package="com.google.android.documentsui" resource-id="com.google.android.documentsui:id/drawer_roots">
      <node index="0" text="OR SAF acceptance" class="android.widget.TextView"
            package="com.google.android.documentsui" resource-id="android:id/title"
            bounds="[64,316][264,335]" />
    </node>
  </node>
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

MEDIA = b"""<?xml version='1.0' encoding='UTF-8'?>
<hierarchy rotation="0">
  <node index="0" text="tiny.mkv" class="android.widget.LinearLayout"
        package="com.android.documentsui" bounds="[0,200][1080,300]" />
  <node index="1" text="tiny-second.mkv" class="android.widget.LinearLayout"
        package="com.android.documentsui" bounds="[0,320][1080,420]" />
</hierarchy>
"""

MEDIA_OPEN = b"""<?xml version='1.0' encoding='UTF-8'?>
<hierarchy rotation="0">
  <node index="0" text="tiny.mkv" class="android.widget.LinearLayout"
        package="com.android.documentsui" bounds="[0,200][1080,300]" selected="true" />
  <node index="1" text="tiny-second.mkv" class="android.widget.LinearLayout"
        package="com.android.documentsui" bounds="[0,320][1080,420]" selected="true" />
  <node index="2" text="Open" class="android.widget.Button"
        package="com.android.documentsui" enabled="true" bounds="[900,700][1080,800]" />
</hierarchy>
"""

SAVE_DISABLED = b"""<?xml version='1.0' encoding='UTF-8'?>
<hierarchy rotation="0">
  <node index="0" text="SAVE" class="android.widget.Button"
        package="com.android.documentsui" enabled="false" bounds="[900,700][1080,800]" />
</hierarchy>
"""

SAVE = b"""<?xml version='1.0' encoding='UTF-8'?>
<hierarchy rotation="0">
  <node index="0" text="SAVE" class="android.widget.Button"
        package="com.android.documentsui" enabled="true" bounds="[900,700][1080,800]" />
</hierarchy>
"""


class FakeAdb:
    """An `adb` that fails like a cold guest, then serves the real tree."""

    def __init__(self, transient_dumps, flow="open"):
        self.transient = transient_dumps
        self.tree_index = 0
        self.taps = []
        self.trees = {
            "open": [DRAWER, PROVIDER, DOCUMENT],
            "media": [DRAWER, PROVIDER, MEDIA, MEDIA, MEDIA_OPEN],
            "export": [DRAWER, PROVIDER, SAVE_DISABLED, SAVE],
            "both": [DRAWER, PROVIDER, DOCUMENT, DRAWER, PROVIDER, MEDIA, MEDIA,
                     MEDIA_OPEN,
                     DRAWER, PROVIDER, SAVE_DISABLED, SAVE],
        }[flow]

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


def run(transient_dumps, flow="open"):
    with tempfile.TemporaryDirectory() as work:
        root = Path(work)
        guest = root / "guest.log"
        guest.write_text("ANDROID_SAF_DOCUMENTS_UI_READY\nANDROID_SAF_MEDIA_IMPORT_DOCUMENTS_UI_READY\nANDROID_SAF_EXPORT_DOCUMENTS_UI_READY\n")
        output = root / "out"
        adb = FakeAdb(transient_dumps, flow)
        with mock.patch.object(subprocess, "check_output", adb.check_output):
            selector.select("emulator-5554", guest, output, flow)
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

    def test_media_flow_selects_the_fixture_video(self):
        taps, selected = run(transient_dumps=0, flow="media")
        self.assertTrue(selected)
        self.assertEqual(len(taps), 5)
        self.assertEqual(taps[2], ["540", "250"])
        self.assertEqual(taps[3], ["540", "370"])
        self.assertEqual(taps[4], ["990", "750"])

    def test_open_drawer_prefers_provider_root_over_obscured_recent_tile(self):
        taps, selected = run(transient_dumps=0)
        self.assertTrue(selected)
        self.assertEqual(taps[1], ["164", "325"])

    def test_selection_marker_is_written(self):
        _, selected = run(transient_dumps=1)
        self.assertTrue(selected)

    def test_export_uses_the_native_picker_and_save_action(self):
        taps, selected = run(transient_dumps=1, flow="export")
        self.assertTrue(selected)
        self.assertEqual(len(taps), 3, f"expected drawer, provider, Save taps; got {taps}")

    def test_disabled_save_action_is_never_tapped_or_reported_as_selected(self):
        taps, selected = run(transient_dumps=0, flow="export")
        self.assertTrue(selected)
        self.assertEqual(len(taps), 3, f"expected disabled Save to be skipped; got {taps}")

    def test_all_saf_flows_use_documentsui(self):
        taps, selected = run(transient_dumps=0, flow="both")
        self.assertTrue(selected)
        self.assertEqual(len(taps), 11, f"expected open, multi-media and export picker flows; got {taps}")


if __name__ == "__main__":
    unittest.main(verbosity=2)
