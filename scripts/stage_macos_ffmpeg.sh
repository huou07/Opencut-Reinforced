#!/usr/bin/env bash
# Stage the pinned LGPL-only FFmpeg 8.1.3 prefix used by the macOS app.
#
#   bash scripts/stage_macos_ffmpeg.sh [--prefix DIR] [--work DIR] [--force]
#
# Why this exists: the app refuses a runtime whose license is not exactly
# "LGPL version 2.1 or later", and Homebrew's FFmpeg is a GPL build. A local
# package built against Homebrew therefore starts, then fails the first time a
# viewer frame or an export asks for the linked runtime. This script produces
# the same prefix the hosted macOS lane uses: pinned FFmpeg source, pinned
# libvpx/libopus, the packaged-helper profile, and VideoToolbox H.264/AAC for
# MP4 delivery.
#
# Output (default under the user cache, never in the repository):
#   ffmpeg-install/                 the prefix packaging consumes
#   ffmpeg-install/BUILD-INFO.txt   pins, configure arguments and license
#   ffmpeg-8.1.3.tar.xz             verified source archive
#   ffmpeg-8.1.3/                   extracted source tree
#   ffmpeg-codecs/                  pinned static libvpx/libopus prefix
#
# Exit codes: 0 prefix is ready; 1 a build or verification step failed;
#             2 the invocation is wrong; 3 a host precondition is missing.
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
FFMPEG_VERSION=8.1.3
FFMPEG_URL="https://ffmpeg.org/releases/ffmpeg-$FFMPEG_VERSION.tar.xz"
FFMPEG_SHA512=acc7dbacde58c91bbc4a86fa589d317515d40bc9bc1ca5f9c020034bb54bf3c8cd61ed70ce437b962bd50332ea8378b3ad52da1942cfdb29e380c33a90d2f0c1
DEPLOYMENT_TARGET="${MACOSX_DEPLOYMENT_TARGET:-12.0}"
CACHE_ROOT="${OR_FFMPEG_CACHE:-$HOME/Library/Caches/OpencutReinforced/ffmpeg-$FFMPEG_VERSION-macos-image-support}"
prefix="$CACHE_ROOT/ffmpeg-install"
work="$CACHE_ROOT"
force=0

while (( $# )); do
  case "$1" in
    --prefix) (( $# >= 2 )) || { echo "stage_macos_ffmpeg: --prefix needs a value" >&2; exit 2; }; prefix="$2"; shift 2 ;;
    --work) (( $# >= 2 )) || { echo "stage_macos_ffmpeg: --work needs a value" >&2; exit 2; }; work="$2"; shift 2 ;;
    --force) force=1; shift ;;
    -h|--help) sed -n '2,32p' "${BASH_SOURCE[0]}"; exit 0 ;;
    *) echo "stage_macos_ffmpeg: unknown argument: $1" >&2; exit 2 ;;
  esac
done

fail() { echo "stage_macos_ffmpeg: $*" >&2; exit 1; }
blocked() { echo "stage_macos_ffmpeg: BLOCKED: $*" >&2; exit 3; }

[[ "$(uname -s)" == Darwin ]] || blocked "this script stages the macOS app runtime"
[[ "$(uname -m)" == arm64 ]] || blocked "the distributable app is Apple Silicon only"
for tool in curl shasum xz tar make pkg-config python3 cmake git clang; do
  command -v "$tool" >/dev/null 2>&1 || blocked "missing tool on PATH: $tool"
done

mkdir -p "$work/ffmpeg-codecs"

license_of() {
  python3 - "$1" <<'PY'
import ctypes
import sys

library = ctypes.CDLL(sys.argv[1])
library.avutil_license.restype = ctypes.c_char_p
print(library.avutil_license().decode())
PY
}

verify_prefix() {
  [[ -x "$prefix/bin/ffmpeg" && -x "$prefix/bin/ffprobe" ]] || return 1
  python3 - "$prefix" "$DEPLOYMENT_TARGET" <<'PY'
import re
import subprocess
import sys
from pathlib import Path

prefix = Path(sys.argv[1])
target = tuple(int(part) for part in sys.argv[2].split(".")[:2])
paths = [prefix / "bin/ffmpeg", prefix / "bin/ffprobe"]
paths += sorted(prefix.glob("lib/libav*.dylib"))
for path in paths:
    if not path.exists():
        continue
    output = subprocess.run(
        ["otool", "-l", str(path)], capture_output=True, text=True, check=True
    ).stdout
    match = re.search(r"LC_BUILD_VERSION.*?\n.*?minos\s+([0-9.]+)", output, re.S)
    if match is None:
        match = re.search(r"LC_VERSION_MIN_MACOSX.*?\n.*?version\s+([0-9.]+)", output, re.S)
    if match is None:
        raise SystemExit(f"no deployment target in {path}")
    actual = tuple(int(part) for part in match.group(1).split(".")[:2])
    if actual > target:
        raise SystemExit(
            f"{path.name} requires macOS {match.group(1)}, expected {sys.argv[2]}"
        )
PY
  if [[ $? -ne 0 ]]; then return 1; fi
  [[ -f "$prefix/lib/pkgconfig/libavutil.pc" ]] || return 1
  local license
  license="$(DYLD_LIBRARY_PATH="$prefix/lib" license_of "$prefix/lib/libavutil.dylib")" || return 1
  [[ "$license" == "LGPL version 2.1 or later" ]] || return 1
  local configuration
  configuration="$(DYLD_LIBRARY_PATH="$prefix/lib" "$prefix/bin/ffmpeg" -version 2>/dev/null | sed -n 's/^ *configuration: //p')"
  [[ -n "$configuration" ]] || return 1
  [[ "$configuration" != *--enable-gpl* && "$configuration" != *--enable-nonfree* && "$configuration" != *--enable-version3* ]] || return 1
  return 0
}

if (( force == 0 )) && verify_prefix; then
  echo "$prefix"
  exit 0
fi

archive="$work/ffmpeg-$FFMPEG_VERSION.tar.xz"
source="$work/ffmpeg-$FFMPEG_VERSION"
if [[ ! -f "$archive" ]]; then
  curl --fail --location --silent --show-error --retry 10 --retry-all-errors \
    --retry-delay 5 --max-time 600 "$FFMPEG_URL" --output "$archive"
fi
actual="$(shasum -a 512 "$archive" | awk '{print $1}')"
[[ "$actual" == "$FFMPEG_SHA512" ]] || fail "FFmpeg source SHA-512 mismatch: $actual"
xz --test "$archive"
[[ -d "$source" ]] || tar -xJf "$archive" -C "$work"

export MACOSX_DEPLOYMENT_TARGET="$DEPLOYMENT_TARGET"
bash "$ROOT/scripts/build_desktop_export_codecs.sh" macos \
  "$work/ffmpeg-codecs" "$work/codec-work"

deployment_flag="-mmacosx-version-min=$DEPLOYMENT_TARGET"

configure_args=(
  "--prefix=$prefix"
  --install-name-dir=@rpath
  --disable-static --enable-shared --disable-programs --disable-doc
  --disable-autodetect --disable-everything --disable-asm
  --disable-avdevice --disable-avfilter
  --enable-protocol=file --enable-protocol=pipe
  --enable-demuxer=matroska,mov,wav,mp3,ogg,flac,aiff,avi,image2,image2pipe,png_pipe,jpeg_pipe,webp_pipe
  --enable-muxer=matroska --enable-muxer=mov --enable-muxer=mp4 --enable-muxer=webm --enable-muxer=image2
  --enable-muxer=image2pipe --enable-muxer=wav
  --enable-decoder=ffv1,pcm_s16le,pcm_s24le,pcm_s32le,pcm_f32le,pcm_s24be,pcm_s16be,h264,hevc,av1,vp8,vp9,prores,aac,mp3,mp3float,opus,vorbis,flac,alac,ac3,eac3,mpeg4,png,mjpeg,webp
  --enable-parser=h264,hevc,av1,vp8,vp9,vorbis,flac,ac3,mpegaudio,aac,mp3
  --enable-encoder=ffv1,pcm_s16le,png,mpeg4,aac
  --enable-ffmpeg --enable-ffprobe --enable-avfilter --enable-zlib
  --enable-filter=scale,setpts,format,aformat,aresample,showwavespic,afftdn
  --pkg-config-flags=--static --enable-libvpx --enable-libopus
  --enable-encoder=libvpx_vp9,libopus
  --enable-videotoolbox --enable-encoder=h264_videotoolbox
  "--extra-cflags=$deployment_flag"
  "--extra-cxxflags=$deployment_flag"
  "--extra-ldflags=$deployment_flag"
)

export PKG_CONFIG_PATH="$work/ffmpeg-codecs/lib/pkgconfig"
cd "$source"
if [[ -f Makefile ]]; then make clean; fi
./configure "${configure_args[@]}"
make -j2
make install

# The bundle must carry the license texts for everything statically linked into
# the FFmpeg shared libraries, not only FFmpeg's own notice.
cp "$source/COPYING.LGPLv2.1" "$prefix/COPYING.LGPLv2.1"
mkdir -p "$prefix/ThirdPartyNotices/libvpx" "$prefix/ThirdPartyNotices/libopus"
cp "$work/codec-work/LIBVPX-LICENSE" "$prefix/ThirdPartyNotices/libvpx/LICENSE"
cp "$work/codec-work/LIBVPX-PATENTS" "$prefix/ThirdPartyNotices/libvpx/PATENTS"
cp "$work/codec-work/OPUS-COPYING" "$prefix/ThirdPartyNotices/libopus/COPYING"

verify_prefix || fail "the built prefix did not verify as the approved LGPL runtime"

{
  echo "Target: macOS $(uname -m), deployment target $DEPLOYMENT_TARGET"
  echo "FFmpeg release: $FFMPEG_VERSION"
  echo "Official source URL: $FFMPEG_URL"
  echo "Source archive SHA-512: $FFMPEG_SHA512"
  echo "Patch status: unpatched upstream release archive"
  echo "Configure arguments: ${configure_args[*]}"
  echo "License: $(license_of "$prefix/lib/libavutil.dylib")"
  echo "Runtime: app-bundled shared libraries; app refuses any other license"
  cat "$work/codec-work/EXPORT-CODEC-SOURCES.txt" 2>/dev/null || true
} > "$prefix/BUILD-INFO.txt"

echo "$prefix"
