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
segments; the result is ordinary editable clip gain. The voice track may be an
audio track or a video track, because a video clip carries its own soundtrack:
the music dialog lists both kinds and labels them, so a talking-head take ducks
the bed the same way a placed narration clip does. The app uses the same command
from its music dialog, and the CLI exposes it as `timeline duck-clip`.

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
tracks a peak that falls one decibel a second and an ≈2 ms level envelope, then pulls
material more than roughly 10 dB under that peak toward `-reduction_db` while
leaving sustained tones and music at their level. One gate belongs to one clip.
The streaming mixer keeps gates in a map across blocks so a hiss tail is not
re-opened at every audio block, and the export cache runs the same gate over a
whole decoded clip, so preview and export agree. Measured end to end on the
exported MP4: a 3 s tone followed by 7 s of hiss exports the tone at the same
level (−24.09 dBFS before and after) and the hiss 15 dB lower (−51.4 → −66.3
dBFS), with the reducer set to 18 dB. The gate runs at decode, before
gain/pan/fades, so the level it sees is the source level.

A video clip carries the sound of its own media: `VisualSettings.audio` holds the
same `AudioSettings`, the program builds an audio clip for a video clip whose media
has an audio stream, and the inspector shows the audio fields beside the picture
fields. A visual update carries the audio half in the same command
(`ProjectTimelineVisualSettingsView.audio`), so moving picture and changing sound in
one Apply is one revision and one undo step; a view without audio leaves the clip's
sound untouched, which is what the transform overlay and the text inspector send.
Measured on a 640×360 H.264+AAC clip exported to MP4: the tone stays at −24.1 dB
and the hiss falls from −51.4 dB to −66.8 dB with 18 dB reduction. Text, caption
and audio-track behaviour is unchanged, and the media-library "clean audio" action
still writes a new cleaned WAV; the timeline action no longer needs it.

## Deferred

Keyframed parameters, effect stacks beyond the closed set, masks, adjustment
layers, HDR, hardware-accelerated effects, spectral denoising (the current
reduction is a gate, not a noise-print removal), and per-stream selection when a
video file carries more than one audio stream.

## Related

- [editing.md](editing.md)
- [architecture/realtime-plane.md](../architecture/realtime-plane.md)
- [design/design-language.md](../design/design-language.md) — timeline colour restraint
