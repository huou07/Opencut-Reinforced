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
user_data_dir=""

cleanup() {
  if [[ -n "$user_data_dir" ]]; then
    rm -rf -- "$user_data_dir"
  fi
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
sentinel="$user_data_dir/project-data.txt"
printf 'user data stays outside the application package\n' > "$sentinel"
apt_get apt-get install --yes --no-install-recommends "$deb"
test "$(dpkg-query -W -f='${Status}' "$package_name")" = "install ok installed"
test -x "$app"
test -s "$desktop"
test -s "$copyright_file"
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

apt_get apt-get remove --yes "$package_name"
test ! -e "$app"
test ! -e "$desktop"
test ! -e "$copyright_file"
test -s "$sentinel"
echo "Debian package install, launcher, runtime start, uninstall, and user-data preservation passed."
