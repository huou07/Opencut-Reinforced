# Feature — Visual Settings, Effects, Colour and Audio Controls

Last reviewed: 2026-10-26

Scope: per-clip visual and audio parameters, the closed effect/transition set, and
the colour assumptions behind preview and export.

## Visual settings

Bounded fixed-point transform (position, scale, rotation, anchor), crop and
opacity, stored exactly in the project and applied in the wgpu layer pipeline.
GPU uniforms and textures stay runtime-only. The inspector edits these in human
units (%, °, px, dB, s) while retaining exact project values.

## Effects and transitions

A closed, typed set with typed parameters and presets that fill the fields before
Apply. An effect reference is part of clip state; it is not an arbitrary shader or
a script. New effects require a typed parameter contract and render-path work.

## Colour

SDR delivery assumes Rec.709 / sRGB, linear-light working space, and
premultiplied-alpha compositing. VP9 export converts bounded BGRA to YUV420P
tagged BT.709 limited-range SDR. HDR remains deferred.

## Audio

Automatic ducking is not a project field yet. `timeline.clip.duck` computes the
edit in Rust: the music clip is split at every overlapping voice boundary and the
overlapping segments are set to the chosen duck level (−12 dB by default). It is
one revision, one history entry and one undo step even when it creates many
segments; the result is ordinary editable clip gain. The app uses the same
command from its music dialog, and the CLI exposes it as `timeline duck-clip`.

Clip gain and mute are canonical clip state; gain, pan, fades and later DSP belong
to the audio engine (`or_audio`), never to Flutter widgets. Desktop audio output
is available; Android audio output is explicitly unavailable.

## Deferred

Keyframed parameters, effect stacks beyond the closed set, masks, adjustment
layers, HDR, hardware-accelerated effects, and audio DSP beyond the current
gain/pan/fade contract.

## Related

- [editing.md](editing.md)
- [architecture/realtime-plane.md](../architecture/realtime-plane.md)
- [design/design-language.md](../design/design-language.md) — timeline colour restraint
