#!/usr/bin/env python3
"""Drive the packaged macOS app from the command line.

Why this exists: the semantic harness (`scripts/or_ui.py`) runs a *debug* build
through the Flutter test binding, so it cannot prove anything about the shipped
bundle. This driver talks to the packaged app the way a person does -- real
clicks and real keystrokes -- and captures only the app window, never the desktop
(see docs/operations/local-macos.md, "Driving the built app").

Requirements: macOS, `swiftc` (Xcode command line tools), Accessibility permission
for the terminal that runs it, and the packaged app launched with the same bundle
identifier as the running process.

Usage:
    python3 scripts/packaged_drive.py bounds
    python3 scripts/packaged_drive.py shot /tmp/step.png
    python3 scripts/packaged_drive.py click 103 155     # window-relative points
    python3 scripts/packaged_drive.py key 45 cmd        # ⌘N
    python3 scripts/packaged_drive.py paste 'some text' # clipboard + ⌘V

Coordinates are window-relative *points*: the capture and the window share the
same origin, so a point read off a screenshot is a point to click, as long as the
screenshot is not a 2x downscale (see `scale()`).
"""
from __future__ import annotations

import os
import subprocess
import sys
import time

BINARY = os.environ.get('OR_CLICKER', os.path.join(os.path.dirname(__file__), '.packaged_drive_clicker'))
BUNDLE_HINT = os.environ.get('OR_BUNDLE_PROCESS', 'or_app.app/Contents/MacOS/or_app')

SWIFT = r'''
import CoreGraphics
import Foundation

let source = CGEventSource(stateID: .hidSystemState)

func postMouse(_ type: CGEventType, _ p: CGPoint, clicks: Int64) {
    guard let e = CGEvent(mouseEventSource: source, mouseType: type, mouseCursorPosition: p, mouseButton: .left) else { return }
    e.setIntegerValueField(.mouseEventClickState, value: clicks)
    e.post(tap: .cghidEventTap)
}

func click(_ x: Double, _ y: Double) {
    let p = CGPoint(x: x, y: y)
    // An interrupted run can leave the button logically held, which makes the next
    // click a drag. Always release first.
    if let up = CGEvent(mouseEventSource: source, mouseType: .leftMouseUp, mouseCursorPosition: p, mouseButton: .left) {
        up.post(tap: .cghidEventTap)
        usleep(20_000)
    }
    postMouse(.mouseMoved, p, clicks: 0)
    usleep(60_000)
    postMouse(.leftMouseDown, p, clicks: 1)
    usleep(120_000)
    postMouse(.leftMouseUp, p, clicks: 1)
}

func typeText(_ text: String) {
    for ch in text.unicodeScalars {
        var u = UniChar(ch.value)
        guard let down = CGEvent(keyboardEventSource: nil, virtualKey: 0, keyDown: true),
              let up = CGEvent(keyboardEventSource: nil, virtualKey: 0, keyDown: false) else { continue }
        down.keyboardSetUnicodeString(stringLength: 1, unicodeString: &u)
        up.keyboardSetUnicodeString(stringLength: 1, unicodeString: &u)
        down.post(tap: .cghidEventTap)
        up.post(tap: .cghidEventTap)
        usleep(12_000)
    }
}

func key(_ code: CGKeyCode, flags: CGEventFlags) {
    guard let down = CGEvent(keyboardEventSource: nil, virtualKey: code, keyDown: true),
          let up = CGEvent(keyboardEventSource: nil, virtualKey: code, keyDown: false) else { return }
    down.flags = flags
    up.flags = flags
    down.post(tap: .cghidEventTap)
    usleep(30_000)
    up.post(tap: .cghidEventTap)
}

let args = CommandLine.arguments
guard args.count >= 2 else { exit(2) }
switch args[1] {
case "click":
    click(Double(args[2])!, Double(args[3])!)
case "type":
    typeText(args[2])
case "key":
    var flags: CGEventFlags = []
    for extra in args.dropFirst(3) {
        if extra == "cmd" { flags.insert(.maskCommand) }
        if extra == "shift" { flags.insert(.maskShift) }
        if extra == "alt" { flags.insert(.maskAlternate) }
    }
    key(CGKeyCode(args[2])!, flags: flags)
default:
    exit(2)
}
'''


def ensure_clicker() -> str:
    if os.path.exists(BINARY):
        return BINARY
    swift_path = BINARY + '.swift'
    with open(swift_path, 'w') as handle:
        handle.write(SWIFT)
    subprocess.run(['swiftc', '-O', swift_path, '-o', BINARY], check=True)
    return BINARY


def pid() -> str:
    found = subprocess.run(['pgrep', '-f', BUNDLE_HINT], capture_output=True, text=True)
    ids = found.stdout.split()
    if not ids:
        raise SystemExit('the packaged app is not running')
    return ids[0]


def bounds() -> tuple[str, int, int, int, int]:
    owner = pid()
    script = (
        f'tell application "System Events" to tell (first process whose unix id is {owner}) '
        'to get {position, size} of window 1'
    )
    out = subprocess.run(['osascript', '-e', script], capture_output=True, text=True).stdout
    nums = [int(float(value)) for value in out.replace('{', '').replace('}', '').split(',')]
    return (owner, nums[0], nums[1], nums[2], nums[3])


def front() -> None:
    """Raise the app: synthetic clicks land on whatever is on top."""
    owner = pid()
    subprocess.run([
        'osascript', '-e',
        f'tell application "System Events" to tell (first process whose unix id is {owner}) to set frontmost to true',
    ], capture_output=True)


def shot(path: str) -> str:
    _, x, y, w, h = bounds()
    subprocess.run(['screencapture', '-x', '-R', f'{x},{y},{w},{h}', path], check=True)
    return path


def scale(path: str) -> float:
    """Points per captured pixel for a screenshot of the window."""
    out = subprocess.run(['sips', '-g', 'pixelWidth', path], capture_output=True, text=True).stdout
    width = int(out.split('pixelWidth:')[1].split()[0])
    _, _, _, window_width, _ = bounds()
    return window_width / width


def click(x: float, y: float) -> None:
    _, wx, wy, _, _ = bounds()
    subprocess.run([ensure_clicker(), 'click', str(int(wx + x)), str(int(wy + y))], check=True)
    time.sleep(0.6)


def key(code: int, *modifiers: str) -> None:
    subprocess.run([ensure_clicker(), 'key', str(code), *modifiers], check=True)
    time.sleep(0.5)


def paste(text: str) -> None:
    subprocess.run(['pbcopy'], input=text.encode(), check=True)
    key(9, 'cmd')


def main(argv: list[str]) -> int:
    if len(argv) < 2:
        print(__doc__)
        return 2
    command = argv[1]
    if command == 'bounds':
        print(bounds())
    elif command == 'front':
        front()
    elif command == 'shot':
        print(shot(argv[2]))
    elif command == 'scale':
        print(scale(argv[2]))
    elif command == 'click':
        click(float(argv[2]), float(argv[3]))
    elif command == 'key':
        key(int(argv[2]), *argv[3:])
    elif command == 'paste':
        paste(argv[2])
    else:
        print(__doc__)
        return 2
    return 0


if __name__ == '__main__':
    raise SystemExit(main(sys.argv))
