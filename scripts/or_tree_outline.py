#!/usr/bin/env python3
"""Print a compact outline of an or_ui semantics tree artifact.

`scripts/or_ui.py tree --name X` writes both the framework dump and a flat JSON
node list under the run's artifacts directory. The dump is precise but long; this
prints one line per node so an agent can read a screen quickly:

    python3 scripts/or_tree_outline.py apps/or_app/build/ui-harness/artifacts/tree-X-0.json

Each line is `indent id role "label" (rect) actions`; a label is truncated to the
first line unless --full is passed. Nodes with neither label nor value are folded
away by default (`--all` keeps them) so the outline stays readable.
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path


def clips(text: str, limit: int, full: bool) -> str:
    text = " ".join(text.split())
    if full or len(text) <= limit:
        return text
    return text[: limit - 1] + "…"


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("tree", help="path to an or_ui tree-*.json artifact")
    parser.add_argument("--all", action="store_true", help="keep unlabeled structure nodes")
    parser.add_argument("--full", action="store_true", help="do not truncate labels")
    parser.add_argument("--limit", type=int, default=90, help="label width (default 90)")
    args = parser.parse_args(argv)

    payload = json.loads(Path(args.tree).read_text(encoding="utf-8"))
    nodes = payload.get("nodes", [])
    if not nodes:
        print("or_tree_outline: no nodes in this artifact", file=sys.stderr)
        return 2
    parents = {node["id"]: node.get("parent", -1) for node in nodes}
    # The artifact is ordered by traversal, so a depth counter suffices when no
    # explicit parent is recorded.
    depth = 0
    previous_id = -1
    for node in nodes:
        if "parent" in node:
            chain, cursor = 0, node.get("parent", -1)
            while cursor in parents and cursor != -1 and chain < 64:
                cursor = parents[cursor]
                chain += 1
            depth = chain
        elif node["id"] < previous_id:
            depth = 0
        previous_id = node["id"]
        role = node.get("role") or "structure"
        label = clips(node.get("label") or "", args.limit, args.full)
        value = clips(node.get("value") or "", 40, args.full)
        actions = ",".join(node.get("actions") or [])
        rect = node.get("rect") or {}
        geometry = "{:.0f},{:.0f} {:.0f}x{:.0f}".format(
            rect.get("left", 0), rect.get("top", 0), rect.get("width", 0), rect.get("height", 0))
        if not args.all and role == "structure" and not label and not value:
            continue
        parts = [f"{'  ' * depth}{node['id']:>4} {role:<9}"]
        if label:
            parts.append(f'"{label}"')
        if value:
            parts.append(f"value={value!r}")
        parts.append(geometry)
        if actions:
            parts.append(actions)
        print(" ".join(parts))
    return 0


if __name__ == "__main__":
    sys.exit(main())
