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

# The bridge hook resolves this path relative to apps/or_app and then builds the
# per-ABI prefix as `<link>/<abi>`. The link must therefore be the FFmpeg
# *install root* that holds one directory per ABI, not one ABI's prefix.
# Measured 2026-10-18: linking it at `.../install/x86_64` made the hook look for
# `.../install/x86_64/x86_64/include`, so bindgen failed with
# "'libavutil/adler32.h' file not found" and the APK carried no FFmpeg at all.
ffmpeg_install_root="$(dirname -- "$OR_ANDROID_FFMPEG_PREFIX")"
staging="$APP/android/.ffmpeg-install"
ln -sfn "$ffmpeg_install_root" "$staging"
if [[ ! -f "$staging/x86_64/lib/pkgconfig/libavcodec.pc" ]]; then
  printf 'build_android_case_apks: %s does not resolve to an FFmpeg install root holding x86_64/lib/pkgconfig (OR_ANDROID_FFMPEG_PREFIX=%s)\n' \
    "$staging" "$OR_ANDROID_FFMPEG_PREFIX" >&2
  exit 3
fi

# --- the Android build environment ------------------------------------------
# This workstation's shared Cargo config ($HOME/.cargo/config.toml) force-pins
# FFMPEG_DIR, PKG_CONFIG_PATH and a native search path to the *Linux desktop*
# FFmpeg prefix. `force = true` beats the environment, and the bridge's
# native-assets hook does not pass CARGO_HOME through, so a naive Android build
# silently links host libraries: measured 2026-10-18, the first locally built
# libor_app_bridge.so required libavcodec.so.62 (the Linux soname) while the
# staged x86_64 Android prefix provides an unversioned SONAME, so the APK could
# not load on any guest and the FFmpeg link probe correctly refused it.
#
# Two independent defences, because the failure mode was silent. (1) This script
# gives the build its own HOME whose .cargo config pins the *Android* prefix and
# reuses the staged project-local registry, so it stays offline. (2) The bridge
# hook states the same pin on cargo's command line and carries the native search
# path in RUSTFLAGS, which *replaces* the config's rustflags — cargo concatenates
# `build.rustflags` arrays, so a config-supplied `-L` cannot be overridden by
# another config file. CI runners have no host config, so neither changes
# anything there.
prefix="$OR_ANDROID_FFMPEG_PREFIX"
android_build_home="${OR_ANDROID_BUILD_HOME:-$APP/.dart_tool/android-build-home}"
android_cargo_home="$android_build_home/.cargo"
real_home="${HOME:-/root}"
real_pub_cache="${PUB_CACHE:-$real_home/.pub-cache}"
mkdir -p "$android_cargo_home"
cargo_seed="${OR_ANDROID_CARGO_SEED:-$APP/.dart_tool/aeh-cargo}"
for entry in registry git; do
  if [[ -d "$cargo_seed/$entry" && ! -e "$android_cargo_home/$entry" ]]; then
    ln -sfn "$cargo_seed/$entry" "$android_cargo_home/$entry"
  fi
done
cat > "$android_cargo_home/config.toml" <<EOF
# Written by scripts/build_android_case_apks.sh for one Android build; it pins
# the Android FFmpeg prefix so the host's forced Linux pin cannot win.
[env]
FFMPEG_DIR = { value = "$prefix", force = true }
PKG_CONFIG_PATH = { value = "$prefix/lib/pkgconfig", force = true }
PKG_CONFIG_ALLOW_CROSS = { value = "1", force = true }
EOF
export HOME="$android_build_home"
export CARGO_HOME="$android_cargo_home"
export PUB_CACHE="$real_pub_cache"

# The runtime libraries the bridge needs at load time must be inside the APK.
# CI copies them into the module's jniLibs in its FFmpeg step; the local lane had
# no equivalent, which is why a locally built APK could not load its own bridge.
jni_root="$APP/android/app/src/main/jniLibs"
mkdir -p "$jni_root/x86_64"
for library in avcodec avformat avutil swresample swscale; do
  shared="$prefix/lib/lib$library.so"
  if [[ ! -f "$shared" ]]; then
    printf 'build_android_case_apks: %s is missing from the staged prefix\n' "$shared" >&2
    exit 3
  fi
  for runtime in "$prefix/lib/lib$library.so"*; do
    [[ -e "$runtime" ]] || continue
    cp -L "$runtime" "$jni_root/x86_64/$(basename "$runtime")"
  done
done

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

# Reads the ELF dynamic section of the bridge inside a built APK and refuses an
# APK whose bridge requires a library the APK does not carry. This is the check
# that would have caught the Linux-soname mistake before a guest was booted.
verify_apk_loads_its_bridge() { # apk
  local apk="$1" inspect missing=() needed sofile
  inspect="$(mktemp -d)"
  unzip -jo -q "$apk" 'lib/x86_64/libor_app_bridge.so' -d "$inspect" 2>/dev/null || {
    printf 'build_android_case_apks: %s carries no x86_64 bridge library\n' "$apk" >&2
    rm -rf "$inspect"
    return 1
  }
  local reader=""
  for tool in llvm-readelf readelf; do
    command -v "$tool" >/dev/null 2>&1 && { reader="$tool"; break; }
  done
  if [[ -z "$reader" ]]; then
    printf 'build_android_case_apks: no readelf on PATH; cannot check %s bridge requirements\n' \
      "$apk" >&2
    rm -rf "$inspect"
    return 3
  fi
  while IFS= read -r needed; do
    [[ -n "$needed" ]] || continue
    sofile="$(unzip -l "$apk" 2>/dev/null | awk -v lib="$needed" '$NF ~ ("lib/x86_64/" lib "$") {found=1} END{print found+0}')"
    if [[ "$sofile" != "1" ]]; then
      missing+=("$needed")
    fi
  done < <("$reader" -d "$inspect/lib/x86_64/libor_app_bridge.so" 2>/dev/null \
    | sed -n 's/.*NEEDED.*\[\(libav[^]]*\|libsw[^]]*\)\].*/\1/p')
  rm -rf "$inspect"
  if (( ${#missing[@]} > 0 )); then
    printf 'build_android_case_apks: %s requires libraries it does not bundle: %s\n' \
      "$apk" "${missing[*]}" >&2
    printf '  the build linked a foreign FFmpeg; re-run scripts/stage_android_ffmpeg.sh and rebuild\n' >&2
    return 1
  fi
  return 0
}

build_case_apk() {
  local name="$1" entrypoint="$2"
  shift 2
  (cd "$APP" && flutter build apk --debug --target-platform android-x64 --target "$entrypoint" "$@")
  local built="$APP/build/app/outputs/flutter-apk/app-debug.apk"
  test -s "$built"
  cp "$built" "$output_dir/$name.apk"
  test -s "$output_dir/$name.apk"
  verify_apk_loads_its_bridge "$output_dir/$name.apk"
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
