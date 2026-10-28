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

## Unreleased

### Added
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

### Changed

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
