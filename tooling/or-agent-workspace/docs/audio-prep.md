# Audio prep

Scope: audio the user records or already has, made usable before it becomes
narration, captions or a music bed. The script path is
[script-and-voice.md](script-and-voice.md); placing the result is
[timeline-editing.md](timeline-editing.md).

## Record a take — the app owns the microphone

The workspace has no audio capture. The Audio tool's **Record voice…** dialog
records one take with the system microphone, writes it under
`<application support>/voice-recordings`, and on accept imports it and places it
as an ordinary audio clip. When the user asks to record, say the app does it;
afterwards read `media list` and the new audio track instead of guessing a path.

A recorded take is the sample an ElevenLabs personal voice is created from, but
that creation happens in **Settings → Narration** and is the user's action.

## Clean up noise

Two surfaces, one behaviour: write a new file, then import it as its own media
item so the original stays available for comparison.

- In the app: a media item's menu → **Clean up Audio**. It measures the floor,
  reduces 12 dB, writes a `.cleaned.wav` file into
  `<application support>/audio-cleanup/` and imports the result.
- In the workspace, file to file:

```sh
tools/or media clean-audio --source /absolute/in.wav --out /absolute/out.wav --json
```

`--noise-floor-db` (whole dBFS) and `--reduction` (whole dB, 12 by default) are
optional. Without `--noise-floor-db` the tool measures the recording's own floor
first, because a constant either does nothing or eats the voice. The JSON result's
`cleanup` block reports the floor it used, the reduction, the bytes written and
the filter chain.

The clean-up runs through the packaged FFmpeg helper and opens no project, so
nothing changes in the project until you import the output:

```sh
tools/or media add --attach project.json --source /absolute/out.wav --json
```

If the helper is missing (a plain debug CLI), the verb fails with a helper error:
say so and point the user at the app's **Clean up Audio** action rather than
retrying.

Clean-up can thin a voice or dull music. Compare before replacing: leave the
original clip in place, place the cleaned item where the user asked, and let them
choose. Never delete or mute the original unless asked.

## Speech models and transcription

The model belongs to the app, not the workspace. **Audio → Speech model…**
chooses one of five multilingual checkpoints (Base 141 MiB, Small 465 MiB,
Medium 1.43 GiB, Large v3 Turbo q5 547 MiB, Large v3 Turbo 1.51 GiB), downloads
it once after explicit consent, verifies it by size and SHA-256, and keeps it in
the app's private folder. The picker shows the installed total and can remove a
model; removing the active one falls back to the catalog default. `or captions
transcribe` and the MCP tool reuse that choice.

```sh
tools/or captions transcribe --attach project.json --media MEDIA_ID \
  --srt-out work/captions/clip.srt --json
tools/or timeline import-captions --attach project.json \
  --file work/captions/clip.srt --json
```

Transcription needs the running app: `--project PATH` is refused with a message
saying so. It decodes audio locally, detects Vietnamese or English, changes no
project state, and returns bounded cues with `start_ms`, `end_ms`, `text` and the
detected `language`. Review them, fix the SRT if needed, then import; the import
creates one caption track with one editable clip per cue.

`TRANSCRIPTION_MODEL_UNAVAILABLE` means no model is installed or the runtime is
not in this build. Do not download a model from the workspace: ask the user to
open **Speech model…** once (or install the local helper binaries with
`scripts/install_local_transcription_helpers.sh`).

## Bounds

- No network call: cloud voices and cloud narration are app-only, configured in
  the app's Settings; this workspace works with local files and local voices.
- No model weights, no network call, no microphone from the CLI.
- Clean-up output and transcripts are files under `work/` until imported; they are
  not project state.
- Never claim a recording, a transcription or a cleaned file exists because a
  command was attempted; read `media list` and the caption track back.
