# Active Product Roadmap

## Authority and status

This outcome-based roadmap supersedes the old checkpoint sequence for active
work as of the convergence pivot recorded in
[OPEN_SOURCE_CONVERGENCE.md](OPEN_SOURCE_CONVERGENCE.md). It groups real user
capabilities and technical dependencies; its rows are not separate operator
prompts or mandatory commits. `PLAN.json`, `STATE.json`, and
`execution/phases/` remain immutable historical traceability for the work
already performed. Their future `NEXT` cursor does not authorize a return to
the old implementation sequence.

Product requirements in [PRODUCT.md](PRODUCT.md), UX truth in
[DESIGN.md](../DESIGN.md) and [UX_ACCEPTANCE.md](UX_ACCEPTANCE.md), and
architecture/security/persistence invariants remain in force. A roadmap row
can be narrowed only when the source requirements or measured user value
justify it; record the exact rationale and evidence here. Do not label a
capability done from plans, source inspection, or model claims alone.

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
| **B. Bring media into the project reliably** | Media import for multiple files/folders where platform APIs permit, useful metadata/errors, thumbnails/waveforms, offline/relink workflow, media removal, Android SAF/document provider access, and bounded cache/proxy controls. Legacy: 9A/9B/9D/9E and 5A–5F. | Permissions and source references precede ingest. Test revoked/missing media, large libraries, >64 assets, SAF providers and packaged ffprobe/FFmpeg. Existing evidence covers desktop local files and Android SAF preview/export/recovery. Single- and multi-file SAF import plus desktop multi-file selection are implemented at `8dcadd6`; clearer missing-source feedback and the corrected acceptance assertion are at `8648be7`. Undoable desktop local-file relink preserving MediaId and timeline references is implemented at `d313a13`, with the packaged desktop journey at `e40c1ef`. Run `37861883543` on `3678792` passed Linux/macOS/Windows plus Rust/Flutter/descriptor checks but failed Android because the fixture project was read-only on the save-and-close step. The fixture now exercises writable SAF project synchronization and verifies the provider document digest changes on save; hosted rerun still required. Android real-picker relink acceptance now targets the existing item at index 64 and checks MediaId/timeline preservation through save, reopen, and process recovery; the selected source's persisted DocumentsUI grant is checked without synthesizing it. This acceptance extension is unverified pending the exact-SHA hosted run. Folder import and large-library cache behavior remain open. |
| **C. Make and revise an edit** | Clear timeline selection, tracks/clips, insert/move/trim/split/ripple, undo/redo, keyboard/pointer/touch operations, zoom/scroll, snapping, clip properties, markers, and mobile adaptations. Legacy: 6A–6E, 8A–8F, 9C, 13A/13B/13G/13H. | Keep exact Rust time and one command/history source. Group interaction and command work by user journey. Verify representative complex edits, persistence, CLI parity, and touch/keyboard accessibility. Defer advanced nested/multicam semantics until normal edits are smooth. |
| **D. Preview, audio, effects, and text coherently** | Reliable seek/play/scrub/frame-step, audio sync and device controls, typed transforms, text/caption styling, effects/transitions, and preview/export semantic parity. Legacy: 7A–7H, 8A–8F, 10C–10D, 13C–13F. | Preserve bounded queues, immutable snapshots, deterministic fonts, fallback behavior, and frame/audio resource budgets. Run the MLT comparison from the convergence audit before considering a runtime replacement. Measure latency, decode failures, CPU/GPU/RSS and audio behavior on packaged targets. |
| **E. Export a useful result** | Easy output presets and destinations, explicit format/codec capabilities, progress/cancel/error recovery, valid audio/video output, and repeatable packaged export. Legacy: 8F, 9D, 13 and release requirements. | Same typed project/render semantics as preview; FFmpeg provenance and redistribution review per target. Verify files in independent media probes/players and compare output behavior across desktop/mobile. |
| **F. Add focused AI assistance** | Transcript/caption proposals, translation, scene/silence analysis, selected generative assets, voice workflows, provenance, and review-before-apply. Legacy: 10A–10H, 12A–12F, 14A–14G. | Implement only after corresponding human edit/import/export paths work. Provider-independent tasks; credentials stay outside project/CLI; explicit consent, costs, network/data flow and model licenses. Outputs remain editable proposals/assets applied through normal commands. Use existing providers/services where useful; do not bundle models by default. |
| **G. Make the product extensible and contributable** | Stable documented command/API contracts, useful import/export interoperability (prioritize SRT/WebVTT captions against the `subtitler` integration proof), declarative templates/motion/assets, accessible developer docs, reproducible builds, community contribution path, and only then a justified plugin or asset distribution model. Legacy: 11A–11F, 15A–15D, 16A–16G and 4F. | Favor formats and upstream contributions over a marketplace/backend. Preserve no-code-execution defaults and explicit permissions. Do not build executable plugins, OpenFX, WebMotion, accounts, or hosted registries without a demonstrated user need and security/license acceptance. The parser's SRT/VTT tests and exact-time boundary proof pass; upstream PR [#8](https://github.com/subtitle-rs/subtitler/pull/8) proposes removing 22 active package nodes from that library-only feature set while preserving default CLI behavior. OR still needs to validate cue/style loss, undo behavior, and shipped-package size before depending on it. |
| **H. Cross-platform release quality** | Consistent design/terminology and material feature parity on macOS, Windows, Linux, Android; accessibility; signed/notarized production distribution; install/upgrade/uninstall; performance and reliability. Legacy: 7H–9E and release requirements. | Platform-specific file pickers, permissions, packaging, window integration and hardware APIs may differ. Editing semantics and recovery must not. Maintain an explicit platform matrix and report unverified cells honestly. iOS is future until supported. |

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

The MLT comparison is a bounded architecture experiment, not a prerequisite
for all product work. Do not let it block project usability, Android import,
or finishing the existing preview/export journey unless results establish a
real engine migration dependency.

## Next coherent implementation slice

SRT/WebVTT caption interchange now has a bounded core codec, one-revision
atomic `timeline.captions.import`, desktop file selection, Android file-backed
SAF staging, pre-import cue/loss preview, and caption export through the same
exact-time codec. Export pages caption tracks under one captured revision and
uses an atomic destination write. Desktop bridge coverage now imports, saves,
reopens, and exports a real SRT fixture. Android DocumentsUI acceptance has
been extended to exercise real SAF import/export and validate provider bytes;
that hosted journey is still pending. The generic provider harness also had a
pre-existing report bug: it imported two media files but called `.single` when
recording their URIs. Run `37864682420` exposed this at
`android_saf_preview_test.dart:950`; the report and verifier now retain both
URIs and assert both selected files.

The first hosted run for the caption journey, `37868974276` on
`2a931eae5a89be3d629544b21c42547702728152`, exposed a stale isolated
`tools/ffmpeg-link-probe/Cargo.lock`: Rust, Linux, and Android stopped before
product verification because their `--locked` probe builds needed the new
`or_core` subtitle dependency graph. The probe lockfile now resolves under
`--locked`; a follow-up exact-SHA hosted run is required. Windows and macOS
jobs from the first run were still in progress when this repair was prepared.

Run `37869690548` on the lockfile repair passed Rust, Flutter static/widget,
descriptor-boundary, and Linux packaged product checks. Its macOS native
bridge test exposed a stale revision in the new SRT integration test: the test
did not use the view returned by its preceding manual-caption insertion, so
the import was correctly rejected. The acceptance test now refreshes that
view; the production revision guard is unchanged. Android and Windows were
still running when this correction was prepared, and a new exact-SHA run is
required before closing caption interchange.

That run's Android job subsequently failed while compiling the SAF fixture:
`ControlActivity.java` used `File.isFile` as a field instead of calling the
Java `isFile()` method. The APK itself built successfully, but Android SAF
acceptance did not run. The fixture compile error is corrected in the current
working tree; the Android journey still needs a fresh hosted run.

Run `37869690548` completed with Linux and Windows clean packaged product
journeys and FFmpeg runtime provenance passing, together with Rust, Flutter
static/widget, and descriptor-boundary checks. Its only failing jobs were the
macOS stale-revision assertion and Android fixture compile described above.
The test and fixture corrections are committed at
`f88d674e8bb37ee78b01c465d9df48ea5d8ecaf1` and
`9b16806b4302e685a5baff01b718921311a57ac4`; the corrected desktop caption
bridge and Android SAF caption journeys have not yet run on a hosted platform.

Run `37871818918` passed macOS native caption import/save/reopen/export,
Linux/Windows packaged product journeys, Rust, Flutter static/widget, and the
descriptor boundary. Android built the all-ABI APK and passed its emulator
bridge check, then its SAF selector timed out before media import completed.
The preserved native DocumentsUI image showed one file selected, while the
selector had assumed its earlier gestures selected both; the device guest log
has no OR crash or ANR signature. The selector now derives its actions from
the filenames actually marked selected and records those states/actions, with
a fixture reproducing the observed one-file selection. All ten selector and
seven report tests pass locally; Android caption acceptance still requires a
hosted rerun.

Run `37874871831` passed Rust, Flutter static/widget, Linux and Windows
packaged-product checks, and the descriptor boundary. Android built the
all-ABI APK and passed the packaged FFmpeg/Flutter bridge check, but the real
SAF journey failed at the two-file media import assertion. The preserved
DocumentsUI tree and provider log show that the picker returned one source
(`count=1`); the driver had treated the currently visible filenames as the
complete expected set while DocumentsUI was still loading its provider list.
This was a selector false-positive, and the product correctly exposed that
only one document had been imported. The selector now requires both known
fixture filenames to be present together before it can complete the selection,
and its regression fixture models the partial-list state. All ten selector
tests pass locally. The macOS job from this run is still completing; the next
exact-SHA run must verify the revised selector and caption journey on Android.

Run `37877882018` passed Rust, Flutter static/widget, Linux, Windows, macOS,
and the descriptor boundary on `ee416c2355973f64349acac2839ea04157b9d987`.
Android built its all-ABI APK and passed the emulator bridge, but the revised
selector correctly refused to complete from a partial tree and timed out. The
failure screenshot shows DocumentsUI's grid view at 320×640: the `tiny.mkv`
tile is clipped below the viewport and its filename is absent from the
accessibility tree, although its preview icon is visible. DocumentsUI exposes
a native `List view` control. The selector now switches through that control
when a required filename is absent, then verifies both names and selected
states before finishing. A regression test models the clipped grid, the list
view transition, and both selection states. Eleven selector tests pass
locally; a fresh hosted run must verify the Android journey. The same guest
log reports 143 skipped UI frames during cold Android startup. That did not
cause the picker selection timeout, but it is a separate runtime performance
signal and must be measured against the product's Android frame-time/resource
budgets before Android performance can be accepted.

The published `subtitler` crate still adds 29 unique packages to the OR graph
and brings `clap` and `tracing-subscriber` into `or_core`; upstream PR #8
proposes removing that library-only cost but is not merged. Measure the final
packaged runtime size and run the completed desktop and Android caption
journeys before deciding whether to keep the dependency. Imported cue
boundaries remain exact rational times, and unsupported styling/placement is
reported for confirmation before applying the import.

Run `37880371173` on exact product SHA
`cf5542af6070dafae0009ae009da635afe2af3db` passed Rust, Flutter static/widget,
descriptor-boundary, Linux, Windows and macOS checks. Android built the
all-ABI package, passed the emulator bridge, and selected both known media
files through DocumentsUI; `MainActivity` recorded `openMedia count=2` and a
successful storage operation. The journey then stopped at
`android_saf_preview_test.dart:609`, before caption import. The saved Flutter
log says the tap on `timeline-import-captions` missed the target and hit only
the root view; the native activity log has no `openCaptionFile` result. This
was a test-action miss, not evidence of a product picker defect. The selector
correction at `cf5542a` remains a verified improvement.

Run `37883136736` on exact product SHA
`5c7d6120ceb66ae8dd9d6c8785ffdc0cbe97f52d` passed Rust, Flutter static/widget,
descriptor-boundary, Linux and Windows checks. Android built the all-ABI
package and passed its FFmpeg bridge check. DocumentsUI selected both media
files, imported `captions.srt`, then selected a destination for caption export.
The journey stopped at `android_saf_preview_test.dart:649` because the
acceptance gateway's completed-export counter remained zero. The logs confirm
`openCaptionFile` and `createCaptionExport` both returned successfully, but no
export gateway completion was recorded. The saved selector log proves both
caption picker flows completed. The preserved
[`37883136736` Android evidence](https://github.com/huou07/Opencut-Reinforced/actions/runs/37883136736)
includes the SAF process-recovery bundle and guest/provider logs. This run does
not prove caption interchange end to end. The harness now waits for the export control to become enabled
after caption import and logs gateway entry separately from completion; the
next exact-SHA run will distinguish an app busy-state short circuit from a
stalled Rust export. The Android SAF and caption journey remains open, and the
legacy roadmap stays paused.

Run `37885867632` on exact product SHA
`d08683131d6fa4dc2a757cd912c6311ce69e2003` passed Rust, Flutter static/widget,
descriptor-boundary, Linux and macOS packaged journeys, and Windows packaged
journey. Android built the all-ABI APK and passed its emulator bridge check,
then failed the SAF journey at `android_saf_preview_test.dart:658` before the
caption export gateway was entered. DocumentsUI selected the destination and
native storage logged `createCaptionExport` success. The Dart method-channel
contract requires `workingPath` and `documentUri`, but
`MainActivity.captionExportLocation()` returned only `workingPath`; Flutter
therefore rejected the valid native result before invoking the export gateway.
The Kotlin result now includes the already-held content URI. The next hosted
run must verify actual gateway entry, caption bytes published to the chosen
provider document, the rest of the SAF journey, and recovery. This run also
reported 138 skipped Android UI frames during cold startup; that performance
signal is separate from the caption contract failure and remains unresolved.

Run `37888323616` on exact product SHA
`ca5eddc9b59643dd8995387a8f5209ca2d3a40b8` proves the URI response fix: the
Android real editor journey entered and completed `exportTimelineCaptions`,
showed `Caption file exported.`, and the isolated SAF provider verified
nonempty valid SRT bytes. After caption export, media-panel interaction,
caption/clip checks, seeking, and Inspector access, the journey failed at
`android_saf_preview_test.dart:762` because the test waited for literal
`Saved` text. The user-facing save action does not provide that snackbar; its
success contract is the gateway save plus a changed provider project digest.
The acceptance now observes both. The run also reported cold-start and
surface-conversion stalls, including 125 and 123 skipped frames; these remain
separate performance evidence to analyze and measure rather than suppress.
All other completed checks passed: Rust, Flutter static/widget, descriptor
boundary, Linux and Windows packaged journeys. The macOS packaged journey also
passed. The platform run's Android failure is limited to the stale acceptance
expectation described above; packaged Android caption export itself is now
verified, and the complete Android SAF journey and process recovery now pass.
Android packaged performance remains open.

Run `37891034387` on exact product SHA
`4d0aa122beaf377eb194bfaa1a250b7ef7f8c979` passed Android all-ABI packaging,
the complete SAF editor journey and process recovery, Rust, Flutter
static/widget, descriptor-boundary, Linux, and macOS checks. Windows built and
launched both packaged journey phases; project creation/save/reopen passed.
The reopen/relink phase updated the visible replacement media row, then failed
at line 252 while immediately checking for a timeline clip. `_refreshProjectState`
reloads media and timeline concurrently, so the harness could observe the new
row before the timeline refresh completed. The Rust command and existing core
regression test preserve the same MediaId and timeline references. The Windows
journey now waits for the clip to reappear and the save control to become
available before proceeding; the packaged Windows relink/save/export path still
required a hosted rerun at that point. Run `37893752172` below closes that
acceptance.

Run `37893752172` on exact product SHA
`dffa88e948e3b42b0e6d5c0ddadb53c59ca5de8f` completed successfully. Rust,
Flutter static/widget, the granted-descriptor boundary, and native/package
verification passed on Linux, Windows, macOS, and Android. The clean packaged
Linux and Windows project journeys passed; macOS project lifecycle and preview
transport passed. Android's real DocumentsUI SAF journey verified multi-file
media import, visible pixels, touch editing surfaces, SRT import/export,
Matroska video export (193,191 bytes), persisted project synchronization,
permission recovery, surface recreation, and process-death recovery. The
recovery checker independently verified the prepared and relaunched project
reports. The exact run's
[`android-saf-process-recovery` artifact](https://github.com/huou07/Opencut-Reinforced/actions/runs/37893752172)
contains the guest/provider logs, report, screenshots, and process evidence.

Android measurements are from this debug APK on the API 36 x86_64 SwiftShader
emulator, using the acceptance fixture's 16×16 video. The recorded play call
was 99,402 µs; the slowest main-thread surface draw was 90,386 µs during the
bounded-presentation stress; media import was 2,706 µs. Stress reached the
configured eight pending-result limit, rejected excess requests, and ended
with zero pending work, frame leases, media descriptors, or OS media FDs. The
guest log also contains 30–87 skipped-frame reports and HWUI `Davey` frames of
828–1,274 ms during the multi-app journey. These measurements flag an Android
latency issue, but do not yet separate OR work from debug instrumentation,
SwiftShader, or system-picker scheduling. They are not release-performance
evidence. Keep Android performance open until a profile/release measurement on
representative device hardware identifies the responsible path and shows
acceptable playback and interaction latency. The current run proves journey
correctness and resource release, not performance acceptance.

Run `37898610367` on exact product SHA
`eee5d2413cbf5334c27a05a6523fc09a9b6acf41` passed Rust, Flutter static/widget,
descriptor-boundary, Linux and Windows packaged verification. Android built
the all-ABI APK and passed the packaged FFmpeg/Rust bridge smoke, but its SAF
journey failed before opening the relink picker: the test mounted the Media
panel and called `ensureVisible` on “Load more” before its asynchronous first
media page rendered the control (`StateError: Bad state: No element` at
`android_saf_preview_test.dart:724`). The test now waits for that control. This
is an acceptance-harness readiness race, not a relink result; the Android
DocumentsUI relink and process-recovery path remain unverified pending a new
exact-SHA hosted run. The macOS job was still running when this correction was
prepared.

Run `37901750115` on exact product SHA
`c77f9ccb425bc82e8c08d8ea9b575c45f0ac6001` passed Linux, Rust, Flutter
static/widget and descriptor checks, and Android packaging/bridge setup. The
Android journey then timed out waiting for “Load more” at the same test line.
The panel uses a lazy `ListView`, so an offscreen paging control does not exist
in the widget tree until the actual list scroll reaches it. The acceptance now
scrolls that panel with a bounded loop before loading the final page. Its
DocumentsUI selection log contains no relink action, so the real relink path
remains unverified pending a new exact-SHA hosted run.

Only after manual caption interchange works end to end should transcription
create editable caption proposals. Review the selected speech-recognition
upstreams and model licenses as part of that task; provider output must enter
the same validated command/history path and remain reviewable before apply.

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
