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

## Known gaps

1. No presented-frame (vsync/compositor) budget; pacing is read from the app's own
   log, not a dedicated gate.
2. Hardware encode exists for the MP4 profile and its colour conversion runs on
   the GPU (ADR 0011); hardware decode, shared GPU surfaces, a hardware path for
   VP9 and Android audio are not implemented.
3. Native/packaged claims for Windows, Linux and Android remain hosted-CI
   authority; a local result never becomes acceptance for them.
4. Representative camera-media compatibility, large libraries and
   physical-device performance are unproven.
5. Clean-machine install/upgrade/uninstall is unverified.
6. The largest sources are still single files — `apps/or_app/lib/shell/app_shell.dart`,
   `apps/or_app/lib/screens/editor_shell_preview_screen.dart` and
   `crates/or_app_bridge/src/api/project.rs` — and `crates/or_core/src/application` is a
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
   pointer/scrub gestures, clip context menu and text-clip editing,
   `apps/or_app/lib/screens/editor_timeline_painters.dart` the ruler, grid and
   placeholder painting, and
   `apps/or_app/lib/production/audio_tool_flows.dart` the voiceover, recording and
   music workflows, `apps/or_app/lib/shell/export_flow.dart` the export job, its
   polling timer, the chosen destination and publishing the finished file,
   `apps/or_app/lib/production/media_import_flow.dart` the per-file import walk,
   its progress label and the closing batch statement,
   `crates/or_core/src/application/types.rs` the command and
   query descriptors, envelopes, change sets, result records and error types,
   `crates/or_core/src/application/timeline.rs` the timeline track, clip and
   marker commands with their staging helpers, `crates/or_core/src/application/queries.rs`
   the read queries, `crates/or_core/src/application/media.rs` the media add,
   remove and relink commands, and `crates/or_core/src/application/history.rs`
   the undo and redo command, and `crates/or_core/src/application/tests/` one
   module per owner file over a shared support module, and
   `crates/or_app_bridge/src/api/media_artifact_config.rs` the artifact cache
   location, worker-pool sizing and service construction. Next seams, in order:
   the project lifecycle and the queue behind the new
   `apps/or_app/lib/production/media_preview_kinds.dart` helpers in
   `app_shell.dart`, then
   the artifact event forwarding and request path in
   `crates/or_app_bridge/src/api/project.rs`, then the remaining dispatch
   catalog in `crates/or_core/src/application/mod.rs`. The split may not change behaviour and lands under the
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
