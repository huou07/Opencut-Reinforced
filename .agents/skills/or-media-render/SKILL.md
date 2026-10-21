---
name: or-media-render
description: Add or fix OR media import, decode/playback, audio, compositing, FFmpeg, export or media-performance behavior. Use for ordinary non-AI media pipelines. Do NOT use for timeline command semantics (or-edit-feature), UI review (or-ui-ux), declarative MotionScene authoring (or-motionscene), local verification mechanics (or-dev-loop), or AI/provider work.
---

# OR media and rendering

Keep media behavior inside the existing Rust media, audio and render layers,
with Flutter acting as the presentation surface. Follow
[DEVELOPMENT_WORKFLOW.md](../../../docs/DEVELOPMENT_WORKFLOW.md); this skill
records only OR media boundaries.

## Boundaries

- Reuse crates/or_media, crates/or_audio, crates/or_render and the existing
  bridge, FFmpeg helper and export paths before proposing new infrastructure.
- Do not put decode, audio processing or compositing work in Flutter widgets or
  project-mutation commands. Keep queues, buffers and background work bounded.
- Preview and export should represent the same composition. Runtime frame/audio
  activity does not mutate the project or increment its revision.
- Check the actual FFmpeg build, linked libraries, supported codecs, muxers and
  licenses before exposing a format or acceleration option.
- Keep a correct fallback. Measure software and hardware paths on equal settings
  before claiming that acceleration is faster or higher quality.

## Evidence

For preview, exercise real media in the macOS app and verify seek/playback and
audio behavior as relevant. For export, record profile, codec, dimensions,
frame rate, backend, duration and file size; inspect with ffprobe, decode the
result, and play it. Use or-dev-loop for the local build and checks. Use a
separate UX review when controls or layout change.

AI providers, model downloads and cloud services are outside this skill and the
current product scope.
