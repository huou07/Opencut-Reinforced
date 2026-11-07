# Active Product Roadmap

Last reviewed: 2026-11-05

Scope: what the owner selected to build now, what acceptance means, and what is
deferred. Requirements live in [product.md](product.md); capability contracts in
[features/](../features/).

## Current focus (owner restatement, 2026-11-05)

The owner's 2026-11-05 mission turns OR into a professional, beginner-usable
video platform on one shared codebase. The work is organised so each increment is
a product improvement with recorded evidence, not a library addition:

11. **Guided first video (owner request, 2026-11-05).** A beginner's next step is
    always one click away in the timeline toolbar, the export dialog asks what
    the file is for instead of which codec, and composition guides show the safe
    areas. Delivered with `apps/or_app/lib/screens/editor_guided_bar.dart`, the
    intent-first export dialog, `_CompositionGuidesPainter`, and the
    `guided-beginner` journey through the real app. Settings exposes guided mode,
    next-step tips, guides and the inspector as preferences over the same state,
    in their own **Editor** panel rather than trailing the narration settings.
12. **Scene typography (owner request, 2026-11-05).** MotionScene schema v2 adds
    family, weight, alignment, line height and tracking inside an explicit text
    box, with the two metrics shared through `or_core::TextFormatting`.
    Delivered per [ADR 0012](../adr/0012-scene-typography-schema-v2.md). The
    bridge view was widened afterwards, so the project text dialog sets line
    height and tracking too (`timeline-text-line-height`, `timeline-text-tracking`)
    and the `text-typography` journey drives both through the real app.
13. **Measured media capability (owner request, 2026-11-05).** The capability
    claim is now a measurement: `scripts/probe_media_matrix.py` probes and decodes
    18 fixtures per host, import proves a real decode and refuses an unusable
    decoder with `DECODE_UNAVAILABLE`, and
    [media-capability-matrix.md](media-capability-matrix.md) records the rows.
    The AV1 finding on Apple silicon is the reason the gate exists.
14. **Technology adoption review (owner request, 2026-11-05).** Twenty candidate
    repositories were checked against their real licences; no runtime dependency
    was adopted and the reasons are recorded in
    [technology-review.md](technology-review.md).
15. **Linux verification of the same branch (owner request, 2026-11-05).** The
    Debian 13 host built and tested the branch with a user-space Flutter and Rust
    toolchain: `flutter analyze` clean, 462 widget tests, the bridge/IPC/runtime
    Rust lanes, a CLI journey on a real project, and the semantic editor journey
    under bounded Xvfb (`scripts/run_linux_ui_preflight.py`). That run is what
    diagnosed the six failing widget tests and the failing `editor` workflow: the
    guided band took 46 px from the panes. The step is now a toolbar item.
16. **Setup is a GUI surface (owner request, 2026-11-05).** Settings → Resources
    reports each bundled component, keeps speech models and the harness behind
    explicit actions, and exposes the external-tools controls; the
    `resources-setup` journey records that opening it installs nothing.
17. **Windows-sized editing (owner request, 2026-11-05).** The editor stacks
    below 980 px instead of squeezing three panes, and a compact window reaches
    every tool through the dock sheet; the guided action keeps a 26 px target
    instead of the 18 px compact density produced. The macOS suite, the Linux
    suite at the same commit, and the bounded Linux preflight all pass.
18. **The local acceptance gate states its own environment (owner request,
    2026-11-05).** `bash scripts/acceptance.sh` now passes every lane it can
    select on this host (`PROJECT_ACCEPTANCE=PASS with 10 UNVERIFIED lane(s)`):
    the Linux-only preflight is UNVERIFIED off Linux instead of a traceback, the
    two media lanes that synthesise fixtures with `-f lavfi` are UNVERIFIED with
    that reason when the packaged build omits the device, and `command.create`
    carries a measured Darwin baseline beside the Debian ceiling. The media crate
    tests and the export writer run against the pinned FFmpeg 8 through the
    `$HOME/.local/ffmpeg8` shims recorded in
    [local-macos.md](../operations/local-macos.md).
19. **Export formats are measured, not assumed (owner request, 2026-11-05).**
    `or_media::describe_export_capability` reports which profiles this build can
    write and which encoder it would use; the bridge exposes it and the app reads
    it once per launch. The Export dialog lists only formats whose encoder opens,
    defaults to the engine's preferred profile, and warns when MP4 is missing;
    Settings → Resources shows the same Ready/Unavailable list. The `no-h264`
    widget test and the `resources-setup` journey record both halves.

20. **Setup reports the machine it will render on (owner request, 2026-11-05).**
    Settings → Resources opens with a measured **This machine** panel —
    platform, processors, memory and free space on the volume that will hold an
    export — and warns with the consequence when the disk is short of the 4 GB
    staging headroom. `df`/`sysctl`/PowerShell are the only host commands, each
    behind one seam (`readHostFacts`), and a host that refuses reports "unknown"
    rather than a guess. Proven by `flutter test test/system_check_test.dart` and
    the `resources-setup` journey, which reads back
    "macOS Version 27.2 (Build 26B123) · 10 cores · 16 GB RAM · 102 GB free".

21. **Removing an optional component is as explicit as installing it (owner
    request, 2026-11-05).** Installing records exactly which files the app wrote;
    removal reads that record and deletes only those entries, refusing paths that
    leave the folder, so a model the user downloaded and a runtime they copied by
    hand both survive. The action is offered only when the record exists, which
    keeps a bundled engine from looking removable.
22. **Every bundled asset states its licence in the app (owner request,
    2026-11-05).** Library → Guide ends with the credits for both bundled font
    families and every generated asset: licence, what it permits, and where it
    came from. The Text and Sound tabs name their licence beside the content, so
    nobody has to open the repository to learn what they may do with a font or a
    sound.
23. **Scene shapes stop looking unfinished (owner request, 2026-11-05).** A
    rectangle carries a `corner_radius` and any shape a two-stop `gradient`
    (schema version 3, ADR 0013), so cards, chart panels and lower thirds are
    styled in the scene language instead of being drawn as hard-edged blocks.
    The inspector steps the radius; gradients are authored by the agent or in the
    file. Running the suites on Windows also found and fixed a real escape: a
    rooted asset path such as `\Users\me\pic.png` is not `is_absolute()` there,
    so scene image loading now refuses `has_root()` as well.

24. **0.1.16 is published for all three desktops (owner request, 2026-11-05).**
    One pull request and one tag run produced the macOS `.dmg`, the Linux
    `.deb`, the Windows `.setup.exe` and the portable `.zip`, plus
    `SHA256SUMS.txt`; each asset was then downloaded again, matched its recorded
    checksum, and opened on its own platform. The first tag run failed while
    staging the Windows runtime because the stager's Python probe compared a
    `$null` success against `-notmatch`, so no interpreter looked usable; the fix
    starts each candidate instead of reading its banner, the release job installs
    Python explicitly, and the tag was moved once onto the merged fix. Details in
    [release.md](release.md) and the parity receipt.

Blocked, and reported as blocked rather than attempted:

- **The Windows installer (.setup.exe).** The Windows ZIP package is built,
  installed, replaced, launched and uninstalled on a real Windows 11 host from
  the same commit as macOS and Linux (`WINDOWS_LANE=PASS`, SHA-256
  `d24a1bd2…`, see the parity receipt). The Inno Setup definition is not
  exercised locally because Inno Setup 6 is not installed on that host; the
  `.zip` is the tested artifact. The published 0.1.16 `.setup.exe` was produced
  by the tag run and its checksum verified after download; it has not been
  installed on the local Windows host, so the installer itself stays
  locally-unproven.
- **Installer-level parity on Linux.** Closed on 2026-11-05: the pinned LGPL
  FFmpeg 8.1.3 build and the codec prefix (Opus, libvpx, x264, nv-codec-headers)
  were compiled in user space on the Debian host, and
  `scripts/run_local_lane.sh linux --packaged --steps deb,journey` now reports
  `RESULT: PASS` — the `.deb` (33.9 MB, SHA-256 `bd7391fd…`) was built, installed
  into a local root, launched, and the packaged journey exported a real
  1920×1080 24 fps VP9+Opus WebM that the pinned `ffprobe` decodes. The debug
  package is what was proven; a `--build-mode release` `.deb` is still produced
  only by the release workflow.
- **An optional download installed and removed through the GUI.** Closed on
  2026-11-05 at `0c7f269`: the `speech-runtime-install` journey
  (`RESULT: OR_UI=PASS`, 18 steps) tapped Install in the running Debian build,
  which downloaded the pinned 10.4 MB archive, checked its SHA-256, unpacked 37
  files and wrote the install manifest; the same journey then removed the copy
  after confirmation and saw the Install control return. The pinned archives are
  re-checked by `scripts/verify_pinned_assets.py` on macOS, Debian and Windows,
  and the tampered-checksum case fails a widget test instead of a user's
  install. macOS ships the engine with the app, so it has no Install control to
  exercise — that is the packaging decision, not an untested path. What remains
  unmeasured on Windows is only the `.setup.exe` itself (Inno Setup 6 is not
  installed on that host).

25. **A removable optional component keyed off the wrong state** — found by the
    Debian journey on 2026-11-05: the remove control required both the engine
    and the adapter, so a build that installed only the engine offered a second
    download and no way back. Fixed by keying the control off the install
    record and hiding Install once an engine is present.
26. **The Windows installer could not update a running app** — found on 2026-11-05
    while proving the `.setup.exe`: over an existing 0.1.13 install the silent
    upgrade retried `DeleteFile` on a mapped `avcodec-62.dll`, then aborted with
    exit code 5 and no message at all, so a user with the app open would see the
    installer fail without a reason. `PrepareToInstall` now stops
    `or_app.exe`/`or.exe`/`or-whisper-provider.exe` first and
    `RestartApplications=no` stops a silent update from relaunching the app; a
    fresh install, an upgrade over a running app and the uninstall were re-run on
    the Windows host (exit 0 each, `or version` 0.1.16).
27. **A staged runtime could carry a stale CLI** — found the same day: a package
    named for 0.1.16 shipped an `or.exe` reporting 0.1.15 because the local lane
    staged whatever was in `target/release`. The lane now builds `or_cli` and
    `or_whisper_provider` before staging, and staging refuses a staged `or.exe`
    whose version disagrees with `Cargo.toml`.

## Previous focus (owner restatement, 2026-10-21)

One near-term delivery mission for the local macOS app, in this order:

1. **Finish ordinary editing.** Complete and dogfood the connected journey:
   project create, common media import, multi-track editing, visible pointer
   trim/move/split/reorder, image/audio, speed/volume, snapping, undo/redo,
   composition preview, save/reopen, playable export.
2. **Finish deterministic MotionScene.** Turn source code into a short
   declarative illustration, preview and render it through the existing
   architecture, insert it into the ordinary timeline, and export the combined
   edit.
3. **Assistant panel (owner request, 2026-10-26).** Let the user edit the open
   project and generate MotionScene illustrations by chatting with the local
   DeepSeek Harness. This is an external client over ACP and `or mcp`, not an OR
   provider integration; provider and inference work inside OR stays deferred.
4. **Agent workspace (owner request, 2026-10-27).** Open the assistant in an
   app-installed workspace whose `AGENTS.md` routes the agent to one guide,
   skill or tool per task, instead of the project folder. Delivered 2026-10-27
   ([agent-workspace.md](../features/agent-workspace.md)); dogfooding it against
   real edits is the next check.
5. **Motion asset library (owner request, 2026-10-27).** Turn motion output into
   reusable, parameterised assets — adjustable text, colour and size; automatic
   classification; a browsable library; drag and drop into the timeline or an
   open scene. Delivered 2026-10-27 through P1–P3 core: the document, typed
   parameters, classifier, library, image import/rendering, Motion surface,
   composition preview, timeline drag, bridge and `or mcp` tools
   ([motion-assets.md](../features/motion-assets.md),
   [ADR 0010](../adr/0010-motion-assets-and-reusable-library.md)). A live
   timeline MotionClip remains deferred.
6. **Assistant production workflow (owner request, 2026-10-27).** Let the
   assistant write a script, narrate it offline, derive exact captions, generate
   a background bed, and caption existing audio with the local multilingual
   model. Delivered as the workspace tools `voiceover.mjs` and `music.mjs`, the
   host-owned `or captions transcribe` runtime request, and the reviewed
   `timeline import-captions` command. Delivered 2026-10-27 also as in-app
   Audio-tool dialogs: Generate voiceover… and Generate music…, with selectable
   speech models and −12 dB ducking under a chosen voice track. Follow-up:
   dogfood the Vietnamese path on a real project and measure transcription time
   and caption quality.
7. **Prompt-driven music (owner request, 2026-10-28).** The Generate music dialog
   reads a Vietnamese/English sentence for mood, bpm, length and drum intent and
   generates exactly that bed; the same prompt is reproducible and Variation
   reseeds it. Delivered in-app and in the workspace `music.mjs` (`--prompt`,
   `--variation`), so an agent reads a sentence the same way the dialog does;
   explicit flags still win for an older command line.
8. **Importable, editable motion scenes (owner request, 2026-10-28).** Open an
   `.ormotion.json` authored elsewhere, validate it, adopt its PNGs into the app
   library and edit it on the existing canvas; a saved scene reopens from its
   library row. Delivered with the `motion_scene_read`/`motion_scene_from_asset`
   bridge functions and `scripts/ui_scenarios/motion-scene-import.json`.
9. **Import feedback and existing-project coverage (owner request, 2026-10-28).**
   The top bar reports per-file batch import progress and a completion line for
   multi-file batches. A new journey opens a project saved without a sequence
   frame rate, imports a real clip, and proves the insert creates the missing
   track (`scripts/ui_scenarios/existing-project-import.json`,
   `crates/or_core/tests/fixtures/null-rate-project.orproj`); the harness fixture
   picker now copies `.orproj` fixtures into the scenario directory.
28. **The saturation lane measured an operating system, not OR (2026-11-06).**
    `crates/or_render/tests/queue_priority.rs` spun two threads per logical core
    while the product's artifact pool is bounded at four workers, so the lane
    reported a 29.69-33.77 ratio on a ten-core Windows laptop against 1.24 on
    macOS. The pool policy now lives in `or_core::artifact_worker_count`, the
    bridge delegates to it, and the lane saturates the real pool: 1.08 / 1.27 /
    1.43 on macOS / Debian / Windows with the 6.0 ratio budget unchanged.
29. **A checked-in scenario only ran on the machine that wrote it (2026-11-06).**
    `scripts/ui_scenarios/export-long.json` imported `/tmp/long720.mp4`, which
    nothing generated, so it could not pass on a fresh checkout; it now imports
    `crates/or_media/tests/fixtures/long720_h264_aac.mp4`, generated and
    documented with its exact command and SHA-256. `track-reorder.json` waited
    for the retired "enabled" wording and now matches "active".
30. **The Windows budget crashed instead of measuring (2026-11-06).** Four
    portability defects — unconditional `pgrep`, a reset `PATH` that hid the
    FFmpeg DLLs, `os.access` launching a `.pdb`, and a fixture probe that
    looked for `ffmpeg` without `.exe`. All three hosts now reach a verdict, and
    the Windows single-threaded raster row carries its own recorded baseline
31. **A stale saved path disabled the assistant (2026-11-06).** `resolveOrCliPath`
    returned the stored `orCliPath` even when the file behind it had gone, so a
    path saved by an earlier packaged build left "The or CLI was not found" with
    no way back but the settings dialog. It now falls through to the same
    detection a fresh install uses, and a run harness can name the CLI it built
    (`OR_CLI_PATH`); `assistant-panel` is the part of the assistant a harness can
    judge without a live provider.
32. **The Windows uninstaller could not uninstall a running copy (2026-11-06).**
    `PrepareToInstall` closes the app for an install, but Inno's two-phase
    uninstaller never calls it: a silent uninstall over a running copy exited 1
    and removed nothing. `InitializeUninstall` now runs the same step, and the
    step waits until the image is really released by retrying a rename of
    `or_app.exe` for up to five seconds — stopping the process alone exited 0 with
    every DLL left behind for the next reboot. Inno still defers the folder itself
    to a reboot when the uninstaller stub is the last file in it; the program
    files, the uninstall data and the shortcuts are gone and the user's projects
    are untouched.

33. **The Windows app could only be checked by reading its process (2026-11-06).**
    `scripts/or_ui.py` drove macOS and Debian but had no Windows target, so the
    Windows evidence stopped at widget tests and an installer smoke test that
    only proved the process launched. `--target windows` now runs the same
    scenarios through `flutter test -d windows`, which needed the `flutter.bat`
    entry point (a bare `flutter` never resolves through `subprocess`), the
    FFmpeg prefix on `PATH` (the bridge links `avcodec-62.dll` and friends), and
    a screenshot step that records `shot-*.png.unavailable.txt` because that test
    binding has no raster surface. Behavioural scenarios pass on the real app
    (workspace chrome, the library tour, and the speech-runtime install and
    removal); scenarios that insert a clip stop at the engine's "No Impeller
    context is available", which is a property of that headless test surface, not
    a verdict on the packaged app — recorded as a limit, not a pass.
34. **The machine panel could spin forever (2026-11-06).** `readHostFacts` ran its
    memory probe and its disk probe one after the other, and on Windows each is a
    PowerShell launch, so the panel showed a spinner while two processes started.
    The probes now run together, and a failure renders "This machine could not be
    read: …" instead of an endless spinner. A timer-based budget was written
    first and removed: a `Future.timeout` leaves a pending timer that fails every
    widget test that disposes its tree while the probe is in flight.
35. **A batch import had no way to stop (2026-11-06).** A large folder import
    was all-or-nothing: the shell locked every other action and the top bar only
    counted files, so the only exit was to wait or kill the app. The bar now
    offers `cancel-import`; the file in flight finishes, the rest are skipped,
    and the closing line says how many landed and how many did not. Adding the
    button also exposed a 168-pixel bar overflow at 1280 wide, fixed by letting
    the transient line ellipsise.
36. **Scene motion only knew four easings (2026-11-06).** Every animation and
    camera move interpolated with a quadratic, so entrances could not lead fast
    and settles could not overshoot. Schema version 4 adds an optional cubic
    Bézier `easing_curve` on animations and on the camera (ADR 0014); the
    inspector has no graph editor yet, so the curve is authored in the file or by
    the agent.
37. **0.1.20 is published (2026-11-06).** One pull request, one green CI run, one
    tag run; the DMG, the `.deb`, the Windows `.setup.exe` and the `.zip` were
    downloaded again, matched their checksums and were opened from those bytes —
    including an in-place Windows upgrade over a running 0.1.19.
38. **The key map was undiscoverable (2026-11-06).** Forty bindings lived only in
    the shell's shortcut map. The `?` sheet lists them all and is checked against
    that map by a test, so a binding cannot ship unlisted; `?` had to bind slash
    plus shift, because the logical key never becomes `question`.
39. **Hardware profiles and an execution plan (2026-11-07).** Export used to walk
    a hardware-first encoder list with no user control and no record of why a
    candidate was skipped. The Rust core now carries a measured report (which
    H.264 encoders actually open, which decoders exist, the GPU adapter the
    preview device uses), four profiles (recommended, maximum performance,
    balanced, compatibility) with a recorded fallback, and resolves the plan once
    per export job. Settings gains a Performance section, the export dialog names
    the encoder the file will use, the CLI carries `--performance`/`--encoder`,
    and the finished job reports the encoder it opened. Verified with semantic
    journeys on macOS, Debian (Weston headless on the UHD 770) and Windows 11
    (interactive session); the Windows lane found a duplicate-key crash in the
    panel that macOS and Debian never built. Details in
    [hardware-profiles.md](hardware-profiles.md).
40. **Clips can animate (2026-11-07).** Transform channels and opacity are
    keyframable per clip with linear and three cubic easings, stored in canonical
    milli-units on the visual settings and sampled in the shared frame loop, so
    preview and export agree. The inspector adds, edits, removes and clears
    keyframes at the playhead. Verified on all three hosts by the `keyframes`
    scenario. See [effects-audio.md](../features/effects-audio.md).
41. **The UI harness could not reach a section below the fold (2026-11-07).** Long
    inspectors build rows lazily, so one large `scroll` parked the list at the
    bottom and `ensureVisible` could not find a row that was never built. The
    harness gained `scroll_until`, and scenarios run from `apps/or_app` now
    resolve `copy:` fixtures from the repository root instead of silently falling
    back to the generated WAV.
42. **2026-11-07 export rows show no regression (2026-11-07).** At `4cf49782` the
    M5 recorded MP4 239.6/245.6 fps and MP4 with one text layer 240.4/234.7 fps,
    matching the 2026-11-06 rows within spread; a clip with no keyframes skips
    sampling entirely. Animated rows were measured later (item 45). Rows in
    [export-performance.md](export-performance.md).

43. **Animation reaches the timeline and the preset row (2026-11-07).** An
    animated clip shows a diamond badge and its semantics label says "animated";
    Fade in / Slide up / Zoom in / Pop write ordinary keyframes; Copy/Paste
    moves one clip's animation to another. Verified on macOS, Debian (Weston on
    the Intel UHD 770) and Windows in an interactive session with the
    `keyframes` journey.

44. **Composited exports convert NV12 in one compute pass (2026-11-07).** The
    two-pass conversion plus padded copies cost 13–14 ms per 1080p frame on the
    Debian Intel host. One compute shader writing a tight buffer doubled the
    composited ceiling (render + text 52.6 → 109–112 fps, full MP4 + text
    59.8–83.2 → 106–112 fps) and left video-only rows unchanged. The Windows
    Intel driver runs the compute version about 8 % slower on the same
    workload. The same revision makes the choice per device: the renderer times
    both conversions once on the first composited frame and keeps the faster one
    (compute on Debian, render passes on Windows) instead of guessing from the
    backend name — both report Vulkan.

45. **Animation reaches agents, and the backend is a stored choice (2026-11-07).**
    `or timeline add-keyframe` / `clear-animation` and the MCP tools
    `add_keyframe` / `clear_animation` write the same clip update as the
    inspector, and re-adding an offset replaces it. Settings → Performance now
    stores a graphics backend (`auto` or a compiled name) applied before the
    first preview; an unknown name or a device that will not open falls back to
    automatic and the report shows both. Animated export rows exist for Debian
    (96.7–119.2 fps with a title) and Windows (126.9 video-only, 80.0 with a
    title), inside each host's static spread.
46. **Six keyframe easings (2026-11-07).** Overshoot (`ease_out_back`) and
    Bounce (`ease_out_bounce`) join linear and the three cubic curves in the
    core model, the CLI/MCP spelling, the inspector (six chips in the keyframe
    dialog) and export sampling. The `keyframes` journey edits a keyframe to
    Overshoot and reads `0.00s · 50.0% · Overshoot` back on macOS.
47. **Templates on Home (2026-11-07).** Home gained a "Start from a template"
    row (landscape, vertical, square, tutorial) plus "All templates"; picking
    one reuses the create-project dialog with the name prefilled and no frame
    rate question. The `home-template` journey passes on macOS, Debian and
    Windows.
48. **Templates show their opening still (2026-11-07).** A template card in the
    Library now renders the first visible frame of the MotionScene it opens with
    through the same `motionScenePreview` bridge call the Motion panel uses,
    once per template and cached for the session; a template without a scene or
    a render that fails keeps the text-only card. `template-stills` drives the
    real macOS app to the template grid and waits on the image label.

10. **Motion panel render record (owner request, 2026-10-28).** After a motion
    clip is inserted the panel keeps a persistent "On timeline · W×H · N frames"
    row naming the last render, with **Render again** which re-materialises the
    same scene and appends the new clip. The record survives tool switches,
    follows the open project (cleared on attach/close) and notices a draft edited
    before re-rendering. Delivered with `apps/or_app/lib/motion/motion_insert_log.dart`
    and the panel widget test "a rendered clip stays visible and can be rendered
    again".

All three desktop platforms — macOS, Windows and Linux — are acceptance targets.
One codebase serves them: shared Dart presentation over the Rust core, with
platform differences confined to the platform seam. Cross-platform parity is
therefore not separate work; a feature that only works on one desktop is an
unfinished feature.

## Deferred by the owner

Cloud or generative AI providers (including neural voices and music), translation,
dubbing, advanced effects, animation-heavy showcase surfaces, Android packaging
work, hosted CI claims and cross-platform parity as a *separate* workstream
(there is one shared product, not three). The in-app library of templates, text
looks, generated sound effects and the usage guide was promoted out of this list
on 2026-11-05 and shipped as a first-run surface.

## Acceptance target

A beginner can install the built application on macOS, Windows or Linux, find
their way in without a manual (first-run panel, library, in-app guide), and
complete both journeys through real pointer and keyboard interaction, with the
resulting Rust-owned project state, save/reopen behaviour, actual preview, and
independently validated playable exports. A widget test, hidden command, screenshot or successful build
alone does not establish a completed journey.

## Longer-term inventory (not the active queue)

| Stream | Capabilities | Gate |
|---|---|---|
| A. Install, open, protect a project | safe create/open, first-run guidance, recent projects, dirty-state protection, autosave/checkpoint, crash recovery, schema compatibility, install/upgrade/uninstall | Rust ownership, exact-base save, explicit migrations, packaged journeys |
| B. Bring media in reliably | multi-file/folder import, metadata and errors, thumbnails/waveforms, offline/relink, removal, SAF access, bounded cache/proxy | permissions before ingest; revoked/missing media and large libraries tested |
| C. Make and revise an edit | selection, tracks/clips, insert/move/trim/split/ripple, undo/redo, keyboard/pointer/touch, zoom/scroll, snapping, properties, markers, mobile adaptation | one exact-time command/history source; complex edits and persistence verified |
| D. Preview, audio, effects, text | seek/play/scrub/frame-step, audio sync, transforms, text/caption styling, effects/transitions, preview/export parity | bounded queues, immutable snapshots, deterministic fonts, measured baseline first |
| E. Export a useful result | presets and destinations, explicit capabilities, progress/cancel/error recovery, valid output, repeatable packaged export | independent probe/playback of the artifact |
| F. Focused AI assistance | transcripts, translation, analysis, generative assets, voice, provenance, review-before-apply | only after human paths work; provider-independent, no secrets to clients |
| G. Extensible and contributable | stable command/API contracts, captions/interchange, declarative templates/motion, reproducible builds | prefer formats and upstream contributions over a marketplace |
| H. Cross-platform release quality | consistent design, feature parity, accessibility, signed distribution, install/upgrade, performance | explicit platform matrix, unverified cells reported honestly |
| I. Code-to-illustration video | code ingestion, tokenisation/highlighting, `.ormotion.json`, materialisation, callouts, captions, templates | ADR 0007 contract, shared text path, zero provider calls |

Dependency order: safe project open/recovery and packaged runtime foundation →
import/media permissions → edit interactions → preview/audio/effects/text →
export; reviewable AI proposals only after the human paths work.

## Architecture direction (convergence decision, 2026-10-09)

Keep OR as an MIT, human-first, shared desktop/mobile product: keep the versioned
project format, exact-time editing model, Rust command/session/recovery authority,
semantic CLI, Flutter UI and wgpu preview/export direction. Do not adopt a
candidate on roadmap promises. OpenCut's rewrite is monitored until its promised
core/API ships and can be tested. MLT/GES are bounded media-engine candidates to
compare only against a concrete measured gap — media-engine replacement is not
approved.

## History

Dated run records, phase status and superseded plans are historical evidence:
`docs/archive/legacy/` and `docs/evidence/`. They are not current status on this
Mac and must not be used to prove a failure still exists or a success still works.
