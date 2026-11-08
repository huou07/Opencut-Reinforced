#!/usr/bin/env bash
# Builds the bundled whisper-cli for the macOS release.
#
# The release targets macOS 13, so the helper must not inherit a newer SDK's
# default deployment target. A debug bundle built on a macOS 26 host carries
# `minos 26.0`, and the DMG script refuses it; this script pins the target and
# verifies the result instead of shipping a helper that only runs on new hosts.
#
# Usage: scripts/build_macos_whisper_cli.sh <whisper.cpp-src> <output-path>
# Env:   OR_WHISPER_DEPLOYMENT_TARGET (default 13.0)
set -euo pipefail

src="${1:-${WHISPER_CPP_SRC:-}}"
out="${2:-${OR_WHISPER_CLI_PATH:-}}"
target="${OR_WHISPER_DEPLOYMENT_TARGET:-13.0}"

if [[ -z "$src" || ! -f "$src/CMakeLists.txt" ]]; then
  echo "error: pass the whisper.cpp source directory (v1.9.4) as the first argument." >&2
  exit 1
fi
if [[ -z "$out" ]]; then
  echo "error: pass the output path for whisper-cli as the second argument." >&2
  exit 1
fi

build="$(mktemp -d "${TMPDIR:-/tmp}/or-whisper-build.XXXXXX")"
trap 'rm -rf "$build"' EXIT

cmake -S "$src" -B "$build" \
  -DCMAKE_BUILD_TYPE=Release \
  -DCMAKE_OSX_DEPLOYMENT_TARGET="$target" \
  -DWHISPER_BUILD_EXAMPLES=ON \
  -DWHISPER_BUILD_TESTS=OFF \
  -DBUILD_SHARED_LIBS=OFF \
  -DGGML_METAL=ON \
  -DGGML_METAL_EMBED_LIBRARY=ON >/dev/null
cmake --build "$build" --config Release -j "$(sysctl -n hw.ncpu)" --target whisper-cli >/dev/null

built="$build/bin/whisper-cli"
[[ -x "$built" ]] || { echo "error: whisper-cli was not produced in $build/bin." >&2; exit 1; }

minos="$(otool -l "$built" | awk '/LC_BUILD_VERSION/{found=1} found && /minos/{print $2; exit}')"
if [[ -z "$minos" ]]; then
  echo "error: cannot read the deployment target of $built." >&2
  exit 1
fi
# Compare as major.minor: the helper must run on the oldest target we ship.
if [[ "$(printf '%s\n%s\n' "$target" "$minos" | sort -V | head -1)" != "$target" ]]; then
  echo "error: whisper-cli requires macOS $minos, expected $target or earlier." >&2
  exit 1
fi

mkdir -p "$(dirname "$out")"
cp "$built" "$out"
chmod +x "$out"
"$out" --help >/dev/null
echo "built whisper-cli $minos at $out"
