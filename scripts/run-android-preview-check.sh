#!/usr/bin/env bash
# A single drive lifecycle with full guest/host observations. No restarts/retries.
set -euo pipefail
case_name="$1"
target="$2"
driver="$3"
apk="$4"
recovery_apk=
if [[ "$case_name" == saf ]]; then
  recovery_apk="${5:-}"
  shift 5
else
  shift 4
fi
android_device_id=emulator-5554
output_directory="${OR_ANDROID_ACCEPTANCE_OUTPUT:-$RUNNER_TEMP/android-$case_name}"
mkdir -p "$output_directory"
log="$output_directory/android-driver-$case_name.log"
guest_log="$output_directory/android-guest-$case_name.log"
health_log="$output_directory/android-health-$case_name.log"
picker_status_file="$output_directory/android-picker-$case_name.status"
lifecycle_status_file="$output_directory/android-lifecycle-$case_name.status"
: > "$picker_status_file"
: > "$guest_log"
adb -s "$android_device_id" logcat -c
adb -s "$android_device_id" logcat -b all -v threadtime > "$guest_log" 2>&1 &
guest_pid=$!
# Collection tolerates failed samples, which are preserved. sed consumes all
# ps output, so pipefail/SIGPIPE cannot silently kill this sampler. The adb
# probe runs once per 15s: two round trips every 5s were killing adb's own
# transport with `timeout` while the software-rendered guest was still busy.
(
  set +e
  while true; do
    date -u '+%Y-%m-%dT%H:%M:%SZ'
    ps -p "$(cat "$RUNNER_TEMP/android-emulator.pid")" -o pid,stat,%cpu,rss,args
    ps -eo pid,%cpu,rss,args --sort=-rss | sed -n '1,11p'
    cat /proc/meminfo
    cat /proc/pressure/memory 2>/dev/null || true
    timeout 15s adb -s "$android_device_id" get-state
    for _ in 1 2; do sleep 5; timeout 5s adb -s "$android_device_id" get-state; done
  done
) > "$health_log" 2>&1 &
health_pid=$!
picker_pid=
lifecycle_pid=
cleanup() {
  kill "$guest_pid" "$health_pid" ${picker_pid:+"$picker_pid"} ${lifecycle_pid:+"$lifecycle_pid"} 2>/dev/null || true
  wait "$guest_pid" "$health_pid" ${picker_pid:+"$picker_pid"} ${lifecycle_pid:+"$lifecycle_pid"} 2>/dev/null || true
}
stop_process_tree() {
  local process_id="$1" child
  for child in $(pgrep -P "$process_id" 2>/dev/null || true); do
    stop_process_tree "$child"
  done
  kill "$process_id" 2>/dev/null || true
}
trap cleanup EXIT
if [[ "$case_name" == saf ]]; then
  rm -f "$picker_status_file"
  echo 'Starting Android SAF DocumentsUI selector.'
  (
    if python3 "$GITHUB_WORKSPACE/scripts/select_android_saf_document.py" \
      --device "$android_device_id" --guest-log "$guest_log" \
      --output "$OR_ANDROID_ACCEPTANCE_OUTPUT" \
      --flow "${OR_ANDROID_SAF_FLOW:-open}" \
      > "$OR_ANDROID_ACCEPTANCE_OUTPUT/documents-ui-selector.log" 2>&1; then
      picker_status=0
    else
      picker_status=$?
    fi
    printf '%s\n' "$picker_status" > "$picker_status_file"
  ) &
  picker_pid=$!
fi
cd "$GITHUB_WORKSPACE/apps/or_app"
status=0
# `adb get-state` only proves the adb server can see the emulator. The Flutter
# device discovery also runs `adb shell` against the guest, and a guest still
# busy from the previous case did not answer it: the drive failed with "No
# supported devices found" while adb had worked 500ms earlier.
#
# Answering at all is not enough. A freshly booted guest answers `adb shell`
# immediately while it is still starved, and the drive then dies inside
# `waitForServiceExtension` before the isolate can serve the VM service. So the
# guest must answer three consecutive probes *quickly*: a settled guest answers
# in well under a second, a busy one takes seconds. This is a precondition, not a
# retry: the case still runs once and still fails if the guest never settles.
ready=0
probes=0
for _ in $(seq 1 36); do
  probes=$((probes + 1))
  probe_start=$SECONDS
  if [[ "$(timeout 15s adb -s "$android_device_id" shell echo or-ready 2>/dev/null | tr -d '\r')" == or-ready ]] &&
    ((SECONDS - probe_start <= 3)); then
    ready=$((ready + 1))
    ((ready == 3)) && break
  else
    ready=0
  fi
  sleep 5
done
echo "Android $case_name guest readiness: ready=$ready probes=$probes"
# A previous case can take the emulator down. Check before launching so the run
# reports a lost emulator instead of a confusing "no supported devices" driver
# error, and so the cause is classified. The case still fails: a lost emulator
# means this acceptance did not run.
if [[ "$ready" != 3 ]]; then
  {
    date -u
    echo "BLOCKED: emulator $android_device_id guest did not answer three consecutive probes before case $case_name."
    echo 'adb devices:'
    adb devices -l || true
  } > "$RUNNER_TEMP/android-state-$case_name.txt" 2>&1
  cat "$RUNNER_TEMP/android-state-$case_name.txt" >&2
  python3 "$GITHUB_WORKSPACE/scripts/classify_android_disconnect.py" \
    --state "$RUNNER_TEMP/android-state-$case_name.txt" --health "$health_log" \
    --case "$case_name" --output "$RUNNER_TEMP/android-disconnect-$case_name.json" || true
  exit 1
fi
# The APK is built before the emulator starts. Driving a prebuilt binary keeps
# Gradle and the Kotlin daemons out of the software-rendered emulator phase,
# where ~4 GiB of build JVMs was competing for the runner's vCPUs.
echo "Starting Android Flutter driver for $case_name."
(
  flutter drive --driver="$driver" --target="$target" -d "$android_device_id" --no-dds \
    --use-application-binary="$apk" "$@" 2>&1 | tee "$log"
) &
drive_pid=$!
if [[ -n "$picker_pid" ]]; then
  while [[ ! -s "$picker_status_file" ]] &&
    kill -0 "$drive_pid" 2>/dev/null && kill -0 "$picker_pid" 2>/dev/null; do
    sleep 1
  done
  if [[ ! -s "$picker_status_file" ]] && ! kill -0 "$picker_pid" 2>/dev/null; then
    echo "Android DocumentsUI selector exited without a status." >&2
    cat "$OR_ANDROID_ACCEPTANCE_OUTPUT/documents-ui-selector.log" >&2
    stop_process_tree "$drive_pid"
    status=1
  fi
  if [[ -s "$picker_status_file" ]]; then
    read -r picker_status < "$picker_status_file"
    if [[ "$picker_status" != 0 ]]; then
      cat "$OR_ANDROID_ACCEPTANCE_OUTPUT/documents-ui-selector.log" >&2
      stop_process_tree "$drive_pid"
      status=1
    else
      echo 'Android SAF DocumentsUI selector completed.'
    fi
  fi
fi
if [[ "$case_name" == saf && "$status" == 0 ]]; then
  : > "$lifecycle_status_file"
  (
    # tee flushes each Flutter driver line. The redirected adb logcat capture is
    # buffered, so its copy of the native fixture marker can arrive too late.
    while ! grep -Fq 'ANDROID_SAF_BACKGROUND_CONTROL_COMPLETE' "$log"; do
      sleep 0.25
    done
    # Let Android complete the move-to-background transition before simulating
    # the launcher action. Starting an activity from OR's background process is
    # blocked on API 36; the host shell launches through Android's real launcher.
    sleep 1.5
    if timeout 30s adb -s "$android_device_id" shell monkey -p io.github.huou07.or_app 1; then
      printf '0\n' > "$lifecycle_status_file"
    else
      printf '%s\n' "$?" > "$lifecycle_status_file"
    fi
  ) > "$output_directory/android-lifecycle-$case_name.log" 2>&1 &
  lifecycle_pid=$!
fi
wait "$drive_pid" || status=$?
echo "Android Flutter driver for $case_name exited with status $status."
if [[ -n "$picker_pid" ]]; then
  if [[ "$status" != 0 ]]; then
    kill "$picker_pid" 2>/dev/null || true
    wait "$picker_pid" 2>/dev/null || true
  else
    wait "$picker_pid" || { cat "$RUNNER_TEMP/android-documents-ui.log" >&2; status=1; }
  fi
  picker_pid=
fi
if [[ -n "$lifecycle_pid" ]]; then
  if [[ "$status" == 0 ]]; then
    wait "$lifecycle_pid" || status=1
    if [[ ! -s "$lifecycle_status_file" ]] ||
      [[ "$(cat "$lifecycle_status_file")" != 0 ]]; then
      cat "$output_directory/android-lifecycle-$case_name.log" >&2
      status=1
    fi
  else
    kill "$lifecycle_pid" 2>/dev/null || true
    wait "$lifecycle_pid" 2>/dev/null || true
  fi
  lifecycle_pid=
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

if [[ "$case_name" == saf && "$status" == 0 ]]; then
  if [[ -z "$recovery_apk" || ! -s "$recovery_apk" ]]; then
    echo 'The Android SAF process recovery APK is missing.' >&2
    exit 1
  fi
  app_id=io.github.huou07.or_app
  recovery_output="${OR_ANDROID_RECOVERY_ACCEPTANCE_OUTPUT:-$OR_ANDROID_ACCEPTANCE_OUTPUT-recovery}"
  mkdir -p "$recovery_output"

  # The first journey has saved a real recovery sidecar. Capture a live process,
  # force-stop it through Android, prove it disappeared, then relaunch the same
  # installed app without clearing its private files or SAF grants.
  adb -s "$android_device_id" shell monkey -p "$app_id" 1 >/dev/null
  old_pid=
  for _ in $(seq 1 60); do
    old_pid="$(adb -s "$android_device_id" shell pidof "$app_id" 2>/dev/null | tr -d '\r' | awk '{print $1}')"
    [[ -n "$old_pid" ]] && break
    sleep 1
  done
  if [[ -z "$old_pid" ]]; then
    echo 'The prepared project process did not start before the force-stop.' >&2
    exit 1
  fi
  adb -s "$android_device_id" shell am force-stop "$app_id"
  stopped=0
  for _ in $(seq 1 30); do
    if [[ -z "$(adb -s "$android_device_id" shell pidof "$app_id" 2>/dev/null | tr -d '\r')" ]]; then
      stopped=1
      break
    fi
    sleep 1
  done
  if [[ "$stopped" != 1 ]]; then
    echo 'Android did not terminate the app process after force-stop.' >&2
    exit 1
  fi
  adb -s "$android_device_id" shell monkey -p "$app_id" 1 >/dev/null
  new_pid=
  for _ in $(seq 1 60); do
    new_pid="$(adb -s "$android_device_id" shell pidof "$app_id" 2>/dev/null | tr -d '\r' | awk '{print $1}')"
    [[ -n "$new_pid" ]] && break
    sleep 1
  done
  if [[ -z "$new_pid" || "$new_pid" == "$old_pid" ]]; then
    echo "The app did not restart as a new process (old=$old_pid new=$new_pid)." >&2
    exit 1
  fi
  python3 - "$OR_ANDROID_ACCEPTANCE_OUTPUT/process-relaunch.json" "$old_pid" "$new_pid" <<'PY'
import json
import sys
from pathlib import Path

Path(sys.argv[1]).write_text(json.dumps({
    "package": "io.github.huou07.or_app",
    "oldPid": int(sys.argv[2]),
    "emptyAfterForceStop": True,
    "newPid": int(sys.argv[3]),
    "result": "PASS",
}, indent=2) + "\n")
PY

  # Reopen the same persisted SAF document through DocumentsUI. This second
  # journey must apply the on-disk checkpoint and preview from recovered state.
  kill "$guest_pid" 2>/dev/null || true
  wait "$guest_pid" 2>/dev/null || true
  adb -s "$android_device_id" logcat -c
  guest_log="$recovery_output/android-guest-saf-recovery.log"
  adb -s "$android_device_id" logcat -b all -v threadtime > "$guest_log" 2>&1 &
  guest_pid=$!
  OR_ANDROID_ACCEPTANCE_OUTPUT="$recovery_output"
  export OR_ANDROID_ACCEPTANCE_OUTPUT
  picker_status_file="$recovery_output/android-picker-saf-recovery.status"
  : > "$picker_status_file"
  echo "Android app process restarted ($old_pid -> $new_pid); starting recovery DocumentsUI selector."
  (
    if python3 "$GITHUB_WORKSPACE/scripts/select_android_saf_document.py" \
      --device "$android_device_id" --guest-log "$guest_log" \
      --output "$OR_ANDROID_ACCEPTANCE_OUTPUT" --flow open \
      > "$recovery_output/documents-ui-recovery.log" 2>&1; then
      picker_status=0
    else
      picker_status=$?
    fi
    printf '%s\n' "$picker_status" > "$picker_status_file"
  ) &
  picker_pid=$!
  log="$recovery_output/android-driver-saf-recovery.log"
  echo 'Starting Android Flutter recovery driver.'
  flutter drive \
    --driver=test_driver/android_saf_recovery.dart \
    --target=integration_test/android_saf_recovery_test.dart \
    -d "$android_device_id" --no-dds \
    --use-application-binary="$recovery_apk" 2>&1 | tee "$log" &
  drive_pid=$!
  recovery_status=0
  while [[ ! -s "$picker_status_file" ]] &&
    kill -0 "$drive_pid" 2>/dev/null && kill -0 "$picker_pid" 2>/dev/null; do
    sleep 1
  done
  if [[ -s "$picker_status_file" ]]; then
    read -r picker_status < "$picker_status_file"
    if [[ "$picker_status" != 0 ]]; then
      cat "$recovery_output/documents-ui-recovery.log" >&2
      stop_process_tree "$drive_pid"
      recovery_status=1
    fi
  elif ! kill -0 "$picker_pid" 2>/dev/null; then
    echo 'Android recovery DocumentsUI selector exited without a status.' >&2
    cat "$recovery_output/documents-ui-recovery.log" >&2
    stop_process_tree "$drive_pid"
    recovery_status=1
  fi
  wait "$drive_pid" || recovery_status=$?
  if [[ "$recovery_status" != 0 ]]; then
    kill "$picker_pid" 2>/dev/null || true
    wait "$picker_pid" 2>/dev/null || true
    exit "$recovery_status"
  fi
  wait "$picker_pid" || {
    cat "$recovery_output/documents-ui-recovery.log" >&2
    exit 1
  }
fi
exit "$status"
