# Preview/export parity, editable text metrics and measured export formats — 2026-11-05

Status: **local macOS evidence** for branch `mission/parity-and-hardening`, plus
one Linux run at the same commit for the Rust and Flutter lanes. Windows stays
unverified. The limits at the end are part of the record.

## Commits

| Commit | Increment |
|---|---|
| `da93fe9` | preview and export are compared pixel by pixel in a Rust lane |
| `04c6d7e` | the lane prints the adapter it measured |
| `7c0cff4` | both adapters recorded in the quality table |
| `31fdb77` | the text dialog sets line height and tracking |
| `4fe75d2` | export formats are measured by the engine, not assumed by the UI |
| `1d56d36` | setup reports the machine the render will happen on |
| `9ef7027` | the first-run panel states the measured machine |
| `62d770e` | optional components can be removed, but only what the app installed |
| `3d43c9c` | the library shows the licence and origin of every bundled asset |
| `ea8cb31` | Windows 11 builds, tests and packages the ZIP from the same tree |
| branch head after this receipt | `ea8cb31`, macOS, Linux and Windows verified from that commit |

## What was proven, and how

| Claim | Evidence |
|---|---|
| Preview and export produce the same picture | `cargo test -p or_render --test preview_export_parity` → **62.60 dB** on Apple M5 (Metal) and **62.60 dB** on Debian's Intel UHD 770 (Mesa); mutation of the limited-range decode to full range drops it to **23.96 dB** and fails |
| Line height and tracking reach the project and the renderer | `flutter test test/widget_test.dart --plain-name "line height and tracking"` → passes; `ProjectTextFormatting` defaults 1320/0 keep old projects byte-identical |
| The text dialog is usable in the real app | `python3 scripts/or_ui.py run scripts/ui_scenarios/text-typography.json` → `RESULT: OR_UI=PASS` |
| The Export dialog offers only formats this build can write, and says why when one is missing | `flutter test test/widget_test.dart --plain-name "without an H.264 encoder"` → passes: the MP4 tile is absent, `export-capability-caveat` is present, `Continue` still sends WebM |
| The Settings panel reports the encoder this build would use | `python3 scripts/or_ui.py run scripts/ui_scenarios/resources-setup.json` → `RESULT: OR_UI=PASS`, 18 steps; the tree artifact holds "This build exports MP4 (VideoToolbox, hardware), WebM (VP9), Lossless (FFV1)." |
| The capability comes from the engine, not a Dart list | `cargo test -p or_media --lib` → 22 pass, including `the_described_profiles_match_the_encoders_this_build_links` |
| Formatting is honest when no encoder is present | `flutter test test/export_capability_test.dart` → 7 pass (hardware, software, no-H.264, lossless-only, none, assumption, fixed cache) |
| Setup reports the machine it will render on | `flutter test test/system_check_test.dart` → 9 pass (df parse, PowerShell parse, refusing host reports unknown, small-disk warning); the `resources-setup` journey reads back **"macOS Version 27.2 (Build 26B123) · 10 cores · 16 GB RAM · 102 GB free"** from the real app |
| The first-run panel states the measured machine, or omits the line | `flutter test test/home_welcome_test.dart` → 3 pass (measured lines, short-disk warning, no measurement); the live journey cannot show it because this profile has already dismissed `welcome.json` |
| Removing an optional component deletes only what the app installed | `flutter test test/speech_runtime_removal_test.dart` → 4 pass (recorded files deleted, a user's model survives, a missing record refuses, a record pointing outside the folder is ignored) and `flutter test test/runtime_setup_panel_test.dart` → 5 pass (confirm dialog, count snackbar, install action returns) |
| Every bundled font and generated asset states its licence in the app | `flutter test test/library_screen_test.dart` → 9 pass, including the guide ending with `library-credits` and the per-asset licence keys; `python3 scripts/or_ui.py run scripts/ui_scenarios/library-tour.json` → `RESULT: OR_UI=PASS` with those keys asserted in the running app |
| Flutter surface stays green | `flutter analyze --no-pub` → no issues; `flutter test` → **498 pass** |

## Windows verification, same commit

The Windows 11 host (`Intel Core 5 120U`, AMD64) already carried Flutter 3.47.5
(`6a19cca564`), rustup `1.98.0`, the VS 2022 Build Tools, msys64 clang64 and an
FFmpeg 8.1.3 prefix at `C:\orb\tmp\ffmpeg-8.1.3-install`; nothing was installed
for this run. The branch was transferred as a git bundle and checked out into a
separate worktree at `ea8cb31`, so the owner's checkout was untouched.

| Step | Command | Result |
|---|---|---|
| environment | `powershell -ExecutionPolicy Bypass -NoProfile -Command ". C:\orb\env.ps1"` | per-process only; the execution policy is not changed |
| dependencies | `flutter pub get` | `Got dependencies!` |
| static | `flutter analyze --no-pub` | `No issues found! (ran in 123.2s)` |
| widget suite | `flutter test --reporter compact` | **493 passed, 5 host-skipped** (same 5 the Linux host skips), exit 0 |
| bridge | `cargo check -p or_app_bridge --locked` | exit 0 after `C:\msys64\clang64\bin` is on `PATH` for libclang |
| release build | `flutter build windows --release` | `or_app.exe` plus the bridge, texture and file-selector DLLs |
| packaging | `pwsh -File scripts/run_windows_lane.ps1 -Mode release -Steps stage,zip` | `WINDOWS_LANE=PASS`, `SHA256=d24a1bd260ea1ea4cbe68604083119c54f6d260c36cd4b3e7fd8b67b7e85cb68` |

The ZIP step is not a checksum: it installs the package into the user install
location, launches it, replaces it with a second copy, uninstalls it and proves
project data survives all three. Two repository defects were found and fixed on
the way, both recorded in [release.md](../reference/release.md): PowerShell 5.1
cannot load `NativeLibrary` (the lane needs `pwsh`), and the staging script both
crashed on a Store-aliased `python3` and silently fell back to the stager's
`whisper,provider` default when no runtime was requested.

Not verified on Windows: the Inno Setup `.setup.exe` (Inno Setup 6 is not
installed on that host) and the speech engine in the package (the local run
stages the app and the `or` CLI without the pinned Whisper archive).

## Packaging on all three hosts, same commit

A package is evidence only when it was built, opened and exercised, so each host
was asked for the artifact its users install.

| Host | Artifact | Result |
|---|---|---|
| macOS 15 arm64 | `dist/Opencut-Reinforced-0.1.15-arm64.dmg` (36 MB, SHA-256 `1f75ff94…`) | built by `scripts/build_macos_dmg.sh`, checksum valid, mounted read-only, `Contents/MacOS/or version` prints `Opencut Reinforced 0.1.15`, `whisper-cli --help` starts, `codesign -dv` reports an ad-hoc arm64 bundle |
| Windows 11 x86_64 | `build/windows/or-windows-x64-release.zip` (35 MB, SHA-256 `d24a1bd2…`) | `WINDOWS_LANE=PASS`: staged, installed to the user location, launched, replaced, uninstalled, project data survived |
| Debian 13 x86_64 | none | `LOCAL_LANE=BLOCKED` before packaging: the pinned FFmpeg 8 prefix, codec prefix, `COPYING.LGPLv2.1` and codec notices are not on that host |

Two blockers were removed rather than reported: the macOS gate rejects any
Mach-O above the public minimum, and the debug bundle carried a `whisper-cli`
built with `minos 26.0`, so whisper.cpp `b5454` was rebuilt with
`-DCMAKE_OSX_DEPLOYMENT_TARGET=13.0` and the provider with
`MACOSX_DEPLOYMENT_TARGET=13.0` (`otool -l` now reports `minos 13.0` for both).
The Linux lane named exactly what it was missing instead of fabricating a
package; building a pinned LGPL FFmpeg 8 and the codec prefix on that host is a
multi-hour compile that this round did not spend.

Not verified: the Inno Setup `.setup.exe` (Inno Setup 6 is absent on the Windows
host) and the speech engine inside the Windows package (the local stage skipped
the optional runtime).

## Limits

- Windows: no Flutter, no FFmpeg and no C toolchain on that host, and
  `docs/operations/permissions.md` forbids installing system tooling. The
  Windows export row therefore remains a claim about the package, not a
  measurement on that machine.
- The parity lane renders a 320×180 composition. It compares the two writer
  paths on the same snapshot; it does not sweep every resolution or every
  adapter.
- `ExportCapability.assumed()` still exists for the case where the bridge cannot
  answer. It says "platform default", and no test can prove what a build that is
  not loaded would write.
- The CLI has no `capabilities` verb yet; the measurement is reachable through
  the app and through the Rust API only.
