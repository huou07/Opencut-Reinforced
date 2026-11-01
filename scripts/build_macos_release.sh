#!/usr/bin/env bash
# Build the local macOS release bundle without shipping a stale Rust bridge.
#
# Why this exists: `or_app_bridge` is compiled by Flutter's native-assets hook,
# which keeps its own cargo target directory under
# apps/or_app/.dart_tool/hooks_runner/. A `flutter_rust_bridge_codegen` change
# regenerates the Dart bindings and the Rust glue, but the hook can hand back the
# previously compiled dylib, so the app starts with a "Content hash on Dart side
# is different from Rust side" error. Clearing the hook cache only when the
# generated content hash actually changed keeps normal builds fast while making
# the broken combination impossible to ship by accident.
#
# Usage:
#   scripts/build_macos_release.sh                 # build only
#   scripts/build_macos_release.sh --package <bundle-id>
#                                                  # build, then package and codesign
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
APP="$ROOT/apps/or_app"
RUST_GLUE="$ROOT/crates/or_app_bridge/src/frb_generated.rs"
DART_GLUE="$ROOT/packages/or_app_bridge/lib/src/rust/frb_generated.dart"
MARKER="$APP/.dart_tool/or_bridge_hash"
FFMPEG_PREFIX="${OR_FFMPEG_PREFIX:-$HOME/Library/Caches/OpencutReinforced/ffmpeg-8.1.3-macos-image-support/ffmpeg-install}"

package_id=""
if [[ "${1:-}" == "--package" ]]; then
  package_id="${2:?--package needs a bundle identifier}"
fi

rust_hash="$(grep -oE 'FLUTTER_RUST_BRIDGE_CODEGEN_CONTENT_HASH: i32 = -?[0-9]+' "$RUST_GLUE" | grep -oE -- '-?[0-9]+$')"
dart_hash="$(grep -oE 'rustContentHash => -?[0-9]+' "$DART_GLUE" | grep -oE -- '-?[0-9]+$')"
if [[ -z "$rust_hash" || -z "$dart_hash" ]]; then
  echo "error: could not read the bridge content hash from the generated files" >&2
  exit 1
fi
if [[ "$rust_hash" != "$dart_hash" ]]; then
  echo "error: generated Dart ($dart_hash) and Rust ($rust_hash) bindings disagree;" >&2
  echo "       run flutter_rust_bridge_codegen generate again" >&2
  exit 1
fi

app_version="$(sed -nE 's/^version:[[:space:]]*([^+[:space:]]+).*/\1/p' "$APP/pubspec.yaml" | head -1)"
marker_value="$rust_hash:$app_version"
built_hash=""
[[ -f "$MARKER" ]] && built_hash="$(cat "$MARKER")"
if [[ "$built_hash" != "$marker_value" ]]; then
  echo "bridge hash or app version changed ($built_hash -> $marker_value): clearing the native-assets hook cache"
  rm -rf "$APP/.dart_tool/hooks_runner/shared/or_app_bridge/build" "$APP/.dart_tool/flutter_build"
fi

(cd "$APP" && flutter --no-version-check build macos --release)
printf '%s' "$marker_value" > "$MARKER"

if [[ -n "$package_id" ]]; then
  # The packaged bundle carries the `or` CLI the assistant edits through, so it
  # must match the app build rather than a stale binary from an earlier run.
  (cd "$ROOT" && cargo build --release -p or_cli)
  staged="$(mktemp -d)/or_app.app"
  ditto "$APP/build/macos/Build/Products/Release/or_app.app" "$staged"
  /usr/libexec/PlistBuddy -c "Set :CFBundleIdentifier $package_id" "$staged/Contents/Info.plist"
  bash "$ROOT/scripts/package_local_macos_app.sh" "$staged" "$FFMPEG_PREFIX"
fi
