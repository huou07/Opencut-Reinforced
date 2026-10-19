#!/usr/bin/env python3
"""Guards the JNI bridge between the Kotlin plugin, its C++ layer and the Rust crate.

A native method is declared in three places and implemented in three languages, so a
method can be declared and never implemented. That is exactly what happened: the plugin
declared `nativeUnregisterMediaFd`, the Rust crate exported `or_media_unregister_seekable_fd`,
and the C++ layer between them resolved neither -- so the app died with
`UnsatisfiedLinkError` the first time a preview source set changed.

This check needs no device, no emulator and no NDK. It reads the three files and fails if
any name in the chain has no counterpart:

  Kotlin `private external fun nativeX`  <->  C++ `Java_..._nativeX`
  C++ `dlsym(bridge, "or_x")`            <->  Rust `pub extern "C" fn or_x`
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
PLUGIN = (
    ROOT
    / "packages/or_viewer_texture/android/src/main/kotlin/dev/opencut/viewertexture"
    / "OrViewerTexturePlugin.kt"
)
JNI = ROOT / "packages/or_viewer_texture/android/src/main/cpp/viewer_texture_jni.cpp"
BRIDGE = ROOT / "crates/or_app_bridge/src"

problems: list[str] = []


def require(condition: bool, message: str) -> None:
    if not condition:
        problems.append(message)


def rust_exported_symbols() -> set[str]:
    symbols: set[str] = set()
    for path in BRIDGE.rglob("*.rs"):
        for match in re.finditer(r'pub extern "C" fn (\w+)', path.read_text(encoding="utf-8")):
            symbols.add(match.group(1))
    return symbols


def main() -> int:
    for path in (PLUGIN, JNI):
        if not path.is_file():
            print(f"FAIL: {path} is missing", file=sys.stderr)
            return 1
    plugin = PLUGIN.read_text(encoding="utf-8")
    jni = JNI.read_text(encoding="utf-8")

    declared = set(re.findall(r"private external fun (native\w+)", plugin))
    require(bool(declared), "no native methods were found; the check would pass vacuously")

    # Every Kotlin declaration must have a JNI wrapper of the matching name.
    for name in sorted(declared):
        require(
            f"Java_dev_opencut_viewertexture_OrViewerTexturePlugin_{name}(" in jni,
            f"{name} is declared in Kotlin but has no Java_..._{name} wrapper in the JNI layer",
        )

    # Every resolved bridge entry point must exist as an exported Rust function.
    exported = rust_exported_symbols()
    require(bool(exported), "no exported Rust entry points were found")
    resolved = set(re.findall(r'dlsym\(bridge, "(or_\w+)"\)', jni))
    require(bool(resolved), "the JNI layer resolves no bridge entry points")
    for symbol in sorted(resolved):
        require(
            symbol in exported,
            f'the JNI layer resolves "{symbol}" but no pub extern "C" fn {symbol} exists',
        )

    # The reverse, scoped to the media registry: every or_media_* entry point must be
    # reachable from the JNI layer, because that is the surface the plugin calls. The two
    # or_viewer_* symbols below are exported and referenced nowhere -- a separate finding,
    # recorded rather than deleted here, because an FFI export is a public boundary and
    # removing one deserves its own decision.
    dead_viewer_exports = {
        "or_viewer_advance_generation",
        "or_viewer_publish_rgba",
    }
    for symbol in sorted(exported):
        if symbol.startswith("or_media_"):
            require(
                f'"{symbol}"' in jni,
                f"{symbol} is exported by Rust but the JNI layer never resolves it",
            )
    for symbol in sorted(dead_viewer_exports):
        require(
            f'"{symbol}"' not in jni,
            f"{symbol} is now resolved; drop it from dead_viewer_exports",
        )

    if problems:
        for problem in problems:
            print(f"FAIL: {problem}", file=sys.stderr)
        return 1
    print(
        "Android JNI bridge: PASS "
        f"({len(declared)} native methods, {len(resolved)} resolved entry points)"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
