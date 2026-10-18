#!/usr/bin/env bash
# Stage the pinned x86_64 Android FFmpeg install the local Android lane needs.
#
#   bash scripts/stage_android_ffmpeg.sh [--root DIR] [--abi x86_64] [--force]
#                                     [--probe-only]
#
# Why this exists: the Android lane can only build its APK against a staged
# x86_64 Android FFmpeg prefix, and until this script the recipe lived only
# inside the `android` job of `.github/workflows/platform-verification.yml`.
# That workflow remains the source of truth for the pins; when it changes, this
# script must change with it. It reproduces, for one local ABI:
#
#   * FFmpeg 8.1.3 from the official release archive, SHA-512 verified;
#   * the pinned static libvpx (v1.17.0) and libopus (1.6.1) codecs that
#     `scripts/build_desktop_export_codecs.sh android` already knows how to
#     build;
#   * the minimal LGPL-2.1-or-later library-only configure profile plus the
#     WebM/VP9/Opus delivery profile;
#   * the `ffmpeg-link-probe` Rust binary from tools/ffmpeg-link-probe.
#
# Nothing here is downloaded at build time by the product: this is a local
# developer staging helper. It writes outside the repository (default
# $HOME/Android/or-android-ffmpeg-8.1.3), and the lane consumes the result
# through OR_ANDROID_FFMPEG_PREFIX.
#
# Output layout under <root>:
#   src/ffmpeg-8.1.3.tar.xz                  verified source archive
#   build/ffmpeg-8.1.3/                      extracted source tree
#   export-codecs/<abi>/                     static libvpx/libopus prefix
#   install/<abi>/                           OR_ANDROID_FFMPEG_PREFIX
#   probes/<abi>/ffmpeg-link-probe           guest link probe (best effort)
#   build-info/<abi>.txt                     pins and configure arguments
#
# Exit codes: 0 the prefix is installed; 1 a build step failed; 2 the invocation
# is wrong; 3 a host precondition is missing.
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
FFMPEG_VERSION=8.1.3
FFMPEG_URL="https://ffmpeg.org/releases/ffmpeg-$FFMPEG_VERSION.tar.xz"
FFMPEG_SHA512=acc7dbacde58c91bbc4a86fa589d317515d40bc9bc1ca5f9c020034bb54bf3c8cd61ed70ce437b962bd50332ea8378b3ad52da1942cfdb29e380c33a90d2f0c1
NDK_VERSION=28.2.13676358
API_LEVEL=26
abi=x86_64
root="${HOME}/Android/or-android-ffmpeg-$FFMPEG_VERSION"
force=0
probe_only=0

while (( $# )); do
  case "$1" in
    --root) (( $# >= 2 )) || { printf 'stage_android_ffmpeg: --root needs a value\n' >&2; exit 2; }; root="$2"; shift 2 ;;
    --abi) (( $# >= 2 )) || { printf 'stage_android_ffmpeg: --abi needs a value\n' >&2; exit 2; }; abi="$2"; shift 2 ;;
    --force) force=1; shift ;;
    --probe-only) probe_only=1; shift ;;
    -h|--help) sed -n '2,40p' "${BASH_SOURCE[0]}"; exit 0 ;;
    *) printf 'stage_android_ffmpeg: unknown argument: %s\n' "$1" >&2; exit 2 ;;
  esac
done

case "$abi" in
  x86_64) rust_target=x86_64-linux-android; ffmpeg_arch=x86_64; clang_prefix=x86_64-linux-android ;;
  arm64-v8a) rust_target=aarch64-linux-android; ffmpeg_arch=aarch64; clang_prefix=aarch64-linux-android ;;
  *) printf 'stage_android_ffmpeg: unsupported ABI: %s (the local lane needs x86_64)\n' "$abi" >&2; exit 2 ;;
esac
target_suffix="${rust_target//-/_}"
ndk="${ANDROID_NDK_HOME:-${ANDROID_HOME:-$HOME/Android/Sdk}/ndk/$NDK_VERSION}"
toolchain="$ndk/toolchains/llvm/prebuilt/linux-x86_64"
clang="$toolchain/bin/${clang_prefix}${API_LEVEL}-clang"
clangxx="$toolchain/bin/${clang_prefix}${API_LEVEL}-clang++"

install_prefix="$root/install/$abi"
codec_prefix="$root/export-codecs/$abi"
codec_work="$root/export-codecs-work/$abi"
probe_dir="$root/probes/$abi"
info_dir="$root/build-info"
source_dir="$root/build/ffmpeg-$FFMPEG_VERSION"
archive="$root/src/ffmpeg-$FFMPEG_VERSION.tar.xz"

# The minimal library-only profile, copied from FFMPEG_COMMON_CONFIGURE_ARGS.
# shellcheck disable=SC2054  # FFmpeg's comma-separated lists are single arguments.
common_configure_args=(
  --disable-static --enable-shared --disable-programs --disable-doc
  --disable-autodetect --disable-everything --disable-asm --disable-avdevice
  --disable-avfilter --enable-protocol=file
  --enable-demuxer=matroska,mov,wav,mp3 --enable-muxer=matroska
  --enable-decoder=ffv1,pcm_s16le,h264,aac,mp3,mp3float,vp9,opus
  --enable-parser=h264,aac,mp3 --enable-encoder=ffv1,pcm_s16le
)
# The shared WebM/VP9/Opus delivery profile, copied from
# FFMPEG_EXPORT_CONFIGURE_ARGS.
# shellcheck disable=SC2054  # `libvpx_vp9,libopus` is one argument, not two.
export_configure_args=(
  --pkg-config-flags=--static --enable-libvpx --enable-libopus
  --enable-muxer=webm --enable-encoder=libvpx_vp9,libopus
)

fail_precondition() { printf 'stage_android_ffmpeg: BLOCKED: %s\n' "$1" >&2; exit 3; }

for tool in curl sha512sum xz tar make pkg-config python3 cargo; do
  command -v "$tool" >/dev/null 2>&1 || fail_precondition "missing tool on PATH: $tool"
done
[[ -x "$clang" && -x "$clangxx" ]] || fail_precondition "missing API $API_LEVEL clang wrappers under $toolchain (need NDK $NDK_VERSION)"
[[ -f "$ROOT/scripts/build_desktop_export_codecs.sh" ]] || fail_precondition "missing scripts/build_desktop_export_codecs.sh"
# libvpx's x86 assembly needs nasm; the workflow installs it for the same reason.
[[ "$abi" == x86_64 ]] && command -v nasm >/dev/null 2>&1 || true
if [[ "$abi" == x86_64 ]] && ! command -v nasm >/dev/null 2>&1; then
  fail_precondition "missing nasm, which the pinned x86_64 libvpx build requires"
fi

mkdir -p "$root/src" "$codec_prefix" "$codec_work" "$probe_dir" "$info_dir"

# Kept outside the build branch so the build-info record can still name the
# profile when only the probe is rebuilt.
configure_args=()

if (( probe_only == 1 )); then
  [[ -f "$install_prefix/lib/libavcodec.so" ]] ||
    fail_precondition "--probe-only needs an installed prefix at $install_prefix"
else

if [[ ! -f "$archive" ]]; then
  printf 'stage_android_ffmpeg: downloading %s\n' "$FFMPEG_URL"
  curl --fail --location --silent --show-error --retry 3 --max-time 900 \
    "$FFMPEG_URL" --output "$archive.partial"
  mv "$archive.partial" "$archive"
fi
printf '%s  %s\n' "$FFMPEG_SHA512" "$archive" | sha512sum --check --status ||
  { printf 'stage_android_ffmpeg: SHA-512 mismatch for %s\n' "$archive" >&2; exit 1; }
xz --test "$archive"
printf 'stage_android_ffmpeg: source archive verified\n'

if (( force == 1 )); then
  rm -rf "$install_prefix" "$source_dir" "$probe_dir"
fi
mkdir -p "$install_prefix"

printf 'stage_android_ffmpeg: building pinned static export codecs (libvpx v1.17.0, libopus 1.6.1)\n'
ANDROID_NDK_HOME="$ndk" ANDROID_ABI="$abi" \
  bash "$ROOT/scripts/build_desktop_export_codecs.sh" android "$codec_prefix" "$codec_work" "$abi"

if [[ ! -d "$source_dir" ]]; then
  rm -rf "$source_dir"
  mkdir -p "$(dirname "$source_dir")"
  tar -xJf "$archive" -C "$(dirname "$source_dir")"
fi
test -d "$source_dir"

configure_args=(
  "--prefix=$install_prefix"
  --target-os=android "--arch=$ffmpeg_arch" --enable-cross-compile --enable-pic
  "--cc=$clang" "--cxx=$clangxx"
  "--ar=$toolchain/bin/llvm-ar" "--nm=$toolchain/bin/llvm-nm"
  "--ranlib=$toolchain/bin/llvm-ranlib" "--strip=$toolchain/bin/llvm-strip"
  "${common_configure_args[@]}"
)

printf 'stage_android_ffmpeg: configuring FFmpeg %s for %s (API %s)\n' "$FFMPEG_VERSION" "$abi" "$API_LEVEL"
(
  cd "$source_dir"
  PKG_CONFIG_LIBDIR="$codec_prefix/lib/pkgconfig" \
    PKG_CONFIG_PATH="$codec_prefix/lib/pkgconfig" \
    PKG_CONFIG_ALLOW_CROSS=1 \
    ./configure "${configure_args[@]}" "${export_configure_args[@]}"
  grep -q '^#define CONFIG_LIBVPX_VP9_ENCODER 1$' config_components.h
  grep -q '^#define CONFIG_LIBOPUS_ENCODER 1$' config_components.h
  grep -q '^#define CONFIG_WEBM_MUXER 1$' config_components.h
  make -j"$(nproc)"
  make install
)

for library in avcodec avformat avutil swresample swscale; do
  test -f "$install_prefix/lib/pkgconfig/lib$library.pc"
  test "$(PKG_CONFIG_PATH="$install_prefix/lib/pkgconfig" PKG_CONFIG_ALLOW_CROSS=1 \
    pkg-config --variable=prefix "lib$library")" = "$install_prefix"
  test -f "$install_prefix/lib/lib$library.so"
done
printf 'stage_android_ffmpeg: prefix installed at %s\n' "$install_prefix"

fi # probe_only

# The guest link probe is best effort: the prefix above is what the APK build
# requires, and a probe that cannot be built must not hide that the prefix is
# usable. It is reported instead, and the lane warns when it is absent.
probe_status="not built"
probe_env_cc="CC_$target_suffix"
probe_env_ar="AR_$target_suffix"
probe_env_bindgen="BINDGEN_EXTRA_CLANG_ARGS_$target_suffix"
target_upper="${rust_target^^}"
target_upper="${target_upper//-/_}"
probe_env_linker="CARGO_TARGET_${target_upper}_LINKER"
# The linker must be the API-26 NDK clang: the host `cc` cannot resolve Android
# system libraries (`-llog`, `-lunwind`).
#
# FFMPEG_DIR and PKG_CONFIG_PATH are passed on the command line as well as in the
# environment because a shared `$CARGO_HOME/config.toml` on this host pins both
# with `force = true` to a Linux desktop prefix; `force` makes the file win over
# the environment, which silently links the probe against host libraries. The
# override is scoped to this invocation and changes no shared state.
if env "$probe_env_cc=$clang" "$probe_env_ar=$toolchain/bin/llvm-ar" \
  "$probe_env_bindgen=--target=$rust_target --sysroot=$toolchain/sysroot" \
  "$probe_env_linker=$clang" \
  CARGO_TARGET_DIR="$root/probe-target" \
  FFMPEG_DIR="$install_prefix" \
  PKG_CONFIG_PATH="$install_prefix/lib/pkgconfig" \
  PKG_CONFIG_ALLOW_CROSS=1 \
  cargo build --locked --release --target "$rust_target" \
    --config "env.FFMPEG_DIR.value=\"$install_prefix\"" \
    --config "env.FFMPEG_DIR.force=true" \
    --config "env.PKG_CONFIG_PATH.value=\"$install_prefix/lib/pkgconfig\"" \
    --config "env.PKG_CONFIG_PATH.force=true" \
    --manifest-path "$ROOT/tools/ffmpeg-link-probe/Cargo.toml" >"$info_dir/probe-build-$abi.log" 2>&1; then
  built_probe="$root/probe-target/$rust_target/release/ffmpeg-link-probe"
  if [[ -f "$built_probe" ]]; then
    install -m 755 "$built_probe" "$probe_dir/ffmpeg-link-probe"
    probe_status="built"
  fi
else
  probe_status="FAILED; see $info_dir/probe-build-$abi.log"
fi

{
  echo "Target: Android $abi ($rust_target)"
  echo "NDK: $NDK_VERSION"
  echo "API level: $API_LEVEL"
  echo "FFmpeg release: $FFMPEG_VERSION"
  echo "Official source URL: $FFMPEG_URL"
  echo "Source archive SHA-512: $FFMPEG_SHA512"
  echo "Patch status: unpatched upstream release archive"
  echo "Rust binding: ffmpeg-the-third 6.0.0"
  echo "Profile arguments: ${common_configure_args[*]} ${export_configure_args[*]}"
  if (( ${#configure_args[@]} > 0 )); then
    echo "Toolchain arguments: ${configure_args[*]}"
  else
    echo "Toolchain arguments: not recorded (this run rebuilt only the link probe)"
  fi
  echo "Link probe: $probe_status"
  echo "Staged at: $(date -u '+%Y-%m-%dT%H:%M:%SZ')"
} > "$info_dir/$abi.txt"

printf '\n== staged android ffmpeg ==\n'
printf 'export OR_ANDROID_FFMPEG_PREFIX=%s\n' "$install_prefix"
if [[ -x "$probe_dir/ffmpeg-link-probe" ]]; then
  printf 'export OR_ANDROID_FFMPEG_PROBE=%s\n' "$probe_dir/ffmpeg-link-probe"
else
  printf '# no link probe (%s); the lane will record that it did not run\n' "$probe_status"
fi
printf 'build info: %s\n' "$info_dir/$abi.txt"
