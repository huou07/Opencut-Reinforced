# Real-Time Plane — Snapshots, Frames, Preview, Audio Clock

Last reviewed: 2026-10-26

Scope: how committed project state becomes frames and sound. Nothing here may
mutate the project or increment `ProjectRevision`.

## Snapshot identity

A `RenderSnapshot` is an immutable, versioned evaluated view of
`ProjectRevision N` for an exact time or range. Workers adopt a snapshot; a
project mutation produces a snapshot for the next revision and the old one is
dropped. The snapshot may be a full immutable structure, an incremental graph,
structurally shared data or another measured read model — granularity is a
performance decision, not "clone the project per frame".

Workers never mutate `ProjectDocument` and must not hold a heavyweight lock on
mutable project state for their lifetime.

## Frames

A frame carries explicit dimensions, pixel or texture format, colour
information, and exact timing. It is described as a software frame, a GPU
texture, a decoder surface, or an external/shared surface, and its lifetime is
owned by an explicit `FrameLease` that is released by a native callback or
completion fence.

Rules that hold on every platform:

- Per-frame pixels, full-rate frame bytes and raw OS/GPU handles never travel
  through Dart objects, IPC, project state or cache identity.
- The platform adapter keeps a bounded latest-frame mailbox / in-flight set and
  rejects frames from stale generations or project revisions.
- Dropping an obsolete preview frame is normal runtime behaviour.
- The supported desktop presentation path is a bounded BGRA8888 premultiplied
  pixel-buffer texture unless platform API evidence requires RGBA8888.

## Preview pipeline (desktop)

```
RenderSnapshot → software decode (or_media) → wgpu layer evaluation (or_render)
              → bounded pixel buffer → native texture adapter → Flutter viewer
```

`PreviewRuntime` reuses at most four source decoder sessions per project revision
and runtime generation. Forward playback decodes from the current demux
position even when presentation requests were dropped; a backward request seeks;
an explicit transport seek starts a fresh generation. Each session retains at
most one preceding and one lookahead frame, charged to the shared decode budget.

The export path may bypass the GPU for a frame only when it is provably the
identity render: one same-size fully opaque video source, no text or other video
layer, and identity visual settings. Everything else goes through the shared
wgpu renderer, so preview and export keep the same edit semantics.

## Audio clock

`or_audio` owns the device-neutral bounded output buffer, the exact audio master
clock and video pacing decisions. Clock messages carry a `RenderSnapshot`;
synchronisation rejects a different project or revision while allowing audio and
video to request different time ranges. The callback consumes preallocated
samples without waiting or locking; an underrun emits silence and still advances
device time. Gain, pan, fades and later DSP belong in the audio engine, not in
Flutter widgets. `cpal` is used only inside `or_audio`.

## Budgets and capability selection

- Bounded queues, a bounded in-flight frame count, and separate render, audio and
decode budgets.
- Capability-based selection with a software fallback; a hardware path is never
  assumed faster or available.
- Per-frame scheduling decisions are runtime-only and measurable through
  deterministic counters (no telemetry, no wall-clock dependence in tests).

## Status and gaps

Desktop preview uses software FFmpeg decode plus the pixel-buffer texture
adapter on macOS, Windows and Linux. Hardware decode (VideoToolbox, MediaCodec,
NVDEC) and shared GPU surfaces are not approved; the vendored FFmpeg build has
no `h264_videotoolbox` decoder, and software 1080p decode measures about a tenth
of one core on the local M5, so decode is not the preview bottleneck. Android
preview uses a separate software path with SAF descriptors and no audio output.

Export is the one place a frame leaves the GPU in a codec's own layout: the
hardware H.264 profile receives NV12 planes converted on the GPU, so no CPU
colour conversion runs per exported frame (ADR 0011). The lossless and VP9
profiles still read back packed RGBA. Nothing shares a GPU surface with the
encoder yet — that would need wgpu external-texture support or a second Metal
compositor, which ADR 0011 refuses.

## Related

- ADR 0001 (planes), ADR 0002 (wgpu spine), ADR 0003 (frame ownership)
- [control-plane.md](control-plane.md)
- [media-pipeline.md](media-pipeline.md)
- [features/preview-playback.md](../features/preview-playback.md)
