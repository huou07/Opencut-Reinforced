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
| branch head after this receipt | export formats are measured, not assumed |

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
| Flutter surface stays green | `flutter analyze --no-pub` → no issues; `flutter test` → **496 pass** |

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
