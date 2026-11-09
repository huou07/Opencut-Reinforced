#!/usr/bin/env bash
# Produces the companion-tool fixture the provenance journey imports.
#
# The fixture is not a hand-written sample: it is rendered by the catalog's own
# FFmpeg through `or tools run`, and the record beside it is written by
# `or tools provenance write` with the exact argument vector that ran. Running
# this on another host therefore re-records that host's FFmpeg version and
# paths, which is what the journey then displays.
#
# Usage: bash scripts/make_provenance_fixture.sh [path/to/or]
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
or="${1:-$root/target/release/or}"
out="$root/scripts/ui_scenarios/companion-ffmpeg-shot.mp4"
manifest="$root/scripts/ui_scenarios/companion-ffmpeg-shot.filtergraph"

[ -x "$or" ] || { echo "missing or binary: $or" >&2; exit 2; }

cat > "$manifest" <<'EOF'
color=c=#3FC7FF:s=320x180:r=30:d=2[bg];
[bg]drawtext=text='Rendered by FFmpeg':fontcolor=black:fontsize=20:x=(w-tw)/2:y=(h-th)/2[out]
EOF

version="$("$or" tools check ffmpeg --json | python3 -c 'import json,sys; print(json.load(sys.stdin)["version"] or "")')"

args=(-y -f lavfi -i "testsrc2=size=320x180:rate=30" -t 2 -c:v libx264
      -pix_fmt yuv420p -crf 30 "$out")
"$or" tools run ffmpeg -- "${args[@]}" >/dev/null 2>&1

record=(tools provenance write --tool ffmpeg --output "$out"
        --manifest "$manifest")
[ -n "$version" ] && record+=(--version "$version")
record+=(-- ffmpeg "${args[@]}")
"$or" "${record[@]}" >/dev/null

ls -l "$out" "$out.orprovenance.json"
