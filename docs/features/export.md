# Feature — Export

Last reviewed: 2026-11-02

Scope: turning a project into a playable file — profiles, job behaviour, failure
semantics, and how an export is validated.

## Profiles

| Profile | Container | Video | Audio | Where |
|---|---|---|---|---|
| Lossless | Matroska | FFV1 | PCM S16LE, 48 kHz stereo | desktop, Android |
| Delivery | WebM | VP9 | Opus, 48 kHz stereo | desktop, Android (verified on emulator) |
| Hardware H.264 | MP4 | H.264 (VideoToolbox) | AAC, 48 kHz stereo | macOS desktop |

MP4 with hardware H.264/AAC is enabled on macOS. MOV, H.265/AV1 delivery and
other hardware encoders remain future work gated on licence, platform and
build-configuration review. The package build enables only the selected
profile's muxer and encoders.

## Behaviour

- Export captures one project revision and queues bounded background work with
  monotonic frame progress and cancellation.
- Preview and export share the same timeline evaluation, renderer, text
  rasteriser and transform/effect path; resolution, quality and scheduling may
  differ.
- The hardware H.264 profile is fed NV12 planes: the composited frame is
  converted on the GPU and read back as two planes, so no CPU colour conversion
  runs per frame. The lossless and VP9 profiles keep the packed RGBA path.
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
- A directory destination is rejected before encoding starts.
- Export never mutates canonical project state or increments the revision.
- The export receipt names the produced file and offers Show in Finder.

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

`or export start --attach project.json --destination PATH` enqueues the same job
the Export dialog starts — same profiles, quality, short side and validation.
The destination extension selects the container (`.mp4`, `.webm`, `.mkv`); an
optional `--profile` must agree with it and the path must be absolute. The
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
