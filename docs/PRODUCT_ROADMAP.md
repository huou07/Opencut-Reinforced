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
| **D. Preview, audio, effects, and text coherently** | Reliable seek/play/scrub/frame-step, audio sync and device controls, typed transforms, text/caption styling, effects/transitions, and preview/export semantic parity. Legacy: 7A–7H, 8A–8F, 10C–10D, 13C–13F. | Preserve bounded queues, immutable snapshots, deterministic fonts, fallback behavior, and frame/audio resource budgets. Measure the packaged OR baseline first. If it exposes a material media compatibility, reliability, or performance gap, run the bounded MLT/GES comparison in the convergence audit before considering a runtime replacement. |
| **E. Export a useful result** | Easy output presets and destinations, explicit format/codec capabilities, progress/cancel/error recovery, valid audio/video output, and repeatable packaged export. Legacy: 8F, 9D, 13 and release requirements. | Same typed project/render semantics as preview; FFmpeg provenance and redistribution review per target. Verify files in independent media probes/players and compare output behavior across desktop/mobile. |
| **F. Add focused AI assistance** | Transcript/caption proposals, translation, scene/silence analysis, selected generative assets, voice workflows, provenance, and review-before-apply. Legacy: 10A–10H, 12A–12F, 14A–14G. | Implement only after corresponding human edit/import/export paths work. Provider-independent tasks; credentials stay outside project/CLI; explicit consent, costs, network/data flow and model licenses. Outputs remain editable proposals/assets applied through normal commands. Use existing providers/services where useful; do not bundle models by default. |
| **G. Make the product extensible and contributable** | Stable documented command/API contracts, useful import/export interoperability (prioritize SRT/WebVTT captions against the `subtitler` integration proof), declarative templates/motion/assets, accessible developer docs, reproducible builds, community contribution path, and only then a justified plugin or asset distribution model. Legacy: 11A–11F, 15A–15D, 16A–16G and 4F. | Favor formats and upstream contributions over a marketplace/backend. Preserve no-code-execution defaults and explicit permissions. Do not build executable plugins, OpenFX, WebMotion, accounts, or hosted registries without a demonstrated user need and security/license acceptance. The parser's SRT/VTT tests and exact-time boundary proof pass; upstream PR [#8](https://github.com/subtitle-rs/subtitler/pull/8) proposes removing 22 active package nodes from that library-only feature set while preserving default CLI behavior. OR still needs to validate cue/style loss, undo behavior, and shipped-package size before depending on it. |
| **H. Cross-platform release quality** | Consistent design/terminology and material feature parity on macOS, Windows, Linux, Android; accessibility; signed/notarized production distribution; install/upgrade/uninstall; performance and reliability. Legacy: 7H–9E and release requirements. | Platform-specific file pickers, permissions, packaging, window integration and hardware APIs may differ. Editing semantics and recovery must not. Maintain an explicit platform matrix and report unverified cells honestly. iOS is future until supported. |

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

## Packaged journey implementation and verification history

The reports below preserve earlier media, caption, and Android journey
failures and repairs. Statements that hosted acceptance is pending reflect the
status at that point in the sequence. Current accepted results are summarized
after the log.

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

Run `38036564801` on exact product SHA
`7d25d17cc9c4996c556a86b0d349202fedbce371` completed successfully across Rust,
Flutter static/widget, granted-descriptor boundary, and packaged Linux,
Windows, macOS, and Android jobs. Android's SAF journey passed all 25 recorded
checks, including multi-file video/audio import, AAudio playback-clock
progression, preview pixels through background/resume and recovery, mobile
touch editing surfaces, relink identity/reference preservation, export,
permission enforcement/recovery, bounded presentation stress, and zero final
media FDs or in-flight leases. The report measured a 7,897 µs maximum Android
canvas-lock time on the API 36 x86_64 SwiftShader emulator, compared with
145,009 µs in the preceding software-canvas run; full stage measurements and
structured per-file import results are in the
[`android-saf-process-recovery` artifact](https://github.com/huou07/Opencut-Reinforced/actions/runs/38036564801).
This is useful evidence for the hardware-canvas path and confirms that the
earlier missing-audio report came from incomplete test diagnostics, not a
reproduced import failure. It is still debug-emulator evidence: physical-device
GPU behavior, release-build latency, and Android performance acceptance remain
open.

Exact-SHA run `38039913360` on product SHA
`02ebdddfe413099aed97aef40f2a6fed531fb76a` passed the Android SAF journey,
which recorded all 25 checks, the exact preview color through initial,
background/resume, and process-recovery samples, successful video/audio
imports, successful export, and zero final media descriptors or frame leases.
The Android measurements show main-thread draw max 5,039 µs, canvas draw max
76 µs, and direct bitmap-copy max 21,278 µs. Compared with the previous run,
this does not show lower peak copy latency; it removes the per-pixel CPU loop
and scratch buffer while preserving pixels. No Android performance acceptance
claim follows from this debug SwiftShader emulator result. The first macOS
lifecycle attempt timed out after app launch without test output; rerunning
only macOS at the same SHA passed the lifecycle tests and packaged product
journey. Rust, Flutter, descriptor boundary, Linux and Windows jobs had already
passed in the same exact-SHA workflow.

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

Run `37905372511` on exact product SHA
`7ec839651be7c408fd6f99904c1cc1f227ce9d6b` passed Linux, Windows, macOS,
Rust, Flutter static/widget, and descriptor-boundary checks. Android reached
the paging control, but its center was at y=644 on a 640-pixel viewport. The
test attempted a tap before that lazy child was hit-testable and the control
action did not run; no DocumentsUI relink picker opened. The acceptance now
scrolls until both the paging control and the index-64 media action are
hit-testable before interacting with them.
The picker/relink and recovery behavior remain unverified pending a new
exact-SHA hosted run.

Run `37908656472` on exact product SHA
`707f3a940537c444e4cb3c9f46dda55c5136733b` passed Linux, Windows, macOS,
Rust, Flutter static/widget, descriptor-boundary, Android all-ABI packaging,
and the Android packaged bridge smoke. Android reached the real DocumentsUI
picker and selected `tiny-second.mkv`; the persisted-grant check and picker
selection succeeded. The final source assertion still read `late65` instead
of `media-second`. That run is not evidence of a successful relink: the test
only counted gateway invocation and did not record the returned
`ProjectActionResult`. The acceptance now captures and logs the command's
success, error code, message, and returned revision, requires command success
before checking persisted state, and the report verifier requires that result
evidence. This distinguishes a rejected command (including a stale-revision
conflict) from a post-command read/refresh defect; no product mutation is
justified until the exact command result is observed. Android relink and
recovery remain unverified pending the exact-SHA rerun.

Only after manual caption interchange works end to end should transcription
create editable caption proposals. Review the selected speech-recognition
upstreams and model licenses as part of that task; provider output must enter
the same validated command/history path and remain reviewable before apply.

Run `37911913906` on exact product SHA
`1356ee8c6e060f7c9399c5e80cde9bb99bfa1ffa` confirmed the prior source
mismatch was caused by the acceptance choosing `media-second`, a document it
had already imported into the same project. The Rust command returned
`MEDIA_SOURCE_ALREADY_EXISTS`, correctly preserving the unique-source
invariant. The acceptance now selects a separate provider document,
`relink-replacement`, and the DocumentsUI helper identifies it independently
from the two-file import set. Its list-completion condition also now uses the
expected selection for each picker flow; it previously compared the relink
listing against the two-file import list and could loop forever. The report
and recovery verifiers now require the unique replacement URI. This is a
fixture/selector correction; Android SAF relink and process recovery remain
unverified until the corrected exact-SHA journey passes.

Run `37915335465` on exact product SHA
`e65f349cd7596d66d0cabce4de683b04d689c9a7` passed Linux, Windows, Rust,
Flutter static/widget, and descriptor-boundary checks. Android reached the real
relink command with the unique document selected, which correctly rejected the
replacement as timeline-incompatible. The acceptance project had declared a
one-second source stream and clip while its actual `tiny.mkv` fixture is 0.5
seconds; relink's source-range validation exposed that fixture mismatch. The
acceptance now records the actual half-second media and clip duration. Android
relink/recovery remain unverified pending a new exact-SHA hosted run; macOS was
still running when this correction was prepared.

Run `37917915879` on exact product SHA
`34157138170d631632fee5cd1ff1d0a0fccfd1fb` passed Linux, Windows, macOS,
Rust, Flutter static/widget, descriptor-boundary, and Android packaging/bridge
checks. The hosted Android DocumentsUI relink command now succeeds on the
unique replacement and returns revision 6, confirming the half-second fixture
correction. Later in the same real user journey, the writable project document
SHA-256 remained equal to its pre-edit value after the Save UI action. The run
therefore does not establish SAF save/recovery acceptance. Android logs also
recorded a 108-frame main-thread stall during the journey. The acceptance now
records the canonical save result and the actual picker synchronizer's call,
readback result, or error so the next run distinguishes command-save, picker
state, and provider-write failures. Keep the provider hash assertion and
investigate the stall from the hosted trace; do not treat the successful
relink alone as completion.

Run `37920881547` on exact product SHA
`091a9eafccf36d7cf3f402d6ab640b69aa1f05cd` passed Linux, Windows, macOS,
Rust, Flutter static/widget, descriptor-boundary, and Android packaging/bridge
checks. Android again completed relink at revision 6. Added telemetry then
showed the canonical save itself failed with `RECOVERY_REQUIRED` before the
picker synchronizer was called. The recovery candidate was produced by this
same live session's earlier autosave; `ProjectFileSession` already tracks that
exact checkpoint in `last_autosaved_project`, but save only accepted a
checkpoint equal to the newest live state. Save now accepts the session's own
recorded checkpoint while retaining canonical-base equality and rejecting
unrelated recovery candidates. A core regression test covers saving newer live
state after autosave. Exact-SHA run `37923946991` on `e6d1d9c` passed Linux,
Windows, macOS, Rust, Flutter, and descriptor-boundary jobs. Android relink and
Save both succeeded; Save invoked the picker synchronizer once, but the picker
returned `null` before calling native SAF synchronization. The provider-readback
assertion therefore failed. The captured report classifies Android main-thread
stalls, including 281 skipped frames during startup and several 37–105-frame
stalls in the journey. Exact-SHA run `37928517402` on `a2e99a7` passed the
Linux, macOS, and Windows packaged jobs, Rust, Flutter static/widget, and
descriptor-boundary checks. Android now confirmed identical DocumentsUI
selected/opened/synchronized working paths and a verified SAF provider
readback. The later fresh-session assertion failed because the acceptance
reopened its original temporary fixture seed instead of the selected working
copy; this was a test-path error, not evidence of a lost relink. A Rust
save/reopen regression independently confirms that relinked SAF URIs survive
project-file persistence. The corrected acceptance reopens the selected
working path and still checks media identity and timeline references. The run
also recorded Android main-thread stalls (140, 103, and 73 skipped frames at
startup/bridge/surface phases); measure and investigate these separately from
the persistence assertion.

Exact-SHA run `37932093237` on `94cd320` confirms the corrected Android
acceptance reopened the DocumentsUI working copy and proceeded through the
fresh-session relink, save, provider readback, and project reopen checks. It
then failed the final pixel assertion after the test released the Android
texture producer and registered a new texture ID: the center pixel was white,
and captured `saf-surface-recreated.png` shows only part of the red frame. The
source fixture is a solid red frame. In-place SurfaceProducer reset and
fresh-session playback checks had passed earlier in the same journey. A capture
immediately before producer release now distinguishes repeated surface-reset
damage from new-producer restoration; preserve the pixel check and investigate
the renderer lifecycle before changing it.

Exact-SHA run `37938319509` on `230b083` passed Linux, Windows, Rust, Flutter,
descriptor-boundary, and macOS packaged/lifecycle checks, but Android stopped
before the new surface capture. The test awaited the gateway Save call, then
read `lastSyncResult` before the separately awaited SAF synchronization had
completed; the log showed one sync call and `syncVerified=null`. The
acceptance now waits for the synchronizer's completion and still requires a
verified provider readback. This run supplies no new evidence about the
recreated-surface pixel; that rendering failure remains open.

Exact-SHA run `37941319421` on `2005d5d` confirmed the synchronization wait:
Save completed with `syncVerified=true`, and the journey advanced through
fresh-session reopen and playback. The new pre-release texture capture then
failed because the center pixel was black. Its screenshot shows the imported
caption on black, not the fixture's red source frame. The fresh-session Play
guard intentionally has no preceding seek; the 0.5-second video can finish
while its caption remains visible. Keep that no-seek Play assertion, then seek
to exact time zero before using the source frame to assess surface resets.

Exact-SHA run `37949628216` on `de33051` passed Rust, Flutter static/widget,
descriptor-boundary, Linux and Windows checks; the Android SAF journey passed
provider sync/readback, fresh-session reopen/play, and the exact-zero frame
wait, then failed the texture pixel assertion after eight forced surface
recreations. The preserved capture is 320×640: the solid-red source is red at
both sides but has a white vertical band across the center. The regular editor
captures from the same journey show the red source in its square preview. The
stress-only harness now retains a square preview aspect ratio for both
existing- and recreated-producer checks, but the next hosted run disproved the
layout hypothesis: the center band remained. The strict source-pixel check is
unchanged. A subsequent run isolated the failure to the first call of the
test-only forced-surface-reset path.

Exact-SHA run `37953579159` on `4cf79ed` passed Flutter static/widget,
descriptor-boundary, Linux, Windows, and macOS build, lifecycle, preview, and
packaged-product checks. Rust checks did not run: the hosted runner could not
resolve `ffmpeg.org` while fetching the pinned FFmpeg source (`curl: (6) Could
not resolve host`). Its Android APK built and emulator bridge check passed,
then SAF acceptance again failed at the white center pixel after the surface
stress sequence. The square-aspect screenshot confirms a red/white/red band
inside the preview itself, so incorrect portrait stretching does not explain
the result. The Android job evidence is preserved in the
[workflow run](https://github.com/huou07/Opencut-Reinforced/actions/runs/37953579159).

Run `37957951853` on `4f66ce3` captured the same band immediately after one
`getForcedNewSurface()` call; no repeated reset was needed. In that exact run,
the normal editor's real background/resume and permission-recovery captures
both showed the solid-red frame, and fresh-session Play, seek, SAF readback,
and resource bounds passed before the diagnostic failure. CodeGraph shows that
the `recreateSurface` and `releaseTexture` channel operations have no product
caller; they existed only to make this synthetic integration test replace a
surface outside Flutter's app lifecycle. Flutter documents `getSurface()` plus
`onSurfaceCleanup`/`onSurfaceAvailable` as the lifecycle contract, and the real
background/resume path already verifies that behavior. Remove those test-only
channel operations and keep the strict pixel assertion around real session
frames, bounded concurrent frame requests, and the actual OS lifecycle. This
does not claim that arbitrary forced surface replacement is product behavior.
[Flutter SurfaceProducer API](https://api.flutter.dev/javadoc/io/flutter/view/TextureRegistry.SurfaceProducer.html),
[Flutter Android surface lifecycle guidance](https://docs.flutter.dev/release/breaking-changes/android-surface-plugins).

## Current product priority: everyday media and useful delivery

The verified lossless export profile emits Matroska with FFV1 video and PCM
S16LE audio; it proves correctness but is not useful general-purpose delivery.
H.264/AAC import now uses the shared Rust probe, decoder, timeline, and Android
SAF paths. Exact-SHA desktop journeys use representative 1080p H.264/AAC media
on Linux, Windows, and macOS; Android currently verifies H.264/AAC with a
generated phone-shaped fixture, while representative phone-source acceptance
and per-market codec distribution review remain open. A three-second CC0 MP3
music excerpt is being added to the shared import/decode path and the Android
SAF-to-audio-track playback journey; current source checks pass, with packaged
exact-SHA verification pending. MP4 export and a generally useful compressed
delivery profile remain the highest-value follow-on gap. Treat format support
as a connected user capability spanning probe/import, package configuration,
decode/preview, editing, persistence, and delivery.

A real-media Rust proof now complements the small packaged fixture: OR's CLI
probe accepted a pinned 1080p H.264 sample; the software decoder test decodes a
1080p frame and generated silent 5.1 AAC to the editor's stereo output. The
separately licensed score was stripped. The redistributed half-second CC BY
3.0 excerpt and pinned source are documented in
`crates/or_media/tests/fixtures/README.md`. The clean packaged desktop journey
uses it for import, timeline preview, save/reopen, relink, and export.
Exact-SHA product SHA `f13427cb8997f313429ae450584c74cc896154e8` passed all
seven jobs in [run 38060985339](https://github.com/huou07/Opencut-Reinforced/actions/runs/38060985339).
Linux, Windows, and macOS clean packaged journeys imported the real 1080p
source, previewed it, saved/reopened, relinked, and exported. The test relinks
to a tiny fixture before export; the resulting 98 KB FFV1/PCM file is valid
correctness evidence, not proof of useful full-quality delivery. Android's
journey still uses synthetic media. This does not establish broader camera
variation, phone-source Android acceptance, or codec distribution review.
The desktop acceptance exports before relink and uses packaged `ffprobe` to
fully decode the real-media 1920×1080 result. The assertion initially used a
`null` muxer omitted from the minimal FFmpeg helper; it now uses packaged
`ffprobe -count_frames`. Exact-SHA run
[38069838351](https://github.com/huou07/Opencut-Reinforced/actions/runs/38069838351)
on `11e82c03bd82e03c3af3e9a089c73b44ef13bcde` passed all seven platform,
packaging, Rust, Flutter, and descriptor-boundary jobs. Linux, Windows, and
macOS reports each record a 267,197-byte Matroska/FFV1/PCM correctness output
with 12 video and 12 audio frames fully decoded by the packaged tool; Android
passed its SAF journey. This 0.5-second lossless output is not evidence of
practical compressed delivery or long-form export performance.

The same real-phone source exposed a separate preview correctness gap: its
H.264 stream is coded at 1920×1080 with a 90-degree display matrix. The Rust
decoder previously returned coded dimensions and pixels unchanged, so portrait
phone footage could appear sideways or stretched while FFmpeg-generated
thumbnails honor orientation. A pinned, metadata-scrubbed CC0 excerpt and
regression test now cover this case; the shared decoder fix must pass packaged
desktop and Android verification before this compatibility gap is closed.
This finding prioritizes everyday source behavior over further emulator-only
micro-optimization; emulator correctness and resource bounds remain intact,
while physical-device performance is still unverified.

Before shipping a profile, inspect actual decoder/encoder support and build
flags for every target; review component licenses, patent/distribution
constraints and FFmpeg notices/source provenance; exercise generated legal
fixtures through clean packaged desktop and Android paths; and measure decode,
preview, export time, output size, CPU, memory, and failure behavior. Preserve
structured unsupported-format reporting and keep the existing Matroska
lossless path available. Do not select a delivery codec from familiarity or
README claims alone, and do not promote any candidate to supported until its
real package and user journey pass. The audit keeps FFmpeg as the current
media foundation; MLT/GES remain measured comparison candidates, not
prerequisites or assumed replacements.

Run [38051317718](https://github.com/huou07/Opencut-Reinforced/actions/runs/38051317718)
on `6d9d1854111cc81140e4f3b45b7e1b1e8a74e16a` exposed a packaged-test fixture
mismatch: the 1.0s MP4 clip could not be relinked to the 0.5s Matroska file, and
the core correctly preserved its source-range guard. The fixture duration was
aligned in `7fa7f749eae235536a6ac5a5cd4ffecf7443cbf2`. Exact-SHA run
[38053268217](https://github.com/huou07/Opencut-Reinforced/actions/runs/38053268217)
on `7fa7f749eae235536a6ac5a5cd4ffecf7443cbf2` then passed all seven hosted
jobs, including clean packaged create/reopen/relink/export on Linux, Windows,
and macOS, plus Android all-ABI APK, H.264/AAC SAF import, captions, audio,
recovery, relink, and export. Android's report records 1,046µs MP4 import, a
valid 96,963-byte Matroska export, 43,034µs maximum main draw, zero final
media descriptors/frame leases, and `hardware=UNVERIFIED`; the guest log also
records 136 skipped frames on the OR process. This emulator result is not
physical-device performance evidence. The candidate uses a tiny generated
fixture, so representative camera-media compatibility and codec
patent/distribution review remain before H.264/AAC is a supported release
profile. MP4 export remains separate future work.

Run [37962108563](https://github.com/huou07/Opencut-Reinforced/actions/runs/37962108563)
on exact product SHA `e6f2afb6c808a8bea073829b17128e0b3116720e` passed Rust,
Flutter static/widget, descriptor-boundary, Linux, Windows, and macOS
packaged/lifecycle checks. Android built the all-ABI APK and passed the
packaged FFmpeg/Flutter bridge check, but the SAF journey stopped at the
surface callback assertion. The app resumed and a frame remained available;
the emulator did not destroy the Flutter surface during its ordinary
background/resume transition, so no cleanup callback was due. The acceptance
had incorrectly required a cleanup/restoration pair on every such transition.
The correction now preserves the real post-resume pixel assertion and records
the actual callback counters, requiring restoration only when cleanup is
reported. Run [37967821791](https://github.com/huou07/Opencut-Reinforced/actions/runs/37967821791)
on `457c1a0cc5221f27ef90516571aa0417481581c2` passed every non-Android job,
including macOS lifecycle/package verification, but Android failed earlier at
the assertion that playback remains active immediately before backgrounding.
The native Play command returned without an error, while the observed preview
state was `playing=false`; the test stopped before reaching the surface
lifecycle assertion. Therefore this run does not verify the revised surface
condition. A new diagnostic records both returned and polled play state, exact
position/content end, frame sequence, and call duration; the Android SAF gate
remains open. This run's logs also recorded a 328-frame stall and other long
frame gaps. The 328-frame record is on PID 1107 before the OR
process starts, so its ownership is not established; the SAF journey later
logged 160 skipped frames on the OR process. Android performance remains
unaccepted and needs a measured diagnosis.

Run [37975542884](https://github.com/huou07/Opencut-Reinforced/actions/runs/37975542884)
on exact SHA `ab0a8fc01abf58420a997c5bb45c1075eac2ecbd` passed Rust,
Flutter static/widget, descriptor-boundary, Linux, Windows, and macOS
packaged/lifecycle jobs. Android built all FFmpeg ABIs and passed its emulator
bridge check; the full SAF journey then passed backgrounding, resume, real
texture reacquisition, captions, relink, save synchronization, and project
reopen through the point where a fresh-session raw `Texture` screenshot was
sampled. The strict center-pixel check read white (`[255,255,255,255]`) even
though the preserved image shows red frame pixels on either side of a narrow
white center band; the earlier in-editor and resumed-texture screenshots in
the same artifact show a solid red preview. `frameAvailable` only confirms a
frame was posted to Android's producer surface, and this fresh-session check
mounted the Flutter `Texture` after its frame request, then sampled after one
100 ms pump. The harness now requests bounded real frames after mounting
before keeping the same strict pixel assertion. This compositor-settling
hypothesis still requires an exact-SHA hosted rerun; Android user-journey
acceptance remains open. Android logs also recorded 226 skipped frames on PID
1107 before the OR process (ownership unestablished), plus 129 and 107 skipped
frames on OR PID 3702 during the journey. Android performance remains
unaccepted pending measured diagnosis.

Run [37979354142](https://github.com/huou07/Opencut-Reinforced/actions/runs/37979354142)
on exact SHA `7f001265169017ce140eafe51bbbc73fa4c191f1` passed Rust, Flutter
static/widget, descriptor-boundary, Linux, Windows, and macOS packaged/
lifecycle jobs. Android again built the all-ABI package and passed its bridge
check, then failed at the same center-pixel assertion with white after the
test mounted the raw `Texture` and completed all eight bounded frame
submissions. This falsifies the simple compositor-settling hypothesis. The
preserved fresh-session screenshot again shows red on both sides of a white
center band, while the editor-view screenshots show a solid-red frame. The
acceptance therefore remains open pending evidence that distinguishes a
second-session preview/surface defect from a raw-texture test-layout mismatch;
the strict pixel assertion is unchanged. The run also recorded 173 skipped
frames on OR PID 3639 during the SAF journey. Android performance remains
unaccepted.

Run [37982658035](https://github.com/huou07/Opencut-Reinforced/actions/runs/37982658035)
on exact SHA `44eb9b527476ae6f057ae7a42089b55b9ff750ac` passed Rust, Flutter
static/widget, descriptor-boundary, Linux, Windows, and macOS packaged/
lifecycle jobs. Android failed at the same reopened raw-texture center-pixel
assertion after all eight post-mount frame submissions succeeded. The added
geometry diagnostic confirms the sample is correctly inside the texture:
`Rect(0,160,320,480)` in a `320x640` screenshot at DPR 1. Horizontal RGBA
samples across the texture center row are red at 10% and 25%, white at 50%,
pale red at 75%, and red at 90%. The coordinates are therefore correct, but
the reopened raw texture is not a uniform red frame. Normal editor and
background-resume screenshots from this journey remain solid red. The next
diagnostic will inspect sampled colors in the copied native Android bitmap
before it is posted to `SurfaceProducer`, separating the bridge/decoder output
from Android surface composition. Keep the strict screenshot assertion. The
run also recorded 146 skipped UI frames on OR PID 3664 and 122 skipped frames
on PID 2895 during the SAF journey; Android performance remains unaccepted.

Run [37987749155](https://github.com/huou07/Opencut-Reinforced/actions/runs/37987749155)
on exact SHA `c56be4bb44d3308e000de18639e3f94e99e0211a` confirmed the Android
pixel mismatch was the expected caption overlay. The imported SRT cue starts
at zero and lasts one second, covering the full 0.5-second red fixture clip.
Native bitmap samples before caption import were red; samples after import and
after project reopen had a white center with red video pixels on both sides.
This proves the reopened shared renderer preserved and rendered the caption;
the test incorrectly expected an uncaptioned red center. The test now checks
opaque red pixels on either side and an opaque bright caption pixel at center,
while retaining strict red-center assertions for uncaptioned preview paths.
Temporary native pixel logging has been removed. The exact-SHA Android journey
must pass this corrected assertion before SAF acceptance closes. Android
performance remains unaccepted: the run recorded 108 skipped UI frames on OR
PID 3883 during the SAF journey.

Run [37990736878](https://github.com/huou07/Opencut-Reinforced/actions/runs/37990736878)
on exact SHA `76b99fcdb9674fb45878ec731b04505de69c2620` confirmed that both
fresh-session caption-aware checks passed, including red pixels on both sides
of the white center cue. The Android journey then reached the post-stress
texture capture and failed because that third capture still used the
uncaptioned red-center helper, even though the same t=0 frame retained the
one-second caption. This is the same stale test expectation, not a renderer or
stress regression. The post-stress capture now uses the same strict
caption-aware red/white/red assertions. Rust, Flutter static/widget, and the
descriptor-boundary jobs passed; native/package jobs were still running when
this record was written. Android performance remains unaccepted; this run's
artifact/log indicates 108 skipped frames during the SAF journey.

Exact-SHA run [37997313827](https://github.com/huou07/Opencut-Reinforced/actions/runs/37997313827)
on `90e0ea4384c1722cdc714fdd474245c103c81fe6` passed the main Android SAF
editor journey, including all three strict caption-aware preview captures and
the resource stress checks. Its separate real process-stop/relaunch test then
recovered the project but failed at `android_saf_recovery_test.dart:158`: the
fixture expected the pre-relink `media-second` source after the same journey
had saved `relink-replacement`. The independent recovery verifier and its test
already require `relink-replacement`, so this is a stale Dart fixture
expectation; the integration test now checks the replacement source and its
persisted SAF grant. Windows packaging was still running when this finding was
recorded. Android release acceptance remains open until the exact-SHA process
recovery test passes. Android performance remains unaccepted.

Exact-SHA run [38001123678](https://github.com/huou07/Opencut-Reinforced/actions/runs/38001123678)
on `e100e7247fc0d987446096f11b18cb11bc0ca130` passed both Android Flutter
journeys and their independent report verifiers. The recovery report confirms
same-project force-stop/relaunch, explicit checkpoint recovery, restored
`relink-replacement` SAF source, preserved timeline reference, and preview
playback. The job still failed at its final required-image check because the
workflow demanded `saf-surface-recreated.png`, a synthetic screenshot no longer
produced after removing the forced surface-replacement test path. The real OS
background/resume capture `saf-background-resumed-texture.png` is present and
the report verifies that lifecycle. The workflow now requires that actual
capture, and a regression test ties required screenshots to names emitted by
the SAF journey. Android performance remains unaccepted; this run measured
104,141 µs for Play and a 117,018 µs slowest main-thread draw in the 16×16
emulator fixture.

Exact-SHA run [38007605677](https://github.com/huou07/Opencut-Reinforced/actions/runs/38007605677)
on `e76297bfa4fc16153f0287e3d35432036a640fe6` passed Rust, Linux, Windows,
macOS, and descriptor-boundary jobs. Windows and Linux packaged journeys and
the macOS native lifecycle/package journey passed. Flutter static checks caught
a Dart formatting issue in the new Android report assertion; that formatting
is corrected. Android built the all-ABI APK and passed through emulator bridge
verification, but the SAF journey stopped before the new WAV import at the
pre-background playback assertion. Its trace shows the test had sought to
0.45 s on a 0.5 s fixture; the Play request took 243,946 µs and the next poll
observed the preview at its end and paused. The test now seeks near the start
before checking active playback and background/resume, preserving the same
assertion with adequate fixture headroom. WAV import is not accepted as
packaged Android capability until that corrected exact-SHA journey reaches
and verifies the WAV import. Android performance remains unaccepted.

Exact-SHA run [38009873452](https://github.com/huou07/Opencut-Reinforced/actions/runs/38009873452)
on `fc00df936708393daa46fddc8023a32008bf88f9` passed Rust, Flutter,
descriptor-boundary, Linux, Windows, and macOS jobs, including all desktop
packaged journeys. Android built the all-ABI APK, passed emulator bridge
verification, and passed background/resume with a real resumed frame. Its
play request measured 93,891 µs at 47.8 ms into the 0.5-second fixture. The
SAF journey then reached the media picker but timed out before import: after
selecting both video fixtures, the third `tiny.wav` accessibility node was
disabled and clipped to y=632–640 at the bottom of a 640-pixel display. The
selector repeatedly tapped that clipped coordinate. This is a test-driver
scrolling defect, not evidence against WAV decode or import. The selector now
scrolls the native DocumentsUI list when a requested item is disabled, waits
for a fresh visible/actionable node, and caps retries; a regression test
reproduces the clipped tile. Fourteen local selector tests pass. Android WAV
import remains unverified until the exact-SHA packaged journey imports and
checks all three media sources. Android performance remains unaccepted.

Exact-SHA run [38012735537](https://github.com/huou07/Opencut-Reinforced/actions/runs/38012735537)
on `4fd142faa8861a83915d44a54bc72d1763baa78d` passed Rust, Flutter static and
widget, descriptor-boundary, Linux, and Windows jobs. macOS lifecycle checks
are still running. Android built the all-ABI package and passed the emulator
bridge, preview, background/resume, project recovery, permission recovery, and
export portions before its real media picker stopped at `tiny.wav`. The
preserved DocumentsUI tree showed the WAV row fully visible but disabled while
two video items remained selected. Inspection traced this to the product
intent: Android `openMedia` requested only `video/*`, so the picker excluded
audio files. This is an Android product import defect, not a WAV decoder or
scrolling failure. The candidate correction requests both `video/*` and
`audio/*`; WAV remains unaccepted on Android until a new exact-SHA journey
imports all three sources. Android performance remains unaccepted.

Exact-SHA run [38014795078](https://github.com/huou07/Opencut-Reinforced/actions/runs/38014795078)
on `186ed1ff3e4bda577da80e35e269cb14a8d19e9b` passed Rust, Flutter
static/widget, descriptor-boundary, Linux, and Windows packaged checks. The
Android all-ABI package and emulator bridge passed. The updated media filter
made `tiny.wav` selectable: native DocumentsUI recorded all three fixture
files selected, and the product returned `openMedia result count=3`. The
Android journey then failed at its post-import wait because the integration
test still expected exactly two `importMedia` calls. That assertion reflected
the former two-video fixture; it now requires all three video/audio imports.
This run proves selection and bridge delivery, not completed project import;
the exact-SHA hosted Android journey remains required. macOS lifecycle tests
were still running when this evidence was captured. Android performance
remains unaccepted.

Exact-SHA run [38016755374](https://github.com/huou07/Opencut-Reinforced/actions/runs/38016755374)
on `172dee37dbb7683d6034d884ee22614805ab8095` passed the Android SAF
preview/import/export/recovery job. The required report confirms all 24
picker/editor/resource checks, including `mediaImportAudioOnlyPcmWav=true`
and the three exact source URIs for both MKV fixtures and the WAV fixture.
The imported project reached revision 5; the WAV import call measured
1,267 µs. The packaged MKV export was valid at 96,963 bytes. Process-recovery
acceptance reports the checkpoint as a candidate before restart, then explicit
recovery at revision 8 after relaunch from PID 3577 to PID 5760; relinked media
identity and its timeline reference survived. The preview Play call measured
74,018 µs, with a 13,550 µs maximum main draw during the base journey and a
62,418 µs maximum under resource stress. Final OS media descriptors and
native leases returned to zero. Run 38016755374 on product SHA
172dee37dbb7683d6034d884ee22614805ab8095 predates the Android output wiring;
it verifies video frames and WAV library import, not audio-track playback.
Android CPAL output and its device-master clock are now implemented and await
exact-SHA hosted APK and playback acceptance. Android reports
`hardware=UNVERIFIED`, and
the guest log still contains Choreographer warnings for 111, 89, 65, 62, 38,
37, and 36 skipped frames. Thus bounded-resource and functional checks pass,
but Android performance is not accepted; profile the jank on representative
hardware before making a release claim. Linux, Windows, and macOS packaged
journeys/lifecycle checks, Rust, Flutter static/widget, and descriptor-boundary
jobs all passed on this SHA; workflow run 38016755374 completed successfully.

Exact-SHA run [38024117671](https://github.com/huou07/Opencut-Reinforced/actions/runs/38024117671)
on `da65ceb047ce0886d9783a05bb9873f5f5017765` passed Linux/Windows packaged
journeys, Rust, Flutter static/widget, and descriptor-boundary checks. Its
Android SAF WAV journey imported/recovered the project and reached AAudio, then
exposed `AUDIO_PLAYBACK_FAILED`. The cause was an ordering defect: Android Play
started CPAL before the bridge registered the audio-only SAF URI as a
seekable descriptor. Fix `0df7ea4c9e03d8a9034766ab8e4f19e747ee44e2` gathers
active visual and still-relevant audio SAF sources under the existing 64-source
bound, then starts audio only after native registration completion. Focused
Rust tests (18 bridge unit tests), clippy, Flutter analysis, formatting, and
diff checks pass locally.

Exact-SHA run [38026118020](https://github.com/huou07/Opencut-Reinforced/actions/runs/38026118020)
on `0df7ea4c9e03d8a9034766ab8e4f19e747ee44e2` passed all hosted checks:
Rust, Flutter static/widget, descriptor boundary, macOS lifecycle/package,
Linux and Windows packaged product journeys, and Android all-ABI packaging,
emulator bridge, SAF import/audio playback, recovery, captions, relink, and
export acceptance. The Android report records 1,028 µs WAV import, 152,660 µs
Play call, an advancing AAudio device clock with no preview error, a valid
96,963-byte Matroska export, and zero remaining media FDs or frame leases.
Base maximum main draw was 17,982 µs and resource-stress maximum was 54,331 µs;
the guest log also reports up to 125 skipped UI frames. Therefore Android audio
playback is accepted on the API 36 x86_64 SwiftShader emulator, but physical
device/acoustic output and Android performance are not accepted. Continue
profiling the frame skips on representative hardware before release claims.

The stage diagnostics split native acquisition, bitmap allocation and copy,
and overall main-thread surface presentation. The latest result shows high
variance in acquisition and total draw, so the next diagnostic update splits
the main-thread cost into surface resize, `lockCanvas`, bitmap draw, and
`unlockCanvasAndPost`. These counters are diagnostic only and do not declare
performance acceptable; inspect both ordinary preview and resource stress
before changing the presentation architecture.

Exact-SHA run [38028004390](https://github.com/huou07/Opencut-Reinforced/actions/runs/38028004390)
on `71254777be99b9b16d06fc5caa7d9fd1f5e86049` passed all hosted platforms
and emitted the first stage-level data. For a 921,600-byte presented frame,
native acquisition peaked at 501 µs, bitmap conversion/copy at 68,474 µs,
bitmap allocation at 288 µs, and main-thread surface drawing at 26,556 µs in
the ordinary journey. Under resource stress, surface drawing peaked at
100,379 µs; Choreographer reported up to 153 skipped frames. The largest
measured normal-path stage is the current per-pixel byte-channel conversion
plus bitmap copy, so the candidate improvement replaces eight byte-buffer
operations per pixel with a 32-bit channel swap while retaining the bounded
reusable buffers. Exact-SHA Android visual and stress measurements must
confirm the change before performance can be accepted; emulator-only evidence
still cannot establish physical-device performance.

Android compilation on exact-SHA run
[38029705498](https://github.com/huou07/Opencut-Reinforced/actions/runs/38029705498)
for candidate `0fff8bcac0771ecff1b21968fc4284e48a88140f` stopped before the
packaged journey. Kotlin reported an unresolved `order` reference because
`ByteBuffer.clear()` is typed as `Buffer` in this toolchain. The fix separates
`clear()` from `order()`; this failed run supplies no pixel or performance
acceptance evidence, and the corrected candidate still needs a hosted rerun.

Corrected exact-SHA run
[38030603727](https://github.com/huou07/Opencut-Reinforced/actions/runs/38030603727)
on `0e922ea552476042de5a424859a41a0636a40d3f` passed Rust, Flutter,
descriptor-boundary, Linux, Windows, macOS, and Android packaging/product
journeys. Android's real red-frame pixel checks pass with the native-order
pixel-word conversion; SAF WAV playback, recovery, export, and resource checks
also pass. On the 921,600-byte ordinary frame, total bitmap conversion/copy
was 70,736 µs over 28 presented frames (max 51,417 µs), versus 84,038 µs over
27 frames (max 68,474 µs) on the prior run. This is a modest measured decrease,
not a stable performance claim: native acquisition and main-thread surface draw
outliers were much higher on this run (27,341 µs and 168,227 µs maxima versus
501 µs and 26,556 µs before), showing substantial emulator/run variance. Stress
draw peaked at 168,227 µs; Choreographer still reported up to 79 skipped frames.
Android performance remains unaccepted pending representative-hardware
profiling and better-isolated surface-draw measurements. No acoustic-device
output claim is made from the emulator.

Caption interchange was subsequently verified in exact-SHA run
[38026118020](https://github.com/huou07/Opencut-Reinforced/actions/runs/38026118020)
on product SHA `0df7ea4c9e03d8a9034766ab8e4f19e747ee44e2`. All seven hosted jobs
passed, including macOS caption import/save/reopen/export, Linux and Windows
packaged journeys, and Android SAF caption, relink, recovery, audio playback,
and export acceptance. The earlier pending statements in this section are
historical. Cue/style fidelity boundaries and the cost/benefit of adopting the
external `subtitler` parser remain open; the passing journeys do not prove
those separate integration claims.

Explicit user discard and host teardown now have distinct policies in one
atomic close operation. Under the host lock, the UI may remove only a recovery
candidate still matching its last autosaved snapshot; ordinary shutdown,
including process-death simulation, preserves recovery. Tests cover user
discard, replacement by another session, and dirty teardown retaining
recovery. The local Rust and widget checks pass; the latest exact-SHA hosted
verification remains pending.

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
