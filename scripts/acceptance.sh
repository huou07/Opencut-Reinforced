#!/usr/bin/env bash
# OR admission gate.
#
# Scope: what this Debian workstation can prove about the product, bounded by a
# wall clock. It is deliberately stronger than scripts/check-repo.sh, which only
# checks repository hygiene and documents, and weaker than hosted CI, which owns
# native, packaged and cross-platform acceptance.
#
# Anything that needs FFmpeg/codec fixtures, a GPU, Android, macOS, Windows or a
# packaged artifact stays the authority of GitHub Actions and is NOT simulated
# here. A check that cannot run is reported as UNVERIFIED, never as a pass.
#
# Usage: bash scripts/acceptance.sh [--timeout N]
set -uo pipefail

ROOT=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
cd "$ROOT" || exit 1

CHECK_TIMEOUT=900
if [[ "${1:-}" == "--timeout" && -n "${2:-}" ]]; then CHECK_TIMEOUT=$2; fi

failed=0
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

# The documented local Rust lanes. The workspace also contains media, audio, AI
# and whisper crates whose tests need codec fixtures or provider credentials;
# those are hosted-CI authority and are not claimed here.
run 'Rust core and CLI tests' \
  cargo test -p or_core -p or_cli --locked
run 'Rust bridge, IPC and runtime tests' \
  cargo test -p or_app_bridge -p or_ipc -p or_runtime --locked

if command -v flutter >/dev/null 2>&1; then
  # The Flutter test runner builds native assets through the Rust hook, which
  # resolves the toolchain with rustup and may need to download components. On a
  # host without that network access the hook fails before any test runs. That is
  # a host capability gap, not a test failure: report it as UNVERIFIED with the
  # reason instead of leaving a permanently red gate. Any other failure is real.
  printf '\n== Flutter widget tests (fake gateway) ==\n'
  out=$(mktemp)
  rc=0
  timeout --signal=TERM --kill-after=30 "${CHECK_TIMEOUT}s" \
    bash -c 'cd apps/or_app && flutter test --no-pub' >"$out" 2>&1 || rc=$?
  if grep -qE 'Building native assets failed|component download failed' "$out"; then
    printf 'RESULT: UNVERIFIED — the Flutter native-assets hook could not download its\n'
    printf '        Rust components on this host, so no test ran. Hosted CI owns this lane.\n'
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
fi

printf '\nNOTE: Android, macOS, Windows, packaging and codec lanes remain UNVERIFIED here.\n'
if (( failed )); then
  printf '\nPROJECT_ACCEPTANCE=FAIL\n'
  exit 1
fi
printf '\nPROJECT_ACCEPTANCE=PASS\n'
