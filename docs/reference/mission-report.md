# Mission report — native performance and product polish

Last reviewed: 2026-11-07

Scope: what the 2026-11-07 mission (hardware profiles, export job detail, CLI
parity, clip keyframes and motion presets) shipped, what was verified on each
host, and what remains unverified. Revision `37f39a67`; branch
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
- **Motion presets.** Fade in, Slide up, Zoom in and Pop write ordinary
  keyframes through the same animation command, so a preset is a starting point
  whose chips stay editable and the project file never stores a preset name.
- **Harness fixes found by the above.** The UI harness gained `scroll_until`
  (long inspectors build rows lazily) and resolves `copy:` fixtures from the
  repository root; both were found while driving the keyframe journey.

## Evidence

| Host | Build | Rust tests | Flutter | UI journey | Packaged app |
|---|---|---|---|---|---|
| macOS M5 (Metal, VideoToolbox) | PASS | `or_core` hardware 11, timeline 75, jobs 17; `or_media` 23; `or_render` NV12 1 | analyze clean, 520 widget tests | `performance-profiles.json`, `performance-receipt.json`, `keyframes.json` PASS | PASS (debug; release build not re-taken this mission) |
| Debian 13 (Weston headless, Mesa GL, UHD 770) | PASS | `or_core` timeline 75, hardware 11 | analyze clean | `performance-profiles-linux.json`, `keyframes.json`, preset run PASS | PASS (debug) |
| Windows 11 (Vulkan preview, Media Foundation) | PASS | `or_core` hardware 11 | analyze clean, `export_flow`/`performance_panel` 10 | `performance-profiles-windows.json`, `keyframes.json`, preset run PASS | PASS (`or_app.exe` window, title, screenshot) |

Benchmarks at this revision (macOS only, 300 frames, idle): MP4 239.6/245.6 fps,
MP4 + one text layer 240.4/234.7 fps — within the 2026-11-06 spread. Rows in
[export-performance.md](export-performance.md).

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
- **PASS** — all three checkouts build and run at `4cf49782` and the core editing
  and export workflows still pass their tests.
- **PASS (macOS)** — an export with animated scale keyframes (`OR_BENCH_ANIM=1`)
  ran at 238.9/243.8 fps against the static 239.6/245.6 spread; Debian and
  Windows animated rows remain **UNVERIFIED**.
- **UNVERIFIED** — Debian and Windows benchmark rows were not re-taken at this
  revision; the last rows are from 2026-11-06.
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

The broader mission (perf audit of decode/composite/encode, 20-repo ideas,
Rust-quality review, animated-export measurement, final benchmark table) is not
finished; this report covers the four increments above and the evidence they
produced. See [roadmap.md](roadmap.md) for the queue.
