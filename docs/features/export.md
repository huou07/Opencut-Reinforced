# Feature — Export

Last reviewed: 2026-11-07

Scope: turning a project into a playable file — profiles, job behaviour, failure
semantics, and how an export is validated.

## Profiles

| Profile | Container | Video | Audio | Where |
|---|---|---|---|---|
| Lossless | Matroska | FFV1 | PCM S16LE, 48 kHz stereo | desktop, Android |
| Delivery | WebM | VP9 | Opus, 48 kHz stereo | desktop, Android (verified on emulator) |
| H.264 delivery | MP4 | H.264 (VideoToolbox, NVENC, Media Foundation or VAAPI; x264 fallback) | AAC, 48 kHz stereo | desktop |

MP4 is offered on every desktop. The core does not name one encoder: it walks a
preference list and commits to the first candidate the linked FFmpeg carries
*and* this machine can open. The order is `h264_videotoolbox`, `h264_nvenc`,
`h264_mf`, `h264_vaapi`, `h264_open`, then `libx264` — a name the package does
not carry is simply skipped, and a carrier is proved before it is used: NVENC
asks for a CUDA device, Media Foundation and VideoToolbox open on a 64×64 NV12
frame, and VAAPI builds its own device and frame pool and pushes a frame through
it. A machine that has no matching device therefore falls through to the next
encoder instead of failing the export. The macOS package carries VideoToolbox,
the Windows package carries NVENC and Media Foundation (the operating system's
own encoder, hardware-backed when the driver exposes a hardware MFT), and the
Linux package carries NVENC, VAAPI and x264, so MP4 export works on all three
without a platform branch in the profile code. Every package also carries AAC.
The same probe answers the UI: `or_media::describe_export_capability` reports
which profiles this build can write and which encoder it would use, the bridge
exposes it as `export_capabilities()`, and the app reads it once per launch.
The richer `or_media::probe_h264_encoders` report records every candidate the
build carries, whether it opened and why not, together with the decoders, the
GPU adapter and the backend the preview device uses; Settings → Performance
shows it and the export dialog names the planned encoder.
Consequences in the surface: the Export dialog lists only formats whose encoder
opens, starts on the engine's preferred profile instead of a hardcoded platform
default, and shows a warning line when MP4 is missing; Settings → Resources
lists MP4/WebM/lossless as Ready or Unavailable with the encoder name. A build
whose only working encoder is FFV1 says so rather than offering an MP4 export
that would fail.

### Small canvases and the Media Foundation floor

Every candidate is probed at 64x64, which is enough to tell "this machine can
open this encoder" but not "this machine can open it for *this* frame". Measured
on the Windows 11 host (2026-11-09, `h264_mf`): a 128x128, 64x64 and 48x48 MP4
export succeeds, and a 32x32 export fails at encoder open with
`Generic error in an external library`. The probe alone therefore picked
`h264_mf` for a canvas it cannot encode, and the failure surfaced as
"this machine has no working H.264 encoder" — a false statement on a machine
that had just encoded three files.

`or_core::h264_canvas_floor` carries the measured floor (Media Foundation:
48x48) and `apply_canvas_floor` marks a candidate as not opening, with the
reason, when the export's canvas is below it. The plan then chooses an encoder
that can really encode this frame; if none remains, the error names the canvas
that was too small instead of claiming the machine has no encoder.

### Encoder, not a performance profile

Performance has one mode: Automatic. The export dialog names the encoder the
machine will open instead of offering tiers to pick, and the line says whether
an override pinned it:

- The plan is the first encoder that answered a probe on this boot, with the
  operating-system encoder (VideoToolbox, Media Foundation, VAAPI) ahead of a
  vendor one. Settings → Performance shows the same plan with a reason for each
  part of the job (render, composite, decode, encode).
- Quality, resolution and codec stay in this dialog: they choose the file, not
  the machine. The retired `recommended`, `maximum_performance`, `balanced` and
  `compatibility` names still parse and all resolve to the one mode, so an old
  script or settings file keeps working.
- When this build cannot measure the machine, the dialog says so and the export
  still starts with the default encoder.

An advanced encoder override wins over the profile. If the named encoder did not
open, the job falls back to one that did and records the reason; it does not fail
on the name. `or_core::hardware::plan_h264_encoding` owns the policy,
`or_media::probe_h264_encoders` supplies the facts, and the bridge exposes
`performance_report()` and `encoding_plan()`. The decision travels on
`ExportRequest::Start.performance`, so the writer resolves it once when the job
starts and nothing re-probes inside the frame loop. A profile is a preference,
not a promise: every profile still requires an encoder that opened here.

The export job reports the encoder it actually opened in its snapshot `detail`
line (`ProjectExportJobView.detail`), including the fallback reason when the
planned encoder could not be used. The receipt shown after a finished export
prints that line, so a person can see which path wrote the file without reading
the console, and a profile regression becomes visible in the app rather than
only in a benchmark.

The quality preference reaches the hardware encoder too: a `Quality` export sets
the NVENC preset to `p5` and a `Speed` export to `p2`, so the choice trades time
for picture on the GPU instead of only changing the bitrate. Software encoding
keeps its own ladder (`x264` uses its defaults, VP9 uses the `cpu-used` table).

VAAPI is the Linux path for Intel and AMD devices. The writer builds the
`AVHWFrameContext` pool, attaches it to the encoder and uploads each composited
NV12 frame with `av_hwframe_transfer_data`, so a machine with
`/dev/dri/renderD128` and a VAAPI H.264 entrypoint encodes on the GPU. `libva`
stays a system library: the `.deb` declares it rather than bundling one driver
generation that would only match a single GPU family.

QSV and AMF are deliberately not listed. QSV needs the oneVPL dispatcher and, on
Linux, drives through the same VAAPI device, so VAAPI already covers Intel
there; AMF needs the AMD header SDK, and Media Foundation already reaches AMD
hardware on Windows. Both would add build dependencies without a test surface
the project can exercise, so they stay future work.

The Linux build links x264 because Linux has no operating-system H.264 encoder;
x264 is GPL-2.0-or-later, ships as `licenses/x264/COPYING` in the package, and
applies to that FFmpeg build only. See
[ADR 0011](../adr/0011-linux-delivery-encoder.md).

MOV, H.265/AV1 delivery and other hardware encoders remain future work gated on
licence, platform and build-configuration review. The package build enables only
the selected profile's muxer and encoders.

### Destination write, before any frame is rendered

A failed export used to surface as `Operation not permitted (os error 1)` — the
OS text for a folder the system refused, with nothing to act on. The writer
stages a hidden `.name.or-export-<pid>-<n>` directory beside the destination and
renames the finished file into place, so a folder that accepts the final file but
not a new directory is not a case the design needs to handle; a folder that
refuses both is now named:

- `or_media::check_export_destination` runs in the bridge's `start_export`
  before the job is queued. It rejects a directory, refuses a destination file it
  cannot open for writing without truncating it, and proves the parent accepts a
  new directory by creating and removing one. A refusal returns
  `EXPORT_DESTINATION_NOT_WRITABLE` with the folder path, so the dialog answers
  while it is still on screen instead of at the first frame.
- `describe_staging_failure` owns the wording for a permission refusal
  ("Choose a folder inside your home folder, such as Movies, or allow this app to
  write there in System Settings, then export again."), a missing folder, and
  anything else.
- The macOS save panel starts in `~/Movies`. Desktop, Documents and Downloads are
  withheld from an app until the owner allows it, and this build is ad-hoc signed
  so a rebuild can look like a new app to that decision; Movies needs no grant.
  The panel's return value grants nothing on its own — the app is not sandboxed,
  and folder access stays the system's to allow.
- A failure is repeated as a notice with a Dismiss action, because the top bar
  keeps one short line and drops its text entirely on a compact layout.

`scripts/ui_scenarios/export-blocked-destination.json` drives it: the scenario
flag `exportWriteDenied` hands the export a folder the runner made write-denied
(0500) and the journey waits for the folder-naming sentence and the remedy in the
live semantics tree. It is a macOS/Linux journey — `chmod` is the mechanism, so a
Windows run has no equivalent state to enter.

## Verification

`scripts/ui_scenarios/export-text.json` imports a clip, adds a title over it and
exports an MP4 through the real dialog, so the composited conversion, encoder
and job receipt are exercised together. It passes on macOS, Debian under Weston
with a real Intel GPU, and Windows in an interactive session.

## Behaviour

- Export captures one project revision and queues bounded background work with
  monotonic frame progress and cancellation.
- Preview and export share the same timeline evaluation, renderer, text
  rasteriser and transform/effect path; resolution, quality and scheduling may
  differ.
- The H.264 profile is fed NV12 planes: the composited frame is
  converted on the GPU and read back as two planes, so no CPU colour conversion
  runs per frame. The lossless and VP9 profiles keep the packed RGBA path.
- The two readback layouts are held together by a test rather than by
  inspection: `crates/or_render/tests/preview_export_parity.rs` renders one
  snapshot twice, as RGBA for the viewer and as NV12 for the hardware encoder,
  decodes the planes back to RGB and compares pictures (PSNR, ink centroid, black
  level). Measured at 320×180: **62.60 dB** on Apple M5 via Metal and the same
  62.60 dB on Debian's Intel UHD 770 via Mesa, so the agreement is not one
  driver's accident; mutating the expected limited-range black level to full
  range drops it to 23.96 dB, which is the failure the lane exists to catch.
- A clip that covers the whole canvas with no effect, transition, transform,
  crop or text on top is decoded straight to NV12 at the output size, so that
  frame never becomes an RGBA texture or a readback: the decoder runs one
  `swscale` pass and the planes go to the encoder. Any rotation on the source
  falls back to the composited path.
- The VP9 profile runs with its own row threading: the encoder is told how many
  threads the host can feed (capped at sixteen) and `row-mt` is on, because
  libvpx otherwise encodes on a single thread and an eight or sixteen core host
  waited on one core.
- Rendering and writing are pipelined: a render thread keeps at most two frames
  ahead of the writer, so decode, GPU composite and readback overlap colour
  conversion and encoding instead of alternating with them. Frames still reach
  the writer in index order and audio blocks are still written between them, so
  the file is the one a frame-at-a-time loop produces — same bytes, less wall
  time.
- Before the first frame, export decodes each timeline audio clip once into a
  timestamped in-memory cache. The writer then mixes from memory instead of
  reopening a decoder and polling for every video frame, which was the dominant
  cost on timelines with many short clips. The cache uses its own media budget,
  so a playing preview cannot starve it; the export pauses preview playback and
  releases its audio leases first. Timelines whose decoded audio exceeds the
  1 GB cache cap keep the streaming path rather than failing.
- The cache decodes with every usable core rather than a fixed four, sizes its
  in-flight budget from that worker count, and reserves each clip's estimated
  footprint before decoding so the workers cannot jointly overshoot the cap. A
  twelve-clip timeline went from 0.456 s to 0.158 s on the M5. See
  [export performance](../reference/export-performance.md) for the rows.
- Output is staged beside the destination and published only after streams and
  trailer complete. Cancellation, offline media or encode failure cleans staging,
  preserves an existing destination, and leaves a new destination absent.
- A lossy profile takes a quality preference: `quality` (the default) spends
  encoding time and bytes on a cleaner picture, `speed` takes the cheapest
  setting. The lossless profile ignores it because it has no rate control. The
  preference travels on `ExportRequest::Start` and defaults when an older client
  omits it. The export surface offers it as a row beside the profile, and the
  bridge takes it as an optional argument so an old caller keeps compiling.
- The chosen resolution is applied once to the output frame size; the project
  canvas and clip transforms are not rewritten by an export.
- The performance mode and an optional encoder name are optional fields on
  `ExportRequest::Start`. A client that omits them gets the candidate walk; a
  name that cannot open is a recorded fallback, not a failed request. Settings
  persists it in `editor-settings.json`.
- A directory destination is rejected before encoding starts.
- Export never mutates canonical project state or increments the revision.
- The export receipt names the produced file and offers Show in Finder.
- What the dialog offers is measured, not assumed: `ExportCapability.supports`
  filters the profile list, `preferredProfile` picks the default, and
  `ExportCapability.assumed()` keeps an export startable when the bridge cannot
  answer. A widget test mounts a build with no H.264 encoder and asserts the
  MP4 tile is absent, the caveat names the reason, and `Continue` still sends
  the WebM profile.

## Validation

An export is proven by an independent probe/playback of the actual file
(container, streams, duration, dimensions, expected frames), not by the export
job reporting success. See [operations/verification.md](../operations/verification.md).

On 2026-10-30 the reference 4:46 project (1920×1080, 30 fps, 8593 frames)
exported in 71 s with the audio cache and GPU NV12 path, versus 713 s (12.04
fps) before the cache; the MP4 probe read h264+aac, 286.433 s, and audio and
decoded video were bit-identical to the pre-NV12 export. Exporting with the
preview playing completed in 129 s; the pre-fix build failed immediately with
`runtime media budget exceeded`.

On 2026-11-04 the same 1080p NV12 path was measured on an Apple M5 (10 logical
cores, 8 GPU cores) with the export fixtures in `preview::desktop::export_pipeline_tests`
(`OR_BENCH_SOURCE`, `OR_BENCH_FRAMES`, `OR_BENCH_TEXT`, release build,
`DYLD_FALLBACK_LIBRARY_PATH` set to the pinned FFmpeg prefix):

- 1080p source → 1080p MP4, 300 frames: **1.3 s, 227–231 fps** across repeated runs.
- 4K source → 1080p MP4, 150 frames: **2.7 s, 54.9 fps**.
- 1080p source + one text clip → 1080p MP4, 300 frames: **4.0–4.2 s, 72–75 fps**.
- Ceiling rows on the same host: VideoToolbox H.264 blank frames 3.55 ms/frame
  (279.8 fps), NV12 decode 1.48 ms/frame (674.8 fps).

These are synthetic fixtures, not a full project export, and they describe this
host only. What they establish is where the pipeline is bound: a single-video
export is limited by the hardware encoder, so adding render threads or extra
cores does not move it, while a composited export is limited by the per-frame
GPU upload and readback. Two measured dead ends are recorded here so they are
not retried: concurrent render lanes do not scale (one shared `wgpu` device
serialises every frame at `poll(PollType::Wait)`; 2 and 4 lanes measured slower
than one), and the VideoToolbox `prio_speed`/`realtime` options leave throughput
unchanged or halve it.

`scripts/ui_scenarios/export-encode.json` drives the dialog + a real encode and
waits for `Export complete`; the produced file lands in
`apps/or_app/build/ui-harness/artifacts/replacements/export.mp4`. On 2026-10-29
`ffprobe` from the packaged app read it as **h264 32x24, 15 frames, aac, 0.512 s,
2639 bytes** — matching the fixture, so the lane is checked by a probe and not
by the job's own success flag. `python3 scripts/or_export_probe.py` repeats that
probe (finds the newest artifact, uses the packaged `ffprobe`, fails unless it
sees h264 + aac with positive duration and size, and fails when the video
stream's frame count does not cover the container duration — a job can report
success and still write a short clip).

## CLI and agent surface

`or export start --attach ENDPOINT.json --destination PATH` enqueues the same job
the Export dialog starts — same profiles, quality, short side and validation.
`--attach` takes the **endpoint descriptor** of a running project host (the file
the app writes under its support directory, or the one `or session serve
--descriptor` writes), not a project file; passing a project answers `local IPC
endpoint descriptor is invalid`. The host that answers must own an export
runtime: the desktop app does, and `or session serve` does not yet, so a CLI
export today attaches to the app. The destination extension selects the
container (`.mp4`, `.webm`, `.mkv`); an optional `--profile` must agree with it
and the path must be absolute. The
command waits for the terminal state; `--no-wait` returns the queued job. `or export
status --job ID` and `or export cancel --job ID` poll or stop it. Because the
queue is shared, a job the user started in the app can be polled by id. The
workspace guide is `docs/export-and-delivery.md`.

## Interchange

SRT/WebVTT subtitles are supported. `or captions export --out FILE.srt|.vtt`
collects every caption clip in start order with the same encoder the app uses.
OpenTimelineIO is a bounded Rust/serde JSON
adapter over a documented subset (never a second project format), with Lottie
1.0/dotLottie 2.0 as bounded interchange; EDL/XML are future. Importers do not
execute expressions or fetch remote assets.

## Related

- [architecture/media-pipeline.md](../architecture/media-pipeline.md)
- [text-captions.md](text-captions.md)
- [reference/security-licensing.md](../reference/security-licensing.md) — codec obligations
