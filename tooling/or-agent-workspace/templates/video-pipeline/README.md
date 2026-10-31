# OR video pipeline — the showcase standard

A packaged workflow for building a **cinematic product film** from code: local
narration → a tiled beat plan → MotionScenes in the showcase style → parallel
render → one assembly pass into the open project. The reference is the 59-second
OpenCut Reinforced showcase: a finished shot breaks into its layers, the editor,
the assistant and the tools each get a designed beat, and the film closes on the
product.

This is the quality bar, not a slideshow generator. A beat that is a static title,
a paragraph of text or a diagram that only appears is **out of style**. Every
beat moves, transforms and hands off to the next one.

Nothing here changes the OR engine. It drives the app through `tools/or`, the
app's local TTS server and the deterministic MotionScene renderer.

## Contents

| Path | Purpose |
|---|---|
| `scripts/new-video.mjs` | Scaffold `videos/<slug>/` with `script.json` + `video.config.json` |
| `scripts/narration.mjs` | Local TTS → atempo → placed lines → measured manifest + caption cues |
| `scripts/build-scenes.mjs` | Beat windows → one MotionScene per beat (windows must tile the film) |
| `scripts/render.mjs` | Materialise every beat in parallel (files only, no project writes) |
| `scripts/assemble.mjs` | Tracks, narration, music, ducking, scenes, caption track, markers |
| `scripts/style-captions.mjs` | Re-apply the caption look after an SRT edit |
| `lib/kit.mjs` | Drawing kit: palette, primitives, camera presets, particles, waveforms |
| `lib/composers.mjs` | The eight showcase composers and the composer map |
| `lib/scenes/*.mjs` | The seven reference scenes, one file per beat |
| `beats/beat-library.json` | What each composer is for — pick by idea, not by habit |
| `CHECKLIST.md` | Image, motion, sync and honesty gates |
| `videos/example-showcase59/` | The 59 s showcase script and beat map |

## The five steps

Run from the workspace root (the folder with `project.json` and `tools/or`).
`$V` is the video folder; `$T` is this template's folder.

```sh
node $T/scripts/new-video.mjs --slug my-film --title "My film" --seconds 20

# 1. write the lines (start_ms per line) and the beat windows
$EDITOR videos/my-film/script.json videos/my-film/video.config.json

# 2. narration at 1.6×, placed at the planned times, with a fit check
node $T/scripts/narration.mjs --video videos/my-film --attach project.json

# 3. the bed: 120 bpm, so every whole-second cut lands on a beat
node tools/music.mjs --out work/music/bed.wav \
  --prompt "uplifting, 120 bpm, strong drums, 59 giây" --json

# 4. beats → scenes → rendered clips (3 workers is the sweet spot)
node $T/scripts/build-scenes.mjs --video videos/my-film
node $T/scripts/render.mjs --video videos/my-film --workers 3 --attach project.json

# 5. import everything into the open project, then review and save
node $T/scripts/assemble.mjs --video videos/my-film --attach project.json --dry-run
node $T/scripts/assemble.mjs --video videos/my-film --attach project.json
tools/or project save --attach project.json --json
```

`assemble.mjs` creates the tracks it needs, clears the ones it owns, imports the
SRT as a real caption track and styles every cue. `--dry-run` prints the plan
without touching the project.

## Config shape

```jsonc
{
  "slug": "my-film",
  "title": "My film",
  "tempo": 1.6,                       // narration time-stretch; 1.6 is the house tempo
  "duration_ms": 59000,               // exact film length; the windows must add up to this
  "canvas": { "width": 1920, "height": 1080 },
  "captions": { "anchor": "bottom", "sizePt": 26, "offsetY": -420, "weight": "medium" },
  "music": { "file": "work/music/bed.wav", "duckDb": -9 },
  "beats": [
    { "id": "open", "composer": "deconstruct", "start_ms": 0,     "end_ms": 5000,  "lines": ["mo"] },
    { "id": "ui",   "composer": "editor",      "start_ms": 5000,  "end_ms": 13000, "lines": ["ui", "ui2"] },
    { "id": "close","composer": "close",       "start_ms": 52000, "end_ms": 59000, "lines": ["ket"] }
  ]
}
```

* `start_ms`/`end_ms` are the beat window. Windows must start at 0, touch each
  other and end at `duration_ms`; `build-scenes.mjs` refuses anything else and
  refuses a narration line no beat owns. For a long explainer you may drop
  `end_ms` and let a beat run to the next beat's first line.
* `lines` are `script.json` ids. A line's `start_ms` is where its voice sits, so
  keep it inside its beat and leave at least 180 ms before the next line.
* `composer` is a key in `beats/beat-library.json`.
* Composers take 5–13 s. A five-minute film is 25–35 beats, not five long ones.

## The showcase arc

The 59 s reference, which is the default shape for a product intro:

| Window | Composer | What happens |
|---|---|---|
| 0–5 s | `deconstruct` | A finished shot holds the frame, then breaks into picture / sound / voice / captions and folds into a line |
| 5–13 s | `editor` | The editor builds itself, clips drop in, the playhead sweeps, the camera dives into the viewer |
| 13–22 s | `assistant` | A request leaves the chat, an MCP beam reaches external harnesses, a clip comes home |
| 22–31 s | `motionScene` | Code lines turn into geometry that assembles a shot, then the camera enters it |
| 31–39 s | `voiceWhisper` | The script is spoken, the waveform fills, Whisper returns synced captions |
| 39–52 s | `assembly` | Four lanes converge, gaps close, the camera dives into the programme monitor and the finished film plays |
| 52–59 s | `close` | Mark and wordmark, what it is, the honest licence line, one call to action |

Vary the viewpoint between beats: wide shot, user view, close-up, inside-view,
comparison. Two UI beats in a row is a fail even if both are polished.

## Timing budget

* This voice at tempo 1.6 speaks about **17–20 characters per second**. Measure,
  never estimate: `narration.mjs` prints each line's real duration and its slack.
* Spoken audio should fill **45–60 %** of the runtime. A 59 s film carries
  25–30 s of speech; the rest is music and picture. Do not pad with empty scenes
  and do not speed the voice up to hit a length.
* Silence is a tool: leave 0.8–2 s after a big visual moment.
* The film ends on the last frame of the last beat, not on a trailing black gap.
* Music at 120 bpm puts a beat every 0.5 s, so any whole-second cut lands on the
  grid. Duck the bed −9 dB under the voice.

## Rules the template enforces

* **The windows tile the film.** Every beat is a non-empty window, the first
  starts at 0, each starts where the previous ended, the last ends at
  `duration_ms`. The sum of rendered frames is the film length, exactly.
* **Captions are a track, not baked.** A scene camera moves underneath a fixed
  subtitle. `assemble.mjs` imports the SRT and runs `timeline set-text` on every
  cue (bottom anchor, 26 pt, −420 offset).
* **Captions never lead the voice.** The first chunk of a line starts at the
  measured audio start; later chunks start at their character position + 110 ms,
  clamped to the next cue.
* **No line can lose words.** A line that comes back under 80 % of the duration
  its text implies is re-spoken sentence by sentence and stitched.
* **Renders are revision-safe.** `render.mjs` only writes files; every project
  write happens later, sequentially, in `assemble.mjs`.
* **Every frame moves.** Composers animate a camera over the whole beat and keep
  two or three secondary motions running. There is no still frame.

## Pitfalls that cost time

* A media item caches the duration it was imported with. Re-rendering a file in
  place keeps the old length; remove the media item (or re-insert the clip) when
  a render's length changes.
* `insert-clip --duration` must not exceed the media duration. A 1.181875 s WAV
  refuses `--duration 1.182`; floor to milliseconds.
* Keep rationals reduced (`591/500`, not `1182000/1000000`) and times exact.
* Node ids are unique across a whole scene. Prefix every piece (`imSun`,
  `auWave`, `c1`) — a duplicate id fails validation, not just review.
* Scene text is laid out with a 5 % margin inside the remaining box; `kit.mjs`
  converts ink coordinates for you. Keep type at 14 px or larger and lines short.
* Camera zoom stays at 0.94–1.5 except for a deliberate dive; the backdrop is
  oversized so a zoom never shows the canvas edge.
* Editing the project while renders are running causes revision conflicts —
  render and assemble are separate steps for that reason.
* Three parallel workers is the sweet spot; more workers mostly add contention.

## A worked example

`videos/example-showcase59/` is the reference film: 19 narration lines, 7 beats,
59.000 s, 30 fps. It expects `work/music/bed.wav` (generate it with step 3) and
runs the five steps unchanged. Read its `video.config.json` first; it is the
clearest statement of the standard.

## Provenance

Built from the 59 s OpenCut Reinforced showcase assembled in the OR workspace:
seven hand-authored MotionScene beats, 1770 frames at 30 fps, VieNeu-TTS at 1.6,
a 120 bpm bed, and a real caption track. The kit and the seven scene modules are
those scenes with the project-specific narration removed.
