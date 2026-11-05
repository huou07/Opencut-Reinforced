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

Blocked, and reported as blocked rather than attempted:

- **The Windows installer (.setup.exe).** The Windows ZIP package is built,
  installed, replaced, launched and uninstalled on a real Windows 11 host from
  the same commit as macOS and Linux (`WINDOWS_LANE=PASS`, SHA-256
  `d24a1bd2…`, see the parity receipt). The Inno Setup definition is not
  exercised locally because Inno Setup 6 is not installed on that host; the
  `.zip` is the tested artifact and `.setup.exe` remains a CI-produced,
  locally-unproven package.
- **Installer-level parity on Linux.** No `.deb` was built or installed on the
  Debian host in this round; the Linux journey runs from a developer build under
  Xvfb. The Windows ZIP install/uninstall journey and the macOS bundle are
  recorded in the parity receipt.
- **An optional download installed and removed through the GUI.** The
  `resources-setup` journey proves the surface is reachable and inert until a
  user acts; it does not download a model or an update, so that path is not
  claimed as verified.

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
