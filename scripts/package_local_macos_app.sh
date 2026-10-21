#!/usr/bin/env bash
set -euo pipefail

# shortcut: this local bundle omits third-party notices; use release packaging before distribution.
fail() {
  echo "$*" >&2
  exit 1
}

if [[ "$(uname -s)" != Darwin ]]; then
  fail "This helper packages macOS application bundles only."
fi
if [[ $# -ne 2 || "$1" != *.app ]]; then
  fail "usage: $0 <local-app.app> <ffmpeg-install-prefix>"
fi

app="$(cd "$1" && pwd -P)" || fail "No such app bundle: $1"
ffmpeg_prefix="$(cd "$2" && pwd -P)" || fail "No such FFmpeg prefix: $2"
ffmpeg_lib_dir="$ffmpeg_prefix/lib"
ffmpeg_bin_dir="$ffmpeg_prefix/bin"
frameworks="$app/Contents/Frameworks"
helpers="$app/Contents/MacOS"
bridge="$frameworks/or_app_bridge.framework/Versions/A/or_app_bridge"
runtime_dir="$(dirname "$bridge")"

[[ -f "$app/Contents/Info.plist" ]] || fail "Not a macOS app bundle: $app"
[[ -f "$bridge" ]] || fail "Rust bridge framework not found: $bridge"
[[ -d "$ffmpeg_lib_dir" && -d "$ffmpeg_bin_dir" ]] || fail "FFmpeg prefix is incomplete: $ffmpeg_prefix"
for tool in otool install_name_tool codesign python3; do
  command -v "$tool" >/dev/null || fail "Required macOS tool is missing: $tool"
done
for helper in ffmpeg ffprobe; do
  [[ -x "$ffmpeg_bin_dir/$helper" ]] || fail "FFmpeg helper is missing: $ffmpeg_bin_dir/$helper"
done

mkdir -p "$frameworks" "$runtime_dir" "$helpers"
for pattern in libavcodec*.dylib libavformat*.dylib libavutil*.dylib \
  libswresample*.dylib libswscale*.dylib libavfilter*.dylib; do
  found=0
  for source in "$ffmpeg_lib_dir"/$pattern; do
    [[ -e "$source" ]] || continue
    found=1
    cp -a "$source" "$runtime_dir/"
    cp -a "$source" "$frameworks/"
  done
  [[ "$found" -eq 1 ]] || fail "No FFmpeg library matched $ffmpeg_lib_dir/$pattern"
done
install -m 755 "$ffmpeg_bin_dir/ffmpeg" "$helpers/ffmpeg"
install -m 755 "$ffmpeg_bin_dir/ffprobe" "$helpers/ffprobe"

remove_build_rpath() {
  local binary="$1"
  if otool -l "$binary" | grep -Fq "path $ffmpeg_lib_dir "; then
    install_name_tool -delete_rpath "$ffmpeg_lib_dir" "$binary"
  fi
}

ensure_rpath() {
  local binary="$1"
  local rpath="$2"
  remove_build_rpath "$binary"
  if ! otool -l "$binary" | grep -Fq "path $rpath "; then
    install_name_tool -add_rpath "$rpath" "$binary"
  fi
}

ensure_rpath "$bridge" @loader_path
for library in "$runtime_dir"/libavcodec*.dylib "$runtime_dir"/libavformat*.dylib \
  "$runtime_dir"/libavutil*.dylib "$runtime_dir"/libswresample*.dylib \
  "$runtime_dir"/libswscale*.dylib "$runtime_dir"/libavfilter*.dylib \
  "$frameworks"/libavcodec*.dylib "$frameworks"/libavformat*.dylib \
  "$frameworks"/libavutil*.dylib "$frameworks"/libswresample*.dylib \
  "$frameworks"/libswscale*.dylib "$frameworks"/libavfilter*.dylib; do
  [[ -f "$library" && ! -L "$library" ]] || continue
  ensure_rpath "$library" @loader_path
done
ensure_rpath "$helpers/ffmpeg" @loader_path/../Frameworks
ensure_rpath "$helpers/ffprobe" @loader_path/../Frameworks

codesign --force --deep --sign - --timestamp=none --preserve-metadata=entitlements "$app"
codesign --verify --deep --strict "$app"
sandbox_entitlement="$(codesign --display --entitlements - --xml "$app" 2>/dev/null | \
  python3 -c 'import plistlib, sys; print(plistlib.loads(sys.stdin.buffer.read()).get("com.apple.security.app-sandbox", False))')"
[[ "$sandbox_entitlement" == True ]] || fail "The app sandbox entitlement was lost while signing: $app"

check_relocated() {
  local binary="$1"
  if otool -L "$binary" | grep -Fq "$ffmpeg_prefix"; then
    fail "Absolute FFmpeg cache reference remains in dependencies: $binary"
  fi
  if otool -l "$binary" | grep -Fq "$ffmpeg_prefix"; then
    fail "Absolute FFmpeg cache reference remains in run paths: $binary"
  fi
}

check_relocated "$bridge"
check_relocated "$helpers/ffmpeg"
check_relocated "$helpers/ffprobe"
for library in "$runtime_dir"/libavcodec*.dylib "$runtime_dir"/libavformat*.dylib \
  "$runtime_dir"/libavutil*.dylib "$runtime_dir"/libswresample*.dylib \
  "$runtime_dir"/libswscale*.dylib "$runtime_dir"/libavfilter*.dylib \
  "$frameworks"/libavcodec*.dylib "$frameworks"/libavformat*.dylib \
  "$frameworks"/libavutil*.dylib "$frameworks"/libswresample*.dylib \
  "$frameworks"/libswscale*.dylib "$frameworks"/libavfilter*.dylib; do
  [[ -f "$library" && ! -L "$library" ]] || continue
  check_relocated "$library"
done

encoders="$(env -u DYLD_LIBRARY_PATH -u DYLD_FALLBACK_LIBRARY_PATH \
  -u DYLD_FRAMEWORK_PATH -u DYLD_ROOT_PATH \
  "$helpers/ffmpeg" -hide_banner -encoders)"
printf '%s\n' "$encoders" | grep -Fq h264_videotoolbox || fail "Bundled FFmpeg has no VideoToolbox H.264 encoder"
printf '%s\n' "$encoders" | grep -Eq '[[:space:]]aac[[:space:]]' || fail "Bundled FFmpeg has no AAC encoder"
env -u DYLD_LIBRARY_PATH -u DYLD_FALLBACK_LIBRARY_PATH \
  -u DYLD_FRAMEWORK_PATH -u DYLD_ROOT_PATH \
  "$helpers/ffprobe" -hide_banner -version >/dev/null
env -u DYLD_LIBRARY_PATH -u DYLD_FALLBACK_LIBRARY_PATH \
  -u DYLD_FRAMEWORK_PATH -u DYLD_ROOT_PATH \
  python3 -c 'import ctypes, sys; ctypes.CDLL(sys.argv[1])' "$bridge"

echo "Packaged and verified local macOS app: $app"
