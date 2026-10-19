#!/usr/bin/env python3
"""Guards the invariant that a preview request cannot cancel a source registration.

The Android plugin invalidates in-flight work with an epoch counter. One counter served
preview presentation, source registration and clearing at once, so *any* newer request
cancelled *any* in-flight one: a preview for a clip the user had just imported cancelled
the registration of the next selected file, and the import silently skipped it. The SAF
journey selected three documents and imported two, then one of two folder children.

The fix is one counter per kind of state. This check reads the plugin source and fails if
the two kinds are ever merged again, or if the registration path consults the preview
counter. It needs no device, no emulator and no Kotlin toolchain, which is the point: the
behaviour it protects can only be exercised on a device, and this at least makes the
regression shape visible in the source gate.
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

problems: list[str] = []


def require(condition: bool, message: str) -> None:
    if not condition:
        problems.append(message)


def main() -> int:
    if not PLUGIN.is_file():
        print(f"FAIL: {PLUGIN} is missing", file=sys.stderr)
        return 1
    source = PLUGIN.read_text(encoding="utf-8")

    require(
        "private val previewEpoch = AtomicLong(0)" in source,
        "the preview epoch counter is gone; preview and registration must not share one",
    )
    require(
        "private val registrationEpoch = AtomicLong(0)" in source,
        "the registration epoch counter is gone; a preview would cancel a registration",
    )
    require(
        not re.search(r"\bbindingEpoch\b", source),
        "a single shared bindingEpoch is back; that is the defect this check exists for",
    )

    def body(name: str) -> str:
        start = source.find(f"private fun {name}(")
        if start < 0:
            return ""
        # The next top-level `private fun` or the closing of the class bounds the body.
        end = source.find("\n    private fun ", start + 1)
        if end < 0:
            end = len(source)
        return source[start:end]

    registration = body("registerMediaSourceForImport")
    require(bool(registration), "registerMediaSourceForImport was not found")
    require(
        "registrationEpoch.get() != epoch" in registration,
        "registration no longer checks the registration epoch",
    )
    require(
        "previewEpoch" not in registration,
        "registration consults the preview epoch, so a preview can cancel it again",
    )

    preview = body("setMediaSources")
    require(bool(preview), "setMediaSources was not found")
    require(
        "previewEpoch.get() != epoch" in preview,
        "the preview path no longer checks the preview epoch",
    )
    require(
        "registrationEpoch" not in preview,
        "the preview path consults the registration epoch",
    )

    # A preview must not remove the descriptors an import registered. Clearing the whole
    # registry from the preview path is what made the second of three imports fail with
    # SOURCE_UNAVAILABLE: the descriptor was gone by the time Rust opened it.
    require(
        "importRegisteredSources" in source,
        "the import-registered source set is gone; a preview clear would drop import descriptors",
    )
    require(
        "private fun clearPreviewSources()" in source,
        "clearPreviewSources is gone; the preview path has no way to clear only its own sources",
    )
    require(
        "nativeUnregisterMediaFd" in source,
        "the per-source native removal is gone; only a whole-registry clear remains",
    )
    require(
        "clearPreviewSources()" in preview,
        "the preview path no longer clears only its own sources",
    )
    require(
        "clearNativeSources()" not in preview,
        "the preview path clears every source, which drops import descriptors again",
    )
    require(
        "nativeUnregisterMediaFd(source)" in registration,
        "a failed registration no longer removes only the source it was registering",
    )
    require(
        "clearNativeSources()" not in registration,
        "a failed registration clears every source, which drops other imports' descriptors",
    )
    # The whole-registry clear stays for the two cases that really mean everything:
    # the clear method itself and the helper's definition.
    require(
        source.count("clearNativeSources()") == 2,
        "clearNativeSources must remain only its definition and the explicit clear method",
    )

    # A clear removes every source, so it must invalidate both kinds of work.
    clear = body("clearMediaSources")
    require(bool(clear), "clearMediaSources was not found")
    require(
        "previewEpoch.incrementAndGet()" in clear
        and "registrationEpoch.incrementAndGet()" in clear,
        "clearing media sources must invalidate both preview and registration work",
    )

    # Teardown invalidates everything.
    detach = source[source.find("override fun onDetachedFromEngine") :]
    require(
        "previewEpoch.incrementAndGet()" in detach
        and "registrationEpoch.incrementAndGet()" in detach,
        "detaching the plugin must invalidate both preview and registration work",
    )

    if problems:
        for problem in problems:
            print(f"FAIL: {problem}", file=sys.stderr)
        return 1
    print("Android registration epoch: PASS (preview and registration are separate)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
