#!/usr/bin/env python3
"""Print the FFmpeg configure-argument groups the workflow declares.

The three groups live in `.github/workflows/platform-verification.yml` as job
environment variables so a hosted build has exactly one definition of what a
supported build enables. A real host has no job environment, so this reader
emits shell assignments for it instead of a second, drifting copy of the flags:

    eval "$(python3 scripts/ffmpeg_configure_args.py .github/workflows/...yml)"

Only the folded (`>-`) block form is handled, which is what the workflow uses.
"""

from __future__ import annotations

import sys

GROUPS = (
    "FFMPEG_COMMON_CONFIGURE_ARGS",
    "FFMPEG_HELPER_CONFIGURE_ARGS",
    "FFMPEG_EXPORT_CONFIGURE_ARGS",
)


def read_groups(path: str) -> dict[str, str]:
    lines = open(path, encoding="utf-8").read().splitlines()
    groups: dict[str, str] = {}
    index = 0
    while index < len(lines):
        line = lines[index]
        stripped = line.strip()
        name = stripped.split(":", 1)[0] if ":" in stripped else ""
        if name not in GROUPS:
            index += 1
            continue
        value = stripped.split(":", 1)[1].strip()
        if value in (">-", ">", "|", "|-"):
            body = []
            cursor = index + 1
            while cursor < len(lines):
                candidate = lines[cursor]
                if not candidate.strip():
                    cursor += 1
                    continue
                if len(candidate) - len(candidate.lstrip(" ")) < 4:
                    break
                body.append(candidate.strip())
                cursor += 1
            value = " ".join(part for part in body if part)
            index = cursor
        groups[name] = value
        if len(groups) == len(GROUPS):
            break
    missing = [name for name in GROUPS if name not in groups]
    if missing:
        raise SystemExit(f"{path}: missing {', '.join(missing)}")
    return groups


def main(argv: list[str]) -> int:
    if len(argv) != 2:
        print(__doc__.strip(), file=sys.stderr)
        return 2
    for name, value in read_groups(argv[1]).items():
        quoted = value.replace('"', '\\"')
        print(f'export {name}="{quoted}"')
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
