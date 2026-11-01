# Changelog

All notable changes to Opencut Reinforced are recorded here.

This project is **pre-MVP**. There are no stable releases. Developer Preview
packages (`dev-<sha>`) are debug builds produced from `main`, expire after a
short window, and are not production installers. Nothing below is a stability
or compatibility promise.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versions
follow [Semantic Versioning](https://semver.org/spec/v2.0.0.html) once stable
releases exist. See [docs/reference/release.md](docs/reference/release.md) for how a release is
produced, and [docs/reference/roadmap.md](docs/reference/roadmap.md) for what the
product still needs.

## 0.1.3 — 2026-11-02

### Added
- **Agent toolkit: the CLI now covers every project edit.** New verbs:
  `timeline set-clip-visual` (transform, crop, opacity, effects, transitions),
  `timeline set-clip-audio` (gain, pan, fades), `timeline set-clip-playback`
  (speed, reverse, freeze), `timeline set-canvas`, `timeline move-clips`,
  `media get`, `captions export`, and `export start|status|cancel`. Canvas,
  group moves, per-clip settings, subtitle files and export were app-only; an
  agent working in the installed workspace can now do them through the same
  validated command path the editor uses.
- **MCP parity**: the `or` server gains `set_clip_visual`, `set_clip_audio`,
  `set_clip_playback`, `set_canvas`, `move_clips`, `media_get`,
  `captions_export`, `export_start`, `export_status` and `export_cancel`, plus
  `run_cli`, which runs any `or` verb with the same attach and JSON flags so no
  operation is left without a fallback.
- **Agent workspace**: new `docs/export-and-delivery.md`; the timeline, captions,
  CLI-first, media, workspace and troubleshooting guides document the full surface
  instead of calling these operations app-only.

### Changed
- `or --help` lists every timeline verb with its flags and units (milli-canvas,
  milli-degrees, basis points, millidecibels) instead of a partial list, and the
  unknown-action error names the same set.
- Workspace bundle version 17 refreshes the framework files on the next assistant
  connection.

### Fixed
- The unknown-timeline-action error no longer omits verbs the CLI accepts, and
  `--help` no longer hides track, clip, trim and playback verbs.

## 0.1.1 — 2026-11-01

> The published 0.1.1 image predates the bundled FFmpeg and speech-runtime
> fixes below. Install 0.1.2 or later; the in-app **Resources & updates**
> panel can download it.

### Added
- **Project canvas**: a landscape, portrait or square composition shape chosen from
  the viewer header (YouTube 16:9, HD, 4:3, TikTok/Reels/Shorts 9:16, Instagram feed
  4:5, square) or matched to the first clip. The canvas is an undoable project
  command, persists in `.orproj` as schema 9, drives preview and sets the export
  shape. Projects without one keep the previous automatic size.
- **Export resolution**: the export dialog now names profile, quality and
  resolution together — match project, 2160p, 1440p, 1080p, 720p or 480p — and a
  short side keeps the canvas shape (a 9:16 project at 1080p exports 1080 × 1920).
- **Full preview transport**: full screen now has play/pause, ±5 s, a scrub bar,
  elapsed and total time, speed and exit; controls fade while watching and return
  on pointer movement or any transport key. The docked viewer gained ±5 s buttons,
  a 0.25×–2× speed menu and `[`/`]` shortcuts.
- **Decoder-aware import**: a codec the running build cannot decode is refused at
  import with `UNSUPPORTED_CODEC` instead of being admitted and failing on the
  first preview frame.

### Changed
- Pressing play with the playhead at the content end now replays from the start.
- Timeline snapping threshold doubled, from 1/8 s to 1/4 s, so a coarse drag snaps
  more readily; the exact-boundary tests moved with it.
- Import now accepts the broad profile: Matroska/WebM with AV1, VP8/VP9, H.264,
  H.265, FFV1, ProRes, MJPEG, MPEG-4 and similar video plus AAC, Opus, Vorbis,
  FLAC, ALAC, AC-3/E-AC-3 and PCM audio; MOV/MP4/M4V, WAV, MP3, Ogg, FLAC, AIFF and
  AVI containers; PNG, JPEG and WebP stills. Folder import includes those
  extensions.

### Fixed
- **FFmpeg runtime in packaged builds**: packaging now stages the pinned LGPL
  FFmpeg 8.1.3 runtime instead of Homebrew's GPL build, removes any stale ABI or
  GPL codec library from an earlier package, and refuses to sign a bundle whose
  `libavutil` does not report `LGPL version 2.1 or later`. The macOS release
  workflow builds the same pinned prefix, and `scripts/stage_macos_ffmpeg.sh`
  reproduces it locally. The pinned profile also enables the `mov` muxer required
  by the MP4/VideoToolbox export path, and the decoder list now covers the documented
  import profile (HEVC, AV1, VP8, ProRes, Vorbis, FLAC, ALAC, AC-3/E-AC-3, PCM and
  Ogg/FLAC/AIFF/AVI demuxers). This fixes `the approved LGPL FFmpeg runtime
  could not be loaded` on the first viewer frame or export in the new DMG, and the
  MP4 export failing with an argument error.
- **Bluetooth headphones**: preview used to fail with "The default output device
  does not support stereo f32 audio at 48 kHz" whenever the default device offered
  another sample format, channel count or rate. Audio output now picks the closest
  usable device configuration and adapts channels and sample rate, while the
  timeline clock still runs at 48 kHz.
- A narrow viewer no longer overflows its transport row; optional skip and speed
  controls drop out instead of clipping the time readout.
- The viewer header no longer shows the texture size as if it were the project
  shape; the canvas control replaces that badge.

## 0.1.2 — 2026-11-01

- **Resources & updates**: Settings → Resources & updates (also a Home quick
  action) reports whether FFmpeg, the local speech runtime and the `or` CLI are
  bundled, links to the assistant harness, opens the speech-model manager when a
  project is open, and checks the public GitHub release. A newer DMG can be
  downloaded to Downloads, checksum-verified when GitHub publishes a digest, and
  opened for drag-to-Applications; the app never replaces itself silently.
- **Clean-machine setup path**: a packaged build now distinguishes a missing
  runtime from a missing optional resource and points at the latest signed image
  for repair instead of failing with an unexplained FFmpeg or caption error.
- **Release checksum**: the macOS release publishes `SHA256SUMS.txt` beside the
  DMG, and the release job writes install, requirement and limitation notes.

### Added
- **Opt-in update check**: it calls the public GitHub Releases API only when the
  user presses **Check for updates**, and never checks in the background.
- **Viewer zoom**: the viewer header and full-screen controls offer Fit, 25%, 50%,
  100% and 200%. The zoom is presentation-only: it scales the picture and never
  changes the project, its revision or the frame the runtime is asked for.
- **Motion presets apply to a selected object**: a `motion` library asset can be
  applied to the chosen canvas node from the Motion panel. It writes one property
  track through the validated scene-edit path, replacing an existing track for
  that property rather than duplicating it.
- **Packaged local speech runtime**: the macOS release pipeline builds pinned
  whisper.cpp v1.9.4 and the provider adapter, the packaging script copies both
  beside the app and refuses an image that asks for them without them, and the
  speech-model picker offers a **Download** action on each not-installed row.
  A build without the runtime now says so and disables **Use model** instead of
  leading to a download it cannot run.
- **Helper lookup follows PATH and Resources**: `whisper-cli` and the provider are
  found beside the executable, under the bundle's `Resources`, or on `PATH`, so a
  system install documented in the local guide is actually reachable.
- Packaged builds now carry the whisper.cpp and FFmpeg third-party notices.
- **Prompt-driven background music**: the Generate music dialog accepts a free
  sentence in Vietnamese or English, shows the interpreted mood, tempo, length
  and drum intent, then generates exactly that bed. A tempo in bpm, a length in
  seconds/minutes and "không trống"/"no drums" are honoured; unrecognised words
  leave the controls alone instead of failing the dialog. The same prompt and
  variation are reproducible, and **Variation** reseeds the arpeggio without
  changing the readback.
- **Music prompt in the assistant workspace**: `tooling/or-agent-workspace/tools/music.mjs`
  takes `--prompt` and reads it with the same interpreter as the dialog, printing
  the same summary and `plan.notes`; `--variation` reseeds it and explicit
  `--mood`/`--tempo`/`--duration`/`--seed` still win.
- **Open a MotionScene file on the canvas**: the Motion panel opens an
  `.ormotion.json` authored by the CLI or an agent. Rust bounds the file, parses
  and validates it, and adopts its referenced PNGs into the app library first, so
  the imported scene edits, previews and materialises like a composed draft; a
  missing or unsafe image is a named refusal. A scene saved to the library can be
  reopened from its row for further editing.
- **`motion_scene_read` / `motion_scene_from_asset`** bridge functions and their
  adoption test in `or_motion::library`.
- `scripts/ui_scenarios/motion-scene-import.json` — imports a scene fixture,
  proves its objects reach the canvas, nudges one, and inserts the edited scene
  on the timeline.
- **Import batch progress**: while a batch runs, the top bar shows
  `Importing 2 of 5 · name…` with an import glyph, and a completed multi-file
  batch reports its count. A slow single file is visibly in flight instead of
  leaving the surface silent.
- **Existing-project import journey**: `scripts/ui_scenarios/existing-project-import.json`
  opens a project saved without a sequence frame rate (the shape a new project
  can be saved in), imports a real clip and proves inserting it creates the
  missing track. The harness fixture picker now copies an `.orproj` fixture into
  the scenario directory so the open path is real, and
  `crates/or_core/tests/fixtures/null-rate-project.orproj` is that shape.
- **Still-image insertion journey**: `scripts/ui_scenarios/image-import-timeline.json`
  imports a PNG with no audio or video stream into a brand-new project and
  proves it becomes a timeline clip and the first track, so the image path is
  covered by a real journey instead of only by video fixtures.
- **Assistant panel**: a right-rail tab connects to a locally installed
  DeepSeek Harness over ACP, lets the user pick the workspace and model, streams
  replies and tool calls, and renders permission prompts. The harness reads and
  edits the open project through the `or` CLI attached to the live session, with
  the `or mcp` tools as the fallback for motion insertion and the motion library;
  both use the same local IPC descriptor and validated command path.
- **`or mcp`**: an MCP stdio bridge with project, timeline, media, text, marker,
  save/undo and motion-insertion tools; `or motion materialize` renders a code
  file or scene through the app's MotionScene/FFmpeg runtime. The assistant turns
  a code file into an ordinary, editable timeline clip this way.
- An `or_core` application request and IPC route for MotionScene
  materialisation, plus focused contract, MCP and UI tests.
- Two real-app journeys: `scripts/ui_scenarios/assistant-edit.json` and
  `assistant-motion.json`, plus `scripts/ui_scenarios/motion-scene-edit.json` for
  the scene-editing journey (resize, rotate, marquee, multi-delete, undo/redo).

- **Scene editing on the Motion canvas**: the composed draft is editable in place
  — select one or many objects (click, shift-click, marquee), drag a selection,
  resize a shape from its corner grips, rotate it from the rotate grip, retitle,
  recolour, outline, change type size, add text, append library objects, delete,
  and undo/redo the draft — inline in the panel or in an expanded editor. Every
  gesture is a typed operation applied in Rust and re-validated as an ordinary
  MotionScene, so an edit can never produce a scene the renderer refuses; geometry
  is written against the evaluated frame, so an animated object lands where the
  pointer is instead of jumping by its track offset. The bridge returns the edited
  scene, its first frame and the node boxes in one call, and the draft keeps a
  32-step history that never touches project undo.
- **Motion library multi-selection**: rows can be ticked (or selected with the
  list's select-all) and added to the composition or deleted together.
- **Local multilingual captions from audio**: Generate Captions now downloads a
  multilingual whisper model once and detects Vietnamese or English
  automatically. The host-owned transcription job is also exposed to the CLI and
  agents as `or captions transcribe`, with `or timeline import-captions` applying
  a reviewed SRT/WebVTT as a new caption track. The transcript stays a proposal
  until the user applies it.
- **In-app voiceover and music**: the Audio tool exposes Generate voiceover…
  and Generate music…. The voiceover dialog speaks one line per scene with the
  chosen macOS voice, imports the WAVs, places them on a new audio track and
  imports exact captions. The music dialog synthesises a quiet royalty-free bed,
  places it, and can duck it −12 dB under a selected voice track.
- **One-step music ducking**: `timeline.clip.duck` splits a music clip at voice
  boundaries and lowers the overlapping segments in one Rust command, so the app
  and `timeline duck-clip` (and `duck_music` over MCP) produce one revision and
  one undo entry.
- **Speech-model management**: the picker shows total installed size and can
  remove a downloaded model to free disk; removing the active model falls back
  to Small. The Audio tool links directly to the picker.
- **Selectable speech models**: Base, Small, Medium, Large v3 Turbo q5 and
  Large v3 Turbo are listed in the app with their real sizes and licences; the
  user picks one, downloads it once, and the app and `or captions transcribe`
  share the choice. Installed-model listing checks presence and size, while the
  transcription job still verifies the full SHA-256.
- **Local transcription helpers**: `scripts/install_local_transcription_helpers.sh`
  copies `whisper-cli` and the rebuilt provider into a local debug bundle,
  re-signs it ad hoc, and can install a verified small model.
- **Assistant production workflow**: the installed workspace carries
  `tools/voiceover.mjs` — an offline `or-assistant-script` narrator using the
  macOS `say` voice that measures real durations, writes a timing manifest and
  SRT, and can import, place and caption the result — and `tools/music.mjs`, a
  deterministic, royalty-free background-music generator. Both are documented by
  a new workspace guide and the `or-script-production` skill.
- Empty new projects present a working **Import Media** action that reaches the
  existing platform picker and permission path, on desktop and compact layouts.
- **Folder import**: selecting a folder imports the selected files plus files in
  its nested subdirectories, through the same picker and permission path.
- **Recent project return**: a recently opened project can be reopened from the
  Home surface after an application restart; on Android the persisted recent URI
  is rebound through a fresh SAF picker grant.
- `docs/evidence/claims/` records machine-readable verification claims for
  product requirements, with the run, job, evidence class and limitations each
  one does and does not establish.

- **Motion panel render record**: after a motion clip is inserted, the Motion
  panel keeps an "On timeline · W×H · N frames" row naming the last render and a
  **Render again** action that re-materialises the same scene (draft or library
  asset) and appends the new clip. The record follows the open project — it is
  cleared on attach and close — and the panel still notices a draft edited after
  the render.

### Changed

- **The assistant rail stops rebuilding at frame rate.** While playback runs,
  the editor publishes its playhead to the assistant at most four times a second
  instead of every frame, and a context the panel already has no longer notifies
  it. A paused seek, a frame step and a clip selection still arrive at once, so
  "this clip" and "this moment" stay exact where it matters. Four seconds of
  playback now publishes 16 contexts instead of 240 panel rebuilds, measured by
  `apps/or_app/test/assistant_context_test.dart`.
- **Every viewer frame publishes faster.** The RGBA→premultiplied-BGRA step that
  every macOS preview frame takes before Flutter uploads it now writes into a
  sized buffer and uses a shift-divide instead of a per-pixel append and four
  divisions, and opaque frames skip the multiply entirely. A 1920×1080 publish
  measured 59.0 ms → 13.5 ms in a debug build and 2.8 ms → 2.0 ms in release on
  this Mac; the published bytes are unchanged.
- **Templates and Asset Library destinations removed**: both surfaced shields and
  filter chips for catalogs the product does not ship (cloud/community assets are
  deferred by the owner), which read as broken controls. The shell now lists only
  Home, Projects, Settings and the developer Editor preview; the dead screens,
  their disabled chips and the `OrSearchField` widget are gone.

- **Assistant edits the project through the `or` CLI by default.** The installed
  workspace and persona attach `tools/or` through the workspace `project.json`
  pointer, and the `or mcp` tools stay as the documented fallback for motion
  insertion and the motion library.
- **`or --attach` accepts a session pointer.** The descriptor may be named by a
  pointer file such as `project.json`; the CLI follows its `descriptor_path` on
  every call, so a command line survives a project switch. `timeline trim-clip`
  also takes `--ripple` as a boolean flag.
- **macOS Developer Preview runs unsandboxed** so the assistant can start the
  local harness and its bridge; App Sandbox cannot grant that process boundary
  (ADR 0009). Packaging now asserts the entitlement stays absent.
- **The transcription model is multilingual.** The previous English-only
  `whisper-tiny-en` checkpoint is replaced by `whisper-small-multilingual`
  (about 465 MiB, MIT, downloaded only after consent and verified by SHA-256), so
  the same local install serves Vietnamese and English with automatic language
  detection. Existing English-only installs download the new model once.
- **Documentation restructured around task routing.** `AGENTS.md` is now a pure
  index: one project sentence, one rule, and a name / when-to-use / exact-path
  entry for every document, tool and skill. The former monolithic documents are
  split into `docs/architecture/`, `docs/features/`, `docs/design/`,
  `docs/operations/` and `docs/reference/`; the superseded set is preserved under
  `docs/archive/legacy/`, and OR skills were trimmed to their OR-specific
  contracts.
- **Compact editor layout**: the compact presentation no longer repeats the
  desktop `Viewer` and `Timeline` section headers, so a 320x480 preview surface
  grows from 27.7 px to 51.7 px and the timeline lanes stay visible. Compact
  transport, timeline toolbar and view controls now meet the 44 px touch target;
  the desktop keeps its denser 40 px controls and no longer squeezes the
  timeline view controls to 33 px.
- **Add to Timeline** now inserts at the occupied timeline's content end rather
  than at the timeline origin.
- Home no longer presents the visual-only editor shell as a usable project.
- Documentation reorganised: the paused checkpoint ledger moved to
  `docs/archive/execution/`, `docs/archive/legacy/ROADMAP.md` is labelled archival, and
  `DEVELOPMENT_WORKFLOW.md` selects work from the product roadmap instead of the
  historical `NEXT` cursor.

### Added (infrastructure)

- `scripts/check_doc_links.py` — verifies that relative links in tracked Markdown
  resolve. Run by repository hygiene.
- `scripts/check_doc_freshness.sh` — verifies that documents making current
  claims carry a recent `Last reviewed` date.
- `scripts/check_claims.py` — validates claim-record schema and honesty fields.
- `scripts/check_app_bundle.py` — refuses to package a desktop Flutter bundle
  whose Dart kernel carries the integration-test listener instead of the
  application. Its hermetic checks run in `scripts/check-repo.sh`.

### Fixed

- **A local voice-cloning run could re-clone a voice whose consent had been
  withdrawn.** The local-server path heals a restarted server by re-enrolling
  the saved reference clip, but that automatic enrolment ignored the rights
  acknowledgement and then wrote `local_clone_acknowledged: true` back into the
  narration settings. Re-enrolment now requires the acknowledgement that is on
  record and a clip to enrol; a run without it stops with "tick the rights
  acknowledgement, then press Enrol voice" instead of silently cloning, and a
  re-enrolment records the new voice id and clip without touching consent.
- **Preview no longer failed where Generate healed.** After the local speech
  server restarted, pressing Preview with a cloned voice showed the server's raw
  "unknown voice" error while Generate recovered by re-enrolling the saved clip.
  Generate and Preview now share one recovery path, so an audition heals the
  same voice, obeys the same consent rule, and plays the healed sample.
- **An MP3 reference clip was uploaded as WAV.** The local enrolment form
  declared `audio/wav` for every reference clip, while the picker accepts WAV,
  MP3, FLAC, OGG and M4A and the upload is the file unchanged. A server that
  decodes by the declared type therefore failed on any non-WAV clip, including
  the automatic re-enrolment after a restart. The part now declares the clip's
  real format.
- **The editor could look frozen after a native file panel.** macOS open/save
  panels are now sheets attached to the editor window with a single-flight guard
  and focus restoration after dismissal. A panel that previously ended up behind
  the editor left an invisible application-modal session that swallowed every
  click while keyboard shortcuts still worked; a second request while one sheet
  is open now reports `FILE_PANEL_BUSY` instead of stacking another session.
- **Import Media and Add track could do nothing with no message** while another
  project action was running. The concurrency guard still refuses to stack
  commands, but it now tells the user what to wait for.

- **Text input in every field**: the shell's shortcut map no longer captures
  typing keys while a text field is focused. Backspace, Space, letters, arrows,
  Undo, Copy and Paste now reach the focused field; the native menu no longer
  owns modifier-less or ⌘Z/⌘C/⌘V equivalents. The assistant composer also lets
  an IME commit a Vietnamese composition before Enter sends.

- **Developer Preview desktop packages opened to a black screen.** The macOS,
  Windows and Linux jobs ran `flutter test -d <platform>` after building the app
  and then packaged the same output directory, which the test run rebuilt with
  the integration-test entrypoint. The application `main()` never ran and the
  window never drew. Each job now stages the application bundle before the test
  run, packages the staged copy, and verifies the entrypoint.
- The editor inspector's text fields collapsed to a height where the floating
  label overlapped its value; the fields now use a full-height content padding
  and keep the rational-time hint on one line.
- The desktop tool rail clipped `Templates`, `Captions` and `Transitions`; it is
  wider and ellipsizes rather than clipping.
- The timeline toolbar hid `Export Captions` and `Add Caption` at 1280 px with
  no visible scroll affordance; narrower desktops now use short action labels
  and the toolbar exposes a persistent horizontal scrollbar.
- The command-palette hint printed `Ctrl/⌘ K` on Linux and Windows; each
  platform now names its own shortcut.
- The empty inspector asked to "unlock the selected track" when no clip was
  selected; it now asks the user to select a clip.

## Verification

Every claim above has a dated record under [docs/evidence/claims/](docs/evidence/claims/),
naming the implementation SHA and the hosted run that verified it. Exact
measurement, platform scope and limitations are in
[docs/evidence/PRODUCT_ROADMAP_HISTORY.md](docs/evidence/PRODUCT_ROADMAP_HISTORY.md).

Physical-device performance, Android transcription, clean-machine
install/upgrade/uninstall, and ordinary-user dogfood remain unverified. Emulator
evidence does not establish any of them.
