#!/usr/bin/env bash
set -euo pipefail

if [[ "$#" -ne 3 ]]; then
  echo "usage: package_linux_deb.sh <linux-bundle> <output.deb> <40-character-source-sha>" >&2
  exit 2
fi

bundle="$1"
output="$2"
source_sha="$3"
package_name="opencut-reinforced"
desktop_entry="io.github.huou07.or_app.desktop"
script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd -- "$script_dir/.." && pwd)"

[[ "$source_sha" =~ ^[0-9a-f]{40}$ ]] || { echo "source SHA must be 40 lowercase hexadecimal characters" >&2; exit 2; }
test -x "$bundle/or_app"
test -x "$bundle/ffmpeg"
test -x "$bundle/ffprobe"
test -s "$bundle/licenses/FFmpeg/COPYING.LGPLv2.1"
test -s "$bundle/licenses/FFmpeg/FFmpeg-NOTICE.txt"
test -s "$bundle/licenses/Opus/COPYING"
test -s "$bundle/licenses/libvpx/LICENSE"
test -s "$bundle/licenses/libvpx/PATENTS"
command -v dpkg-deb >/dev/null
command -v dpkg-shlibdeps >/dev/null
command -v readelf >/dev/null
command -v desktop-file-validate >/dev/null
desktop-file-validate "$repo_root/apps/or_app/linux/packaging/$desktop_entry"

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
stage="$work/debian/$package_name"
runtime="$stage/opt/opencut-reinforced"
mkdir -p "$stage/DEBIAN" "$stage/usr/share/applications" "$stage/usr/share/doc/$package_name" "$runtime"
cp -a "$bundle/." "$runtime/"
rm -f "$runtime/ffmpeg-link-probe"
install -m 644 "$repo_root/apps/or_app/linux/packaging/$desktop_entry" "$stage/usr/share/applications/$desktop_entry"
install -m 644 "$repo_root/LICENSE" "$stage/usr/share/doc/$package_name/copyright"

mkdir -p "$work/debian"
cat > "$work/debian/control" <<'CONTROL'
Source: opencut-reinforced
Section: video
Priority: optional
Maintainer: Opencut Reinforced contributors <huou07@users.noreply.github.com>
Standards-Version: 4.6.2

Package: opencut-reinforced
Architecture: amd64
Depends: ${shlibs:Depends}
Description: Open-source cross-platform video editor
 Developer Preview build tested on Debian 13 and Ubuntu 24.04 amd64.
CONTROL

package_executables=()
while IFS= read -r -d '' binary; do
  if readelf -h "$binary" >/dev/null 2>&1; then
    relative_binary="$(realpath --relative-to="$work" "$binary")"
    package_executables+=("-e$relative_binary")
  fi
done < <(find "$runtime" -type f -print0)
test "${#package_executables[@]}" -gt 0
dependency_output="$(cd "$work" && dpkg-shlibdeps -O --ignore-missing-info -S"debian/$package_name" -l"$runtime/lib" "${package_executables[@]}")"
dependencies="$(sed -n 's/^shlibs:Depends=//p' <<< "$dependency_output")"
test -n "$dependencies"

run_number="${GITHUB_RUN_NUMBER:-0}"
run_attempt="${GITHUB_RUN_ATTEMPT:-1}"
[[ "$run_number" =~ ^[0-9]+$ && "$run_attempt" =~ ^[0-9]+$ ]] || { echo "GitHub run number and attempt must be integers" >&2; exit 2; }
version="0.0.0~preview.$run_number.$run_attempt+$(cut -c1-12 <<< "$source_sha")"
{
  printf 'Package: %s\n' "$package_name"
  printf 'Version: %s\n' "$version"
  printf 'Section: video\n'
  printf 'Priority: optional\n'
  printf 'Architecture: amd64\n'
  printf 'Maintainer: Opencut Reinforced contributors <huou07@users.noreply.github.com>\n'
  printf 'Homepage: https://github.com/huou07/Opencut-Reinforced\n'
  printf 'Depends: %s\n' "$dependencies"
  printf 'Description: Open-source cross-platform video editor\n'
  printf ' Developer Preview build tested on Debian 13 and Ubuntu 24.04 amd64.\n'
} > "$stage/DEBIAN/control"

mkdir -p "$(dirname -- "$output")"
dpkg-deb --build --root-owner-group --compression=xz --compression-level=6 "$stage" "$output"
dpkg-deb --info "$output"
package_entries="$(dpkg-deb --contents "$output")"
for entry in "usr/share/applications/$desktop_entry" "usr/share/doc/$package_name/copyright" "opt/opencut-reinforced/or_app"; do
  grep -F "./$entry" <<< "$package_entries"
done
for notice in licenses/FFmpeg/COPYING.LGPLv2.1 licenses/FFmpeg/FFmpeg-NOTICE.txt licenses/Opus/COPYING licenses/libvpx/LICENSE licenses/libvpx/PATENTS; do
  grep -F "/opt/opencut-reinforced/$notice" <<< "$package_entries"
done
