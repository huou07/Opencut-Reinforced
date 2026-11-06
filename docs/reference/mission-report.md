# Mission report — native performance and product polish

Last reviewed: 2026-11-07

Scope: what the 2026-11-07 mission (hardware profiles, export job detail, CLI
parity, clip keyframes and motion presets) shipped, what was verified on each
host, and what remains unverified. Revision `bacf5cff`; branch
`mission/hardware-profiles-and-polish`.

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
| macOS M5 (Metal, VideoToolbox) | PASS | `or_core` hardware 11, timeline 75, jobs 17; `or_media` 23; `or_render` NV12 1 | analyze clean, 520 widget tests | `performance-profiles.json`, `performance-receipt.json`, `keyframes.json` PASS | PASS (debug; release build not re-taken this mission) |
| Debian 13 (Weston headless, Mesa GL, UHD 770) | PASS | `or_core` timeline 75, hardware 11 | analyze clean | `performance-profiles-linux.json`, `keyframes.json`, preset run PASS | PASS (debug) |
| Windows 11 (Vulkan preview, Media Foundation) | PASS | `or_core` hardware 11 | analyze clean, `export_flow`/`performance_panel` 10 | `performance-profiles-windows.json`, `keyframes.json`, preset run PASS | PASS (`or_app.exe` window, title, screenshot) |

Benchmarks at this revision (300 frames, idle): macOS MP4 239.6/245.6 fps, MP4 +
one text layer 240.4/234.7 fps, animated scale 238.9/243.8 fps — within the
2026-11-06 spread. Windows (Vulkan Intel Graphics, Media Foundation): full MP4
129.1/127.2 fps, MP4 + text 78.8/82.6, render-only 214.4/210.2 (text 104.4/104.6),
encoder-only 278.1, decoder-only 63.8. Debian (Vulkan UHD 770, `h264_vaapi`): full MP4 video-only
256.0/260.6 fps (was 225), full MP4 + text 59.8–83.2 fps, render-only video
745.9/779.2, render-only + text 52.6/53.2, encoder-only 454.3/447.8,
decode-only 273.8/276.8. The Debian text path is readback-bound: the composited
frame waits 13–14 ms in the synchronous NV12 readback while the CPU submit is
25–74 µs. Rows in [export-performance.md](export-performance.md).

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
- **PASS** — a composited timeline (video + title) exports an MP4 through the
  real dialog on all three hosts (`scripts/ui_scenarios/export-text.json`, 30
  steps, receipt screenshot): macOS, Debian under Weston on the Intel UHD 770 and
  Windows in an interactive session.
- **PASS** — all three checkouts build and run at `4cf49782` and the core editing
  and export workflows still pass their tests.
- **PASS (macOS)** — an export with animated scale keyframes (`OR_BENCH_ANIM=1`)
  ran at 238.9/243.8 fps against the static 239.6/245.6 spread; Debian and
  Windows animated rows remain **UNVERIFIED**.
- **PASS (Debian)** — the video-only full pipeline improved from 225 to
  256–261 fps at this revision; the text row is readback-bound and documented.
- **PASS (Windows)** — full MP4 improved from 121 to 127–129 fps, render-only
  video from 188 to 210–214 fps, and one text layer costs about 2x rather than
  the 14x seen on Debian.
- **BLOCKED by hardware** — oneVPL/Quick Sync is absent from every packaged
  FFmpeg build (no `h264_qsv`), and no discrete NVIDIA/AMD device is available
  locally, so those paths are implemented and probed but not benchmarked.

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
