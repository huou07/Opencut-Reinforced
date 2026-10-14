#!/usr/bin/env python3
"""Enforce the `or_core` dependency boundary.

`or_core` holds canonical project state, the command/application path and the
time model. It must stay free of GPU, media, audio, AI-runtime and platform
bindings, so that those layers can be replaced without touching the thing that
owns user data. That is the rule from ARCHITECTURE_INVARIANTS.md, and this
script is the automated form of it.

This checker replaces `scripts/archive/check_architecture_policy.py`, which
enforced the same rule as a side effect of validating the paused execution
plan. It is deliberately much smaller: one rule, one manifest, no plan, no
supervisor, no repository state to freeze.

    python3 scripts/check_or_core_deps.py

Exit 0 when the boundary holds, 1 when it does not. Stdlib only, read-only,
no network, no cargo invocation: it reads the manifest, so it is fast and
cannot fail for an unrelated reason.

**Adding a dependency to `or_core` is a gated decision, not a formatting
change.** If this script fails, do not add an exclusion to the pattern list to
make it green. Record an architecture decision (see docs/adr/) that justifies
the dependency, then update this list and ARCHITECTURE_INVARIANTS.md in the
same change. The failure is the point.
"""
from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CORE_MANIFEST = ROOT / "crates" / "or_core" / "Cargo.toml"

# Substring match against the normalised dependency name, so `cosmic-text` and
# `cosmic_text` are one rule, and `ffmpeg-the-third` is caught by `ffmpeg`.
# A pattern is a substring, not a regex: these are names, not expressions.
FORBIDDEN = (
    # GPU / rendering
    "wgpu", "metal", "objc", "objc2", "cuda", "cudarc", "mlx",
    # Media
    "ffmpeg", "cpal", "rodio", "alsa", "coreaudio", "wasapi", "oboe",
    # Text shaping / rasterisation
    "cosmic-text", "cosmic_text", "swash", "fontdb",
    # Scripting engines and web/canvas content
    "wasmi", "browser", "node", "remotion", "motion-canvas", "quick-js",
    # AI runtimes and providers
    "onnxruntime", "whisper", "ort",
)

# `or_core` is allowed exactly one category outside itself: the shared time and
# document model. Anything else is a boundary decision, not a convenience.
ALLOWED_INTERNAL = ("or_time",)


def normalise(name: str) -> str:
    return name.strip().lower().replace("_", "-")


def parse_direct_dependencies(text: str) -> list[str]:
    """Return direct dependency names from the [dependencies] table.

    Deliberately small: it reads the table it needs and nothing else. It is not
    a Cargo manifest parser, and it does not try to be. Cargo writes dependency
    entries flush at column 0, one per line, so the table ends at the next line
    that opens another `[...]` header. Blank lines and comments inside the table
    are skipped rather than treated as its end.
    """
    names: list[str] = []
    in_table = False

    for raw in text.splitlines():
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        if line.startswith("["):
            # Any table header ends the previous table. `[dependencies]` starts
            # ours; `[target.'cfg(...)'.dependencies]` and friends do not, so a
            # platform-specific dependency is deliberately out of scope here.
            in_table = line == "[dependencies]"
            continue
        if not in_table:
            continue

        # `name = { ... }` or `name = "1.0"`. Strip a trailing comment first.
        entry = line.split("#", 1)[0].strip()
        if "=" not in entry:
            continue
        name = entry.split("=", 1)[0].strip().strip('"').strip("'")
        if name:
            names.append(name)

    return names


def main() -> int:
    if not CORE_MANIFEST.is_file():
        print(f"FAIL: or_core manifest is missing: {CORE_MANIFEST}", file=sys.stderr)
        print("      The dependency boundary cannot be checked without it.",
              file=sys.stderr)
        return 1

    try:
        text = CORE_MANIFEST.read_text(encoding="utf-8")
    except OSError as error:
        print(f"FAIL: cannot read {CORE_MANIFEST}: {error}", file=sys.stderr)
        return 1

    if "[dependencies]" not in text:
        print(f"FAIL: {CORE_MANIFEST} has no [dependencies] table. "
              f"A renamed or restructured manifest must be reviewed here "
              f"deliberately, not pass by being unreadable.", file=sys.stderr)
        return 1

    direct = parse_direct_dependencies(text)
    if not direct:
        print(f"FAIL: parsed zero direct dependencies from {CORE_MANIFEST}. "
              f"The reader no longer understands this manifest; fix the reader "
              f"rather than treating an empty result as a clean boundary.",
              file=sys.stderr)
        return 1

    failures: list[str] = []
    for raw in sorted(direct):
        name = normalise(raw)
        if name in ALLOWED_INTERNAL:
            continue
        for pattern in FORBIDDEN:
            if pattern in name:
                failures.append(
                    f"forbidden direct or_core dependency: {raw!r} "
                    f"(matched {pattern!r}) — GPU, media, audio, text-shaping, "
                    f"scripting and AI-runtime layers must not be reachable from "
                    f"the crate that owns canonical project state")
                break

    if failures:
        for failure in failures:
            print(f"FAIL: {failure}", file=sys.stderr)
        print("      Do not extend the pattern list to silence this. Record an "
              "architecture decision in docs/adr/ that justifies the dependency, "
              "then update ARCHITECTURE_INVARIANTS.md in the same change.",
              file=sys.stderr)
        print(f"or_core dependency boundary: FAIL ({len(failures)} violation(s))",
              file=sys.stderr)
        return 1

    print(f"or_core dependency boundary: PASS "
          f"({len(direct)} direct dependencies, none GPU/media/audio/AI-runtime)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
