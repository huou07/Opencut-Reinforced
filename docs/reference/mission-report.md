# Mission report — native performance and product polish

Last reviewed: 2026-11-07

Scope: what the 2026-11-07 mission (hardware profiles, export job detail, CLI
parity, clip keyframes, motion presets and agent animation verbs) shipped, what was verified on each
host, and what remains unverified. Branch `mission/hardware-profiles-and-polish`.

Three revisions appear in this report and they are not interchangeable:

- `bacf5cff0150757b0a74fd416e64ed02ef235cd5` is the revision the export rows
  below were recorded at (`perf(export): record the Windows benchmark rows at
  the mission revision`).
- `20543449ec39b52d209d58d6393d7355f45da608` is the revision the closing
  verification ran at (`chore: remove the two unused-mut warnings the checks
  reported`).
- `f1db393cac65c5285780a2b82d3b8d1e95fcd9e3` is the tip that was synced to the
  Debian and Windows checkouts; `git diff 20543449..f1db393c` touches only this
  file, so the three hosts built and tested identical code.

## Shipped

- **Hardware capability report and profiles.** `or_core::hardware` holds the
  pure policy; `or_media::probe_h264_encoders` opens every H.264 candidate the
  build carries (including the VAAPI pool and the CUDA device), `probe_decoders`
  lists the decoders, and `or_render` reports the adapter, backend and device
  type of the device preview and export actually use. Four profiles resolve to
  one encoder with a recorded fallback reason; the choice is made once per
  export job in `ExportRequest::Start.performance`, never in the frame loop.
  Settings → Performance shows the machine, the four profiles, an encoder
  override and a reset; the export dialog names the encoder; the job snapshot
  reports the encoder it opened; `or export start --performance/--encoder`
  carries the same choice for the CLI.
- **Clip keyframes.** Position, scale, rotation and opacity are keyframable per
  clip with linear and three cubic easings, stored as canonical milli-units on
  the visual settings and sampled in the shared frame loop, so preview and
  export animate identically. The inspector adds a keyframe at the playhead and
  edits time, value and easing in the unit the rest of the inspector uses.
- **Per-device conversion choice.** The renderer times the compute conversion
  and the two render passes once on the first composited frame and keeps the
  faster one for the device; Debian keeps the compute pass, Windows the render
  passes.
- **Motion presets.** Fade in, Slide up, Zoom in and Pop write ordinary
  keyframes through the same animation command, so a preset is a starting point
  whose chips stay editable and the project file never stores a preset name.
- **Graphics backend preference.** Settings → Performance stores a backend
  (`auto` or a compiled name) and the app passes it to Rust before the first
  preview, so a machine whose default adapter misbehaves can pick another one;
  an unusable choice falls back to automatic and the report shows both values.
- **Agent animation verbs.** `or timeline add-keyframe` (channel, exact offset,
  value, easing) and `or timeline clear-animation` write the same clip update as
  the inspector, and re-adding an offset replaces that keyframe instead of
  duplicating it. The MCP server exposes `add_keyframe` and `clear_animation`, so
  an agent can build a title animation the same way a person does.
- **Timeline discoverability.** A clip with keyframes shows a diamond badge in
  its timeline block and says "animated" in its accessibility label, so the
  feature is visible without opening the inspector. Verified at `07d91f64` on
  all three hosts in the `keyframes` journey, whose tree dump contains
  `, animated` for the edited clip.
- **One-pass NV12 conversion.** The composited export path converts its RGBA
  target with one compute shader into a tight NV12 buffer instead of two render
  passes and two padded copies, which doubled the composited ceiling on the
  Debian Intel host (52.6 → 109–112 fps render, 59.8–83.2 → 106–112 fps full MP4
  with one text layer). macOS is unchanged within spread; the Windows Intel
  driver runs the compute version about 8 % slower on that workload, recorded as
  a measured tradeoff with a per-device follow-up.
- **Six easings.** Overshoot (back) and Bounce join the four cubic curves, in
  the core model, the bridge/CLI spelling, the inspector dialog (six chips, no
  menu) and the export path. The dialog shows every curve at once, and the
  keyframes journey edits a keyframe to Overshoot and reads
  `0.00s · 50.0% · Overshoot` back on the real inspector on macOS, Debian
  (Weston/Intel GPU) and Windows (interactive session).
- **Templates on Home.** The four most-used starter shapes sit under Home's
  quick actions with an "All templates" link, so a first-time user can start
  from a real project shape without discovering the Library tab first. Picking
  one reuses the ordinary create-project path, and the supplied overlays are
  ordinary text clips. Proven by the `home-template` journey on all three hosts.
- **Harness fixes found by the above.** The UI harness gained `scroll_until`
  (long inspectors build rows lazily) and resolves `copy:` fixtures from the
  repository root; both were found while driving the keyframe journey.

## What the twenty reviewed repositories contributed

No runtime dependency was added; the ideas below were reimplemented in Rust and
Dart inside the existing architecture.

| Idea | Source | What shipped | What stayed deferred |
|---|---|---|---|
| Easing vocabulary and motion presets | GSAP, GSAP skills (MIT text) | Fade in, Slide up, Zoom in and Pop write ordinary keyframes with the four inspector easings; the presets are authored in OR, not copied runtime behaviour | A curve/graph editor |
| Keyframe timeline editing | Motion, Anime.js | Add at playhead, edit time/value/easing, delete, Copy/Paste between clips, deterministic sampling in the Rust frame loop | Stagger across several selected clips |
| Composition templates and scene sequencing | Remotion, Motion Canvas | Already covered by MotionScene; the mission reused it instead of adding a second scene model | A template gallery built on MotionScene assets |
| Typography and vector workflow | Satori, SVGO | Existing text presets and the scene typography schema stay the native path | A Satori-style layout generator; no SVG import pipeline exists |
| Animated vector assets | Rive, dotLottie | Not adopted: the MIT option (`dotlottie-rs`) needs a second asset pipeline | Lottie import behind a capability check, only if user demand appears |
| 3D | Three.js, R3F, Drei | Not adopted: a sidecar renderer would be a second pipeline with no offline-deterministic path today | A separate 3D tool producing a clip |
| Audio preview and media workflow | Howler.js, Mediabunny | Rust decode/playback and the export path already cover it | Nothing |

## Evidence

| Host | Build | Rust tests | Flutter | UI journey | Packaged app |
|---|---|---|---|---|---|
| macOS M5 (Metal, VideoToolbox) | PASS | workspace lib tests 706 pass / 10 ignored (incl. `or_core` 398, `or_render` 52, `or_media` 23, `or_app_bridge` 57); `or_cli` 45 semantic + 4 MCP | analyze clean, 526 widget tests | `performance-profiles.json`, `performance-receipt.json`, `keyframes.json`, `home-template.json`, `export-text.json` PASS | PASS (debug; release build not re-taken this mission) |
| Debian 13 (Weston headless, Mesa GL, UHD 770) | PASS | `cargo check --workspace --all-targets` at `20543449`; `or_core` timeline 77; `or_render` NV12 4; `or_cli` keyframe verb | analyze clean | `performance-profiles-linux.json`, `keyframes.json`, `home-template.json`, `export-text.json` PASS | PASS (debug) |
| Windows 11 (Vulkan preview, Media Foundation) | PASS | `cargo check --workspace --all-targets` at `20543449`; `or_render` NV12 5; `or_core` hardware 11 | analyze clean, `export_flow`/`performance_panel` 10 | `performance-profiles-windows.json`, `keyframes.json`, `home-template.json`, `export-text.json` PASS | PASS (`or_app.exe` window, title, screenshot) |

Benchmarks at this revision (300 frames, idle): macOS MP4 239.6/245.6 fps, MP4 +
one text layer 240.4/234.7 fps, animated scale 238.9/243.8 fps — within the
2026-11-06 spread. Windows (Vulkan Intel Graphics, Media Foundation): full MP4
129.1/127.2 fps, MP4 + text 78.8/82.6, render-only 214.4/210.2 (text 104.4/104.6),
encoder-only 278.1, decoder-only 63.8. Debian (Vulkan UHD 770, `h264_vaapi`): full MP4 video-only
256.0/260.6 fps (was 225), full MP4 + text 101.9–118.3 fps after the one-pass
compute conversion (was 59.8–83.2), render-only video 745.9/779.2,
encoder-only 454.3/447.8, decode-only 273.8/276.8. The composited path still
waits 6–14 ms in the synchronous NV12 readback, so it stays the ceiling on this
iGPU. Rows in [export-performance.md](export-performance.md).

## Claims, stated as evidence allows

- **PASS** — the four profiles resolve against the encoders this machine actually
  opens, fall back with a reason, and the export receipt names the encoder used
  (macOS journey + unit tests on all three hosts).
- **PASS** — keyframes are authored in the real inspector, written through the
  ordinary clip command and read back (`0.00s · 50.0% · Linear`) on macOS, Debian
  under Weston with a real Intel GPU, and Windows in an interactive desktop
  session.
- **PASS** — applying Slide up writes ordinary keyframes and the chip reads back
  (`0.00s · 0.12 · Ease out`) on all three hosts in the same journey.
- **PASS** — one undo removes a motion preset's keyframes, checked in the
  `keyframes` journey after the Slide up chip appears.
- **PASS** — a composited timeline (video + title) exports an MP4 through the
  real dialog on all three hosts (`scripts/ui_scenarios/export-text.json`, 30
  steps, receipt screenshot): macOS, Debian under Weston on the Intel UHD 770 and
  Windows in an interactive session.
- **PASS** — all three checkouts build and run at `20543449` (`cargo check
  --workspace --all-targets` on Debian and Windows, workspace lib tests on
  macOS) and the core editing and export workflows still pass their tests.
- **PASS (all three)** — an export with animated scale keyframes
  (`OR_BENCH_ANIM=1`) stays inside the static spread: macOS 238.9/243.8 vs
  239.6/245.6; Debian 260.6 video and 96.7–119.2 with text; Windows 126.9 video
  and 80.0 with text vs 78.8–88.0 static.
- **PASS (Debian)** — the video-only full pipeline improved from 225 to
  256–261 fps at this revision; the text row is readback-bound and documented.
- **PASS (Windows)** — full MP4 improved from 121 to 127–129 fps, render-only
  video from 188 to 210–214 fps, and one text layer costs about 2x rather than
  the 14x seen on Debian.
- **BLOCKED by hardware** — oneVPL/Quick Sync is absent from every packaged
  FFmpeg build (no `h264_qsv`), and no discrete NVIDIA/AMD device is available
  locally, so those paths are implemented and probed but not benchmarked.

## Closing verification at `20543449`

- `cargo test --workspace --lib`: 706 passed, 0 failed, 10 ignored (the ignored
  ones are the media/export benchmarks that need `OR_BENCH_*` and a source).
- `cargo check --workspace --all-targets`: no warnings beyond the
  `block 0.1.6` future-incompatibility note; the two unused-`mut` warnings from
  earlier revisions are gone.
- `flutter analyze --no-pub`: “No issues found!”; `flutter test`: 526 passed.
- `./scripts/check-repo.sh`: PASS (913 tracked files).
- Debian and Windows checkouts fast-forwarded to the same code and compile;
  their journeys at the same feature set pass (home-template, keyframes/easing,
  composited export), and the final `home-template` run passed on both after the
  last sync.
- Nothing was pushed, tagged or released; the mission left 83 local commits on
  `mission/hardware-profiles-and-polish` over `main`.

## Queued NV12 readback (2026-11-07, later than the rows above)

The composited export used to block the render thread on its own readback:
`composite_layers_nv12` submitted the conversion, waited for that submission
with `PollType::Wait`, repacked both planes and only then returned, so decode,
text rasterisation and the GPU conversion never overlapped. The renderer now
queues that work: `composite_layers_nv12_pending` submits the conversion and the
buffer mapping and returns a `PendingNv12Planes`, the export channel carries the
`Presentation` itself, and the writer thread resolves it before handing the
planes to the encoder. A dropped frame waits for its own submission and unmaps
its buffer in `Drop`, so a pooled staging buffer is never handed out mapped.

Measured as an interleaved A/B in the same session on each host, parent
`69aa1b31` against `02cd025c`, 300 frames, 1080p, one text layer, `--release`:

| Host | Waited readback | Queued readback | Output |
|---|---|---|---|
| Debian 13, UHD 770 (Vulkan, `h264_vaapi`) | 107.9 fps | 190.6 / 193.6 / 198.0 / 201.2 fps | byte-identical, `md5 540f516a…9eb164` |
| Windows 11, Intel Graphics (Vulkan, Media Foundation) | 81.7 / 82.6 fps | 105.2 / 105.2 fps | identical frames |
| macOS M5 (Metal, VideoToolbox) | 179.9 / 196.4 / 212.9 fps | 187.3 / 190.6 / 215.8 / 220.6 fps | `PSNR y:inf`, identical pictures |

The Debian composited row is 1.77× faster with an unchanged file; Windows gains
27 %; macOS is unchanged inside its ±8 % spread. Stage rows are in
[export-performance](export-performance.md#2026-11-07-rows--queued-nv12-readback).

The same session closed two other questions with measurements instead of
arguments:

- **Hardware decode** is slower than software decode on both Intel hosts when
  the frames must come back to CPU memory, which is what this compositor needs:
  VA-API + `hwdownload` 610 fps against 743 fps software on Debian, D3D11VA
  253 fps against 264 fps on Windows. Not integrated.
- **Quick Sync.** The packaged FFmpeg builds pass no `--enable-libvpl`, which is
  why no `h264_qsv` exists in them. A oneVPL build was produced and probed on
  the Debian host; device creation fails with `Error creating a MFX session:
  -9` on this UHD 770, so QSV would ship as an encoder that cannot run. AMF is
  not compiled into any prefix at all.

## Rust quality review

Reviewed the new policy, keyframe and bridge code against the Rust API
Guidelines and the `rust-review` checks, and fixed what had a real cost:

- the frame loop skipped animation sampling entirely when a clip has no
  keyframes, instead of walking six empty tracks per layer per frame;
- a duplicated `#[allow(dead_code)]` on `AudioClip::playback_rate`;
- `probe_decoders` sat after the test module in `or_media/src/decoder.rs`;
- a nested `if` in the plan-note loop of `or_core/src/hardware.rs`;
- `ProjectHostHandle::start_export` now states why it carries nine optional
  arguments instead of silently tripping the lint.

`cargo fmt --check` is clean. Default clippy on `or_core`, `or_media` and
`or_app_bridge` leaves one pre-existing warning (`ClipSettings` variant size);
the workspace-wide pedantic run is still noisy from older code and was not
turned into a mass refactor.

## Remaining work

The broader mission (full audit of decode/composite/encode, 20-repo ideas beyond
the motion presets, cross-host benchmark rows at this revision, and a wider UX
pass) is not finished; this report covers the six increments above and the
evidence they produced. See [roadmap.md](roadmap.md) for the queue.

## Remaining, with the reason

- **Async or GPU-resident readback** — the queued readback shipped (see above)
  and removed the per-frame stall: the render thread no longer waits for its own
  conversion. What is still not done is keeping the frame on the GPU: the planes
  still travel through system memory into FFmpeg. That needs a VAAPI/D3D11
  surface the compositor can write and the encoder can read; wgpu has no such
  interop, so the encoder boundary stays CPU-side with a measured, queued path
  instead of an unmeasured zero-copy claim.
- **Decoder and CPU-worker choice** — the graphics backend now has a stored
  picker (next start, automatic fallback); a decoder picker still needs a
  hardware-decode path the product declined until it is measured, and an
  explicit CPU-worker count would fight the runtime's own scheduling, so both
  are recorded as open rather than faked.
- **Discrete NVIDIA/AMD (NVENC, AMF)** — probed and probed-off paths exist, but no
  such device is attached to the three hosts, so no benchmark is claimed.
- **Template gallery on MotionScene** — the 20-repo table lists it as deferred;
  the library still starts empty and presets cover the beginner path.
- **Masks, graph editor, 3D and vector import** — each is a numbered product
  decision, not a silent omission: masks and the graph editor are PLANNED in
  `docs/reference/product.md`, 3D and Lottie/SVG import were declined in the
  20-repo table because each would add a second renderer or asset pipeline with
  no offline-deterministic path today. Crop, the closed effect set, keyframes
  and text presets cover the same intents on the current renderer.
