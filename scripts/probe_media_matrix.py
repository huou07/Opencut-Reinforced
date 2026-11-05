#!/usr/bin/env python3
"""Measure what media this machine's FFmpeg build can actually do.

The capability matrix in docs/reference/media-capability-matrix.md must be
evidence, not intent. This script generates one tiny fixture per container and
codec, asks the local ffprobe what it sees, decodes the whole file, and checks
the result against the import matrix that `crates/or_core/src/media_probe.rs`
enforces. The codec lists are read from that Rust source, so this probe cannot
drift into a second, friendlier opinion.

Usage:
    python3 scripts/probe_media_matrix.py [--out FILE.json] [--markdown]
    python3 scripts/probe_media_matrix.py --codec-source PATH   # other checkout

Exit code is 0 when the probe ran, 2 when the toolchain it needs is missing.
`--strict` instead fails when an expected round trip does not work.
"""

from __future__ import annotations

import argparse
import json
import os
import platform
import re
import shutil
import subprocess
import sys
import tempfile
import time
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
BUNDLED = [
    REPO / "apps/or_app/.dart_tool/macos-ffmpeg-install/bin",
    REPO / "apps/or_app/build/macos/Build/Products/Debug/or_app.app/Contents/MacOS",
]

# One fixture per container/codec combination the product claims to support,
# plus the ones it deliberately refuses. `encoders` names the encoder the file
# needs; a build without it skips the row instead of guessing.
FIXTURES = [
    {
        "id": "mp4_h264_aac",
        "container": "MP4",
        "video": "h264",
        "audio": "aac",
        "encoders": ["libx264", "h264_videotoolbox"],
        "args": ["-c:v", "{v}", "-pix_fmt", "yuv420p", "-c:a", "aac", "-f", "mp4"],
        "expected": "supported",
    },
    {
        "id": "mov_h264_aac",
        "container": "MOV",
        "video": "h264",
        "audio": "aac",
        "encoders": ["libx264", "h264_videotoolbox"],
        "args": ["-c:v", "{v}", "-pix_fmt", "yuv420p", "-c:a", "aac", "-f", "mov"],
        "expected": "supported",
    },
    {
        "id": "mkv_h264_opus",
        "container": "MKV",
        "video": "h264",
        "audio": "opus",
        "encoders": ["libx264", "h264_videotoolbox"],
        "audio_encoder": "libopus",
        "args": ["-c:v", "{v}", "-pix_fmt", "yuv420p", "-c:a", "libopus", "-f", "matroska"],
        "expected": "supported",
    },
    {
        "id": "webm_vp9_opus",
        "container": "WebM",
        "video": "vp9",
        "audio": "opus",
        "video_encoder": "libvpx-vp9",
        "audio_encoder": "libopus",
        "args": ["-c:v", "libvpx-vp9", "-b:v", "0", "-crf", "45", "-c:a", "libopus", "-f", "webm"],
        "expected": "supported",
    },
    {
        "id": "mkv_ffv1_flac",
        "container": "MKV",
        "video": "ffv1",
        "audio": "flac",
        "video_encoder": "ffv1",
        "audio_encoder": "flac",
        "args": ["-c:v", "ffv1", "-c:a", "flac", "-f", "matroska"],
        "expected": "supported",
    },
    {
        "id": "mkv_av1_opus",
        "container": "MKV",
        "video": "av1",
        "audio": "opus",
        "video_encoder": "libsvtav1",
        "audio_encoder": "libopus",
        "args": ["-c:v", "libsvtav1", "-preset", "12", "-c:a", "libopus", "-f", "matroska"],
        "expected": "supported",
    },
    {
        "id": "mov_prores",
        "container": "MOV",
        "video": "prores",
        "audio": None,
        "video_encoder": "prores_ks",
        "args": ["-c:v", "prores_ks", "-profile:v", "3", "-f", "mov"],
        "expected": "supported",
    },
    {
        "id": "mp4_hevc_aac",
        "container": "MP4",
        "video": "hevc",
        "audio": "aac",
        "encoders": ["libx265", "hevc_videotoolbox"],
        "args": ["-c:v", "{v}", "-pix_fmt", "yuv420p", "-c:a", "aac", "-f", "mp4"],
        "expected": "supported",
    },
    {
        "id": "wav_pcm_s16le",
        "container": "WAV",
        "video": None,
        "audio": "pcm_s16le",
        "args": ["-vn", "-c:a", "pcm_s16le", "-f", "wav"],
        "expected": "supported",
    },
    {
        "id": "mp3_lame",
        "container": "MP3",
        "video": None,
        "audio": "mp3",
        "audio_encoder": "libmp3lame",
        "args": ["-vn", "-c:a", "libmp3lame", "-f", "mp3"],
        "expected": "supported",
    },
    {
        "id": "flac",
        "container": "FLAC",
        "video": None,
        "audio": "flac",
        "audio_encoder": "flac",
        "args": ["-vn", "-c:a", "flac", "-f", "flac"],
        "expected": "supported",
    },
    {
        "id": "m4a_aac",
        "container": "M4A",
        "video": None,
        "audio": "aac",
        "args": ["-vn", "-c:a", "aac", "-f", "mp4"],
        "expected": "supported",
    },
    {
        "id": "ogg_opus",
        "container": "OGG",
        "video": None,
        "audio": "opus",
        "audio_encoder": "libopus",
        "args": ["-vn", "-c:a", "libopus", "-f", "ogg"],
        "expected": "supported",
    },
    {
        "id": "png_still",
        "container": "PNG",
        "video": "png",
        "audio": None,
        "still": True,
        "args": ["-frames:v", "1", "-c:v", "png", "-f", "image2"],
        "expected": "supported",
    },
    {
        "id": "jpeg_still",
        "container": "JPEG",
        "video": "mjpeg",
        "audio": None,
        "still": True,
        "args": ["-frames:v", "1", "-c:v", "mjpeg", "-f", "image2"],
        "expected": "supported",
    },
    {
        "id": "webp_still",
        "container": "WebP",
        "video": "webp",
        "audio": None,
        "still": True,
        "args": ["-frames:v", "1", "-c:v", "libwebp", "-f", "image2"],
        "expected": "supported",
    },
    {
        "id": "gif_still",
        "container": "GIF",
        "video": "gif",
        "audio": None,
        "still": True,
        "args": ["-frames:v", "1", "-c:v", "gif", "-f", "gif"],
        # The shipped import matrix has no GIF row, and the render path has no
        # GIF decoder of its own; claiming it would be a lie.
        "expected": "refused",
    },
    {
        "id": "tiff_still",
        "container": "TIFF",
        "video": "tiff",
        "audio": None,
        "still": True,
        "args": ["-frames:v", "1", "-c:v", "tiff", "-f", "image2"],
        "expected": "refused",
    },
]

# The shipped build muxes only the delivery containers plus image2/wav, so a
# decode is proven by piping the decoded frames into one of those sink formats
# rather than by the (absent) null muxer.
DECODE_SINKS = {
    "video": ["-f", "matroska", "-c:v", "ffv1"],
    "audio": ["-f", "wav"],
}


LIBRARY_DIRS: list[str] = []


def find_tool(name: str, env: str) -> str | None:
    override = os.environ.get(env)
    if override and Path(override).is_file():
        record_library_dir(Path(override))
        return override
    for directory in BUNDLED:
        candidate = directory / name
        if candidate.is_file():
            # The pinned prefix links its own dylibs; a bare binary copy would
            # otherwise need a global search path.
            record_library_dir(candidate)
            return str(candidate)
    found = shutil.which(name)
    if found:
        record_library_dir(Path(found))
    return found


def record_library_dir(tool: Path) -> None:
    lib = tool.resolve().parent.parent / "lib"
    if lib.is_dir():
        LIBRARY_DIRS.append(str(lib))


def run(command: list[str], timeout: float = 120.0) -> tuple[int, str, str]:
    environment = os.environ.copy()
    if LIBRARY_DIRS:
        key = "DYLD_FALLBACK_LIBRARY_PATH" if sys.platform == "darwin" else "LD_LIBRARY_PATH"
        parts = [*LIBRARY_DIRS, environment.get(key, "")]
        environment[key] = ":".join(part for part in parts if part)
    try:
        done = subprocess.run(
            command,
            capture_output=True,
            text=True,
            timeout=timeout,
            check=False,
            env=environment,
        )
        return done.returncode, done.stdout, done.stderr
    except subprocess.TimeoutExpired:
        return 124, "", "timed out"


def run_quiet(command: list[str], timeout: float = 120.0) -> tuple[int, str]:
    """Run a command whose stdout is binary (a decoded frame or sample pipe)."""

    environment = os.environ.copy()
    if LIBRARY_DIRS:
        key = "DYLD_FALLBACK_LIBRARY_PATH" if sys.platform == "darwin" else "LD_LIBRARY_PATH"
        parts = [*LIBRARY_DIRS, environment.get(key, "")]
        environment[key] = ":".join(part for part in parts if part)
    try:
        done = subprocess.run(
            command,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.PIPE,
            timeout=timeout,
            check=False,
            env=environment,
        )
        return done.returncode, done.stderr.decode("utf-8", "replace")
    except subprocess.TimeoutExpired:
        return 124, "timed out"


def parse_codec_lists(source: Path) -> dict[str, list[str]]:
    text = source.read_text()
    lists: dict[str, list[str]] = {}
    for name in (
        "IMPORT_VIDEO_CODECS",
        "IMPORT_AUDIO_CODECS",
        "STILL_IMAGE_FORMAT_NAMES",
    ):
        match = re.search(
            rf"const {name}:[^=]*=\s*&?\[(.*?)\];", text, re.DOTALL
        )
        if not match:
            raise SystemExit(f"cannot find {name} in {source}")
        lists[name] = re.findall(r'"([^"]+)"', match.group(1))
    return lists


def build_fixture(
    ffmpeg: str,
    workdir: Path,
    spec: dict,
    encoders: set[str],
) -> tuple[Path | None, str | None]:
    extension = {
        "mp4_h264_aac": "mp4",
        "mov_h264_aac": "mov",
        "mkv_h264_opus": "mkv",
        "webm_vp9_opus": "webm",
        "mkv_ffv1_flac": "mkv",
        "mkv_av1_opus": "mkv",
        "mov_prores": "mov",
        "mp4_hevc_aac": "mp4",
        "wav_pcm_s16le": "wav",
        "mp3_lame": "mp3",
        "flac": "flac",
        "m4a_aac": "m4a",
        "ogg_opus": "ogg",
        "png_still": "png",
        "jpeg_still": "jpeg",
        "webp_still": "webp",
        "gif_still": "gif",
        "tiff_still": "tiff",
    }[spec["id"]]
    path = workdir / f"{spec['id']}.{extension}"

    video_encoder = spec.get("video_encoder")
    if video_encoder is None and spec.get("encoders"):
        video_encoder = next(
            (name for name in spec["encoders"] if name in encoders), None
        )
    audio_encoder = spec.get("audio_encoder")
    for encoder in (video_encoder, audio_encoder):
        if encoder and encoder not in encoders:
            return None, f"encoder {encoder} is not in this FFmpeg build"

    args = [ffmpeg, "-hide_banner", "-v", "error", "-y"]
    if spec.get("video"):
        args += ["-f", "lavfi", "-i", "testsrc2=size=320x180:rate=30"]
    if spec.get("audio"):
        args += ["-f", "lavfi", "-i", "sine=frequency=440:sample_rate=48000"]
    if not spec.get("still"):
        args += ["-t", "1"]
    rendered = []
    for token in spec["args"]:
        if token == "{v}":
            rendered.append(video_encoder or "")
        else:
            rendered.append(token)
    args += [*rendered, str(path)]
    code, _, err = run(args)
    if code != 0 or not path.exists():
        return None, f"fixture generation failed: {err.strip()[:200]}"
    return path, None


def probe(ffprobe: str, path: Path) -> dict:
    code, out, err = run(
        [
            ffprobe,
            "-v",
            "error",
            "-print_format",
            "json",
            "-show_format",
            "-show_streams",
            str(path),
        ]
    )
    if code != 0:
        return {"ok": False, "error": err.strip()[:300]}
    data = json.loads(out)
    names: list[str] = []
    for name in (data.get("format", {}).get("format_name") or "").split(","):
        if name:
            names.append(name)
    codecs: list[tuple[str, str]] = []
    for stream in data.get("streams", []):
        kind = stream.get("codec_type")
        if kind in {"video", "audio"}:
            codecs.append((kind, stream.get("codec_name") or ""))
    return {"ok": True, "format_names": names, "codecs": codecs}


HEADERS = {
    ".png": b"\x89PNG",
    ".jpeg": b"\xff\xd8\xff",
    ".mp3": b"ID3",
    ".ogg": b"OggS",
    ".flac": b"fLaC",
    ".webp": b"RIFF",
    ".wav": b"RIFF",
    ".avi": b"RIFF",
    ".aiff": b"FORM",
}


def container_accepted(path: Path, names: list[str]) -> bool:
    header = path.read_bytes()[:12]
    suffix = path.suffix.lower()
    if suffix in HEADERS and header.startswith(HEADERS[suffix]):
        return True
    if header.startswith(b"\x1a\x45\xdf\xa3"):
        return True
    if len(header) >= 8 and header[4:8] == b"ftyp":
        return True
    return any(
        name in {"matroska", "webm"}
        or name in {"mov", "mp4", "m4a", "3gp", "3g2", "mj2"}
        or name in {"wav", "mp3", "ogg", "flac", "aiff", "avi"}
        for name in names
    )


def matrix_verdict(
    path: Path, probed: dict, lists: dict[str, list[str]], spec: dict
) -> tuple[str, str]:
    if not probed["ok"]:
        return "refused", probed["error"]
    names = probed["format_names"]
    if not container_accepted(path, names):
        return "refused", f"container {','.join(names) or 'unknown'} is outside the profile"
    still = any(name in lists["STILL_IMAGE_FORMAT_NAMES"] for name in names)
    video = [codec for kind, codec in probed["codecs"] if kind == "video"]
    audio = [codec for kind, codec in probed["codecs"] if kind == "audio"]
    if still:
        if len(probed["codecs"]) == 1 and video and video[0] in {"png", "mjpeg", "webp"}:
            return "accepted", "still image"
        return "refused", "a still must be a single PNG, JPEG or WebP stream"
    if not video and not audio:
        return "refused", "no audio or video stream"
    if spec.get("still"):
        # A still encoded into a container the matrix calls a movie is not a still.
        return "refused", f"container {','.join(names)} is a movie container"
    for codec in video:
        if codec not in lists["IMPORT_VIDEO_CODECS"]:
            return "refused", f"video codec {codec} is outside the profile"
    for codec in audio:
        if codec not in lists["IMPORT_AUDIO_CODECS"]:
            return "refused", f"audio codec {codec} is outside the profile"
    return "accepted", "import profile accepts it"


def decode_file(ffmpeg: str, path: Path, probed: dict) -> tuple[bool, str]:
    """Decode every audio and video stream and report the first failure."""

    if not probed.get("ok"):
        return False, "the file could not be probed"
    kinds = {kind for kind, _ in probed["codecs"]}
    if not kinds:
        return False, "the file has no audio or video stream"
    for kind in sorted(kinds):
        code, err = run_quiet(
            [
                ffmpeg,
                "-v",
                "error",
                "-i",
                str(path),
                "-map",
                f"0:{kind[0]}:0",
                *DECODE_SINKS[kind],
                "-",
            ]
        )
        if code != 0 or err.strip():
            return False, err.strip() or f"exit {code} decoding {kind}"
    return True, ""


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--out", type=Path, help="write the raw result as JSON")
    parser.add_argument("--markdown", action="store_true", help="print a table")
    parser.add_argument(
        "--codec-source",
        type=Path,
        default=REPO / "crates/or_core/src/media_probe.rs",
        help="the media_probe.rs whose codec lists define the import profile",
    )
    parser.add_argument("--strict", action="store_true")
    args = parser.parse_args()

    ffmpeg = find_tool("ffmpeg", "OR_FFMPEG_PATH")
    ffprobe = find_tool("ffprobe", "OR_FFPROBE_PATH")
    # The shipped build is decode-oriented: it carries no test-source filters or
    # encoders, so fixtures are generated with a development FFmpeg and then
    # measured against the shipped one.
    generator = os.environ.get("OR_FFMPEG_GENERATOR") or shutil.which("ffmpeg")
    if not ffmpeg or not ffprobe:
        print("FAIL: no ffmpeg/ffprobe found (set OR_FFMPEG_PATH/OR_FFPROBE_PATH)")
        return 2
    if not args.codec_source.is_file():
        print(f"FAIL: codec source not found: {args.codec_source}")
        return 2

    lists = parse_codec_lists(args.codec_source)
    decoders, encoders = ffmpeg_lists(ffmpeg)
    gen_encoders = encoders if generator in (None, ffmpeg) else ffmpeg_lists(generator)[1]
    if not generator:
        generator = ffmpeg
        gen_encoders = encoders
    banner = run([ffmpeg, "-version"])[1].splitlines()
    version = banner[0] if banner else "unknown"

    rows = []
    failures = []
    with tempfile.TemporaryDirectory(prefix="or-media-matrix-") as raw:
        workdir = Path(raw)
        for spec in FIXTURES:
            path, skipped = build_fixture(generator, workdir, spec, gen_encoders)
            row = {
                "id": spec["id"],
                "container": spec["container"],
                "video": spec["video"],
                "audio": spec["audio"],
                "expected": spec["expected"],
            }
            if path is None:
                row.update(
                    {
                        "verdict": "skipped",
                        "detail": skipped,
                        "decode": "not attempted",
                    }
                )
                rows.append(row)
                continue
            probed = probe(ffprobe, path)
            verdict, detail = matrix_verdict(path, probed, lists, spec)
            decoders_missing = []
            if probed["ok"]:
                for _, codec in probed["codecs"]:
                    if codec and codec not in decoders:
                        decoders_missing.append(codec)
            decodes, decode_error = decode_file(ffmpeg, path, probed)
            if not decodes:
                failures.append(f"{spec['id']}: decode failed: {decode_error[:160]}")
            wanted = "accepted" if spec["expected"] == "supported" else "refused"
            if verdict != wanted:
                failures.append(
                    f"{spec['id']}: import matrix says {verdict}, expected {wanted} ({detail})"
                )
            candidates = [
                name
                for name in [
                    *(spec.get("encoders") or []),
                    spec.get("video_encoder"),
                    spec.get("audio_encoder"),
                ]
                if name
            ]
            row["encoders_present"] = sorted(
                name for name in candidates if name in encoders
            )
            if decoders_missing:
                failures.append(
                    f"{spec['id']}: build has no decoder for {', '.join(decoders_missing)}"
                )
            row.update(
                {
                    "verdict": verdict,
                    "detail": detail,
                    "format_names": probed.get("format_names", []),
                    "codecs": probed.get("codecs", []),
                    "decode": "ok" if decodes else "failed",
                    "decoders_missing": decoders_missing,
                }
            )
            rows.append(row)

    result = {
        "host": {
            "platform": platform.system(),
            "release": platform.release(),
            "machine": platform.machine(),
            "python": platform.python_version(),
        },
        "ffmpeg": {"path": ffmpeg, "version": version},
        "fixture_generator": generator,
        "ffprobe": ffprobe,
        "codec_source": str(args.codec_source),
        "generated_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        "rows": rows,
        "failures": failures,
    }

    if args.markdown:
        print("| Container | Video | Audio | Import matrix | Decode (shipped) | Encode (shipped) |")
        print("|---|---|---|---|---|---|")
        for row in rows:
            present = row.get("encoders_present")
            print(
                "| {container} | {video} | {audio} | {verdict} | {decode} | {encode} |".format(
                    container=row["container"],
                    video=row["video"] or "—",
                    audio=row["audio"] or "—",
                    verdict=row["verdict"],
                    decode=row["decode"],
                    encode=", ".join(present) if present else "—",
                )
            )
    print(
        f"\n{sum(1 for row in rows if row['decode'] == 'ok')}/{len(rows)} fixtures decoded; "
        f"{len(failures)} unexpected result(s) on {platform.system()} {platform.machine()}",
        file=sys.stderr,
    )
    for failure in failures:
        print(f"  ! {failure}", file=sys.stderr)

    if args.out:
        args.out.parent.mkdir(parents=True, exist_ok=True)
        args.out.write_text(json.dumps(result, indent=2) + "\n")
    return 1 if (args.strict and failures) else 0


def ffmpeg_lists(ffmpeg: str) -> tuple[set[str], set[str]]:
    decoders: set[str] = set()
    encoders: set[str] = set()
    for flag, target in (("-decoders", decoders), ("-encoders", encoders)):
        code, out, _ = run([ffmpeg, "-hide_banner", flag])
        if code != 0:
            continue
        for line in out.splitlines():
            fields = line.split()
            if len(fields) >= 2 and len(fields[0]) == 6:
                target.add(fields[1])
    return decoders, encoders


if __name__ == "__main__":
    sys.exit(main())
