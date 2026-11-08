#!/usr/bin/env bash
set -euo pipefail

if [[ "$#" -ne 3 ]]; then
  echo "usage: package_linux_flatpak.sh <linux-bundle> <output.flatpak> <40-character-source-sha>" >&2
  exit 2
fi

bundle="$1"
output="$2"
source_sha="$3"
app_id="io.github.huou07.or_app"
script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd -- "$script_dir/.." && pwd)"
packaging="$repo_root/flatpak"

[[ "$source_sha" =~ ^[0-9a-f]{40}$ ]] || { echo "source SHA must be 40 lowercase hexadecimal characters" >&2; exit 2; }

missing=()
require_file() {
  local path="$1" kind="$2"
  if [[ ! -e "$path" ]]; then
    missing+=("missing: $path")
  elif [[ "$kind" == "executable" && ! -x "$path" ]]; then
    missing+=("not executable: $path")
  elif [[ "$kind" == "non-empty" && ! -s "$path" ]]; then
    missing+=("empty: $path")
  fi
}
require_tool() {
  command -v "$1" >/dev/null 2>&1 || missing+=("missing tool on PATH: $1")
}

[[ -d "$bundle" ]] || {
  echo "packaging failed: the Flutter Linux bundle does not exist: $bundle" >&2
  echo "  build it first: (cd apps/or_app && flutter build linux --release)" >&2
  exit 1
}

require_file "$bundle/or_app" executable
require_file "$bundle/ffmpeg" executable
require_file "$bundle/ffprobe" executable
require_file "$packaging/io.github.huou07.or_app.yml" non-empty
require_file "$packaging/io.github.huou07.or_app.metainfo.xml" non-empty
require_file "$packaging/io.github.huou07.or_app.desktop" non-empty
require_file "$packaging/icon.png" non-empty
require_tool flatpak
require_tool flatpak-builder
require_tool appstreamcli

if ! bash "$script_dir/stage_codec_licences.sh" verify "$bundle"; then
  missing+=("the packaged licence tree is incomplete (listed above)")
fi

if (( ${#missing[@]} )); then
  echo "packaging failed: $(( ${#missing[@]} )) precondition(s) not met for $bundle" >&2
  printf '  %s\n' "${missing[@]}" >&2
  exit 1
fi

appstreamcli validate --no-net "$packaging/io.github.huou07.or_app.metainfo.xml"
desktop-file-validate "$packaging/io.github.huou07.or_app.desktop"

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
src="$work/src"
mkdir -p "$src/bundle"
cp -a "$bundle/." "$src/bundle/"
rm -f "$src/bundle/ffmpeg-link-probe"
install -m 755 "$packaging/launcher.sh" "$src/launcher.sh"
install -m 644 "$packaging/io.github.huou07.or_app.desktop" "$src/io.github.huou07.or_app.desktop"
install -m 644 "$packaging/io.github.huou07.or_app.metainfo.xml" "$src/io.github.huou07.or_app.metainfo.xml"
install -m 644 "$packaging/icon.png" "$src/icon.png"

# The manifest is staged beside its source tree so flatpak-builder never walks
# the repository (target/, build/ and dist/ are not Flatpak inputs).
sed 's/^app-id:.*/app-id: '"$app_id"'/' "$packaging/io.github.huou07.or_app.yml" \
  | sed 's|path: \.|path: src|' > "$work/manifest.yml"

flatpak-builder --user --force-clean \
  --repo="$work/repo" --state-dir="$work/state" "$work/build" "$work/manifest.yml"

mkdir -p "$(dirname -- "$output")"
flatpak build-bundle "$work/repo" "$output" "$app_id" \
  --runtime-repo=https://flathub.org/repo/flathub.flatpakrepo
flatpak build-import-bundle --help >/dev/null 2>&1 || true

echo "built $output"
flatpak info --show-metadata "$output" 2>/dev/null | head -20 || true
sha256sum "$output"
