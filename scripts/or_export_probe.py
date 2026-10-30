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
         "format=duration,size:stream=codec_name,width,height,nb_frames,avg_frame_rate,r_frame_rate",
         "-of", "json", path],
        capture_output=True, text=True, env=env, check=False,
    )
    if probe.returncode != 0:
        print(probe.stderr or "ffprobe failed", file=sys.stderr)
        return 1
    data = json.loads(probe.stdout)
    raw_streams = data.get("streams", [])
    streams = [s.get("codec_name", "") for s in raw_streams]
    duration = float(data.get("format", {}).get("duration", 0) or 0)
    size = int(data.get("format", {}).get("size", 0) or 0)
    video = next(
        (s for s in raw_streams if s.get("codec_name") == "h264" and s.get("width")),
        None,
    )
    print(
        f"{os.path.basename(path)} codecs={streams} duration={duration:.3f}s size={size}B"
        + (
            f" video={video.get('width')}x{video.get('height')} "
            f"frames={video.get('nb_frames')} fps={ratio(video.get('r_frame_rate'))}"
            if video
            else ""
        )
    )
    if "h264" not in streams or "aac" not in streams or duration <= 0 or size <= 0:
        print("probe mismatch: expected h264 + aac with positive duration/size", file=sys.stderr)
        return 1
    # A job that reports success but writes a short clip still fails here: the
    # stream's own frame count has to cover the container duration.
    if video:
        fps = ratio(video.get("r_frame_rate"))
        frames = int(video.get("nb_frames") or 0)
        if fps and fps > 1:
            expected = duration * fps
            if frames <= 0 or abs(frames - expected) > max(2.0, expected * 0.02):
                print(
                    f"probe mismatch: {frames} frames at {fps:.2f} fps does not cover "
                    f"{duration:.3f}s (expected about {expected:.1f})",
                    file=sys.stderr,
                )
                return 1
    return 0


def ratio(value):
    """Parse ffprobe's 'num/den' frame rate into a float."""
    if not value:
        return 0.0
    head, _, tail = value.partition("/")
    try:
        numerator = float(head)
        denominator = float(tail) if tail else 1.0
    except ValueError:
        return 0.0
    return numerator / denominator if denominator else 0.0


if __name__ == "__main__":
    sys.exit(main())
