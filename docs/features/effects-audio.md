# Feature — Visual Settings, Effects, Colour and Audio Controls

Last reviewed: 2026-11-06

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

## Agent surface

`or timeline set-clip-visual|set-clip-audio|set-clip-playback` writes these
fields through `timeline.clip.update`, the same command the inspector uses, so a
CLI edit cannot produce state the UI rejects. The CLI names the model's units
(milli-canvas, milli-degrees, basis points, millidecibels) and takes transitions
as `KIND:NUM/DEN`; neutral effect values clear an effect. `or timeline
set-canvas` sets the sequence shape.

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

### Reduce noise

`AudioSettings.noise_reduction_decibels` (0..=24, absent meaning 0) is canonical
clip state like gain, so a reduction survives save/reopen, exports, and one undo
step for a whole change. The inspector exposes a `Reduce noise (dB)` field and
No reduction / 6 / 12 / 18 dB chips; both write the same `timeline.clip.update`
command the field below it uses.

The DSP is a peak-keyed expander, not a spectral denoiser: `or_audio::NoiseGate`
tracks a decaying peak (≈0.9 dB/s) and an ≈2 ms level envelope, then pulls
material more than roughly 10 dB under that peak toward `-reduction_db` while
leaving sustained tones and music at their level. One gate belongs to one clip.
The streaming mixer keeps gates in a map across blocks so a hiss tail is not
re-opened at every audio block, and the export cache runs the same gate over a
whole decoded clip, so preview and export agree. The gate runs at decode, before
gain/pan/fades, so the level it sees is the source level.

Video-embedded audio is not part of the product yet: only `TrackKind::Audio`
clips build an audio clip, so a video clip's own track cannot carry a reduction
until that pipeline exists. The media-library "clean audio" action still writes a
new cleaned WAV; the timeline action no longer needs it.

## Deferred

Keyframed parameters, effect stacks beyond the closed set, masks, adjustment
layers, HDR, hardware-accelerated effects, spectral denoising (the current
reduction is a gate, not a noise-print removal), and video-embedded audio.

## Related

- [editing.md](editing.md)
- [architecture/realtime-plane.md](../architecture/realtime-plane.md)
- [design/design-language.md](../design/design-language.md) — timeline colour restraint
