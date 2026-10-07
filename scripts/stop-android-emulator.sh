#!/usr/bin/env bash
# Stops the Android emulator started by start-android-emulator.sh and waits
# until its process has exited before another emulator lifecycle can start.
# Usage: stop-android-emulator.sh
set -euo pipefail
pid_file="${RUNNER_TEMP:-/tmp}/android-emulator.pid"
if [[ -f "$pid_file" ]]; then
  emulator_pid="$(cat "$pid_file")"
  if [[ ! "$emulator_pid" =~ ^[0-9]+$ ]]; then
    echo "Invalid Android emulator PID in $pid_file" >&2
    exit 1
  fi

  process_state() {
    ps -o stat= -p "$emulator_pid" 2>/dev/null | tr -d '[:space:]'
  }
  process_running() {
    local state
    state="$(process_state)"
    [[ -n "$state" && "$state" != Z* ]]
  }

  if process_running; then
    kill -TERM "$emulator_pid" 2>/dev/null || true
    # The emulator may take about 20 seconds to flush its guest before exit.
    # Wait up to 25 seconds, then force-stop only this script's recorded PID.
    wait_seconds="${OR_ANDROID_EMULATOR_STOP_WAIT_SECONDS:-25}"
    if [[ ! "$wait_seconds" =~ ^[0-9]+$ ]]; then
      echo "Invalid Android emulator stop wait: $wait_seconds" >&2
      exit 1
    fi
    for ((attempt = 0; attempt < wait_seconds * 5; attempt++)); do
      process_running || break
      sleep 0.2
    done
    if process_running; then
      kill -KILL "$emulator_pid" 2>/dev/null || true
      for _ in $(seq 1 25); do
        process_running || break
        sleep 0.2
      done
    fi
    if process_running; then
      echo "Android emulator process $emulator_pid did not exit after SIGKILL" >&2
      exit 1
    fi
  fi
  rm -f "$pid_file"
fi
