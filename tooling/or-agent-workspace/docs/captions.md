# Captions

Scope: turning speech into an editable caption track, keeping cues readable, and
styling them consistently. The narration script is
[script-and-voice.md](script-and-voice.md); transcription of existing audio is
[audio-prep.md](audio-prep.md).

## Captions are clips, not a burn-in

Every cue is an ordinary caption clip on a caption track. It can be moved,
retimed, restyled and deleted like any clip. Never bake subtitles into a render
and never duplicate a video to change a caption.

## Two sources

| Source | Path |
|---|---|
| A written script | `tools/voiceover.mjs` writes `captions.srt` from measured speech; `--apply` imports it |
| Existing audio or video | `tools/or captions transcribe --attach project.json --media ID --srt-out FILE` then `timeline import-captions` |

## Deliver subtitles

`tools/or captions export --attach project.json --out work/exports/NAME.srt`
writes every caption clip in start order; `.vtt` writes WebVTT. This is the same
collection the app exports, so it is the right sidecar for a delivered video.
`CAPTION_TRACK_EMPTY` means there are no cues yet. Details:
[export-and-delivery.md](export-and-delivery.md).

Review the SRT before importing. Re-importing the same file creates a second
track; delete the old one instead of stacking duplicates.

## Cue rules

- One cue is one phrase, not one sentence paragraph. Split at punctuation and at
  natural breath points.
- At most two lines per cue; around 42 characters per line for Latin script.
- Reading speed 12–18 characters per second for narration; never leave a cue on
  screen for less than about 0.8 seconds.
- A cue starts when the word starts and ends a few frames after it ends. Do not
  pad every cue to the same length.
- Keep the speaker's words; fix only obvious ASR errors. Do not rewrite the
  meaning in a caption.
- Vietnamese keeps its diacritics. Never strip marks to make a line fit; shorten
  the line instead.

## Styling

Style travels with each caption clip. The CLI form:

```sh
tools/or timeline set-text --attach project.json --track TRACK --clip CLIP \
  --anchor bottom --size-pt 26 --offset-y -420 --json
```

Use one style per video: same font size, anchor and offset on every cue. The
pipeline's `templates/video-pipeline/scripts/style-captions.mjs` re-applies a
template's look after an SRT edit. The app's Edit Caption dialog is the visual
editor for the same fields.

## Verify

- `tools/or timeline clips --attach project.json --track TRACK --json` lists the
  imported cues with their times.
- Play the first, middle and last cue; check that no cue is clipped by the next.
- `session describe` must be clean after the save.

## Failures

- `TRANSCRIPTION_MODEL_UNAVAILABLE` — no model installed; send the user to
  **Audio → Speech model…**, never download weights here.
- Transcription needs the running app; `--project FILE` is refused.
- A caption track that will not accept a cue is usually locked or on the wrong
  track; read `timeline tracks` first.
