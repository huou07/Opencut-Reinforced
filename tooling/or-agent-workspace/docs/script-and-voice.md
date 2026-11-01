# Script, narration, captions and music

Scope: turning a spoken idea into an OR edit — a written script, narration, exact
caption clips, and a generated background-music bed. Timeline mechanics are in
[timeline-editing.md](timeline-editing.md); importing files is in
[media-and-import.md](media-and-import.md); audio the user already has (recording,
noise, speech models) is in [audio-prep.md](audio-prep.md).

## Order of work

1. Decide the length, language (`vi` or `en`), audience, tone, voice, and music
   mood with the user. Do not invent a length the user did not ask for.
2. Write the script as `work/scripts/<slug>.json` (format below).
3. Validate it without speaking: `node tools/voiceover.mjs --script PATH --plan`.
4. Narrate it. Without `--apply` the tool writes `manifest.json`, the per-line
   WAV files, and `captions.srt`; with `--apply` it also imports the WAVs, places
   them on an audio track, and imports the captions.
5. Generate the music bed, import it, and place it on another audio track.
6. Add the visuals, review the timeline, then save.

## Script format

`or-assistant-script` schema 1 is a JSON object with one narration line per
scene. The narration is what the voice says; the caption is what the subtitle
shows (it defaults to the narration).

```json
{
  "format": "or-assistant-script",
  "schema_version": 1,
  "title": "Giới thiệu OR",
  "language": "vi",
  "rate": 180,
  "gap_ms": 250,
  "scenes": [
    {
      "id": "mo-dau",
      "marker": "Mở đầu",
      "narration": "Xin chào, đây là Opencut Reinforced.",
      "caption": "Xin chào, đây là OR.",
      "duration_ms": 6000
    },
    {
      "id": "dung-phim",
      "narration": "Bạn có thể dựng phim ngay trên máy.",
      "duration_ms": 5000
    }
  ]
}
```

| Field | Meaning |
|---|---|
| `id` | lowercase kebab-case, unique; names the WAV and the caption order |
| `narration` | the exact words spoken by `say`; required, at most 2000 bytes |
| `caption` | the subtitle text; defaults to `narration` |
| `marker` | optional timeline marker label for this scene boundary |
| `duration_ms` | the planned visual length; only used by `--plan` |

A script may also set `voice` and `language` at the top level. The script may
contain at most 300 scenes. Every field except `scenes` is optional. `rate`
defaults to 180 words per minute and `gap_ms` to 250 milliseconds between lines.

## Two narration surfaces

The app and the workspace both narrate, and they are not the same path:

- **Workspace, offline:** `tools/voiceover.mjs` speaks each line with the macOS
  `say` voice, measures the real WAV duration, and derives exact caption times.
  The voice is the script's `voice`, then the app's saved default from **Settings
  → Narration** (`narration-settings.json`, read through `project.json`), then the
  first installed Vietnamese voice on this machine. It never falls back to a
  product constant such as `Linh`.
- **App, optional cloud voices:** **Audio → Generate voiceover…** uses the voice
  chosen in **Settings → Narration**, including cloud providers. Provider setup,
  testing and voice creation all happen in that panel and stay inside the app.
  When the user wants a cloud voice, they run the app action and the result
  arrives as ordinary media you can edit and save like any other.

```sh
node tools/voiceover.mjs --script work/scripts/gioi-thieu.json --plan --json
node tools/voiceover.mjs --script work/scripts/gioi-thieu.json \
  --out work/voiceover/gioi-thieu --apply --attach project.json --json
```

The tool writes one 44.1 kHz mono PCM WAV per scene, measures its real duration
from the WAV, and builds exact start/end times for every caption. `--apply`:

- reuses the first unlocked audio track that is empty, or adds one;
- imports each WAV with `media add` and inserts it with `timeline insert-clip`;
- imports `captions.srt` with `timeline import-captions`;
- leaves the project unsaved, so you can review before `project save`.

Without `--apply`, the output is the manifest, the WAVs and the SRT. Use that
when the user wants to hear the narration before it enters the timeline. The
manifest is the same shape the applied path uses:

```json
{
  "format": "or-voiceover-manifest",
  "schema_version": 1,
  "rate": 180,
  "gap_ms": 250,
  "total_ms": 6484,
  "lines": [
    {
      "scene_id": "mo-dau",
      "start_ms": 0,
      "end_ms": 3672,
      "duration_ms": 3672,
      "file": "/absolute/path/work/voiceover/gioi-thieu/mo-dau.wav",
      "caption": "Xin chào, đây là OR."
    }
  ]
}
```

The `start_ms`/`end_ms` values are the visual scene boundaries too: add markers
with `timeline add-marker` at those times if the user wants the structure
visible in the editor.

## Captions from existing audio

The narration path derives captions from the script. For audio or video the user
already has, use the app's local multilingual transcription:

```sh
tools/or captions transcribe --attach project.json --media MEDIA_ID \
  --srt-out work/captions/clip.srt --json
tools/or timeline import-captions --attach project.json \
  --file work/captions/clip.srt --json
```

This runs in the running app (a `--project` file is refused), detects Vietnamese
or English automatically, and never changes the project. The model is the one the
app has installed and selected (Base, Small, Medium, Large v3 Turbo q5 or Large
v3 Turbo), so if it reports `TRANSCRIPTION_MODEL_UNAVAILABLE`, send the user to
**Audio → Speech model…** instead of downloading anything here. Review the cues
(the JSON carries `start_ms`, `end_ms`, `text`, and the detected `language`) before
importing; the second command creates a new caption track and one editable
caption clip per cue. Edit the SRT first if timing or wording needs correcting.

Caption style — font, size, anchor, offset, background box and text opacity —
travels with each clip. The app's Edit Caption dialog is the visual editor; the
same update is available from the workspace as `timeline set-text --clip --track
--anchor bottom --size-pt 26 --offset-y -420`. Captions from transcription are
ordinary caption clips, so every cue can be restyled without recreating the
track, and `templates/video-pipeline/scripts/style-captions.mjs` re-applies the
template's look after an SRT edit.

## Music

```sh
node tools/music.mjs --out work/music/bed.wav \
  --prompt "nhạc nền nhẹ nhàng 30 giây, không trống" --json
```

The prompt is read by the same interpreter as the app's **Audio → Generate
music…** dialog: a mood word (`calm`, `warm`, `uplifting`, `tense`, `minimal`, in
Vietnamese or English), a length (`90 giây`, `2 phút`, `40s`), a tempo (`90 bpm`)
and drums on or off (`không trống`, `no drums`, `strong drums`). The JSON carries
`summary` — `calm · 62 bpm · 30 s · no drums` — which is the readback to report,
and `plan.notes` when nothing in the sentence was understood. `--variation N`
reseeds the same request. Explicit `--mood`, `--tempo`, `--duration` and `--seed`
still work and win over the prompt, so an older command line keeps its meaning.

The bed is synthesised locally, differs by `--seed`, and is royalty-free. It is
generated quiet (about −20 dB RMS, −9 dB peak) so it sits under speech; change the
clip gain in the inspector rather than regenerating louder. The app's dialog also
ducks it under a voice track in one step. To place the workspace file:

```sh
tools/or media add --attach project.json --source /absolute/path/work/music/bed.wav --json
tools/or timeline insert-clip --attach project.json --track AUDIO_TRACK \
  --media MEDIA_ID --at 0/1 --source-start 0/1 --duration 60/1 --json
```

Find the track with `timeline tracks`; add one with `timeline add-track --kind
audio` if the project has none.

## Rules and bounds

- The workspace path is offline: `say` is a system voice, the music is
  synthesised, and the transcription model is the app's local model. No cloud key
  is read or written here; cloud voices belong to the app's Settings.
- The script and generated media live under `work/`; they are not project state.
  Importing them is what makes them part of the project.
- Never claim the narration or captions are recorded or transcribed when they
  came from a written script, and never claim a cloud voice was used when the
  system voice spoke. Say which path produced them.
- One narration line should say one thing. Long paragraphs become one long
  caption and a slow clip; split them into scenes.
- `say` is macOS-only. On another platform, say the offline voice path is
  unavailable and ask whether to use an installed voice tool, rather than
  pretending the command worked.
- A `--apply` run is several commands, so it is several undo steps; report that
  instead of calling it one undoable change.

## Verification

- `node tools/voiceover.mjs --script PATH --plan --json` prints the planned
  order and timing without side effects.
- `ffprobe -v error -show_entries format=duration -of csv=p=0 FILE.wav` confirms a
  generated WAV's real length.
- `tools/or timeline tracks` and `timeline clips` confirm what `--apply` placed.
- `tools/or project save` then `project summary` confirms the edit persisted.
