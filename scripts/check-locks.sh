#!/usr/bin/env bash
# Resolve every committed Cargo lock file without writing to it.
#
# A workspace member version bump updates the root `Cargo.lock` and leaves
# `tools/ffmpeg-link-probe/Cargo.lock` stale. The hosted Rust lane runs the link
# probe with `--locked`, so that drift fails there and nowhere else; this gate
# is the local equivalent, and `cargo metadata` is enough to prove it without
# building anything.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

checked=0
while IFS= read -r manifest; do
  printf 'lock: %s\n' "${manifest%/*}"
  cargo metadata --locked --offline --format-version 1 --manifest-path "$manifest" >/dev/null
  checked=$((checked + 1))
done < <(git ls-files '*Cargo.toml' | grep -v '^target/' | while IFS= read -r file; do
  dir="${file%/*}"
  [[ "$dir" == "$file" ]] && dir='.'
  [[ -f "$dir/Cargo.lock" ]] && printf '%s\n' "$dir/Cargo.toml"
done)

if (( checked == 0 )); then
  printf 'error: no locked workspace was found\n' >&2
  exit 1
fi
printf 'lock files resolve without a write: %s workspaces\n' "$checked"
