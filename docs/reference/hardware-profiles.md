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
the plan. `encoding_plan(profile, preferred_encoder)` returns one plan, so a
dialog can name the encoder before a file is written.
The app shows the same data in **Settings → Performance** and in the export
dialog. The decision is made once per job when `ExportRequest::Start` is
handled; nothing in the frame loop probes hardware.

## The one mode

There is no performance tier to pick. **Automatic — best for this machine** is
the only mode, and it is what the product means by "performance":

| Part of the job | The plan | Why |
|---|---|---|
| Preview and render | The adapter and backend the preview device opened | It is the device frames are already on |
| Composite | The same device | A second device would copy every frame between them |
| Decode | Software (the decoders this build carries) | See the decode rule below |
| Encode | The first encoder that answered a probe on this boot, platform encoder (`videotoolbox`/`mf`/`vaapi`) before a vendor one | Encoding on the device is the whole point of the hardware path |

Quality is not part of the mode. The export dialog keeps quality, bitrate,
codec and compression speed because those choose the file, not the machine.

**Encode choice.** An explicit encoder chosen in Settings → Performance wins
when it opened; a name that cannot open records `fallback` and the next encoder
that did, instead of failing the job. The resolved encoder is written into the
job snapshot (`detail`, e.g. `Encoding with h264_videotoolbox (hardware).`) and
shown in the export receipt.

**Decode rule.** A hardware decoder that hands frames back through host memory
(copy-back) costs more than it saves: the frame is decoded on the device, copied
to the CPU, then uploaded again for compositing. This build therefore decodes on
the CPU and the panel says so. The rule is measured, not assumed — a decode path
that keeps frames device-resident would be selected instead; nothing selects a
hardware decoder by name alone.

**Retired names.** `recommended`, `maximum_performance`, `balanced` and
`compatibility` still parse, so a settings file, script or agent that carries one
keeps working; they all resolve to the one mode. The app never writes them.

**When the plan is decided.** Once per export request and once when Settings →
Performance is opened. Nothing probes hardware on launch, in the frame loop, or
per frame. The plan costs a probe-cached encoder open, not a benchmark run.

**Hardware fingerprint.** The panel stores what the plan was decided from
(processor, graphics device, driver, compiled backends, decoders) as
`hardware_fingerprint` with `hardware_checked_at` in `editor-settings.json`. A
machine that no longer matches it — a new GPU, a driver update, an OS upgrade —
is reported in the panel and decided again. **Re-detect** repeats the
measurement on demand, and **Reset to Automatic** clears every override.

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

| Host | GPU / backend | Automatic plan | Evidence |
|---|---|---|---|
| MacBook Air M5, macOS 15 | Apple M5, Metal, integrated | `h264_videotoolbox` (hardware) | `scripts/ui_scenarios/performance-profiles.json` PASS, receipt scenario PASS |
| Debian 13, Intel UHD 770 | Intel UHD Graphics 770, Vulkan (Mesa), integrated | `h264_vaapi` (hardware) | `scripts/ui_scenarios/performance-profiles-linux.json` PASS on Weston headless with `--renderer=gl` |
| Windows 11, Core 5 120U | Direct3D 12, integrated | `h264_mf` (hardware) | `scripts/ui_scenarios/performance-profiles-windows.json` PASS in an interactive session; desktop shots `shot-3..7.png` |

The tier scenarios above ran while the four profiles still existed; the same
scenario files now drive the one mode, and their shots are re-recorded with the
redesigned panel (the rows keep their measurements, the labels changed).

The Debian row needs the GPL-capable prefix (`~/or-build/ffmpeg-8.1.3-install`)
on `LD_LIBRARY_PATH`; the minimal LGPL prefix linked by default on that host
carries no H.264 encoder, and the panel correctly says so
(`no H.264 encoder opened on this machine`) rather than offering a profile that
cannot produce a file.

## Encoder settings chosen per host (2026-11-08)

The plan picks an encoder; these are the settings the picked encoder is opened
with, and each one is either a measured win or a probe-guarded default:

| Setting | Hosts | Rule |
| --- | --- | --- |
| VAAPI `async_depth` | Debian | 4. The wrapper default of 2 idles the GPU between frames; 8 and 16 re-measured no better. `OR_VAAPI_ASYNC_DEPTH` overrides. |
| VAAPI `low_power` | Debian | 1 when `vaapi_low_power_available()` opens a 64×64 `h264_vaapi` with the option and sends a frame (cached probe). VDENC is +9 % on the UHD 770 at equal app-level quality. A driver without VDENC never sees the flag. |
| `h264_vaapi` quality | Debian | `quality=7` for a Speed export, `quality=2` for Quality; without it the driver uses its default for both. |
| VideoToolbox | macOS | No tuning knob is set; the encoder is not the pipeline's limit on the M5 (3.1 ms/frame against 1.5 ms decode). |
| Media Foundation | Windows | No tuning knob is set. This host is render-thread-bound, not encoder-bound: decode costs 7.3 ms inside the export against 4.8 ms standalone, so the encoder settings are not where its missing throughput is. |
| Decoder threads | all | FFmpeg's automatic frame threading (`OR_DECODE_THREADS=0`). Forcing 2 threads costs ~10 %; 4 is inside the run-to-run spread. |

`OR_VAAPI_OPTIONS=key=value,...` overrides any VAAPI driver option for an
experiment without a rebuild; `OR_EXPORT_TRACE=1` prints the per-stage medians
that justify the table above.

## What is not available in the packaged runtimes

The runtimes this mission measured link no oneVPL, so Quick Sync is not an
option without rebuilding FFmpeg with `libvpl`. The packaged configure lines
say so directly: the Debian prefix passes `--enable-vaapi
--enable-hwaccel=h264_vaapi --enable-encoder=h264_vaapi --enable-ffnvcodec
--enable-nvenc --enable-encoder=h264_nvenc` and no `--enable-libvpl`, and the
Windows prefix passes `--enable-encoder=h264_mf --enable-nvenc
--enable-encoder=h264_nvenc` and no `--enable-amf`. FFmpeg 8 has no
`--enable-libmfx`; `h264_qsv` exists only when the build links oneVPL.

A `libvpl` rebuild was produced and probed on the Debian host on 2026-11-07
(`~/or-build/ffmpeg-qsv-install`, `libvpl-dev` 2.14.0, `intel-media-va-driver`
25.2.3). It compiles and advertises `h264_qsv`, but every session fails at
device creation: `Use Intel(R) oneVPL to create MFX session, API version is
2.14` then `Error creating a MFX session: -9` and
`Device creation failed: -1313558101`. On this integrated UHD 770 the VAAPI
path opens and encodes, so shipping QSV would add an encoder that cannot run
here, and encode is not the bottleneck anyway (the blank-frame encoder ceiling
is 454 fps against a 201 fps composited export). Quick Sync therefore stays out
of the packaged builds; the probe build is kept as the evidence.


- macOS prefix: VideoToolbox family only (no libx264, no OpenH264).
- Debian minimal prefix: no H.264 encoder at all; the GPL prefix has
  `libx264`, `h264_vaapi` and `h264_nvenc`.
- Windows prefix: `h264_mf`, `h264_nvenc`; no `libx264`, no `h264_qsv`, no
  `h264_amf`.

`h264_nvenc` is present in the Debian and Windows builds but does not open on
either host because there is no NVIDIA device; the report records
`the device this encoder needs did not open`. Discrete NVIDIA and AMD GPUs have
no test hardware in this mission: their paths are implemented through the same
candidate list and probe, but no benchmark row is claimed for them. AMD AMF is
not compiled into any prefix (no `--enable-amf`), so on an AMD host the plan
falls back the same way an unopenable candidate does: the encoder is never
offered, the reason is recorded, and the software encoder is used.

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
