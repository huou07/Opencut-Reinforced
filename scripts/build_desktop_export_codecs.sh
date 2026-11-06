#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 3 || $# -gt 4 ]]; then
  echo "usage: $0 <linux|macos|windows|android> <prefix> <work-directory> [android-abi]" >&2
  exit 2
fi

platform="$1"
android_abi="${4:-}"
if [[ "$platform" == android && -z "$android_abi" ]] || [[ "$platform" != android && -n "$android_abi" ]]; then
  echo "android builds require an ABI; desktop builds do not accept one" >&2
  exit 2
fi
prefix="$(cd "$(dirname "$2")" && pwd)/$(basename "$2")"
work="$(cd "$(dirname "$3")" && pwd)/$(basename "$3")"
opus_version=1.6.1
opus_sha256=6ffcb593207be92584df15b32466ed64bbec99109f007c82205f0194572411a1
opus_upstream_url="https://downloads.xiph.org/releases/opus/opus-$opus_version.tar.gz"
opus_mirror_url="https://ftp.osuosl.org/pub/xiph/releases/opus/opus-$opus_version.tar.gz"
vpx_commit=6df3ec34557879fff673706f4a1d9fbd0f3a6f0e
x264_commit=b35605ace3ddf7c1a5d67a2eb553f034aef41d55
x264_sha256=6eeb82934e69fd51e043bd8c5b0d152839638d1ce7aa4eea65a3fedcf83ff224
x264_url="https://code.videolan.org/videolan/x264/-/archive/$x264_commit/x264-$x264_commit.tar.bz2"
nv_headers_version=n12.2.72.0
nv_headers_sha256=dbeaec433d93b850714760282f1d0992b1254fc3b5a6cb7d76fc1340a1e47563
nv_headers_url="https://github.com/FFmpeg/nv-codec-headers/archive/refs/tags/$nv_headers_version.tar.gz"

sha256_file() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$1" | awk '{print $1}'
  else
    echo "no SHA-256 utility is available" >&2
    return 1
  fi
}

mkdir -p "$work" "$prefix"

if [[ "$platform" == linux ]]; then
  case "$(uname -m)" in
    x86_64) vpx_target=x86_64-linux-gcc ;;
    aarch64|arm64) vpx_target=arm64-linux-gcc ;;
    *) echo "unsupported Linux architecture: $(uname -m)" >&2; exit 2 ;;
  esac
elif [[ "$platform" == macos ]]; then
  deployment_target="${MACOSX_DEPLOYMENT_TARGET:-12.0}"
  export MACOSX_DEPLOYMENT_TARGET="$deployment_target"
  export CFLAGS="${CFLAGS:-} -mmacosx-version-min=$deployment_target"
  export CXXFLAGS="${CXXFLAGS:-} -mmacosx-version-min=$deployment_target"
  export LDFLAGS="${LDFLAGS:-} -mmacosx-version-min=$deployment_target"
  case "$(uname -m)" in
    arm64|aarch64) vpx_target=arm64-darwin25-gcc ;;
    x86_64) vpx_target=x86_64-darwin24-gcc ;;
    *) echo "unsupported macOS architecture: $(uname -m)" >&2; exit 2 ;;
  esac
elif [[ "$platform" == windows ]]; then
  if [[ -z "${VSINSTALLDIR:-}" ]]; then
    echo "Visual Studio install directory is unavailable" >&2
    exit 2
  fi
  # `VisualStudioVersion` is the authoritative major version and both vcvars and
  # the CI MSVC action export it; the install path only carries the release year
  # (…\2022\BuildTools), so deriving the major from the path shape is wrong.
  if [[ -n "${VisualStudioVersion:-}" ]]; then
    vs_major="${VisualStudioVersion%%.*}"
  else
    vs_major="$(basename "$(dirname "$(cygpath -u "${VSINSTALLDIR%/}")")")"
  fi
  case "$vs_major" in
    2017) vs_major=15 ;;
    2019) vs_major=16 ;;
    2022) vs_major=17 ;;
    2026) vs_major=18 ;;
  esac
  case "$vs_major" in
    15) vs_year=2017 ;;
    16) vs_year=2019 ;;
    17) vs_year=2022 ;;
    18) vs_year=2026 ;;
    *) echo "unsupported Visual Studio version: ${VisualStudioVersion:-$vs_major}" >&2; exit 2 ;;
  esac
  vpx_target="x86_64-win64-vs$vs_major"
  win_cmake_generator="Visual Studio $vs_major $vs_year"
elif [[ "$platform" == android ]]; then
  android_ndk="${ANDROID_NDK_HOME:-${ANDROID_HOME:-}/ndk/28.2.13676358}"
  android_toolchain="$android_ndk/toolchains/llvm/prebuilt/linux-x86_64/bin"
  case "$android_abi" in
    armeabi-v7a) vpx_target=armv7-android-gcc; clang_prefix=armv7a-linux-androideabi ;;
    arm64-v8a) vpx_target=arm64-android-gcc; clang_prefix=aarch64-linux-android ;;
    x86_64) vpx_target=x86_64-android-gcc; clang_prefix=x86_64-linux-android ;;
    *) echo "unsupported Android ABI: $android_abi" >&2; exit 2 ;;
  esac
  android_clang="$android_toolchain/${clang_prefix}26-clang"
  android_clangxx="$android_toolchain/${clang_prefix}26-clang++"
  android_as="$android_clang"
  if [[ "$android_abi" == x86_64 ]]; then
    command -v nasm >/dev/null
    android_as=nasm
  fi
  android_vpx_tools=(
    "CC=$android_clang"
    "CXX=$android_clangxx"
    "AR=$android_toolchain/llvm-ar"
    "LD=$android_clang"
    "AS=$android_as"
    "STRIP=$android_toolchain/llvm-strip"
  )
  for tool in "$android_clang" "$android_clangxx" \
    "$android_toolchain/llvm-ar" "$android_toolchain/llvm-strip"; do
    test -x "$tool"
  done
else
  echo "unsupported platform: $platform" >&2
  exit 2
fi

opus_archive="$work/opus-$opus_version.tar.gz"
if [[ ! -f "$opus_archive" ]]; then
  curl --fail --location --silent --show-error --retry 3 --max-time 300 \
    "$opus_mirror_url" \
    --output "$opus_archive"
fi
actual_opus_sha256="$(sha256_file "$opus_archive")"
if [[ "$actual_opus_sha256" != "$opus_sha256" ]]; then
  echo "Opus source SHA-256 mismatch: $actual_opus_sha256" >&2
  exit 1
fi
if [[ ! -d "$work/opus-$opus_version" ]]; then
  tar -xzf "$opus_archive" -C "$work"
fi

opus_build="$work/opus-build"
vpx_build="$work/libvpx-build"
opus_cmake_args=(
  -DCMAKE_BUILD_TYPE=Release \
  "-DCMAKE_INSTALL_PREFIX=$prefix" \
  -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
  -DOPUS_BUILD_SHARED_LIBRARY=OFF \
  -DOPUS_BUILD_TESTING=OFF \
  -DOPUS_BUILD_PROGRAMS=OFF
)
if [[ "$platform" == android ]]; then
  opus_build+="-$android_abi"
  vpx_build+="-$android_abi"
  opus_cmake_args+=(
    "-DCMAKE_TOOLCHAIN_FILE=$android_ndk/build/cmake/android.toolchain.cmake"
    "-DANDROID_ABI=$android_abi"
    -DANDROID_PLATFORM=26
    -DANDROID_STL=c++_static
  )
fi
if [[ "$platform" == macos ]]; then
  opus_cmake_args+=("-DCMAKE_OSX_DEPLOYMENT_TARGET=$deployment_target")
fi
if [[ "$platform" == windows ]]; then
  opus_cmake_args[1]="-DCMAKE_INSTALL_PREFIX=$(cygpath -aw "$prefix")"
  cmake -S "$work/opus-$opus_version" -B "$opus_build" \
    -G "$win_cmake_generator" -A x64 "${opus_cmake_args[@]}"
else
  cmake -S "$work/opus-$opus_version" -B "$opus_build" "${opus_cmake_args[@]}"
fi
cmake --build "$opus_build" --config Release --parallel 2
cmake --install "$opus_build" --config Release

vpx_source="$work/libvpx"
ensure_vpx_source() {
  local attempt fetch_log="$work/libvpx-fetch.log"
  for attempt in 1 2 3; do
    : > "$fetch_log"
    if [[ ! -d "$vpx_source/.git" ]] &&
      ! git -c advice.detachedHead=false clone --filter=blob:none \
        https://chromium.googlesource.com/webm/libvpx "$vpx_source" >"$fetch_log" 2>&1; then
      :
    elif [[ -d "$vpx_source/.git" ]] &&
      git -C "$vpx_source" checkout --detach "$vpx_commit" >>"$fetch_log" 2>&1 &&
      [[ "$(git -C "$vpx_source" rev-parse HEAD)" == "$vpx_commit" ]]; then
      cat "$fetch_log"
      return 0
    fi

    cat "$fetch_log" >&2
    if ! grep -Eq 'HTTP (429|500|502|503|504)' "$fetch_log"; then return 1; fi
    rm -rf "$vpx_source"
    if [[ "$attempt" -lt 3 ]]; then
      local delay=$((attempt * 5))
      echo "Retrying pinned libvpx source fetch after transient HTTP failure (attempt $((attempt + 1))/3) in ${delay}s." >&2
      sleep "$delay"
    fi
  done
  return 1
}

ensure_vpx_source
test "$(git -C "$vpx_source" rev-parse HEAD)" = "$vpx_commit"
mkdir -p "$vpx_build"
cd "$vpx_build"
vpx_configure_args=(
  "--target=$vpx_target" \
  "--prefix=$prefix" \
  --disable-examples \
  --disable-unit-tests \
  --disable-tools \
  --disable-docs \
  --disable-shared \
  --enable-pic \
  --enable-vp9
)
if [[ "$platform" == macos ]]; then
  vpx_configure_args+=("--extra-cflags=-mmacosx-version-min=$deployment_target")
fi
if [[ "$platform" == windows ]]; then
  # FFmpeg's documented MSVC toolchain uses /MT; match it to avoid mixing the
  # UCRT imports from libvpx's default /MD archive into FFmpeg's msvcrt build.
  vpx_configure_args+=(--enable-static-msvcrt)
fi
if [[ "$platform" == android ]]; then
  env "${android_vpx_tools[@]}" "$vpx_source/configure" "${vpx_configure_args[@]}"
else
  "$vpx_source/configure" "${vpx_configure_args[@]}"
fi
if [[ "$platform" == macos && -f Makefile ]]; then make clean; fi
make -j"${OR_BUILD_JOBS:-2}"
make install

if [[ "$platform" == windows ]]; then
  # The MSVC target installs vpxmt.lib below lib/x64 but does not install a
  # pkg-config file. Describe the /MT-compatible archive for FFmpeg.
  test -f "$prefix/lib/x64/vpxmt.lib"
  mkdir -p "$prefix/lib/pkgconfig"
  cat > "$prefix/lib/pkgconfig/vpx.pc" <<EOF
prefix=$prefix
exec_prefix=\${prefix}
libdir=\${prefix}/lib
includedir=\${prefix}/include

Name: libvpx
Description: VP8 and VP9 video codec
Version: 1.17.0
Libs: -L\${libdir}/x64 -lvpxmt
Libs.private:
Cflags: -I\${includedir}
EOF
fi

# Linux has no operating-system H.264 encoder: VideoToolbox and Media
# Foundation do not exist there. x264 is the software encoder Linux delivery
# needs, and it is GPL-2.0, so it is linked into the Linux FFmpeg only and its
# licence ships beside the package notices. See
# docs/adr/0011-linux-delivery-encoder.md.
if [[ "$platform" == linux ]]; then
  x264_archive="$work/x264-$x264_commit.tar.bz2"
  if [[ ! -f "$x264_archive" ]]; then
    curl --fail --location --silent --show-error --retry 5 --retry-all-errors \
      "$x264_url" --output "$x264_archive"
  fi
  actual_x264_sha256="$(sha256_file "$x264_archive")"
  if [[ "$actual_x264_sha256" != "$x264_sha256" ]]; then
    echo "x264 source SHA-256 mismatch: $actual_x264_sha256" >&2
    exit 1
  fi
  if [[ ! -d "$work/x264-$x264_commit" ]]; then
    tar -xjf "$x264_archive" -C "$work"
  fi
  x264_source="$work/x264-$x264_commit"
  x264_build="$work/x264-build"
  rm -rf "$x264_build"
  mkdir -p "$x264_build"
  cd "$x264_build"
  "$x264_source/configure" \
    --prefix="$prefix" \
    --enable-static \
    --enable-pic \
    --disable-cli \
    --disable-opencl
  make -j"${OR_BUILD_JOBS:-2}"
  make install
  test -f "$prefix/lib/libx264.a"
  test -f "$prefix/lib/pkgconfig/x264.pc"
  cp "$x264_source/COPYING" "$work/X264-COPYING"
fi

# Hardware H.264 needs FFmpeg's NVENC headers at build time only: the encoder
# loads the driver's libnvcuda at run time and falls back to software when no
# NVIDIA device answers, which the export path checks before it commits. The
# archives are plain headers, so this costs seconds on both desktop builds.
if [[ "$platform" == linux || "$platform" == windows ]]; then
  nv_archive="$work/nv-codec-headers-$nv_headers_version.tar.gz"
  if [[ ! -f "$nv_archive" ]]; then
    curl --fail --location --silent --show-error --retry 5 --retry-all-errors \
      "$nv_headers_url" --output "$nv_archive"
  fi
  actual_nv_sha256="$(sha256_file "$nv_archive")"
  if [[ "$actual_nv_sha256" != "$nv_headers_sha256" ]]; then
    echo "nv-codec-headers source SHA-256 mismatch: $actual_nv_sha256" >&2
    exit 1
  fi
  if [[ ! -d "$work/nv-codec-headers-$nv_headers_version" ]]; then
    tar -xzf "$nv_archive" -C "$work"
  fi
  make -C "$work/nv-codec-headers-$nv_headers_version" install PREFIX="$prefix"
  test -f "$prefix/include/ffnvcodec/nvEncodeAPI.h"
fi

pkgconfig="$prefix/lib/pkgconfig"
mkdir -p "$pkgconfig"
cat > "$pkgconfig/opus.pc" <<EOF
prefix=$prefix
exec_prefix=\${prefix}
libdir=\${prefix}/lib
includedir=\${prefix}/include

Name: opus
Description: Opus interactive audio codec
Version: $opus_version
Libs: -L\${libdir} -lopus
Libs.private: -lm
Cflags: -I\${includedir}/opus
EOF
test -f "$prefix/lib/libopus.a" || test -f "$prefix/lib/opus.lib"
if [[ "$platform" == windows ]]; then
  test -f "$prefix/lib/x64/vpxmt.lib"
else
  test -f "$prefix/lib/libvpx.a"
fi
test -f "$pkgconfig/vpx.pc"
if [[ "$platform" == windows ]]; then
  # MSVC provides math/thread symbols in its C runtime rather than -lm/-lpthread.
  sed -i 's/^Libs\.private:.*/Libs.private:/' "$pkgconfig/opus.pc" "$pkgconfig/vpx.pc"
  if ! PKG_CONFIG_PATH="$pkgconfig" pkg-config --static --libs vpx | grep -Fq -- '-lvpxmt'; then
    echo "MSVC libvpx pkg-config flags do not name the installed vpxmt archive" >&2
    exit 1
  fi
  if PKG_CONFIG_PATH="$pkgconfig" pkg-config --static --libs vpx | grep -Eq -- '(^| )-l(m|pthread)( |$)'; then
    echo "MSVC libvpx pkg-config flags still contain POSIX-only libraries" >&2
    exit 1
  fi
fi

cp "$work/opus-$opus_version/COPYING" "$work/OPUS-COPYING"
git -C "$vpx_source" show "$vpx_commit:LICENSE" > "$work/LIBVPX-LICENSE"
git -C "$vpx_source" show "$vpx_commit:PATENTS" > "$work/LIBVPX-PATENTS"
git -C "$vpx_source" -c core.autocrlf=false archive \
  --format=tar --prefix=libvpx-v1.17.0/ \
  --output="$work/libvpx-v1.17.0.tar" HEAD
vpx_archive_sha256="$(sha256_file "$work/libvpx-v1.17.0.tar")"
cat > "$work/EXPORT-CODEC-SOURCES.txt" <<EOF
libopus version: $opus_version
libopus upstream source: $opus_upstream_url
libopus download mirror: $opus_mirror_url
libopus archive SHA-256: $opus_sha256
libvpx version: v1.17.0
libvpx source: https://chromium.googlesource.com/webm/libvpx
libvpx commit: $vpx_commit
libvpx source archive SHA-256: $vpx_archive_sha256
Build profile: static libraries linked into LGPL FFmpeg shared libraries
x264 commit: $x264_commit
x264 source: https://code.videolan.org/videolan/x264
x264 source archive SHA-256: $x264_sha256
x264 licence: GPL-2.0-or-later, linked into the Linux FFmpeg only
EOF
inherited_pkg_config_path="${PKG_CONFIG_PATH:-}"
export PKG_CONFIG_PATH="$pkgconfig${inherited_pkg_config_path:+:$inherited_pkg_config_path}"
test "$(pkg-config --modversion opus)" = "$opus_version"
test "$(pkg-config --variable=prefix opus)" = "$prefix"
test "$(pkg-config --modversion vpx)" = "1.17.0"
test "$(pkg-config --variable=prefix vpx)" = "$prefix"
if [[ -n "${GITHUB_ENV:-}" && "$platform" != android ]]; then
  echo "FFMPEG_CODEC_PREFIX=$prefix" >> "$GITHUB_ENV"
  echo "LIBVPX_CONFIGURE_TARGET=$vpx_target" >> "$GITHUB_ENV"
  echo "PKG_CONFIG_PATH=$PKG_CONFIG_PATH" >> "$GITHUB_ENV"
fi
printf 'Codec dependency install: %s\n' "$prefix"
pkg-config --modversion opus
pkg-config --modversion vpx
