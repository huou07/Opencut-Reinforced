# Active Product Roadmap

Last reviewed: 2026-10-18

## Authority and status

This outcome-based roadmap supersedes the old checkpoint sequence for active
work as of the convergence pivot recorded in
[OPEN_SOURCE_CONVERGENCE.md](OPEN_SOURCE_CONVERGENCE.md). It groups real user
capabilities and technical dependencies; its rows are not separate operator
prompts or mandatory commits. `docs/archive/execution/PLAN.json`, `STATE.json`
and `phases/` remain immutable historical traceability for the work
already performed. Their future `NEXT` cursor does not authorize a return to
the old implementation sequence.

Product requirements in [PRODUCT.md](PRODUCT.md), UX truth in
[DESIGN.md](../DESIGN.md) and [UX_ACCEPTANCE.md](UX_ACCEPTANCE.md), and
architecture/security/persistence invariants remain in force. A roadmap row
can be narrowed only when the source requirements or measured user value
justify it; record the exact rationale and evidence here. Do not label a
capability done from plans, source inspection, or model claims alone.

## Current product focus (owner restatement, 2026-10-18)

The owner restated the product direction on 2026-10-18. These four capability
groups are the active focus, in dependency order. Dogfood validation of the
first-project journey continues in parallel rather than as the single trumping
focus.

1. **Complete basic editing** — the editing journey must be complete, fast and
   understandable, with the same concepts, terminology and command semantics on
   macOS, Linux and Windows, and an adapted but equivalent Android presentation.
2. **Code-to-illustration video** — turn a code artifact into a declarative
   MotionScene and render it deterministically. Track A is primary; Track B, an
   external generative-video provider, is a later optional addition.
3. **Automatic subtitles** — implemented for desktop debug packages; Android
   support, word timings and translation remain open.
4. **Noise filtering and audio cleanup** — belongs in the audio layer.

Build and test run fast locally on the Debian workstation for Linux desktop and
the Android emulator; every release still passes hosted CI before publication.

Recent focused work: Candidate `3bf2e6e8980c093072b217ba33c6c91a67e1aa82`
removes the visual-only editor shell from Home and makes Add to Timeline
default to the current content end, so consecutive clips can be assembled
without manually calculating a start time. Exact-SHA Platform Verification
[38366017857](https://github.com/huou07/Opencut-Reinforced/actions/runs/38366017857)
passed all seven jobs. Validation-only Developer Preview
[38369979125](https://github.com/huou07/Opencut-Reinforced/actions/runs/38369979125)
built Android, Linux, macOS, and Windows packages and passed packaged desktop
transcription/caption acceptance. That is four build jobs; the Linux job emits
both a tarball and a `.deb`, so a complete run produces five app packages (see
RELEASE.md). The five temporary artifacts expire October
20, 2026; the run summary includes the dogfood journey and feedback route.
These are debug packages, not stable installers or a public release.
Clean-machine installation/upgrade and physical-device responsiveness remain
unverified; Android transcription is unsupported. Detailed historical
evidence remains in the [history archive](evidence/PRODUCT_ROADMAP_HISTORY.md).

Developer Preview [dev-656cd656199c](https://github.com/huou07/Opencut-Reinforced/releases/tag/dev-656cd656199c)
targets exact product SHA `656cd656199ca6a236921c83ba9bdb5705f61d57`;
Platform Verification [38192698936](https://github.com/huou07/Opencut-Reinforced/actions/runs/38192698936)
passed all seven jobs, and Developer Preview
[38194980068](https://github.com/huou07/Opencut-Reinforced/actions/runs/38194980068)
published and downloaded all 13 assets to re-verify `SHA256SUMS.txt`. The
release is an unsigned or ad-hoc-signed debug preview, not a stable release;
Android verification covers emulator correctness and resource bounds only,
not physical-device performance. The earlier transcription WAV handoff, pinned
Whisper proof, and proposal contract are recorded in the
[transcription integration note](evidence/local-transcription-poc.md).

The 2026-10-15 quality pass adds a bounded Linux debug journey and executable
semantics actions for custom timeline clips/trim handles. Local Rust-backed
create/import/recovery/edit/undo/seek/save-reopen and selected screenshots ran;
[testing evidence](TESTING.md#evidence-selection-and-bounded-linux-preflight)
records scope and rendering limitations. Exact source `66164227f3e39aa32567890eb88850d54ff9aa37`
passed [hosted Linux UI preflight 38679257316](https://github.com/huou07/Opencut-Reinforced/actions/runs/38679257316).
This accepts the scoped Linux debug journey; it does not advance packaged,
preview rendering, media playback or cross-platform acceptance claims.

## Product acceptance target

A non-developer can install a supported package, understand the workspace,
create or open a project, import local media through the platform's real
permission model, assemble and revise a useful edit, preview with responsive
audio/video behavior, use available assisted features as reviewable proposals,
save/reopen/recover safely, and export a valid result using shipped
dependencies. The same project/editing concepts, terms, layout intent, and
command semantics hold across macOS, Windows, Linux, and Android; input and
layout adapt to platform and screen size. iOS remains an intentional future
parity target, not an acceptance claim until supported tooling and packages
exist.

Acceptance covers packaged user journeys, usability, performance, and
resources on real hosted platform paths. Green unit tests alone are
insufficient. Track crashes, corruption, import/playback/export failures,
accessibility and task-completion issues as product defects.

## Work streams

The order below preserves actual dependencies while allowing independent work
to proceed together. Exact evidence is attached to requirement IDs and
implementation/workflow SHAs in the durable execution evidence ledger; this
roadmap supplies the human mapping.

| Work stream | Capabilities and mapped legacy requirements | Dependencies and evidence required |
|---|---|---|
| **A. Install, open, and protect a project** | Finish safe project creation/opening, clear first-run guidance, recent projects where justified, dirty-state protection, autosave/checkpoint policy, crash recovery, schema compatibility, and package install/upgrade/uninstall. Legacy: 9E; relevant 4E1/4E2/4F and release requirements. | Preserve Rust ownership, exact-base and explicit migrations. Verify clean packaged installs and create/edit/save/close/reopen/recovery across desktop; verify Android SAF grants and process death through hosted device acceptance. |
| **B. Bring media into the project reliably** | Media import for multiple files/folders where platform APIs permit, useful metadata/errors, thumbnails/waveforms, offline/relink workflow, media removal, Android SAF/document provider access, and bounded cache/proxy controls. Legacy: 9A/9B/9D/9E and 5A–5F. | Permissions and source references precede ingest. Test revoked/missing media, large libraries, >64 assets, SAF providers and packaged ffprobe/FFmpeg. Existing evidence covers desktop local files and Android SAF preview/export/recovery. Single- and multi-file SAF import plus desktop multi-file selection are implemented at `8dcadd6`; clearer missing-source feedback and the corrected acceptance assertion are at `8648be7`. Undoable desktop local-file relink preserving MediaId and timeline references is implemented at `d313a13`, with the packaged desktop journey at `e40c1ef`. Android real-picker relink acceptance targets the existing item at index 64 and checks MediaId/timeline preservation through save, reopen, and process recovery; recent and selected-source SAF grants are validated through the real picker. Folder import passed exact-SHA packaged verification at `a7f4bf4`; remaining gaps are large-library cache behavior and a broader offline-media/bulk-relink workflow. Historical failure receipts are in the [history archive](evidence/PRODUCT_ROADMAP_HISTORY.md). |
| **C. Make and revise an edit** | Clear timeline selection, tracks/clips, insert/move/trim/split/ripple, undo/redo, keyboard/pointer/touch operations, zoom/scroll, snapping, clip properties, markers, and mobile adaptations. Legacy: 6A–6E, 8A–8F, 9C, 13A/13B/13G/13H. | Keep exact Rust time and one command/history source. Group interaction and command work by user journey. Verify representative complex edits, persistence, CLI parity, and touch/keyboard accessibility. Defer advanced nested/multicam semantics until normal edits are smooth. |
| **D. Preview, audio, effects, and text coherently** | Reliable seek/play/scrub/frame-step, audio sync and device controls, typed transforms, text/caption styling, effects/transitions, and preview/export semantic parity. Legacy: 7A–7H, 8A–8F, 10C–10D, 13C–13F. | Preserve bounded queues, immutable snapshots, deterministic fonts, fallback behavior, and frame/audio resource budgets. Measure the packaged OR baseline first. If it exposes a material media compatibility, reliability, or performance gap, run the bounded MLT/GES comparison in the convergence audit before considering a runtime replacement. |
| **E. Export a useful result** | Easy output presets and destinations, explicit format/codec capabilities, progress/cancel/error recovery, valid audio/video output, and repeatable packaged export. Legacy: 8F, 9D, 13 and release requirements. | Same typed project/render semantics as preview; FFmpeg provenance and redistribution review per target. Verify files in independent media probes/players and compare output behavior across desktop/mobile. |
| **F. Add focused AI assistance** | Transcript/caption proposals, translation, scene/silence analysis, selected generative assets, voice workflows, provenance, and review-before-apply. Legacy: 10A–10H, 12A–12F, 14A–14G. | Implement only after corresponding human edit/import/export paths work. Provider-independent tasks; credentials stay outside project/CLI; explicit consent, costs, network/data flow and model licenses. Outputs remain editable proposals/assets applied through normal commands. Use existing providers/services where useful; do not bundle models by default. |
| **G. Make the product extensible and contributable** | Stable documented command/API contracts, useful import/export interoperability (prioritize SRT/WebVTT captions against the `subtitler` integration proof), declarative templates/motion/assets, accessible developer docs, reproducible builds, community contribution path, and only then a justified plugin or asset distribution model. Legacy: 11A–11F, 15A–15D, 16A–16G and 4F. | Favor formats and upstream contributions over a marketplace/backend. Preserve no-code-execution defaults and explicit permissions. Do not build executable plugins, OpenFX, WebMotion, accounts, or hosted registries without a demonstrated user need and security/license acceptance. The parser's SRT/VTT tests and exact-time boundary proof pass; upstream PR [#8](https://github.com/subtitle-rs/subtitler/pull/8) proposes removing 22 active package nodes from that library-only feature set while preserving default CLI behavior. OR still needs to validate cue/style loss, undo behavior, and shipped-package size before depending on it. |
| **H. Cross-platform release quality** | Consistent design/terminology and material feature parity on macOS, Windows, Linux, Android; accessibility; signed/notarized production distribution; install/upgrade/uninstall; performance and reliability. Legacy: 7H–9E and release requirements. | Platform-specific file pickers, permissions, packaging, window integration and hardware APIs may differ. Editing semantics and recovery must not. Maintain an explicit platform matrix and report unverified cells honestly. iOS is future until supported. |
| **I. Code-to-illustration video** | Turn a code artifact into an illustrated explainer: code ingestion and region selection, tokenisation and syntax highlighting, a versioned declarative `.ormotion.json` MotionScene, deterministic materialisation to persistent media, callouts and arrows, captions and narration, and reusable templates. Track B — an external generative-video provider — is a later, costed, licence-reviewed extension. | Depends on the [ADR 0007](adr/0007-declarative-motion-scenes-and-procedural-isolation.md) contract, the `or_render` text and layer path, export, and captions. Reuse the deterministic bundled-font rasteriser and the existing Text/Caption clip content; do not add a second text engine. Materialisation registers output through normal media/timeline commands, and validation, preview, render and materialisation make zero provider calls. Code and its highlight theme are untrusted input, so bound the parser and never execute the sample. |

The current Android app floor is API 26 (Android 8.0), matching CPAL's AAudio
backend. This intentionally excludes older Android releases; the Android
package and native linker both target API 26.

## Dependency graph

```text
safe project open/recovery ──┐
                             ├─> import/media permissions ─> edit interactions
packaged runtime foundation ─┘                                 │
                                                              ├─> preview/audio/effects/text
                                                              │                  │
                                                              └──────────────────┴─> export
human edit paths ─> reviewable AI proposals ─> apply through shared commands
all user paths ─> platform parity, accessibility, performance, release quality
```

The MLT/GES comparison is a bounded, baseline-triggered architecture
experiment, not a prerequisite for all product work. Do not let it block
project usability, Android import, or finishing the existing preview/export
journey unless results establish a real engine migration dependency.

## Current product status and highest-value gaps

## OR-BUILD Phase 0 status (work in progress, 2026-10-18)

Phase 0 exists so the later phases can be measured. Its state, with the evidence
class each claim actually has:

- **Android lane floor and daemon reaping — implemented (VERIFIED_SOURCE, hermetic
  tests).** `scripts/run_android_lane.sh` derives its `MemAvailable` floor from the
  selected steps instead of a flat 7 GiB (guest 6144 + emulator 1024 + host margin
  1024 for a guest run; 5120 for an apk-only run), records the derivation in
  `android.json`, and stops the Gradle/Kotlin daemons its own apk step started
  before the emulator step — attributable by the wrapper's Gradle version, with a
  foreign daemon reported and left alone. `scripts/test_android_lane.py` covers
  both rules hermetically (20 checks).
- **The APK "hang" — diagnosed, not a product defect (VERIFIED_BEHAVIORAL, local
  developer evidence).** The build was fetching a 119,914,309-byte Flutter engine
  artifact at ~0.4 MB/s with no Gradle HTTP timeout; it completed in 217 s and
  produced a 112,532,328-byte APK. Gradle HTTP timeouts are now bounded. Facts and
  measurements are in [AGENT-AUTHORITY.md](AGENT-AUTHORITY.md).
- **Android lane — first guest result (VERIFIED_BEHAVIORAL, local developer
  evidence, not acceptance).** At source `7b473f7`, `bash
  scripts/run_local_lane.sh android` with `OR_ANDROID_GUEST_RAM_MB=3072` reported:
  `preflight` PASS (derived floor 5120 MiB = guest 3072 + emulator 1024 + host
  margin 1024), `deps` PASS, `apk` PASS 67.3 s (six APKs, hashes in the report),
  `emulator` PASS 20.1 s on `an3-api36` / `emulator-5580` / SwiftShader, and
  `preview` PASS 30.7 s — the packaged FFmpeg libraries loaded on the guest
  (`not_run` is empty, so the link probe ran) and the bridge-diagnostics and
  preview-surface cases passed. `saf` is BLOCKED: it exceeded its 2700 s bound
  (exit 124) after the export stage, which is a local bound, not an accepted
  result — hosted `platform-verification` remains the SAF authority. The run is
  DIRTY (two uncommitted files were present), so the lane reports no attributable
  PASS. Each fix that made this run possible is committed with its own hermetic
  guard: the emulator port, the install-root FFmpeg link, the RUSTFLAGS link,
  jniLibs staging, the absolute `RUNNER_TEMP`, and one offline lockfile-enforced
  dependency resolution.
- **`scripts/or_ui.py` — implemented and passing on Linux (VERIFIED_SEMANTIC +
  VERIFIED_BEHAVIORAL + VERIFIED_VISUAL, local developer evidence).** The full
  editor journey (50 steps) passes end to end on the running app with semantic
  selectors only, including the rejected-import state, a keyboard duplicate, undo,
  frame step and save, a save/reopen round trip and the export dialog's cancel
  path. The screen inventory finds five destinations at 1280x800 and 320x440; the
  negative control fails loudly with `not_found`. The Android target is
  implemented but has not yet produced a guest transcript: it needs a booted guest
  and `ui.apk`, which the case-APK builder now produces.
- **Performance budget — 16 measured rows (VERIFIED_BEHAVIORAL, local developer
  evidence).** `scripts/bench_quality.py` now measures and enforces CLI cold start,
  the FFmpeg export lane and its throughput, ten canonical-command and project-file
  rows on a 200-clip project, a 640x360 offscreen render frame plus readback on the
  host's Intel UHD 770 through Vulkan, and the render-frame p95 while 40 CPU-bound
  opportunistic jobs saturate the job pool. All passed on 2026-10-18; a genuine
  injected 500 ms regression trips the CLI row (exit 1). **Still unmeasured:** a
  *presented* viewer frame (vsync/compositor/swapchain), and release-build speed.
- **Shared native UI lock — hardened after a measured blockage
  (VERIFIED_SOURCE, hermetic tests).** While a sibling project's build ran, the
  per-UID lock was held by that build's processes and every OR UI lane failed with
  `[Errno 11] Resource temporarily unavailable`. The preflight now names the
  contention as `BLOCKED` with the settle command, opens the descriptor
  non-inheritable and always closes it.
- **Local gate state at `0d18014` (developer evidence).** `bash
  scripts/check-repo.sh` PASS; `bash scripts/acceptance.sh`
  `PROJECT_ACCEPTANCE=PASS` with seven correctly named `UNVERIFIED` lanes (all
  hosted-CI authority: platform-verification, release-gate, packaging, or_render,
  or_audio, or_ai, or_whisper_provider). The Flutter widget suite is 102 tests,
  the performance budget is sixteen measured rows, and the bounded Linux UI
  preflight passes. The Linux lane reports hygiene, flutter, preflight and bench
  PASS.
- **Compact-layout status (VERIFIED_BEHAVIORAL on this host).**
  `scripts/or_ui.py --viewport 320x440 screens` walks all five screens and the
  compact *editor* journey now runs end to end (create, rejected import, import,
  track, media sheet, insert, clip assertion, duplicate, undo) with no render
  exception. Four defects found and fixed with permanent widget guards: header
  overflow 44 px, dropdown overflow 46 px, a message stealing the dock tap, and
  the viewer strip overflowing once a clip gave the preview a range (with
  `AspectRatio` inside a tight-height `Expanded` as its second cause). The
  compact editor's vertical allocation leaves the preview surface ~1 px tall at
  320x440 — clean, not clipped, and recorded as a limitation with its number.
  The same journey at 390x844 is still **UNVERIFIED**: `or_ui.py` takes one
  `--viewport` per invocation and the taller compact run has not been performed.
- **Phase 1.1 — overwrite edit mode: implemented at the command layer
  (VERIFIED_SOURCE + VERIFIED_BEHAVIORAL at the Rust command boundary).**
  `timeline.clip.insert` now takes an explicit `mode` (`insert` | `overwrite`,
  default `insert`) and, in overwrite, replaces the target range in one
  transaction: a fully covered clip is removed, a clip crossed at its head is
  shortened, a clip crossed at its tail is moved with its source advanced, and a
  clip that strictly contains the new span is split into a head that keeps the
  original id and a tail that takes the caller's `overwrite_split_clip_id`.
  Without that id the command refuses with
  `TimelineOverwriteSplitRequiresClipId` rather than dropping the tail.
  - The overwrite error code crosses the app bridge as a wire code
    (`TIMELINE_OVERWRITE_SPLIT_REQUIRES_CLIP_ID`) in
    `crates/or_app_bridge/src/api/project.rs`. Missing that arm broke the app
    bridge's exhaustive `match` and made the Linux lane's flutter step fail with
    `E0004`; the earlier note that the new error code had no client surface was
    wrong and is corrected here. `cargo test -p or_app_bridge` is 24 passing.
  - The history entry is the single change `ProjectChange::TimelineClipsOverwritten
    { track_id, before, after }`, whose staged form is a whole-track
    `ReplaceTimelineTrack` in both directions — the constraint recorded last round
    (`ChangeSet::stage_history_change` accepts exactly one change) is what that
    variant exists for. The change is a serialized Rust surface only: no Dart
    model, IPC payload or bridge signature exposes `ProjectChange`, so this needed
    no regeneration.
  - `OPERATION_SCHEMA_VERSION` is **not** bumped: `mode` and
    `overwrite_split_clip_id` are optional with defaults, so a caller written
    before the change keeps its exact meaning, and a schema bump would invalidate
    every unrelated command's descriptors for no behavioural gain.
  - Six tests in `crates/or_core/src/application.rs` cover covered-removal,
    head-cross, tail-cross, the refusal, the split with undo restoring the
    unsplit clip, and the unchanged default mode. `cargo test -p or_core --lib`
    is 323 passing; `cargo fmt` and `cargo clippy -p or_core --all-targets` are
    clean.
  - `or timeline insert-clip` takes `--mode insert|overwrite` and
    `--overwrite-split-id CLIP_ID`; an unknown mode is a usage error, and the
    flags are documented in `docs/TECHNICAL_PLAN.md`. The CLI has no automated
    overwrite test yet — the domain tests cover the semantics.
  - **The binding generator works here when it gets a writable `HOME`
    (2026-10-19, second attempt).** `HOME=/tmp/frb-home flutter_rust_bridge_codegen
    generate` from `packages/or_app_bridge` completed with no analysis-server
    crash; the regeneration diff is exactly the two new optional parameters plus
    import ordering, which `cargo fmt -p or_app_bridge` normalizes. The bridge's
    `insert_timeline_clip` now takes `mode: Option<String>` and
    `overwrite_split_clip_id: Option<String>` and builds the envelope through
    `CommandEnvelope::insert_timeline_clip_with_mode`, so the generated Dart
    signature carries `String? mode, String? overwriteSplitClipId` as *optional*
    named parameters and every existing Dart caller still compiles
    (`flutter analyze` clean, `cargo test -p or_app_bridge` 24 passing). The Dart
    presentation is therefore the only remaining step.
  - **First attempt, kept because the failure mode matters (2026-10-19).** `flutter_rust_bridge_codegen` 2.13.0 was installed here and
    `generate` run from `packages/or_app_bridge`; it printed `Done!` but also
    `Bad state: The analysis server crashed unexpectedly`, and it had rewritten
    `crates/or_app_bridge/src/frb_generated.rs`, `lib/src/rust/api/project.dart`
    and `lib/src/rust/frb_generated.dart`. A generated surface that half-succeeded
    cannot be trusted, so the bridge signature change and all generated files were
    reverted; only the or_core side landed: a public `TimelineEditMode` (with
    `as_wire`/`parse_wire`) and `CommandEnvelope::insert_timeline_clip_with_mode`,
    which is what the bridge will call once regeneration is trustworthy. Next
    attempt: run `generate` with a writable `HOME` and a warm Dart analysis
    server, verify `git diff` touches only the expected files plus the new
    parameters, then build the app before committing anything.
  - **Failed attempt to write that CLI guard (2026-10-19), recorded so it is not
    repeated blind:** a test modelled on `headless_marker_cli_round_trips_...`
    (same `TestDirectory`, `media_path`, `probe_stub`, `json_success_with_probe`
    calls, in the same order, with the same media name) failed at `media add` with
    `PROBE_FAILED / ffprobe exited unsuccessfully without a diagnostic`, while the
    sibling test passes in the same environment. The fixture is therefore not the
    difference; the next attempt should first diff the two tests' environment
    (`json_success_with_probe` vs `json_success`, and whether `probe_stub` must be
    built before `create_project`), then write the guard. The test was removed
    rather than left red.
  - **Journey evidence (2026-10-19).**
    `python3 scripts/or_ui.py run scripts/ui_scenarios/overwrite-insert.json`
    passes on the desktop layout: it creates a project, imports the fixture, adds an
    audio track, inserts a 1 s clip through the dialog, then inserts again at 1/2 s
    with the dialog's Edit Mode set to Overwrite and reaches
    `Audio track A1, 2 clips`. Insert mode refuses that overlap, so reaching two
    clips is only possible because overwrite trimmed the first clip and placed the
    new one — a behavioural result read from the app's own track summary, which is
    the strongest evidence this harness can produce without reading canonical state
    directly. Transcript and screenshots are in
    `apps/or_app/build/ui-harness/overwrite/`.
  - **Reachable by a user (2026-10-19, this revision).** The insert dialog carries
    an `Edit Mode` selector (`timeline-insert-mode`, Insert | Overwrite) whose value
    travels with the command through the dialog callback, `app_shell` and the
    gateway into the bridge; it is presentation state and is never written into the
    project. Both presentations reach it: the compact sheet opens the same dialog.
    The drag-and-drop quick insert uses `insert` explicitly, because it has no
    dialog to ask in. Guarded by `flutter analyze` clean, the widget suite (103
    passing), and both editing journeys passing through `scripts/or_ui.py` at
    1280x800 and 320x440 after the change.
  - **Measured (2026-10-19).** `QUALITY.md` carries a
    `command.overwrite_insert` row: replacing a covered clip in one transaction is
    p50 0.260 ms / p95 0.286 ms against a 6.0 ms p95 budget on the 200-clip project,
    so the mode pays the delete and the insert together and is far inside the
    interactive bar. The lane prints seventeen rows and the budget is PASS.
  - **Still not done, and the reason the capability is not finished:** the
    strict-containment case has no UI field for `overwrite_split_clip_id`, so a
    user who lands a clip strictly inside another gets the command's refusal
    message instead of a way to finish the edit; and there is no `or_ui.py`
    journey step that drives an overwrite end to end,
    the timeline-toolbar mode toggle, a measurement row in `QUALITY.md`, and the
    `or_ui.py` journey step that drives an overwrite. The command is therefore
    usable by a client that already sends `mode`, and not yet reachable by a
    pointer or keyboard user.
- **Phase 1.2 — ripple trim: implemented at the command layer (VERIFIED_SOURCE).**
  `timeline.clip.trim` takes an opt-in `ripple` flag; absent or false keeps the
  plain trim meaning exactly, and a test pins that because silently moving every
  later clip would be the worst failure mode here. With `ripple: true` an **end**
  trim also shifts every later clip on that track by the delta, so shortening
  closes the gap and lengthening opens one; clips before the trimmed one never
  move, and `validate_track_states` proves the shifted list is still ordered.
  **Start-edge ripple is refused with `InvalidArguments`**, because ripple-in has
  two defensible meanings and choosing one silently is a product decision rather
  than a bug fix; a test asserts the refusal and that the clip is untouched.
  - The history entry is the single change
    `ProjectChange::TimelineClipsRippleTrimmed { track_id, before, after }`, which
    shares the whole-track staged form with the overwrite variant — the same
    one-change constraint that shaped Phase 1.1. `ProjectChange` is Rust-only and
    no new error code was introduced, so the bridge and wire surface are unchanged.
  - Six tests: later clips shift by the same delta with one revision and one undo
    restoring every shifted clip, lengthening opens the gap, the default leaves
    later clips alone, a ripple to the current edge is a no-op that moves nothing,
    an invalid target is refused with the track untouched, and start-edge ripple is
    refused. `cargo test -p or_core --lib` is 329 passing; `cargo fmt` and
    `cargo clippy -p or_core --all-targets` clean.
  - One thing learned while testing: an end trim *past* the current end is a
    lengthening and is legal (the existing trim allows it), so the refusal case is
    a target at or before the clip's own start, not one beyond its end.
  - **Measured (2026-10-19).** `QUALITY.md` carries a `command.ripple_trim` row:
    shortening the first clip of the 200-clip project and pulling the other 199 with
    it is p50 0.295 ms / p95 0.312 ms against a 6.0 ms p95 budget, so shifting almost
    two hundred clips costs about twice a single-clip trim. The lane prints eighteen
    rows and the budget is PASS. Writing it also taught the boundary: 40 iterations
    of 1/512 s stay inside the first clip's 1/4 s span, because a ripple whose edge
    reaches the clip's own start is correctly refused.
  - **Reachable from the app and the CLI (2026-10-19).**
    `or timeline trim-clip` takes `--ripple`; the app bridge passes an optional
    `ripple` through `trim_timeline_clip_with_ripple` (regenerated bindings, no
    other generated code moved); the trim dialog carries a `Ripple later clips`
    checkbox (`timeline-trim-ripple`) that is enabled only for an end edge and is
    disabled with the reason shown for a start edge. It is presentation state and
    never written into the project; a drag along the timeline performs a plain trim
    because there is no dialog to ask in.
  - **Journey evidence, and its limit (2026-10-19).**
    `scripts/or_ui.py run scripts/ui_scenarios/ripple-trim.json` passes 51 steps: it
    seeds two adjacent clips through the insert dialog, trims the first clip's end
    with ripple off, then trims it again with the checkbox set. What that proves is
    the **path** — the dialog choice reaches the command through the real app and
    the app accepts the result. What it does *not* prove is the positional
    difference, because the only timeline state a scenario can read is the track
    summary (`Audio track A1, 2 clips`), which carries a count and not clip times.
    The positional behaviour rests on the six domain tests; giving the scenario a
    positional assertion needs either a clip-range label in the semantics tree or a
    runner op that reads one, and that is the recorded next step for this guard.
  - **Not done:** the CLI has no automated ripple test (its first attempt for
    overwrite hit the recorded fixture problem), and no journey asserts positions.
- **Phase 1.3 — slip: command implemented (VERIFIED_SOURCE); slide and roll not
  started.** `timeline.clip.slip` takes a `delta` and moves the clip's source
  window by it while the clip's timeline start and duration stay exactly where they
  were, so a slip never moves a cut or a neighbour. It reuses
  `trim_content_start` for the source arithmetic and `validate_track_states` for
  the bounds, so a slip past the start or end of the media is refused by the same
  validation every other edit uses instead of leaving a clip pointing at material
  that does not exist. The change is the existing single-clip
  `ProjectChange::TimelineClipUpdated`, so no new variant, no bridge work and no
  wire change were needed. Four tests: the cut does not move and the source window
  does (with one revision and undo restoring it), a negative delta takes earlier
  material, a slip past the start of the source is refused with the clip untouched,
  and a zero delta is a no-op. `cargo test -p or_core --lib` is 333 passing and the
  frozen command catalog was updated with its count.
  **Reachable from the CLI and the app (2026-10-19).**
  `or timeline slip-clip --clip CLIP_ID --delta NUM/DEN`; the app bridge exposes
  `slip_timeline_clip` (regenerated bindings, nothing else moved); and the clip
  dialog has a `Slip` action (`timeline-slip-<clip id>`) that opens a small dialog
  with a `Source delta` field (`timeline-slip-delta`) and a confirm
  (`timeline-confirm-slip`), enabled only for a media clip on an unlocked track,
  because a slip is meaningless for text or caption content. Both presentations
  reach it, since the compact layout opens the same clip dialog.
  **Not done:** no journey step drives a slip through the app, no latency row
  measures it, and slide and roll are not started.
- **Phase 1.3 roll: command implemented (VERIFIED_SOURCE); slide not started.**
  `timeline.clip.roll` moves the boundary between two adjacent clips: the left
  clip's end and the right clip's start move by the same `delta`, and the right
  clip's source advances with its start, so neither clip moves as a whole. The two
  clips must be exactly adjacent — a gap means there is no shared boundary to roll
  and the command refuses rather than inventing one — and both must keep a positive
  duration with their source inside the media, which `validate_track_states` proves
  rather than trusting the arithmetic above it.
  - The history entry is the single change `ProjectChange::TimelineClipsRolled
    { track_id, before, after }`, sharing the whole-track staged form with the
    overwrite and ripple variants. `ProjectChange` is Rust-only, so no bridge or
    wire work was needed; the frozen command catalog and its count went to 25.
  - Four tests: the boundary moves with both source windows and one undo restoring
    both sides, a gap between the clips is refused, rolling past either clip's
    extent is refused with the track untouched, and a zero delta is a no-op.
    `cargo test -p or_core --lib` is 337 passing.
  - **Correcting an earlier report:** rounds 13–17 recorded `cargo clippy` as clean
    on the strength of a grep for `unused`/`error` only. The full lint had one real
    finding — the ripple pre-state helper I wrote in round 13 collapsed three nested
    `if let`s and **silently skipped a failed subtraction**, which would have put a
    wrong pre-state into a history entry and made undo restore something the project
    never contained. It now returns a `Result` and propagates the failure, and
    clippy is clean across all targets with no narrow grep.
  - **Not done:** no CLI, bridge or UI surface sends a roll, no journey step and no
    latency row; **slide is not started**, and it is the last of the slip/slide/roll
    group.
- **Phase 1.3 slide: command implemented (VERIFIED_SOURCE). The slip/slide/roll
  group is complete at the command layer.** `timeline.clip.slide` moves a clip in
  time and lets its neighbours absorb the move: the clip keeps its duration and its
  source, the clip before it gives up or gains the same amount at its end, and the
  clip after it does the same at its start with its source advancing — so a slide
  never opens a gap or an overlap. It requires a neighbour that exactly touches it
  on both sides (a track-edge clip or a gap is refused, because there is nothing to
  borrow from) and both neighbours must keep a positive duration with their source
  inside the media, which `validate_track_states` proves.
  - History is the single change `ProjectChange::TimelineClipsSlid { track_id,
    before, after }`, sharing the whole-track staged form with the overwrite, ripple
    and roll variants. `ProjectChange` is Rust-only, so no bridge or wire work; the
    frozen command catalog and its count went to 26.
  - Six tests, and one of them corrected a wrong expectation of mine rather than the
    code: sliding a clip *earlier* needs the next clip to have earlier source
    material to give, so the same move is refused when that clip starts at the
    beginning of its own source and succeeds — with the source moving back with the
    start — when it does not. Both cases are now pinned.
  - **Measured (2026-10-20).** Each has a `QUALITY.md` row and a budget threshold
    on the tiled 200-clip project: `command.slip` p50 0.131 / p95 0.136 ms (budget
    4.0), `command.roll` p50 0.229 / p95 0.238 ms (budget 4.0), `command.slide`
    p50 0.271 / p95 0.291 ms (budget 5.0). Slide costs the most of the three because
    it rewrites the moving clip and both neighbours' source ranges. The lane prints
    twenty-two rows and the budget is PASS. Writing them needed one correction:
    the trim row trims the first forty clips' starts and never undoes them, so the
    tiling only still holds past that range and the three rows measure a clip well
    beyond it.
  - **All three are reachable (2026-10-20).** `or timeline slip-clip`,
    `roll-clip` and `slide-clip` each take `--clip` and `--delta`; the bridge
    exposes `slip_timeline_clip`, `roll_timeline_clip` and `slide_timeline_clip`
    with regenerated bindings; and the clip dialog carries Slip, Roll and Slide
    actions (`timeline-slip-<id>`, `timeline-roll-<id>`, `timeline-slide-<id>`),
    each opening a delta dialog whose confirm key names the operation
    (`timeline-confirm-slip`, `timeline-confirm-roll`, `timeline-confirm-slide`).
    Roll and Slide share one dialog because their input is identical: a signed
    delta. All three are enabled only for a media clip on an unlocked track, and
    both presentations reach them because the compact layout opens the same clip
    dialog.
  - **Journey evidence with positional assertions (2026-10-20).**
    `scripts/or_ui.py run scripts/ui_scenarios/slip-roll-slide.json` passes 56
    steps: it seeds three adjacent quarter-second clips, slips the middle one and
    asserts `spans 1/4 to 1/2, source 1/8` (the span did not move; only the source
    window did, which is the defining property of a slip), rolls its right edge and
    asserts `spans 1/4 to 5/8`, then slides it a sixteenth and asserts
    `spans 5/16 to 11/16`. The runner gained the ability to assert a span because
    the clip's semantics label now carries it — a real accessibility improvement,
    since a screen reader announces the edit instead of only the clip's name.
    Writing it also corrected two of my expectations rather than the code: a roll
    leaves the *other* clip's span alone, and a slide by an eighth was correctly
    refused because it left the next clip no duration.
- **Phase 1.4 track reordering: command implemented (VERIFIED_SOURCE).**
  `timeline.track.move` takes a track and a destination index; the moved track keeps
  its id, kind, state and clips, and the tracks around it shift by one. Order is
  presentation-visible but canonical — it decides which track a client reaches by
  index and which track is "the first video track" everywhere — so the move goes
  through the command path and is undoable like any other edit.
  - It needed a new `StagedHistoryChange::MoveTimelineTrack` and its
    `ProjectChange::TimelineTrackMoved` arm, plus `Timeline::move_track_for_command`
    and `ProjectDocument::move_timeline_track_for_command`. A move is its own
    inverse, and getting that wrong is easy: the first attempt returned the swapped
    pair from the history arm, which made **redo fail with `HistoryConflict`** while
    undo passed. The arm now applies the direction and records the pair as given,
    and the test exercises undo *and* redo.
  - `ProjectChange` is Rust-only and no new error code was introduced, so no bridge
    work; the frozen catalog and its count went to 27.
  - Three tests: a reorder moves the track and leaves its clips alone with one
    revision and undo restoring the order (and redo re-applying it), moving a track
    to its own index is a no-op, and an out-of-range destination or unknown track is
    refused with the order untouched. `cargo test -p or_core --lib` is 346 passing,
    with `cargo fmt --check` and `cargo clippy --all-targets` clean.
  - **Reachable from the CLI and the app (2026-10-20).** `or timeline move-track
    --id TRACK_ID --to INDEX`; the bridge exposes `move_timeline_track` with
    regenerated bindings; and each track header carries a move menu
    (`timeline-track-move-<track id>`) whose items are Move up and Move down,
    disabled at the ends of the track list and while busy. The header's semantics
    label also gained `position N of M`, so a track's place in the list is
    announced and assertable.
  - **Two things the surfaces cost, both caught by guards rather than review.** Two
    icon buttons overflowed the 272 px compact header (the compact guards failed),
    so the two moves became one menu control; and that control then squeezed the
    desktop clip count into an ellipsis (the count-readability guard failed), so the
    desktop header width grew by the control's footprint with the reason recorded
    at the constant. `cargo`-style reasoning would not have found either.
  - **Journey evidence, and the fix that unblocked it (2026-10-20).**
    `scripts/or_ui.py run scripts/ui_scenarios/track-reorder.json` passes: it creates
    two audio tracks, gives the second one a clip so the two are distinguishable by
    clip count, then moves that track up through its header menu and asserts that
    the app now renders it as `Audio track A1, 1 clip` — a track's displayed name
    follows its position, so the assertion is the order change rather than an
    accepted command.
  - The control needed `container: true` on its semantics wrapper. The track header
    is a semantics container, so without that the menu's label merged into the
    track's and no selector could reach it — which is also what a screen reader saw:
    one long undifferentiated track node. The earlier attempt was removed red rather
    than committed, and the harness also gained the `index` narrowing its `act` path
    was missing.
  - **Measured (2026-10-20).** `command.track_move` is p50 0.002 / p95 0.003 ms
    against a 2.0 ms budget — it costs the track count, not the clip count, which is
    why it sits two orders of magnitude below the clip-level rows. The budget prints
    twenty-three rows and is PASS.
- **Phase 1.5 clip speed/rate: the model forbids it today, and the change is sized
  (2026-10-20). No code from this gap has landed; what follows is verified fact and
  a decision request.**
  - `validate_clip_content` refuses a media clip whose source duration differs from
    its timeline duration (`source_duration != clip.timeline_duration` →
    `TimelineValidationError`), so **speed is not expressible at all** in the current
    canonical model. A test now pins that invariant
    (`a_media_clip_cannot_yet_play_at_a_different_rate`): one second of source over
    half a second of timeline is refused, and the project is left untouched. The
    test exists so the rate change cannot silently relax the rule without replacing
    it — a clip that referenced more source than it shows would be a real defect.
  - The preview maps timeline time to source time 1:1
    (`source_time = clip.source_start + (time - clip.timeline_start)`, in
    `crates/or_app_bridge/src/preview.rs`), and the export path has its own mapping.
    A rate that the model accepts but the mapping ignores would be a capability that
    persists a field and changes nothing a viewer sees, which is why the mapping is
    part of the change and not a follow-up.
  - **Correction to my own earlier reasoning:** I assumed the `.orproj` format was a
    protected specification needing owner authorisation to extend.
    [PROJECT_FORMAT.md](PROJECT_FORMAT.md) says the opposite — OR is pre-MVP, the
    format "changes with the product", and the guarantees are that every version ever
    written stays readable through explicit migrations and that an unknown future
    version is rejected. So a v8 with a `playback_rate` and a migration is an
    ordinary product change, not a control-plane one, and no owner decision is needed
    to start it.
  - **Sized, so the owner can choose the shape:** the field lives on
    `ClipContent::Media` (18 construction and match sites across five files), with
    validation as `source_duration == timeline_duration * rate`, a 1/1 default so v7
    files migrate by opening, a schema bump to 8 with the migration test, the preview
    and export mappings, then the usual command, CLI, bridge, both presentations, a
    journey and a latency row. That is two to three rounds at this session's rate.
  - **The alternative on the same critical path is Phase 4 (noise filtering)**, which
    is a deliberate FFmpeg build-configuration change and touches no project format;
    it is self-contained and could complete in the time Phase 1.5's model work alone
    would take.
- **Phase 4 noise filtering: the build configuration and its measurement are in
  place; the audio path that uses it is not (2026-10-20).**
  - The packaged desktop helper now enables FFmpeg's own `afftdn` FFT noise
    reducer (`FFMPEG_HELPER_CONFIGURE_ARGS` in
    `.github/workflows/platform-verification.yml`). It is a native libavfilter
    component under the unchanged LGPL-2.1-or-later posture, links no external
    library, and is recorded in [SECURITY_LICENSING.md](SECURITY_LICENSING.md)
    beside the existing helper additions.
  - `scripts/measure_denoise.py` proves the capability by measurement rather than by
    a capability listing, and each of the three platform jobs now runs it after
    building the helper. On this host (FFmpeg 7.1.5) `afftdn=nf=-21:nr=12` dropped
    the noise floor 11.50 dB while the signal fell 4.90 dB: 6.60 dB of
    discrimination. Numbers, thresholds and the reason the check bounds the ratio
    rather than the absolute signal loss are in [QUALITY.md](QUALITY.md).
  - The measurement also produced the design constraint the product half has to
    honour: `afftdn`'s noise floor is a parameter, a value below the recording's real
    floor does nothing and a value above it eats the audio. The script therefore
    measures the fixture's own floor and passes it in; a real audio path must derive
    the same figure per clip instead of hard-coding it.
  - **The first interface exists (2026-10-20).** `or media clean-audio --source
    PATH --out PATH [--noise-floor-db N] [--reduction N]` reads one media file and
    writes a cleaned audio file through the packaged helper: no project is opened and
    none changes, so the result is ordinary media a user can import, compare, and
    discard, and noise reduction stays out of the project format. The core function
    (`or_core::clean_audio`) resolves the helper the same way probes do, is bounded
    (900 s, killed on expiry, partial output removed), validates its parameters, and
    builds its arguments through a pure function so the chain is asserted without
    running a process. Five core tests plus a CLI-level measurement.
  - **Measured through the product (2026-10-20).**
    `scripts/measure_denoise.py --bin <helper> --cli <or binary>` drives the command
    on the synthetic fixture and reports both windows; the command and a direct
    helper call give identical numbers (11.50 dB floor drop, 4.90 dB signal change,
    6.60 dB discrimination), which is what shows the command uses the chain it claims.
  - **Not done, and why the capability is still not finished:** there is no desktop
    or compact surface, so a pointer or keyboard user cannot reach it; no import of
    the result as a new media item in one step; no project-aware path that derives
    the noise floor from the clip's own audio instead of asking the caller; and no
    journey. Those are the remaining half of Phase 4, and this gap stays
    `UNVERIFIED` at the interface bar until the GUI surface exists.
  - **Shape decided for the product half, so the next attempt does not relitigate
    it.** The clean-up is a **helper-spawned derived artifact**, not a clip property:
    `or_core` already spawns the packaged `ffmpeg` for probes, proxies and waveforms,
    and the export encoder is a *library* path (`ffmpeg_the_third`), where denoising
    would mean building and driving a libavfilter graph from Rust. A clip property
    would also be a project-format change, which this gap does not need. So the
    command spawns the helper with `-af afftdn=nf=<measured>:nr=<n>`, writes a cleaned
    WAV into the project's artifact area, and the result is imported as ordinary
    media — which means the user can compare, undo by deleting, and export normally.
    Its acceptance is the measurement this round already built: the exported or
    imported file's noise floor must drop while the signal survives, and the noise
    floor must be derived per clip rather than hard-coded.
- **Phase 1 (complete basic editing) is not started in this revision.** The
  enumerated gaps stand exactly as the prompt lists them: overwrite edit mode,
  ripple trim, slip/slide/roll, track reordering, clip speed/rate, clip linking
  and grouping, keyframes. The compact insert defect above is the first Phase 1
  task because it blocks the compact editing journey that every later gap has to
  clear.
- **Next action for Phase 0:** drive the Android guest through
  `bash scripts/run_local_lane.sh android` (or `python3 scripts/or_ui.py --target
  android run scripts/ui_scenarios/editor-journey.json` against a booted guest)
  and either record a guest result naming the AVD, serial, GPU mode, probes, APK
  hash, every test result and the guest/host health samples, or record `BLOCKED`
  with the measured reason.

- **Verified baseline:** exact-SHA run [38087624707](https://github.com/huou07/Opencut-Reinforced/actions/runs/38087624707) passed the required Rust, Flutter, descriptor, and packaged platform checks on `1dade2093a739322a894764f453700be0f9009fb`. Prior packaged desktop journeys exercised representative 1080p H.264/AAC import, preview, save/reopen, relink, and export; exact run reports and limitations are in the [history archive](evidence/PRODUCT_ROADMAP_HISTORY.md).
- **Packaged edit/export:** exact-SHA run [38162539040](https://github.com/huou07/Opencut-Reinforced/actions/runs/38162539040) on `c3bcd95030d7fbcd7905a5a28d947e375f46497c` passed Linux, macOS, and Windows real-media journeys. Exact repeat on [38165543230](https://github.com/huou07/Opencut-Reinforced/actions/runs/38165543230) on `81491556f6035a309cee129f25b7e762bd1dd11c` passed Windows again. Both runs imported 1080p H.264/AAC, saved/reopened/relinked, exported 30 seconds of VP9/Opus, and independently decoded 720 video plus 1,501 audio frames. Hosted export times were Linux 47.299 s, macOS 57.394 s, and Windows 182.114 s then 169.702 s; monitor the Windows result through dogfood before changing quality/speed settings. Run 38165543230 also had a macOS lifecycle-suite timeout with no assertion failure (the identical suite passed on 38162539040) and a transient Linux TLS failure downloading Opus from `downloads.xiph.org`. Its Android loop exposed a repository-relative script path that fails after FFmpeg changes directories; the absolute-path repair was validated by the Android job in exact-SHA run [38172368290](https://github.com/huou07/Opencut-Reinforced/actions/runs/38172368290). Details are in the [history archive](evidence/PRODUCT_ROADMAP_HISTORY.md).
- **Exact-SHA platform run:** run [38172368290](https://github.com/huou07/Opencut-Reinforced/actions/runs/38172368290) on `33f7f355a5bb1c848551babe9c4c5c11c2a65316` passed Rust, Flutter, repository hygiene, descriptor-boundary, Linux, macOS, Windows, and Android gates. The desktop packaged journeys imported, edited, saved/reopened/relinked and independently decoded 30-second VP9/Opus exports: Linux 46.883 s, macOS 54.526 s, Windows 151.311 s, each with 720 video and 1,501 audio frames. Android built all three ABIs and its real DocumentsUI SAF journey wrote valid WebM with independently decoded video/audio; its small half-second fixture proves correctness only, not representative mobile performance. Full measurements and limits are in the [history archive](evidence/PRODUCT_ROADMAP_HISTORY.md).
- **Android multi-source import:** the initial exact-SHA run [38196732759](https://github.com/huou07/Opencut-Reinforced/actions/runs/38196732759) exposed a real SAF multi-import regression; root cause, fix, and final evidence are in the [history archive](evidence/PRODUCT_ROADMAP_HISTORY.md). Platform Verification [38199486640](https://github.com/huou07/Opencut-Reinforced/actions/runs/38199486640) on `39afed57adf3a04faba842f78ac73a9d68a1bbed` passed all seven jobs. Its Android DocumentsUI journey imported H.264/AAC MP4 and audio-only MP3 in one selection, then passed captions, relink, export, process recovery, and bounded-resource checks; desktop Linux, macOS, and Windows packaged journeys also passed. The fixture establishes emulator correctness, not representative media performance. The emulator logged skipped frames and a 121 ms maximum main-draw measurement under this acceptance workload; physical-device responsiveness remains unverified, so further emulator tuning is not the next task.
- **Current product candidate:** `c329d8e26c994fb24af5f53d3f4c50e31087d1cd` stages the pinned whisper.cpp CLI and OR provider in desktop preview bundles and adds a real packaged transcription, caption review/edit/apply, save/reopen, and SRT export gate. Exact-SHA PR Platform Verification [38234509928](https://github.com/huou07/Opencut-Reinforced/actions/runs/38234509928) passed all seven required jobs, including macOS, Windows, Linux packaged journeys and Android SAF/resource acceptance. Those checks verify existing cross-platform behavior. The validation-only workflow passed its exact-SHA prepare gate on `4033f90a16c9042008e404c1d597db7eaed6c493`, built Android, and skipped all release-only jobs. Its macOS, Windows, and Linux packaged transcription gates failed; root causes and follow-up evidence are recorded in the history archive. Android transcription is unsupported and physical-device performance remains unverified.
- **Folder import — exact-SHA verified:** `a7f4bf4010c3d3283e788387839edbc6f83a0f3d` passed Platform Verification [38275213412](https://github.com/huou07/Opencut-Reinforced/actions/runs/38275213412), Repository Hygiene, and all Linux, macOS, Windows, Android, Rust, Flutter, and descriptor jobs. The clean desktop journeys and Android DocumentsUI flow passed; Android imported three selected files plus two nested files under the selected `Footage` grant, then passed preview, captions, relink, export, recovery, and descriptor/resource checks. A prior Android run timed out before the import gateway, but this run passed; its cause is still unlocalized. Emulator evidence does not establish physical-device performance. Detailed timing and scope evidence are in the [history archive](evidence/PRODUCT_ROADMAP_HISTORY.md).
- **Recent project return — exact-SHA verified:** `a3db4f63319029ea683b6d4cc749df7fdba27a11` passed Platform Verification [38328757217, attempt 2](https://github.com/huou07/Opencut-Reinforced/actions/runs/38328757217), with Repository Hygiene [38328757237](https://github.com/huou07/Opencut-Reinforced/actions/runs/38328757237). Linux, macOS, and Windows packaged journeys reopened a recent project after application restart. Android verified real project selection, the persisted recent URI, a fresh-picker SAF rebind, read-grant/path validation, and provider synchronization; its larger journey also passed multi-media import, captions, relink, process recovery, preview resource bounds, and independently decoded VP9/Opus WebM. Attempt 1's Android provider freeze and macOS launch timeout, and their exact-SHA passing rerun evidence, are detailed in the [history archive](evidence/PRODUCT_ROADMAP_HISTORY.md).
- **First-project handoff — exact-SHA verified:** `72fe14c` adds an empty-timeline Import Media action that uses the existing platform picker and permission path on desktop and compact layouts. Its first Platform Verification run exposed a close-control readiness race in the packaged test on Linux and Windows; a fresh rerun passed Windows but reproduced Linux. Commit `f5cf1c26353feaebf49e9e5ca058414ad9798809` now waits for the actual enabled, hit-testable Close control and reports its state if closing stalls. Platform Verification [38353664539](https://github.com/huou07/Opencut-Reinforced/actions/runs/38353664539) passed all seven jobs, including the Linux packaged journey. The UI change preserves project and import semantics.
- **Most recent interaction work — exact-SHA verified:** at `4d4c6fb091b5930cb1cbed0769ba83acd1693df9`, Home no longer presents the visual-only editor shell as a usable project. At `3bf2e6e8980c093072b217ba33c6c91a67e1aa82`, Add to Timeline starts at the occupied timeline's content end. Platform Verification [38366017857](https://github.com/huou07/Opencut-Reinforced/actions/runs/38366017857) passed seven jobs for the latter SHA; its packaged desktop and Android journeys cover create/import/edit/preview/persistence/export paths.
- **Compact short-viewport editor — hosted Linux UI preflight verified:** the compact editor no longer overflows at 320x480. The reproduced 31 px overflow came from the viewer's empty placeholder, not the transport: the compact split left the surface 55.7 px tall while the icon-and-text column needed more. A 320x440 viewport additionally squeezed the fixed 34 px timeline view controls into an 8.7 px strip. Both now absorb a short viewport (the placeholder keeps its icon and exposes its message through semantics below 56 px high or 120 px wide; the timeline view controls are omitted below 60 px high). A headless regression mounts the editor at 320x480 and 320x440. Hosted Linux UI preflight [39013698516](https://github.com/huou07/Opencut-Reinforced/actions/runs/39013698516) passed the analyzer, the focused semantics tests including that regression, and the real Linux journey at exact source `c90f026bc7e0c816e0c10ecb2eb09b38d12f2493`; its machine-readable record [the claim](evidence/claims/2026-10-17-C-compact-short-viewport.json) is retained but superseded: it claimed the only later change was a dart-format-only commit, while 21 commits actually followed. Exact-HEAD Platform verification [39043596474](https://github.com/huou07/Opencut-Reinforced/actions/runs/39043596474) later passed all seven required platform jobs plus the aggregate `release-gate` at `c8b627be9e931f2b332ac1c8f2de459b4c30b22d`, and the superseding machine-readable record is [the claim](evidence/claims/2026-10-17-C-compact-short-viewport-c8b627b.json). Compact-viewport rendering on a physical mobile device remains unverified.
- **Large-library media previews — fixed, hosted Flutter verification:** a media library larger than one 50-item page dropped the thumbnail or waveform already generated for every item on an earlier page as soon as the next page loaded: `_loadMoreMedia` passed only the newly fetched items to `_queueMediaPreviews`, which treats its argument as the complete loaded set, prunes every preview outside it and never re-requests the pruned sources. The load-more path now passes the accumulated page. Canonical project state and `ProjectRevision` are untouched; only disposable Flutter presentation state changed and no command is dispatched. Platform Verification [39017187079](https://github.com/huou07/Opencut-Reinforced/actions/runs/39017187079) job *Flutter static and widget checks* ran `dart format`, `flutter analyze` and the full `flutter test` suite on the canonical Flutter 3.47.5 pin at exact source `5255a0dbd123d289c49a93e82a8d84b36a9a9c7d`, including the 55-item regression; Linux UI preflight [39017187058](https://github.com/huou07/Opencut-Reinforced/actions/runs/39017187058) also passed at that SHA. The machine-readable record is [the claim](evidence/claims/2026-10-17-B-large-library-media-previews.json). The broader offline-media and bulk-relink workflow named by work stream B remains unimplemented and unverified.
- **Current dogfood build available:** validation-only Developer Preview [38369979125](https://github.com/huou07/Opencut-Reinforced/actions/runs/38369979125) passed the exact-source gate for SHA `3bf2e6e8980c093072b217ba33c6c91a67e1aa82`, built all five app packages (four build jobs; Linux emits a tarball and a `.deb`), and passed Linux, macOS, and Windows packaged transcription, caption review/edit/apply, save/reopen, and SRT export. Artifacts expire October 20, 2026; publication and CLI packages were skipped. The packages are debug builds, not production installers.
- **Clean desktop install, upgrade, and removal — exact-SHA verified:** commit `53de6d51c6d45c7ec7ae0a2989455972118406c1` passed Platform Verification [38570870756](https://github.com/huou07/Opencut-Reinforced/actions/runs/38570870756) and Repository Hygiene [38570870938](https://github.com/huou07/Opencut-Reinforced/actions/runs/38570870938). The Linux job installed, upgraded, launched, and purged the `.deb` in Ubuntu 24.04 and Debian 13 containers while preserving a user-data sentinel. The macOS and Windows jobs extracted, installed, launched, replaced, and removed their ZIP packages from per-user application directories while preserving user data. Limits and exact job evidence are recorded in the [Linux claim](evidence/claims/2026-10-14-9E-linux-debian-install-upgrade-uninstall.json), [macOS claim](evidence/claims/2026-10-14-9E-macos-zip-install-replacement-removal.json), and [Windows claim](evidence/claims/2026-10-14-9E-windows-zip-install-replacement-removal.json).
- **Local Linux `.deb` packaged evidence:** at exact source `33b84ebe2df37280ca0a27759030c9efc6fb3dca` this workstation built the pinned libvpx 1.17.0 / libopus 1.6.1 / FFmpeg 8.1.3 runtime, built the Flutter Linux debug bundle against it, and packaged `opencut-reinforced-linux-x64-debug.deb` (31,599,552 bytes, SHA-256 `7253f9e13589da1051391595fe389fc2143d4133dbde4163feec8f5dab4b72d2`). The package was extracted into a disposable root, the installed `/opt/opencut-reinforced/or_app` ran for the full 20-second smoke window, and `scripts/run_packaged_desktop_journey.py` then used the installed package's `ffmpeg`, `ffprobe`, Rust bridge and media libraries: all four app processes passed and the export independently decoded to 720 VP9 video and 1,501 Opus audio frames at 30.008 s. This is local packaged evidence on this host (Flutter 3.47.7, not the pinned 3.47.5), not an `apt` install/upgrade/purge and not hosted acceptance; the commands, payload hashes and limits are in [the record](evidence/2026-10-18-linux-deb-local-acceptance.md).
- **One local Linux lane — implemented, native step blocked by host memory:** `bash scripts/run_local_lane.sh linux` is now the single local entry point for the hygiene gate, the Flutter widget suite, the bounded preflight and the performance budget, and it leaves one machine-readable report (`apps/or_app/build/local-lanes/linux.json`). At exact source `b4a84956883422ed51ef1fb799d6cae5c9ff18b4` the lane reported `BLOCKED` (exit 3) with `hygiene`, `flutter` (124 tests) and `bench` PASS and the preflight BLOCKED at its own 6 GiB `MemAvailable` floor; a deliberately broken source check made the same command exit 1, and the packaged steps' blocked branches were exercised with a deliberately absent pinned prefix. `bash scripts/acceptance.sh` passed at that source with 8 named UNVERIFIED lanes. Evidence class: local developer evidence only, no claim record and no `ACCEPTED`; the native journey at this source is unverified and `--packaged` was not executed, and the commands, timings, hashes and limits are in [the record](evidence/2026-10-18-local-linux-lane.md).
- **Basic editing — implemented baseline with enumerated gaps:** 27 commands and
  8 queries dispatch through the shared application path
  (`crates/or_core/src/application.rs:64`), covered by 346 core unit tests and 103
  Flutter widget tests across tracks (add, remove, state and reorder), insert
  (insert and overwrite modes), move, trim (plain and ripple), slip, slide, roll,
  split, ripple delete, clip properties, markers, captions and undo/redo. Not
  implemented: clip speed/rate change; keyframes; clip linking or grouping. Zoom, scroll and
  snap-guide rendering are presentation state, and snapping itself is the
  read-only `timeline.snap` query.
- **Automatic subtitles — desktop implemented, Android unsupported by design:**
  whisper.cpp `tiny.en` is pinned by hash and downloaded only on confirmation
  (`crates/or_ai/src/model.rs:38`), the provider runs as a sidecar
  (`crates/or_whisper_provider`), transcription yields a provenance-bound
  proposal that a user reviews, edits and applies through a revision-checked
  command, and SRT/WebVTT round-trips through `crates/or_core/src/subtitle_io.rs`.
  Packaged desktop journeys pass on macOS, Windows and Linux.
  `crates/or_app_bridge/src/transcription.rs:798-826` has no Android model-store
  arm, so `TranscriptionService::new()` returns `None` and every call reports
  `ProviderUnavailable`; Android transcription stays unsupported until that path
  and its acceptance exist.
- **Noise filtering and audio cleanup — absent, and not reachable through the
  shipped FFmpeg:** the only audio processing is clip gain, pan and linear fades
  applied in both preview and export (`crates/or_audio/src/lib.rs:428-436`,
  reached from `crates/or_app_bridge/src/preview.rs:2612`). No denoise, EQ,
  compressor, limiter or filter graph exists in code, export profiles or the
  command catalog. The pinned shared library build disables avfilter
  (`FFMPEG_COMMON_CONFIGURE_ARGS`, `.github/workflows/platform-verification.yml:33`);
  only the desktop helper executables re-enable it with six filters
  (`:51,58-63`), so the library the editor links cannot denoise as shipped.
  **Owner decision, 2026-10-18:** the route is a deliberate FFmpeg
  build-configuration change that enables the filters this capability needs,
  with the licence, packaging and per-platform consequences re-verified before
  it is enabled anywhere. Bounded Rust DSP in `or_audio`/`or_media` is not the
  chosen route. The change is phase 4 of
  [prompts/OR-BUILD.md](prompts/OR-BUILD.md) and no part of it is implemented
  yet.
- **Code-to-illustration video — absent; only enabling primitives exist:** there
  is no code ingestion, tokenisation, syntax highlighting, MotionScene schema or
  parser, scene evaluator or materialisation path. `crates/or_ai/src/protocol.rs:26`
  declares an `AiTask::GenerateMotionScene` variant with no producer or consumer.
  What exists to build on: deterministic bundled-font text rasterisation
  (`crates/or_render/src/text.rs:18-39`), first-class text and caption clip
  content (`crates/or_core/src/timeline.rs:181`), and the shared wgpu layer render
  path (`crates/or_render/src/lib.rs:602`). ADR 0007 is architecture intent, not
  implementation.
- **One local Android lane — entry point landed, guest drive unexecuted:**
  `bash scripts/run_local_lane.sh android` selects it and delegates to
  `scripts/run_android_lane.sh`, which probes `MemAvailable` against a 7 GiB
  floor, the AVD, `/dev/kvm`, `adb`, `flutter` and `OR_ANDROID_FFMPEG_PREFIX`
  before booting anything, drives the repository's guest-lifecycle and
  SAF/preview scripts, always stops the guest it started, and writes
  `apps/or_app/build/local-lanes/android.json` in the Linux report's shape.
  `scripts/test_android_lane.py` checks its usage, blocked-precondition,
  dirty-tree and no-acceptance rules without a device, inside `check-repo.sh`.
  On 2026-10-18 an earlier revision of the lane reported `BLOCKED` with its own
  measurements: `MemAvailable` 5.5 GiB, the ~2.25 GiB `GradleDaemon 8.14.3` that
  belongs to the sibling project VCE, and the unset pinned FFmpeg prefix. That
  revision could not have run even with a prefix: it never created the
  `android/.ffmpeg-install` link the bridge build hook requires, and it invoked
  the preview script with no arguments against a script that takes four. Both
  are repaired, its guest RAM default now matches the emulator script's measured
  size, and reproducible staging is `scripts/stage_android_ffmpeg.sh` (which also
  builds the guest FFmpeg link probe) with `scripts/build_android_case_apks.sh`.
  The staged prefix and probe are now built on this host. The guest drive is
  **BLOCKED by a host precondition, not by the product**: the shared Flutter SDK
  is not writable by the worker UID, so Gradle cannot create the project cache it
  needs inside `packages/flutter_tools/gradle` and fails with `The settings are
  not yet available for build.` The lane now names that path and the remedy in
  its BLOCKED reason, and
  [evidence/2026-10-18-local-android-lane.md](evidence/2026-10-18-local-android-lane.md)
  records both the discriminator that proves the cause and everything the run did
  not establish. Until write access is granted, no Android case can be driven
  locally; the hosted `android` job is unaffected. Both are phase 0 of
  [the current prompt](prompts/OR-BUILD.md).
- **Next product gap — complete the editing journey on desktop and Android:**
  close the enumerated basic-edit gaps in the order product value needs them,
  using the local Linux and Android lanes for fast iteration and hosted CI for
  acceptance. Code-to-illustration video follows once the text, materialisation
  and export paths it depends on are complete. Real-user dogfood feedback
  continues in parallel and still decides which fix matters first.

Detailed historical investigations remain in the archive for provenance. Keep new active-roadmap updates to user-visible outcomes, current limitations, exact acceptance evidence, and the next product gap.

## Traceability from the preserved roadmap

| Legacy scope | Active mapping | Treatment |
|---|---|---|
| Phases 0–4: repository, design, architecture, project/command foundation | A, C, G, H | Mostly implemented foundations; preserve their invariants and tests. |
| Phase 5: media foundation | B | Keep bounded job/cache/proxy and metadata work; prove user-facing behavior. |
| Phase 6: timeline MVP | C | Keep exact semantics; judge completeness by useful interaction, not checkpoint labels. |
| Phase 7: preview/playback | D | Keep bounded runtime and measure real packaged behavior. |
| Phase 8: desktop editing/export | A–E, H | Finish coherent user workflows and release packaging. |
| Phase 9: Android | A–E, H | Preserve 9B/9B1/9C/9D verified work; identify unsupported SAF/import/recovery gaps rather than assuming parity. |
| Phase 10: captions/transcript/AI assist | D, F | Human captions/text first; selective task-oriented assistance. |
| Phase 11: templates/assets/themes/motion | G | Keep declarative and provenance-safe; prioritize only user-valued workflows. |
| Phase 12: dubbing/voice | F | Optional, consent/privacy/licensing-gated workflows. |
| Phase 13: advanced editing/color/audio | C, D, E | Add capabilities as user needs and engine measurements justify. |
| Phase 14: AI generation | F | Integrate outputs as reviewable media/proposals; avoid model/service duplication. |
| Phase 15: community ecosystem | G | Start with contribution and static interchange; defer hosted systems. |
| Phase 16: plugins/interchange/OpenFX/WebMotion | G | Keep security contracts; build only bounded formats/extensions supported by demonstrated demand. |

## Replanning and completion

After each coherent capability, update this roadmap with status, exact product
commit, hosted runs/artifacts, platform coverage, performance measurements,
and any unmet requirement IDs. A requirement may be marked unnecessary only
with evidence grounded in the authoritative product requirements and an
explicit explanation of the replacement or removed user need. Preserve old
execution evidence; add convergence evidence rather than rewriting history.

The roadmap is complete only when all required user journeys and supported
platform gates pass, remaining limitations are explicit, and the product
provides a clear reason for its OR-specific layer to exist alongside its
upstreams.
