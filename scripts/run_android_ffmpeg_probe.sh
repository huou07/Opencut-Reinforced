#!/usr/bin/env bash
# Prove that the packaged Android FFmpeg libraries actually load on the guest.
#
#   bash scripts/run_android_ffmpeg_probe.sh <probe-binary> <diagnostics-apk> <serial>
#
# The APK carries the shared libraries the product ships; the probe binary is a
# Rust program linked against the staged x86_64 Android FFmpeg prefix (built by
# `scripts/stage_android_ffmpeg.sh`). Extraction from the APK is deliberate: it
# proves the libraries that are *in the package* load, not the ones that happen
# to sit in the staging prefix.
#
# This mirrors the `run_ffmpeg_runtime_probe` function in the hosted `android`
# job of `.github/workflows/platform-verification.yml`.
#
# Exit codes: 0 the probe ran and reported success; 1 the probe failed or the
# inputs are unusable; 2 the invocation is wrong; 3 a host precondition is missing.
set -euo pipefail

if [[ $# -ne 3 ]]; then
  printf 'usage: bash scripts/run_android_ffmpeg_probe.sh <probe-binary> <diagnostics-apk> <serial>\n' >&2
  exit 2
fi

probe="$1"
apk="$2"
serial="$3"

command -v adb >/dev/null 2>&1 || { printf 'run_android_ffmpeg_probe: adb is not on PATH\n' >&2; exit 3; }
command -v unzip >/dev/null 2>&1 || { printf 'run_android_ffmpeg_probe: unzip is not on PATH\n' >&2; exit 3; }
[[ -x "$probe" ]] || { printf 'run_android_ffmpeg_probe: probe is not executable: %s\n' "$probe" >&2; exit 3; }
[[ -s "$apk" ]] || { printf 'run_android_ffmpeg_probe: APK is missing: %s\n' "$apk" >&2; exit 1; }

work="${OR_ANDROID_PROBE_WORK:-$(mktemp -d)}"
mkdir -p "$work"
runtime_dir="$work/android-apk-ffmpeg-x86_64"
mkdir -p "$runtime_dir"

unzip -jo "$apk" 'lib/x86_64/libav*.so*' 'lib/x86_64/libsw*.so*' -d "$runtime_dir" >/dev/null
if ! compgen -G "$runtime_dir/*" >/dev/null; then
  printf 'run_android_ffmpeg_probe: %s carries no x86_64 libav*/libsw* libraries\n' "$apk" >&2
  exit 1
fi

adb -s "$serial" shell mkdir -p /data/local/tmp/ffmpeg-runtime
for library in "$runtime_dir"/*; do
  adb -s "$serial" push "$library" "/data/local/tmp/ffmpeg-runtime/$(basename "$library")" >/dev/null
done
adb -s "$serial" push "$probe" /data/local/tmp/ffmpeg-link-probe >/dev/null
adb -s "$serial" shell chmod 755 /data/local/tmp/ffmpeg-link-probe
adb -s "$serial" shell 'LD_LIBRARY_PATH=/data/local/tmp/ffmpeg-runtime /data/local/tmp/ffmpeg-link-probe'

printf 'run_android_ffmpeg_probe: the packaged FFmpeg libraries loaded on %s\n' "$serial"
