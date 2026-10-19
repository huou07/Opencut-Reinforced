#!/usr/bin/env bash
# One honest local Linux developer lane for OR.
#
#   bash scripts/run_local_lane.sh linux [--packaged] [--steps a,b] [--report PATH]
#
# Steps, in order:
#   hygiene    bash scripts/check-repo.sh
#   flutter    cd apps/or_app && flutter test --no-pub
#   preflight  python3 scripts/run_linux_ui_preflight.py     (bounded native journey)
#   bench      python3 scripts/bench_quality.py
# and, only with --packaged:
#   deb        build the Linux bundle, stage the pinned local media runtime,
#              package a .deb through scripts/package_linux_deb.sh
#   journey    extract that .deb into a disposable root and run
#              scripts/run_packaged_desktop_journey.py against the installed payload
#
# It writes one machine-readable report (default
# apps/or_app/build/local-lanes/linux.json, a gitignored path) and exits 0 only
# when every selected step passed on a clean tree. It stops at the first real
# FAIL but keeps going after a BLOCKED or UNVERIFIED step, because a host
# limitation does not invalidate the lanes that can still run. A step this host
# cannot run keeps the lane non-zero: a local lane never promotes itself to
# acceptance. Hosted `platform-verification` remains the authority for native,
# packaged and cross-platform claims, and --packaged says so in its own report.
#
# Why one entry point: the four steps above used to be four commands an agent had
# to remember, in an order, with predicates ("is FFmpeg 8 installed?") that were
# only written down in prose. A lane that cannot state what it ran, how long it
# took and what it could not establish is how a local pass gets mistaken for
# acceptance.
set -uo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT" || exit 1

LANE_DIR="$ROOT/apps/or_app/build/local-lanes"
PREFLIGHT_REPORT="apps/or_app/build/ui-preflight/run.json"
DEFAULT_REPORT="$LANE_DIR/linux.json"
DEFAULT_STEPS="hygiene,flutter,preflight,bench"
PACKAGED_STEPS="deb,journey"
BENCH_JSON="$LANE_DIR/bench.json"

# The pinned FFmpeg 8 prefix and codec prefix a local packaged build stages beside
# the bundle. Both are gitignored build state produced by the documented local
# recipe; nothing here downloads or builds codecs, and a missing prefix is a
# BLOCKED step naming the command that would produce it.
# Absolute, because the packaged steps are handed to the preflight wrapper, which
# runs them from apps/or_app: a path relative to the repository root resolves to the
# wrong place there.
FFMPEG_PREFIX="${OR_LANE_FFMPEG_PREFIX:-$ROOT/apps/or_app/build/local-deb/ffmpeg-install}"
CODEC_PREFIX="${OR_LANE_CODEC_PREFIX:-$ROOT/apps/or_app/build/local-deb/codecs-install}"
BUNDLE="${OR_LANE_BUNDLE:-$ROOT/apps/or_app/build/linux/x64/debug/bundle}"

usage() {
  cat <<'USAGE'
usage: bash scripts/run_local_lane.sh <linux|android> [--packaged] [--steps id,id] [--report PATH]

  linux        the Linux desktop lane (default steps below)
  android      the Android emulator lane; delegated to run_android_lane.sh
  --packaged   also build and package the local .deb and run the packaged journey;
               that is local developer evidence, never packaged acceptance
  --steps      run only the named steps, in this order:
               hygiene, flutter, preflight, bench, deb, journey
  --report     write the machine-readable report somewhere else

Exit codes: 0 every selected step passed on a clean tree; 1 a step failed; 2 the
invocation is wrong; 3 a step was BLOCKED by a host precondition; 4 a step could
not be measured (UNVERIFIED), or the tree was dirty so no PASS is attributable.
USAGE
}

all_args=("$@")
lane=""
steps=""
report="$DEFAULT_REPORT"
packaged=0

while (( $# )); do
  case "$1" in
    linux) lane="linux"; shift ;;
    android) lane="android"; shift ;;
    --packaged) packaged=1; shift ;;
    --steps)
      if (( $# < 2 )) || [[ -z "${2:-}" ]]; then
        printf 'run_local_lane: --steps needs a comma-separated list\n' >&2
        exit 2
      fi
      steps="$2"
      shift 2
      ;;
    --report)
      if (( $# < 2 )) || [[ -z "${2:-}" ]]; then
        printf 'run_local_lane: --report needs a path\n' >&2
        exit 2
      fi
      report="$2"
      shift 2
      ;;
    -h|--help) usage; exit 0 ;;
    *) printf 'run_local_lane: unknown argument: %s\n' "$1" >&2; usage >&2; exit 2 ;;
  esac
done

# The Android lane is a separate script: it boots a guest and has its own
# preconditions and report, and keeping it out of this file keeps the Linux lane
# readable. This stays the single entry point either way.
if [[ "$lane" == "android" ]]; then
  android_args=()
  for arg in "${all_args[@]}"; do
    [[ "$arg" == "android" ]] || android_args+=("$arg")
  done
  exec bash "$ROOT/scripts/run_android_lane.sh" ${android_args[@]+"${android_args[@]}"}
fi

if [[ "$lane" != "linux" ]]; then
  printf 'run_local_lane: name the lane to run: linux or android\n' >&2
  usage >&2
  exit 2
fi

if [[ -z "$steps" ]]; then
  steps="$DEFAULT_STEPS"
  if (( packaged )); then
    steps="$steps,$PACKAGED_STEPS"
  fi
fi

# Reject an unknown or repeated step before doing any work: a lane that silently
# ignores a typo in --steps reports a narrower run than the caller asked for.
seen=","
IFS=',' read -r -a step_list <<<"$steps"
for step in "${step_list[@]}"; do
  case "$step" in
    hygiene|flutter|preflight|bench|deb|journey) ;;
    *) printf 'run_local_lane: unknown step: %s\n' "$step" >&2; usage >&2; exit 2 ;;
  esac
  if [[ "$seen" == *",$step,"* ]]; then
    printf 'run_local_lane: step named twice: %s\n' "$step" >&2
    exit 2
  fi
  seen="$seen$step,"
done

step_budget() {
  case "$1" in
    hygiene) printf '900' ;;
    flutter) printf '900' ;;
    preflight) printf '1000' ;;
    bench) printf '900' ;;
    deb) printf '3600' ;;
    journey) printf '1200' ;;
    *) printf '900' ;;
  esac
}

step_command_text() {
  case "$1" in
    hygiene) printf 'bash scripts/check-repo.sh' ;;
    flutter) printf 'cd apps/or_app && flutter --no-version-check test --no-pub' ;;
    preflight) printf 'python3 scripts/run_linux_ui_preflight.py' ;;
    bench) printf 'python3 scripts/bench_quality.py --json' ;;
    deb) printf 'python3 scripts/run_linux_ui_preflight.py --exec ... -- bash -c packaged_build_deb' ;;
    journey) printf 'python3 scripts/run_linux_ui_preflight.py --exec ... -- bash -c packaged_run_journey' ;;
    *) printf '%s' "$1" ;;
  esac
}

now_ms() { date +%s%3N; }

JOURNAL="$(mktemp)"
ARTIFACTS="$(mktemp)"
WARNINGS="$(mktemp)"
REPORT_TMP="$(mktemp)"
trap 'rm -f "$JOURNAL" "$ARTIFACTS" "$WARNINGS" "$REPORT_TMP"' EXIT

# A warning is part of the run's record, not just console noise: "the bench step
# printed no machine-readable row" is exactly what a reader of the report needs
# to know before trusting the artifact list.
lane_warn() {
  printf 'run_local_lane: warning: %s\n' "$1" >&2
  printf '%s\n' "$1" >>"$WARNINGS"
}

sha256_of() {
  [[ -f "$1" ]] || return 1
  sha256sum "$1" | cut -d' ' -f1
}

# record_artifact <step> <path> [not-before-epoch]
#
# An artifact is a file this run produced, so two things disqualify one: being
# empty, and having existed before the step started. The bounded preflight had
# its run.json attributed to a BLOCKED step that never reached it, which is
# exactly the stale-file mistake this guards against.
record_artifact() {
  local step="$1" path="$2" not_before="${3:-0}"
  if [[ ! -s "$path" ]]; then
    lane_warn "$path is missing or empty, so no $step artifact was recorded"
    return 0
  fi
  if (( not_before > 0 )) && (( $(stat -c '%Y' "$path") < not_before )); then
    lane_warn "$path predates the $step step, so it was not recorded as this run's artifact"
    return 0
  fi
  local digest size
  digest="$(sha256_of "$path")" || return 0
  size="$(stat -c '%s' "$path")"
  printf '%s\t%s\t%s\t%s\n' "$step" "$path" "$digest" "$size" >>"$ARTIFACTS"
}

# The two host reasons `scripts/acceptance.sh` already classifies as a capability
# gap rather than a test failure. The classification is deliberately narrow: it
# only applies when no test progress line was printed at all, so a suite that
# starts and then fails is still a failure.
flutter_host_gap() {
  local log="$1"
  if grep -qE '^[0-9]{2}:[0-9]{2} \+[0-9]+' "$log"; then
    return 1
  fi
  grep -qE 'Building native assets failed|component download failed' "$log" && return 0
  grep -qE 'update_engine_version\.sh.*Permission denied' "$log" && return 0
  return 1
}

# How bad a classification is, so the lane's own result can be the worst step it
# saw without any step silently overwriting a worse one.
result_severity() {
  case "$1" in
    PASS) printf '0' ;;
    DIRTY) printf '1' ;;
    UNVERIFIED) printf '2' ;;
    BLOCKED) printf '3' ;;
    *) printf '4' ;;
  esac
}

# worse_result <current> <candidate>
worse_result() {
  if (( $(result_severity "$1") >= $(result_severity "$2") )); then
    printf '%s' "$1"
  else
    printf '%s' "$2"
  fi
}

# classify_step <id> <rc> <log>
classify_step() {
  local id="$1" rc="$2" log="$3"
  if (( rc == 0 )); then
    printf 'PASS'
    return
  fi
  case "$id:$rc" in
    # The bounded preflight and its --exec mode exit 3 for a host precondition
    # (dirty tree, RAM, disk, tool, lock). That is a blocked lane, not a failure.
    preflight:3|deb:3|journey:3) printf 'BLOCKED' ;;
    # bench_quality.py exits 3 when a row cannot be measured honestly here.
    bench:3) printf 'UNVERIFIED' ;;
    flutter:*)
      if flutter_host_gap "$log"; then
        printf 'UNVERIFIED'
      else
        printf 'FAIL'
      fi
      ;;
    *) printf 'FAIL' ;;
  esac
}

# record_step <id> <rc> <class> <ms> <command-text> <log> <reason>
record_step() {
  python3 - "$JOURNAL" "$@" <<'PY'
import json
import sys

journal, step_id, rc, classification, millis, command, log, reason = sys.argv[1:9]
with open(journal, "a", encoding="utf-8") as handle:
    handle.write(json.dumps({
        "id": step_id,
        "exit_code": int(rc),
        "classification": classification,
        "elapsed_seconds": round(int(millis) / 1000.0, 2),
        "command": command,
        "log": log,
        "reason": reason or None,
    }) + "\n")
PY
}

# run_step <id> <log> <display-command> <command...>
run_step() {
  local id="$1" log="$2" display="$3"
  shift 3
  local budget start end rc=0 classification reason=""
  budget="$(step_budget "$id")"
  mkdir -p "$(dirname "$log")"
  printf '\n== %s — %s ==\n' "$id" "$display"
  start="$(now_ms)"
  LAST_STEP_START_EPOCH="$(date +%s)"
  timeout --signal=TERM --kill-after=30 "${budget}s" "$@" >"$log" 2>&1 || rc=$?
  end="$(now_ms)"
  classification="$(classify_step "$id" "$rc" "$log")"
  case "$classification" in
    BLOCKED) reason="the step reported a host precondition; see $log" ;;
    UNVERIFIED)
      if [[ "$id" == "flutter" ]]; then
        reason="the Flutter native-assets hook or the shared SDK cache blocked the widget suite on this host and no test ran; see $log"
      else
        reason="the step could not be measured honestly on this host; see $log"
      fi
      ;;
    FAIL)
      if (( rc == 124 || rc == 137 )); then
        reason="the step exceeded its ${budget}s budget"
      else
        reason="the step exited ${rc}"
      fi
      ;;
  esac
  # Show the last lines so the console and the report agree about what happened.
  tail -n 15 "$log" || true
  record_step "$id" "$rc" "$classification" "$(( end - start ))" "$display" "$log" "$reason"
  printf 'RESULT: %s — %s (%ss)\n' "$classification" "$id" "$(( (end - start) / 1000 ))"
  LAST_CLASSIFICATION="$classification"
}

LAST_CLASSIFICATION=""
LAST_STEP_START_EPOCH=0

# --- packaged steps ---------------------------------------------------------
# Both run through `run_linux_ui_preflight.py --exec`, which prepares the same
# project-local caches and host-scoped toolchain manifest the bounded journey
# uses. They are exported so the helper can start them with `bash -c`; the lane
# passes its inputs through the environment.

reset_bridge_hook_cache() {
  # Flutter's native-assets hook caches the bridge and does not invalidate it when
  # the pinned FFmpeg changes, so a stale asset linked against the system FFmpeg 7
  # is reused. Removing the cache is the remedy AGENTS.md records.
  rm -rf "$OR_LANE_ROOT/apps/or_app/.dart_tool/hooks_runner/or_app_bridge" \
         "$OR_LANE_ROOT/apps/or_app/.dart_tool/hooks_runner/shared/or_app_bridge/build"
}

packaged_build_deb() {
  set -uo pipefail
  local root="$OR_LANE_ROOT" ff="$OR_LANE_FFMPEG_PREFIX"
  local bundle="$OR_LANE_BUNDLE" deb="$OR_LANE_DEB" copying="$OR_LANE_FFMPEG_COPYING"
  local notices="$OR_LANE_CODEC_NOTICES"
  # The repository root, because the staging and packaging scripts below are relative
  # to it. Flutter is the exception: it needs a project root, and the repository root
  # is not one, so those two commands run in the app directory. Doing it the other way
  # round — moving the whole step into the app — broke the script paths instead.
  cd "$root" || exit 1
  reset_bridge_hook_cache
  (cd apps/or_app && flutter --no-version-check pub get --offline --enforce-lockfile) || exit 1
  (cd apps/or_app && flutter --no-version-check build linux --debug --no-pub) || exit 1

  local bridge
  bridge="$(find "$bundle" -name libor_app_bridge.so -print -quit)"
  if [[ -z "$bridge" ]]; then
    echo "the Linux bundle has no Rust bridge under $bundle"; exit 1
  fi
  local linkage
  linkage="$(readelf -d "$bridge")"
  if [[ "$linkage" != *libavutil.so.60* ]]; then
    echo "the built bridge does not link the pinned FFmpeg 8 libavutil.so.60"; exit 1
  fi
  if [[ "$linkage" != *'$ORIGIN'* ]]; then
    echo "the built bridge has no \$ORIGIN RUNPATH"; exit 1
  fi

  # Stage the media runtime beside the bridge and the helper programs the same way
  # the hosted "Package FFmpeg beside the desktop probe" step does.
  local runtime pattern library helper encoders
  runtime="$(dirname "$bridge")"
  for pattern in 'libavcodec.so*' 'libavformat.so*' 'libavutil.so*' \
                 'libswresample.so*' 'libswscale.so*' 'libavfilter.so*'; do
    for library in "$ff"/lib/$pattern; do
      [[ -e "$library" ]] || continue
      cp -a "$library" "$runtime/"
    done
  done
  for library in "$runtime"/libav*.so* "$runtime"/libsw*.so*; do
    [[ -f "$library" && ! -L "$library" ]] || continue
    patchelf --set-rpath '$ORIGIN' "$library" || exit 1
  done
  for helper in ffmpeg ffprobe; do
    install -m 755 "$ff/bin/$helper" "$bundle/$helper" || exit 1
  done
  local helper_rpath
  helper_rpath="\$ORIGIN/$(basename "$runtime")"
  patchelf --set-rpath "$helper_rpath" "$bundle/ffmpeg" "$bundle/ffprobe" || exit 1

  # Prove the staged payload loads through its relative RUNPATH and still carries
  # the encoders the packaged export needs. A package that only looks right fails
  # much later, inside the journey, where the cause is harder to see.
  if ! env -u LD_LIBRARY_PATH "$bundle/ffprobe" -hide_banner -version >/dev/null; then
    echo "the staged ffprobe does not run with a relative RUNPATH"; exit 1
  fi
  encoders="$(env -u LD_LIBRARY_PATH "$bundle/ffmpeg" -hide_banner -encoders)"
  for helper in libvpx-vp9 libopus; do
    if [[ "$encoders" != *"$helper"* ]]; then
      echo "the staged ffmpeg has no $helper encoder"; exit 1
    fi
  done

  if ! bash scripts/stage_codec_licences.sh stage "$bundle" "$copying" "$notices"; then
    echo "staging the third-party licence tree failed"; exit 1
  fi
  bash scripts/stage_codec_licences.sh verify "$bundle" || exit 1

  mkdir -p "$(dirname "$deb")"
  bash scripts/package_linux_deb.sh "$bundle" "$deb" "$OR_LANE_HEAD" || exit 1
  echo "packaged: $deb"
}

packaged_run_journey() {
  set -uo pipefail
  local root="$OR_LANE_ROOT" deb="$OR_LANE_DEB" install_root="$OR_LANE_INSTALL_ROOT"
  local runtime="$OR_LANE_MEDIA_RUNTIME"
  # The repository root: the journey runner is a repository-relative script.
  cd "$root" || exit 1
  rm -rf "$install_root"
  mkdir -p "$install_root"
  dpkg-deb --extract "$deb" "$install_root" || exit 1

  local installed="$install_root/opt/opencut-reinforced" required
  for required in "$installed/or_app" "$installed/ffmpeg" "$installed/ffprobe" \
                  "$installed/lib/libavutil.so.60" \
                  "$install_root/usr/share/applications/io.github.huou07.or_app.desktop"; do
    if [[ ! -e "$required" ]]; then
      echo "the extracted package is missing $required"; exit 1
    fi
  done

  # The installed payload has to survive its own smoke window before the journey
  # is worth running, and the journey reads the helpers from the installed root.
  xvfb-run -a timeout --signal=TERM 20s "$installed/or_app" >"$OR_LANE_SMOKE_LOG" 2>&1
  local smoke=$?
  if (( smoke != 124 )); then
    echo "the installed editor exited before its 20s smoke window (status $smoke)"
    exit 1
  fi

  mkdir -p "$runtime"
  env -u LD_LIBRARY_PATH \
    OR_PACKAGED_HELPERS_DIRECTORY="$installed" \
    OR_PACKAGED_BRIDGE_DIRECTORY="$installed/lib" \
    OR_PACKAGED_MEDIA_RUNTIME_DIRECTORY="$runtime" \
    RUNNER_TEMP="$OR_LANE_RUNNER_TEMP" \
    python3 scripts/run_packaged_desktop_journey.py || exit 1
  echo "the packaged journey passed and left its report under $OR_LANE_RUNNER_TEMP"
}

export -f reset_bridge_hook_cache packaged_build_deb packaged_run_journey

# --- preconditions ----------------------------------------------------------
started_at="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
started_ms="$(now_ms)"
head="$(git rev-parse HEAD)"
dirty="$(git status --porcelain)"
dirty_at_start=0
if [[ -n "$dirty" ]]; then
  # A dirty tree cannot support a PASS: the report would not be attributable to
  # any commit. It is recorded rather than refused, for one reason: the
  # deliberately broken source check that proves this lane exits non-zero is
  # itself a dirty tree. The bounded journey keeps its own hard clean-tree
  # refusal, and a dirty run can therefore only ever report DIRTY, never PASS.
  dirty_at_start=1
  lane_warn "the working tree is not clean, so this run can only report DIRTY: $(printf '%s' "$dirty" | tr '\n' ';')"
fi

mkdir -p "$LANE_DIR" "$(dirname "$report")"

result="PASS"
for step in "${step_list[@]}"; do
  log="$LANE_DIR/linux-$step.log"
  case "$step" in
    hygiene)
      run_step hygiene "$log" "$(step_command_text hygiene)" bash scripts/check-repo.sh
      ;;
    flutter)
      run_step flutter "$log" "$(step_command_text flutter)" \
        bash -c 'cd apps/or_app && exec flutter --no-version-check test --no-pub'
      ;;
    preflight)
      run_step preflight "$log" "$(step_command_text preflight)" \
        python3 scripts/run_linux_ui_preflight.py
      record_artifact preflight "$PREFLIGHT_REPORT" "$LAST_STEP_START_EPOCH"
      ;;
    bench)
      # The budget deliberately never compiles: a benchmark must not surprise a
      # host with a codec or test build. Its four test-binary rows therefore need
      # prebuilt lanes, and a fresh checkout has none, so build them here once.
      # Without this every in-process row reported UNVERIFIED on a clean tree.
      bench_build_log="$LANE_DIR/linux-bench-build.log"
      : >"$bench_build_log"
      for bench_lane in \
        "-p or_media --test software_export" \
        "-p or_core --test command_latency" \
        "-p or_render --test render_frame_time" \
        "-p or_render --test queue_priority"; do
        # shellcheck disable=SC2086
        if ! cargo test $bench_lane --no-run --locked >>"$bench_build_log" 2>&1; then
          lane_warn "could not build the bench lane '$bench_lane'; its rows will report UNVERIFIED"
        fi
      done
      run_step bench "$log" "$(step_command_text bench)" \
        python3 scripts/bench_quality.py --json
      if grep -qF 'BENCH_JSON=' "$log"; then
        grep -F 'BENCH_JSON=' "$log" | tail -n1 | cut -d= -f2- >"$BENCH_JSON"
        record_artifact bench "$BENCH_JSON" "$LAST_STEP_START_EPOCH"
      else
        lane_warn "$(step_command_text bench) printed no BENCH_JSON line"
      fi
      ;;
    deb)
      missing=""
      [[ -x "$FFMPEG_PREFIX/bin/ffmpeg" ]] || missing="pinned FFmpeg 8 prefix: $FFMPEG_PREFIX"
      [[ -d "$CODEC_PREFIX/lib" ]] || missing="${missing:+$missing; }pinned codec prefix: $CODEC_PREFIX"
      copying="${OR_LANE_FFMPEG_COPYING:-$(dirname "$FFMPEG_PREFIX")/FFmpeg-COPYING.LGPLv2.1}"
      notices="${OR_LANE_CODEC_NOTICES:-$(dirname "$FFMPEG_PREFIX")/codec-notices}"
      [[ -s "$copying" ]] || missing="${missing:+$missing; }FFmpeg COPYING.LGPLv2.1: $copying"
      [[ -d "$notices" ]] || missing="${missing:+$missing; }codec notices: $notices"
      if [[ -n "$missing" ]]; then
        printf '\n== deb — %s ==\n' "$(step_command_text deb)"
        printf 'BLOCKED: the local packaged build needs pinned inputs this host does not have: %s\n' "$missing"
        printf 'settle with: bash scripts/build_desktop_export_codecs.sh linux <codecs-prefix> <work> and the pinned FFmpeg 8 configure in .github/workflows/platform-verification.yml\n'
        record_step deb 3 BLOCKED 0 "$(step_command_text deb)" "$log" \
          "missing pinned local inputs: $missing"
        # Same rule as every other step: a blocked step does not stop the lane,
        # so a caller that also asked for lanes the block does not affect still
        # gets their evidence. The shared post-step logic records the result.
        LAST_CLASSIFICATION="BLOCKED"
      else
        export OR_LANE_ROOT="$ROOT" OR_LANE_FFMPEG_PREFIX="$FFMPEG_PREFIX" \
          OR_LANE_CODEC_PREFIX="$CODEC_PREFIX" OR_LANE_BUNDLE="$BUNDLE" \
          OR_LANE_DEB="$LANE_DIR/opencut-reinforced-linux-x64-debug.deb" \
          OR_LANE_HEAD="$head" OR_LANE_FFMPEG_COPYING="$copying" OR_LANE_CODEC_NOTICES="$notices"
        run_step deb "$log" "$(step_command_text deb)" \
          python3 scripts/run_linux_ui_preflight.py --exec \
            --ffmpeg-prefix "$FFMPEG_PREFIX" --codec-prefix "$CODEC_PREFIX" \
            --local-home "$LANE_DIR/deb-home" -- bash -c packaged_build_deb
        record_artifact deb "$OR_LANE_DEB" "$LAST_STEP_START_EPOCH"
      fi
      ;;
    journey)
      if [[ ! -f "${OR_LANE_DEB:-}" ]]; then
        printf '\n== journey — %s ==\n' "$(step_command_text journey)"
        printf 'BLOCKED: no packaged .deb from this run to install; run the deb step first\n'
        record_step journey 3 BLOCKED 0 "$(step_command_text journey)" "$log" \
          "no packaged .deb was produced by this run"
        LAST_CLASSIFICATION="BLOCKED"
      else
        export OR_LANE_ROOT="$ROOT" OR_LANE_DEB \
          OR_LANE_INSTALL_ROOT="$LANE_DIR/installed-root" \
          OR_LANE_MEDIA_RUNTIME="$LANE_DIR/or-packaged-media-runtime" \
          OR_LANE_RUNNER_TEMP="$LANE_DIR/runner-temp" \
          OR_LANE_SMOKE_LOG="$LANE_DIR/packaged-smoke.log"
        run_step journey "$log" "$(step_command_text journey)" \
          python3 scripts/run_linux_ui_preflight.py --exec \
            --ffmpeg-prefix "$FFMPEG_PREFIX" --codec-prefix "$CODEC_PREFIX" \
            --local-home "$LANE_DIR/deb-home" -- bash -c packaged_run_journey
        record_artifact journey "$LANE_DIR/runner-temp/or-packaged-product-journey/existing-export.webm" "$LAST_STEP_START_EPOCH"
      fi
      ;;
  esac
  if [[ "$LAST_CLASSIFICATION" != "PASS" ]]; then
    result="$(worse_result "$result" "$LAST_CLASSIFICATION")"
    if [[ "$LAST_CLASSIFICATION" == "FAIL" ]]; then
      # A real failure invalidates everything after it, so stop there; the report
      # names the remaining steps as not run.
      break
    fi
    # A BLOCKED or UNVERIFIED step is a host limitation, not a product failure,
    # so the lanes that can still run do: on a host shared with another build,
    # stopping at the native step would hide the evidence that was available.
  fi
done

if (( dirty_at_start )) && [[ "$result" == "PASS" ]]; then
  # Every selected step passed, but on a tree no commit describes. Report that
  # honestly instead of a PASS nobody can reproduce from source.
  result="DIRTY"
fi

finished_at="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
finished_ms="$(now_ms)"

python3 - "$JOURNAL" "$ARTIFACTS" "$WARNINGS" "$REPORT_TMP" "$report" \
  "$head" "$started_at" "$finished_at" "$(( finished_ms - started_ms ))" \
  "$result" "$packaged" "$dirty_at_start" "${step_list[@]}" <<'PY'
import json
import os
import platform
import shutil
import sys
from pathlib import Path

(journal, artifacts_path, warnings_path, report_tmp, report_path, head,
 started_at, finished_at, elapsed_ms, result, packaged, dirty_at_start,
 *requested) = sys.argv[1:]

steps = []
with open(journal, encoding="utf-8") as handle:
    for line in handle:
        if line.strip():
            steps.append(json.loads(line))

artifacts = []
with open(artifacts_path, encoding="utf-8") as handle:
    for line in handle:
        step, path, digest, size = line.rstrip("\n").split("\t")
        artifacts.append({"step": step, "path": path, "sha256": digest,
                          "bytes": int(size)})

ran = [step["id"] for step in steps]
not_run = [step for step in requested if step not in ran]

def memory_total() -> int | None:
    try:
        for line in Path("/proc/meminfo").read_text().splitlines():
            if line.startswith("MemTotal:"):
                return int(line.split()[1]) * 1024
    except OSError:
        return None
    return None

limits = [
    "Local developer evidence for this working tree on this named host only.",
    "Hosted platform-verification remains the authority for native, packaged and cross-platform claims; this lane never promotes itself to ACCEPTED.",
    "A step this host cannot run is BLOCKED or UNVERIFIED and the lane exits non-zero.",
    "A run on a dirty tree reports DIRTY and can never report PASS, because no commit describes that tree.",
    "The flutter step runs the widget suite without a native device; the preflight step runs one native debug app under Xvfb with software GL.",
]
if str(packaged) == "1":
    limits += [
        "The packaged steps are local developer evidence: this host's Flutter is not the CI pin, the media runtime is built locally, and the .deb is extracted rather than installed by apt.",
        "A local packaged journey is not an installed-package or hosted-packaging claim.",
    ]

payload = {
    "lane": "linux",
    "packaged": str(packaged) == "1",
    "evidence_class": "local-developer",
    "acceptance": False,
    "source": head,
    "dirty_at_start": dirty_at_start == "1",
    "host": {
        "os": platform.system(),
        "arch": platform.machine(),
        "kernel": platform.release(),
        "memory_total_bytes": memory_total(),
        "free_disk_bytes": shutil.disk_usage(os.getcwd()).free,
    },
    "started_at": started_at,
    "finished_at": finished_at,
    "elapsed_seconds": round(int(elapsed_ms) / 1000.0, 2),
    "result": result,
    "steps": steps,
    "not_run": not_run,
    "artifacts": artifacts,
    "warnings": [line for line in Path(warnings_path).read_text(
        encoding="utf-8").splitlines() if line],
    "limits": limits,
    "report": str(Path(report_path).resolve()),
}
Path(report_tmp).write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
PY

mv "$REPORT_TMP" "$report"

printf '\n== local %s lane ==\n' "$lane"
printf 'source        : %s\n' "$head"
printf 'report        : %s\n' "$report"
python3 - "$report" <<'PY'
import json
import sys

payload = json.loads(open(sys.argv[1], encoding="utf-8").read())
for step in payload["steps"]:
    print(f"  {step['id']:<10} {step['classification']:<10} "
          f"{step['elapsed_seconds']:>7.2f}s  exit {step['exit_code']}")
for step in payload["not_run"]:
    print(f"  {step:<10} NOT RUN")
print(f"RESULT: LOCAL_LANE={payload['result']}")
PY

case "$result" in
  PASS) exit 0 ;;
  FAIL) exit 1 ;;
  UNVERIFIED|DIRTY) exit 4 ;;
  *) exit 3 ;;
esac
