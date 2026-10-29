#!/usr/bin/env python3
"""Probe the file the UI export lane produced.

`scripts/ui_scenarios/export-encode.json` drives the real Export dialog and
waits for `Export complete`. That flag is the job talking about itself; this
probe reads the produced file so verification.md's rule holds: an export is
proven by an independent probe, not by the export job.

Usage: python3 scripts/or_export_probe.py [--dir <artifacts>/replacements]
Exit 0 when a decodable h264+aac file with positive duration is found.
"""

from __future__ import annotations

import argparse
import glob
import json
import os
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))


def find_ffprobe() -> str:
    for pattern in ("dist/*.app/Contents/MacOS/ffprobe", "dist/*/Contents/MacOS/ffprobe"):
        for candidate in sorted(glob.glob(os.path.join(ROOT, pattern))):
            if os.access(candidate, os.X_OK):
                return candidate
    found = subprocess.run(
        ["which", "ffprobe"], capture_output=True, text=True, check=False
    )
    return found.stdout.strip()


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--dir",
        default=os.path.join(
            ROOT, "apps/or_app/build/ui-harness/artifacts/replacements"
        ),
    )
    args = parser.parse_args()
    exports = sorted(glob.glob(os.path.join(args.dir, "export.*")))
    if not exports:
        print(f"no export artifact under {args.dir}", file=sys.stderr)
        return 2
    path = max(exports, key=os.path.getmtime)
    ffprobe = find_ffprobe()
    if not ffprobe:
        print("ffprobe not found (packaged app or PATH)", file=sys.stderr)
        return 2
    env = dict(os.environ)
    probe = subprocess.run(
        [ffprobe, "-v", "error", "-show_entries",
         "format=duration,size:stream=codec_name,width,height,nb_frames",
         "-of", "json", path],
        capture_output=True, text=True, env=env, check=False,
    )
    if probe.returncode != 0:
        print(probe.stderr or "ffprobe failed", file=sys.stderr)
        return 1
    data = json.loads(probe.stdout)
    streams = [s.get("codec_name", "") for s in data.get("streams", [])]
    duration = float(data.get("format", {}).get("duration", 0) or 0)
    size = int(data.get("format", {}).get("size", 0) or 0)
    print(f"{os.path.basename(path)} codecs={streams} duration={duration:.3f}s size={size}B")
    if "h264" not in streams or "aac" not in streams or duration <= 0 or size <= 0:
        print("probe mismatch: expected h264 + aac with positive duration/size", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
