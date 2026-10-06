#!/usr/bin/env bash
# Stops the Android emulator started by start-android-emulator.sh, if any.
# Usage: stop-android-emulator.sh
set -uo pipefail
pid_file="${RUNNER_TEMP:-/tmp}/android-emulator.pid"
if [[ -f "$pid_file" ]]; then
  kill "$(cat "$pid_file")" 2>/dev/null || true
  rm -f "$pid_file"
fi
exit 0