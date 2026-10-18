#!/usr/bin/env bash
# One honest local Android emulator developer lane for OR.
#
#   bash scripts/run_android_lane.sh [--steps apk,emulator,preview] \
#        [--report PATH] [--keep-guest]
#
# Steps, in order:
#   apk        build the x86_64 debug APK the guest can install
#   emulator   boot a dedicated AVD with KVM (scripts/start-android-emulator.sh)
#   preview    drive the bridge-diagnostics, preview-surface and SAF journeys
#              (scripts/run-android-preview-check.sh)
# The guest is always stopped afterwards unless --keep-guest is given.
#
# It writes one machine-readable report (default
# apps/or_app/build/local-lanes/android.json, a gitignored path) with the same
# shape as the Linux lane report, and exits 0 only when every selected step
# passed on a clean tree. A host precondition that cannot be met is BLOCKED
# (exit 3), never a PASS: this lane produces developer evidence, and hosted
# `platform-verification` remains the authority for Android acceptance.
#
# Why the floor is 7 GiB and not the preflight's 6: the guest takes about
# 3.3 GiB and the Gradle/Kotlin JVMs about 4.4 GiB on this 15 GiB host, so a run
# that starts below the floor fails halfway and wastes the host. The lane
# measures MemAvailable before it boots anything and names the consumers it
# found. It never stops a java daemon it did not start: the Gradle daemon on
# this host belongs to a sibling project.
set -uo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT" || exit 1

LANE_DIR="apps/or_app/build/local-lanes"
LOG_DIR="$LANE_DIR/android-logs"
DEFAULT_REPORT="$LANE_DIR/android.json"
DEFAULT_STEPS="apk,emulator,preview"
STEPS_FILE="$LANE_DIR/.android-steps.tsv"
WARNINGS_FILE="$LANE_DIR/.android-warnings.txt"

MIN_MEM_GIB="${OR_ANDROID_LANE_MIN_MEM_GIB:-7}"
AVD_NAME="${OR_AVD_NAME:-an3-api36}"
DEVICE_ID="${OR_ANDROID_DEVICE_ID:-emulator-5580}"
ANDROID_HOME_DIR="${ANDROID_HOME:-$HOME/Android/Sdk}"
AVD_HOME_DIR="${ANDROID_AVD_HOME:-$HOME/.android/avd}"
GUEST_RAM_MB="${OR_ANDROID_GUEST_RAM_MB:-2048}"
MEMINFO="${OR_LANE_MEMINFO:-/proc/meminfo}"
# The pinned Android FFmpeg staging prefix is deliberately required, not
# defaulted: the repository does not record the local recipe yet, and a lane that
# guesses it would fail confusingly instead of naming the missing input.
FFMPEG_PREFIX="${OR_ANDROID_FFMPEG_PREFIX:-}"
# Test seams: the lane drives the repository's own scripts, and the hermetic test
# points these at stubs instead of a real device.
START_SCRIPT="${OR_ANDROID_START_SCRIPT:-$ROOT/scripts/start-android-emulator.sh}"
PREVIEW_SCRIPT="${OR_ANDROID_PREVIEW_SCRIPT:-$ROOT/scripts/run-android-preview-check.sh}"
STOP_SCRIPT="${OR_ANDROID_STOP_SCRIPT:-$ROOT/scripts/stop-android-emulator.sh}"

usage() {
  cat <<'USAGE'
usage: bash scripts/run_android_lane.sh [--steps apk,emulator,preview]
                                        [--report PATH] [--keep-guest]

  --steps      run only the named steps, in this order
  --report     write the machine-readable report somewhere else
  --keep-guest leave the emulator running (for debugging; you must stop it)

Inputs: OR_ANDROID_FFMPEG_PREFIX must name the staged x86_64 Android FFmpeg
install prefix (it must contain lib/pkgconfig) when the apk step is selected.
See docs/prompts/OR-BUILD.md phase 0 for the recipe source.

Exit codes: 0 every selected step passed on a clean tree; 1 a step failed;
2 the invocation is wrong; 3 a host precondition was not met; 4 a step could
not be measured, or the tree was dirty so no PASS is attributable.
USAGE
}

report="$DEFAULT_REPORT"
steps=""
keep_guest=0
while (( $# )); do
  case "$1" in
    --steps)
      (( $# >= 2 )) || { printf 'run_android_lane: --steps needs a value\n' >&2; exit 2; }
      steps="$2"; shift 2 ;;
    --report)
      (( $# >= 2 )) || { printf 'run_android_lane: --report needs a value\n' >&2; exit 2; }
      report="$2"; shift 2 ;;
    --keep-guest) keep_guest=1; shift ;;
    -h|--help) usage; exit 0 ;;
    *) printf 'run_android_lane: unknown argument: %s\n' "$1" >&2; usage >&2; exit 2 ;;
  esac
done

[[ -n "$steps" ]] || steps="$DEFAULT_STEPS"
seen=","
IFS=',' read -r -a step_list <<<"$steps"
for step in "${step_list[@]}"; do
  case "$step" in
    apk|emulator|preview) ;;
    *) printf 'run_android_lane: unknown step: %s\n' "$step" >&2; usage >&2; exit 2 ;;
  esac
  if [[ "$seen" == *",$step,"* ]]; then
    printf 'run_android_lane: step named twice: %s\n' "$step" >&2
    exit 2
  fi
  seen="$seen$step,"
done

mkdir -p "$LANE_DIR" "$LOG_DIR"
: > "$STEPS_FILE"
: > "$WARNINGS_FILE"

warn() { printf '%s\n' "$1" >> "$WARNINGS_FILE"; }

record() { # id exit classification seconds command log reason
  printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\n' "$1" "$2" "$3" "$4" "$5" "$6" "${7:-}" >> "$STEPS_FILE"
}

step_selected() {
  [[ ",$steps," == *",$1,"* ]]
}

head_sha="$(git rev-parse HEAD 2>/dev/null || printf 'unknown')"
dirty_at_start=0
if [[ -n "$(git status --porcelain 2>/dev/null)" ]]; then
  dirty_at_start=1
  warn "the working tree is not clean, so this run can only report DIRTY: $(git status --porcelain | tr '\n' ';')"
fi

started_ms="$(date +%s%3N)"
started_iso="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

# --- host preconditions ---------------------------------------------------
available_memory() {
  if [[ ! -f "$MEMINFO" ]]; then
    printf '0'
    return
  fi
  awk '/^MemAvailable:/{printf "%d", $2 * 1024}' "$MEMINFO"
}

memory_bytes="$(available_memory)"
floor_bytes=$(( MIN_MEM_GIB * 1024 * 1024 * 1024 ))
failures=()

if (( memory_bytes < floor_bytes )); then
  failures+=("MemAvailable is $(( memory_bytes / 1024 / 1024 )) MiB, below the ${MIN_MEM_GIB} GiB floor")
  while IFS= read -r daemon; do
    [[ -n "$daemon" ]] || continue
    failures+=("running java daemon: $(printf '%s' "$daemon" | cut -c1-140)")
  done < <(pgrep -af 'GradleDaemon|KotlinCompileDaemon' 2>/dev/null)
fi

for tool in adb flutter; do
  command -v "$tool" >/dev/null 2>&1 || failures+=("missing tool on PATH: $tool")
done
[[ -x "$ANDROID_HOME_DIR/emulator/emulator" ]] || failures+=("missing emulator binary: $ANDROID_HOME_DIR/emulator/emulator")
[[ -f "$AVD_HOME_DIR/$AVD_NAME.ini" ]] || failures+=("missing AVD '$AVD_NAME' under $AVD_HOME_DIR")
if [[ -r /dev/kvm && -w /dev/kvm ]]; then
  :
else
  failures+=("/dev/kvm is not readable and writable; the guest would run without acceleration")
fi
for script in "$START_SCRIPT" "$PREVIEW_SCRIPT" "$STOP_SCRIPT"; do
  [[ -f "$script" ]] || failures+=("missing lane script: $script")
done
if command -v adb >/dev/null 2>&1 && adb devices 2>/dev/null | grep -q "^${DEVICE_ID}[[:space:]]"; then
  failures+=("the dedicated serial $DEVICE_ID is already in use")
fi
if step_selected apk; then
  if [[ -z "$FFMPEG_PREFIX" ]]; then
    failures+=("OR_ANDROID_FFMPEG_PREFIX is unset; the pinned x86_64 Android FFmpeg prefix is required to build the APK (see docs/prompts/OR-BUILD.md phase 0)")
  elif [[ ! -d "$FFMPEG_PREFIX/lib/pkgconfig" ]]; then
    failures+=("OR_ANDROID_FFMPEG_PREFIX=$FFMPEG_PREFIX has no lib/pkgconfig")
  fi
fi

if (( ${#failures[@]} > 0 )); then
  reason="$(printf '%s; ' "${failures[@]}")"
  printf 'BLOCKED: %s\n' "$reason" >&2
  printf '%s\n' "$reason" > "$LOG_DIR/preflight.log"
  record "preflight" 3 BLOCKED 0 "host precondition probes" "$LOG_DIR/preflight.log" "$reason"
  result="BLOCKED"
else
  printf 'MemAvailable %s MiB >= %s MiB; adb, flutter, emulator, AVD %s and /dev/kvm present\n' \
    "$(( memory_bytes / 1024 / 1024 ))" "$(( floor_bytes / 1024 / 1024 ))" "$AVD_NAME" > "$LOG_DIR/preflight.log"
  record "preflight" 0 PASS 0 "host precondition probes" "$LOG_DIR/preflight.log" ""
  result="PASS"
fi

guest_started=0
stop_guest() {
  if (( guest_started == 0 )); then
    return 0
  fi
  if (( keep_guest == 1 )); then
    printf 'run_android_lane: guest left running on %s (--keep-guest)\n' "$DEVICE_ID" >&2
    return 0
  fi
  RUNNER_TEMP="$LANE_DIR" OR_ANDROID_DEVICE_ID="$DEVICE_ID" \
    bash "$STOP_SCRIPT" > "$LOG_DIR/stop.log" 2>&1 || warn "the stop step exited non-zero; the guest may still be running"
}
trap stop_guest EXIT

run_step() { # id command budget_seconds
  local id="$1" command="$2" budget="$3" log="$LOG_DIR/$1.log"
  local begin end code classification reason=""
  begin="$(date +%s%3N)"
  printf '+ %s\n' "$command" > "$log"
  timeout "$budget" bash -c "$command" >> "$log" 2>&1
  code=$?
  end="$(date +%s%3N)"
  if (( code == 0 )); then
    classification="PASS"
  elif (( code == 124 )); then
    classification="BLOCKED"; reason="the step exceeded its ${budget}s bound"
  elif (( code == 3 )); then
    classification="BLOCKED"; reason="the step reported a host precondition; see $log"
  else
    classification="FAIL"; reason="the step exited $code; see $log"
  fi
  record "$id" "$code" "$classification" \
    "$(awk -v a="$begin" -v b="$end" 'BEGIN{printf "%.2f", (b-a)/1000}')" "$command" "$log" "$reason"
  printf '  %-10s %-10s exit %s\n' "$id" "$classification" "$code"
  [[ "$classification" == "PASS" ]]
}

if [[ "$result" == "PASS" ]]; then
  export RUNNER_TEMP="$LANE_DIR"
  export GITHUB_WORKSPACE="$ROOT"
  export ANDROID_HOME="$ANDROID_HOME_DIR"
  export ANDROID_AVD_HOME="$AVD_HOME_DIR"
  export OR_AVD_NAME="$AVD_NAME"
  export OR_ANDROID_DEVICE_ID="$DEVICE_ID"
  export OR_ANDROID_GUEST_RAM_MB="$GUEST_RAM_MB"
  export OR_ANDROID_ACCEPTANCE_OUTPUT="$LANE_DIR/android-acceptance"
  export OR_ANDROID_RECOVERY_ACCEPTANCE_OUTPUT="$LANE_DIR/android-acceptance-recovery"
  export PKG_CONFIG_ALLOW_CROSS=1
  if [[ -n "$FFMPEG_PREFIX" ]]; then
    export PKG_CONFIG_PATH="$FFMPEG_PREFIX/lib/pkgconfig"
  fi

  for step in "${step_list[@]}"; do
    case "$step" in
      apk)
        run_step apk "cd '$ROOT/apps/or_app' && flutter build apk --debug --target-platform android-x64" 3600 || result="FAIL"
        ;;
      emulator)
        run_step emulator "bash '$START_SCRIPT'" 900 || result="FAIL"
        if [[ "$(awk -F'\t' '$1=="emulator" && $3=="PASS"{print "yes"}' "$STEPS_FILE")" == "yes" ]]; then
          guest_started=1
        fi
        ;;
      preview)
        run_step preview "bash '$PREVIEW_SCRIPT'" 1800 || result="FAIL"
        ;;
    esac
    [[ "$result" != "FAIL" ]] || break
  done

  if [[ "$result" == "PASS" ]] && awk -F'\t' '$3=="BLOCKED"{found=1} END{exit found?0:1}' "$STEPS_FILE"; then
    result="BLOCKED"
  fi
fi

if [[ "$dirty_at_start" == "1" && "$result" == "PASS" ]]; then
  result="DIRTY"
  warn "no PASS is attributable to a dirty tree"
fi

finished_ms="$(date +%s%3N)"
finished_iso="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
REPORT_TMP="$report.tmp"

python3 - "$STEPS_FILE" "$WARNINGS_FILE" "$REPORT_TMP" "$started_iso" "$finished_iso" \
  "$(( finished_ms - started_ms ))" "$head_sha" "$dirty_at_start" "$result" "$AVD_NAME" "$DEVICE_ID" <<'PY'
import json, os, platform, shutil, sys
from pathlib import Path

steps_file, warnings_file, report, started_at, finished_at, elapsed_ms, head, dirty, result, avd, device = sys.argv[1:13]

def memory_total():
    for line in Path("/proc/meminfo").read_text(encoding="utf-8").splitlines():
        if line.startswith("MemTotal:"):
            return int(line.split()[1]) * 1024
    return 0

steps = [{
    "id": r[0], "exit_code": int(r[1]), "classification": r[2],
    "elapsed_seconds": float(r[3]), "command": r[4], "log": r[5],
    "reason": r[6] or None,
} for r in (line.rstrip("\n").split("\t") for line in open(steps_file, encoding="utf-8") if line.strip())]

payload = {
    "lane": "android",
    "evidence_class": "local-developer",
    "acceptance": False,
    "source": head,
    "dirty_at_start": dirty == "1",
    "guest": {"avd": avd, "serial": device},
    "host": {
        "os": platform.system(), "arch": platform.machine(), "kernel": platform.release(),
        "memory_total_bytes": memory_total(),
        "free_disk_bytes": shutil.disk_usage(os.getcwd()).free,
    },
    "started_at": started_at,
    "finished_at": finished_at,
    "elapsed_seconds": round(int(elapsed_ms) / 1000.0, 2),
    "result": result,
    "steps": steps,
    "not_run": [],
    "artifacts": [],
    "warnings": [line for line in Path(warnings_file).read_text(encoding="utf-8").splitlines() if line],
    "limits": [
        "Local developer evidence on this named host only; the guest is a SwiftShader x86_64 emulator, not a physical device.",
        "Hosted platform-verification remains the authority for Android acceptance; a local guest result never becomes ACCEPTED.",
        "The APK is a debug build signed with the standard debug key; it is not a release artifact.",
        "Performance measured here describes the emulator and the software GPU path, never physical-device responsiveness.",
        "A guest that never settles inside the bounded probes is BLOCKED with the measured reason, never retried into a pass.",
    ],
    "report": str(Path(report).resolve()),
}
Path(report).write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
PY

mkdir -p "$(dirname -- "$report")"
mv "$REPORT_TMP" "$report"
rm -f "$STEPS_FILE" "$WARNINGS_FILE"

printf '\n== local android lane ==\n'
printf 'source        : %s\n' "$head_sha"
printf 'guest         : %s on %s\n' "$AVD_NAME" "$DEVICE_ID"
printf 'report        : %s\n' "$report"
python3 - "$report" <<'PY'
import json, sys
payload = json.loads(open(sys.argv[1], encoding="utf-8").read())
for step in payload["steps"]:
    print(f"  {step['id']:<10} {step['classification']:<10} {step['elapsed_seconds']:>8.2f}s  exit {step['exit_code']}")
print(f"RESULT: LOCAL_LANE={payload['result']}")
PY

case "$result" in
  PASS) exit 0 ;;
  FAIL) exit 1 ;;
  UNVERIFIED|DIRTY) exit 4 ;;
  *) exit 3 ;;
esac
