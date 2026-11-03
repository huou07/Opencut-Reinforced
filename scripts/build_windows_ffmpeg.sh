#!/usr/bin/env bash
# Build and stage the pinned LGPL-only FFmpeg 8.1.3 shared libraries for Windows.
#
# This is the one Windows FFmpeg recipe: the hosted `platform-verification`
# windows job and a real Windows host run this same script, so a local pass and a
# CI pass mean the same thing. It must run under MSYS2 (CLANG64) bash with the
# MSVC x64 environment already imported, and it expects:
#
#   OR_BUILD_ROOT          scratch directory (CI: $RUNNER_TEMP)
#   FFMPEG_CODEC_PREFIX    the libvpx/libopus install built by
#                          scripts/build_desktop_export_codecs.sh windows
#   OR_ZLIB_PREFIX         the static zlib install built by
#                          scripts/build_windows_zlib.ps1
#   VCToolsInstallDir      MSVC toolset root (set by vcvarsall/msvc-dev-cmd)
#   OR_REPO_ROOT           repository root (defaults to the current directory)
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
if [[ -z "${FFMPEG_CODEC_PREFIX:-}" || -z "${OR_ZLIB_PREFIX:-}" || -z "${VCToolsInstallDir:-}" ]]; then
  echo "FFMPEG_CODEC_PREFIX, OR_ZLIB_PREFIX and VCToolsInstallDir are required" >&2
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

temp="$(cygpath -u "$build_root")"
archive="$temp/ffmpeg-8.1.3.tar.xz"
source="$temp/ffmpeg-8.1.3"
prefix="$temp/ffmpeg-8.1.3-install"
prefix_win="$(cygpath -am "$prefix")"
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
tar -xJOf "$archive" ffmpeg-8.1.3/COPYING.LGPLv2.1 > "$temp/FFmpeg-COPYING.LGPLv2.1"
tar -xJf "$archive" -C "$temp"
cd "$source"
read -r -a common_configure_args <<< "$FFMPEG_COMMON_CONFIGURE_ARGS"
read -r -a helper_configure_args <<< "$FFMPEG_HELPER_CONFIGURE_ARGS"
read -r -a export_configure_args <<< "$FFMPEG_EXPORT_CONFIGURE_ARGS"
configure_args=("--prefix=$prefix_win" --toolchain=msvc --target-os=win64 --arch=x86_64 "${common_configure_args[@]}" "${helper_configure_args[@]}" "${export_configure_args[@]}")
# Media Foundation is Windows' own H.264 encoder. It needs no extra library and
# no extra licence, so the MP4 export profile uses it here instead of the Apple
# VideoToolbox encoder that only exists in the macOS package.
configure_args+=("--enable-mediafoundation" "--enable-encoder=h264_mf")
# MSVC lacks the C23 stdbit.h header. Expose FFmpeg's own compatible
# implementation during configure so compiler feature checks can use it.
stdbit_compat="$(cygpath -am "$source/compat/stdbit")"
test -f "$source/compat/stdbit/stdbit.h"
configure_args+=("--extra-cflags=-I$stdbit_compat")
# MSYS2's /usr/bin also contains link.exe, but FFmpeg's MSVC wrapper
# falls back to PATH lookup when bare `cl` is unavailable. Put the
# adjacent Visual Studio linker first so that probe links use MSVC.
msvc_linker_dir="$(cygpath -u "$VCToolsInstallDir")"
msvc_linker_dir="${msvc_linker_dir%/}/bin/HostX64/x64"
export PATH="$msvc_linker_dir:$PATH"
test "$(command -v link.exe)" = "$msvc_linker_dir/link.exe"
# FFmpeg configure discovers zlib through pkg-config. CMake's MSVC
# install provides the static archive and headers but no .pc file.
zlib_prefix="$(cygpath -u "$OR_ZLIB_PREFIX")"
test -f "$zlib_prefix/lib/zlib.lib"
mkdir -p "$prefix/lib/pkgconfig"
cat > "$prefix/lib/pkgconfig/zlib.pc" <<EOF
prefix=$zlib_prefix
exec_prefix=\${prefix}
libdir=\${prefix}/lib
includedir=\${prefix}/include

Name: zlib
Description: zlib compression library
Version: 1.3.2
Libs: -L\${libdir} -lz
Cflags: -I\${includedir}
EOF
# Reconstruct the codec path in this MSYS process rather than
# inheriting the value exported across prior Windows shell steps.
# The codec build validates its own pkg-config environment, but this
# FFmpeg configure process must independently see the same packages.
codec_pkgconfig="$(cygpath -u "$FFMPEG_CODEC_PREFIX")/lib/pkgconfig"
test -f "$codec_pkgconfig/opus.pc"
test -f "$codec_pkgconfig/vpx.pc"
export PKG_CONFIG_PATH="$prefix/lib/pkgconfig:$codec_pkgconfig:/clang64/lib/pkgconfig:/clang64/share/pkgconfig"
test "$(pkg-config --modversion opus)" = "1.6.1"
test "$(pkg-config --modversion vpx)" = "1.17.0"
pkg-config --static --libs opus vpx
test "$(pkg-config --modversion zlib)" = "1.3.2"
pkg-config --cflags --libs zlib
# zlib is statically linked into the MSVC helper executables.
cp "$build_root/ZLIB-LICENSE.txt" "$prefix/ZLIB-LICENSE.txt"
./configure "${configure_args[@]}"
# Codec component selections are emitted in config_components.h;
# config.h contains FFmpeg's build/platform configuration instead.
if ! grep -Eq '^#define CONFIG_H264_MF_ENCODER 1$' config_components.h; then
  echo "FFmpeg did not configure the Media Foundation H.264 encoder, so the"
  echo "Windows package could not export MP4. Configure probe output:"
  grep -n -B 8 -A 16 -i -E 'h264_mf|mfenc|mediafoundation|mfapi' ffbuild/config.log | tail -n 160 || true
  exit 1
fi
if ! grep -Eq '^#define CONFIG_LIBVPX_VP9_ENCODER 1$' config_components.h; then
  echo "FFmpeg did not configure the required libvpx VP9 encoder."
  echo "FFmpeg libvpx configure probes:"
  grep -n -B 8 -A 16 -i -E 'libvpx_vp9_encoder|vpx_codec_vp9_cx|VPX_IMG_FMT_HIGHBITDEPTH|vpxmd|LNK[0-9]{4}' ffbuild/config.log | tail -n 180 || true
  exit 1
fi
make -j2
make install
# The install bin directory holds helper DLLs. Exercise the actual
# reviewed operations before testing the app-relative package.
test -x "$prefix/bin/ffmpeg.exe"
test -x "$prefix/bin/ffprobe.exe"
python3 "$repo_root/scripts/verify_ffmpeg_helpers.py" \
  --bin "$prefix/bin" \
  --lib "$prefix/lib" \
  --media "$repo_root/crates/or_media/tests/fixtures/tiny.mkv" \
  --image "$repo_root/crates/or_media/tests/fixtures/tiny_still.png" \
  --image "$repo_root/crates/or_media/tests/fixtures/tiny_still.jpg" \
  --image "$repo_root/crates/or_media/tests/fixtures/tiny_still.webp" \
  --work "$build_root/ffmpeg-helper-capability-check"
# Phase 4's noise filter is verified by measuring the floor it removes,
# not by finding its name in a listing.
python3 "$repo_root/scripts/measure_denoise.py" \
  --bin "$prefix/bin" \
  --work "$build_root/ffmpeg-denoise-measurement"
export PKG_CONFIG_PATH="$prefix/lib/pkgconfig${PKG_CONFIG_PATH:+:$PKG_CONFIG_PATH}"
for library in avcodec avformat avutil swresample swscale; do
  test -f "$prefix/lib/pkgconfig/lib$library.pc"
done
for library in avcodec avformat avutil swresample swscale; do
  if [ -f "$prefix/lib/$library.lib" ]; then continue; fi
  import_library="$(find "$prefix/lib" "$prefix/bin" -maxdepth 1 -type f -name "$library*.lib" -print -quit)"
  test -n "$import_library"
  cp "$import_library" "$prefix/lib/$library.lib"
done
workspace="$(cygpath -u "$repo_root")"
libclang_path="$(cygpath -am /clang64/bin)"
mkdir -p "$workspace/.cargo"
# Project config is discovered even when Flutter's isolated hook filters CARGO_HOME.
cat > "$workspace/.cargo/config.toml" <<EOF
[env]
FFMPEG_DIR = { value = "$prefix_win", force = true }
PKG_CONFIG_PATH = { value = "$prefix_win/lib/pkgconfig", force = true }
LIBCLANG_PATH = { value = "$libclang_path", force = true }
EOF
record_env "FFMPEG_INSTALL_PREFIX=$prefix_win"
codec_prefix_win="$(cygpath -am "$FFMPEG_CODEC_PREFIX")"
record_env "FFMPEG_CODEC_PREFIX=$codec_prefix_win"
record_env "LIBCLANG_PATH=$libclang_path"
record_env "INCLUDE=$prefix_win/include;$INCLUDE"
record_env "LIB=$prefix_win/lib;$prefix_win/bin;$LIB"
record_path "$prefix_win/bin"
for library in avcodec avformat avutil swresample swscale; do
  pkg-config --modversion "lib$library"
  pkg_config_prefix="$(pkg-config --variable=prefix "lib$library" | tr '\\' '/')"
  test "${pkg_config_prefix,,}" = "${prefix_win,,}"
done
{
  echo "Target: Windows x86_64 (MSVC)"
  echo "FFmpeg release: 8.1.3"
  echo "zlib release: 1.3.2 (statically linked into Windows ffmpeg.exe; zs.lib exposed as zlib.lib for FFmpeg configure)"
  echo "zlib source URL: https://github.com/madler/zlib/releases/download/v1.3.2/zlib-1.3.2.tar.gz"
  echo "zlib source SHA-256: bb329a0a2cd0274d05519d61c667c062e06990d72e125ee2dfa8de64f0119d16"
  echo "Rust binding: ffmpeg-the-third 6.0.0"
  echo "Official source URL: https://ffmpeg.org/releases/ffmpeg-8.1.3.tar.xz"
  echo "Official upstream tag commit: 1041abdc962f4cc4f394aa8de9dc5236c0c3b9e7"
  echo "Source archive SHA-512: acc7dbacde58c91bbc4a86fa589d317515d40bc9bc1ca5f9c020034bb54bf3c8cd61ed70ce437b962bd50332ea8378b3ad52da1942cfdb29e380c33a90d2f0c1"
  echo "Patch status: unpatched upstream release archive"
  echo "Packaging modifications: exposes MSVC import-library aliases and copies DLLs beside the desktop media probe; preserved install prefix remains otherwise unchanged"
  echo "Configure arguments: ${configure_args[*]}"
  echo "Enabled libraries: libavcodec, libavformat, libavutil, libswresample, libswscale"
  echo "Enabled programs: ffmpeg, ffprobe (9B1 packaged media helpers)"
  echo "Enabled protocols: file, pipe"
  echo "Enabled demuxers: matroska, mov, image2, image2pipe, png_pipe, jpeg_pipe, webp_pipe"
  echo "Enabled decoders: ffv1, pcm_s16le, h264, aac, mp3, vp9, opus, png, mjpeg, webp"
  echo "Enabled parsers: h264, aac"
  echo "Enabled encoders: ffv1, pcm_s16le, png, mpeg4"
  echo "Delivery encoders: libvpx-vp9 and libopus, statically linked from pinned upstream sources"
  echo "libvpx configure target: ${LIBVPX_CONFIGURE_TARGET:-ASM=auto by FFmpeg configure (no forced target)}"
  echo "Enabled muxers: matroska, webm, image2, image2pipe"
  echo "Enabled filters: setpts, scale, format, aformat, aresample, showwavespic"
  echo "License posture: LGPL 2.1-or-later configure default; GPL, version3, and nonfree were not enabled"
  cat "$temp/desktop-export-codecs/EXPORT-CODEC-SOURCES.txt"
  echo "Packaging modifications: versionless MSVC import-library aliases are copied for Rust linking; upstream import libraries and DLLs are retained; zlib is statically linked into the helper executables"
  echo "MSVC import library names exposed to Rust: avcodec.lib, avformat.lib, avutil.lib, swresample.lib, swscale.lib"
  echo "Compiler: $(cl 2>&1 | head -n 1 || true)"
  echo "Compiler version: ${VCToolsVersion:-unknown}"
  echo "Build tools: $(make --version | head -n 1); $(pkg-config --version)"
  echo "Rust toolchain: $(rustc --version)"
  echo "Runtime library names:"
  (cd "$prefix/bin" && printf '%s\n' avcodec*.dll avformat*.dll avutil*.dll swresample*.dll swscale*.dll | sort)
} > "$temp/FFMPEG-BUILD-INFO.txt"
cat "$temp/FFMPEG-BUILD-INFO.txt"
tar -czf "$temp/ffmpeg-8.1.3-install.tar.gz" -C "$temp" ffmpeg-8.1.3-install
