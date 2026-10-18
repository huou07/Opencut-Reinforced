#!/usr/bin/env bash
# Build every Android case APK the local lane drives, plus the SAF fixture app.
#
#   bash scripts/build_android_case_apks.sh <output-directory>
#
# Why this is a separate script and not lines inside the lane: each Android
# acceptance case has its own Dart entrypoint, so each needs its own APK, and
# `scripts/run_android_lane.sh` records exactly one single-line command per
# step. This mirrors the `android` job in
# `.github/workflows/platform-verification.yml`, which builds all four case APKs
# before it boots any guest so that Gradle and the Kotlin daemons never compete
# with the software-rendered emulator (see the measured reasons in
# docs/TESTING.md).
#
# It also creates `apps/or_app/android/.ffmpeg-install`, the staged FFmpeg
# install that `packages/or_app_bridge/hook/build.dart` requires before it will
# build an Android bridge; the path is pinned by the `android_ffmpeg_install_root`
# user define in `apps/or_app/pubspec.yaml`. Without that link the APK build
# fails before it starts. Hosted CI creates the same link from its runner temp
# staging root, and `scripts/stage_android_ffmpeg.sh` produces a local prefix.
#
# Inputs:
#   OR_ANDROID_FFMPEG_PREFIX   staged x86_64 Android FFmpeg install prefix; it
#                             must contain lib/pkgconfig.
#   OR_CLI_BOOTSTRAP_JSON      optional; when unset the script derives it from
#                             the CLI exactly as CI does, because the
#                             bridge-diagnostics case asserts it is non-empty.
# Output (in <output-directory>):
#   diagnostics.apk surface.apk saf.apk saf-recovery.apk saf_fixture.apk ui.apk
#
# Exit codes: 0 every APK was produced; 1 a build failed; 2 the invocation or an
# input is wrong; 3 a host precondition is missing.
set -euo pipefail

if [[ $# -ne 1 ]]; then
  printf 'usage: bash scripts/build_android_case_apks.sh <output-directory>\n' >&2
  exit 2
fi

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
APP="$ROOT/apps/or_app"
output_dir="$1"
mkdir -p "$output_dir"
output_dir="$(cd -- "$output_dir" && pwd)"

if [[ -z "${OR_ANDROID_FFMPEG_PREFIX:-}" ]]; then
  printf 'build_android_case_apks: OR_ANDROID_FFMPEG_PREFIX is unset; run scripts/stage_android_ffmpeg.sh first\n' >&2
  exit 3
fi
if [[ ! -d "$OR_ANDROID_FFMPEG_PREFIX/lib/pkgconfig" ]]; then
  printf 'build_android_case_apks: OR_ANDROID_FFMPEG_PREFIX=%s has no lib/pkgconfig\n' \
    "$OR_ANDROID_FFMPEG_PREFIX" >&2
  exit 3
fi
for tool in flutter adb; do
  command -v "$tool" >/dev/null 2>&1 || {
    printf 'build_android_case_apks: missing tool on PATH: %s\n' "$tool" >&2
    exit 3
  }
done

# The bridge hook resolves this path relative to apps/or_app. Replace any stale
# link so a prefix that moved is never silently reused.
staging="$APP/android/.ffmpeg-install"
ln -sfn "$OR_ANDROID_FFMPEG_PREFIX" "$staging"
if [[ ! -f "$staging/lib/pkgconfig/libavcodec.pc" ]]; then
  printf 'build_android_case_apks: %s does not resolve to a staged FFmpeg install\n' "$staging" >&2
  exit 3
fi

bootstrap_json="${OR_CLI_BOOTSTRAP_JSON:-}"
if [[ -z "$bootstrap_json" ]]; then
  cli="$ROOT/target/debug/or"
  if [[ ! -x "$cli" ]]; then
    printf 'build_android_case_apks: target/debug/or is absent; running `cargo build -p or_cli --locked`\n' >&2
    (cd "$ROOT" && cargo build -p or_cli --locked) >&2
  fi
  bootstrap_json="$(cd "$ROOT" && python3 - "$cli" <<'PY'
import json
import subprocess
import sys

cli = sys.argv[1]
commands = {
    "app_info": ["version", "--json"],
    "health": ["health", "--json"],
    "capabilities": ["capabilities", "--json"],
}
snapshot = {}
for key, arguments in commands.items():
    output = subprocess.check_output([cli, *arguments], text=True)
    snapshot[key] = json.loads(output)
print(json.dumps(snapshot, separators=(",", ":")))
PY
)"
fi
if [[ -z "$bootstrap_json" ]]; then
  printf 'build_android_case_apks: could not derive OR_CLI_BOOTSTRAP_JSON; the bridge-diagnostics case requires it\n' >&2
  exit 1
fi

build_case_apk() {
  local name="$1" entrypoint="$2"
  shift 2
  (cd "$APP" && flutter build apk --debug --target-platform android-x64 --target "$entrypoint" "$@")
  local built="$APP/build/app/outputs/flutter-apk/app-debug.apk"
  test -s "$built"
  cp "$built" "$output_dir/$name.apk"
  test -s "$output_dir/$name.apk"
}

build_case_apk diagnostics integration_test/native_bridge_diagnostics_test.dart \
  "--dart-define=OR_CLI_BOOTSTRAP_JSON=$bootstrap_json"
build_case_apk surface integration_test/android_preview_surface_test.dart
build_case_apk saf integration_test/android_saf_preview_test.dart
build_case_apk saf-recovery integration_test/android_saf_recovery_test.dart
# The agent-facing UI harness drives the same app through a JSON scenario; it is
# its own Dart entrypoint, so it is its own APK. `scripts/or_ui.py` installs it,
# pushes a scenario into the app's files directory and pulls the transcript back.
build_case_apk ui integration_test/or_ui_scenario_test.dart

# A separate permission-enforced provider app for the SAF journey. The CI pin is
# `./gradlew -PorSafFixture=true :saf_fixture:assembleDebug`.
(cd "$APP/android" && ./gradlew -PorSafFixture=true :saf_fixture:assembleDebug)
fixture="$APP/build/saf_fixture/outputs/apk/debug/saf_fixture-debug.apk"
test -s "$fixture"
cp "$fixture" "$output_dir/saf_fixture.apk"

printf '\n== android case APKs ==\n'
for apk in diagnostics surface saf saf-recovery saf_fixture; do
  printf '%-14s %s  %s\n' "$apk" \
    "$(sha256sum "$output_dir/$apk.apk" | awk '{print $1}')" "$output_dir/$apk.apk"
done
