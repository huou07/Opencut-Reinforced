# OR video pipeline — reusable template

A packaged version of the workflow used to build the Opencut Reinforced explainer:
local narration → beat-planned MotionScene illustrations with baked captions →
parallel render → one assembly pass into the open project.

Nothing here changes the OR engine. It drives the app through `tools/or`, the
app's local TTS server and the deterministic MotionScene renderer.

## Contents

| Path | Purpose |
|---|---|
| `scripts/new-video.mjs` | Scaffold `videos/<slug>/` with `script.json` + `video.config.json` |
| `scripts/narration.mjs` | Local TTS → atempo → measured manifest + caption cues |
| `scripts/build-scenes.mjs` | Beat map → one MotionScene per beat (captions baked in) |
| `scripts/render.mjs` | Materialise every beat in parallel (files only, no project writes) |
| `scripts/assemble.mjs` | Clear tracks, import narration + music, duck, insert beats, markers |
| `lib/world.mjs` | Drawing kit: canvas, palette, primitives, camera presets, captions |
| `lib/composers.mjs` | 19 beat compositions (see `beats/beat-library.json`) |
| `beats/beat-library.json` | What each composition is for — pick by idea, not by habit |
| `CHECKLIST.md` | Story, sync and visual review gates |

## The five steps

Run everything from the OR workspace root (the folder with `project.json` and
`tools/or`). `$V` is your video folder.

```sh
node templates/video-pipeline/scripts/new-video.mjs --slug my-video --title "My video"

# 1. write the script (one line per spoken sentence) and the beat map
$EDITOR videos/my-video/script.json videos/my-video/video.config.json

# 2. narration at 1.6× with a completeness guard + cue timing
node templates/video-pipeline/scripts/narration.mjs --video videos/my-video --attach project.json

# 3. check the read before spending render time
node tools/music.mjs --out videos/my-video/build/music/bed.wav \
  --prompt "<mood, bpm, drums, seconds>" --json

# 4. beats → scenes → rendered clips (3 workers is a good default)
node templates/video-pipeline/scripts/build-scenes.mjs --video videos/my-video
node templates/video-pipeline/scripts/render.mjs --video videos/my-video --workers 3 --attach project.json

# 5. import everything into the open project, then save
node templates/video-pipeline/scripts/assemble.mjs --video videos/my-video --attach project.json --dry-run
node templates/video-pipeline/scripts/assemble.mjs --video videos/my-video --attach project.json
tools/or project save --attach project.json --json
```

`--dry-run` prints the tracks it will use, the beat timings and the marker count
without touching the project.

## Config shape

```jsonc
{
  "slug": "my-video",
  "title": "My video",
  "tempo": 1.6,                       // narration time-stretch (1.0 = natural)
  "gap_ms": 120,                      // silence between narration lines
  "canvas": { "width": 1920, "height": 1080 },
  "captions": { "band": 952, "font": 28, "baked": true },   // baked = no caption track
  "music": { "file": "videos/my-video/build/music/bed.wav", "duckDb": -12 },
  "beats": [
    { "id": "open",      "composer": "open",      "camera": "push", "lines": ["mo-dau"] },
    { "id": "assembly",  "composer": "assembly",  "camera": "pull", "lines": ["y-tuong"] },
    { "id": "close",     "composer": "close",     "camera": "push", "lines": ["ket"] }
  ]
}
```

* `lines` are the `id`s from `script.json`. A beat owns consecutive lines; the
  scene lasts until the next beat starts, so narration never plays over the
  wrong picture. Lines left out of every beat are reported by
  `build-scenes.mjs` — fix them, don't ignore them.
* `camera`: `push`, `pull`, `pan`, `fly` or `none`. Change it between beats.
* `composer`: any key in `beats/beat-library.json`.

## Rules the template already enforces

* **Captions never lead the voice.** The first chunk of a line starts exactly at
  the measured audio start; later chunks start at their character-proportional
  position plus 130 ms, and every cue is clamped to the next cue's start.
* **No line can lose words.** A line that comes back shorter than 80 % of the
  expected duration for its text is re-spoken sentence by sentence and stitched.
* **Beat boundaries are exact.** Beats tile the whole narration: no gaps, no
  overlapping clips.
* **Renders are revision-safe.** `render.mjs` only writes files; every project
  write happens in `assemble.mjs`, sequentially.
* **Captions are baked** into the scene strip by default, so the timeline carries
  no caption track and nothing covers the illustration.

## Segment shape that works

One beat = one idea = 8–20 s. Sequence:

1. hook (result or question) — `open`
2. the discovery, one concrete action at a time — `chatLink`, `chatRun`,
   `assembly`, `voiceGen`, `whisperListen`
3. the mechanism behind it — `motionDetail`, `layers`, `previewSync`
4. the payoff — `finale`
5. one takeaway — `close`

Do not stack two diagram beats in a row; alternate user view, close-up,
inside-view and comparison.

## Timing budget

Narration at 1.6× runs about **28 characters per second**. For a 5-minute video:

```
chars ≈ 300 s × 28 ≈ 8,400 characters  (≈ 60–75 spoken lines)
```

Add `gap_ms × lines` for the real total, then check `manifest.total_ms` before
rendering. Too short: add mechanism lines, not filler. Too long: cut a beat.

## Pitfalls that cost time (already handled, but know them)

* A media item caches the duration it was imported with. If you re-render a file
  in place, remove that media item before re-inserting, or the clip keeps the old
  length.
* `insert-clip --duration` must not exceed the media duration; the template floors
  to whole milliseconds.
* Exact rationals with huge denominators are rejected by the CLI; keep
  `n/d` reduced or use milliseconds.
* Materialisation validates more strictly than `motion validate`. Text nodes that
  overlap each other or sit on identical lines can trip `MOTION_INVALID_TEXT_SIZE`
  at render time — keep labels apart.
* Editing the project while renders are running causes revision conflicts. That
  is why render and assemble are separate steps.
* Three parallel workers is the sweet spot; more workers mostly add contention.

## A worked example

`videos/example-opencut/` holds the script and beat map of the shipped explainer
(74 lines, 19 beats, 4:46 at tempo 1.6). No media is stored there — point
`music.file` at a bed you generated and run the five steps to rebuild it from
scratch, or copy the structure for a new video.

## Provenance

Built from `work/local-tts-v4.mjs` and `work/build-scenes-v6.mjs`, the exact
generators used for the shipped explainer (74 narration lines, 19 beats, 4:46,
baked captions, tense bed). The live project and those files are left untouched.
