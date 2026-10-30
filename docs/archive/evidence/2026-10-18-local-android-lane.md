# One local Android lane — 2026-10-18

Status: **local developer evidence** for exact source
`1c731776130d0fe4a248a54137e8253cefa5f1c9`, recorded on the named Debian
workstation. It is not hosted CI evidence and it is not an Android acceptance
claim: Flutter here is 3.47.7 while the hosted jobs pin 3.47.5, and hosted
`platform-verification` remains the authority for native, packaged and
cross-platform behaviour.

This record covers `scripts/run_local_lane.sh android` after the wiring repair in
`f8eb2b2` and the staging recipe in `46eaa4f`. **The lane reports `BLOCKED` and
exits 3: the APK cannot be built because the shared Flutter SDK is not writable
by this host's worker UID.** That is a host precondition with a proven
discriminator, not a product defect, and it is recorded as such.

## What the earlier revision could not do

The lane that landed in `d5adcd1` could not have produced a guest result even
with a staged FFmpeg prefix. Three wiring defects, each read from source:

| Defect | Evidence | Consequence |
|---|---|---|
| `apk` ran `flutter build apk` without creating `apps/or_app/android/.ffmpeg-install` | `packages/or_app_bridge/hook/build.dart:11-20`, `apps/or_app/pubspec.yaml:22` | The bridge build hook throws before the build starts |
| `preview` invoked `run-android-preview-check.sh` with no arguments | `scripts/run-android-preview-check.sh:4-8` under `set -u`; each case has its own Dart entrypoint | The step failed on an unbound variable, and a single default APK could not have driven a case entrypoint anyway |
| `run-android-preview-check.sh` hardcoded `emulator-5554` | `scripts/run-android-preview-check.sh:15`; the lane reserves `emulator-5580` | The drive would have targeted the wrong guest |

A fourth defect was found while repairing them: the lane forced
`OR_ANDROID_GUEST_RAM_MB=2048`, below the 2560 MB image default that
`scripts/start-android-emulator.sh` records as too small for the product's own
bounded budgets.

## Staged inputs

`bash scripts/stage_android_ffmpeg.sh --root ~/Android/or-android-ffmpeg-8.1.3`
reproduced the pinned cross-build on this host:

| Item | Value |
|---|---|
| Prefix | `~/Android/or-android-ffmpeg-8.1.3/install/x86_64` |
| Probe | `~/Android/or-android-ffmpeg-8.1.3/probes/x86_64/ffmpeg-link-probe`, SHA-256 `bb8ffdf0de656475…`, 64-bit x86-64 ELF |
| Pins | FFmpeg 8.1.3 (SHA-512 verified), libvpx v1.17.0, libopus 1.6.1, NDK 28.2.13676358, API-26 clang, `--target-os=android --arch=x86_64` |
| Build info | `~/Android/or-android-ffmpeg-8.1.3/build-info/x86_64.txt` |

Building this required two defences against the host. The shared
`$CARGO_HOME/config.toml` pins `FFMPEG_DIR` and `PKG_CONFIG_PATH` with
`force = true` to a Linux desktop prefix, which silently linked the probe
against host libraries, and the probe needs the NDK linker or the host `cc`
cannot resolve `-llog`/`-lunwind`. Both are handled inside the staging script
with scoped `--config` overrides; no shared state was changed.

## Result

| Item | Value |
|---|---|
| Source HEAD | `1c731776130d0fe4a248a54137e8253cefa5f1c9` |
| Command | `OR_ANDROID_FFMPEG_PREFIX=… OR_ANDROID_FFMPEG_PROBE=… bash scripts/run_local_lane.sh android` |
| Lane result | `BLOCKED` (exit 3): `preflight` BLOCKED, every later step not run |
| Report | `apps/or_app/build/local-lanes/android.json` (gitignored), clean tree |
| Guest | AVD `an3-api36`, serial `emulator-5580`, configured 6144 MB — never booted |
| Host headroom at the probe | 7.65 GiB `MemAvailable`, no Gradle or Kotlin daemon resident |

The lane's own reason, verbatim:

> the Flutter SDK's Gradle plugin directory
> `/home/huou07/.local/share/flutter/packages/flutter_tools/gradle` is not
> writable by `dsh-worker`; Gradle must create a project cache there and
> otherwise fails with 'The settings are not yet available for build.' Grant
> write access to it (for example `setfacl -R -m u:dsh-worker:rwX` on that
> directory) before running the apk step

### Discriminator

The cause was separated from a repository defect by measurement:

| Observation | Result |
|---|---|
| `./gradlew projects` in `apps/or_app/android` | `FAILURE: The settings are not yet available for build.` (308-392 ms) |
| Same wrapper and distribution on a minimal project in `/tmp` | `BUILD SUCCESSFUL` |
| Minimal project with `includeBuild("<real flutter sdk>/packages/flutter_tools/gradle")` | same failure, exit 1 |
| Minimal project with `includeBuild("<writable copy of that same directory>")` | `BUILD SUCCESSFUL`, `Included build ':gtest-include'` |
| `git -C ~/.local/share/flutter status` | the SDK is intact; its Gradle directory is unmodified, merely not writable |

The included build is `includeBuild("$flutterSdkPath/packages/flutter_tools/gradle")`
from `apps/or_app/android/settings.gradle.kts`, and Gradle 9.3.1 needs to create
a project cache inside it. The SDK directory is owned by the host user with
`r-x` for worker UIDs, so the create fails and Gradle reports it as a missing
build definition. The lane now probes this and names it before the APK step, in
`scripts/run_android_lane.sh`, with
`scripts/test_android_lane.py::test_a_read_only_flutter_gradle_plugin_is_blocked_by_name`
pinning the rule hermetically.

## What this record does not establish

- No Android case was driven. The bridge-diagnostics, preview-surface and
  DocumentsUI SAF journeys were not executed, so no APK, no guest screenshot, no
  guest probe result and no per-case verdict exists in this session.
- The staged prefix and probe were **built** but never **loaded on a guest**.
  `scripts/run_android_ffmpeg_probe.sh` has not run.
- Nothing here is Android acceptance evidence; hosted `platform-verification`
  decides that, and a local guest result never would either.
- The lane's step arguments, blocked rules and artifact isolation are verified
  by `scripts/test_android_lane.py` (14 hermetic checks, no device), not by a
  real drive.

## What the hosted gates caught

The first push of the repaired lane passed `scripts/check-repo.sh` locally and
failed the hosted `Repository hygiene` job (run 39120503854). The hermetic
control-flow checks asserted that the lane drove its steps, but the lane's
preflight refuses to start without `/dev/kvm`, and a hosted runner has no
`/dev/kvm` at all, so every step check saw an empty capture. Locally the device
exists, which is why the defect was invisible here. The lane now probes
`OR_LANE_KVM_DEVICE` (defaulting to `/dev/kvm`) and the checks point it at a
temporary readable, writable file, so they exercise control flow on a machine
with no accelerator. This is the honest reason the local suite was not evidence
of a green gate: only the hosted job proved it.
