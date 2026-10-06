#!/usr/bin/env bash
# Starts a fresh Android emulator and waits for a responsive guest.
# Usage: OR_AVD_NAME=... start-android-emulator.sh [wipe-data]
#
# Each Android acceptance case gets its own emulator. One long-lived guest that
# has already driven two cases wedges under SwiftShader: qemu stays at ~187% CPU
# and 3.6 GiB RSS while the guest stops answering adb, and the case then fails as
# "device offline" with no product cause. A fresh guest per case removes that
# accumulated state. It is not a retry: the case still runs exactly once and still
# fails if the product is wrong.
set -euo pipefail
avd_name="${OR_AVD_NAME:-or-api36-x86_64}"
android_device_id="${OR_ANDROID_DEVICE_ID:-emulator-5554}"
export ANDROID_AVD_HOME="$HOME/.android/avd"
mkdir -p "$ANDROID_AVD_HOME"
if [[ ! -f "$ANDROID_AVD_HOME/$avd_name.ini" ]]; then
  printf 'no\n' | avdmanager create avd --force --name "$avd_name" \
    --package 'system-images;android-36;google_apis;x86_64'
fi
test -f "$ANDROID_AVD_HOME/$avd_name.ini"
avd_list="$(avdmanager list avd -c)"
if ! grep -Fxq "$avd_name" <<< "$avd_list"; then
  echo "Expected Android AVD '$avd_name' was not created; available AVDs:" >&2
  printf '%s\n' "$avd_list" >&2
  exit 1
fi
test -e /dev/kvm
sudo chown "$USER" /dev/kvm
test -r /dev/kvm
test -w /dev/kvm
: > "$RUNNER_TEMP/android-emulator.log"

wipe_args=()
if [[ "${1:-false}" == wipe-data ]]; then wipe_args=(-wipe-data); fi
"$ANDROID_HOME/emulator/emulator" -avd "$avd_name" "${wipe_args[@]}" \
  -no-window -no-audio -no-boot-anim -no-snapshot \
  -gpu swiftshader_indirect >> "$RUNNER_TEMP/android-emulator.log" 2>&1 &
emulator_pid=$!
printf '%s\n' "$emulator_pid" > "$RUNNER_TEMP/android-emulator.pid"

if ! timeout 120s adb wait-for-device; then
  echo 'Android emulator did not appear within 120 seconds; emulator log follows:' >&2
  cat "$RUNNER_TEMP/android-emulator.log" >&2 || true
  exit 1
fi
booted=0
for _ in $(seq 1 120); do
  if [[ "$(adb shell getprop sys.boot_completed 2>/dev/null | tr -d '\r')" == 1 ]]; then
    booted=1
    break
  fi
  sleep 5
done
if [[ "$booted" != 1 ]]; then
  cat "$RUNNER_TEMP/android-emulator.log" >&2 || true
  exit 1
fi
test "$(adb shell getprop ro.build.version.sdk | tr -d '\r')" = 36
test "$(adb shell getprop ro.product.cpu.abi | tr -d '\r')" = x86_64
adb shell settings put global window_animation_scale 0
adb shell settings put global transition_animation_scale 0
adb shell settings put global animator_duration_scale 0
echo "Android emulator $android_device_id is responsive (pid $emulator_pid)."