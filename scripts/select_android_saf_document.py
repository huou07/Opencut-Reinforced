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


def select(device, guest_log, output, flow="open"):
    if flow == "both":
        select(device, guest_log, output, "open")
        select(device, guest_log, output, "export")
        select(device, guest_log, output, "media")
        select(device, guest_log, output, "caption-import")
        select(device, guest_log, output, "caption-export")
        return
    if flow == "both-relink":
        select(device, guest_log, output, "both")
        select(device, guest_log, output, "media-relink")
        return
    output.mkdir(parents=True, exist_ok=True)
    # Building/installing belongs to the owning drive/workflow lifecycle. The
    # action bound starts only when the in-app test reaches its native picker.
    marker = {
        "open": "ANDROID_SAF_DOCUMENTS_UI_READY",
        "media": "ANDROID_SAF_MEDIA_IMPORT_DOCUMENTS_UI_READY",
        "media-relink": "ANDROID_SAF_MEDIA_RELINK_DOCUMENTS_UI_READY",
        "export": "ANDROID_SAF_EXPORT_DOCUMENTS_UI_READY",
        "caption-import": "ANDROID_SAF_CAPTION_IMPORT_DOCUMENTS_UI_READY",
        "caption-export": "ANDROID_SAF_CAPTION_EXPORT_DOCUMENTS_UI_READY",
    }[flow]
    while marker not in guest_log.read_text(errors="replace"):
        time.sleep(0.25)
    # Bounded, but generous: this guest is software-rendered and its first
    # DocumentsUI frame can take tens of seconds. The bound exists to fail
    # instead of hanging, not to race a cold window.
    deadline = time.monotonic() + 300
    opened_roots = selected_root = False
    list_view_requested = False
    last_media_action = None
    last_media_action_at = 0.0
    last_reported_selection = None
    selector_log = output / "documents-ui-selector.log"

    def complete_relink_if_ready():
        if flow != "media-relink" or "ANDROID_SAF_MEDIA_RELINK_COMPLETE" not in guest_log.read_text(errors="replace"):
            return False
        with (output / "documents-ui-selection.txt").open("a", encoding="utf-8") as record:
            record.write("Selected tiny-second.mkv for media-relink through native DocumentsUI.\n")
        return True

    def adb(*args):
        return subprocess.check_output(["adb", "-s", device, *args], timeout=10)

    while time.monotonic() < deadline:
        if complete_relink_if_ready():
            return
        # UIAutomator transiently answers "null root node returned by
        # UiTestAutomationBridge" while DocumentsUI is still animating in, and
        # `adb shell cat` fails until the dump file exists. Those are ordinary
        # startup races inside this bounded wait, not failures of the journey:
        # treating them as fatal killed the helper and left the drive waiting on
        # a selection that could never arrive.
        try:
            adb("shell", "uiautomator", "dump", "/sdcard/or-saf-window.xml")
            raw = adb("shell", "cat", "/sdcard/or-saf-window.xml")
            root = ET.fromstring(raw)
        except (subprocess.CalledProcessError, subprocess.TimeoutExpired, ET.ParseError):
            time.sleep(0.25)
            continue
        (output / "documents-ui-last.xml").write_bytes(raw)
        nodes = list(root.iter("node"))
        # Restrict clicks to the native system picker, never Flutter widgets.
        nodes = [node for node in nodes if node.get("package", "").endswith(".documentsui")]
        document = next((node for node in nodes if node.get("text") == "acceptance.orproj"), None)
        caption_file = next((node for node in nodes if node.get("text") == "captions.srt"), None)
        media = {
            name: next((node for node in nodes if node.get("text") == name), None)
            for name in ("tiny.mkv", "tiny-second.mkv")
        }
        selected_media = {
            name
            for name, node in media.items()
            if node is not None and node.get("selected") == "true"
        }
        if flow in {"media", "media-relink"} and selected_media != last_reported_selection:
            with selector_log.open("a", encoding="utf-8") as record:
                record.write(f"selected={','.join(sorted(selected_media)) or 'none'}\n")
            last_reported_selection = selected_media
        drawer_roots = next(
            (node for node in nodes if node.get("resource-id", "").endswith(":id/drawer_roots")),
            None,
        )
        if drawer_roots is None:
            provider = next((node for node in nodes if node.get("text") == "OR SAF acceptance"), None)
        else:
            # Recent can expose the provider tile behind an open navigation
            # drawer. Prefer the clickable root in the visible drawer; tapping
            # the obscured tile does nothing but would otherwise advance state.
            provider = next(
                (node for node in drawer_roots.iter("node") if node.get("text") == "OR SAF acceptance"),
                None,
            )
        save = next(
            (
                node
                for node in nodes
                if (node.get("text") or "").casefold() == "save"
                or (node.get("content-desc") or "").casefold() == "save"
            ),
            None,
        )
        if save is not None and save.get("enabled") != "true":
            save = None
        open_action = next(
            (
                node
                for node in nodes
                if (node.get("text") or "").casefold() == "open"
                or (node.get("content-desc") or "").casefold() == "open"
            ),
            None,
        )
        if open_action is not None and open_action.get("enabled") != "true":
            open_action = None
        media_action = next(
            (
                node
                for node in nodes
                if (node.get("text") or "").casefold() in {"open", "select"}
                or (node.get("content-desc") or "").casefold() in {"open", "select"}
            ),
            None,
        )
        if media_action is not None and media_action.get("enabled") != "true":
            media_action = None
        list_view = next(
            (node for node in nodes if node.get("content-desc") == "List view"),
            None,
        )
        drawer = next((node for node in nodes if node.get("content-desc") in
                       ("Show roots", "Show navigation drawer", "Open navigation drawer")), None)
        target = None
        pending_media = None
        if selected_root and flow == "open" and document is not None:
            target = document
        elif selected_root and flow == "caption-import" and caption_file is not None:
            target = caption_file
        elif selected_root and flow in {"media", "media-relink"}:
            available_media = {name for name, node in media.items() if node is not None}
            expected_media = set(media) if flow == "media" else {"tiny-second.mkv"}
            next_media = next(
                (
                    name
                    for name in media
                    if name in expected_media
                    and name not in selected_media
                    and media[name] is not None
                ),
                None,
            )
            # DocumentsUI can briefly expose only part of its provider listing
            # while loading/relayout is in progress. Do not infer that a
            # partial set is the whole acceptance selection; the known fixture
            # names are the completion contract for this journey.
            listing_complete = available_media == set(media)
            if not listing_complete and list_view is not None and not list_view_requested:
                # The acceptance provider can place one tile below the fold in
                # DocumentsUI's default grid at compact emulator sizes. List
                # view exposes the actual filename nodes without guessing at
                # card coordinates or scrolling.
                target = list_view
            elif listing_complete and next_media is not None:
                action = "long-press" if flow == "media" and not selected_media else "tap"
                retry_wait = time.monotonic() - last_media_action_at < 1.5
                if last_media_action != (next_media, action) or not retry_wait:
                    target = media[next_media]
                    pending_media = next_media
            elif (
                listing_complete
                and expected_media.issubset(selected_media)
                and media_action is not None
            ):
                target = media_action
        elif selected_root and flow in {"export", "caption-export"} and save is not None:
            target = save
        elif not selected_root and provider is not None:
            target = provider
            selected_root = True
        elif not opened_roots and drawer is not None:
            target = drawer
            opened_roots = True
        if target is not None:
            x, y = center(target.get("bounds", ""))
            try:
                if pending_media is not None and flow == "media" and not selected_media:
                    # DocumentsUI opens a file on a normal first tap. Long-press
                    # the first item to enter multi-select mode.
                    adb("shell", "input", "swipe", str(x), str(y), str(x), str(y), "800")
                else:
                    adb("shell", "input", "tap", str(x), str(y))
            except (subprocess.CalledProcessError, subprocess.TimeoutExpired):
                # The window can move between the dump and the tap. Re-dump and
                # decide again from the fresh tree.
                continue
            if pending_media is not None:
                action = (
                    "long-press"
                    if flow == "media" and not selected_media
                    else "tap"
                )
                last_media_action = (pending_media, action)
                last_media_action_at = time.monotonic()
                with selector_log.open("a", encoding="utf-8") as record:
                    record.write(f"action={action} target={pending_media}\n")
            if target is list_view:
                list_view_requested = True
                with selector_log.open("a", encoding="utf-8") as record:
                    record.write("action=tap target=list-view\n")
            if (
                target is document
                or target is caption_file
                or target is save
                or (flow == "media" and target is media_action)
            ):
                with (output / "documents-ui-selection.txt").open("a", encoding="utf-8") as record:
                    detail = (
                        ",".join(sorted(selected_media)) if flow == "media" else
                        "tiny-second.mkv" if flow == "media-relink" else
                        "caption file" if flow == "caption-import" else
                        "caption export" if flow == "caption-export" else
                        "OR SAF acceptance"
                    )
                    record.write(f"Selected {detail} for {flow} through native DocumentsUI.\n")
                return
        time.sleep(0.25)
    try:
        (output / "documents-ui-failure.png").write_bytes(adb("exec-out", "screencap", "-p"))
    except (subprocess.CalledProcessError, subprocess.TimeoutExpired):
        pass
    raise RuntimeError("The native DocumentsUI selection did not complete within the action bound")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--device", required=True)
    parser.add_argument("--guest-log", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--flow", choices=("open", "media", "media-relink", "export", "caption-import", "caption-export", "both", "both-relink"), default="open")
    args = parser.parse_args()
    select(args.device, args.guest_log, args.output, args.flow)
