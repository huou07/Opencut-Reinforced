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
| `9676ef3` | the Windows stager works on a real host, not only in CI |
| `c64a128d` | the DMG is built and opened on all three hosts' evidence in one place |
| `018b5ab` | scene shapes carry a corner radius and a two-stop gradient |
| PT2 | rooted scene asset paths are refused on Windows too |
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

## MotionScene shapes, schema version 3, same commit

| Claim | Evidence |
|---|---|
| A rectangle draws rounded corners | `cargo test -p or_render --lib a_rounded_rectangle_leaves_its_corner_empty` — pixel (11,11) of a 40×40 card at (10,10) with radius 12 has alpha 0, the centre is opaque, ink stays inside the box |
| A gradient varies across the box | `a_gradient_changes_colour_across_the_box` — 0° black→white over 40 px gives red < 40 at x=0 and > 215 at x=39 |
| Version 1 and 2 documents still parse and render identically | `cargo test -p or_motion --lib version_one_document_still_resolves_with_the_classic_look`; every new field defaults to the old behaviour |
| Bad shape style is refused with a location | `an_out_of_range_corner_radius_is_reported_with_a_location` (`MOTION_INVALID_CORNER_RADIUS`, `/nodes/0/corner_radius`), `an_unknown_gradient_key_is_refused` (`InvalidJson`) |
| Only shapes can be rounded | `corner_radius_edits_shapes_and_refuses_text` — `MOTION_EDIT_INVALID_VALUE` for −1, `MOTION_EDIT_NOT_SHAPE` for a text node |
| The inspector steps the radius | `flutter test test/motion_library_test.dart` — `the inspector rounds a shape without a bridge regeneration` |
| The scene journey still inserts a motion clip | `python3 scripts/or_ui.py run scripts/ui_scenarios/motion-scene-edit.json` → `RESULT: OR_UI=PASS`, 63 steps, ends "Motion clip added to the timeline." |
| Local gates | `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, or_core 376 / or_motion 99 / or_render 45, `flutter analyze --no-pub`, `flutter test` **499 pass**, `check-repo.sh` OK (875 files), doc links PASS (159), doc freshness PASS |

Running the Rust suites on Windows found a real defect rather than passing
quietly: `scene_images_resolve_relative_to_the_scene_and_refuse_escape` failed
because `Path::new("/etc/passwd").is_absolute()` is **false** on Windows (a
rooted path without a drive prefix is not absolute there), so a scene could name
`\\Users\me\pic.png` and the loader would join it to `\Users\me\pic.png`,
outside the scene's own folder. Windows PT2 fixed the refusal with
`relative.has_root()` and the test now passes on both platforms
(`cargo test -p or_render --lib` 45 pass on macOS and Windows).

Not verified for this increment: a gradient chosen in the app's inspector (a
scene file or the agent sets it), and any materialised render of a rounded,
gradient-filled card beyond the rasteriser tests above.

## Three hosts at `9938dd4`, the shape-style increment

| Host | Static | Widget suite | Rust suites | Journey |
|---|---|---|---|---|
| macOS 15 arm64 | `flutter analyze --no-pub` no issues | `flutter test` **499 pass** | `or_motion` 99, `or_render` 45, `or_core` 376 | `motion-scene-edit` `RESULT: OR_UI=PASS`; `scripts/acceptance.sh` `PROJECT_ACCEPTANCE=PASS with 10 UNVERIFIED lane(s)` |
| Debian 13 x86_64 | `flutter analyze --no-pub` no issues | `flutter test` **494 pass, 5 skipped** | `or_motion` 99, `or_render` 45 | `scripts/run_linux_ui_preflight.py` under Xvfb |
| Windows 11 x86_64 | `flutter analyze --no-pub` no issues | `flutter test` **494 pass, 5 skipped** | `or_render` 45 (after the rooted-path fix), `or_motion` 99 | native build is hosted-CI authority |

All three checked out the same tree from one `git bundle`; Windows needed
`LIBCLANG_PATH` and `C:\msys64\clang64\bin` on `PATH` for the FRB native-assets
build, which is an environment note, not a code change.

## The Linux package, built and installed on the Debian host, `a9b30ed`

| Claim | Evidence |
|---|---|
| The pinned inputs build in user space | `scripts/build_desktop_export_codecs.sh linux …` (Opus 1.6.1, libvpx `v1.17.0`, x264, nv-codec-headers `n12.2.72.0`) plus FFmpeg 8.1.3 configured with `scripts/ffmpeg_configure_args.py`; `scripts/verify_ffmpeg_helpers.py` then encoded a proxy (mpeg4), a thumbnail, a waveform and a VP9+Opus WebM and decoded it back |
| The lane builds a `.deb` | `bash scripts/run_local_lane.sh linux --packaged --steps deb,journey` → `deb PASS 86.3 s`; `opencut-reinforced-linux-x64-debug.deb` 33,882,200 B, SHA-256 `bd7391fd7bedc74986935c6c0902fb70ebf621404c8ffc7bbdc0088b783da055` |
| The package installs, launches and exports | `journey PASS 317.2 s`; the packaged product journey wrote `existing-export.webm` (13,105,798 B) and the pinned `ffprobe` reports `vp9 1920×1080 24 fps` + `opus`, `duration=30.008` |
| Lane result | `RESULT: LOCAL_LANE=PASS`, `elapsed_seconds 403.64` |

Two real defects surfaced only by running the lane: the exported
`packaged_build_deb` function used the unexported `$ROOT` (now `$root`), which
failed the step with `ROOT: unbound variable` after the bundle was already
built, and the first run had no pinned inputs at all.

## The Windows release staging failure, fixed at `34ed9f3`

The `v0.1.16` tag run built and verified macOS and Linux, then failed on
Windows in `scripts/stage_windows_bundle.ps1:109` with `Python 3 is required to
stage the runtime; set OR_PYTHON to its executable` — although
`%LOCALAPPDATA%\Programs\Python\Python312\python.exe` existed. The candidate
probe captured output with `2>&1` and tested it with `-notmatch`; a successful
silent run leaves that variable `$null`, a comparison against `$null` is not a
boolean, so every candidate was rejected.

| Claim | Evidence |
|---|---|
| The discovery works without `OR_PYTHON` | On the Windows 11 host the extracted block prints `FOUND: C:\Users\hp\AppData\Local\Programs\Python\Python312\python.exe` (it printed `NONE` before the fix) |
| The stage step works without `OR_PYTHON` | `pwsh -NoProfile -File scripts\run_windows_lane.ps1 -Mode release -Steps stage,zip -RepoRoot C:\orb\wt-verify` → `=== OK stage packaged runtime`, `=== OK desktop ZIP install, replacement and removal`, `WINDOWS_LANE=PASS`, `SHA256=d88bd50b603b311046a43065fc5e0e7b6021baa578b63c6dd4dbb26bcf976a2c` |
| CI no longer depends on the runner image | the Windows release job gained `actions/setup-python` 3.12, so the staging step always has an interpreter |

The `v0.1.16` tag was re-pointed at the merge of this fix, because the run it
tagged had published no release and the same version number is the one in
`CHANGELOG.md`, `Cargo.toml` and `apps/or_app/pubspec.yaml`. A corrective tag
move is recorded here instead of being left to be discovered later.

## Release `v0.1.16` published and opened from every asset

| Claim | Evidence |
|---|---|
| One tag run produced all four packages | `Desktop release` run 42051101193 on `6ee16160`: Ubuntu, Apple Silicon and Windows jobs each success, `Publish every desktop asset` success |
| The recorded checksums match the published bytes | `gh release download v0.1.16` then `shasum -a 256 -c SHA256SUMS.txt` → four `OK` lines |
| The macOS image opens | mounted read-only: `Contents/MacOS/or version` → `Opencut Reinforced 0.1.16`, `whisper-cli --help` prints its usage, `file` reports `Mach-O 64-bit executable arm64` |
| The Debian package installs and runs | `sudo dpkg -i Opencut-Reinforced-0.1.16-linux-x64.deb` → `Setting up opencut-reinforced (0.1.16+6ee161607b4d)`; `/opt/opencut-reinforced/or version` → `Opencut Reinforced 0.1.16` |
| The Windows archive runs | expanded with `Expand-Archive`: `Get-FileHash` matches `25118f97…`, `or.exe version` → `Opencut Reinforced 0.1.16`, `ffmpeg.exe -version` → `ffmpeg version 8.1.3` |

The `v0.1.16` tag was moved once, from `d713d55d` to `6ee16160`, because the
first tag run failed on Windows while staging the runtime and published
nothing; the fix and the decision are recorded above. No other tag was moved,
deleted or re-created, and no version was published twice.

## Limits

- Windows: Flutter 3.47.5, rustup 1.98.0, FFmpeg 8.1.3 and the MSVC toolchain do
  exist on that host, so the Windows analyse/test/build rows above were measured
  there. What stays unmeasured is the `.setup.exe` (Inno Setup 6 is not
  installed) and the optional speech runtime inside the package.
- The parity lane renders a 320×180 composition. It compares the two writer
  paths on the same snapshot; it does not sweep every resolution or every
  adapter.
- `ExportCapability.assumed()` still exists for the case where the bridge cannot
  answer. It says "platform default", and no test can prove what a build that is
  not loaded would write.
- The CLI has no `capabilities` verb yet; the measurement is reachable through
  the app and through the Rust API only.

## The optional speech runtime, installed and removed through the GUI, `0c7f269`

| Claim | Evidence |
|---|---|
| The install control is only offered when nothing is installed | `scripts/ui_scenarios/speech-runtime-install.json` step 4 (`expect tools-speech-install`) on the Debian host, in a build whose engine folder had just been deleted |
| A real download, checksum and unpack happen from the running app | the journey tapped `tools-speech-install` on Debian and `~/.local/share/opencut-reinforced/speech-runtime/` held 37 files 7 s later, with `.or-speech-install.json` listing every one |
| The same bytes pass the pinned check on every host | `python3 scripts/verify_pinned_assets.py` → `PINNED_ASSETS=PASS (4 asset(s))` on macOS, `--only ubuntu-x64` `PASS` on Debian, all four `PASS` on Windows from `0c7f269` |
| Removal is offered for a recorded copy even when the adapter is missing | the journey saw `Remove copy` in a dev build with no `or-whisper-provider` beside the binary; the panel now keys the control off the install record, not off "both halves present" |
| Removal deletes exactly the manifest and nothing else | after the confirmed removal the folder held `0` entries and the snackbar read "Removed 37 files the setup installed." |
| The control returns to "Install" once the folder is empty | journey steps 15–16 (`wait_label_absent "Remove copy"`, `expect tools-speech-install`), `RESULT: OR_UI=PASS` (18 steps) |
| A tampered archive installs nothing | `apps/or_app/test/speech_runtime_install_test.dart` "a tampered download is refused and installs nothing": checksum `f…f` → `GithubReleaseException` "checksum does not match", empty target, no manifest |
| The installer path itself is covered without the network | the same test file drives the real download/verify/unpack code over a canned `HttpClient` and asserts one request, the installed files, the manifest and a clean removal |

The panel defect this found is recorded in `docs/reference/roadmap.md` item 25:
the remove control previously required both halves of the runtime, so a host
that installed the engine without the adapter offered only a second download.

## The Windows installer, exercised over a running app, 2026-11-06

The published 0.1.16 setup had never been installed on a Windows host, so it was
built again from the branch and driven through the three transitions a user
actually performs. The first attempt failed in a way no test had covered: over an
existing install the silent upgrade retried `DeleteFile` on a mapped
`avcodec-62.dll`, then reported `User canceled the installation process` and
exited with code 5, having done nothing.

| Evidence | Result |
| --- | --- |
| The failure, from the installer's own log | `C:\orb\tmp\setup.log`: `DeleteFile: The existing file appears to be in use (5). Retrying.` ×4, then `Defaulting to Abort for suppressed message box (Abort/Retry/Ignore)` → exit 5, `Rolling back changes` |
| The lock came from the app itself | `Get-Process \| Where-Object { $_.Path -like '*Opencut Reinforced*' }` on the host listed `or_app.exe` running from `%LOCALAPPDATA%\Programs\Opencut Reinforced` |
| A fresh install succeeds | `start /wait setup.exe /VERYSILENT …` → `EXIT1=0`, `or.exe version` = `Opencut Reinforced 0.1.16` |
| An upgrade over the running app now succeeds | app started, then the same setup → `EXIT2=0`, version still 0.1.16, and `app-closed-by-setup` (the installer stopped it) |
| The uninstall removes the tree | `unins000.exe /VERYSILENT` → `EXIT3=0`, `removed` |
| The staged CLI cannot silently disagree with the package | `stage_windows_bundle.ps1` now compares `or.exe version` with `Cargo.toml`; before the guard, a package named 0.1.16 shipped an `or.exe` that printed 0.1.15 because the lane reused a stale `target/release` |

Both defects are recorded as `docs/reference/roadmap.md` items 26 and 27. The
installer change is `scripts/windows_installer.iss` `PrepareToInstall` plus
`RestartApplications=no`; the staging change is the release build of `or_cli` and
`or_whisper_provider` in `scripts/run_windows_lane.ps1` and the version guard in
`scripts/stage_windows_bundle.ps1`.

## The performance budget on three hosts, 2026-11-06

`scripts/bench_quality.py` reached a verdict on macOS, Debian and Windows from
the same commit for the first time. Getting there exposed four harness defects
and one lane that was measuring the wrong thing; the details are in
`docs/reference/quality.md` under "Performance budget on all three hosts".

| Evidence | Result |
| --- | --- |
| macOS, branch `4ee97f0` | `PERFORMANCE_BUDGET=PASS`; `queue-priority` ratio 1.08 with 16 jobs on 4 workers, `motion-rasterize-frame` p95 66.6 ms, `render-frame-time` p95 1.71 ms on Metal |
| Debian 13, same commit | `PERFORMANCE_BUDGET=PASS`; ratio 1.27, `motion-rasterize-frame` p95 60.5 ms, `command.import` p95 40.7 ms |
| Windows 11, same commit | every row has a verdict: ratio 1.39 with the re-modelled lane against 29.69-33.77 before it, `render-frame-time` p95 1.87 ms on Intel Graphics/Vulkan |
| The lane was measuring an operating system | it spun two threads per logical core while the product pool is four workers; `or_core::artifact_worker_count` now owns the policy and the lane saturates that pool |
| A clean machine could not run `export-long` | the scenario imported `/tmp/long720.mp4`, which nothing generated; it now imports the checked-in generated fixture and the run measures `export-720p-5s` p50 3436 ms |
| A stale label kept `track-reorder` red | the track lane says `active`, not `enabled`; the scenario now matches the wording the app produces |
| Windows single-threaded raster is a clock class, not a regression | firmware `MaxClockSpeed 1400` MHz with no boost, PNG encode 400 ms against 105 ms on the M5; the row carries a Windows baseline (p95 317 ms) and a 450 ms Windows ceiling, Debian stays 250 ms |
| Windows save cost is host class too | 256 KiB flush-and-rename probe on the same volume: p50 4.0 ms, p95 20.7 ms, on top of a 1400 MHz CPU; the row records a Windows baseline (p95 102 ms) and a 160 ms ceiling, Debian stays 80 ms |
| `command.*` on Windows | verified once a general-purpose FFmpeg was installed: `command.import` p95 36.2/33.9 ms, `caption_import` 37.0 ms, `audio_cleanup` 239.6 ms, all inside their budgets |

The worker-priority experiment that preceded the lane fix is not in the tree: it
measured 33.77 against 29.69 before it, so it was dropped rather than shipped.

## The assistant's saved path, and a scenario a harness can judge

The assistant panel said "The or CLI was not found" on a machine whose CLI was
present and built. The saved settings named a path from an earlier packaged
build — `/Applications/Opencut Reinforced.app/Contents/MacOS/or` — and
`resolveOrCliPath` returned that path's resolution, which was `null`, instead of
falling through to the detection a fresh install uses. Every later rung (the run
harness's `OR_CLI_PATH`, the walk up the checkout, the CLI beside the executable)
was unreachable.

| Evidence | Result |
| --- | --- |
| Reproduction | the panel's own copy and the semantics tree: "The or CLI was not found, so the assistant cannot edit the project." while `./target/debug/or version` printed `Opencut Reinforced 0.1.16` |
| Fix | a stored path that does not resolve now falls through; `apps/or_app/test/assistant_settings_test.dart` covers both a stale saved path and a stale override |
| Run harness | `scripts/or_ui.py` exports `OR_CLI_PATH` for the build it launches, so the debug app reaches its own CLI |
| Deterministic assistant scenario | `assistant-panel` (23 steps) connects the panel, opens settings, sends a turn and passes; `assistant-edit` keeps the live-provider half, which a harness cannot judge |
