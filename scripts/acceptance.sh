#!/usr/bin/env bash
# OR admission gate.
#
# Scope: what this Debian workstation can prove about the product, bounded by a
# wall clock. It is deliberately stronger than scripts/check-repo.sh, which only
# checks repository hygiene and documents, and weaker than hosted CI, which owns
# native, packaged and cross-platform acceptance.
#
# Codec-fixture lanes run here when the pinned FFmpeg 8 binaries are installed
# (FFmpeg 8 is required: the proxy filter graph uses `scale=...:reset_sar=1`,
# which Debian 13's FFmpeg 7 rejects). Anything that needs a GPU, Android, macOS,
# Windows or a packaged artifact stays the authority of GitHub Actions and is NOT
# simulated here. A check that cannot run is reported as UNVERIFIED, never as a
# pass, and every unselected lane is named in the final record.
#
# Usage: bash scripts/acceptance.sh [--timeout N]
set -uo pipefail

ROOT=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
cd "$ROOT" || exit 1

CHECK_TIMEOUT=900
if [[ "${1:-}" == "--timeout" && -n "${2:-}" ]]; then CHECK_TIMEOUT=$2; fi

failed=0
unverified=()
# Machine-readable gate report for `.aeh/quality.json`. `not_run` mirrors every
# lane this run did not execute, with the reason printed on the console, and
# `covered_gates` only ever names a gate whose local checks actually ran and
# passed. The report is runtime state under `.hub/` and is gitignored.
not_run_checks=()
covered_gates=()
unverified_lane() {
  local name=$1 reason=$2
  printf '\n== %s ==\n' "$name"
  printf 'RESULT: UNVERIFIED — %s\n' "$reason"
  unverified+=("$name: $reason")
  not_run_checks+=("$name"$'\t'"$reason")
}

# Exit status of the most recent `run` call, for callers that need to know
# whether a gate passed without re-running it.
run_rc=0
run() {
  local name=$1; shift
  printf '\n== %s ==\n' "$name"
  local rc=0
  timeout --signal=TERM --kill-after=30 "${CHECK_TIMEOUT}s" "$@" || rc=$?
  run_rc=$rc
  case "$rc" in
    0) printf 'RESULT: PASS — %s\n' "$name" ;;
    124|137) printf 'RESULT: FAIL (timeout %ss) — %s\n' "$CHECK_TIMEOUT" "$name"; failed=1 ;;
    *) printf 'RESULT: FAIL (exit %s) — %s\n' "$rc" "$name"; failed=1 ;;
  esac
}

run 'Repository hygiene, document and claim gates' \
  bash scripts/check-repo.sh
if (( run_rc == 0 )); then
  covered_gates+=("source-integrity")
fi

# The documented local Rust lanes.
run 'Rust core and CLI tests' \
  cargo test -p or_core -p or_cli --locked
run 'Rust bridge, IPC and runtime tests' \
  cargo test -p or_app_bridge -p or_ipc -p or_runtime --locked

# The media lanes and the eight tests the media crates mark `#[ignore]` were
# previously omitted entirely, so a green gate said nothing about them. They are
# runnable here once the pinned FFmpeg 8 binaries are on PATH: the proxy filter
# graph uses `scale=...:reset_sar=1`, an FFmpeg 8 option, so Debian 13's FFmpeg 7
# fails it. Widen to them when FFmpeg 8 is installed; otherwise name every
# omitted lane as UNVERIFIED instead of silently dropping it.
FFMPEG8_DIR="${HOME}/.local/ffmpeg8"
if [[ -x "$FFMPEG8_DIR/ffmpeg" && -x "$FFMPEG8_DIR/ffprobe" ]]; then
  export PATH="$FFMPEG8_DIR:$PATH"
  printf '\n== Media codec toolchain ==\n'
  printf 'using FFmpeg 8 from %s (%s)\n' "$FFMPEG8_DIR" "$("$FFMPEG8_DIR/ffmpeg" -version | head -n1)"
  run 'Media probe integration tests (real FFmpeg 8)' \
    cargo test -p or_core --test media_probe_integration --locked -- --ignored
  run 'Media artifact integration tests (real FFmpeg 8)' \
    cargo test -p or_core --test media_artifacts_integration --locked -- --ignored
  run 'Media crate tests' \
    cargo test -p or_media --locked
  run 'Media crate codec-enabled export writer' \
    cargo test -p or_media --test software_export --locked -- --ignored
else
  unverified_lane 'Media probe integration tests (5 ignored tests)' \
    "FFmpeg 8 is not installed at $FFMPEG8_DIR, and the proxy filter graph needs its reset_sar option"
  unverified_lane 'Media artifact integration tests (2 ignored tests)' \
    "FFmpeg 8 is not installed at $FFMPEG8_DIR, and the proxy filter graph needs its reset_sar option"
  unverified_lane 'or_media crate tests and the codec-enabled export writer' \
    "or_media was not selected because the pinned FFmpeg 8 binaries are not installed at $FFMPEG8_DIR"
fi

# One bounded performance budget, measured on this named host class. A breach
# fails the gate; a host that cannot produce an honest number is UNVERIFIED with
# the exact command that would settle it, never a fabricated threshold. Scope,
# baselines and limits are in `scripts/bench_quality.py` and `docs/reference/quality.md`.
printf '\n== Performance budget (local, Debian OptiPlex-class host) ==\n'
bench_rc=0
timeout --signal=TERM --kill-after=30 "${CHECK_TIMEOUT}s" \
  python3 scripts/bench_quality.py || bench_rc=$?
case "$bench_rc" in
  0) printf 'RESULT: PASS — Performance budget\n' ;;
  3)
    printf 'RESULT: UNVERIFIED — the benchmark could not measure a row on this host; see the reason above.\n'
    unverified+=("Performance budget: a benchmark row could not be measured on this host")
    not_run_checks+=("Performance budget"$'\t'"a benchmark row could not be measured on this host; the exact command to settle it is: cargo test -p or_media --test software_export --no-run --locked && python3 scripts/bench_quality.py")
    ;;
  124|137) printf 'RESULT: FAIL (timeout %ss) — Performance budget\n' "$CHECK_TIMEOUT"; failed=1 ;;
  *) printf 'RESULT: FAIL (exit %s) — Performance budget\n' "$bench_rc"; failed=1 ;;
esac

# Crates the bounded local gate deliberately does not select. They stay named so
# the record never reads as broader coverage than it has.
unverified_lane 'or_render tests' \
  'not selected by this bounded local gate; GPU/offscreen-render behaviour is hosted-CI authority'
unverified_lane 'or_audio tests' \
  'not selected by this bounded local gate; there is no audio device on this host and audio behaviour is hosted-CI authority'
unverified_lane 'or_ai tests' \
  'not selected by this bounded local gate; provider credentials and hosted model runtimes are owner/hosted-CI authority'
unverified_lane 'or_whisper_provider tests' \
  'not selected by this bounded local gate; the model download and runtime are owner/hosted-CI authority'

if command -v flutter >/dev/null 2>&1; then
  # The Flutter test runner can fail to start for host reasons: the native-assets
  # hook may need to download Rust components, and the `flutter` wrapper rewrites
  # `bin/cache/engine.stamp` beside its own script, which a read-only shared SDK
  # cache denies. Both are host capability gaps, not test failures: report
  # UNVERIFIED with the reason instead of leaving a permanently red gate. The
  # classification is deliberately narrow. It applies only when the run produced
  # no test progress at all, so a suite that starts and then fails is still FAIL.
  printf '\n== Flutter widget tests (fake gateway) ==\n'
  out=$(mktemp)
  rc=0
  timeout --signal=TERM --kill-after=30 "${CHECK_TIMEOUT}s" \
    bash -c 'cd apps/or_app && flutter test --no-pub' >"$out" 2>&1 || rc=$?
  host_gap=''
  if ! grep -qE '^[0-9]{2}:[0-9]{2} \+[0-9]+' "$out"; then
    if grep -qE 'Building native assets failed|component download failed' "$out"; then
      host_gap='the Flutter native-assets hook could not download its Rust components on this host, so no test ran'
    elif grep -qE 'update_engine_version\.sh.*Permission denied' "$out"; then
      host_gap='the shared Flutter SDK cache is not writable by this user, so the flutter wrapper could not start and no test ran'
    fi
  fi
  if [[ -n "$host_gap" ]]; then
    printf 'RESULT: UNVERIFIED — %s.\n' "$host_gap"
    printf '        Hosted CI owns this lane; this is not a pass.\n'
    unverified+=("Flutter widget tests: $host_gap")
    not_run_checks+=("Flutter widget tests"$'\t'"$host_gap")
  else
    case "$rc" in
      0) printf 'RESULT: PASS — Flutter widget tests (fake gateway)\n' ;;
      124|137) printf 'RESULT: FAIL (timeout %ss) — Flutter widget tests\n' "$CHECK_TIMEOUT"; failed=1 ;;
      *) printf 'RESULT: FAIL (exit %s) — Flutter widget tests\n' "$rc"; failed=1 ;;
    esac
  fi
  rm -f "$out"
else
  printf '\nRESULT: UNVERIFIED — flutter is not installed; UI behaviour was not exercised\n'
  unverified+=("Flutter widget tests: flutter is not installed")
  not_run_checks+=("Flutter widget tests"$'\t'"flutter is not installed")
fi

# The bounded Linux UI preflight is the one runtime lane this workstation can
# execute end to end: a native Flutter debug app under Xvfb with real Rust state,
# canonical screenshots and a semantics dump. It refuses to run on a dirty tree
# and on insufficient RAM/disk so its temporary toolchain rewrite stays
# attributable; those preconditions are UNVERIFIED, not failures.
printf '\n== Bounded Linux UI preflight (native editor journey under Xvfb) ==\n'
preflight_rc=0
timeout --signal=TERM --kill-after=30 "${CHECK_TIMEOUT}s" \
  python3 scripts/run_linux_ui_preflight.py || preflight_rc=$?
case "$preflight_rc" in
  0) printf 'RESULT: PASS — Bounded Linux UI preflight\n' ;;
  3)
    printf 'RESULT: UNVERIFIED — the bounded preflight reported a host precondition (clean tree, RAM, disk or tool availability)\n'
    unverified+=("Bounded Linux UI preflight: host precondition not met; see its report")
    not_run_checks+=("Bounded Linux UI preflight"$'\t'"host precondition not met (clean tree, RAM, disk or tool availability); see its report")
    ;;
  124|137) printf 'RESULT: FAIL (timeout %ss) — Bounded Linux UI preflight\n' "$CHECK_TIMEOUT"; failed=1 ;;
  *) printf 'RESULT: FAIL (exit %s) — Bounded Linux UI preflight\n' "$preflight_rc"; failed=1 ;;
esac

if command -v gitleaks >/dev/null 2>&1; then
  # The repository has GitHub's hosted secret scanning and push protection, but
  # that is a hosting control: nothing local scanned for secrets before this.
  run "Secret scan (gitleaks $(gitleaks version 2>/dev/null || echo unknown))" \
    gitleaks detect --source . --redact --no-banner --exit-code 1
else
  printf '\n== Secret scan (gitleaks) ==\n'
  printf 'RESULT: UNVERIFIED — gitleaks is not installed, so no secret scan backed this gate.\n'
  printf '        Install gitleaks or rely on the pinned scan in repo-hygiene.yml.\n'
  unverified+=("Secret scan: gitleaks is not installed")
  not_run_checks+=("Secret scan (gitleaks)"$'\t'"gitleaks is not installed, so no secret scan backed this gate; the pinned scan in repo-hygiene.yml owns it")
fi

# The hosted release lanes are the authority for native, packaged and
# cross-platform claims; this local gate never simulates them. They are named so
# the gate report and the console agree about what this run did not cover.
unverified_lane 'platform-verification' \
  'GitHub Actions authority: the hosted Android, macOS, Windows and Linux platform matrix was not run on this host'
unverified_lane 'release-gate' \
  'the aggregate hosted release-gate check that gates merges runs only in GitHub Actions'
unverified_lane 'packaging' \
  'packaged installers and their independent decode checks are built and installed only in hosted CI'

# Emit the gate report the project-owned quality contract declares. It is written
# on both outcomes: a failed gate still has to say which lanes ran. A lane that
# ran is never in `not_run`, and a gate is never `covered` unless its checks
# passed, so the report can only understate coverage, never overstate it.
write_gate_report() {
  local report_dir="$ROOT/.hub"
  local report_path="$report_dir/gate-report.json"
  local not_run_file covered_file
  not_run_file="$(mktemp)"
  covered_file="$(mktemp)"
  if (( ${#not_run_checks[@]} )); then
    printf '%s\n' "${not_run_checks[@]}" >"$not_run_file"
  fi
  if (( ${#covered_gates[@]} )); then
    printf '%s\n' "${covered_gates[@]}" >"$covered_file"
  fi
  mkdir -p "$report_dir"
  if python3 - "$report_path" "$not_run_file" "$covered_file" "$(uname -s)" "$(uname -m)" <<'PY'
import json
import pathlib
import sys

report_path, not_run_path, covered_path, os_name, arch = sys.argv[1:6]


def lines(path):
    try:
        text = pathlib.Path(path).read_text(encoding="utf-8")
    except OSError:
        return []
    return [line for line in text.splitlines() if line]


not_run = []
for line in lines(not_run_path):
    check, _, reason = line.partition("\t")
    not_run.append({"check": check, "reason": reason})

payload = {
    "skipped": [],
    "not_run": not_run,
    "covered_gates": lines(covered_path),
    "artifacts": [],
    "runner": {
        "kind": "local",
        "os": os_name.lower(),
        "arch": arch,
        "label": "debian-optiplex-class",
    },
    "evidence_class": "local-source",
    "limitations": [
        "This report is local Debian evidence only; the hosted release gates (platform-verification, release-gate and packaging) were not executed here.",
        "Every lane this gate did not run is named in not_run with the reason it printed; covered_gates names only gates whose local checks passed.",
        "The bounded performance budget is a coarse best-of-N regression guard for the named Debian OptiPlex-class host, not a frame-time or latency budget.",
    ],
}
pathlib.Path(report_path).write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
PY
  then
    printf '\ngate report: %s\n' "$report_path"
  else
    printf '\nWARN: could not write the gate report at %s\n' "$report_path" >&2
  fi
  rm -f "$not_run_file" "$covered_file"
}

printf '\nNOTE: Android, macOS, Windows and packaged-acceptance lanes remain hosted-CI authority and are UNVERIFIED here.\n'
if (( failed == 0 )); then
  covered_gates+=("product-build")
fi
write_gate_report
if (( failed )); then
  printf '\nPROJECT_ACCEPTANCE=FAIL\n'
  exit 1
fi
if (( ${#unverified[@]} )); then
  printf '\nPROJECT_ACCEPTANCE=PASS with %d UNVERIFIED lane(s):\n' "${#unverified[@]}"
  for lane in "${unverified[@]}"; do
    printf '  - %s\n' "$lane"
  done
else
  printf '\nPROJECT_ACCEPTANCE=PASS\n'
fi
