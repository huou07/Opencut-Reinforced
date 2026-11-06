# Hardware profiles

Last reviewed: 2026-11-07

Scope: what the product detects on a machine, which encoder an export will open
there, and why. A row is listed only after the running app probed the machine;
the measured codec/container matrix lives in
[media-capability-matrix.md](media-capability-matrix.md), and throughput in
[export-performance.md](export-performance.md).

## What is measured

`or_core::hardware` is pure policy. Callers hand it facts:

- `or_media::probe_h264_encoders()` opens every H.264 encoder the linked FFmpeg
  carries for a 64x64 NV12 frame — the same open the export performs, including
  the VAAPI frame pool and the CUDA device check for NVENC — and records the
  reason each candidate was skipped.
- `or_media::probe_decoders()` records which decoders the build carries.
- `or_render` reports the adapter the preview device actually uses: name,
  backend (Metal/Vulkan/Direct3D 12/OpenGL ES), device type and driver plus the
  compiled backends.

`PerformanceReportView` (bridge `performance_report()`) bundles those facts and
the plan for all four profiles. `encoding_plan(profile, preferred_encoder)`
returns one plan, so a dialog can name the encoder before a file is written.
The app shows the same data in **Settings → Performance** and in the export
dialog. The decision is made once per job when `ExportRequest::Start` is
handled; nothing in the frame loop probes hardware.

## Profiles

| Profile | Encoder order | Quality default |
|---|---|---|
| Recommended | First encoder that opened, hardware candidates first | Quality |
| Maximum performance | Same order, vendor hardware preferred | Speed |
| Balanced | Platform encoder (`videotoolbox`/`mf`/`vaapi`) first, then software | Quality |
| Compatibility | Software only; when none opened, the only encoder that did, with the reason recorded | Quality |

An explicit encoder choice in Settings wins over the order above, even under
Compatibility, and the plan notes it. When the chosen encoder cannot open, the
plan records `fallback` and uses the next encoder that did, instead of failing
the job. The resolved encoder is written into the job snapshot (`detail`, e.g.
`Encoding with h264_videotoolbox (hardware).`) and shown in the export receipt.

## Choosing a graphics backend

Settings → Performance also offers **Graphics backend**: Automatic (every
backend this build compiles) or one of the compiled names (`metal`, `vulkan`,
`dx12`, `gles`). The value is stored as `graphics_backend` in
`editor-settings.json`, read at startup before the first preview, and passed to
`or_render::RenderDevice::new_with_backends`. The device is opened once per
process, so a change is applied on the next app start and the panel says so.

A name the build does not compile, or a backend whose adapter fails to open,
falls back to automatic selection instead of leaving the app without a
renderer; the report prints the stored preference (`preferred_backend`) next to
the backend the preview actually opened, so a fallback is visible rather than
silent. The decoder and CPU-worker pickers are intentionally absent: decoding
has one automatic policy in this build, and the export render thread count is
an internal pipeline constant rather than a user preference.

## Verified rows

| Host | GPU / backend | Recommended plan | Compatibility plan | Evidence |
|---|---|---|---|---|
| MacBook Air M5, macOS 15 | Apple M5, Metal, integrated | `h264_videotoolbox` (hardware) | no software encoder in the build; falls back to VideoToolbox with the recorded note | `scripts/ui_scenarios/performance-profiles.json` PASS, receipt scenario PASS |
| Debian 13, Intel UHD 770 | Intel UHD Graphics 770, Vulkan (Mesa), integrated | `h264_vaapi` (hardware) | `libx264` (software) | `scripts/ui_scenarios/performance-profiles-linux.json` PASS on Weston headless with `--renderer=gl`; shots `shot-performance-recommended-4.png`, `shot-performance-compatibility-7.png` |
| Windows 11, Core 5 120U | Direct3D 12, integrated | `h264_mf` (hardware) | no software encoder in the build; falls back to Media Foundation with the recorded note | `scripts/ui_scenarios/performance-profiles-windows.json` PASS in an interactive session; desktop shots `shot-3..7.png` |

The Debian row needs the GPL-capable prefix (`~/or-build/ffmpeg-8.1.3-install`)
on `LD_LIBRARY_PATH`; the minimal LGPL prefix linked by default on that host
carries no H.264 encoder, and the panel correctly says so
(`no H.264 encoder opened on this machine`) rather than offering a profile that
cannot produce a file.

## What is not available in the packaged runtimes

The runtimes this mission measured link no oneVPL, so Quick Sync is not an
option without rebuilding FFmpeg with `libvpl`:

- macOS prefix: VideoToolbox family only (no libx264, no OpenH264).
- Debian minimal prefix: no H.264 encoder at all; the GPL prefix has
  `libx264`, `h264_vaapi` and `h264_nvenc`.
- Windows prefix: `h264_mf`, `h264_nvenc`; no `libx264`, no `h264_qsv`, no
  `h264_amf`.

`h264_nvenc` is present in the Debian and Windows builds but does not open on
either host because there is no NVIDIA device; the report records
`the device this encoder needs did not open`. Discrete NVIDIA and AMD GPUs have
no test hardware in this mission: their paths are implemented through the same
candidate list and probe, but no benchmark row is claimed for them.

## Windows verification

The interactive lane is proven: a Task Scheduler task with `/IT` runs in
session 1 as the logged-in user, not in the SSH session 0. `flutter test -d
windows` drove the real desktop window through the scenario and captured the
desktop while it ran.

The first run failed at step 5 with `Duplicate keys found ... multiple
children with key [<'performance-plan-notes'>]`: the panel keyed every plan
note identically, and Windows has two notes (`h264_nvenc` did not open, no
software encoder), so the plan block could not build. That was a real
Windows-only crash: the same panel built on macOS and Debian because those
hosts had one note. Each note now carries its own index key, the block keeps
one key, and a widget test covers a multi-note plan. The rerun passed in 9 s.

The packaged release `or_app.exe` also starts in session 1 with a real
`Opencut Reinforced` window (`Handle=329874`, `Exited=False` after 15 s,
desktop shot `shot-app.png`). That is launch evidence, not a second journey:
the semantic steps above ran against the test binding, which renders the same
widget tree on the same desktop.
