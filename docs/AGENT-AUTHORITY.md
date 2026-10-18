# Agent authority on this host

Last reviewed: 2026-10-18. Source: the owner's direct instructions in the AEH
sessions of 2026-10-17 and 2026-10-18. This file exists so a later agent does not
have to ask again. It grants no host privilege, no credential beyond what is
named here, and it does not override the evidence rules in `AGENTS.md`,
`docs/TESTING.md` or `docs/UX_ACCEPTANCE.md`.

## Granted without further approval

- Cohesive commits on the working branch, and pushes to `origin`
  (`huou07/Opencut-Reinforced`). Prefer one reviewed commit per coherent change.
- Opening and updating pull requests, and letting required CI decide.
- Running the local lanes on this host: `bash scripts/check-repo.sh`,
  `bash scripts/acceptance.sh`, `cargo test`, `flutter test` in
  `apps/or_app/`, `python3 scripts/run_linux_ui_preflight.py`, and
  `python3 scripts/bench_quality.py`.
- Running the **local Android emulator lane** on this host: creating/starting an
  AVD, building and installing the debug APK, driving the integration tests
  against the guest, and stopping what you started. Its single entry point is
  `bash scripts/run_local_lane.sh android`, which delegates to
  `scripts/run_android_lane.sh`. The owner's 2026-10-18
  instruction is the source of this grant. It produces **developer evidence
  only**: the hosted `platform-verification` Android job remains the acceptance
  authority, and a local guest result never becomes `ACCEPTED`.
- Measuring real GPU behaviour locally through `or_render`/wgpu on this host's
  Intel GPU, and publishing those numbers as a named-host-class budget. Xvfb or
  SwiftShader runs stay software-path evidence and are never reported as GPU
  performance.
- Publishing a stable release only after the release requirements in
  `docs/RELEASE.md` are met for the exact source commit: a successful exact-SHA
  Platform Verification, a tag that resolves to that commit, and non-empty
  checksummed assets. That document defines no separate version, health or
  rollback gate, and none is claimed here; a green gate never grants
  publication by itself.
- Cleaning up leftovers from the 2026-10 migration: stale paths, orphaned
  untracked files and superseded tool references may be committed or removed
  when the current source supports it; record what changed and why.
- Installing project-local build tooling and using the owner's recovered shared
  toolchains (`~/.local/share/flutter`, `~/.rustup`, `~/.cargo/bin`,
  `~/.local/share/playwright-cli`).
- **Two host ACL grants on the shared Flutter SDK**, authorized by the owner on
  2026-10-18 as the resolution of the Android lane's blocker (offer "A" of the
  2026-10-18 authority question). Exactly these two paths are granted write
  access to the worker UID, and nothing else in the SDK was touched:

  ```sh
  sudo setfacl -m   u:dsh-worker:rwx /home/huou07/.local/share/flutter/packages/flutter_tools/gradle
  sudo setfacl -R -m u:dsh-worker:rwX /home/huou07/.local/share/flutter/packages/integration_test/android
  ```

  Flutter's Gradle build configures both as included projects and must create a
  project cache inside them. Reverse either with
  `sudo setfacl -m u:dsh-worker:r-x <path>` (add `-R` for the second). This is a
  narrow, dated exception to the "Not granted" rule below, not a general
  permission: do not widen it to the rest of the SDK, and when the build asks for
  a further path, record the path with the evidence that it asked before granting
  it.

## Not granted

- Host `sudo`, host account/group changes, host ACL changes, or production
  service restarts outside this project's own release procedure.
- Reading or copying owner credentials. Only the exact GitHub CLI files the
  owner approved are readable, and only for publishing this repository.
- Force-pushing `main`, rewriting tags, or discarding unrelated work.
- Claiming Android, macOS or Windows **acceptance** evidence from this host.
  Local Android results are developer evidence; the hosted Android job decides.
- Running two writers in this working tree at once. The Android emulator, Gradle
  daemons and the Flutter preflight are shared host resources: serialise them and
  use a dedicated AVD/serial so another project's guest is not disturbed.

## Verified facts about this host (do not re-derive)

| Fact | Value |
|---|---|
| Host | Dell OptiPlex Micro 7010, i5-13500T (20 hardware threads), 15 GiB RAM, `systemd-detect-virt` = none (bare metal) |
| Flutter | 3.47.7 at `~/.local/share/flutter`; CI pins 3.47.5, so a local pass is developer evidence, not exact-CI evidence. **The SDK is read-only for worker UIDs** (`r-x` under
`/home/huou07/.local/share/flutter`, owned by the host user), except the two
paths granted write access on 2026-10-18 under "Granted without further
approval" above. Gradle 9.3.1 must create a project cache inside its included
builds, so before that grant any Gradle-based Android build failed with the
misleading `The settings are not yet available for build.`; `scripts/run_local_lane.sh
android` probes the writability and reports BLOCKED with the path instead. With
both grants applied on 2026-10-18 the lane's `preflight` step PASSes and the APK
build reaches Gradle `assembleDebug`. Measured on 2026-10-18: the same directory
copied to a writable location builds, the shared one did not. Measured on 2026-10-18: the same directory
copied to a writable location builds, the shared one does not. `cd apps/or_app && flutter test --no-pub` **passes locally** (124 tests, 2026-10-18); the older claim that a raw `flutter test` fails on macOS-target resolution is no longer current |
| Rust | `1.98.0` is installed and is what `crates/or_app_bridge/rust-toolchain.toml` declares; `stable` is also 1.98.0 |
| `~/.cargo` | **not writable** by the worker. Builds use the preflight's project-local offline cache; adding a new dependency needs a writable `CARGO_HOME` |
| GPU | Intel UHD 770 (ADL-S GT1), Vulkan 1.4.305 / Mesa ANV 25.0.7, `/dev/dri/renderD128` readable and writable; llvmpipe is also present, so a wgpu backend choice must be explicit in any performance claim |
| Linux UI preflight | `python3 scripts/run_linux_ui_preflight.py` is the supported entry point. A raw `flutter test -d linux` fails because rustup then tries to install the macOS targets declared in the bridge manifest; the preflight narrows that list to the host target and restores it byte-for-byte |
| Shared Flutter hook cache | `apps/or_app/.dart_tool/hooks_runner/shared/**` is shared between worker UIDs. Artifacts built by another worker are unreadable and cargo fails with `failed to link or copy … build-script-build`; delete that cache so it is rebuilt by the current UID |
| Android | **available.** SDK at `~/Android/Sdk` (platform-tools, emulator, NDK 26.3/27.2/28.2, `system-images;android-36;google_apis;x86_64`); AVD `an3-api36`; `/dev/kvm` readable+writable and `emulator -accel-check` reports "KVM (version 12) is installed and usable". A local x86_64 debug APK with NDK r28c FFmpeg libraries was built and installed on 2026-10-18. The staging root and commands are now recorded: `scripts/stage_android_ffmpeg.sh` builds the pinned x86_64 prefix and the guest link probe, and `scripts/build_android_case_apks.sh` builds the per-case APKs. On 2026-10-18 the prefix was then rebuilt with those scripts at `~/Android/or-android-ffmpeg-8.1.3/install/x86_64` with its probe at `~/Android/or-android-ffmpeg-8.1.3/probes/x86_64/ffmpeg-link-probe`; no guest drive has recorded a result yet |
| GitHub | `gh` is authenticated as `huou07` through the owner-approved file exception; `git push` and release publication work |
| FFmpeg 8 local prefix | `~/.local/ffmpeg8/{ffmpeg,ffprobe}` is present, so the three media lanes in `scripts/acceptance.sh` run instead of reporting `UNVERIFIED` |
| Host headroom is contended | Measured 2026-10-18: `MemAvailable` 5.4 GiB against the Linux preflight's 6 GiB floor and the Android lane's ~7.7 GiB need. The `GradleDaemon 8.14.3` holding ~2.25 GiB belongs to the sibling project **VCE** (`~/Projects/VCE/native-offline/src-tauri/gen/android`), not to OR. Attribute any java daemon before touching it (`pgrep -af GradleDaemon`, `readlink /proc/<pid>/cwd`, sample `/proc/<pid>/stat` over a few seconds) and never stop another project's active build. When headroom is short, the honest lane result is `BLOCKED` with the probe output; do the phase that needs no runtime lane and return when the host is free |
| Android guest RAM against the lane floor | The lane's 7 GiB `MemAvailable` floor and `OR_ANDROID_GUEST_RAM_MB` default of 6144 cannot both hold on a 15 GiB host with no daemon resident: measured `MemAvailable` 6.19–7.27 GiB on 2026-10-18, and the first run of the repaired lane was BLOCKED at the floor with no daemons left to reclaim. A run on 2026-10-18 therefore used `OR_ANDROID_GUEST_RAM_MB=3072 OR_ANDROID_LANE_MIN_MEM_GIB=5`, which is an operator override, not a new default. Deriving the floor from the guest size plus a margin, and stopping the lane's own Gradle/Kotlin daemons before the `emulator` step, are open lane defects |
| Android APK build hangs after the SDK grant | Measured 2026-10-18 21:08–21:45 UTC+07: with both SDK paths writable, `scripts/build_android_case_apks.sh` reached Gradle 9.3.1 `assembleDebug` for the first case (`integration_test/native_bridge_diagnostics_test.dart`) and then hung. Evidence: no file written under `apps/or_app/build` or `apps/or_app/android` for over five minutes, 297 of 302 daemon threads in `futex_do_wait`, 98% idle CPU, zero swap in/out, and sockets to Google/Cloudflare stuck in `CLOSE-WAIT`. It was stopped by hand and its daemon reaped with `./gradlew --stop`. An interrupted or stalled dependency fetch is the leading hypothesis and is **not confirmed**; no APK was produced, so this is not a product defect and no guest result exists |

## Evidence discipline

Local runs are developer evidence: they prove source behaviour on this host for
this tree, not packaged acceptance. The authoritative gate is the hosted CI run
for the exact commit SHA. Every claim keeps its class — VERIFIED_SOURCE,
VERIFIED_BEHAVIORAL, VERIFIED_SEMANTIC, VERIFIED_NATIVE — and no class promotes
another automatically. A local pass never substitutes for a skipped hosted job.
