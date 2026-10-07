#!/usr/bin/env bash
# Behavioral test for the Android guest readiness gate in
# scripts/run-android-preview-check.sh.
#
# The gate must reject a guest that answers `adb shell` only after several
# seconds. Answering at all is not enough: on a freshly booted emulator the
# drive died inside `waitForServiceExtension` before the isolate could serve the
# VM service, because the gate accepted a guest that was still starved.
#
# Usage: bash scripts/test_android_guest_readiness_gate.sh
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
GATE="$ROOT/scripts/run-android-preview-check.sh"
fails=0

if ! grep -q 'SECONDS - probe_start <= 3' "$GATE"; then
  echo "FAIL the gate does not require a fast guest answer"
  fails=$((fails + 1))
else
  echo "PASS the gate requires a fast guest answer"
fi

work="$(mktemp -d "${TMPDIR:-/tmp}/or-gate.XXXXXX")"
trap 'rm -rf "$work"' EXIT

# The gate body, with adb/timeout replaced by a controllable stub and sleep
# collapsed so the test does not take minutes.
sed -n '/^ready=0$/,/^echo "Android .* guest readiness/p' "$GATE" |
  sed -e 's|timeout 15s adb -s "$android_device_id" shell echo or-ready|stub|' \
    -e 's|^  sleep 5$|  sleep 0|' > "$work/gate.sh"

cat > "$work/stub" <<'STUB'
#!/usr/bin/env bash
count_file="${GATE_TEST_COUNT:?}"
count=$(cat "$count_file" 2>/dev/null || echo 0)
count=$((count + 1))
echo "$count" > "$count_file"
# The first N probes answer correctly but only after the guest has settled.
if [ "$count" -le "${GATE_TEST_SLOW_PROBES:-0}" ]; then sleep 4; fi
echo or-ready
STUB
chmod +x "$work/stub"

run_gate() {
  PATH="$work:$PATH" GATE_TEST_COUNT="$work/count" bash "$work/gate.sh" |
    sed -n 's/.*ready=\([0-9]*\) probes=\([0-9]*\).*/\1 \2/p'
}

# A settled guest passes on the first probe.
export GATE_TEST_SLOW_PROBES=0
rm -f "$work/count"
out="$(run_gate)"
echo "settled guest: ready/probes = $out"
if [[ "$out" == "3 3" ]]; then
  echo "PASS a settled guest is accepted immediately"
else
  echo "FAIL a settled guest was not accepted immediately (got: $out)"
  fails=$((fails + 1))
fi

# A starved guest answers correctly but slowly, so it must not be accepted.
export GATE_TEST_SLOW_PROBES=3
rm -f "$work/count"
out="$(run_gate)"
echo "starved guest: ready/probes = $out"
if [[ "$out" == "3 "* && "$out" != "3 3" ]]; then
  echo "PASS a starved guest is re-probed instead of accepted"
else
  echo "FAIL a starved guest was accepted (got: $out)"
  fails=$((fails + 1))
fi

# The AVD must be sized explicitly. The image default is 2560 MB, and with it
# the app skipped 193 frames on its first frame while the drive lost the VM
# service; the host had ~12 GiB free, so this is a guest limit.
start_script="$ROOT/scripts/start-android-emulator.sh"
if grep -q 'set_avd_value hw.ramSize' "$start_script" &&
  grep -q 'set_avd_value hw.cpu.ncore' "$start_script" &&
  grep -q 'grep -Fxq "hw.ramSize=\$guest_ram_mb"' "$start_script"; then
  echo "PASS the guest is sized explicitly instead of inheriting the image default"
else
  echo "FAIL the guest does not pin its RAM and core count"
  fails=$((fails + 1))
fi

# Emulator startup probes must stay bound to the requested serial, and startup
# must not report success after its guest or process has disappeared.
if grep -Fq 'adb -s "$android_device_id" wait-for-device' "$start_script" &&
  grep -Fq 'adb -s "$android_device_id" shell getprop sys.boot_completed' "$start_script" &&
  grep -Fq 'adb -s "$android_device_id" get-state' "$start_script"; then
  echo "PASS emulator startup probes the requested serial"
else
  echo "FAIL emulator startup can accept a different or vanished guest"
  fails=$((fails + 1))
fi

# A new Android step can reuse emulator-5554 only after the prior QEMU process
# has drained. Give the fake emulator a delayed TERM handler to prove stop waits.
stop_script="$ROOT/scripts/stop-android-emulator.sh"
cat > "$work/fake-emulator.py" <<'PY'
import signal
import sys
import time

stopping = False
def stop(_signal, _frame):
    global stopping
    time.sleep(0.7)
    stopping = True

signal.signal(signal.SIGTERM, stop)
open(sys.argv[1], "w").close()
while not stopping:
    time.sleep(0.05)
PY
python3 "$work/fake-emulator.py" "$work/emulator-ready" &
fake_pid=$!
for _ in $(seq 1 50); do
  [[ -e "$work/emulator-ready" ]] && break
  sleep 0.02
done
if [[ ! -e "$work/emulator-ready" ]]; then
  echo "FAIL fake emulator did not start"
  fails=$((fails + 1))
else
  printf '%s\n' "$fake_pid" > "$work/android-emulator.pid"
  RUNNER_TEMP="$work" OR_ANDROID_EMULATOR_STOP_WAIT_SECONDS=3 \
    python3 - "$stop_script" "$fake_pid" <<'PY'
import os
import subprocess
import sys
import time

started = time.monotonic()
subprocess.run(["bash", sys.argv[1]], check=True, env=os.environ.copy())
elapsed = time.monotonic() - started
state = subprocess.run(
    ["ps", "-o", "stat=", "-p", sys.argv[2]],
    check=False, capture_output=True, text=True,
).stdout.strip()
if elapsed < 0.6 or (state and not state.startswith("Z")):
    raise SystemExit(f"stop returned before emulator exit (elapsed={elapsed:.2f}s state={state!r})")
print(f"PASS emulator stop waited for process exit ({elapsed:.2f}s)")
PY
  if [[ "$?" -ne 0 ]]; then
    echo "FAIL emulator stop returned before process exit"
    fails=$((fails + 1))
  fi
  wait "$fake_pid" 2>/dev/null || true
fi

if [[ "$fails" -eq 0 ]]; then echo "GATE_TESTS=PASS"; else echo "GATE_TESTS=FAIL ($fails)"; exit 1; fi
