#!/usr/bin/env bash
# Build the distributable Apple Silicon macOS disk image.
#
# Usage: scripts/build_macos_dmg.sh
# Env: OR_FFMPEG_PREFIX overrides the bundled FFmpeg install prefix.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
APP_NAME="Opencut Reinforced"
BUNDLE_ID="io.github.huou07.orApp"
APP_BUILD="$ROOT/apps/or_app/build/macos/Build/Products/Release/or_app.app"
FFMPEG_PREFIX="${OR_FFMPEG_PREFIX:-$HOME/Library/Caches/OpencutReinforced/ffmpeg-8.1.3-macos-image-support/ffmpeg-install}"
VERSION="$(sed -nE 's/^version:[[:space:]]*([^+[:space:]]+).*/\1/p' "$ROOT/apps/or_app/pubspec.yaml" | head -1)"
[[ -n "$VERSION" ]] || VERSION="0.0.0"
ARCH="$(uname -m)"
[[ "$ARCH" == "arm64" ]] || { echo "error: the distributable image must be built on Apple Silicon" >&2; exit 1; }
[[ -f "$FFMPEG_PREFIX/lib/pkgconfig/libavutil.pc" ]] || {
  echo "error: no pinned FFmpeg prefix at $FFMPEG_PREFIX" >&2
  echo "       run scripts/stage_macos_ffmpeg.sh first (Homebrew's GPL FFmpeg is rejected)" >&2
  exit 1
}

# The distributable app must carry the local transcription runtime; otherwise the
# Speech model picker advertises models it can never install or run. Prefer an
# explicit path, then the helpers a debug bundle already carries, then PATH, and
# build the provider adapter from this checkout when it is not supplied.
whisper_cli="${OR_WHISPER_CLI_PATH:-}"
if [[ -z "$whisper_cli" ]]; then
  for candidate in \
    "$ROOT/apps/or_app/build/macos/Build/Products/Debug/or_app.app/Contents/MacOS/whisper-cli" \
    "$ROOT/apps/or_app/build/macos/Build/Products/Release/or_app.app/Contents/MacOS/whisper-cli"; do
    if [[ -x "$candidate" ]]; then whisper_cli="$candidate"; break; fi
  done
fi
if [[ -z "$whisper_cli" ]]; then
  whisper_cli="$(command -v whisper-cli || true)"
fi
[[ -n "$whisper_cli" && -x "$whisper_cli" ]] || {
  echo "error: whisper-cli was not found. Set OR_WHISPER_CLI_PATH or build whisper.cpp v1.9.4." >&2
  exit 1
}

whisper_provider="${OR_WHISPER_PROVIDER_PATH:-$ROOT/target/release/or-whisper-provider}"
if [[ ! -x "$whisper_provider" ]]; then
  (cd "$ROOT" && cargo build --locked --release -p or_whisper_provider)
fi
[[ -x "$whisper_provider" ]] || {
  echo "error: the whisper provider adapter could not be built." >&2
  exit 1
}

export OR_WHISPER_CLI_PATH="$whisper_cli"
export OR_WHISPER_PROVIDER_PATH="$whisper_provider"
export OR_WHISPER_LICENSE_PATH="${OR_WHISPER_LICENSE_PATH:-}"
export OR_REQUIRE_TRANSCRIPTION=1

"$ROOT/scripts/build_macos_release.sh"
app_archs="$(lipo -archs "$APP_BUILD/Contents/MacOS/or_app")"
[[ "$app_archs" == "arm64" ]] || {
  echo "error: the distributable app must be arm64 only (found: $app_archs)" >&2
  exit 1
}
(cd "$ROOT" && cargo build --release -p or_cli)

STAGE="$ROOT/dist/dmg-stage"
rm -rf "$STAGE"
mkdir -p "$STAGE"
ditto "$APP_BUILD" "$STAGE/$APP_NAME.app"
/usr/libexec/PlistBuddy -c "Set :CFBundleIdentifier $BUNDLE_ID" "$STAGE/$APP_NAME.app/Contents/Info.plist"

bash "$ROOT/scripts/package_local_macos_app.sh" "$STAGE/$APP_NAME.app" "$FFMPEG_PREFIX"

ln -s /Applications "$STAGE/Applications"
cat > "$STAGE/Read me first.txt" <<'TEXT'
Opencut Reinforced — macOS (Apple Silicon)

1. Drag "Opencut Reinforced" into the Applications folder.
2. Open Applications, right-click "Opencut Reinforced", choose Open, then Open
   again. macOS asks this once because the app is signed locally, not notarized.
3. The app creates its own project library and installs the assistant workspace
   and the `or` command on first launch. No terminal is needed.
4. Local captions: open a project, choose Audio, then Speech model, and press
   Download once. The model stays on this Mac.
5. Optional: Generate voiceover can download a local speech server with one
   click. In Assistant settings, pick DSH, Claude Code, Codex, OpenCode or your
   own ACP harness.
6. Settings -> Resources & updates shows the bundled runtime, model and harness
   status, and checks public GitHub releases. macOS 13 or later is required.

Do not move the app out of Applications after using it; the assistant and the
local server are started from the app bundle.
TEXT

DMG="$ROOT/dist/Opencut-Reinforced-${VERSION}-arm64.dmg"
rm -f "$DMG"
hdiutil create -volname "$APP_NAME" -srcfolder "$STAGE" -ov -format UDZO "$DMG"
hdiutil verify "$DMG"
echo "$DMG"
