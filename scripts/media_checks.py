#!/usr/bin/env python3
"""Independent export observations reused by UI and packaged acceptance."""
from __future__ import annotations

import argparse
import array
import hashlib
import json
import math
import os
from pathlib import Path
import shutil
import subprocess
import sys


class MediaFailure(Exception):
    pass


class MediaBlocked(Exception):
    pass


def run(command: list[str], *, timeout: float = 60, binary: bool = False):
    try:
        result = subprocess.run(command, capture_output=True, text=not binary,
                                timeout=timeout)
    except FileNotFoundError as error:
        raise MediaBlocked(f"missing tool: {command[0]}") from error
    except subprocess.TimeoutExpired as error:
        raise MediaFailure(f"bounded media operation timed out: {command[0]}") from error
    if result.returncode or result.stderr.strip():
        detail = result.stderr[:2000]
        raise MediaFailure(f"media tool returned {result.returncode}: {detail}")
    return result.stdout


def check_metadata(probe: dict, expected: dict) -> dict:
    """Compare the selected preset, including an upper duration bound."""
    streams = probe.get("streams", [])
    if not isinstance(streams, list):
        raise MediaFailure("malformed streams")
    video = [s for s in streams if s.get("codec_type") == "video"]
    audio = [s for s in streams if s.get("codec_type") == "audio"]
    if len(video) != 1:
        raise MediaFailure(f"expected one video stream, observed {len(video)}")
    if (video[0].get("width"), video[0].get("height")) != (
            expected["width"], expected["height"]):
        raise MediaFailure(f"wrong dimensions: {video[0]}")
    if video[0].get("codec_name") != expected["video_codec"]:
        raise MediaFailure(f"wrong video codec: {video[0]}")
    if expected.get("audio_codec"):
        if len(audio) != 1 or audio[0].get("codec_name") != expected["audio_codec"]:
            raise MediaFailure(f"wrong/missing audio: {audio}")
    elif audio:
        raise MediaFailure("unexpected audio stream")
    duration = float(probe.get("format", {}).get("duration", "nan"))
    target = float(expected["duration"])
    if not math.isfinite(target) or not 0 < target <= 120:
        raise MediaFailure("smoke preset duration must be finite and within 0..120 seconds")
    tolerance = float(expected.get("duration_tolerance", 0.15))
    if not math.isfinite(duration) or abs(duration - target) > tolerance:
        raise MediaFailure(f"duration {duration}, expected {target} +/- {tolerance}")
    container = probe.get("format", {}).get("format_name", "").split(",")
    if expected["container"] not in container:
        raise MediaFailure(f"wrong container: {container}")
    return {"duration": duration, "streams": streams, "format": probe["format"]}


def verify_export(path: Path, expected: dict, *, ffprobe: str = "ffprobe",
                  ffmpeg: str = "ffmpeg", player: bool = False) -> dict:
    if not path.is_file() or not path.stat().st_size:
        raise MediaFailure(f"missing/empty export: {path}")
    if path.stat().st_size > 256 * 1024 * 1024:
        raise MediaBlocked("smoke verifier supports outputs up to 256 MiB")
    probe = json.loads(run([ffprobe, "-v", "error", "-show_format",
                            "-show_streams", "-of", "json", str(path)]))
    report = check_metadata(probe, expected)
    duration = float(expected["duration"])
    # Full bounded decode, with errors fatal even if a helper exits zero.
    # Frame hashes prove decoded content; moving fixtures must actually change.
    frames = run([ffmpeg, "-v", "error", "-xerror", "-i", str(path),
                  "-map", "0:v:0", "-an", "-t", str(duration + 0.2),
                  "-vf", "scale=64:36", "-f", "framemd5", "-"])
    hashes = [line.rsplit(",", 1)[-1].strip() for line in frames.splitlines()
              if line and not line.startswith("#")]
    if len(hashes) < 2:
        raise MediaFailure("fewer than two decoded video frames")
    if expected.get("moving_video", False) and len(set(hashes)) < 2:
        raise MediaFailure("moving fixture exported frozen video")
    report["decoded_video_frames"] = len(hashes)
    report["distinct_frame_hashes"] = len(set(hashes))
    if expected.get("audio_codec"):
        pcm = run([ffmpeg, "-v", "error", "-xerror", "-i", str(path),
                   "-map", "0:a:0", "-vn", "-t", str(duration + 0.2),
                   "-ac", "1", "-ar", "8000", "-f", "s16le", "-"], binary=True)
        samples = array.array("h")
        samples.frombytes(pcm)
        if sys.byteorder != "little":
            samples.byteswap()
        if not samples:
            raise MediaFailure("no decoded audio samples")
        rms = math.sqrt(sum(float(s) ** 2 for s in samples) / len(samples))
        if expected.get("audible_audio", False) and rms == 0:
            raise MediaFailure("tone fixture exported silent audio")
        report.update(decoded_audio_samples=len(samples), audio_rms_pcm16=rms)
    report["independent_player"] = "UNVERIFIED"
    if player:
        ffplay = shutil.which("ffplay")
        if not ffplay:
            raise MediaBlocked("ffplay unavailable; independent player unverified")
        command = [ffplay, "-v", "error", "-autoexit", "-volume", "0",
                   "-t", str(duration), str(path)]
        if sys.platform.startswith("linux") and not os.environ.get("DISPLAY"):
            xvfb = shutil.which("xvfb-run")
            if not xvfb:
                raise MediaBlocked("independent player needs DISPLAY or Xvfb")
            command = [xvfb, "-a", *command]
        run(command, timeout=duration + 15)
        report["independent_player"] = "PASS (ffplay process completed)"
    digest = hashlib.sha256(path.read_bytes()).hexdigest()
    return report | {"result": "PASS", "path": str(path), "sha256": digest,
                     "expected": expected,
                     "player_audio_backend": os.environ.get("SDL_AUDIODRIVER", "system"),
                     "audio_output_heard": "UNVERIFIED",
                     "limits": ["Independent file validation; not an app/UI or human claim.",
                                "Nonzero audio is not A/V synchronization evidence."]}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("media", type=Path)
    parser.add_argument("--expected", required=True, type=Path)
    parser.add_argument("--report", required=True, type=Path)
    parser.add_argument("--player", action="store_true")
    args = parser.parse_args()
    try:
        report = verify_export(args.media, json.loads(args.expected.read_text()),
                               player=args.player)
        code = 0
    except MediaBlocked as error:
        report, code = {"result": "BLOCKED", "reason": str(error)}, 3
    except (MediaFailure, ValueError, KeyError, OSError) as error:
        report, code = {"result": "FAIL", "reason": str(error)}, 1
    args.report.parent.mkdir(parents=True, exist_ok=True)
    args.report.write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps(report, indent=2))
    return code


if __name__ == "__main__":
    raise SystemExit(main())
