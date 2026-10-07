#!/usr/bin/env bash
# Starts a fresh Android emulator and waits for a responsive guest.
# Usage: OR_AVD_NAME=... start-android-emulator.sh [wipe-data]
#
# Each Android acceptance case gets its own emulator, and each guest gets enough
# memory and cores to run the product's own bounded budgets under SwiftShader.
#
# Measured reasons, not guesses:
#  - One long-lived guest that has already driven other cases stops answering adb
#    while qemu still holds ~187-217% CPU and 3.4-3.6 GiB RSS, with ~12 GiB
#    `MemAvailable` on the host and no OR fatal signal, ANR, or tombstone.
#  - The `avdmanager` default for this image is 2560 MB and the default core
#    count. The app then reports "Skipped 193 frames" on its first frame and the
#    drive dies inside `waitForServiceExtension` before the isolate can serve the
#    VM service. The host had ~12 GiB free the whole time, so this is a guest
#    sizing limit rather than host exhaustion.
#
# A bigger guest and a fresh guest per case are preconditions, not retries: the
# case still runs exactly once and still fails if the product is wrong.
set -euo pipefail
avd_name="${OR_AVD_NAME:-or-api36-x86_64}"
android_device_id="${OR_ANDROID_DEVICE_ID:-emulator-5554}"
guest_ram_mb="${OR_ANDROID_GUEST_RAM_MB:-6144}"
guest_cores="${OR_ANDROID_GUEST_CORES:-4}"
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
# Pin the guest size instead of inheriting the image default.
avd_config="$ANDROID_AVD_HOME/${avd_name}.avd/config.ini"
touch "$avd_config"
set_avd_value() {
  local key="$1" value="$2"
  if grep -qE "^${key}=" "$avd_config"; then
    sed -i '' -E "s/^${key}=.*/${key}=${value}/" "$avd_config" 2>/dev/null ||
      sed -i -E "s/^${key}=.*/${key}=${value}/" "$avd_config"
  else
    printf '%s=%s\n' "$key" "$value" >> "$avd_config"
  fi
}
set_avd_value hw.ramSize "$guest_ram_mb"
set_avd_value hw.cpu.ncore "$guest_cores"
grep -Fxq "hw.ramSize=$guest_ram_mb" "$avd_config"
grep -Fxq "hw.cpu.ncore=$guest_cores" "$avd_config"
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

if ! timeout 120s adb -s "$android_device_id" wait-for-device; then
  echo 'Android emulator did not appear within 120 seconds; emulator log follows:' >&2
  cat "$RUNNER_TEMP/android-emulator.log" >&2 || true
  exit 1
fi
booted=0
for _ in $(seq 1 120); do
  if [[ "$(adb -s "$android_device_id" shell getprop sys.boot_completed 2>/dev/null | tr -d '\r')" == 1 ]]; then
    booted=1
    break
  fi
  sleep 5
done
if [[ "$booted" != 1 ]]; then
  cat "$RUNNER_TEMP/android-emulator.log" >&2 || true
  exit 1
fi
test "$(adb -s "$android_device_id" shell getprop ro.build.version.sdk | tr -d '\r')" = 36
test "$(adb -s "$android_device_id" shell getprop ro.product.cpu.abi | tr -d '\r')" = x86_64
adb -s "$android_device_id" shell settings put global window_animation_scale 0
adb -s "$android_device_id" shell settings put global transition_animation_scale 0
adb -s "$android_device_id" shell settings put global animator_duration_scale 0
test "$(adb -s "$android_device_id" get-state)" = device
if ! kill -0 "$emulator_pid" 2>/dev/null; then
  echo "Android emulator process $emulator_pid exited during startup; emulator log follows:" >&2
  cat "$RUNNER_TEMP/android-emulator.log" >&2 || true
  exit 1
fi
echo "Android emulator $android_device_id is responsive (pid $emulator_pid)."
