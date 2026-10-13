# Product roadmap investigation receipts

This archive preserves the detailed run-by-run reports moved out of the active `docs/PRODUCT_ROADMAP.md` during the export milestone. Pending/future-tense statements are historical snapshots; use current exact-run evidence and the active roadmap summary to determine present status. Run IDs, SHAs, diagnostics, and investigation detail are retained verbatim below.

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

## 2026-10-11 — ordinary MP4 packaged journey and Windows codec setup

Exact-SHA run [38116245081](https://github.com/huou07/Opencut-Reinforced/actions/runs/38116245081)
on `346d2402cf0a519cb840380512b5fc6b6c7bcb2d` passed the Linux packaged
H.264/AAC MP4 import, edit, title, save/reopen, relink, and WebM VP9/Opus export
journey, including independent output probing and full decode. Android's
packaged build passed its SAF preview/resource journey. Windows stopped at the
pinned libvpx build because the MSVC static-CRT option was incorrectly spelled
`--static-crt`; the exact upstream spelling `--enable-static-msvcrt` is
recorded in product commit `e6d1b5d`.

On exact SHA `346d2402cf0a519cb840380512b5fc6b6c7bcb2d`, the macOS native Flutter
lifecycle step later ran to the job's 100-minute timeout after Flutter logged
`Failed to foreground app; open returned 1`. No macOS lifecycle result can be
claimed from this run. The same three lifecycle tests passed on exact SHA
`fae38c7` in run `38114223195`; this isolates the result as inconsistent hosted
app launch behavior, not an accepted 346d2402 product result.

Exact-SHA run [38121804385](https://github.com/huou07/Opencut-Reinforced/actions/runs/38121804385)
on `a7183b34734c4cfeec3066dfe944d7cbdae4d0f8` passed the Linux and macOS
packaged ordinary-MP4 journeys, Android SAF preview/resource acceptance, and
the Rust, Flutter, and descriptor checks. Both desktop apps imported the pinned
22,718,509-byte 1080p H.264/six-channel AAC MP4, saved/reopened/relinked the
edit, added its title, and exported WebM VP9/Opus. Linux exported 12,763,556
bytes in 61,361 ms; macOS exported 12,768,681 bytes. Both outputs were 30.008
seconds, independently decoded to 720 video frames and 1501 audio frames, and
all four app phases exited 0 with host FFmpeg/ffprobe lookup blocked. Windows
passed its corrected libvpx CRT build but failed in FFmpeg configure while
probing for MSVC `stdbit.h`; that run is not a cross-platform pass. The next
candidate supplies FFmpeg 8.1.3's own `compat/stdbit` include path directly to
the Windows configure probes. The macOS log again printed `open returned 1`
before its Flutter tests, but its lifecycle, preview-transport, and packaged
journey tests all passed, so that message alone is not treated as a failure.

Exact-SHA run [38124449401](https://github.com/huou07/Opencut-Reinforced/actions/runs/38124449401)
on `af73be02d5907b38851bb45b88132b2b3a136f4d` passed Linux and macOS packaged
ordinary-MP4 journeys, Android all-ABI packaging and SAF preview/resource
acceptance, plus Rust, Flutter, and descriptor checks. Windows passed the
corrected libvpx CRT build and compiled FFmpeg's pinned `compat/stdbit` header,
but the feature probe failed at the MSVC link step. In the configure log,
earlier generic MSVC link probes omit `-debug`; the failing stdbit link probe
includes it. The follow-up disables FFmpeg debug symbols in the Windows release
runtime, retains the pinned compatibility header, and requires exact-SHA hosted
verification. No physical-device Android performance result is claimed.

Exact-SHA run [38126190565](https://github.com/huou07/Opencut-Reinforced/actions/runs/38126190565)
on `2c43fccad8cb80c850c52f5291927e57843a6e6c` disproved the hypothesis that
MSVC's `-debug` linker flag caused the stdbit probe failure. `--disable-debug`
removed that flag, but the probe's minimal `mslink` call still returned 1 with
no emitted linker diagnostic; preceding basic and library probes linked. This
run's Linux, macOS, Android, Rust, Flutter, and descriptor jobs passed. Debug
symbols remain enabled. The next diagnostic will expose the linker executable
resolved by FFmpeg's wrapper and preserve its return details before any further
Windows build change.

In exact-SHA run [38127990851](https://github.com/huou07/Opencut-Reinforced/actions/runs/38127990851)
on `bfb5b6d7a20430c127d795f585607983ae798dfd`, Windows again failed at the
same stdbit link probe. The failure branch then stopped before its diagnostic
because `command -v cl` returned no match under `set -e`; the toolchain exposes
`cl.exe`, and FFmpeg's own wrapper falls back to `link.exe` when bare `cl` is
missing. The next diagnostic checks both names and replays the probe through
that fallback. Linux and macOS packaged journeys, Android APK/SAF acceptance,
Rust, Flutter, and descriptor checks all passed on this SHA.

Exact-SHA run [38130438314](https://github.com/huou07/Opencut-Reinforced/actions/runs/38130438314)
on `22b0d07d6e2a0a6f913fd2d4edd90030cf05dfbe` again passed the Linux and
macOS packaged editing/export journeys, Android all-ABI package and SAF
preview/resource journey, Rust, Flutter, and descriptor checks. Windows failed
its FFmpeg build at the MSVC `stdbit.h` link probe. The in-step diagnostics
added to this SHA produced no runtime output, leaving the linker cause
unresolved. The next workflow revision moves capture into a failure-only
Actions step so it can inspect the retained FFmpeg source/configure log after
the failing step exits, and replay the exact wrapper link. No Windows packaged
journey or cross-platform pass is claimed from this run.

Exact-SHA run [38133514739](https://github.com/huou07/Opencut-Reinforced/actions/runs/38133514739)
on `ff0f0fd5f276c7bae2c30780b37aaeee170c4b90` passed Linux and macOS packaged
editing/export journeys, Android all-ABI packaging and SAF preview/resource
acceptance, Rust, Flutter, and descriptor checks. Windows again failed FFmpeg
configure, and the separate failure-only diagnostic ran successfully. It
reported `cl.exe` under Visual Studio, but bare `cl` was not found; PATH
resolved `link.exe` to `/usr/bin/link.exe` even though the adjacent MSVC
`link.exe` existed. FFmpeg's wrapper therefore selected the wrong linker on
fallback. The workflow now prepends Visual Studio's linker directory and
asserts the resolution before configure. Exact-SHA hosted verification of that
fix is pending; the run does not establish Windows packaged behavior.

Exact-SHA run [38135332968](https://github.com/huou07/Opencut-Reinforced/actions/runs/38135332968)
on `8de5b346e529ad142b405018fe8b83df370548dd` passed Linux and macOS packaged
journeys, Android all-ABI/SAF acceptance, Rust, Flutter, and descriptor checks.
Windows failed again at the FFmpeg stdbit link probe. The configure shell
asserted that MSVC's `link.exe` resolved first, so prepending that directory
did not fix the probe. The failure-only step reported `/usr/bin/link.exe` from
its own fresh shell; that output is not evidence of the configure process's
resolved linker. The next workflow change traces FFmpeg's wrapper from inside
configure and records it for the independent failure step. No Windows package
or cross-platform pass is claimed.

Correction from exact-SHA run [38138251731](https://github.com/huou07/Opencut-Reinforced/actions/runs/38138251731)
on `e1bc1d9e5fe29aa42e32d0f6ea3d7ec9e84e0d41`: the direct trace inside
FFmpeg's configure process shows that the Visual Studio `cl` and adjacent
`link.exe` were selected and the `stdbit.h` probe linked successfully. The
configure summary completed; Windows then failed at OR's VP9 guard because it
searched for `CONFIG_LIBVPX_VP9_ENCODER` in `config.h`. FFmpeg 8.1.3 generates
codec component selections in `config_components.h`, which is also the header
included by its libvpx encoder implementation. Thus the probe-failure
interpretations in the preceding entries were incorrect; the linker PATH
selection did resolve correctly. Linux packaged journey, macOS lifecycle and
packaged journey, Android packaged SAF/runtime acceptance, Rust, Flutter, and
descriptor checks all passed on this SHA. Windows packaging did not run because
the VP9 guard stopped the job. The follow-up changes only the guard's inspected
header, removes temporary wrapper tracing, and preserves the codec requirement.

Exact-SHA run [38140193039](https://github.com/huou07/Opencut-Reinforced/actions/runs/38140193039)
on `a81672b64f5ff2280de18de21c089cf03b1c49f0` passed repository hygiene,
Linux/macOS packaged journeys, Rust, Flutter, and descriptor checks. Attempt 1
failed Windows packaged export after its existing 180-second wait and failed
the third selected Android SAF media import with `SOURCE_UNAVAILABLE`; the
same unchanged SHA's attempt 2 passed the full Android SAF preview/import/
recovery journey and again failed Windows export at that timeout. This
establishes repeatable Windows journey failure and non-repeatable Android
failure on this pair of attempts, but provides no Windows export progress or
encoder error. The timeout was not increased. The next acceptance edit records
the already visible export status every ten seconds and on timeout, so the
next run can distinguish slow progress from a reported failure without
changing product behavior or its acceptance window.

Exact-SHA run [38162539040](https://github.com/huou07/Opencut-Reinforced/actions/runs/38162539040)
on `c3bcd95030d7fbcd7905a5a28d947e375f46497c` passed all non-Android jobs,
including Linux, macOS, and Windows packaged journeys on the same real 1080p
H.264/AAC sample. Each saved, reopened, relinked, exported 30 seconds of
VP9/Opus, and independently decoded 720 video frames and 1,501 audio frames.
Export times were Linux 47.299 seconds, macOS 57.394 seconds, and Windows
182.114 seconds; the Windows journey passed, but its runtime is recorded as a
dogfood performance concern rather than treated as a failed acceptance gate.
Android reached FFmpeg's successful configure summary with VP9, Opus, and WebM
enabled, then failed because the workflow searched for
`ffbuild/config_components.h`. FFmpeg 8.1.3 writes `config_components.h` at the
source root. The local fix changes only those three assertions; all-ABI Android
packaging and SAF output verification remain unproven pending a fresh exact-SHA
run. Rust, Flutter, and descriptor jobs passed. Repository hygiene passed on
the same SHA.

Exact-SHA run [38165543230](https://github.com/huou07/Opencut-Reinforced/actions/runs/38165543230)
on `81491556f6035a309cee129f25b7e762bd1dd11c` passed Rust, Flutter, descriptor,
and Windows packaged verification. The Windows journey again imported,
revised, saved/reopened/relinked, exported, and independently decoded the
30-second 1080p H.264/AAC-to-VP9/Opus workflow: 169.702 seconds and 12,768,681
bytes, with 720 video and 1,501 audio frames decoded. This is a successful
journey with a repeated slow hosted Windows export (the previous run was
182.114 seconds), not an acceptance failure. Linux failed before compilation
while curl's TLS connection to `downloads.xiph.org` ended with
`SSL_ERROR_SYSCALL`; this is an identified external download failure, not a
codec source/build diagnostic. Android successfully cross-built the first ABI
and then failed on the next ABI because FFmpeg's prior `cd` left the shell in
its temporary source tree while the workflow invoked the codec builder with a
relative repository path. The follow-up uses `$GITHUB_WORKSPACE` for that
script. The macOS lifecycle integration-test step timed out after 35 minutes
without an assertion result; its identical test selection passed on
`38162539040`. Logs show the app launch attempt reported `open returned 1`,
the test process remained alive until timeout, and cleanup terminated it. This
is unresolved; the next exact-SHA run will distinguish a repeatable test hang
from hosted-runner variance before any retry or product change.

Exact-SHA run [38169492902](https://github.com/huou07/Opencut-Reinforced/actions/runs/38169492902)
on `178763b645ef3ceca20659c97ce0cc561ff287e3` passed the Linux, macOS,
Windows, Rust, Flutter, descriptor, and repository-hygiene jobs. All three
desktop packaged journeys imported the real 1080p H.264/AAC sample, edited,
saved/reopened/relinked, exported 30 seconds of VP9/Opus, and independently
decoded 720 video frames and 1,501 audio frames. Linux output was 12,763,556
bytes in 46,504 ms; macOS was 12,768,681 bytes in 40,188 ms; Windows was
12,768,681 bytes in 161,731 ms. Windows remains slower on the hosted runner
(earlier successful measurements were 169,702 and 182,114 ms); this is a
dogfood/performance observation, not evidence for an unmeasured codec change.
The identical macOS lifecycle selection that timed out on `81491556` passed
here, so the earlier timeout is not repeatable product-failure evidence. Linux
also passed after the prior one-off Xiph download TLS error.

Android built the packaged app and FFmpeg runtime for all three ABIs, then
passed SAF preview, source permission revoke/regrant, and bounded resource
checks before its real export journey. The journey failed at
`android_saf_preview_test.dart:736`: after tapping Export it pumped one frame
and immediately expected the `Export video` profile dialog. Product code first
awaits the Rust project summary before presenting that dialog, so this is
consistent with a test-side asynchronous scheduling race; it does not establish
Android export failure or success. The follow-up on exact SHA
`5448f4d4933e378ea36547fa5ecbf283ba1b0769` adds a bounded wait for that dialog
and retains the profile assertions and actual SAF destination write plus
independent decode checks. The c5 emulator snapshot before export reported 22
presented frames, max main draw 19,196 microseconds, total main draw 106,611
microseconds, zero pending presentations, peak one pending frame result, zero
in-flight frame leases, one registered duplicated media FD, zero queued
operations, and one surface creation. These emulator results establish
boundedness/correctness for the observed path only; they are not physical-device
performance evidence and do not complete Android WebM acceptance.

Exact-SHA Platform Verification run
[38172368290](https://github.com/huou07/Opencut-Reinforced/actions/runs/38172368290)
passed on `33f7f355a5bb1c848551babe9c4c5c11c2a65316`. Linux, macOS, and
Windows packaged desktop journeys each imported, revised, saved/reopened,
relinked, exported 30 seconds of VP9/Opus, and independently decoded 720 video
and 1,501 audio frames. Export times were Linux 46,883 ms, macOS 54,526 ms,
and Windows 151,311 ms. Android built all three FFmpeg ABIs and passed
DocumentsUI SAF WebM export to the actual provider destination; the result was
965 bytes, reported VP9 video and Opus audio, and independently decoded to two
video buffers and 26 audio buffers. This Android fixture is only half a second.
The hosted emulator also logged skipped-frame stalls (including a 162-frame
startup stall); resource bounds passed, but this is not representative
physical-device performance evidence. Linux, Windows, and macOS lifecycle
coverage and repository hygiene also passed.

Developer Preview run
[38175016011](https://github.com/huou07/Opencut-Reinforced/actions/runs/38175016011)
on the same exact SHA failed before publication. All seven app/CLI build jobs
completed; Android and all three CLI packages succeeded. The macOS, Linux,
and Windows app builds succeeded but each packaging job looked for
`OPUS-COPYING` at the root of the downloaded FFmpeg artifact. Platform
Verification uploads codec provenance from `desktop-export-codecs/`; because
the artifact paths have `$RUNNER_TEMP` as their common ancestor, that folder
is preserved on download. The workflow now consumes that path and normalizes
the provenance files before checksum/release assembly. No preview was
published by run 38175016011; fresh exact-SHA verification and preview builds
are required.

Platform Verification run
[38175927309](https://github.com/huou07/Opencut-Reinforced/actions/runs/38175927309)
on `58d49321f2f1d0fe487cb159f21d2d5f316bbab3` verified Rust, Flutter static
and widget checks, and the descriptor boundary. Its Linux packaged journey
built the app and exported a 30-second VP9/Opus file in 46,957 ms
(12,763,556 bytes), then timed out before opening the insert dialog for that
reimported export. The acceptance path waited for the media name to appear but
tapped Add to Timeline before checking that the action was enabled. That action
is intentionally disabled until the media page, project, and timeline track
snapshots share the same revision; the test can therefore tap during refresh
and never reopen the dialog. A bounded readiness wait now checks that the
button is enabled and visible before tapping, preserving the same editing
assertion and timeout if the snapshots never converge. This is a falsifiable
test-race hypothesis, not a product PASS; fresh exact-SHA Platform Verification
is required. The completed run passed Android all-ABI build and SAF preview /
resource checks, macOS lifecycle and packaged journey, and the Windows
packaged journey; Rust, Flutter, and descriptor checks also passed. Linux was
the only failed job. This run used the fixed codec artifact paths, but did not
pass the full Platform Verification gate because of the Linux test failure.

Developer Preview run
[38181421768](https://github.com/huou07/Opencut-Reinforced/actions/runs/38181421768)
on `407a8f1d7a844240af89c6b7d119904d3b2f3531` built all seven application and
CLI packages but failed before publication. The release assembly log reported
GNU tar's stdout write error while checking required libvpx license members
with `tar -tf archive | grep -q member` under `pipefail`; early consumer exit
can close the listing pipe while tar is still writing. The workflow now
materializes the archive member listing in a runner temporary file before
checking exact license paths. A synthetic 1.5 MiB member listing passed through
the new path. This workflow change still requires fresh exact-SHA Platform
Verification and preview publication; the failed run did not publish a
release.

## Developer Preview provenance correction

Platform Verification run [38185861652](https://github.com/huou07/Opencut-Reinforced/actions/runs/38185861652) passed all seven jobs on `3c1e18f0f1119657bd6788717ab336e40a80887c`. Developer Preview run [38187844020](https://github.com/huou07/Opencut-Reinforced/actions/runs/38187844020) built all seven platform/application packages, then failed before release publication because `EXPORT-CODEC-SOURCES.txt` differed at the generated libvpx source archive hash.

Root cause reproduced locally against the pinned upstream libvpx commit `6df3ec34557879fff673706f4a1d9fbd0f3a6f0e`: `git archive` with `core.autocrlf=false` produced SHA-256 `aaa322cf8fbcd3be464e022c9748b736ba4f9bdcc4de81ce7a208148ed86010a`, matching the Linux/macOS exact-SHA logs. With `core.autocrlf=true`, it produced `9b35be70acb5ad09963528a4bf051ce3ea036d7139618f11c08f9613618a0ae8`, matching the Windows exact-SHA log. The archives differed starting at byte 1670, confirming Git's line-ending conversion changed archive content despite an identical pinned source commit.

`scripts/build_desktop_export_codecs.sh` now disables `core.autocrlf` specifically for the source archive generation. The release gate still verifies each archive against its recorded digest, checks required license and patent entries, checks the pinned source commit, and compares provenance/source bytes across platforms. A fresh exact-SHA Platform Verification and Developer Preview run are required before publication; this correction has not yet been hosted-verified.

## Timestamp-preserving transcription audio handoff and dogfood preview

Platform Verification run
[38188955493](https://github.com/huou07/Opencut-Reinforced/actions/runs/38188955493)
on `8d3e177311ec93dd8dbfa633b71c2eb9481cb446` failed only when the macOS
Flutter project-lifecycle test step hit its 35-minute runner timeout. It emitted
no test assertion or test-case output before timeout. The same selected test
group subsequently passed on `656cd656199ca6a236921c83ba9bdb5705f61d57` in run
[38192698936](https://github.com/huou07/Opencut-Reinforced/actions/runs/38192698936),
along with all other required gates. In that successful run the three lifecycle
tests took 14m29s, while the identical group had passed in 52 seconds on the
previous exact-SHA run 38185861652. This confirms a large macOS integration-test
latency/flakiness signal without a reproduced product assertion failure; it is
not a reason to loosen the test or claim a product regression. The Linux,
Windows, and macOS packaged real-media journeys all passed. The macOS 30-second
VP9/Opus export took 54.115 seconds and produced 12,768,681 bytes. Android
passed its emulator FFmpeg/bridge and SAF preview, export, relink, and recovery
checks; its acceptance media is small and does not establish physical-device
performance.

The same source SHA adds the first FFmpeg-backed, bounded mono 16 kHz WAV
handoff for future local transcription. The writer uses container-relative
decoded PTS, inserts bounded silence for positive stream offsets and real
forward gaps, rejects material overlaps, and checks cancellation. Generated
offset/gap fixtures plus AAC/MP3 decoder tests passed; the WAV remains an
internal media primitive, not a packaged or user-facing transcription feature.

Developer Preview run
[38194980068](https://github.com/huou07/Opencut-Reinforced/actions/runs/38194980068)
published the exact-SHA tag
[`dev-656cd656199c`](https://github.com/huou07/Opencut-Reinforced/releases/tag/dev-656cd656199c).
It contains macOS, Windows, and Linux debug apps; Android debug APK; all three
desktop CLI packages; build/source provenance; codec source archives; and a
13-asset checksum manifest. The publisher downloaded the published assets and
verified every entry in `SHA256SUMS.txt`. This makes a multi-platform dogfood
journey available; the release notes identify missing automatic captions,
unsigned/ad-hoc signing, Android physical-device performance, and other preview
limitations.

## Android multi-source SAF import correction

Platform Verification run
[38196732759](https://github.com/huou07/Opencut-Reinforced/actions/runs/38196732759)
on `13dcb51237668015e616c9d33bf66621697d161e` failed the Android SAF
multi-import assertion at `android_saf_preview_test.dart:800`: the first two
selected media imports succeeded and the audio-only third source returned
`SOURCE_UNAVAILABLE`. This was a product failure, not a CI setup issue. The
Flutter import path registered every selected SAF descriptor up front; after
each successful project command, preview refresh replaced the native active
source registry, invalidating descriptors for the remaining sources.

Commit `39afed57adf3a04faba842f78ac73a9d68a1bbed` registers each granted
source immediately before its import command, matching the preview refresh
lifecycle. Registration failures that occur before native ownership is
established preserve currently usable descriptors. A regression test verifies
that a missing source does not clear an existing native descriptor. Existing
per-source URI/grant validation, descriptor ownership, epoch checks, the
64-source registry cap, and bounded worker queue remain in force.

Exact-SHA Platform Verification run
[38199486640](https://github.com/huou07/Opencut-Reinforced/actions/runs/38199486640)
on that commit passed all seven jobs: Rust, Flutter, descriptor boundary, and
Linux, macOS, Windows, and Android packaged checks. Android SAF report confirms
one DocumentsUI selection imported the H.264/AAC MP4 sources and audio-only
MP3, and passed caption import/export, relink, WebM export, process recovery,
and relaunch checks. Export was independently decoded as VP9 video and Opus
audio. At journey completion the report shows zero OS media file descriptors,
zero in-flight frame leases, zero pending presentations, and zero queued
operations; bounded presentation stress peaked at eight pending frame results.
The same report records skipped emulator frames and a maximum 121.467 ms main
draw during this workload. The emulator fixture proves the repaired SAF
correctness and resource bounds only; representative physical-device
responsiveness remains unverified. No physical-performance claim is made.

## Local transcription package runtime work — 2026-10-12

The candidate desktop packaging implementation is in `0ba383f` and its
follow-up evidence-field correction is `ae3305e106fb379888512fe67f7a1a3e52f13c50`.
Exact-SHA Platform Verification
[38227008302](https://github.com/huou07/Opencut-Reinforced/actions/runs/38227008302)
passed all seven jobs on that SHA. Android's DocumentsUI journey, caption
import/export, relink, recovery, and bounded-resource checks passed; the
verifier accepted the restored stable `captionImportCalls` report field. This
run validates regression behavior and does not execute the new Whisper package
journey. Its resource report is a SwiftShader emulator correctness/resource
observation, not physical-device responsiveness evidence.

The delayed PR-triggered duplicate run
[38227057973](https://github.com/huou07/Opencut-Reinforced/actions/runs/38227057973)
passed six jobs but timed out after 35 minutes in the macOS native project
lifecycle step. The app bundle built; Flutter logged `Failed to foreground app;
open returned 1`, and the test group produced no case output before timeout.
The same exact-SHA workflow-dispatch run 38227008302 passed that lifecycle
group, as well as its packaged macOS journey. This is an intermittent hosted
macOS integration-launch failure without a product assertion; preserve it as a
runner reliability signal and do not count it as another product failure or
erase it by retry.

The Developer Preview workflow now contains a real desktop transcription
journey gate using the pinned `whisper.cpp` JFK speech fixture, the production
model-download and local-transcription UI, a human-edited proposal, project
save/reopen, and SRT output. Its macOS, Windows, and Linux runs remain pending
until an exact verified `main` commit reaches the main-only preview workflow.

Exact-SHA Platform Verification
[38234509928](https://github.com/huou07/Opencut-Reinforced/actions/runs/38234509928)
on `c329d8e26c994fb24af5f53d3f4c50e31087d1cd` passed Rust, Flutter
static/widget, descriptor-boundary, Linux, macOS, Windows, and Android jobs;
Repository Hygiene [38234510013](https://github.com/huou07/Opencut-Reinforced/actions/runs/38234510013)
also passed. The three desktop packaged product journeys and Android
DocumentsUI/resource journey passed. This exact-SHA run does not include the
new transcription runtime journey, which is gated by the main-only Developer
Preview workflow. The emulator does not establish Android hardware
responsiveness.

The subsequent bounded folder-import candidate is being verified on its own
exact product SHA. Its folder batch is capped at 64 to match Android's shared
preview/import descriptor registry; the application clears stale preview
registrations before an Android folder batch. Until that hosted run completes,
local picker/widget tests and selector tests are implementation evidence only,
not packaged Android or desktop user-journey acceptance.

The first folder candidate run, [38240164385](https://github.com/huou07/Opencut-Reinforced/actions/runs/38240164385)
on `78737472c5d9c7e5a4ce7bd184bb0c689314d2e2`, passed Linux, macOS, Windows,
Rust, Flutter, descriptor, and hygiene checks. The Android journey timed out in
the DocumentsUI selector during the existing multi-file import step, before the
new folder-picker interaction. Its retained
[`android-saf-process-recovery` artifact](https://github.com/huou07/Opencut-Reinforced/actions/runs/38240164385)
shows the last row extending below the usable screen while DocumentsUI still
reported it enabled. The selector tapped the clipped row instead of scrolling
its list container. The logged skipped-frame classification is not a separate
acceptance failure: the journey exited because native picker selection never
completed, before the Android report verifier ran. The selector now detects
screen clipping and scrolls the row's own container; a regression test covers
the enabled-but-clipped accessibility state. Folder import remains unaccepted
until the corrected exact-SHA run completes.

The first selector correction was verified on exact SHA
`39c919235594b4456459ab8dbdd3979050471be8` by Platform Verification
[38245348150](https://github.com/huou07/Opencut-Reinforced/actions/runs/38245348150).
Rust, Flutter, Linux, and repository hygiene passed; Android completed the APK
and runtime bridge steps but again timed out in the native multi-file picker
before the folder interaction. The retained
[`android-saf-process-recovery` artifact](https://github.com/huou07/Opencut-Reinforced/actions/runs/38245348150)
shows list view with the last expected filename omitted from the accessibility
tree. Since the list-view toggle was already active, the selector had no target
and never scrolled the directory. The second correction scrolls the specific
DocumentsUI directory list when the expected listing is incomplete; a regression
test covers that absent-row state. The candidate remains unaccepted until a new
exact-SHA run completes. The Windows job in this run separately failed while
downloading pinned zlib: GitHub returned HTTP 504 from the release download
endpoint before checksum or build work began. Windows completed that same step
on the following candidate SHA, `581c641`.

The second selector correction was exercised on exact SHA
`581c641933cbc0cd0094090d04a02389e604d07f` by Platform Verification
[38250084884](https://github.com/huou07/Opencut-Reinforced/actions/runs/38250084884).
Linux and Windows packaged journeys passed; Android reached the new folder
acceptance but did not open the tree picker. The retained
[`android-saf-process-recovery` artifact](https://github.com/huou07/Opencut-Reinforced/actions/runs/38250084884)
shows the previous three-file DocumentsUI import completed, followed by the
folder-test readiness marker without a new native picker launch. The test
waited for the observed gateway-call count, which increments before AppShell
finishes refreshing its project read model and releases the busy state; the
real Import Folder button was correctly disabled during that interval. The
test now waits for that real control to become enabled before tapping it. This
was an acceptance synchronization race, not evidence that Android tree
enumeration or folder import failed. Folder import remains unaccepted pending
the corrected exact-SHA hosted run.

The macOS job for `581c641` built the app bundle but its bounded native project
lifecycle test produced no case output: Flutter logged `Failed to foreground
app; open returned 1`, and the step timed out after 35 minutes. This is a
hosted launch failure without a product assertion. The required macOS run must
still be repeated on the corrected candidate SHA.

Platform Verification [38258271103](https://github.com/huou07/Opencut-Reinforced/actions/runs/38258271103)
ran on exact product SHA `c74d6759fe3ff5b8b0845b9cc88fe8b0f376c0fd`.
Linux, Windows and macOS packaged journeys passed, as did Rust, Flutter and
the granted-descriptor boundary; Repository Hygiene also passed. Android built
the APK and passed runtime bridge checks, then failed in the SAF folder-import
journey after DocumentsUI selected `Footage`. The retained
[`android-saf-process-recovery` artifact](https://github.com/huou07/Opencut-Reinforced/actions/runs/38258271103)
shows `DocumentsProvider` rejecting `footage-nested` as not a descendant of
`footage`. The separate-UID fixture exposed nested documents but did not
implement `isChildDocument`; Android's
[`DocumentsProvider` contract](https://developer.android.com/reference/android/provider/DocumentsProvider#isChildDocument(java.lang.String,java.lang.String))
requires that method for `ACTION_OPEN_DOCUMENT_TREE`. The fixture now computes
descendants from the same bounded child listing it exposes to DocumentsUI.
The next exact-SHA run confirmed nested enumeration and import work; its
remaining URI-scope assertion and hosted macOS launch result are recorded below.

Platform Verification [38262615923](https://github.com/huou07/Opencut-Reinforced/actions/runs/38262615923)
ran on exact product SHA `aa6b4d512c078fe96398d4fc074310b6c3a52bbb`.
Linux, Windows, Rust, Flutter and descriptor-boundary checks passed; Repository
Hygiene passed separately. Android completed folder enumeration and the import
operation successfully, then failed the expected-source assertion: the test
expected `tree/root/document/...`, while DocumentsUI had selected `Footage` and
the correct returned references were `tree/footage/document/...`. The retained
[`android-saf-process-recovery` artifact](https://github.com/huou07/Opencut-Reinforced/actions/runs/38262615923)
contains the successful import result and exact mismatch. The integration test
now checks both nested sources beneath the selected subtree and the persisted
`tree/footage` grant, while rejecting a broader provider-root tree grant.

The macOS job on `aa6b4d5` built the app but again failed to foreground it:
`open returned 1`, followed by the 35-minute native lifecycle timeout before
any test case ran. This remains a hosted launch limitation with no product
assertion; the exact-SHA macOS lifecycle journey still needs a passing run.

Platform Verification [38269354703](https://github.com/huou07/Opencut-Reinforced/actions/runs/38269354703)
ran on exact product SHA `2428751ba9067d8fd643cb49683b35f23e7baa81`.
Linux, Windows, macOS, Rust, Flutter, and descriptor-boundary checks passed;
Repository Hygiene passed separately. Android failed before folder selection,
while waiting for three media imports after the DocumentsUI picker returned.
The native SAF handler logged a successful selection result and completion;
the separate-UID fixture opened the three selected documents, but the test's
gateway completion count did not advance within its 60-second bound. The
artifact's report classified the run as `MAIN_THREAD_STALL`; guest logs include
`Skipped 122 frames` in the surrounding acceptance journey and show the fixture
provider process frozen at 15:48:44Z and unfrozen at 15:49:19Z after the test timeout.
Hosted process snapshots recorded QEMU at 252–273% CPU and 5.2–6.1 GB RSS,
with about 10.1 GB host `MemAvailable`. Those measurements show substantial
emulator load but do not establish whether the stall is in descriptor
registration, Rust probing, or hosted scheduling. The retained
[`android-saf-process-recovery` artifact](https://github.com/huou07/Opencut-Reinforced/actions/runs/38269354703)
contains the driver, guest, and resource logs. Commit `fb6b3f0` adds
test-only per-item gateway entry/completion timing and, on timeout, a bounded
descriptor/resource snapshot. It leaves the 60-second acceptance limit and
production code unchanged; a new exact-SHA run is needed to localize the stall.


Platform Verification [38275213412](https://github.com/huou07/Opencut-Reinforced/actions/runs/38275213412)
ran on exact product SHA `a7f4bf4010c3d3283e788387839edbc6f83a0f3d`.
Linux, macOS, Windows, Android, Rust, Flutter static/widget checks, the granted
descriptor boundary, and Repository Hygiene all passed. Clean packaged desktop
journeys passed on all three desktop operating systems. Android's separate-UID
DocumentsUI journey imported H.264/AAC MP4 and MP3 together, then selected the
`Footage` tree and imported its nested MP3 and Matroska files. The persisted
SAF grant remained scoped to `tree/footage`; a broader provider-root grant was
rejected. Caption import/export, relink with MediaId and timeline preservation,
WebM export with independent video/audio decoding, project recovery, preview
surface behavior, and resource-release assertions passed.

The added gateway markers show all five Rust `importMedia` calls completed
successfully in 1.8–6.8 ms. Android's half-second emulator export took 6.87 s;
that fixture proves output correctness only. The report measured a 112.8 ms
maximum main draw on this emulator, ended with zero duplicated media file
descriptors and zero in-flight leases, and labels hardware `UNVERIFIED`. This
is not physical-device performance evidence. The timeout on the preceding exact
SHA `2428751` did not recur. Because that failure had no gateway-entry marker,
its pre-gateway cause remains unresolved; the successful retry does not prove a
root-cause repair. The retained
[`android-saf-process-recovery` artifact](https://github.com/huou07/Opencut-Reinforced/actions/runs/38275213412)
contains the reports and guest/driver logs.


Validation-only Developer Preview dispatch [38286437542](https://github.com/huou07/Opencut-Reinforced/actions/runs/38286437542) ran on exact PR SHA `4033f90a16c9042008e404c1d597db7eaed6c493`, after Platform Verification [38281539863](https://github.com/huou07/Opencut-Reinforced/actions/runs/38281539863) and Repository Hygiene [38281539848](https://github.com/huou07/Opencut-Reinforced/actions/runs/38281539848) passed. The prepare gate accepted the exact branch, SHA, pull-request event, and successful run; all three CLI jobs and the publication job were skipped. The Android package succeeded. The Linux transcription job built the app but could not start it in the headless runner (`Unable to start the app on the device`); unlike Platform Verification's packaged journey, this direct integration launch omitted Xvfb. The macOS gate failed before opening the Generate Captions menu item. The Windows gate downloaded the model and began transcription but timed out after eight minutes waiting for caption review. The then-current test did not report whether transcription had failed or remained active. Follow-up changes add Linux Xvfb, detect surfaced error snackbars, and include the active dialog text in timeout diagnostics; desktop transcription is not yet verified. No desktop candidate package was uploaded, and no release or tag was created.

The next validation-only Developer Preview candidate [38293559262](https://github.com/huou07/Opencut-Reinforced/actions/runs/38293559262) ran on exact PR SHA `b758ef9270d81611161b2bb6e9fe1aa2d4bc8655`; the run is terminal. Android APK build and package succeeded, all CLI jobs and the publication job were skipped. Linux failed before Flutter initialized because its test process loaded a bridge without finding `libavutil.so.60`; unlike the proven packaged journey, the transcription step had not selected the bridge from the package directory. macOS and Windows both reached the actual transcription job and reported `The selected media path is invalid.` Source inspection found `transcribe_media` converted a native `Path` lossily to text and called the core API that accepts only a validated `file:` URI. The repair adds `MediaSourceRef::from_file_path` (preserving platform path encoding) and explicitly points all three hosted candidate journeys at the bridge inside the built package. Focused regression tests and hosted exact-SHA verification are pending. No desktop package was uploaded, no release/tag was created, and this candidate provides no transcription completion evidence.

Validation-only Developer Preview [38301589578](https://github.com/huou07/Opencut-Reinforced/actions/runs/38301589578) ran on exact PR SHA `f5a6ec9bad8eb86978f8515a5eac49ebdc637d30`. The source/release gate passed; Android APK build succeeded; CLI and publish jobs were skipped. All three desktop app build steps succeeded, but the transcription journey did not pass, so no desktop package artifacts were uploaded. Linux's test rebuild invoked the existing CMake clean-bundle install rule, deleting the FFmpeg libraries before the rebuilt bridge loaded. macOS did not expose the Generate Captions menu item after reopening the project; the next acceptance now checks persisted audio metadata and waits for the action to become available. Windows reached the provider and reported that private transcription audio failed verification. Comparing the host and provider paths identified the mismatch: the host wrote `audio.wav`, while the provider derived `<managed-id>.wav`. The new change centralizes both input and proposal paths in `or_ai`, writes the WAV using its managed UUID path, restores transcription helpers/licenses with the existing Linux runtime hook, and improves the macOS persistence/readiness diagnostic. Exact-SHA platform and packaged-candidate verification remain pending.

Platform Verification [38304231816](https://github.com/huou07/Opencut-Reinforced/actions/runs/38304231816) ran on exact PR SHA `20c1b65498904071f7101b217fda968cdaec8ffb`. Flutter static/widget checks and Repository Hygiene passed. The Rust job failed `cargo clippy` on a needless borrow in the new proposal-path call; the exact error reproduced locally, and the redundant borrow was removed. Other platform jobs were still running when the corrective commit was prepared, so their incomplete results do not count as verification for either SHA.

Platform Verification [38305268594](https://github.com/huou07/Opencut-Reinforced/actions/runs/38305268594) ran on exact PR SHA `84a41cd56c7426038ab07aec116217a089715157`. Repository Hygiene, Rust/Clippy, Flutter, Linux packaged journey, and macOS native/package journey passed. In attempt 1, Android failed during the pinned libvpx checkout with HTTP 503; Windows' persistent cache test `independent_stores_coordinate_budget_updates_through_sqlite` returned `Index("database is locked")` at the first joined `CacheStore::put`, while the other 40 cache tests passed. Rerunning failed jobs on the same SHA reproduced the upstream HTTP 503 during libvpx fetch on both Android (`chromium.googlesource.com`) and Windows (`chromium.googlesource.com`), so Windows did not reach the cache test on attempt 2. The source host's transient 503 is now handled by at most two delayed retries for HTTP 429/5xx only, clearing incomplete partial clones and rechecking the pinned commit. The Windows cache contention remains unconfirmed and must be observed in a complete retry; no cache limits or acceptance tests were changed.

Validation-only Developer Preview [38312462175](https://github.com/huou07/Opencut-Reinforced/actions/runs/38312462175) was dispatched on exact PR SHA `44b2197c688f0a229369983c2dad200c4d768332` after Platform Verification passed. Its exact-source gate succeeded, but Linux failed in `Seed Linux packaged media runtime` because `"$prefix/lib/$pattern"` quoted the wildcard and `cp` could not find literal `libavcodec.so*`. The correction leaves the directory quoted and expands only the verified library-name glob. Other package jobs were still running when recorded; this is not a completed preview or artifact claim.

The corrected candidate passed exact-SHA Platform Verification [38313145653](https://github.com/huou07/Opencut-Reinforced/actions/runs/38313145653) and validation-only Developer Preview [38316339457](https://github.com/huou07/Opencut-Reinforced/actions/runs/38316339457) on PR SHA `1d63db88d787653c1875006625ddbfb4c34217e0`. Platform Verification passed its seven jobs: Rust, Flutter, descriptor boundary, Linux, macOS, Windows, and Android. In the validation-only preview, the exact-source gate succeeded; Linux, macOS, and Windows each passed the packaged real JFK speech workflow through consented Whisper model download, transcription, caption review/edit/apply, project save/reopen, and SRT export. Android built the debug APK and verified its three packaged FFmpeg ABIs; Android transcription remains unsupported. The preview uploaded `developer-preview-linux` (45,859,207 bytes), `developer-preview-macos` (48,285,086 bytes), `developer-preview-windows` (52,045,982 bytes), and `developer-preview-android` (103,448,404 bytes); each temporary artifact expires October 19, 2026. CLI and publication jobs were skipped: no release or tag was created. This verifies packaged debug behavior, not clean-machine install/upgrade/uninstall, production signing, or physical-device performance.

Platform Verification [38318462544](https://github.com/huou07/Opencut-Reinforced/actions/runs/38318462544) ran on exact PR SHA `83b141730bdb13a78ad4db0ab332fe8813136b96`. Windows, Android, Linux, Rust, Flutter, descriptor-boundary checks, and Repository Hygiene passed. Android's hosted DocumentsUI/resource journey passed. macOS built the native app, then logged `Failed to foreground app; open returned 1` on runner macOS 26.6.2 and emitted no lifecycle test result before the 35-minute timeout. This is not macOS product acceptance evidence; the launch/test-startup issue needs reevaluation on the current candidate.
