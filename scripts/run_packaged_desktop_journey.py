#!/usr/bin/env python3
"""Run the desktop product journey twice in a clean helper environment."""

from __future__ import annotations

import hashlib
import json
import math
import os
import shutil
import subprocess
import sys
import tempfile
import time
from pathlib import Path
from urllib.error import URLError
from urllib.request import Request, urlopen


ROOT = Path(__file__).resolve().parents[1]
TEST_FILE = "integration_test/packaged_product_journey_test.dart"
TEST_NAME = "packaged desktop product journey"
REPRESENTATIVE_MEDIA_URL = (
    "https://raw.githubusercontent.com/chthomos/video-media-samples/"
    "997cb58f16bc3433652506910734be75bc64d768/"
    "big-buck-bunny-1080p-30sec.mp4"
)
REPRESENTATIVE_MEDIA_SIZE = 22_718_509
REPRESENTATIVE_MEDIA_SHA256 = (
    "07b756a4c7b481829776645c153167ca14c9df802ddbea7dffda3d817aa5261a"
)
FORBIDDEN_ENVIRONMENT = (
    "OR_FFMPEG_PATH",
    "OR_FFPROBE_PATH",
    "LD_LIBRARY_PATH",
    "DYLD_LIBRARY_PATH",
    "DYLD_FALLBACK_LIBRARY_PATH",
    "DYLD_FRAMEWORK_PATH",
    "DYLD_ROOT_PATH",
    "DYLD_INSERT_LIBRARIES",
    "FFMPEG_DIR",
    "FFMPEG_INSTALL_PREFIX",
    "PKG_CONFIG_PATH",
    "FRB_DART_LOAD_EXTERNAL_LIBRARY_NATIVE_LIB_DIR",
)


def _helper_names() -> tuple[str, ...]:
    if os.name == "nt":
        return "ffmpeg.exe", "ffprobe.exe"
    return "ffmpeg", "ffprobe"


def _guard_helper_lookup(directory: Path) -> None:
    directory.mkdir(parents=True, exist_ok=True)
    if os.name == "nt":
        inert_executable = Path(os.environ["SystemRoot"]) / "System32" / "where.exe"
        for name in _helper_names():
            shutil.copyfile(inert_executable, directory / name)
    else:
        for name in _helper_names():
            executable = directory / name
            executable.write_text(
                "#!/bin/sh\nprintf '%s\\n' 'Host FFmpeg lookup is disabled.' >&2\nexit 127\n",
                encoding="utf-8",
            )
            executable.chmod(0o755)


def _sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _download_representative_media(destination: Path) -> None:
    request = Request(
        REPRESENTATIVE_MEDIA_URL,
        headers={"User-Agent": "Opencut-Reinforced-packaged-journey"},
    )
    for attempt in range(3):
        digest = hashlib.sha256()
        size = 0
        try:
            with urlopen(request, timeout=180) as response, destination.open("wb") as output:
                while chunk := response.read(1024 * 1024):
                    size += len(chunk)
                    if size > REPRESENTATIVE_MEDIA_SIZE:
                        destination.unlink(missing_ok=True)
                        raise SystemExit(
                            "Pinned representative media exceeded its size bound."
                        )
                    digest.update(chunk)
                    output.write(chunk)
        except (OSError, URLError) as error:
            destination.unlink(missing_ok=True)
            if attempt == 2:
                raise SystemExit(
                    f"Could not fetch pinned representative media: {error}"
                ) from error
            time.sleep(2**attempt)
            continue
        break
    if (
        size != REPRESENTATIVE_MEDIA_SIZE
        or digest.hexdigest() != REPRESENTATIVE_MEDIA_SHA256
    ):
        destination.unlink(missing_ok=True)
        raise SystemExit("Pinned representative media failed its size or SHA-256 check.")


def _prepare_representative_media(
    ffmpeg: Path, source: Path, silence_fixture: Path, destination: Path
) -> None:
    completed = subprocess.run(
        [
            str(ffmpeg),
            "-hide_banner",
            "-loglevel",
            "error",
            "-nostdin",
            "-i",
            str(source),
            "-stream_loop",
            "-1",
            "-i",
            str(silence_fixture),
            "-map",
            "0:v:0",
            "-map",
            "1:a:0",
            "-map_chapters",
            "-1",
            "-c:v",
            "copy",
            "-c:a",
            "copy",
            "-t",
            "30",
            "-map_metadata",
            "-1",
            "-movflags",
            "+faststart",
            str(destination),
        ],
        capture_output=True,
        text=True,
        env=os.environ.copy(),
    )
    if completed.returncode != 0:
        raise SystemExit(
            "Packaged FFmpeg could not prepare the representative media: "
            f"{completed.stderr[-2000:]}"
        )


def _restore_macos_bridge_alias(bridge: Path) -> None:
    framework_binary = bridge.with_name("or_app_bridge")
    if not framework_binary.is_file():
        raise SystemExit(f"The macOS Rust bridge binary is missing: {framework_binary}")
    if bridge.is_symlink():
        if bridge.resolve() != framework_binary.resolve():
            raise SystemExit(f"The macOS Rust bridge alias points to the wrong file: {bridge}")
        return
    if bridge.exists():
        raise SystemExit(f"The macOS Rust bridge alias is not a symlink: {bridge}")
    bridge.symlink_to(framework_binary.name)


def _snapshot_linux_media_runtime(
    helpers: Path, bridge_directory: Path, destination: Path
) -> Path:
    if destination.exists():
        shutil.rmtree(destination)
    destination.mkdir(parents=True)
    library_directory = destination / "lib"
    library_directory.mkdir()
    for helper_name in _helper_names():
        shutil.copy2(helpers / helper_name, destination / helper_name)
    for pattern in (
        "libavcodec.so*",
        "libavfilter.so*",
        "libavformat.so*",
        "libavutil.so*",
        "libswresample.so*",
        "libswscale.so*",
    ):
        matches = tuple(bridge_directory.glob(pattern))
        if not matches:
            raise SystemExit(f"The packaged media runtime is missing {pattern}.")
        for library in matches:
            shutil.copy2(
                library,
                library_directory / library.name,
                follow_symlinks=False,
            )
    return destination


def _run(command: list[str], cwd: Path, env: dict[str, str]) -> subprocess.CompletedProcess:
    if os.name == "nt" and command[0].lower().endswith((".bat", ".cmd")):
        return subprocess.run(
            subprocess.list2cmdline(command), cwd=cwd, env=env, shell=True
        )
    return subprocess.run(command, cwd=cwd, env=env)


def main() -> int:
    flutter = shutil.which("flutter")
    if flutter is None:
        raise SystemExit("Flutter is not available on the runner PATH.")
    helper_directory = Path(
        os.environ.get("OR_PACKAGED_HELPERS_DIRECTORY", "")
    ).resolve()
    ffmpeg = helper_directory / ("ffmpeg.exe" if os.name == "nt" else "ffmpeg")
    bridge_directory = Path(
        os.environ.get("OR_PACKAGED_BRIDGE_DIRECTORY", "")
    ).resolve()
    ffprobe = helper_directory / ("ffprobe.exe" if os.name == "nt" else "ffprobe")
    if not ffmpeg.is_file() or not ffprobe.is_file():
        raise SystemExit(f"The packaged FFmpeg helpers are missing: {helper_directory}")
    bridge_name = {
        "nt": "or_app_bridge.dll",
        "posix": "libor_app_bridge.dylib"
        if sys.platform == "darwin"
        else "libor_app_bridge.so",
    }[os.name]
    bridge = bridge_directory / bridge_name
    if not bridge.is_file():
        raise SystemExit(f"The packaged Rust bridge is missing: {bridge}")

    runner_temp = Path(os.environ.get("RUNNER_TEMP", tempfile.gettempdir()))
    work = runner_temp / "or-packaged-product-journey"
    if work.exists():
        shutil.rmtree(work)
    work.mkdir(parents=True)
    path_guard = work / "host-helper-lookup-disabled"
    _guard_helper_lookup(path_guard)
    guarded_path = os.pathsep.join(
        (str(path_guard), os.environ.get("PATH", ""))
    )
    for helper in ("ffmpeg", "ffprobe"):
        resolved = shutil.which(helper, path=guarded_path)
        if resolved is None or Path(resolved).resolve().parent != path_guard.resolve():
            raise SystemExit(f"Host {helper} lookup is not blocked by the acceptance PATH.")
        guarded = subprocess.run([resolved, "--version"], capture_output=True)
        if guarded.returncode == 0:
            raise SystemExit(f"The inert host {helper} guard unexpectedly succeeded.")

    media = work / "big-buck-bunny.mp4"
    representative_source = work / "big-buck-bunny-source.mp4"
    _download_representative_media(representative_source)
    _prepare_representative_media(
        ffmpeg,
        representative_source,
        ROOT / "crates/or_media/tests/fixtures/big_buck_bunny_1080p_h264_aac.mp4",
        media,
    )
    input_probe = subprocess.run(
        [
            str(ffprobe),
            "-v",
            "error",
            "-show_entries",
            "format=format_name,duration,size:stream=codec_type,codec_name,width,height,channels",
            "-of",
            "json",
            str(media),
        ],
        capture_output=True,
        text=True,
        env=os.environ.copy(),
        timeout=30,
    )
    if input_probe.returncode != 0:
        raise SystemExit(
            "Packaged ffprobe could not inspect the prepared representative media: "
            f"{input_probe.stderr[-2000:]}"
        )
    input_probe_result = json.loads(input_probe.stdout)
    representative_duration = float(input_probe_result["format"].get("duration", "0"))
    input_streams = input_probe_result["streams"]
    input_video = next(
        (stream for stream in input_streams if stream["codec_type"] == "video"), None
    )
    input_audio = next(
        (stream for stream in input_streams if stream["codec_type"] == "audio"), None
    )
    if (
        input_video is None
        or "mp4" not in input_probe_result["format"].get("format_name", "").split(",")
        or input_video.get("codec_name") != "h264"
        or (input_video.get("width"), input_video.get("height")) != (1920, 1080)
        or input_audio is None
        or input_audio.get("codec_name") != "aac"
        or input_audio.get("channels") != 6
        or representative_duration < 29.9
    ):
        raise SystemExit(
            "Packaged helpers did not prepare the pinned 30-second H.264/AAC test media: "
            f"{input_probe_result}"
        )
    reimport_timeline_start = f"{math.ceil(representative_duration)}/1"
    replacement_media = work / "relinked-bunny.mp4"
    shutil.copyfile(media, replacement_media)
    unsupported = work / "unsupported.mp4"
    unsupported.write_bytes(b"not an OR supported media file\n")
    missing = work / "missing-source.mkv"
    bad_project = work / "invalid-project.orproj"
    bad_project.write_bytes(b"this is not an OR project\n")
    project = work / "journey.orproj"
    export = work / "existing-export.webm"
    export.write_bytes(b"existing destination must be replaced\n")
    failed_export = work / "failed-export.webm"
    failed_export.mkdir()
    failure_sentinel = failed_export / "preserve-me.txt"
    failure_sentinel.write_text(
        "failed replacement must preserve this\n", encoding="utf-8"
    )

    env = os.environ.copy()
    env.pop("OR_PACKAGED_HELPERS_DIRECTORY", None)
    for name in FORBIDDEN_ENVIRONMENT:
        env.pop(name, None)
    env["PATH"] = guarded_path
    # flutter_rust_bridge otherwise resolves its developer target/release path.
    # Point the acceptance app at the bridge inside the built package; its
    # app-relative runtime paths then resolve only packaged media libraries.
    env["FRB_DART_LOAD_EXTERNAL_LIBRARY_NATIVE_LIB_DIR"] = str(bridge_directory)
    if sys.platform.startswith("linux"):
        runtime_snapshot = Path(
            os.environ.get("OR_PACKAGED_MEDIA_RUNTIME_DIRECTORY", "")
        ).resolve()
        if not runtime_snapshot.is_dir():
            raise SystemExit(
                "The Linux packaged media runtime staging directory is missing: "
                f"{runtime_snapshot}"
            )
        runtime_snapshot = _snapshot_linux_media_runtime(
            helper_directory, bridge_directory, runtime_snapshot
        )
        # Flutter's Linux install step clears the bundle. The project CMake
        # install hook restores this already-packaged payload.
        env["OR_PACKAGED_MEDIA_RUNTIME_DIRECTORY"] = str(runtime_snapshot)
    env.update(
        {
            "OR_PACKAGED_JOURNEY_PROJECT": str(project),
            "OR_PACKAGED_JOURNEY_MEDIA": str(media),
            "OR_PACKAGED_JOURNEY_REIMPORT_START": reimport_timeline_start,
            "OR_PACKAGED_JOURNEY_REPLACEMENT_MEDIA": str(replacement_media),
            "OR_PACKAGED_JOURNEY_UNSUPPORTED_MEDIA": str(unsupported),
            "OR_PACKAGED_JOURNEY_MISSING_MEDIA": str(missing),
            "OR_PACKAGED_JOURNEY_BAD_PROJECT": str(bad_project),
            "OR_PACKAGED_JOURNEY_EXPORT": str(export),
            "OR_PACKAGED_JOURNEY_FAILED_EXPORT": str(failed_export),
        }
    )

    if sys.platform == "darwin":
        device = "macos"
    elif sys.platform == "win32":
        device = "windows"
    elif sys.platform.startswith("linux"):
        device = "linux"
    else:
        raise SystemExit(f"Unsupported acceptance host: {sys.platform}")

    invocations: list[dict[str, object]] = []
    xvfb = shutil.which("xvfb-run") if device == "linux" else None
    if device == "linux" and xvfb is None:
        raise SystemExit("xvfb-run is required for the Linux desktop journey.")
    for phase in ("create", "reopen"):
        phase_env = env | {"OR_PACKAGED_JOURNEY_PHASE": phase}
        command = [
            flutter,
            "--ci",
            "test",
            TEST_FILE,
            "-d",
            device,
            "--plain-name",
            TEST_NAME,
        ]
        if xvfb is not None:
            command = [xvfb, "-a", *command]
        completed = _run(command, ROOT / "apps/or_app", phase_env)
        invocations.append({"phase": phase, "exit_code": completed.returncode})
        if completed.returncode != 0:
            return completed.returncode
        if phase == "create" and not project.is_file():
            raise SystemExit("The first app process did not save its project.")
        if phase == "reopen" and not export.is_file():
            raise SystemExit("The relaunched app did not write its export.")

    # Failure paths use fresh processes so the missing source can be checked
    # with the shipped probe before the missing-probe override is introduced.
    before_failures = _sha256(project)
    for phase in ("missing-source", "missing-probe"):
        phase_env = env | {"OR_PACKAGED_JOURNEY_PHASE": phase}
        if phase == "missing-probe":
            phase_env["OR_FFPROBE_PATH"] = str(work / "missing-packaged-ffprobe")
        command = [
            flutter,
            "--ci",
            "test",
            TEST_FILE,
            "-d",
            device,
            "--plain-name",
            TEST_NAME,
        ]
        if xvfb is not None:
            command = [xvfb, "-a", *command]
        completed = _run(command, ROOT / "apps/or_app", phase_env)
        invocations.append({"phase": phase, "exit_code": completed.returncode})
        if completed.returncode != 0:
            return completed.returncode
    if device == "macos":
        # Flutter rebuilds the framework during each test phase and removes the
        # unversioned alias restored by the workflow before this journey.
        _restore_macos_bridge_alias(bridge)
    if _sha256(project) != before_failures:
        raise SystemExit("A failed import or reopen changed the saved project.")
    if not failure_sentinel.is_file() or failure_sentinel.read_text(
        encoding="utf-8"
    ) != "failed replacement must preserve this\n":
        raise SystemExit("The failed export replacement changed its existing destination.")
    if {path.name for path in failed_export.iterdir()} != {failure_sentinel.name}:
        raise SystemExit("The failed export replacement left unexpected destination contents.")

    probe = subprocess.run(
        [
            str(ffprobe),
            "-v",
            "error",
            "-show_entries",
            "format=format_name,duration,size:stream=codec_type,codec_name,width,height,sample_rate",
            "-of",
            "json",
            str(export),
        ],
        check=True,
        capture_output=True,
        text=True,
        env={key: value for key, value in env.items() if not key.startswith("DYLD_") and key != "LD_LIBRARY_PATH"},
    )
    probe_result = json.loads(probe.stdout)
    streams = probe_result["streams"]
    video = next((stream for stream in streams if stream["codec_type"] == "video"), None)
    audio = next((stream for stream in streams if stream["codec_type"] == "audio"), None)
    if video is None or video.get("codec_name") != "vp9":
        raise SystemExit(f"Export does not contain the required VP9 video stream: {streams}")
    if audio is None or audio.get("codec_name") != "opus":
        raise SystemExit(f"Export does not contain the required Opus audio stream: {streams}")
    if (video.get("width"), video.get("height")) != (1920, 1080):
        raise SystemExit(f"Export did not preserve the real media's 1080p dimensions: {video}")
    export_format = probe_result["format"]
    if "webm" not in export_format.get("format_name", ""):
        raise SystemExit("Export is not an independently recognizable WebM file.")
    if float(export_format.get("duration", "0")) <= 0:
        raise SystemExit("Export has no playable duration.")
    if float(export_format.get("duration", "0")) < 29.9:
        raise SystemExit("The WebM export did not retain the full representative edit.")
    if export.stat().st_size >= media.stat().st_size * 0.8:
        raise SystemExit(
            "The WebM export did not reduce the representative source size by 20%."
        )

    helper_env = {
        key: value
        for key, value in env.items()
        if not key.startswith("DYLD_") and key != "LD_LIBRARY_PATH"
    }
    full_decode = subprocess.run(
        [
            str(ffprobe),
            "-v",
            "error",
            "-count_frames",
            "-show_entries",
            "stream=codec_type,nb_read_frames",
            "-of",
            "json",
            str(export),
        ],
        capture_output=True,
        text=True,
        env=helper_env,
        timeout=180,
    )
    if full_decode.returncode != 0:
        raise SystemExit(
            "The packaged ffprobe could not fully decode the exported video and audio: "
            f"{full_decode.stderr}"
        )
    decoded_streams = json.loads(full_decode.stdout)["streams"]
    decoded_frame_counts = {
        stream["codec_type"]: int(stream.get("nb_read_frames", "0"))
        for stream in decoded_streams
    }
    if not all(decoded_frame_counts.get(kind, 0) > 0 for kind in ("video", "audio")):
        raise SystemExit(
            "The packaged ffprobe did not decode frames from both exported streams: "
            f"{decoded_frame_counts}"
        )
    if decoded_frame_counts.get("video", 0) < 719:
        raise SystemExit(
            "The packaged output did not decode all 720 representative video frames: "
            f"{decoded_frame_counts}"
        )
    report = {
        "platform": device,
        "host_ffmpeg_path_lookup": "blocked",
        "host_ffprobe_path_lookup": "blocked",
        "rust_bridge": str(bridge),
        "rust_bridge_sha256": _sha256(bridge),
        "ffprobe": str(ffprobe),
        "ffprobe_sha256": _sha256(ffprobe),
        "ffmpeg": str(ffmpeg),
        "ffmpeg_sha256": _sha256(ffmpeg),
        "project_sha256_after_failures": _sha256(project),
        "export_sha256": _sha256(export),
        "export_bytes": export.stat().st_size,
        "source_bytes": media.stat().st_size,
        "export_source_size_ratio": export.stat().st_size / media.stat().st_size,
        "representative_media_sha256": REPRESENTATIVE_MEDIA_SHA256,
        "export_streams": streams,
        "export_format": export_format,
        "export_full_decode": decoded_frame_counts,
        "app_processes": invocations,
    }
    report_path = runner_temp / f"or-packaged-product-journey-{device}.json"
    report_path.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(report, indent=2))
    summary = os.environ.get("GITHUB_STEP_SUMMARY")
    if summary:
        with open(summary, "a", encoding="utf-8") as output:
            output.write(f"\n### Packaged desktop journey — {device}\n\n")
            output.write("- PATH resolves `ffmpeg` and `ffprobe` only to inert failure guards.\n")
            output.write(
                "- The app was started in separate create, reopen/export, "
                "missing-source, and missing-probe processes.\n"
            )
            output.write(
                f"- Export: `{export.stat().st_size}` bytes at {video['width']}x{video['height']}; "
                "WebM/VP9/Opus independently probed and fully decoded by packaged ffprobe "
                f"(frames: {decoded_frame_counts}).\n"
            )
            output.write(
                f"- Representative source: 30 seconds, 1080p H.264/AAC; "
                f"{media.stat().st_size} bytes. Export/source size ratio: "
                f"{export.stat().st_size / media.stat().st_size:.3f}.\n"
            )
            output.write(f"- Evidence report: `{report_path}`\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
