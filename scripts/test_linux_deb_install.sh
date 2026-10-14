#!/usr/bin/env bash
set -euo pipefail

if [[ "$#" -ne 1 ]]; then
  echo "usage: test_linux_deb_install.sh <package.deb>" >&2
  exit 2
fi

deb="$(realpath -- "$1")"
package_name="opencut-reinforced"
app="/opt/opencut-reinforced/or_app"
desktop="/usr/share/applications/io.github.huou07.or_app.desktop"
copyright_file="/usr/share/doc/opencut-reinforced/copyright"
temp_root="${RUNNER_TEMP:-${TMPDIR:-/tmp}}"
launch_log="$(mktemp "$temp_root/or-deb-launch.XXXXXX")"
upgrade_dir="$(mktemp -d "$temp_root/or-deb-upgrade.XXXXXX")"
user_data_dir=""

cleanup() {
  if [[ -n "$user_data_dir" ]]; then
    rm -rf -- "$user_data_dir"
  fi
  rm -rf -- "$upgrade_dir"
  rm -f -- "$launch_log"
}
trap cleanup EXIT

apt_get() {
  if [[ "$EUID" -eq 0 ]]; then
    "$@"
  else
    sudo "$@"
  fi
}

test -s "$deb"
mkdir -p "$HOME/Documents"
user_data_dir="$(mktemp -d "$HOME/Documents/or-deb-package-check.XXXXXX")"
sentinel="$user_data_dir/user-data.txt"
printf 'user data survives package upgrades and removal\n' > "$sentinel"

# Build a lower-version copy of the same package payload so apt performs a real
# package upgrade. This proves dpkg replaces an installed version while files a
# user saved beside the app remain intact.
old_root="$upgrade_dir/old-package"
old_deb="$upgrade_dir/opencut-reinforced-previous.deb"
mkdir -p "$old_root"
dpkg-deb --extract "$deb" "$old_root"
dpkg-deb --control "$deb" "$old_root/DEBIAN"
old_version="0.0.0~preview.0.1+upgrade-test"
current_version="$(dpkg-deb -f "$deb" Version)"
if dpkg --compare-versions "$current_version" le "$old_version"; then
  echo "package version $current_version must be newer than upgrade fixture $old_version" >&2
  exit 1
fi
sed -i "s/^Version:.*/Version: $old_version/" "$old_root/DEBIAN/control"
dpkg-deb --build --root-owner-group "$old_root" "$old_deb"

apt_get apt-get install --yes --no-install-recommends "$old_deb"
test "$(dpkg-query -W -f='${Status}' "$package_name")" = "install ok installed"
test "$(dpkg-query -W -f='${Version}' "$package_name")" = "$old_version"
test -x "$app"
test -s "$desktop"
test -s "$copyright_file"
grep -Fxq "Exec=$app" "$desktop"
dpkg -L "$package_name" | grep -Fx "$desktop"
dpkg -L "$package_name" | grep -Fx "$app"
dpkg -L "$package_name" | grep -Fx "$copyright_file"

apt_get apt-get install --yes --no-install-recommends "$deb"
test "$(dpkg-query -W -f='${Status}' "$package_name")" = "install ok installed"
test "$(dpkg-query -W -f='${Version}' "$package_name")" = "$current_version"
test -x "$app"
test -s "$desktop"
test -s "$copyright_file"
test -s "$sentinel"
grep -Fxq "Exec=$app" "$desktop"
dpkg -L "$package_name" | grep -Fx "$desktop"
dpkg -L "$package_name" | grep -Fx "$app"
dpkg -L "$package_name" | grep -Fx "$copyright_file"

if ! command -v xvfb-run >/dev/null || ! command -v xauth >/dev/null; then
  apt_get apt-get update
  apt_get apt-get install --yes --no-install-recommends xvfb xauth
fi

set +e
xvfb-run -a timeout --signal=TERM 20s "$app" > "$launch_log" 2>&1
launch_status=$?
set -e
if [[ "$launch_status" -ne 124 ]]; then
  cat "$launch_log" >&2
  echo "installed editor exited before the launch smoke-test timeout (status $launch_status)" >&2
  exit 1
fi

apt_get apt-get purge --yes "$package_name"
test ! -e "$app"
test ! -e "$desktop"
test ! -e "$copyright_file"
test "$(dpkg-query -W -f='${db:Status-Status}' "$package_name")" = "not-installed"
test -s "$sentinel"
echo "Debian package install, upgrade, launcher, runtime start, purge, and project-data preservation passed."
