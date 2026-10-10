#!/usr/bin/env python3
"""Exercise the exact FFmpeg helper capabilities shipped with desktop builds."""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
from pathlib import Path


PNG_SIGNATURE = b"\x89PNG\r\n\x1a\n"


def _run(label: str, command: list[str], env: dict[str, str]) -> bytes:
    result = subprocess.run(command, capture_output=True, env=env)
    if result.returncode:
        diagnostic = result.stderr.decode("utf-8", errors="replace").strip()
        raise SystemExit(f"{label} failed ({result.returncode}):\n{diagnostic}")
    return result.stdout


def _probe(ffprobe: Path, path: Path, env: dict[str, str]) -> dict[str, object]:
    output = _run(
        f"ffprobe {path.name}",
        [
            str(ffprobe),
            "-v",
            "error",
            "-show_entries",
            "format=format_name:stream=codec_type,codec_name",
            "-of",
            "json",
            str(path),
        ],
        env,
    )
    return json.loads(output)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--bin", required=True, type=Path)
    parser.add_argument("--lib", required=True, type=Path)
    parser.add_argument("--media", required=True, type=Path)
    parser.add_argument("--work", required=True, type=Path)
    args = parser.parse_args()

    executable_suffix = ".exe" if os.name == "nt" else ""
    ffmpeg = args.bin / f"ffmpeg{executable_suffix}"
    ffprobe = args.bin / f"ffprobe{executable_suffix}"
    if not ffmpeg.is_file() or not ffprobe.is_file():
        raise SystemExit(f"FFmpeg helpers are missing from {args.bin}")
    if not args.media.is_file():
        raise SystemExit(f"The generated media fixture is missing: {args.media}")

    env = os.environ.copy()
    if sys.platform == "darwin":
        env["DYLD_LIBRARY_PATH"] = str(args.lib)
    elif sys.platform.startswith("linux"):
        env["LD_LIBRARY_PATH"] = str(args.lib)
    else:
        env["PATH"] = os.pathsep.join((str(args.bin), env.get("PATH", "")))
    build_configuration = _run(
        "ffmpeg build configuration",
        [str(ffmpeg), "-hide_banner", "-buildconf"],
        env,
    ).decode("utf-8", errors="replace")
    restricted_options = ("--enable-gpl", "--enable-version3", "--enable-nonfree")
    if any(option in build_configuration for option in restricted_options):
        raise SystemExit("The packaged helper build enables a restricted FFmpeg license mode.")
    encoders = _run(
        "VP9 and Opus encoder discovery",
        [str(ffmpeg), "-hide_banner", "-encoders"],
        env,
    ).decode("utf-8", errors="replace")
    for codec in ("libvpx-vp9", "libopus"):
        if codec not in encoders:
            raise SystemExit(f"The packaged helper is missing the required {codec} encoder.")

    args.work.mkdir(parents=True, exist_ok=True)
    thumbnail = args.work / "thumbnail.png"
    thumbnail.write_bytes(
        _run(
            "PNG video thumbnail decode/encode",
            [
                str(ffmpeg),
                "-hide_banner",
                "-loglevel",
                "error",
                "-y",
                "-i",
                str(args.media),
                "-map",
                "0:v:0",
                "-frames:v",
                "1",
                "-an",
                "-f",
                "image2pipe",
                "-vcodec",
                "png",
                "pipe:1",
            ],
            env,
        )
    )
    waveform = args.work / "waveform.png"
    waveform.write_bytes(
        _run(
            "showwavespic waveform generation",
            [
                str(ffmpeg),
                "-hide_banner",
                "-loglevel",
                "error",
                "-y",
                "-i",
                str(args.media),
                "-filter_complex",
                "[0:a:0]showwavespic=s=256x96[v]",
                "-map",
                "[v]",
                "-frames:v",
                "1",
                "-f",
                "image2pipe",
                "-vcodec",
                "png",
                "pipe:1",
            ],
            env,
        )
    )
    proxy = args.work / "proxy.mkv"
    _run(
        "MPEG-4 proxy encode",
        [
            str(ffmpeg),
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-i",
            str(args.media),
            "-map",
            "0:v:0",
            "-an",
            "-vf",
            "setpts=PTS-STARTPTS,scale=960:540:force_original_aspect_ratio=decrease:force_divisible_by=2,format=yuv420p",
            "-frames:v",
            "1",
            "-c:v",
            "mpeg4",
            "-f",
            "matroska",
            str(proxy),
        ],
        env,
    )

    source = _probe(ffprobe, args.media, env)
    if "matroska" not in source.get("format", {}).get("format_name", ""):
        raise SystemExit(f"The supported media fixture did not probe as Matroska: {source}")
    for image in (thumbnail, waveform):
        if (
            not image.is_file()
            or image.stat().st_size <= len(PNG_SIGNATURE)
            or image.read_bytes()[: len(PNG_SIGNATURE)] != PNG_SIGNATURE
        ):
            raise SystemExit(f"The generated PNG could not be validated: {image}")
    proxy_result = _probe(ffprobe, proxy, env)
    proxy_streams = proxy_result.get("streams", [])
    if not proxy.is_file() or not proxy_streams or proxy_streams[0].get("codec_name") != "mpeg4":
        raise SystemExit(f"The generated proxy could not be validated: {proxy_result}")

    webm = args.work / "delivery.webm"
    _run(
        "WebM VP9/Opus export",
        [
            str(ffmpeg),
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-i",
            str(args.media),
            "-map",
            "0:v:0",
            "-map",
            "0:a:0?",
            "-c:v",
            "libvpx-vp9",
            "-deadline",
            "realtime",
            "-cpu-used",
            "8",
            "-crf",
            "36",
            "-b:v",
            "0",
            "-c:a",
            "libopus",
            "-b:a",
            "128k",
            "-f",
            "webm",
            str(webm),
        ],
        env,
    )
    webm_result = _probe(ffprobe, webm, env)
    webm_streams = webm_result.get("streams", [])
    codecs = {stream.get("codec_name") for stream in webm_streams}
    if not webm.is_file() or codecs != {"vp9", "opus"}:
        raise SystemExit(f"The generated WebM stream set is invalid: {webm_result}")
    decoded = json.loads(
        _run(
            "WebM full decode",
            [
                str(ffprobe),
                "-v",
                "error",
                "-count_frames",
                "-show_entries",
                "stream=codec_type,nb_read_frames",
                "-of",
                "json",
                str(webm),
            ],
            env,
        )
    )
    decoded_counts = {
        stream.get("codec_type"): int(stream.get("nb_read_frames", "0"))
        for stream in decoded.get("streams", [])
    }
    if not all(decoded_counts.get(kind, 0) > 0 for kind in ("video", "audio")):
        raise SystemExit(f"The WebM did not fully decode both streams: {decoded_counts}")

    print(
        "OR_PACKAGED_FFMPEG_CAPABILITIES "
        + json.dumps(
            {
                "ffmpeg": str(ffmpeg),
                "ffprobe": str(ffprobe),
                "thumbnail_bytes": thumbnail.stat().st_size,
                "waveform_bytes": waveform.stat().st_size,
                "proxy_bytes": proxy.stat().st_size,
                "proxy_codec": proxy_streams[0]["codec_name"],
                "webm_bytes": webm.stat().st_size,
                "webm_codecs": sorted(codecs),
                "webm_decoded_frames": decoded_counts,
            },
            sort_keys=True,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
