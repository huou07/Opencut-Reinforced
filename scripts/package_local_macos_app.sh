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
# Start from a clean FFmpeg surface. A previous package may have left another
# ABI (or a GPL build's codec libraries) in these folders, and loading is by
# install name: a stale libavcodec beside a newly linked one is a runtime error
# waiting for the next rebuild.
for directory in "$runtime_dir" "$frameworks"; do
  rm -f "$directory"/libavcodec*.dylib "$directory"/libavformat*.dylib \
    "$directory"/libavutil*.dylib "$directory"/libswresample*.dylib \
    "$directory"/libswscale*.dylib "$directory"/libavfilter*.dylib \
    "$directory"/libavdevice*.dylib "$directory"/libx264*.dylib \
    "$directory"/libx265*.dylib "$directory"/libSvtAv1Enc*.dylib \
    "$directory"/libvpx*.dylib "$directory"/libopus*.dylib \
    "$directory"/libdav1d*.dylib "$directory"/libmp3lame*.dylib \
    "$directory"/libvmaf*.dylib "$directory"/libmpg123*.dylib \
    "$directory"/libssl*.dylib "$directory"/libcrypto*.dylib \
    "$directory"/liblzma*.dylib
done
# libavdevice is optional: the minimal LGPL build the app links does not carry
# it, while a Homebrew FFmpeg's `ffmpeg` CLI does. Packaging either build must
# work, so the pattern is copied when present and skipped when absent.
for pattern in libavcodec*.dylib libavformat*.dylib libavutil*.dylib \
  libswresample*.dylib libswscale*.dylib libavfilter*.dylib \
  libavdevice*.dylib; do
  found=0
  for source in "$ffmpeg_lib_dir"/$pattern; do
    [[ -e "$source" ]] || continue
    found=1
    cp -a "$source" "$runtime_dir/"
    cp -a "$source" "$frameworks/"
  done
  [[ "$found" -eq 1 || "$pattern" == libavdevice*.dylib ]] || \
    fail "No FFmpeg library matched $ffmpeg_lib_dir/$pattern"
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
  "$runtime_dir"/libavdevice*.dylib \
  "$frameworks"/libavcodec*.dylib "$frameworks"/libavformat*.dylib \
  "$frameworks"/libavutil*.dylib "$frameworks"/libswresample*.dylib \
  "$frameworks"/libswscale*.dylib "$frameworks"/libavfilter*.dylib \
  "$frameworks"/libavdevice*.dylib; do
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
            # A library that was copied above keeps its install name; point the
            # binary at the bundled copy instead of leaving the build prefix in
            # its load commands.
            name="${dependency##*/}"
            if [[ -f "$runtime_dir/$name" ]]; then
              install_name_tool -change "$dependency" "@rpath/$name" "$binary"
            else
              fail "FFmpeg install-prefix dependency remains in $binary: $dependency"
            fi
            ;;
          *) bundle_external_dependency "$binary" "$dependency" ;;
        esac
      done < <(otool -L "$binary" | awk 'NR > 1 {print $1}')
    done
    [[ "$copied_external_dependency" -eq 1 ]] || break
  done
}

bundle_external_dependencies

# The app refuses to run media unless the loaded runtime reports the approved
# license, so packaging must prove it here rather than ship a GPL build that
# fails the first time a user opens the viewer or exports. libavutil has no
# non-system dependencies, which makes a ctypes load safe at package time.
bundled_license="$(python3 - "$frameworks/libavutil.dylib" <<'PY'
import ctypes
import sys

library = ctypes.CDLL(sys.argv[1])
library.avutil_license.restype = ctypes.c_char_p
print(library.avutil_license().decode())
PY
)" || fail "could not read the bundled FFmpeg license"
[[ "$bundled_license" == "LGPL version 2.1 or later" ]] || \
  fail "bundled FFmpeg is not the approved LGPL 2.1-or-later build: $bundled_license"

# Optional local transcription runtime. The release pipeline builds a pinned
# whisper.cpp and the provider adapter and points these variables at them; a
# local release bundle may reuse a debug bundle's helpers. Packaging is not
# allowed to silently omit the runtime when the caller asked for it.
whisper_cli="${OR_WHISPER_CLI_PATH:-}"
whisper_provider="${OR_WHISPER_PROVIDER_PATH:-}"
whisper_license="${OR_WHISPER_LICENSE_PATH:-}"
if [[ -n "$whisper_cli" || -n "$whisper_provider" ]]; then
  [[ -n "$whisper_cli" && -n "$whisper_provider" ]] || \
    fail "set both OR_WHISPER_CLI_PATH and OR_WHISPER_PROVIDER_PATH"
  [[ -x "$whisper_cli" ]] || fail "whisper CLI is missing or not executable: $whisper_cli"
  [[ -x "$whisper_provider" ]] || fail "whisper provider is missing or not executable: $whisper_provider"
  install -m 755 "$whisper_cli" "$helpers/whisper-cli"
  install -m 755 "$whisper_provider" "$helpers/or-whisper-provider"
  notices="$app/Contents/Resources/ThirdPartyNotices/whisper.cpp"
  mkdir -p "$notices"
  if [[ -n "$whisper_license" && -f "$whisper_license" ]]; then
    cp "$whisper_license" "$notices/LICENSE"
  else
    cat > "$notices/LICENSE" <<'EOF'
MIT License

Copyright (c) 2023-2024 The ggml authors

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
EOF
  fi
  cat > "$notices/NOTICE.txt" <<'EOF'
whisper.cpp v1.9.4 is distributed under the included MIT license. The
whisper-cli helper is built from that pinned source; the OR provider adapter is
built from this repository. Model weights are not bundled and are downloaded
only after the user confirms the size, source and licence in the app.
EOF
  env -u DYLD_LIBRARY_PATH -u DYLD_FALLBACK_LIBRARY_PATH \
    -u DYLD_FRAMEWORK_PATH -u DYLD_ROOT_PATH \
    "$helpers/whisper-cli" --version >/dev/null
  test -x "$helpers/or-whisper-provider"
elif [[ "${OR_REQUIRE_TRANSCRIPTION:-0}" == "1" ]]; then
  fail "OR_REQUIRE_TRANSCRIPTION=1 but no whisper helpers were supplied"
fi

# Keep the FFmpeg licence notice with the libraries this bundle relocates. The
# app dynamically links FFmpeg, so the LGPL text and an accurate notice must
# travel with the packaged artifact rather than only living in the build tree.
ffmpeg_notices="$app/Contents/Resources/ThirdPartyNotices/FFmpeg"
mkdir -p "$ffmpeg_notices"
if [[ -f "$ffmpeg_prefix/COPYING.LGPLv2.1" ]]; then
  cp "$ffmpeg_prefix/COPYING.LGPLv2.1" "$ffmpeg_notices/COPYING.LGPLv2.1"
fi
if [[ -d "$ffmpeg_prefix/ThirdPartyNotices" ]]; then
  cp -R "$ffmpeg_prefix/ThirdPartyNotices/." "$app/Contents/Resources/ThirdPartyNotices/"
fi
cat > "$ffmpeg_notices/FFmpeg-NOTICE.txt" <<'EOF'
This application dynamically links FFmpeg under LGPL-2.1-or-later.
FFmpeg shared libraries are separate files beside the Rust bridge and may be
replaced with a compatible LGPL-compliant build under the applicable license
terms. The ffmpeg and ffprobe programs beside the application binary are built
from the same source and configuration for import, media previews and WebM
export. FFmpeg's WebM export statically links libvpx and libopus.
EOF

# A bundle built on a newer Mac must still launch on the documented minimum.
# clang records the SDK's default deployment target in every Mach-O; without
# this gate a locally built FFmpeg/whisper helper silently raises the app's
# minimum macOS version even though the Xcode project says 13.0.
python3 - "$app" "${OR_MACOS_DEPLOYMENT_TARGET:-13.0}" <<'PY'
import re
import subprocess
import sys
from pathlib import Path

bundle = Path(sys.argv[1])
target = tuple(int(part) for part in sys.argv[2].split(".")[:2])
for path in sorted(bundle.rglob("*")):
    if not path.is_file() or path.is_symlink():
        continue
    kind = subprocess.run(
        ["file", "-b", str(path)], capture_output=True, text=True
    ).stdout
    if "Mach-O" not in kind:
        continue
    output = subprocess.run(
        ["otool", "-l", str(path)], capture_output=True, text=True
    ).stdout
    minimums = re.findall(r"LC_BUILD_VERSION.*?\n.*?minos\s+([0-9.]+)", output, re.S)
    minimums += re.findall(
        r"LC_VERSION_MIN_MACOSX.*?\n.*?version\s+([0-9.]+)", output, re.S
    )
    for minimum in minimums:
        actual = tuple(int(part) for part in minimum.split(".")[:2])
        if actual > target:
            raise SystemExit(
                f"{path.relative_to(bundle)} requires macOS {minimum}, "
                f"expected {sys.argv[2]} or earlier"
            )
PY

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
muxers="$(env -u DYLD_LIBRARY_PATH -u DYLD_FALLBACK_LIBRARY_PATH \
  -u DYLD_FRAMEWORK_PATH -u DYLD_ROOT_PATH \
  "$helpers/ffmpeg" -hide_banner -muxers)"
for muxer in matroska mov mp4 webm image2 image2pipe wav; do
  if ! printf '%s\n' "$muxers" | awk -v muxer="$muxer" \
    '$1 ~ /^E/ && $2 == muxer {found = 1} END {exit !found}'; then
    fail "Bundled FFmpeg has no required $muxer muxer"
  fi
done
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
