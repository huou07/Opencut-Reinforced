#!/usr/bin/env bash
# Installs the local transcription runtime into a macOS app bundle so a plain
# local debug build can generate captions without a Developer Preview package.
#
# The app looks for `whisper-cli` and `or-whisper-provider` next to its own
# executable. This script copies both there, re-signs the bundle ad hoc, and can
# install one verified model artifact into the app's private model folder.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
APP="${1:-$ROOT/apps/or_app/build/macos/Build/Products/Debug/or_app.app}"
MODEL_FILE=""

usage() {
  cat <<EOF
Usage:
  scripts/install_local_transcription_helpers.sh [APP_PATH] [--model-file PATH]

  APP_PATH            The .app bundle to update (default: debug build).
  --model-file PATH   Install this ggml-small.bin after verifying its digest.
  --help              Show this text.

The whisper-cli binary is taken from OR_WHISPER_CLI_PATH when set, otherwise
from a packaged local OpenCut Reinforced app, otherwise from PATH. Build it from
whisper.cpp v1.9.4 when none is available; this script never compiles whisper.cpp
itself.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --model-file)
      MODEL_FILE="${2:?--model-file needs a path}"
      shift 2
      ;;
    --help|-h)
      usage
      exit 0
      ;;
    *)
      APP="$1"
      shift
      ;;
  esac
done

die() {
  printf 'install_local_transcription_helpers: %s\n' "$1" >&2
  exit 1
}

[[ "$(uname -s)" == "Darwin" ]] || die "this helper is macOS-only"
[[ -d "$APP" ]] || die "app bundle not found: $APP"
[[ -d "$APP/Contents/MacOS" ]] || die "not a macOS app bundle: $APP"

find_whisper_cli() {
  if [[ -n "${OR_WHISPER_CLI_PATH:-}" ]]; then
    printf '%s\n' "$OR_WHISPER_CLI_PATH"
    return
  fi
  local candidate
  for candidate in \
    "$HOME"/Downloads/opencut-reinforced-macos-*/OpenCut\ Reinforced.app/Contents/MacOS/whisper-cli \
    "$HOME"/Downloads/opencut-reinforced-macos-*/or_app.app/Contents/MacOS/whisper-cli \
    /Applications/OpenCut\ Reinforced.app/Contents/MacOS/whisper-cli \
    /Applications/Opencut\ Reinforced.app/Contents/MacOS/whisper-cli; do
    if [[ -x "$candidate" ]]; then
      printf '%s\n' "$candidate"
      return
    fi
  done
  if command -v whisper-cli >/dev/null 2>&1; then
    command -v whisper-cli
    return
  fi
  return 1
}

WHISPER_CLI="$(find_whisper_cli || true)"
if [[ -z "$WHISPER_CLI" || ! -x "$WHISPER_CLI" ]]; then
  die "whisper-cli was not found. Set OR_WHISPER_CLI_PATH, or build whisper.cpp v1.9.4 with -DWHISPER_BUILD_EXAMPLES=ON and rerun."
fi

printf 'Using whisper-cli: %s\n' "$WHISPER_CLI"
"$WHISPER_CLI" --version | head -1

printf 'Building the provider adapter...\n'
cargo build --locked --release -p or_whisper_provider --manifest-path "$ROOT/Cargo.toml"
PROVIDER="$ROOT/target/release/or-whisper-provider"
[[ -x "$PROVIDER" ]] || die "provider build did not produce $PROVIDER"

install -m 755 "$WHISPER_CLI" "$APP/Contents/MacOS/whisper-cli"
install -m 755 "$PROVIDER" "$APP/Contents/MacOS/or-whisper-provider"

printf 'Re-signing the bundle ad hoc...\n'
codesign --force --sign - --preserve-metadata=entitlements "$APP" >/dev/null
codesign --verify --deep --strict "$APP"

if [[ -n "$MODEL_FILE" ]]; then
  [[ -f "$MODEL_FILE" ]] || die "model file not found: $MODEL_FILE"
  expected="1be3a9b2063867b937e64e2ec7483364a79917e157fa98c5d94b5c1fffea987b"
  actual="$(shasum -a 256 "$MODEL_FILE" | awk '{print $1}')"
  [[ "$actual" == "$expected" ]] || die "model digest mismatch: expected $expected, got $actual"
  models="$HOME/Library/Application Support/Opencut Reinforced/Models"
  mkdir -p "$models"
  destination="$models/26-whisper-small-multilingual-3-1.0.model"
  install -m 644 "$MODEL_FILE" "$destination"
  printf 'Installed the small model at: %s\n' "$destination"
fi

cat <<EOF
Local transcription helpers installed in:
  $APP

Quit and reopen the app, then use Generate Captions. If the model is not
installed yet, the app offers the download on that first use.
EOF
