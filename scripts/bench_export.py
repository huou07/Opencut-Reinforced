#!/usr/bin/env python3
"""Cross-platform export, render and decode throughput measurement.

The export path lives in ``crates/or_app_bridge``: the same code that feeds the
editor's viewer owns decoding, compositing and the encoder writer. The ignored
``export_media_benchmark`` test drives that path directly, so this script only
prepares a fixture, supplies the FFmpeg runtime to the test binary and reads the
result. Nothing here is a synthetic model of the pipeline.

Rows
----
``decode.<profile>``  frames decoded from one 1080p source (no compositing).
``render.<profile>``  frames composited to the export surface (no encoding).
``export.<profile>``  the whole path: decode, composite, encode, mux, write.

Profiles are the three delivery profiles the product ships: ``mp4`` (H.264 and
AAC), ``webm`` (VP9 and Opus) and ``mkv`` (FFV1 and PCM). The numbers therefore
include whatever encoder the host actually selected, which is the point: a
machine without a hardware H.264 encoder is allowed to be slower, and the report
says so instead of hiding it behind one blended figure.

Hosts
-----
Linux and macOS print frames per second. Windows uses the same Rust test, so the
measurement is identical; only the dynamic-library handoff differs
(``DYLD_LIBRARY_PATH`` on macOS, ``LD_LIBRARY_PATH`` on Linux, ``PATH`` on
Windows).

Exit codes: 0 all rows measured, 3 a row could not be measured (the reason is
printed), 1 an unexpected failure.
"""
from __future__ import annotations

import argparse
import json
import os
import re
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
FIXTURE = ROOT / "crates/or_media/tests/fixtures/phone_portrait_90_h264_aac.mp4"
PROFILES = ("mp4", "webm", "mkv")
RESULT = re.compile(r"^OR_EXPORT_(?:BENCH|PIPELINE)\w*(?P<body>.*)$", re.MULTILINE)
FIELD = re.compile(r"(\w+)=([^\s]+)")


def _configured_prefix() -> Path | None:
    """The FFmpeg prefix this checkout was built against.

    The test binary links against a specific FFmpeg soname set, so the binary
    that happens to be first on ``PATH`` is usually not the one that can load it.
    The build prefix is authoritative: ``FFMPEG_DIR`` first, then the checkout's
    cargo config, then the machine prefixes the scripts stage into.
    """
    from_env = os.environ.get("FFMPEG_DIR")
    if from_env and Path(from_env).is_dir():
        return Path(from_env)
    for config in (ROOT / ".cargo/config.toml", Path.home() / ".cargo/config.toml"):
        if not config.is_file():
            continue
        for line in config.read_text().splitlines():
            if "FFMPEG_DIR" not in line:
                continue
            match = re.search(r"""['"]((?:/|[A-Za-z]:[\\/])[^'"]+)['"]""", line)
            if match and Path(match.group(1)).is_dir():
                return Path(match.group(1))
    candidates = [
        ROOT / "apps/or_app/.dart_tool/macos-ffmpeg-install",
        Path.home() / ".local/ffmpeg8",
        Path(r"C:\orprefix"),
        Path(r"C:\orbuild\ffmpeg-8.1.3"),
    ]
    for candidate in candidates:
        if (candidate / "lib").is_dir():
            return candidate
    return None


def _cargo() -> str:
    """The cargo binary, including a rustup install that is not on PATH.

    A non-interactive shell (ssh work, a CI step) often does not source the
    profile that puts ``~/.cargo/bin`` on PATH, and the benchmark should not
    need a login shell to find the toolchain that built the app.
    """
    found = shutil.which("cargo")
    if found:
        return found
    for candidate in (
        Path.home() / ".cargo/bin/cargo",
        Path.home() / ".cargo/bin/cargo.exe",
    ):
        if candidate.exists():
            return str(candidate)
    return "cargo"


def _which(name: str) -> str | None:
    prefix = _configured_prefix()
    if prefix is not None:
        for candidate in (prefix / "bin" / name, prefix / "bin" / f"{name}.exe"):
            if candidate.exists():
                return str(candidate)
    return shutil.which(name)


def _library_paths(prefix: Path | None) -> str:
    """The shared libraries the linked FFmpeg sonames live in."""
    prefixes = []
    if prefix is not None:
        prefixes.append(prefix)
    parts = []
    for prefix in prefixes:
        for part in (prefix / "lib", prefix / "bin"):
            if part.exists() and str(part) not in parts:
                parts.append(str(part))
    return os.pathsep.join(parts)


def _runtime_env(prefix: Path | None) -> dict[str, str]:
    """The environment the test binary needs to load FFmpeg at run time.

    ``cargo test`` starts the test binary with a sanitised dyld search path, so a
    dynamically linked FFmpeg is not found even when ``cargo build`` succeeded.
    The benchmark therefore asks cargo for the executable and runs it directly.
    """
    env = dict(os.environ)
    libraries = _library_paths(prefix)
    if not libraries:
        return env
    for name in ("DYLD_LIBRARY_PATH", "LD_LIBRARY_PATH", "PATH"):
        existing = env.get(name, "")
        env[name] = libraries + (os.pathsep + existing if existing else "")
    return env


def _executable(profile: str, cargo: str) -> str | None:
    """The test binary cargo just built, without running it."""
    completed = subprocess.run(
        [
            cargo,
            "test",
            "-p",
            "or_app_bridge",
            "--release",
            "--no-run",
            "--locked",
            "--message-format=json",
            "--test",
            "or_app_bridge",
        ],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    if completed.returncode == 0:
        for line in completed.stdout.splitlines():
            try:
                message = json.loads(line)
            except json.JSONDecodeError:
                continue
            if message.get("reason") == "compiler-artifact" and message.get("executable"):
                return message["executable"]
    # A library-only crate has no test target; fall back to the unit-test
    # invocation, which also accepts the message format.
    completed = subprocess.run(
        [cargo, "test", "-p", "or_app_bridge", "--release", "--no-run", "--locked",
         "--lib", "--message-format=json"],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    if completed.returncode != 0:
        sys.stderr.write(completed.stderr[-4000:])
        return None
    for line in completed.stdout.splitlines():
        try:
            message = json.loads(line)
        except json.JSONDecodeError:
            continue
        if message.get("reason") == "compiler-artifact" and message.get("executable"):
            return message["executable"]
    return None


def _prepare_fixture(ffmpeg: str, work: Path, seconds: float, source: Path,
                     env: dict[str, str]) -> Path | None:
    """A 1080p source long enough for a measurement, built without an encoder.

    The repository fixtures are deliberately short. Looping one with stream copy
    keeps the pixels and the codec identical on every host and needs no encoder,
    which matters because the shipped LGPL FFmpeg has no H.264 encoder.
    """
    if seconds <= 0:
        return source
    target = work / f"bench-{int(seconds)}s.mp4"
    if target.exists():
        return target
    loops = max(1, int(seconds * 30 / 30) + 1)
    completed = subprocess.run(
        [
            ffmpeg, "-hide_banner", "-loglevel", "error", "-y",
            "-stream_loop", str(loops), "-i", str(source),
            "-c", "copy", str(target),
        ],
        env=env,
        capture_output=True,
        text=True,
    )
    if completed.returncode != 0:
        sys.stderr.write(completed.stderr[-2000:])
        return None
    return target


def _measure(executable: str, env: dict[str, str], profile: str, mode: str,
             fixture: Path, frames: int, width: int, height: int) -> dict[str, object] | None:
    run_env = dict(env)
    run_env.update(
        {
            "OR_BENCH_SOURCE": str(fixture),
            "OR_BENCH_FRAMES": str(frames),
            "OR_BENCH_OUT_W": str(width),
            "OR_BENCH_OUT_H": str(height),
            "OR_BENCH_SRC_W": str(width),
            "OR_BENCH_SRC_H": str(height),
            "OR_BENCH_PROFILE": profile,
            "OR_BENCH_MODE": mode,
        }
    )
    completed = subprocess.run(
        [executable, "export_media_benchmark", "--ignored", "--nocapture"],
        cwd=ROOT,
        env=run_env,
        capture_output=True,
        text=True,
        timeout=1800,
    )
    output = completed.stdout + completed.stderr
    for match in RESULT.finditer(output):
        fields = dict(FIELD.findall(match.group("body")))
        if "fps" not in fields:
            continue
        row: dict[str, object] = {
            "profile": profile,
            "mode": mode,
            "fps": float(fields["fps"]),
            "seconds": float(fields.get("seconds", "0")),
            "frames": frames,
            "size": f"{width}x{height}",
        }
        if "bytes" in fields:
            row["bytes"] = int(fields["bytes"])
        return row
    sys.stderr.write(output[-2000:])
    return None


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--frames", type=int, default=90,
                        help="frames per row (default 90 = 3 s at 30 fps)")
    parser.add_argument("--seconds", type=float, default=4.0,
                        help="fixture length to loop to (default 4 s)")
    parser.add_argument("--size", default="1920x1080")
    parser.add_argument("--profiles", default=",".join(PROFILES))
    parser.add_argument("--fixture", default=str(FIXTURE))
    parser.add_argument("--ffmpeg", default=None)
    parser.add_argument("--cargo", default=_cargo())
    parser.add_argument("--json", default=None, help="write the rows to this path")
    parser.add_argument("--work", default=str(Path(os.environ.get("TMPDIR", "/tmp")) / "or-export-bench"))
    arguments = parser.parse_args()

    prefix = _configured_prefix()
    ffmpeg = arguments.ffmpeg or _which("ffmpeg")
    source = Path(arguments.fixture)
    if not source.exists():
        print(f"bench_export: fixture {source} is missing", file=sys.stderr)
        return 3
    work = Path(arguments.work)
    work.mkdir(parents=True, exist_ok=True)
    env = _runtime_env(prefix)
    if arguments.seconds > 0:
        if not ffmpeg:
            print(
                "bench_export: no ffmpeg found to loop the fixture; pass --ffmpeg, "
                "or pass --seconds 0 with a fixture that is already long enough",
                file=sys.stderr,
            )
            return 3
        fixture = _prepare_fixture(ffmpeg, work, arguments.seconds, source, env)
    else:
        fixture = source
    if fixture is None:
        return 3

    executable = _executable(arguments.profiles, arguments.cargo)
    if not executable:
        print("bench_export: the bridge test binary was not produced", file=sys.stderr)
        return 3

    width, height = (int(part) for part in arguments.size.lower().split("x"))
    rows: list[dict[str, object]] = []
    missing: list[str] = []
    for profile in [item.strip() for item in arguments.profiles.split(",") if item.strip()]:
        for mode in ("decode", "render", "full"):
            row = _measure(executable, env, profile, mode, fixture,
                           arguments.frames, width, height)
            if row is None:
                missing.append(f"{mode}.{profile}")
                print(f"{mode:<7} {profile:<5} NOT MEASURED")
                continue
            rows.append(row)
            print(
                f"{mode:<7} {profile:<5} {row['fps']:8.1f} fps"
                f"  {row['seconds']:7.3f} s  {row['frames']} frames {row['size']}"
                + (f"  {row['bytes']:>10} B" if "bytes" in row else "")
            )

    if arguments.json:
        Path(arguments.json).write_text(
            json.dumps(
                {
                    "frames": arguments.frames,
                    "size": arguments.size,
                    "fixture": str(fixture),
                    "rows": rows,
                    "not_measured": missing,
                },
                indent=2,
            )
            + "\n"
        )
    if missing:
        print("bench_export: not measured: " + ", ".join(missing), file=sys.stderr)
        return 3
    return 0


if __name__ == "__main__":
    sys.exit(main())
