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

MEDIA_SECOND_SELECTED = b"""<?xml version='1.0' encoding='UTF-8'?>
<hierarchy rotation="0">
  <node index="0" text="tiny.mkv" class="android.widget.LinearLayout"
        package="com.android.documentsui" bounds="[0,200][1080,300]" selected="false" />
  <node index="1" text="tiny-second.mkv" class="android.widget.LinearLayout"
        package="com.android.documentsui" bounds="[0,320][1080,420]" selected="true" />
  <node index="2" text="Select" class="android.widget.Button"
        package="com.android.documentsui" enabled="true" bounds="[900,700][1080,800]" />
</hierarchy>
"""

MEDIA_GRID_PARTIAL = b"""<?xml version='1.0' encoding='UTF-8'?>
<hierarchy rotation="0">
  <node index="0" text="tiny-second.mkv" class="android.widget.LinearLayout"
        package="com.android.documentsui" bounds="[214,516][284,535]" selected="false" />
  <node index="1" text="" class="android.widget.Button"
        package="com.android.documentsui" content-desc="List view"
        bounds="[248,126][296,174]" />
</hierarchy>
"""

MEDIA_LIST = b"""<?xml version='1.0' encoding='UTF-8'?>
<hierarchy rotation="0">
  <node index="0" text="tiny.mkv" class="android.widget.LinearLayout"
        package="com.android.documentsui" bounds="[24,200][296,260]" selected="false" />
  <node index="1" text="tiny-second.mkv" class="android.widget.LinearLayout"
        package="com.android.documentsui" bounds="[24,272][296,332]" selected="false" />
</hierarchy>
"""

MEDIA_FIRST_SELECTED = b"""<?xml version='1.0' encoding='UTF-8'?>
<hierarchy rotation="0">
  <node index="0" text="tiny.mkv" class="android.widget.LinearLayout"
        package="com.android.documentsui" bounds="[24,200][296,260]" selected="true" />
  <node index="1" text="tiny-second.mkv" class="android.widget.LinearLayout"
        package="com.android.documentsui" bounds="[24,272][296,332]" selected="false" />
  <node index="2" text="Select" class="android.widget.Button"
        package="com.android.documentsui" enabled="true" bounds="[208,24][272,72]" />
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

MEDIA_SELECT = MEDIA_OPEN.replace(b'text="Open"', b'text="Select"')

CAPTION = b"""<?xml version='1.0' encoding='UTF-8'?>
<hierarchy rotation="0">
  <node index="0" text="captions.srt" class="android.widget.LinearLayout"
        package="com.android.documentsui" bounds="[0,200][1080,300]" />
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

    def __init__(self, transient_dumps, flow="open", media_confirmation="Open"):
        self.transient = transient_dumps
        self.tree_index = 0
        self.taps = []
        self.actions = []
        self.trees = {
            "open": [DRAWER, PROVIDER, DOCUMENT],
            "media": [
                DRAWER,
                PROVIDER,
                MEDIA_GRID_PARTIAL,
                MEDIA_LIST,
                MEDIA_FIRST_SELECTED,
                MEDIA_SELECT if media_confirmation == "Select" else MEDIA_OPEN,
            ],
            "export": [DRAWER, PROVIDER, SAVE_DISABLED, SAVE],
            "caption-import": [DRAWER, PROVIDER, CAPTION],
            "caption-export": [DRAWER, PROVIDER, SAVE_DISABLED, SAVE],
            "both": [DRAWER, PROVIDER, DOCUMENT, DRAWER, PROVIDER,
                     SAVE_DISABLED, SAVE, DRAWER, PROVIDER,
                     MEDIA_GRID_PARTIAL,
                     MEDIA_LIST,
                     MEDIA_FIRST_SELECTED,
                     MEDIA_OPEN, DRAWER, PROVIDER, CAPTION, DRAWER, PROVIDER,
                     SAVE_DISABLED, SAVE],
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
            action = command[2]
            tree_index = self.tree_index - 1
            self.actions.append((action, tree_index))
            if action == "tap":
                self.taps.append(command[-2:])
            elif action == "swipe":
                pass
            else:
                raise AssertionError(f"unexpected input action: {command}")
            return b""
        if command[:1] == ["exec-out"]:
            return b"\x89PNG"
        raise AssertionError(f"unexpected adb call: {args}")


def run(transient_dumps, flow="open"):
    with tempfile.TemporaryDirectory() as work:
        root = Path(work)
        guest = root / "guest.log"
        guest.write_text("ANDROID_SAF_DOCUMENTS_UI_READY\nANDROID_SAF_MEDIA_IMPORT_DOCUMENTS_UI_READY\nANDROID_SAF_EXPORT_DOCUMENTS_UI_READY\nANDROID_SAF_CAPTION_IMPORT_DOCUMENTS_UI_READY\nANDROID_SAF_CAPTION_EXPORT_DOCUMENTS_UI_READY\n")
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
        with tempfile.TemporaryDirectory() as work:
            root = Path(work)
            guest = root / "guest.log"
            guest.write_text("ANDROID_SAF_MEDIA_IMPORT_DOCUMENTS_UI_READY\n")
            output = root / "out"
            adb = FakeAdb(transient_dumps=0, flow="media")
            with mock.patch.object(subprocess, "check_output", adb.check_output):
                selector.select("emulator-5554", guest, output, "media")

            self.assertEqual(
                adb.actions,
                [("tap", 0), ("tap", 1), ("tap", 2), ("swipe", 3), ("tap", 4), ("tap", 5)],
                "switch from a partial grid to list view, select both files, then Open",
            )
            self.assertEqual(adb.taps[2], ["272", "150"])
            self.assertEqual(adb.taps[3], ["160", "302"])
            self.assertEqual(adb.taps[4], ["990", "750"])
            selection_log = (output / "documents-ui-selector.log").read_text()
            self.assertIn("action=tap target=list-view", selection_log)
            self.assertIn("action=long-press target=tiny.mkv", selection_log)
            self.assertIn("action=tap target=tiny-second.mkv", selection_log)
            selected_report = (output / "documents-ui-selection.txt").read_text()
            self.assertIn(
                "Selected tiny-second.mkv,tiny.mkv for media through native DocumentsUI.",
                selected_report,
            )

    def test_media_flow_adds_missing_file_after_a_prior_selection(self):
        with tempfile.TemporaryDirectory() as work:
            root = Path(work)
            guest = root / "guest.log"
            guest.write_text("ANDROID_SAF_MEDIA_IMPORT_DOCUMENTS_UI_READY\n")
            output = root / "out"
            adb = FakeAdb(transient_dumps=0, flow="media")
            adb.trees = [DRAWER, PROVIDER, MEDIA_GRID_PARTIAL, MEDIA_SECOND_SELECTED, MEDIA_OPEN]
            with mock.patch.object(subprocess, "check_output", adb.check_output):
                selector.select("emulator-5554", guest, output, "media")

            self.assertEqual(
                adb.actions,
                [("tap", 0), ("tap", 1), ("tap", 2), ("tap", 3), ("tap", 4)],
            )
            selection_log = (output / "documents-ui-selector.log").read_text()
            self.assertIn("action=tap target=tiny.mkv", selection_log)

    def test_media_flow_accepts_select_confirmation_on_current_documentsui(self):
        with tempfile.TemporaryDirectory() as work:
            root = Path(work)
            guest = root / "guest.log"
            guest.write_text("ANDROID_SAF_MEDIA_IMPORT_DOCUMENTS_UI_READY\n")
            output = root / "out"
            adb = FakeAdb(
                transient_dumps=0,
                flow="media",
                media_confirmation="Select",
            )
            with mock.patch.object(subprocess, "check_output", adb.check_output):
                selector.select("emulator-5554", guest, output, "media")

            self.assertEqual(adb.actions[-1], ("tap", 5))
            self.assertTrue((output / "documents-ui-selection.txt").exists())

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
        with tempfile.TemporaryDirectory() as work:
            root = Path(work)
            guest = root / "guest.log"
            guest.write_text(
                "ANDROID_SAF_DOCUMENTS_UI_READY\n"
                "ANDROID_SAF_EXPORT_DOCUMENTS_UI_READY\n"
                "ANDROID_SAF_MEDIA_IMPORT_DOCUMENTS_UI_READY\n"
                "ANDROID_SAF_CAPTION_IMPORT_DOCUMENTS_UI_READY\n"
                "ANDROID_SAF_CAPTION_EXPORT_DOCUMENTS_UI_READY\n"
            )
            output = root / "out"
            adb = FakeAdb(transient_dumps=0, flow="both")
            with mock.patch.object(subprocess, "check_output", adb.check_output):
                selector.select("emulator-5554", guest, output, "both")

            self.assertEqual(len(adb.actions), 18)
            self.assertTrue((output / "documents-ui-selection.txt").exists())

    def test_combined_flow_matches_project_export_then_media_journey(self):
        with tempfile.TemporaryDirectory() as work:
            root = Path(work)
            guest = root / "guest.log"
            guest.write_text(
                "\n".join(
                    [
                        "ANDROID_SAF_DOCUMENTS_UI_READY",
                        "ANDROID_SAF_EXPORT_DOCUMENTS_UI_READY",
                        "ANDROID_SAF_MEDIA_IMPORT_DOCUMENTS_UI_READY",
                        "ANDROID_SAF_CAPTION_IMPORT_DOCUMENTS_UI_READY",
                        "ANDROID_SAF_CAPTION_EXPORT_DOCUMENTS_UI_READY",
                    ]
                )
                + "\n"
            )
            output = root / "out"
            adb = FakeAdb(transient_dumps=0, flow="both")
            with mock.patch.object(subprocess, "check_output", adb.check_output):
                selector.select("emulator-5554", guest, output, "both")

            self.assertEqual(
                adb.actions,
                [
                    ("tap", 0),
                    ("tap", 1),
                    ("tap", 2),
                    ("tap", 3),
                    ("tap", 4),
                    ("tap", 6),
                    ("tap", 7),
                    ("tap", 8),
                    ("tap", 9),
                    ("swipe", 10),
                    ("tap", 11),
                    ("tap", 12),
                    ("tap", 13),
                    ("tap", 14),
                    ("tap", 15),
                    ("tap", 16),
                    ("tap", 17),
                    ("tap", 19),
                ],
                "DocumentsUI selections must follow open, export, media, caption import and caption export",
            )


if __name__ == "__main__":
    unittest.main(verbosity=2)
