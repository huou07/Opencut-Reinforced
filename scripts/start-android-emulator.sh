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
# The emulator's serial is its console port, so a caller that reserves
# `emulator-5580` must make the emulator listen on 5580. Without this the guest
# boots on the default 5554 while the caller waits for 5580: measured
# 2026-10-18, the local lane's emulator step timed out at exactly its 120 s
# `wait-for-device` bound, the guest was left running, and the reserved serial
# was never occupied. Deriving the port here keeps the reservation meaningful and
# keeps another project's guest on another port untouched.
emulator_port=""
if [[ "$android_device_id" =~ ^emulator-([0-9]{4,5})$ ]]; then
  emulator_port="${BASH_REMATCH[1]}"
elif [[ "$android_device_id" != emulator-* ]]; then
  echo "OR_ANDROID_DEVICE_ID must be an emulator console serial (emulator-<port>); got '$android_device_id'" >&2
  exit 2
fi
android_home="${ANDROID_HOME:-$HOME/Android/Sdk}"
emulator_bin="${OR_ANDROID_EMULATOR_BIN:-$android_home/emulator/emulator}"
kvm_device="${OR_ANDROID_KVM_DEVICE:-/dev/kvm}"
guest_ram_mb="${OR_ANDROID_GUEST_RAM_MB:-6144}"
guest_cores="${OR_ANDROID_GUEST_CORES:-4}"
# The guest's GPU mode is part of the evidence: a SwiftShader guest is
# software-path evidence and can never become a device performance claim.
guest_gpu="${OR_ANDROID_GPU_MODE:-swiftshader_indirect}"
# Honour an AVD home the caller already chose (the lane exports one); the
# documented default is the interactive user's.
export ANDROID_AVD_HOME="${ANDROID_AVD_HOME:-$HOME/.android/avd}"
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
test -e "$kvm_device"
if [[ ! -r "$kvm_device" || ! -w "$kvm_device" ]]; then
  # Only ask for the privileged repair when the device really is unusable; an
  # unconditional chown is a host write this lane does not need.
  sudo chown "$USER" "$kvm_device"
fi
test -r "$kvm_device"
test -w "$kvm_device"
if [[ -n "$emulator_port" ]] && command -v ss >/dev/null 2>&1 \
  && ss -ltn "sport = :$emulator_port" 2>/dev/null | grep -q LISTEN; then
  echo "the emulator console port $emulator_port (serial $android_device_id) is already in use; another guest or program holds it" >&2
  exit 3
fi
: > "$RUNNER_TEMP/android-emulator.log"

wipe_args=()
if [[ "${1:-false}" == wipe-data ]]; then wipe_args=(-wipe-data); fi
port_args=()
if [[ -n "$emulator_port" ]]; then port_args=(-port "$emulator_port"); fi
"$emulator_bin" -avd "$avd_name" "${wipe_args[@]}" ${port_args[@]+"${port_args[@]}"} \
  -no-window -no-audio -no-boot-anim -no-snapshot \
  -gpu "$guest_gpu" >> "$RUNNER_TEMP/android-emulator.log" 2>&1 &
emulator_pid=$!
printf '%s\n' "$emulator_pid" > "$RUNNER_TEMP/android-emulator.pid"

if ! timeout 120s adb -s "$android_device_id" wait-for-device; then
  echo "Android emulator did not appear on the reserved serial $android_device_id within 120 seconds (port ${emulator_port:-default}); emulator log follows:" >&2
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
echo "Android emulator $android_device_id is responsive (pid $emulator_pid, console port ${emulator_port:-default}, gpu $guest_gpu)."
