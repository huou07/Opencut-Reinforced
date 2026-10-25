
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
