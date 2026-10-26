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

- **Assistant panel**: a right-rail tab connects to a locally installed
  DeepSeek Harness over ACP, lets the user pick the workspace and model, streams
  replies and tool calls, and renders permission prompts. The harness edits the
  open project through `or mcp`, which attaches to the same local IPC descriptor
  and validated command path as the CLI.
- **`or mcp`**: an MCP stdio bridge with project, timeline, media, text, marker,
  save/undo and motion-insertion tools; `or motion materialize` renders a code
  file or scene through the app's MotionScene/FFmpeg runtime. The assistant turns
  a code file into an ordinary, editable timeline clip this way.
- An `or_core` application request and IPC route for MotionScene
  materialisation, plus focused contract, MCP and UI tests.
- Two real-app journeys: `scripts/ui_scenarios/assistant-edit.json` and
  `assistant-motion.json`.

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

- **macOS Developer Preview runs unsandboxed** so the assistant can start the
  local harness and its bridge; App Sandbox cannot grant that process boundary
  (ADR 0009). Packaging now asserts the entitlement stays absent.
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
