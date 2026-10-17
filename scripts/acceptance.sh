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
unverified_lane() {
  local name=$1 reason=$2
  printf '\n== %s ==\n' "$name"
  printf 'RESULT: UNVERIFIED — %s\n' "$reason"
  unverified+=("$name: $reason")
}

run() {
  local name=$1; shift
  printf '\n== %s ==\n' "$name"
  local rc=0
  timeout --signal=TERM --kill-after=30 "${CHECK_TIMEOUT}s" "$@" || rc=$?
  case "$rc" in
    0) printf 'RESULT: PASS — %s\n' "$name" ;;
    124|137) printf 'RESULT: FAIL (timeout %ss) — %s\n' "$CHECK_TIMEOUT" "$name"; failed=1 ;;
    *) printf 'RESULT: FAIL (exit %s) — %s\n' "$rc" "$name"; failed=1 ;;
  esac
}

run 'Repository hygiene, document and claim gates' \
  bash scripts/check-repo.sh

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
fi

printf '\nNOTE: Android, macOS, Windows and packaged-acceptance lanes remain hosted-CI authority and are UNVERIFIED here.\n'
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
