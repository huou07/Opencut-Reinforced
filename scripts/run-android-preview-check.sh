#!/usr/bin/env bash
# A single drive lifecycle with full guest/host observations. No restarts/retries.
set -euo pipefail
case_name="$1"
target="$2"
driver="$3"
shift 3
android_device_id=emulator-5554
log="$RUNNER_TEMP/android-driver-$case_name.log"
guest_log="$RUNNER_TEMP/android-guest-$case_name.log"
health_log="$RUNNER_TEMP/android-health-$case_name.log"
: > "$guest_log"
adb -s "$android_device_id" logcat -c
adb -s "$android_device_id" logcat -b all -v threadtime > "$guest_log" 2>&1 &
guest_pid=$!
# Collection tolerates failed samples, which are preserved. sed consumes all
# ps output, so pipefail/SIGPIPE cannot silently kill this sampler.
(
  set +e
  while true; do
    date -u '+%Y-%m-%dT%H:%M:%SZ'
    ps -p "$(cat "$RUNNER_TEMP/android-emulator.pid")" -o pid,stat,%cpu,rss,args
    ps -eo pid,%cpu,rss,args --sort=-rss | sed -n '1,11p'
    cat /proc/meminfo
    cat /proc/pressure/memory 2>/dev/null || true
    timeout 5s adb -s "$android_device_id" get-state
    timeout 5s adb -s "$android_device_id" shell cat /proc/loadavg
    sleep 5
  done
) > "$health_log" 2>&1 &
health_pid=$!
picker_pid=
cleanup() {
  kill "$guest_pid" "$health_pid" ${picker_pid:+"$picker_pid"} 2>/dev/null || true
  wait "$guest_pid" "$health_pid" ${picker_pid:+"$picker_pid"} 2>/dev/null || true
}
trap cleanup EXIT
if [[ "$case_name" == saf ]]; then
  python3 "$GITHUB_WORKSPACE/scripts/select_android_saf_document.py" \
    --device "$android_device_id" --guest-log "$guest_log" \
    --output "$OR_ANDROID_ACCEPTANCE_OUTPUT" > "$RUNNER_TEMP/android-documents-ui.log" 2>&1 &
  picker_pid=$!
fi
cd "$GITHUB_WORKSPACE/apps/or_app"
status=0
# A previous case can take the emulator down. Check before launching so the run
# reports a lost emulator instead of a confusing "no supported devices" driver
# error, and so the cause is classified. The case still fails: a lost emulator
# means this acceptance did not run.
if [[ "$(adb -s "$android_device_id" get-state 2>/dev/null || true)" != device ]]; then
  {
    date -u
    echo "BLOCKED: emulator $android_device_id was not available before case $case_name."
    echo 'adb devices:'
    adb devices -l || true
  } > "$RUNNER_TEMP/android-state-$case_name.txt" 2>&1
  cat "$RUNNER_TEMP/android-state-$case_name.txt" >&2
  python3 "$GITHUB_WORKSPACE/scripts/classify_android_disconnect.py" \
    --state "$RUNNER_TEMP/android-state-$case_name.txt" --health "$health_log" \
    --case "$case_name" --output "$RUNNER_TEMP/android-disconnect-$case_name.json" || true
  exit 1
fi
flutter drive --driver="$driver" --target="$target" -d "$android_device_id" --no-dds "$@" \
  2>&1 | tee "$log" || status=$?
if [[ -n "$picker_pid" ]]; then
  if [[ "$status" != 0 ]]; then
    kill "$picker_pid" 2>/dev/null || true
    wait "$picker_pid" 2>/dev/null || true
  else
    wait "$picker_pid" || { cat "$RUNNER_TEMP/android-documents-ui.log" >&2; status=1; }
  fi
  picker_pid=
fi
{
  date -u
  echo 'Post-drive ADB state:'
  adb devices -l || true
  echo 'Emulator host state:'
  ps -p "$(cat "$RUNNER_TEMP/android-emulator.pid")" -o pid,stat,%cpu,rss,args || true
  echo 'OR memory:'
  timeout 10s adb -s "$android_device_id" shell dumpsys meminfo io.github.huou07.or_app || true
  echo 'OR presentation:'
  timeout 10s adb -s "$android_device_id" shell dumpsys gfxinfo io.github.huou07.or_app || true
  echo 'Guest ANR/tombstone listings:'
  timeout 10s adb -s "$android_device_id" shell 'ls -l /data/anr /data/tombstones' || true
} > "$RUNNER_TEMP/android-state-$case_name.txt" 2>&1
python3 - "$guest_log" "$case_name" <<'PY'
from pathlib import Path
import sys
lines = Path(sys.argv[1]).read_text(errors='replace').splitlines()
print(f'Android {sys.argv[2]} startup/performance/crash signals:')
for line in lines:
    if any(signal in line for signal in ('Skipped ', 'VM service is listening', 'Fatal signal', 'FATAL EXCEPTION', 'ANR in ', 'ANDROID_PREVIEW_', 'ANDROID_SAF_ACCEPTANCE_')):
        print(line)
PY
# Name the disconnect cause instead of reporting a bare non-zero exit. This runs
# on every case so a failure is never ambiguous, and it never changes the status.
python3 "$GITHUB_WORKSPACE/scripts/classify_android_disconnect.py" \
  --driver "$log" --guest "$guest_log" \
  --state "$RUNNER_TEMP/android-state-$case_name.txt" --health "$health_log" \
  --case "$case_name" --output "$RUNNER_TEMP/android-disconnect-$case_name.json" \
  || true
exit "$status"
