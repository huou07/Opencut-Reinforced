#!/usr/bin/env bash
# Produces the Lottie fixture the companion journey imports.
#
# The fixture is a real render, not a stored sample: the animation in
# `scripts/lottie-render/or-badge.json` is drawn by Chromium through
# puppeteer-lottie and encoded by FFmpeg, and the record beside the result is
# written by `or tools provenance write` with the command that ran. Running this
# on another host therefore re-records that host's Node, Chrome and FFmpeg.
#
# Usage: bash scripts/make_lottie_fixture.sh [path/to/or]
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
or="${1:-$root/target/release/or}"
renderer="$root/scripts/lottie-render"
animation="$renderer/or-badge.json"
out="$root/scripts/ui_scenarios/companion-lottie-badge.mp4"

[ -x "$or" ] || { echo "missing or binary: $or" >&2; exit 2; }
command -v node >/dev/null || { echo "missing node" >&2; exit 2; }

if [ ! -d "$renderer/node_modules/puppeteer-lottie" ]; then
  echo "installing the Lottie renderer in $renderer"
  (cd "$renderer" && npm install --no-audit --no-fund >/dev/null)
fi

node_version="$(node --version)"
arguments=("$animation" "$out" 640 360)
(cd "$root" && node "$renderer/render.js" "${arguments[@]}" >/dev/null)

record=(tools provenance write --tool lottie --output "$out"
        --manifest "$animation" --version "$node_version"
        -- node "scripts/lottie-render/render.js" "${arguments[@]}")
"$or" "${record[@]}" >/dev/null

ls -l "$out" "$out.orprovenance.json"
