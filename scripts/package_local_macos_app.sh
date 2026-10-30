#!/usr/bin/env bash
set -euo pipefail

# shortcut: this local bundle omits third-party notices; use release packaging before distribution.
script_dir="$(cd -- "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(dirname "$script_dir")"

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

# The assistant edits the live project through the `or` CLI that sits beside the
# app executable, so a packaged local build carries the same CLI the checkout
# uses. The CLI links only system frameworks today; assert that here instead of
# assuming it, because a stray library path would break a moved bundle.
cli_source="${OR_CLI_BINARY:-}"
if [[ -z "$cli_source" ]]; then
  for candidate in "$repo_root/target/release/or" "$repo_root/target/debug/or"; do
    if [[ -x "$candidate" ]]; then
      cli_source="$candidate"
      break
    fi
  done
fi
[[ -n "$cli_source" ]] || fail "the or CLI is missing; build it with cargo build -p or_cli --release or set OR_CLI_BINARY"
install -m 755 "$cli_source" "$helpers/or"
while IFS= read -r dependency; do
  case "$dependency" in
    @*|/usr/lib/*|/System/Library/*) ;;
    *) fail "the or CLI has a non-system runtime dependency: $dependency" ;;
  esac
done < <(otool -L "$helpers/or" | awk 'NR > 1 {print $1}')

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

copied_external_dependency=0
macho_uuid() {
  otool -l "$1" | awk '/cmd LC_UUID/ {getline; print $2}'
}

bundle_external_dependency() {
  local binary="$1"
  local dependency="$2"
  local name="${dependency##*/}"
  local destination
  local current_id
  [[ -f "$dependency" ]] || fail "Runtime dependency is missing: $dependency"
  for destination in "$runtime_dir/$name" "$frameworks/$name"; do
    if [[ -e "$destination" ]]; then
      [[ "$(macho_uuid "$destination")" == "$(macho_uuid "$dependency")" ]] || \
        fail "Conflicting runtime dependency named $name"
    else
      cp -L "$dependency" "$destination"
      copied_external_dependency=1
    fi
    current_id="$(otool -D "$destination" | sed -n '2p')"
    if [[ "$current_id" != "@rpath/$name" ]]; then
      install_name_tool -id "@rpath/$name" "$destination"
    fi
    ensure_rpath "$destination" @loader_path
  done
  install_name_tool -change "$dependency" "@rpath/$name" "$binary"
}

bundle_external_dependencies() {
  local binary dependency
  while true; do
    copied_external_dependency=0
    for binary in "$bridge" "$helpers/ffmpeg" "$helpers/ffprobe" \
      "$runtime_dir"/*.dylib "$frameworks"/*.dylib; do
      [[ -f "$binary" && ! -L "$binary" ]] || continue
      while IFS= read -r dependency; do
        case "$dependency" in
          @*|/usr/lib/*|/System/Library/*) continue ;;
          "$ffmpeg_prefix"/*)
            fail "FFmpeg install-prefix dependency remains in $binary: $dependency"
            ;;
          *) bundle_external_dependency "$binary" "$dependency" ;;
        esac
      done < <(otool -L "$binary" | awk 'NR > 1 {print $1}')
    done
    [[ "$copied_external_dependency" -eq 1 ]] || break
  done
}

bundle_external_dependencies

codesign --force --deep --sign - --timestamp=none --preserve-metadata=entitlements "$app"
codesign --verify --deep --strict "$app"
# The Developer Preview runs unsandboxed: the in-editor assistant starts the
# local DeepSeek Harness and its `or mcp` bridge, which App Sandbox cannot
# grant (see docs/adr/0009-assistant-panel-and-harness-boundary.md). A stale
# sandbox entitlement would still be a signing regression, so assert its
# absence rather than its presence.
sandbox_entitlement="$(codesign --display --entitlements - --xml "$app" 2>/dev/null | \
  python3 -c 'import plistlib, sys; print(plistlib.loads(sys.stdin.buffer.read()).get("com.apple.security.app-sandbox", False))')"
[[ "$sandbox_entitlement" != True ]] || fail "The app is still sandboxed, so the local assistant host cannot start: $app"

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
for library in "$runtime_dir"/*.dylib "$frameworks"/*.dylib; do
  [[ -f "$library" && ! -L "$library" ]] || continue
  check_relocated "$library"
done

check_bundled_dependencies() {
  local binary dependency
  for binary in "$bridge" "$helpers/ffmpeg" "$helpers/ffprobe" \
    "$runtime_dir"/*.dylib "$frameworks"/*.dylib; do
    [[ -f "$binary" && ! -L "$binary" ]] || continue
    while IFS= read -r dependency; do
      case "$dependency" in
        @*|/usr/lib/*|/System/Library/*) ;;
        *) fail "Non-system absolute runtime dependency remains in $binary: $dependency" ;;
      esac
    done < <(otool -L "$binary" | awk 'NR > 1 {print $1}')
  done
}

check_bundled_dependencies

encoders="$(env -u DYLD_LIBRARY_PATH -u DYLD_FALLBACK_LIBRARY_PATH \
  -u DYLD_FRAMEWORK_PATH -u DYLD_ROOT_PATH \
  "$helpers/ffmpeg" -hide_banner -encoders)"
printf '%s\n' "$encoders" | grep -Fq h264_videotoolbox || fail "Bundled FFmpeg has no VideoToolbox H.264 encoder"
printf '%s\n' "$encoders" | grep -Eq '[[:space:]]aac[[:space:]]' || fail "Bundled FFmpeg has no AAC encoder"
env -u DYLD_LIBRARY_PATH -u DYLD_FALLBACK_LIBRARY_PATH \
  -u DYLD_FRAMEWORK_PATH -u DYLD_ROOT_PATH \
  "$helpers/ffprobe" -hide_banner -version >/dev/null
probe_decoders="$(env -u DYLD_LIBRARY_PATH -u DYLD_FALLBACK_LIBRARY_PATH \
  -u DYLD_FRAMEWORK_PATH -u DYLD_ROOT_PATH \
  "$helpers/ffprobe" -hide_banner -decoders)"
for codec in png mjpeg webp; do
  if ! printf '%s\n' "$probe_decoders" | awk -v codec="$codec" \
    '$1 ~ /^V/ && $2 == codec {found = 1} END {exit !found}'; then
    fail "Bundled FFmpeg cannot probe the required $codec still-image codec"
  fi
done
probe_formats="$(env -u DYLD_LIBRARY_PATH -u DYLD_FALLBACK_LIBRARY_PATH \
  -u DYLD_FRAMEWORK_PATH -u DYLD_ROOT_PATH \
  "$helpers/ffprobe" -hide_banner -formats)"
for demuxer in image2 image2pipe; do
  if ! printf '%s\n' "$probe_formats" | awk -v format="$demuxer" \
    '$1 ~ /D/ && $2 == format {found = 1} END {exit !found}'; then
    fail "Bundled FFmpeg cannot read the required $demuxer image format"
  fi
done
env -u DYLD_LIBRARY_PATH -u DYLD_FALLBACK_LIBRARY_PATH \
  -u DYLD_FRAMEWORK_PATH -u DYLD_ROOT_PATH \
  python3 -c 'import ctypes, sys; ctypes.CDLL(sys.argv[1])' "$bridge"

echo "Packaged and verified local macOS app: $app"
