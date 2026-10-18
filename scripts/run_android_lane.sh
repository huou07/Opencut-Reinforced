#!/usr/bin/env bash
# One honest local Android emulator developer lane for OR.
#
#   bash scripts/run_android_lane.sh [--steps apk,emulator,preview,saf] \
#        [--report PATH] [--keep-guest]
#
# Steps, in order:
#   apk        stage the FFmpeg link and build every case APK
#              (scripts/build_android_case_apks.sh)
#   emulator   boot a dedicated AVD with KVM (scripts/start-android-emulator.sh)
#   preview    drive the bridge-diagnostics and preview-surface journeys, after
#              the FFmpeg guest link probe when one is staged
#   saf        reboot a wiped guest and drive the DocumentsUI SAF journey
# The guest is always stopped afterwards unless --keep-guest is given.
#
# Each Android case has its own Dart entrypoint, so each needs its own APK: the
# lane drives prebuilt per-case binaries exactly as the hosted `android` job
# does, so Gradle and the Kotlin daemons never compete with the software
# renderer. The APK step also creates the `android/.ffmpeg-install` link the
# bridge build hook requires; without it `flutter build apk` fails before it
# starts, which is why the lane previously could not run even with a staged
# prefix.
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
DEFAULT_REPORT="$LANE_DIR/android.json"
DEFAULT_STEPS="apk,emulator,preview,saf"

MIN_MEM_GIB="${OR_ANDROID_LANE_MIN_MEM_GIB:-7}"
AVD_NAME="${OR_AVD_NAME:-an3-api36}"
DEVICE_ID="${OR_ANDROID_DEVICE_ID:-emulator-5580}"
ANDROID_HOME_DIR="${ANDROID_HOME:-$HOME/Android/Sdk}"
AVD_HOME_DIR="${ANDROID_AVD_HOME:-$HOME/.android/avd}"
# 6144 MB is `scripts/start-android-emulator.sh`'s own measured default. The
# lane used to force 2048 MB, which is below the documented 2560 MB image
# default that the emulator script records as too small, so the guest was
# under-sized before it ever booted.
GUEST_RAM_MB="${OR_ANDROID_GUEST_RAM_MB:-6144}"
MEMINFO="${OR_LANE_MEMINFO:-/proc/meminfo}"
# The pinned Android FFmpeg staging prefix is deliberately required, not
# defaulted: a lane that guessed it would fail confusingly instead of naming the
# missing input. `scripts/stage_android_ffmpeg.sh` now produces it locally.
FFMPEG_PREFIX="${OR_ANDROID_FFMPEG_PREFIX:-}"
# Optional: the guest FFmpeg link probe. When it is absent the lane records that
# the probe did not run instead of implying it passed.
FFMPEG_PROBE="${OR_ANDROID_FFMPEG_PROBE:-}"
# Test seams: the lane drives the repository's own scripts, and the hermetic test
# points these at stubs instead of a real device.
START_SCRIPT="${OR_ANDROID_START_SCRIPT:-$ROOT/scripts/start-android-emulator.sh}"
PREVIEW_SCRIPT="${OR_ANDROID_PREVIEW_SCRIPT:-$ROOT/scripts/run-android-preview-check.sh}"
STOP_SCRIPT="${OR_ANDROID_STOP_SCRIPT:-$ROOT/scripts/stop-android-emulator.sh}"
CASE_APK_SCRIPT="${OR_ANDROID_CASE_APK_SCRIPT:-$ROOT/scripts/build_android_case_apks.sh}"

usage() {
  cat <<'USAGE'
usage: bash scripts/run_android_lane.sh [--steps apk,emulator,preview,saf]
                                        [--report PATH] [--keep-guest]

  --steps      run only the named steps, in this order
  --report     write the machine-readable report somewhere else; the logs, the
               step ledger and the case APKs follow it
  --keep-guest leave the emulator running (for debugging; you must stop it)

Inputs: OR_ANDROID_FFMPEG_PREFIX must name the staged x86_64 Android FFmpeg
install prefix (it must contain lib/pkgconfig) when the apk step is selected;
`bash scripts/stage_android_ffmpeg.sh` produces one and prints the export lines.
OR_ANDROID_FFMPEG_PROBE is optional and names the guest link probe that the same
script builds; without it the lane records that the probe did not run.

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
    apk|emulator|preview|saf) ;;
    *) printf 'run_android_lane: unknown step: %s\n' "$step" >&2; usage >&2; exit 2 ;;
  esac
  if [[ "$seen" == *",$step,"* ]]; then
    printf 'run_android_lane: step named twice: %s\n' "$step" >&2
    exit 2
  fi
  seen="$seen$step,"
done

# Every artifact follows the report path. A hermetic test that redirects
# --report to a temporary directory must not write logs or APKs into the real
# build directory, and a real run must keep them beside its own report.
report_dir="$(dirname -- "$report")"
mkdir -p "$report_dir"
LOG_DIR="$report_dir/android-logs"
CASE_APK_DIR="$report_dir/android-case-apks"
STEPS_FILE="$report_dir/.android-steps.tsv"
WARNINGS_FILE="$report_dir/.android-warnings.txt"
mkdir -p "$LOG_DIR" "$CASE_APK_DIR"
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
for script in "$START_SCRIPT" "$PREVIEW_SCRIPT" "$STOP_SCRIPT" "$CASE_APK_SCRIPT"; do
  [[ -f "$script" ]] || failures+=("missing lane script: $script")
done
if command -v adb >/dev/null 2>&1 && adb devices 2>/dev/null | grep -q "^${DEVICE_ID}[[:space:]]"; then
  failures+=("the dedicated serial $DEVICE_ID is already in use")
fi
if step_selected apk; then
  if [[ -z "$FFMPEG_PREFIX" ]]; then
    failures+=("OR_ANDROID_FFMPEG_PREFIX is unset; the staged x86_64 Android FFmpeg prefix is required to build the APK (run scripts/stage_android_ffmpeg.sh and export what it prints)")
  elif [[ ! -d "$FFMPEG_PREFIX/lib/pkgconfig" ]]; then
    failures+=("OR_ANDROID_FFMPEG_PREFIX=$FFMPEG_PREFIX has no lib/pkgconfig")
  fi
  # Gradle creates a project cache inside every included build, and the Flutter
  # Gradle plugin is one (`includeBuild("$flutterSdkPath/packages/flutter_tools/
  # gradle")` in apps/or_app/android/settings.gradle.kts). When that directory is
  # not writable, Gradle 9.3.1 dies with the misleading "The settings are not yet
  # available for build." Measured on this host on 2026-10-18: the same directory
  # copied to a writable location builds, the shared one does not. Name it here
  # instead of letting the APK step fail with an error that reads like a
  # repository defect.
  flutter_bin="$(command -v flutter 2>/dev/null || true)"
  if [[ -n "$flutter_bin" ]]; then
    flutter_sdk="$(cd -- "$(dirname -- "$(readlink -f "$flutter_bin")")/.." && pwd)"
    flutter_gradle_plugin="$flutter_sdk/packages/flutter_tools/gradle"
    if [[ ! -d "$flutter_gradle_plugin" ]]; then
      failures+=("the Flutter SDK at $flutter_sdk has no packages/flutter_tools/gradle, so apps/or_app/android/settings.gradle.kts cannot include the Flutter Gradle plugin")
    elif [[ ! -w "$flutter_gradle_plugin" ]]; then
      failures+=("the Flutter SDK's Gradle plugin directory $flutter_gradle_plugin is not writable by $(id -un); Gradle must create a project cache there and otherwise fails with 'The settings are not yet available for build.' Grant write access to it (for example setfacl -R -m u:$(id -un):rwX on that directory) before running the apk step")
    fi
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
  RUNNER_TEMP="$report_dir" OR_ANDROID_DEVICE_ID="$DEVICE_ID" \
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
  export RUNNER_TEMP="$report_dir"
  export GITHUB_WORKSPACE="$ROOT"
  export ANDROID_HOME="$ANDROID_HOME_DIR"
  export ANDROID_AVD_HOME="$AVD_HOME_DIR"
  export OR_AVD_NAME="$AVD_NAME"
  export OR_ANDROID_DEVICE_ID="$DEVICE_ID"
  export OR_ANDROID_GUEST_RAM_MB="$GUEST_RAM_MB"
  export OR_ANDROID_ACCEPTANCE_OUTPUT="$report_dir/android-acceptance"
  export OR_ANDROID_RECOVERY_ACCEPTANCE_OUTPUT="$report_dir/android-acceptance-recovery"
  export PKG_CONFIG_ALLOW_CROSS=1
  if [[ -n "$FFMPEG_PREFIX" ]]; then
    export PKG_CONFIG_PATH="$FFMPEG_PREFIX/lib/pkgconfig"
  fi
  if [[ -z "$FFMPEG_PROBE" ]]; then
    warn "OR_ANDROID_FFMPEG_PROBE is unset, so the guest FFmpeg link probe did not run; scripts/stage_android_ffmpeg.sh builds one"
  elif [[ ! -x "$FFMPEG_PROBE" ]]; then
    warn "OR_ANDROID_FFMPEG_PROBE=$FFMPEG_PROBE is not executable, so the guest probe did not run"
  fi

  for step in "${step_list[@]}"; do
    case "$step" in
      apk)
        run_step apk "bash '$CASE_APK_SCRIPT' '$CASE_APK_DIR'" 3600 || result="FAIL"
        ;;
      emulator)
        run_step emulator "bash '$START_SCRIPT'" 900 || result="FAIL"
        if [[ "$(awk -F'\t' '$1=="emulator" && $3=="PASS"{print "yes"}' "$STEPS_FILE")" == "yes" ]]; then
          guest_started=1
        fi
        ;;
      preview)
        # One step, several bounded drives: the diagnostics case asserts the CLI
        # snapshot, and the surface case reads its fixture from the app's private
        # files directory, so it must be staged after the app is installed.
        preview_command="set -e; "
        if [[ -x "$FFMPEG_PROBE" ]]; then
          preview_command+="bash '$ROOT/scripts/run_android_ffmpeg_probe.sh' '$FFMPEG_PROBE' '$CASE_APK_DIR/diagnostics.apk' '$DEVICE_ID'; "
        fi
        preview_command+="bash '$PREVIEW_SCRIPT' diagnostics integration_test/native_bridge_diagnostics_test.dart test_driver/integration_test.dart '$CASE_APK_DIR/diagnostics.apk' --keep-app-running; "
        preview_command+="adb -s '$DEVICE_ID' push '$ROOT/crates/or_media/tests/fixtures/tiny.mkv' /data/local/tmp/or-android-preview-tiny.mkv; "
        preview_command+="adb -s '$DEVICE_ID' shell run-as io.github.huou07.or_app mkdir -p files; "
        preview_command+="adb -s '$DEVICE_ID' shell run-as io.github.huou07.or_app cp /data/local/tmp/or-android-preview-tiny.mkv files/or-android-preview-tiny.mkv; "
        preview_command+="bash '$PREVIEW_SCRIPT' surface integration_test/android_preview_surface_test.dart test_driver/integration_test.dart '$CASE_APK_DIR/surface.apk'"
        run_step preview "$preview_command" 2700 || result="FAIL"
        ;;
      saf)
        # The SAF journey is the real product proof, so it gets its own wiped
        # guest rather than inheriting one that has already driven other cases.
        saf_command="set -e; "
        saf_command+="bash '$STOP_SCRIPT' >'$LOG_DIR/saf-stop.log' 2>&1 || true; "
        saf_command+="bash '$START_SCRIPT' wipe-data >'$LOG_DIR/saf-start.log' 2>&1; "
        saf_command+="adb -s '$DEVICE_ID' install -r '$CASE_APK_DIR/saf_fixture.apk' >'$LOG_DIR/saf-fixture-install.log' 2>&1; "
        saf_command+="mkdir -p '$OR_ANDROID_ACCEPTANCE_OUTPUT' '$OR_ANDROID_RECOVERY_ACCEPTANCE_OUTPUT'; "
        saf_command+="bash '$PREVIEW_SCRIPT' saf integration_test/android_saf_preview_test.dart test_driver/android_saf_preview.dart '$CASE_APK_DIR/saf.apk' '$CASE_APK_DIR/saf-recovery.apk'"
        run_step saf "$saf_command" 2700 || result="FAIL"
        # The saf step boots its own guest; the trap must still stop it.
        guest_started=1
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
  "$(( finished_ms - started_ms ))" "$head_sha" "$dirty_at_start" "$result" "$AVD_NAME" "$DEVICE_ID" \
  "$CASE_APK_DIR" "$FFMPEG_PROBE" "$GUEST_RAM_MB" <<'PY'
import hashlib, json, os, platform, shutil, sys
from pathlib import Path

(steps_file, warnings_file, report, started_at, finished_at, elapsed_ms, head,
 dirty, result, avd, device, case_apk_dir, probe, guest_ram_mb) = sys.argv[1:15]

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

# The exact binaries this run drove, so the result is bound to an artifact and
# not only to a source SHA.
artifacts = []
for apk in sorted(Path(case_apk_dir).glob("*.apk")):
    artifacts.append({
        "path": str(apk),
        "size_bytes": apk.stat().st_size,
        "sha256": hashlib.sha256(apk.read_bytes()).hexdigest(),
    })

payload = {
    "lane": "android",
    "evidence_class": "local-developer",
    "acceptance": False,
    "source": head,
    "dirty_at_start": dirty == "1",
    "guest": {"avd": avd, "serial": device, "configured_ram_mb": int(guest_ram_mb)},
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
    "not_run": ([] if probe and os.access(probe, os.X_OK) else [
        "FFmpeg guest link probe: OR_ANDROID_FFMPEG_PROBE was not supplied, so the run does not establish that the packaged FFmpeg libraries load on the guest",
    ]),
    "artifacts": artifacts,
    "warnings": [line for line in Path(warnings_file).read_text(encoding="utf-8").splitlines() if line],
    "limits": [
        "Local developer evidence on this named host only; the guest is a SwiftShader x86_64 emulator, not a physical device.",
        "Hosted platform-verification remains the authority for Android acceptance; a local guest result never becomes ACCEPTED.",
        "The case APKs are debug builds signed with the standard debug key; they are not release artifacts.",
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
