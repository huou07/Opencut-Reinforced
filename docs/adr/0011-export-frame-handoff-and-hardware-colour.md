# ADR 0011 — Export frame handoff, hardware colour conversion and zero-copy

- Status: Proposed — the measurement is recorded; no option is implemented
- Date: 2026-10-27

This record answers one question before any code is written for it: how a
rendered frame should reach the encoder on Apple silicon, and whether the
"zero-copy" path the hardware invites is reachable from the current render
spine.

## Context

An export renders a frame with wgpu (Metal on this host) into an `Rgba8Unorm`
texture, reads it back to system memory, converts BGRA to the encoder's pixel
format on the CPU with swscale, and hands the result to FFmpeg. Measured on the
local MacBook Air M5 at 1920×1080, per frame:

| Step | Cost |
|---|---|
| render, GPU composite and readback (`or_render` row, 640×360 = 1.4 ms) | ~2.8–3.6 ms |
| `sws_scale` BGRA → YUV420P | 2.02 ms |
| VideoToolbox H.264 encode (hardware) | under 1 ms amortised |
| libvpx VP9 encode, `quality` setting | ~22–25 ms |
| readback volume | 8.3 MB per frame (RGBA) |

Two hardware facts shape the options:

1. VideoToolbox encoding wants `nv12` or `yuv420p`; it does not accept BGRA, so
   some colour conversion has to happen before the media engine sees a frame.
2. Zero-copy on Apple silicon means an `IOSurface`-backed `CVPixelBuffer` that is
   both a Metal texture and the encoder's input. wgpu 25.0.2 has no public API to
   import or export an external texture: there is no `IOSurface` mention in
   `wgpu-25.0.2` or `wgpu-hal-25.0.2`'s Metal backend, and its only "external"
   API is the WebGPU DOM video path. Reaching it would mean `wgpu-hal` or raw
   Metal beside the spine.

The render spine is a single wgpu authority (ADR 0002) and the frame memory
domain is explicit (ADR 0003). A second Metal compositor for export would be a
second renderer, not an optimisation.

## Options

1. **GPU colour conversion, no interop.** Composite as today, then convert RGBA
   to NV12 on the GPU (a second render pass into `R8Unorm` + `RG8Unorm`
   attachments) and read back 3.1 MB of planes instead of 8.3 MB of RGBA. The
   encoder still receives system memory, so no interop is needed. Expected
   saving: the whole swscale step plus ~5 MB of readback per 1080p frame, i.e.
   ~3 ms of a ~6 ms MP4 frame.
2. **Raw Metal compositing for the export path.** Render into an
   `IOSurface`-backed texture and hand the `CVPixelBuffer` to VideoToolbox
   directly. Removes readback and conversion entirely, but replaces the spine for
   one job and duplicates compositing, text and effects.
3. **Wait for wgpu external-texture support.** Keep the current path, revisit
   when wgpu can import an `IOSurface` texture. No work now, no gain now.

## Decision

Nothing is implemented in this record. Option 1 is the recommended next
increment: it stays inside the existing spine, is measurable against the same
export lane, and proves the colour maths (BT.709, limited range) that option 2
would also need. Option 2 is refused while ADR 0002 stands: one render spine, and
an export-only Metal path would have to re-earn compositing, text and effect
parity. Option 3 stays the fallback if option 1 measures smaller than expected.

## Consequences

- Option 1 keeps one renderer and one frame ownership model; it changes the
  writer's input format from packed BGRA to NV12 planes, which touches
  `or_media`'s writer, the bridge's export loop and the tests for both.
- Colour correctness becomes ours instead of swscale's: the GPU conversion has to
  match BT.709 primaries, limited range and the existing scaler flags, verified by
  comparing exported frames against the current path.
- Zero-copy on this hardware remains unavailable without option 2; any claim that
  the media engine is fed without a copy would be false until wgpu exposes
  external textures or option 2 is accepted.
