#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 3 ]]; then
  echo "usage: $0 <linux|macos|windows> <prefix> <work-directory>" >&2
  exit 2
fi

platform="$1"
prefix="$(cd "$(dirname "$2")" && pwd)/$(basename "$2")"
work="$(cd "$(dirname "$3")" && pwd)/$(basename "$3")"
opus_version=1.6.1
opus_sha256=6ffcb593207be92584df15b32466ed64bbec99109f007c82205f0194572411a1
vpx_commit=6df3ec34557879fff673706f4a1d9fbd0f3a6f0e

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
  vs_major="$(basename "$(dirname "$(cygpath -u "$VSINSTALLDIR")")")"
  case "$vs_major" in
    15) vs_year=2017 ;;
    16) vs_year=2019 ;;
    17) vs_year=2022 ;;
    18) vs_year=2026 ;;
    *) echo "unsupported Visual Studio version: $vs_major" >&2; exit 2 ;;
  esac
  vpx_target="x86_64-win64-vs$vs_major"
  win_cmake_generator="Visual Studio $vs_major $vs_year"
else
  echo "unsupported platform: $platform" >&2
  exit 2
fi

opus_archive="$work/opus-$opus_version.tar.gz"
curl --fail --location --silent --show-error --retry 3 --max-time 300 \
  "https://downloads.xiph.org/releases/opus/opus-$opus_version.tar.gz" \
  --output "$opus_archive"
actual_opus_sha256="$(sha256_file "$opus_archive")"
if [[ "$actual_opus_sha256" != "$opus_sha256" ]]; then
  echo "Opus source SHA-256 mismatch: $actual_opus_sha256" >&2
  exit 1
fi
tar -xzf "$opus_archive" -C "$work"

opus_cmake_args=(
  -DCMAKE_BUILD_TYPE=Release \
  "-DCMAKE_INSTALL_PREFIX=$prefix" \
  -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
  -DOPUS_BUILD_SHARED_LIBRARY=OFF \
  -DOPUS_BUILD_TESTING=OFF \
  -DOPUS_BUILD_PROGRAMS=OFF
)
if [[ "$platform" == windows ]]; then
  opus_cmake_args[1]="-DCMAKE_INSTALL_PREFIX=$(cygpath -aw "$prefix")"
  cmake -S "$work/opus-$opus_version" -B "$work/opus-build" \
    -G "$win_cmake_generator" -A x64 "${opus_cmake_args[@]}"
else
  cmake -S "$work/opus-$opus_version" -B "$work/opus-build" "${opus_cmake_args[@]}"
fi
cmake --build "$work/opus-build" --config Release --parallel 2
cmake --install "$work/opus-build" --config Release

vpx_source="$work/libvpx"
git -c advice.detachedHead=false clone --filter=blob:none \
  https://chromium.googlesource.com/webm/libvpx "$vpx_source"
git -C "$vpx_source" checkout --detach "$vpx_commit"
test "$(git -C "$vpx_source" rev-parse HEAD)" = "$vpx_commit"
mkdir -p "$work/libvpx-build"
cd "$work/libvpx-build"
"$vpx_source/configure" \
  "--target=$vpx_target" \
  "--prefix=$prefix" \
  --disable-examples \
  --disable-unit-tests \
  --disable-tools \
  --disable-docs \
  --disable-shared \
  --enable-pic \
  --enable-vp9
make -j2
make install

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
test -f "$prefix/lib/libvpx.a" || test -f "$prefix/lib/vpx.lib"
test -f "$pkgconfig/vpx.pc"
if [[ "$platform" == windows ]]; then
  # MSVC provides math/thread symbols in its C runtime rather than -lm/-lpthread.
  sed -i 's/ -lm$//' "$pkgconfig/vpx.pc"
  sed -i 's/^Libs\.private:.*/Libs.private:/' "$pkgconfig/opus.pc" "$pkgconfig/vpx.pc"
  if PKG_CONFIG_PATH="$pkgconfig" pkg-config --static --libs vpx | grep -Eq -- '(^| )-l(m|pthread)( |$)'; then
    echo "MSVC libvpx pkg-config flags still contain POSIX-only libraries" >&2
    exit 1
  fi
fi

cp "$work/opus-$opus_version/COPYING" "$work/OPUS-COPYING"
cp "$vpx_source/LICENSE" "$work/LIBVPX-LICENSE"
cp "$vpx_source/PATENTS" "$work/LIBVPX-PATENTS"
git -C "$vpx_source" archive --format=tar --prefix=libvpx-v1.17.0/ \
  --output="$work/libvpx-v1.17.0.tar" HEAD
vpx_archive_sha256="$(sha256_file "$work/libvpx-v1.17.0.tar")"
cat > "$work/EXPORT-CODEC-SOURCES.txt" <<EOF
libopus version: $opus_version
libopus source: https://downloads.xiph.org/releases/opus/opus-$opus_version.tar.gz
libopus archive SHA-256: $opus_sha256
libvpx version: v1.17.0
libvpx source: https://chromium.googlesource.com/webm/libvpx
libvpx commit: $vpx_commit
libvpx source archive SHA-256: $vpx_archive_sha256
Build profile: static libraries linked into LGPL FFmpeg shared libraries
EOF
inherited_pkg_config_path="${PKG_CONFIG_PATH:-}"
export PKG_CONFIG_PATH="$pkgconfig${inherited_pkg_config_path:+:$inherited_pkg_config_path}"
test "$(pkg-config --modversion opus)" = "$opus_version"
test "$(pkg-config --variable=prefix opus)" = "$prefix"
test "$(pkg-config --modversion vpx)" = "1.17.0"
test "$(pkg-config --variable=prefix vpx)" = "$prefix"
if [[ -n "${GITHUB_ENV:-}" ]]; then
  echo "FFMPEG_CODEC_PREFIX=$prefix" >> "$GITHUB_ENV"
  echo "LIBVPX_CONFIGURE_TARGET=$vpx_target" >> "$GITHUB_ENV"
  echo "PKG_CONFIG_PATH=$PKG_CONFIG_PATH" >> "$GITHUB_ENV"
fi
printf 'Codec dependency install: %s\n' "$prefix"
pkg-config --modversion opus
pkg-config --modversion vpx
