#!/usr/bin/env bash
# Produces the glTF fixture the companion journey imports.
#
# The model is drawn by Chromium through three.js, one PNG per frame, and FFmpeg
# turns the frames into a clip. OR holds no glTF renderer: the .gltf file stays
# the source and `or tools provenance write` records it beside the render, so the
# model can be reopened in whatever produced it.
#
# Usage: bash scripts/make_gltf_fixture.sh [path/to/or]
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
or="${1:-$root/target/release/or}"
renderer="$root/scripts/gltf-render"
model="$renderer/or-cube.gltf"
frames="$root/target/gltf-fixture-frames"
out="$root/scripts/ui_scenarios/companion-gltf-cube.mp4"

[ -x "$or" ] || { echo "missing or binary: $or" >&2; exit 2; }
command -v node >/dev/null || { echo "missing node" >&2; exit 2; }

if [ ! -d "$renderer/node_modules/three" ]; then
  echo "installing the glTF renderer in $renderer"
  (cd "$renderer" && npm install --no-audit --no-fund >/dev/null)
fi

rm -rf "$frames"
(cd "$renderer" && node render.js "$model" "$frames" 90 640 360 >/dev/null)

"$or" tools run ffmpeg -- -y -hide_banner -loglevel error \
  -framerate 30 -i "$frames/frame-%04d.png" \
  -c:v libx264 -pix_fmt yuv420p -crf 20 "$out" >/dev/null

version="$(node -p "require('$renderer/node_modules/three/package.json').version")"
"$or" tools provenance write --tool gltf --output "$out" --manifest "$model" \
  --version "$version" -- node "scripts/gltf-render/render.js" \
  "$model" "$frames" 90 640 360 >/dev/null

ls -l "$out" "$out.orprovenance.json"
