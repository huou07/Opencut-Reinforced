# Quality Bar

Last reviewed: 2026-10-29

Scope: the attributes OR holds itself to, how each is measured, and which gaps
are known. The current runtime target is the local macOS app on the owner's M5
MacBook Air; older Linux/hosted numbers are historical, not a current baseline.

## Attributes

| Attribute | Target | Measured by | Gate |
|---|---|---|---|
| Correctness | acceptance journeys pass | `bash scripts/acceptance.sh` | acceptance |
| Correctness (source) | hygiene, docs, claims, core dependency boundary | `bash scripts/check-repo.sh` | verify |
| Performance | bounded local budget rows (process, command, project file, render frame, queue priority) on the named host class | `python3 scripts/bench_quality.py` | acceptance |
| Reliability | save/reopen, undo/redo, autosave and recovery in named journeys | widget + UI journeys | acceptance / hosted |
| Accessibility / UX | 4.5:1 contrast, ≥11 px type, 24/44 px hit areas, keyboard alternatives | headless guards in [design/ux-invariants.md](../design/ux-invariants.md) | widget tests |
| Security | no secrets tracked; dependency boundary enforced | `gitleaks detect`, `check_or_core_deps.py` | acceptance / hosted |
| Compatibility | macOS/Windows/Linux/Android hosted jobs | CI matrix | hosted |
| Maintainability | links resolve, current docs dated, claims resolve to a commit | doc and claim gates | verify |

## Performance discipline

A performance claim needs the workload, source/build, codec/backend, machine and
measured result. The recorded budget rows were measured on 2026-10-18 on an idle
Debian 13 (trixie) OptiPlex-class x86_64 host; do not reuse them as M5 numbers. Add a current Mac measurement only after recording
those fields.

### MotionScene materialisation on the local host (2026-10-27)

- Workload: the ignored `measures_materialisation_throughput_at_a_named_canvas`
  scene — a 24-line Rust sample, 640×360 at 10 fps, 88 frames.
- Source/build: `or_app_bridge` the commit that parallelised the motion sweep,
  release profile.
- Codec/backend: FFV1 in Matroska, software encode, CPU rasteriser.
- Machine: MacBook Air M5 (Mac17,3), 10 CPU cores (4 performance, 6 efficiency),
  8 GPU cores, 16 GB unified memory, idle host.
- Result: 351 frames/s end to end (0.251 s) against 78 frames/s before the
  change (1.124 s) — 4.5× — with an identical 2,104,841-byte output. The
  raster-only sweep at 1920×1080 went from 33 to 166 frames/s in the same run.

### Export through the render-ahead pipeline (2026-10-27)

- Workload: the ignored `a_render_ahead_export_writes_a_probeable_file` — a blank
  1920×1080, 30 fps, 3 s timeline (90 frames), no source media, so the row measures
  the export loop itself: GPU composite, readback, colour conversion, FFV1 encode,
  mux.
- Source/build: `or_app_bridge` the commit that pipelined the export loop, debug
  test profile.
- Codec/backend: FFV1 in Matroska, software encode, wgpu Metal for the composite.
- Machine: MacBook Air M5 (Mac17,3), 10 CPU cores, 8 GPU cores, 16 GB unified
  memory, idle host.
- Result: 21.6 frames/s (4.166 s) against 13.7 frames/s (6.569 s) for the
  frame-at-a-time loop — 1.58× — with both runs writing 947,529 bytes. The file
  probes as Matroska, `ffv1` 1920×1080 at 30 fps plus `pcm_s16le` 48 kHz stereo,
  duration 3.000 s on both streams.

This row has no source decode in it: a project with camera media will show a
smaller multiple, because decode adds work to the render side that pipelining
cannot overlap with itself.

### Media artifact pool width (2026-10-29)

- Workload: the ignored `artifact_throughput_by_pool_width` test —
  `crates/or_core/tests/artifact_throughput.rs` — generating one thumbnail and
  one waveform for each of eight 640×360, 1.5 s clips (16 artifacts), cold cache,
  at four pool widths.
- Source/build: `or_core` release test profile, system FFmpeg 8.x.
- Codec/backend: H.264/AAC sources, FFmpeg subprocess per artifact.
- Machine: MacBook Air M5 (Mac17,3), 10 CPU cores, 16 GB unified memory, idle
  host.
- Result: 34.4 ms per artifact at one worker, 15.3 ms at two (2.2×), 8.7 ms at
  four (1.8×), 5.9 ms at eight. Scaling stays close to linear because a job is
  one short-lived FFmpeg process, not a persisted thread.
- Decision: the app asks for four workers when the machine reports at least six
  parallel slots and two otherwise (`media_artifact_worker_count` in
  `crates/or_app_bridge/src/api/project.rs`), so a cold multi-clip import fills
  the library about twice as fast while four of ten cores remain for preview and
  playback.

These are measurements of one workload, not a budget row: they are not enforced
by `scripts/bench_quality.py` and must be re-measured before being quoted again.

### Where an exported frame's time actually goes (2026-10-27)

Instrumented on the same blank 1920×1080 export, per frame, after the render-ahead
change:

- render (decode, GPU composite, readback): 2.8–3.6 ms per frame.
- delivery encode (VP9, BGRA→YUV plus libvpx): 22–25 ms per frame, so a WebM
  export is encoder-bound, not render-bound.
- the first frame carries a one-off 2.5 s render-device initialisation. In the
  app this is paid by the first preview, not by an export started afterwards; in
  a fresh test process it dominates a short export.
- `or_render`'s own 640×360 row is 1.4 ms p50 including readback on the same
  Metal adapter, so the bridge adds little per frame.

### MotionScene preview stills (2026-10-27)

- Workload: a 24-line Rust code scene, one frame rasterised, then
  `encode_png`, at both the panel default and 1080p.
- Source/build: `or_render` the commit that moved PNG encoding onto the `png`
  crate, release profile.
- Machine: MacBook Air M5.
- Result: the hand-rolled stored-deflate writer produced 0.92 MB in 3.77 ms at
  640×360 and 8.3 MB in 34.4 ms at 1920×1080. Fast deflate through the bundled
  `png` crate produces 26 KB in 0.17 ms and 67 KB in 0.74 ms. A flat vector scene
  compresses exceptionally well; a scene carrying photographic image nodes will
  compress far less, and there the encode gets slower than a stored block in
  exchange for a smaller payload, which is the right trade for a still that
  crosses the bridge into the panel.

### Interaction latency after in-place preview refresh (2026-10-28)

- Workload: `scripts/ui_scenarios/interaction-latency.json` on the Flutter test
  binding — ten repeats per block, timed to the visible state rather than to the
  harness's settle.
- Result: preview frame step forward/back **p50 5.2 ms, p95 14.8 ms** (inside the
  16.68 ms budget); Projects/Home screen switch p50 101.7 ms, p95 179.1 ms;
  export dialog open/cancel p50 133.6 ms, p95 165.3 ms.
- Re-run 2026-10-29 after the shell and timeline extraction: frame step
  **p50 5.4 ms, p95 12.8 ms**; Projects/Home switch p50 138.5 ms, p95 171.2 ms —
  the same order as the 2026-10-28 row, so the refactors did not move this path.
- Caveat: debug-lane wall clock, not a release or GPU claim. The edit-to-preview
  path is structural evidence (one seek, texture kept) plus the widget contract,
  not a separate wall-clock row yet.

### Media preview repaint scope (2026-10-29)

- Workload: the widget test `a preview for media that is not on the timeline
  repaints only the media pane` (six library-only items, three thumbnails
  finishing after a baseline).
- Result: the media pane repainted for every finished thumbnail while the
  timeline panel build count stayed flat, so library-only previews no longer
  rebuild the editor. Media used by a clip still repaints through the shell.
- Caveat: widget-test build instrumentation, not frame timing; it bounds the
  rebuild scope, not the paint cost.

### Hardware export from GPU NV12 planes (2026-10-27)

- Workload: the ignored `a_hardware_export_writes_from_gpu_nv12_planes` — a blank
  1920×1080, 30 fps, 4 s timeline (120 frames) exported to the MP4/H.264 profile,
  run three times per path on the same host.
- Source/build: `or_app_bridge` the commit that moved the colour conversion to
  the GPU, debug test profile.
- Codec/backend: VideoToolbox H.264, wgpu Metal composite and conversion.
- Machine: MacBook Air M5, 10 CPU cores, 8 GPU cores, idle-ish host.
- Result: GPU NV12 planes 1.868 s against packed RGBA plus swscale 4.447 s — 2.4×.
  The decoded picture of a blank frame is luma 16, chroma 128, i.e. limited-range
  black, and the container probes as `h264` 1920×1080, `color_range=tv`,
  `color_space=bt709`.

The saving is larger than the 2.02 ms swscale step alone because the packed path
also wrote an 8.3 MB RGBA frame back, swizzled it to BGRA and copied it into an
FFmpeg frame: 3.1 MB of planes replaces all of that.

### Delivery quality preference (2026-10-27)

- Workload: the ignored `a_delivery_export_honours_the_quality_preference` — a
  blank 640×360, 30 fps, 2 s timeline exported twice to WebM, once per quality.
- Source/build: `or_app_bridge` the commit that added the preference, debug test
  profile, software VP9.
- Machine: MacBook Air M5, idle host.
- Result: `speed` wrote 3,680 bytes and `quality` wrote 3,764 bytes. A blank
  frame is the hardest case for a rate-control difference; the earlier 1080p clip
  measurement (`good`/`cpu-used 4`/`crf 32` against `realtime`/`8`/`36`) was
  SSIM 0.9811 against 0.9778 at 8.81 MB against 8.40 MB, for 4.6× the encoding
  time. On a delivery export the render side is the slower half at 1080p, so most
  of that encoding time is hidden.

### Measured dead ends (2026-10-27)

Four candidates were measured on the same host and refused; recording them stops
the next attempt from repeating the work:

- **Assembly.** A local FFmpeg 8.1.3 built without `--disable-asm` decoded the
  same 1080p H.264 clip at 310.9 fps against 308.8 fps for the shipped
  `--disable-asm` build, and `yuv420p`→BGRA conversion came out within noise of a
  NEON build. The cost of that conversion is memory traffic, not missing SIMD, so
  the flag buys nothing at this size.
- **VideoToolbox decode.** Software 1080p H.264 decode already runs at 309 fps
  and spends 0.97 CPU-seconds on ten seconds of video — about a tenth of one
  core. VideoToolbox would remove most of that tenth, and the vendored build does
  not ship a hardware decoder; the plumbing needs raw `AVCodecContext` handling
  outside the safe binding. Not worth it at 1080p; revisit at 4K or HEVC.
- **Encoder thread count.** VP9 exports at 0, 4, 8 and 10 explicit threads land
  within 3.44–3.56 s of each other on the same 90-frame workload, because
  libvpx's own threading already scales through the frame budget. Setting a
  thread count changed nothing worth keeping.
- **Higher VP9 quality.** `deadline=good, cpu-used=4, crf=32` raises SSIM from
  0.9778 to 0.9811 on a 1080p clip but costs 4.6× the encode time (355 → 76.5
  frames/s) and about 10% of end-to-end wall time. The delivery defaults stay as
  they are until an export-quality decision asks for that trade.

What is left for the GPU is MotionScene rasterisation, which is still a CPU
rasteriser, and zero-copy surfaces between the renderer and the encoder. Both are
their own increments with their own evidence.

## Session-scoped invariants

`MediaPreviewQueue` and `MediaArtifactMailbox` belong to one project session:
both reset on every session swap (open, close, reopen), and
`_requestMediaPreview`'s `finally` only settles the submission counter when the
session is still identical. Any new owner of a session-scoped listener must be
reset in the same two sites, or the counter and the early-event map leak across
projects.

## Job slots fade with their job

`ExportFlow.isActive` is false once the flow has given up polling
(`_failed`), even though the client never saw a terminal job state: otherwise a
dead job keeps Export disabled and makes closing the project wait for work that
will never finish (round 137). The polling budget is also per job — `start()`
resets it. The same rule applies to any future job host: a superseded or
abandoned attempt releases the slot it claimed.

## macOS keychain bridge

`NarrationBridge` runs every keychain route (`get`, `has`, `set`, `delete`) on a
global queue and replies on the main thread: `SecItemCopyMatching` blocks the
caller while macOS asks for keychain access, and running it on the platform
thread froze the whole app when the prompt was never answered (traced with
`sample` on the hung process). The Dart side bounds each call to ten seconds and
fails with `KEYCHAIN_TIMEOUT`, so an unanswered prompt reads as a check that is
still pending instead of a frozen window.

A refresh that loses a generation race **returns without touching state**: the
newer refresh owns the page, tracks and error. Clearing them (or reporting the
race) emptied the library or flashed an empty timeline, which rounds 106, 111
and 112 removed.

## Harness environment gaps

- The app launched by `flutter build macos` + the harness lists **no installed
  `say` voices** ("No installed voice was found."), even though
  `VoiceoverService().listVoices()` returns `Linh (vi_VN)` under `flutter test`
  on this machine. The dialog now explains how to add a voice, but the harness
  app's inability to run `say` needs its own round. Round 121 ruled out two
  causes: the voice service now calls `/usr/bin/say` absolutely (GUI apps get a
  minimal PATH), and the harness build carries no `app-sandbox` entitlement —
  neither changed the result. Next hypotheses: the app's locale/environment
  makes `say -v ?` print a different table, or speech synthesis is gated by
  TCC in that launch context. `scripts/ui_scenarios/production-voice-music.json`
  stays voice-dependent until then. The service now throws ("say printed N
  rows") instead of returning an empty catalog, and the dialog shows the
  empty-state text; a diagnostic scenario that waits for that error text is the
  next cheap probe of the harness app's `say` output.

## Harness isolation

The integration harness launches the real app against the **user's** support
directory, so narration settings persist across runs: after a round that left
`provider: local`, `production-voice-music` opened on the local provider and
its `Linh (vi_VN)` wait timed out. Scenarios that assume a default provider must
select it explicitly (`voiceover-provider-system`). A tree captured right
after a tap also raced the build; the runner now settles before `tree`.

## Verification ideas not yet built

- A generation-gated fake (`summaryGate`, `mediaPageGate`) can pin the
  superseded-refresh branches behind a stale `listMediaPage`/`getProject` reply,
  which is the only way to assert the round 112 rule ("a losing refresh touches
  no state") end to end.

## Known gaps

0. Harness wall clock is not app latency. A temporary in-app stopwatch around
   `_openProjectLocation`/`_startProject` (file sink, one `reopen-twice` run on
   2026-10-29) read **recovery 4 ms, `openProject` 9-11 ms, summary+first
   media/timeline fetch 11-62 ms**, while the harness block timed the same
   `tap recent -> empty timeline` at **1454 ms**. The difference is the debug
   test binding: semantics lookups, `pump` loops and `wait` polling. Quote the
   harness numbers as a relative regression signal, never as user latency, and
   use in-app timers before claiming an open-path cost. Write those timers to a
   sandbox-writable sink (`path_provider` temp), not `/tmp`: the packaged app is
   sandboxed and a `/tmp` write fails silently, which looked like 'no timings'
   until the sink moved (round 157). The follow-up also showed the timer must
   sit in the *paging* function (`_refreshMediaPage`), not in a helper such as
   `_findMaterializedMedia` that only runs on one path — a timer in the wrong
   function produces "no numbers", which reads exactly like "no cost". With the
   probe finally in `_refreshMediaPage`, the page fetch is **0-6 ms** in app
   (17 calls over `editor-journey`: 0 ms once warm, 5-6 ms cold, library
   of 0-1 items). Paging is not a bottleneck; the harness-inflated numbers are
   a test-binding artifact, not UI cost. The export path now has its own
   end-to-end row (`export-encode`): dialog + real encode of the 1 s fixture
   measured 2933 ms (n=1, debug lane) through to `Export complete`, so a
   regression in the export lane finally has a scenario that fails.

0. The media page-two failure path is covered by a widget test
   (`widget_test.dart`, "a failed media page offers a retry that reloads it"):
   it drags the keyed `media-list` until `media-load-more` is built, fails the
   second page, asserts the footer error, then retries and asserts the error is
   cleared. A retry also puts the panel into its loading state before the
   summary is awaited (the error footer is replaced by the loading indicator),
   and "the media error retry shows a reload state" pins that: it holds the
   summary with `summaryGate`, taps retry, asserts the retry is gone and a
   progress indicator is shown, then completes the gate and asserts the error
   cleared.

1. Pacing has engine frame timings and now fails a scenario when a `frames`
   step crosses its `max_build_p95_ms` / `max_span_p95_ms` budget
   (`scripts/ui_scenarios/editor-frames.json`), but it still runs under the
   Flutter test binding: no presented-frame (vsync/compositor) budget and no
   measurement of a GPU-composited window.
2. Hardware encode exists for the MP4 profile and its colour conversion runs on
   the GPU (ADR 0011); hardware decode, shared GPU surfaces, a hardware path for
   VP9 and Android audio are not implemented.
3. Native/packaged claims for Windows, Linux and Android remain hosted-CI
   authority; a local result never becomes acceptance for them.
4. Representative camera-media compatibility, large libraries and
   physical-device performance are unproven.
5. Clean-machine install/upgrade/uninstall is unverified.
6. The largest sources are still single files — `apps/or_app/lib/shell/app_shell.dart`
   and `apps/or_app/lib/screens/editor_shell_preview_screen.dart`, and
   `crates/or_app_bridge/src/api/project.rs` still holds the opaque handle and most
   bridge methods after its view types moved to `crates/or_app_bridge/src/api/views.rs`
   (re-exported so the generated bridge keeps the same paths) and the media artifact
   requests moved to `crates/or_app_bridge/src/api/project_artifacts.rs`, the
   transcription surface to `crates/or_app_bridge/src/api/project_transcription.rs`
   and media import/relink/removal to `crates/or_app_bridge/src/api/project_media.rs` — and
   `crates/or_core/src/application` is a
   module whose `mod.rs` still carries request dispatch and the command catalog,
   while the suite is split into `crates/or_core/src/application/tests/` with one
   module per owner file. Extraction follows feature ownership, not line counts: `apps/or_app/lib/shell/platform_menus.dart`
   holds the macOS menu bar, `apps/or_app/lib/motion/motion_insert_log.dart` the
   motion insert record, `apps/or_app/lib/screens/editor_tool_actions.dart` the tool
   panel copy and action lists, `apps/or_app/lib/screens/editor_media_library.dart`
   the media library pane (items and empty/filter/error states), and
   `apps/or_app/lib/screens/editor_media_preview.dart` one item's preview states
   (pending, ready, failed-with-retry),
   `apps/or_app/lib/screens/editor_panel_header.dart` the shared tool-panel title
   row, `apps/or_app/lib/screens/editor_inspector_panel.dart` the clip and sequence
   inspector, `apps/or_app/lib/screens/editor_viewer_panel.dart` the preview,
   transport and scrub state, `apps/or_app/lib/screens/editor_frame_rate.dart` the
   shared sequence rate menu, `apps/or_app/lib/screens/editor_labels.dart` the time,
   speed and shortcut labels, `apps/or_app/lib/screens/editor_rational_time.dart`
   the seconds → canonical time conversion,
   `apps/or_app/lib/screens/editor_timeline_panel.dart` the timeline panel, its
   pointer/scrub gestures and clip context menu,
   `apps/or_app/lib/screens/editor_timeline_actions.dart` the clip context menu
   contents, the marker rename/delete flows and the actions dialog,
   `apps/or_app/lib/screens/editor_timeline_dialogs.dart` the trim, split, slip,
   boundary, move, speed and delete dialogs plus the text-clip editor,
   `apps/or_app/lib/screens/editor_timeline_shared.dart` the track labels, exact
   rational comparison and unit field the panel and its dialogs share,
   `apps/or_app/lib/screens/editor_timeline_painters.dart` the ruler, grid and
   placeholder painting,
   `apps/or_app/lib/screens/editor_media_preview.dart` the media library preview
   surface with its failed-state retry,
   and
   `apps/or_app/lib/production/audio_tool_flows.dart` the voiceover, recording and
   music workflows, `apps/or_app/lib/shell/export_flow.dart` the export job, its
   polling timer, the chosen destination and publishing the finished file,
   `apps/or_app/lib/production/media_import_flow.dart` the per-file import walk,
   its progress label and the closing batch statement,
   `apps/or_app/lib/shell/caption_import_dialog.dart` the caption-file review
   dialog (blank-skip, byte limit, explicit accept),
   `crates/or_core/src/application/types.rs` the command and
   query descriptors, envelopes, change sets, result records and error types,
   `crates/or_core/src/application/timeline.rs` the timeline track, clip and
   marker commands with their staging helpers, `crates/or_core/src/application/queries.rs`
   the read queries, `crates/or_core/src/application/media.rs` the media add,
   remove and relink commands, and `crates/or_core/src/application/history.rs`
   the undo and redo command, and `crates/or_core/src/application/tests/` one
   module per owner file over a shared support module, and
   `apps/or_app/lib/production/media_preview_kinds.dart` the preview kind and
   artifact-event vocabulary, `apps/or_app/lib/shell/speech_model_messages.dart`
   the speech-model failure explanations,
   `crates/or_app_bridge/src/api/views.rs` the bridge view types,
   `crates/or_app_bridge/src/api/project_artifacts.rs` artifact requests, early
   events and subscriptions, `crates/or_app_bridge/src/api/project_media.rs` the
   import walk, relink and removal, `crates/or_app_bridge/src/api/project_transcription.rs`
   the transcription model and job calls, and
   `crates/or_app_bridge/src/api/media_artifact_config.rs` the artifact cache
   location, worker-pool sizing and service construction. Next seams, in order:
   the project lifecycle and the media preview request path in `app_shell.dart` —
   bookkeeping moved to `apps/or_app/lib/production/media_artifact_mailbox.dart`,
   the publish choice to `media_preview_publisher.dart`, the artifact read to
   `media_preview_reader.dart`, the queue/drain rules to
   `media_preview_queue.dart`, the page-pruning and retry policy to
   `media_preview_plan.dart` (pure `planMediaPreviews` and `retryablePreview`),
   and the selection guard to `timeline_project_identity.dart`; the
   request/dispatch pair stays where it is on purpose — it binds the gateway,
   the event subscription, the mailbox, the session identity and the shell's
   setState together, so an extraction would trade one coupling for ten hooks
   into the same object, and the pair is already covered by shell tests. Next
   seams: the project lifecycle, then the timeline bridge methods in
   `crates/or_app_bridge/src/api/project.rs` (they interleave with stateful
   dispatch), then the remaining dispatch catalog in
   `crates/or_core/src/application/mod.rs`. The split may not change behaviour and lands under the
   existing tests.

## Evidence rules

- `scripts/check-repo.sh` is the source gate; `scripts/acceptance.sh` adds the
  documented Rust and Flutter lanes.
- A skipped, missing or unavailable check is `UNVERIFIED`, never PASS.
- Hosted CI is the only authority for native, packaged and cross-platform claims.
- Interface behaviour and interaction performance are part of the bar, not extras.

## Related

- [testing.md](testing.md)
- [operations/verification.md](../operations/verification.md)
