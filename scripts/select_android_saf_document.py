#!/usr/bin/env python3
"""Select the acceptance document through Android's real DocumentsUI."""
import argparse
from pathlib import Path
import re
import subprocess
import time
import xml.etree.ElementTree as ET


def center(bounds):
    values = [int(value) for value in re.findall(r"\d+", bounds)]
    if len(values) != 4:
        raise ValueError(f"Invalid Android UI bounds: {bounds}")
    return [(values[0] + values[2]) // 2, (values[1] + values[3]) // 2]


def select(device, guest_log, output):
    output.mkdir(parents=True, exist_ok=True)
    # Building/installing belongs to the owning drive/workflow lifecycle. The
    # action bound starts only when the in-app test reaches its native picker.
    while "ANDROID_SAF_DOCUMENTS_UI_READY" not in guest_log.read_text(errors="replace"):
        time.sleep(0.25)
    deadline = time.monotonic() + 90
    opened_roots = selected_root = False

    def adb(*args):
        return subprocess.check_output(["adb", "-s", device, *args], timeout=10)

    while time.monotonic() < deadline:
        adb("shell", "uiautomator", "dump", "/sdcard/or-saf-window.xml")
        raw = adb("shell", "cat", "/sdcard/or-saf-window.xml")
        (output / "documents-ui-last.xml").write_bytes(raw)
        root = ET.fromstring(raw)
        nodes = list(root.iter("node"))
        # Restrict clicks to the native system picker, never Flutter widgets.
        nodes = [node for node in nodes if node.get("package", "").endswith(".documentsui")]
        document = next((node for node in nodes if node.get("text") == "acceptance.orproj"), None)
        provider = next((node for node in nodes if node.get("text") == "OR SAF acceptance"), None)
        drawer = next((node for node in nodes if node.get("content-desc") in
                       ("Show roots", "Show navigation drawer", "Open navigation drawer")), None)
        target = None
        if selected_root and document is not None:
            target = document
        elif not selected_root and provider is not None:
            target = provider
            selected_root = True
        elif not opened_roots and drawer is not None:
            target = drawer
            opened_roots = True
        if target is not None:
            adb("shell", "input", "tap", *map(str, center(target.get("bounds", ""))))
            if target is document:
                (output / "documents-ui-selection.txt").write_text(
                    "Selected OR SAF acceptance / acceptance.orproj in native DocumentsUI.\n")
                return
        time.sleep(0.25)
    (output / "documents-ui-failure.png").write_bytes(adb("exec-out", "screencap", "-p"))
    raise RuntimeError("The native DocumentsUI selection did not complete within the action bound")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--device", required=True)
    parser.add_argument("--guest-log", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    select(args.device, args.guest_log, args.output)
