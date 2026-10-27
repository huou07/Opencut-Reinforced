---
name: or-script-production
description: Plan and produce a narrated OR edit offline — write an or-assistant-script, generate macOS say voiceover, derive exact caption clips, and add a generated music bed. Use when the user asks for a script, voiceover/dubbing, automatic subtitles for audio, or background music. Do NOT use for ordinary timeline edits (or-timeline-edit) or for changing OR's own implementation.
---

# Script production

Read `docs/script-and-voice.md` for the exact formats and command lines. This
skill is the working order; the guide is the reference.

## 1. Fix the brief before writing

Ask for, or confirm from the request: length in seconds, language (`vi` or
`en`), audience, tone, voice, and music mood. If the user asked for a 30-second
video, do not write a 90-second script.

## 2. Write one line per scene

Save `work/scripts/<slug>.json` with `format: "or-assistant-script"`,
`schema_version: 1`, and a `scenes` array. Each scene has a unique kebab-case
`id`, a short `narration`, an optional `caption`, and an optional `marker`. Keep
each line to one sentence or two; split long paragraphs.

## 3. Validate before speaking

```sh
node tools/voiceover.mjs --script work/scripts/<slug>.json --plan --json
```

Fix format errors and read the planned timings back to the user if the total is
far from the requested length.

## 4. Narrate and caption

Preferred one-step path when the user wants it in the edit:

```sh
node tools/voiceover.mjs --script work/scripts/<slug>.json \
  --out work/voiceover/<slug> --apply --attach project.json --json
```

Without `--apply`, generate only, report the manifest/`captions.srt`, and let
the user listen first. When you use `--apply`, confirm the result with `timeline
tracks` and `timeline clips` before saving.

## 5. Add music

```sh
node tools/music.mjs --out work/music/<slug>.wav --duration <seconds> --mood <mood> --json
```

Import it, place it on an audio track, and tell the user the bed is intentionally
quiet and adjustable in the inspector. Do not raise the generated level.

## 6. Finish

Add the visuals the user asked for, review the timeline, then `project save`.
Report which parts came from the script (narration and captions) and which from
transcription or generation, and what remains unverified.

## Existing audio instead of a script

If the user already has audio or video and wants subtitles, use
`tools/or captions transcribe --media MEDIA_ID --srt-out PATH`, review the cues,
then `tools/or timeline import-captions --file PATH`. This is the only path that
is "from audio"; the script path derives captions from the narration it wrote.

## Checks

- `--plan` is side-effect free; use it after every script edit.
- `say` and the music generator are offline; never claim a network call.
- A `--apply` failure leaves an unsaved edit; inspect with `timeline tracks`
  before deciding whether to undo or retry.
- Never save a stale revision: if a command reports a revision conflict, read
  `project summary` and re-apply the remaining steps.
