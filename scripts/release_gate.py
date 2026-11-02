#!/usr/bin/env python3
"""Fail unless every required platform-verification job succeeded.

`release-gate` is the single aggregate check that is meant to be required for
merge. It must fail not only when a required job failed, but also when one was
skipped, neutral, cancelled or never reported: in the evidence vocabulary a
skipped or neutral check is not success, so it cannot satisfy a gate.

The gate covers the three desktop platforms the product ships for. Each native
desktop job builds and exercises the same shared tree, so a green gate is the
only evidence that Linux, Windows and macOS are all release-ready together.

Usage:

    python3 scripts/release_gate.py --needs-json "$NEEDS_JSON"

`NEEDS_JSON` is the workflow job's `toJSON(needs)` context, for example:

    env:
      NEEDS_JSON: ${{ toJSON(needs) }}
    run: python3 scripts/release_gate.py --needs-json "$NEEDS_JSON"

Exits 0 only when every required job reports `success`; exits 1 otherwise.
"""
from __future__ import annotations

import argparse
import json
import sys

# Shared evidence that is not platform-specific.
CORE_JOBS = (
    "rust",
    "flutter-static",
    "granted-descriptor-capability",
)

# The native desktop jobs that must build and pass on the same source tree.
# Adding a desktop platform to the product means adding it here; a skipped or
# neutral platform job never satisfies the gate.
DESKTOP_JOBS = (
    "macos",
    "linux",
    "windows",
)

REQUIRED_JOBS = CORE_JOBS + DESKTOP_JOBS


def evaluate(needs: object, required: tuple[str, ...] = REQUIRED_JOBS) -> list[str]:
    """Return one problem line per required job that did not succeed."""
    if not isinstance(needs, dict):
        return ["the needs context is not a JSON object"]

    problems: list[str] = []
    for job in required:
        entry = needs.get(job)
        if not isinstance(entry, dict):
            problems.append(f"required job {job!r} was not reported (missing)")
            continue
        result = entry.get("result")
        if result != "success":
            problems.append(f"required job {job!r} reported {result!r}, not 'success'")
    return problems


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--needs-json", required=True,
                        help="the workflow's toJSON(needs) context")
    parser.add_argument("--required", nargs="+", default=list(REQUIRED_JOBS),
                        help="job names that must all report success")
    args = parser.parse_args(argv)

    try:
        needs = json.loads(args.needs_json)
    except json.JSONDecodeError as error:
        print(f"release-gate: FAIL — needs context is not valid JSON ({error})",
              file=sys.stderr)
        return 1

    problems = evaluate(needs, tuple(args.required))
    if problems:
        for problem in problems:
            print(f"release-gate: FAIL — {problem}", file=sys.stderr)
        print("release-gate: a failed, skipped, neutral, cancelled or missing "
              "required job cannot pass the aggregate gate.", file=sys.stderr)
        return 1

    print(f"release-gate: PASS — all {len(args.required)} required jobs succeeded")
    return 0


if __name__ == "__main__":
    sys.exit(main())
