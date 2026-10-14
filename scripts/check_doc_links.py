#!/usr/bin/env bash
# Markdown link check for tracked documentation.
#
# Deterministic, read-only, stdlib-only Python. Relative links are resolved on
# disk; a link that names a file which is not there is a real defect, because
# the documentation index is the routing layer an agent depends on.
#
#   python3 scripts/check_doc_links.py            # check tracked markdown
#   python3 scripts/check_doc_links.py path ...   # check specific files
#
# Exit 0 when every relative link resolves, 1 otherwise. Anchors are not
# verified: this checks that the target file exists, not that the target
# contains the named heading.
from __future__ import annotations

import re
import subprocess
import sys
from pathlib import Path

LINK = re.compile(r"\[[^\]]*\]\(([^)\s]+)(?:\s+\"[^\"]*\")?\)")
SKIP_DIRS = {".git", "build", ".dart_tool", "node_modules", "target",
             "migration-preserved", ".codegraph", ".opencode"}
# Directories whose markdown is archival receipts, not a routing surface.
ARCHIVE_OK = ("docs/archive/",)


def tracked_markdown() -> list[Path]:
    try:
        out = subprocess.run(["git", "ls-files", "-z", "*.md", "*.markdown"],
                             capture_output=True, text=True, check=True).stdout
    except (subprocess.CalledProcessError, FileNotFoundError):
        return []
    return [Path(p) for p in out.split("\0") if p]


def is_skipped(path: Path) -> bool:
    parts = path.parts
    return bool(SKIP_DIRS.intersection(parts))


def main(argv: list[str]) -> int:
    targets = [Path(a) for a in argv[1:]] or tracked_markdown()
    broken: list[tuple[Path, str, str]] = []

    for doc in targets:
        if is_skipped(doc) or not doc.is_file():
            continue
        try:
            text = doc.read_text(encoding="utf-8", errors="ignore")
        except OSError:
            continue
        # Fenced code blocks quote paths as examples; a broken link there is
        # illustrative, not navigational.
        lines, fenced, buf = text.splitlines(), False, []
        for line in lines:
            if line.lstrip().startswith("```"):
                fenced = not fenced
                buf.append("")
                continue
            buf.append("" if fenced else line)
        for match in LINK.finditer("\n".join(buf)):
            target = match.group(1)
            if target.startswith(("http://", "https://", "mailto:", "#", "tel:")):
                continue
            path_part = target.split("#", 1)[0]
            if not path_part:
                continue
            # Archived documents keep their historical paths verbatim as
            # evidence; rewriting them would falsify the record.
            if str(doc).startswith(ARCHIVE_OK):
                continue
            resolved = (doc.parent / path_part).resolve()
            if not resolved.exists():
                broken.append((doc, target, str(resolved)))

    if not broken:
        print(f"Doc links: PASS ({len(targets)} markdown file(s) checked)")
        return 0

    for doc, target, resolved in broken:
        print(f"FAIL: broken link in {doc}: {target} -> {resolved}", file=sys.stderr)
    print(f"Doc links: FAIL ({len(broken)} broken link(s))", file=sys.stderr)
    return 1


if __name__ == "__main__":
    sys.exit(main(sys.argv))
