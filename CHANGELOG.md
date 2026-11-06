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

## 0.1.19 — 2026-11-06

### Added
- **The real app is driven on Windows too.** `scripts/or_ui.py --target windows`
  runs the same semantics scenarios as macOS and Linux through
  `flutter test -d windows`. The Windows workspace chrome, the library tour, the
  Resources and updates section and the speech-runtime install/remove journey
  all pass on the real Windows build.

### Fixed
- **The machine panel no longer spins.** The "This machine" panel ran its memory
  probe and its disk probe one after the other, and on Windows each one is a
  PowerShell launch, so the panel showed a spinner for the length of two process
  startups. Both probes now run together, and a probe that fails prints "This
  machine could not be read: …" instead of leaving the row empty forever.
- **Eighteen scenarios named a fixture on their author's Mac.** A
  `copy:/Users/meomeo/...` fixture staged nothing on any other host, so those
  scenarios could only ever pass on one machine. They now name a
  repository-relative path.

## 0.1.18 — 2026-11-06

### Fixed
- **The Windows installer can uninstall itself while the app is running.** Inno
  runs `PrepareToInstall` for an install but not for an uninstall, so a silent
  uninstall over a running copy exited 1 and left every file in place, and a
  build that only stopped the process exited 0 but left the DLLs for the next
  reboot. `InitializeUninstall` now stops the app and waits until Windows has
  released its image sections before the uninstaller touches a file. Verified on
  the Windows host on a fresh install: uninstall over a running copy removes the
  program and keeps the user's projects. Nothing else changed, and no macOS or
  Linux behaviour is affected; a patch release is cheaper for users than asking
  them to kill the app by hand before uninstalling.

## 0.1.17 — 2026-11-06

### Added
- **The performance budget runs, and reports, on all three desktops.**
  `scripts/bench_quality.py` reaches a verdict on macOS, Debian and Windows from
  one commit, prints the host it measured, and gives a row its own baseline
  instead of a loosened default where the machine's hardware explains the
  number (the Windows single-threaded raster and save rows).

### Changed
- **The saturation lane measures OR's pool, not the operating system.** The
  policy for the artifact worker count lives in `or_core` (the bridge delegates
  to it), so the app and the lane cannot drift; the lane now saturates that pool
  with twelve jobs waiting behind it. The same lane reported a 29.7-33.8 ratio on
  a ten-core Windows laptop before and 1.08 / 1.27 / 1.43 on macOS / Debian /
  Windows after, with the 6.0 ratio budget unchanged.

### Fixed
- **A saved CLI path that no longer exists no longer disables the assistant.**
  The stored path can outlive the build it named; detection now decides instead,
  and the run harness names the debug CLI it built (`OR_CLI_PATH`), so
  `assistant-panel` can judge the panel rather than the machine.
- **The Windows installer can update a running app.** It stops the running
  executables before it replaces files instead of retrying a locked DLL and
  rolling back with no message; a fresh install, an upgrade over the running app
  and an uninstall were all exercised on a Windows 11 host.
- **A package cannot ship a stale CLI.** The Windows lane rebuilds `or_cli` and
  the provider before staging, and the stager refuses a staged `or.exe` whose
  version disagrees with the workspace.
- **The benchmark crashes on Windows are gone.** No unconditional `pgrep`, the
  configured FFmpeg `bin` is prepended so the DLLs load, the test binary is
  filtered to `.exe`, and the clean-up fixture probe looks for `ffmpeg.exe`.
- **Two scenarios only ran on the machine that wrote them.** `export-long`,
  `mov-import` and `media-decode-refused` imported files under `/tmp` that nothing
  generated; they now import small checked-in fixtures whose generating command
  and SHA-256 are recorded, and `track-reorder` matches the label the track lane
  really produces.
- **A complete run is not reported as a failure.** When the macOS tool refuses to
  bring the window forward during teardown, the harness still passes a run whose
  transcript is complete, and it writes the runner's exit code into its log.
- **A command pressed in the frame a dialog closes reaches its action.** The
  shell no longer treats the focus node of a torn-down text field as owning the
  keyboard.

## 0.1.16 — 2026-11-05

### Added
- **A guided first project.** Simple mode is on by default: the timeline toolbar
  names the next step (import, first clip, title, audio, export), the export
  dialog asks what the file is for instead of which codec, and composition
  guides (`G`) show the action- and title-safe areas without ever reaching the
  export. Every step writes the ordinary command, so a beginner and a
  professional edit the same project.
- **Scene typography (MotionScene schema version 2).** A text node sets its own
  font, weight, alignment, line height and tracking, and an explicit `size` is a
  real text box, so titles wrap and anchor instead of shrinking. Project text
  clips reach the same two metrics from their own dialog.
- **Scene shape style (schema version 3).** A rectangle carries a corner radius
  and any shape a two-stop gradient, so cards, chart panels and lower thirds are
  styled in the scene language instead of being drawn as hard-edged blocks. The
  inspector steps the radius.
- **The app states the machine it will render on.** Settings → This machine and
  the first-run panel report the CPU class, memory and the measured export
  formats, with the 4 GiB headroom rule stated rather than hidden.
- **Export formats are measured, not assumed.** The engine reports which encoder
  the build actually has, the dialog filters profiles by that answer, and a
  caveat replaces the choice instead of failing at the end of a long export.
- **Optional components can be removed.** Setup deletes exactly the files it
  installed (recorded in a manifest), refuses to touch anything a user added,
  and says so when there is no record to act on.
- **Library credits.** Library → Guide lists the licence, origin and permitted
  use of every bundled font and generated asset; the Text and Sound tabs name
  their licence beside the content.

### Changed
- **Preview and export share one picture.** A parity test renders the same frame
  through the viewer path and the export path, decodes the export the way a
  player does and refuses more than a 30 dB difference (measured 62.6 dB on
  Apple Silicon and on Intel under Mesa).

### Fixed
- **Scene image loading refuses rooted paths on Windows.** A path such as
  `\Users\me\pic.png` is not `is_absolute()` there, so a scene could read
  outside its own folder; the loader now refuses `has_root()` as well.
- **The packaged Linux lane uses the root it was handed** instead of an
  unexported variable, and it now runs end to end with the pinned FFmpeg 8 and
  codec prefixes: the `.deb` is built, installed, launched and shown to export a
  VP9+Opus 1080p file that an independent `ffprobe` decodes.
- **The Windows package stages the runtime it is asked for**, accepts a real
  Python on a host whose `python3` is a Store alias, and the lane runs under
  pwsh 7.

## 0.1.15 — 2026-11-05

### Added
- **Library: templates, text looks, generated sound effects and an in-app guide.**
  A new `Library` destination on every desktop, browsable without a project. A
  template sets its own canvas and frame rate and places its starter text through
  the ordinary timeline command; a text look states its real font, size, weight,
  alignment and anchor rather than promising one; a sound effect is synthesised
  as a real 16-bit PCM WAV and imported through the media command; the guide
  covers start, import, edit, text, audio, export, shortcuts, files and
  troubleshooting and renders `Cmd` or `Ctrl` from the platform seam.
- **First-run panel on Home** that explains where media lives, that a project is
  one atomically saved `.orproj` file, and that export runs locally with the
  hardware encoder when the machine has one. Dismissal is remembered, and an
  unreadable preference file shows the panel rather than hiding it.
- `or project create --file PATH --name NAME [--json]`, so a script or an agent
  can start a project headlessly and get the same refusal the app applies when
  the destination already holds one.
- `scripts/bench_export.py`: portable `decode`/`render`/`full` export rows for
  MP4, WebM and Matroska at one picture size, on any desktop, with the FFmpeg
  prefix and library path resolved automatically.

### Changed
- **Parity is the product rule.** macOS, Windows and Linux are all acceptance
  targets over one codebase; `docs/architecture/platforms.md`,
  `docs/reference/roadmap.md` and `docs/operations/workflow.md` now say so, and
  the workflow records the one-pull-request, one-CI-run release rule.
- The NV12 readback path pools its two textures and one staging buffer instead of
  allocating them per frame, and issues one submit and one wait instead of two.
  Measured on the local MacBook at 1920×1080: NV12 render 89.7 → 96.3 fps, full
  MP4 export 81.3 → 87.0 fps.

### Fixed
- **MP4 export failure named nothing.** A build with no usable H.264 encoder
  returned FFmpeg's bare `Invalid argument`. It now names the encoder that did
  not open and points at WebM or Matroska.
- **A template's starter text was silently dropped.** The insert ran while the
  shell still held its project-action lock; the overlays are now placed after the
  create action releases it, which two widget tests guard.
- **The first-run panel could stay hidden when the preference read stalled**, so
  a beginner saw a bare Home screen. A bounded timer now shows it anyway.

## 0.1.4 — 2026-11-02

### Added
- **Linux x86_64 `.deb` and Windows x86_64 `.zip` release packages** next to
  the macOS Apple Silicon `.dmg`. All three ship the same Flutter app, the same
  Rust core and the same command surface; only the native adapter and packaging
  layer differ. Each package carries its own FFmpeg 8.1.3 runtime, codec licence
  notices, the `or` CLI and the assistant workspace, so no developer toolchain is
  needed to run it.

### Changed
- The Windows build is produced by `scripts/run_windows_lane.ps1` and
  `scripts/stage_windows_bundle.ps1`, which the hosted Windows job now runs
  unchanged. The real-machine lane and CI therefore share one build and one
  staging procedure instead of two copies of the same steps.

### Fixed
- **Windows IPC served one client at a time.** The named-pipe accept loop ran
  every request on the accepting thread, so a query arriving while a motion
  scene rendered waited for the render; a shutdown request also left the loop
  parked in `ConnectNamedPipe`. Each connection now gets its own handler thread
  and a shutdown wakes the loop with a connection. This is what made
  `a_motion_render_does_not_block_queries` fail on Windows only.
- **The release gate could pass on a red platform.** `release-gate` now requires
  the macOS, Linux and Windows jobs (plus the Rust, Flutter static and descriptor
  boundary jobs) and rejects `skipped`, `cancelled` and `neutral` results, so a
  missing native platform can no longer leave the gate green.
- **The packaged journey and the Linux UI preflight drove controls the editor no
  longer has.** They tapped `timeline-add-audio-track`, `timeline-undo` and
  `timeline-add-video-track`, dismissed a clip dialog that is now a selection
  surface, and read the save control as a `TextButton`. The import step also
  tapped a disabled button while the shell was still opening the project, which
  looked like a 1080p H.264/AAC import timeout on Linux CI. The tests now use the
  create menu, the current save/undo controls and a key-based readiness wait.
- The local lane gives benchmark binaries the pinned FFmpeg library path and
  builds the motion raster test, so the performance budget reports measured rows
  instead of `UNVERIFIED`. The packaged journey refuses to run when
  `OR_PACKAGED_MEDIA_RUNTIME_DIRECTORY` is unset rather than staging into the
  current directory.
- The CLI export argument test used Unix-only paths, so on Windows it asserted
  the absolute-path error instead of the error under test.
- **The Linux build linked the host FFmpeg.** `libor_app_bridge.so` came out
  with `NEEDED libavutil.so.59` because another dependency's pkg-config search
  path preceded the packaged prefix, so the editor refused to export with the
  approved-runtime message. `scripts/configure_linux_cargo_for_ffmpeg.sh` now
  writes the pinned prefix into rustflags ahead of every build-script search
  path, and both the lane and the hosted Linux job call that one script.
- **`or_media` did not compile for the desktop Windows target.** The SAF
  seekable-media probe was declared unconditionally while the capability it
  takes is Unix-only; the helper is now gated to the platforms that have it.
- **Windows could not build the bridge through the Flutter hook.** The hook
  runs cargo with an empty environment, so the FFmpeg build script fell through
  to pkg-config; `scripts/configure_windows_cargo_for_ffmpeg.ps1` pins the
  prefix in the cargo config the hook reads. Staging also needs PowerShell 7 and
  the Debug bundle the journey actually runs, both of which the lane now
  guarantees.
- **The close-project control was enabled while a project action was in
  flight** and silently ignored the click, which is what stalled the Windows
  journey after reopening a recent project. The control is disabled while the
  shell is busy, matching the save control.
- The packaged journey's missing-inspector phase asserted a message the app
  cannot show while the in-process decode fallback exists; it now asserts the
  refusal the reader actually sees and that no media joins the library.
- The bounded Linux preflight builds its own bundle, so the lane now stages the
  packaged runtime for it instead of relying on whatever an earlier step left
  behind; the release lane can therefore build a release package and still run
  the preflight.

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
