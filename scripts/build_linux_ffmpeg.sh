#!/usr/bin/env bash
# Build and stage the pinned FFmpeg 8.1.3 shared libraries for Linux.
#
# This is the one Linux FFmpeg recipe: the hosted `platform-verification` linux
# job and a real Linux host run this same script, so a local pass and a CI pass
# mean the same thing. It expects:
#
#   OR_BUILD_ROOT          scratch directory (CI: $RUNNER_TEMP)
#   FFMPEG_CODEC_PREFIX    the libvpx/libopus/libx264 install built by
#                          scripts/build_desktop_export_codecs.sh linux
#   OR_REPO_ROOT           repository root (defaults to the current directory)
#   OR_BUILD_JOBS          make parallelism (defaults to 2, the CI value)
#
# Linux has no operating-system H.264 encoder: VideoToolbox and Media Foundation
# belong to macOS and Windows. Delivery therefore links the pinned x264 build and
# VAAPI for Intel/AMD devices, and NVENC as a driver-loaded extra. That x264 link
# makes this build GPL-2.0-or-later; the app reports the licence rather than
# hiding it.
#
# In CI it appends the resulting paths to $GITHUB_ENV/$GITHUB_PATH; locally it
# also writes $OR_BUILD_ROOT/ffmpeg-build-env.txt for the caller to import.
set -euo pipefail

repo_root="${OR_REPO_ROOT:-$(pwd)}"
build_root="${OR_BUILD_ROOT:-${RUNNER_TEMP:-}}"
if [[ -z "$build_root" ]]; then
  echo "OR_BUILD_ROOT or RUNNER_TEMP must name a scratch directory" >&2
  exit 2
fi
if [[ -z "${FFMPEG_CODEC_PREFIX:-}" ]]; then
  echo "FFMPEG_CODEC_PREFIX is required" >&2
  exit 2
fi
env_file="$build_root/ffmpeg-build-env.txt"
: > "$env_file"
record_env() {
  echo "$1" >> "$env_file"
  if [[ -n "${GITHUB_ENV:-}" ]]; then echo "$1" >> "$GITHUB_ENV"; fi
}
record_path() {
  echo "$1" >> "$build_root/ffmpeg-build-path.txt"
  if [[ -n "${GITHUB_PATH:-}" ]]; then echo "$1" >> "$GITHUB_PATH"; fi
}

# The configure argument groups live in the workflow so CI and a real host cannot
# drift apart. Hosted CI exports them from the job environment; a local run reads
# the same YAML block instead of keeping a second copy of the flags.
if [[ -z "${FFMPEG_COMMON_CONFIGURE_ARGS:-}" ]]; then
  eval "$(python3 "$repo_root/scripts/ffmpeg_configure_args.py" "$repo_root/.github/workflows/platform-verification.yml")"
fi

archive="$build_root/ffmpeg-8.1.3.tar.xz"
source="$build_root/ffmpeg-8.1.3"
prefix="$build_root/ffmpeg-8.1.3-install"
ffmpeg_source_sha512=acc7dbacde58c91bbc4a86fa589d317515d40bc9bc1ca5f9c020034bb54bf3c8cd61ed70ce437b962bd50332ea8378b3ad52da1942cfdb29e380c33a90d2f0c1
if [[ -f "$archive" ]] &&
  echo "$ffmpeg_source_sha512  $archive" | sha512sum --check --status 2>/dev/null; then
  # A retry after a transient download failure should not re-fetch a tarball
  # that is already here and already verified.
  echo "Reusing the verified FFmpeg source archive at $archive"
else
  curl --fail --location --silent --show-error --retry 10 --retry-all-errors --retry-delay 5 --max-time 600 \
    https://ffmpeg.org/releases/ffmpeg-8.1.3.tar.xz --output "$archive"
  echo "$ffmpeg_source_sha512  $archive" | sha512sum --check
fi
xz --test "$archive"
tar -xJOf "$archive" ffmpeg-8.1.3/COPYING.LGPLv2.1 > "$build_root/FFmpeg-COPYING.LGPLv2.1"
tar -xJf "$archive" -C "$build_root"
cd "$source"
read -r -a common_configure_args <<< "$FFMPEG_COMMON_CONFIGURE_ARGS"
read -r -a helper_configure_args <<< "$FFMPEG_HELPER_CONFIGURE_ARGS"
read -r -a export_configure_args <<< "$FFMPEG_EXPORT_CONFIGURE_ARGS"
# Linux has no operating-system H.264 encoder (VideoToolbox and Media Foundation
# are macOS and Windows only), so delivery links the pinned x264 build from
# scripts/build_desktop_export_codecs.sh.
export_configure_args+=(--enable-gpl --enable-libx264 --enable-encoder=libx264)
# VAAPI drives Intel and AMD devices on Linux; libva stays a system library so
# the package does not bundle one driver generation.
export_configure_args+=(--enable-vaapi --enable-hwaccel=h264_vaapi --enable-encoder=h264_vaapi)
# NVENC from the pinned headers: the driver library loads at run time, so the
# package stays self-contained and the export probe falls back to x264 on a
# machine without an NVIDIA device.
export_configure_args+=(--enable-ffnvcodec --enable-nvenc --enable-encoder=h264_nvenc)
configure_args=("--prefix=$prefix" "${common_configure_args[@]}" "${helper_configure_args[@]}" "${export_configure_args[@]}")
codec_prefix="$(cd "$FFMPEG_CODEC_PREFIX" && pwd)"
export PKG_CONFIG_PATH="$codec_prefix/lib/pkgconfig${PKG_CONFIG_PATH:+:$PKG_CONFIG_PATH}"
for package in vpx opus x264 ffnvcodec; do
  if ! pkg-config --exists "$package"; then
    echo "the codec prefix is missing $package.pc: $codec_prefix/lib/pkgconfig" >&2
    exit 2
  fi
done
./configure "${configure_args[@]}"

# Presence in a configure line is not evidence that the encoder compiled: a
# header generation or licence mismatch drops the component and leaves a build
# that cannot deliver H.264. Fail here instead of inside an export.
for component in CONFIG_LIBX264_ENCODER CONFIG_H264_VAAPI_ENCODER CONFIG_H264_NVENC_ENCODER CONFIG_LIBVPX_VP9_ENCODER CONFIG_LIBOPUS_ENCODER; do
  if ! grep -Eq "^#define $component 1$" config_components.h; then
    echo "FFmpeg did not configure $component; the delivery encoders are incomplete."
    exit 1
  fi
done
make -j"${OR_BUILD_JOBS:-2}"
make install
# Exercise the reviewed operations against a generated fixture; a textual
# capability listing alone does not prove the tools work.
test -x "$prefix/bin/ffmpeg"
test -x "$prefix/bin/ffprobe"
python3 "$repo_root/scripts/verify_ffmpeg_helpers.py" \
  --bin "$prefix/bin" \
  --lib "$prefix/lib" \
  --media "$repo_root/crates/or_media/tests/fixtures/tiny.mkv" \
  --image "$repo_root/crates/or_media/tests/fixtures/tiny_still.png" \
  --image "$repo_root/crates/or_media/tests/fixtures/tiny_still.jpg" \
  --image "$repo_root/crates/or_media/tests/fixtures/tiny_still.webp" \
  --work "$build_root/ffmpeg-helper-capability-check"
# The noise filter is verified by measuring the floor it removes, not by finding
# its name in a listing.
python3 "$repo_root/scripts/measure_denoise.py" \
  --bin "$prefix/bin" \
  --work "$build_root/ffmpeg-denoise-measurement"
export PKG_CONFIG_PATH="$prefix/lib/pkgconfig${PKG_CONFIG_PATH:+:$PKG_CONFIG_PATH}"
for library in avcodec avformat avutil swresample swscale; do
  test -f "$prefix/lib/pkgconfig/lib$library.pc"
done
record_env "FFMPEG_INSTALL_PREFIX=$prefix"
record_env "FFMPEG_CODEC_PREFIX=$codec_prefix"
record_env "PKG_CONFIG_PATH=$prefix/lib/pkgconfig"
record_path "$prefix/bin"
for library in avcodec avformat avutil swresample swscale; do
  pkg-config --modversion "lib$library"
  test "$(pkg-config --variable=prefix "lib$library")" = "$prefix"
done
{
  echo "Target: Linux $(uname -m)"
  echo "FFmpeg release: 8.1.3"
  echo "Rust binding: ffmpeg-the-third 6.0.0"
  echo "Official source URL: https://ffmpeg.org/releases/ffmpeg-8.1.3.tar.xz"
  echo "Official upstream tag commit: 1041abdc962f4cc4f394aa8de9dc5236c0c3b9e7"
  echo "Source archive SHA-512: $ffmpeg_source_sha512"
  echo "Patch status: unpatched upstream release archive"
  echo "Configure arguments: ${configure_args[*]}"
  echo "Enabled libraries: libavcodec, libavformat, libavutil, libswresample, libswscale"
  echo "Enabled programs: ffmpeg, ffprobe (9B1 packaged media helpers)"
  echo "Delivery encoders: h264_vaapi (system libva), h264_nvenc (driver loaded), libx264, libvpx-vp9, libopus"
  echo "Linux delivery encoder: libx264 (GPL-2.0-or-later), statically linked from the pinned upstream commit"
  echo "libvpx configure target: ${LIBVPX_CONFIGURE_TARGET:-ASM=auto by FFmpeg configure (no forced target)}"
  echo "License posture: GPL-2.0-or-later because of the x264 link; version3 and nonfree were not enabled"
  if [[ -f "$codec_prefix/EXPORT-CODEC-SOURCES.txt" ]]; then cat "$codec_prefix/EXPORT-CODEC-SOURCES.txt"; fi
  echo "Compiler: $(${CC:-cc} --version | head -n 1)"
  echo "Build tools: $(make --version | head -n 1); $(pkg-config --version)"
  echo "Rust toolchain: $(rustc --version 2>/dev/null || echo unknown)"
} > "$build_root/ffmpeg-linux-build-info.txt"
cat "$build_root/ffmpeg-linux-build-info.txt"
