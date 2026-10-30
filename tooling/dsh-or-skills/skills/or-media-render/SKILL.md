---
name: or-media-render
description: Add or fix OR media import, decode/playback, audio, compositing, FFmpeg, export or media-performance behavior. Use for ordinary non-AI media pipelines. Do NOT use for timeline command semantics (or-edit-feature), UI review (or-ui-ux), declarative MotionScene authoring (or-motionscene), local verification mechanics (or-dev-loop), or AI/provider work.
---

# OR media and rendering

Keep media behaviour inside the existing Rust media, audio and render layers,
with Flutter as presentation. Read `docs/architecture/media-pipeline.md`,
`docs/architecture/realtime-plane.md` and the owning feature document from
[AGENTS.md](AGENTS.md).

## Boundaries

- Reuse `crates/or_media`, `crates/or_audio`, `crates/or_render`, the bridge, the
  FFmpeg helper and the export paths before proposing new infrastructure.
- No decode, audio processing or compositing work in Flutter widgets or
  project-mutation commands. Keep queues, buffers and background work bounded.
- Preview and export represent the same composition. Frame/audio activity never
  mutates the project or increments its revision.
- Check the actual FFmpeg build, linked libraries, codecs, muxers and licences
  before exposing a format or acceleration option.
- Keep a correct fallback, and measure software and hardware paths on equal
  settings before claiming acceleration is faster or better.

## Evidence

For preview, exercise real media in the macOS app and verify seek, playback and
audio as relevant. For export, record profile, codec, dimensions, frame rate,
backend, duration and file size; probe the file, decode it and play it. Use
`or-dev-loop` for the local build and `or-ui-ux` when controls change.

AI providers, model downloads and cloud services are outside this skill and the
current product scope.
