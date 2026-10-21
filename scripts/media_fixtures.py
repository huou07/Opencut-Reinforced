#!/usr/bin/env python3
"""Generate tiny original test patterns; no downloads or third-party footage."""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import subprocess


RECIPES = {
    "video-audio.mkv": ["-f", "lavfi", "-i", "testsrc2=size=160x90:rate=12:duration=2",
                        "-f", "lavfi", "-i", "sine=frequency=440:sample_rate=48000:duration=2",
                        "-c:v", "ffv1", "-c:a", "pcm_s16le"],
    "silent-video.mkv": ["-f", "lavfi", "-i", "testsrc2=size=320x180:rate=24:duration=2",
                         "-an", "-c:v", "ffv1"],
    "video-audio.webm": ["-f", "lavfi", "-i", "testsrc2=size=160x90:rate=12:duration=2",
                         "-f", "lavfi", "-i", "sine=frequency=440:sample_rate=48000:duration=2",
                         "-c:v", "libvpx-vp9", "-c:a", "libopus", "-b:v", "100k"],
    "silent-video.avi": ["-f", "lavfi", "-i", "testsrc2=size=64x48:rate=15:duration=2",
                         "-an", "-c:v", "rawvideo", "-pix_fmt", "yuv420p"],
    "audio-only.wav": ["-f", "lavfi", "-i", "sine=frequency=660:sample_rate=44100:duration=2",
                        "-c:a", "pcm_s16le"],
}


def generate(destination: Path, ffmpeg: str = "ffmpeg") -> dict:
    destination.mkdir(parents=True, exist_ok=True)
    version = subprocess.check_output([ffmpeg, "-version"], text=True).splitlines()[0]
    files = []
    for name, recipe in RECIPES.items():
        path = destination / name
        command = [ffmpeg, "-v", "error", "-nostdin", "-y", *recipe,
                   "-map_metadata", "-1", "-fflags", "+bitexact",
                   "-flags:v", "+bitexact", "-flags:a", "+bitexact",
                   "-threads", "1", str(path)]
        subprocess.run(command, check=True, capture_output=True, timeout=60)
        files.append({"name": name, "command": command})
    (destination / "unsupported.txt").write_bytes(b"OR original unsupported media fixture\n")
    (destination / "corrupt.mkv").write_bytes(b"\x1a\x45\xdf\xa3OR deliberately truncated fixture")
    for name in ("unsupported.txt", "corrupt.mkv"):
        files.append({"name": name, "command": "literal bytes in media_fixtures.py"})
    for entry in files:
        path = destination / entry["name"]
        entry.update(bytes=path.stat().st_size,
                     sha256=hashlib.sha256(path.read_bytes()).hexdigest())
    report = {"schema": 1, "kind": "smoke", "generator": version,
              "provenance": "Original lavfi test patterns/tones and literal corrupt data; "
                            "MIT repository recipe; no external footage, fonts or models.",
              "determinism": "Byte checksums apply to this recorded FFmpeg build. "
                             "Compare only identical generator/version/recipe.",
              "files": files}
    (destination / "manifest.json").write_text(json.dumps(report, indent=2) + "\n")
    return report


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--out", required=True, type=Path)
    args = parser.parse_args()
    print(json.dumps(generate(args.out), indent=2))
