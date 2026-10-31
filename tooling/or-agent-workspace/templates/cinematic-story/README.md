# Cinematic story template — the long-form documentary standard

A packaged workflow and house style for a **narrated, chaptered product film** built
from MotionScenes: a measured local voice, a chaptered beat plan, cinematic scenes
that always move, a real caption track, and one assembly pass into the open
project. The reference is the 5:00 OpenCut Reinforced documentary built in project
`11`: 37 beats, 12 chapters, 65 narration lines, 191 caption cues, 30 fps.

This is the long-form sibling of `templates/video-pipeline/` (the 59 s showcase).
Same renderer, same kit, different film: the showcase sells a moment, this template
carries an argument for five minutes. Read `STYLE.md` for the grammar, this file
for the process.

## What it packages

| Piece | Where | What it fixes |
|---|---|---|
| House defaults | `defaults/video.config.json` | tempo, gaps, caption look, chunk rules, bed level |
| Scaffold | `scripts/new-video.mjs` | `videos/<slug>/` with the documentary shape |
| Narration | `scripts/narration.mjs` | local TTS at 1.6, measured placement, fit check, markers |
| Subtitle re-chunk | `scripts/build-captions.mjs` | 12 words / 72 chars / 900 ms cues from measured audio |
| Beat → scene | `scripts/build-scenes.mjs` | windows derived from the voice, one scene per beat |
| Parallel render | `scripts/render.mjs` | files only, three workers, never races the project |
| Assembly | `scripts/assemble.mjs` | tracks, voice, bed, scenes, captions, chapter markers |
| Re-style captions | `scripts/style-captions.mjs` | the 100-clip `set-text` window, solved |
| Drawing kit | `lib/kit.mjs` | palette, primitives, camera, particles, waveforms |
| UI kit | `lib/ui.mjs` | window, viewer, bin, inspector, timeline, playhead |
| Composers | `lib/composers.mjs` + `lib/scenes/*.mjs` | 38 reference beats in eight chapters |
| Beat chooser | `beats/beat-library.json` | what each composer is for |
| Review gates | `CHECKLIST.md` | frame, chapter, caption, voice, duration, honesty |
| Reference film | `videos/example-story/` | the 5:00 script and beat map |

## The house style, in numbers

| | Value | Why |
|---|---|---|
| Voice | from **Settings → Narration** (reference: Ngọc Huyền, VieNeu-TTS local) | never hardcode a voice name; `script.voice` overrides for one film |
| Tempo | **1.6** | the house reading speed; 1.0 sounds like an audiobook |
| Gaps | **190 ms** between lines | a breath, not a pause |
| Speaking rate | **≈28 characters/second** at 1.6 | measured, used by the fit check; recalibrate per voice |
| Line length | 25–45 words, 3.5–5.5 s | one idea per line |
| Film length | measured voice + 1.4 s tail, or an exact `duration_ms` | a documentary is as long as its argument |
| Speech coverage | **90–96 %** of runtime | unlike the showcase, the voice carries the film |
| Beats | 4–13 s, 25–40 per five minutes | a beat is one idea |
| Chapters | 12 numbered markers in 5:00; a chapter lasts as long as its argument (7–50 s) | the marker is a promise, not a metronome |
| Captions | Be Vietnam Pro 25 pt, bottom, −280 bp, `#F4F6FA` on `#0B0D11` @120, pad 8000 | fixed under a moving camera |
| Cue rules | ≤12 words, ≤72 chars, ≥900 ms, +110 ms lead | two to three cards per spoken line |
| Music | 120 bpm, **constant −6 dB**, no per-sentence ducking | a bed, not a ducking demo |
| Frame | 1920×1080, 30 fps | whole-second cuts land on the 0.5 s beat grid |

## The six steps

Run from the workspace root (the folder with `project.json` and `tools/or`).
`$V` is the video folder, `$T` is this template's folder.

```sh
node $T/scripts/new-video.mjs --slug my-film --title "My film" --seconds 300

# 1. Write the words and the chapter plan. One line per idea, 25–45 words.
$EDITOR videos/my-film/script.json videos/my-film/video.config.json

# 2. Speak the script at 1.6 and measure it. Needs the local TTS server running.
node $T/scripts/narration.mjs --video videos/my-film --attach project.json --json

# 3. Re-chunk the measured audio into documentary cues.
node $T/scripts/build-captions.mjs --video videos/my-film

# 4. Build the bed: 120 bpm, then bake it to −6 dB (clip volume is app-only).
node tools/music.mjs --out work/music/bed.wav \
  --prompt "cinematic documentary, warm, 120 bpm, light drums" --json
ffmpeg -y -v error -i work/music/bed.wav -filter:a "volume=-6dB" \
  -ac 2 -ar 48000 work/music/bed-minus6.wav

# 5. Beat windows → scenes → rendered clips. Three workers is the sweet spot.
node $T/scripts/build-scenes.mjs --video videos/my-film
node $T/scripts/render.mjs --video videos/my-film --workers 3 --attach project.json

# 6. Import everything into the open project, review, then save.
node $T/scripts/assemble.mjs --video videos/my-film --attach project.json --dry-run
node $T/scripts/assemble.mjs --video videos/my-film --attach project.json
tools/or project save --attach project.json --json
```

`assemble.mjs` creates the tracks it needs, clears the ones it owns, imports the
SRT as a real caption track, styles every cue, and refreshes the chapter markers.
`--dry-run` prints the plan without touching the project.

## Config shape

```jsonc
{
  "slug": "my-film",
  "title": "My film",
  "tempo": 1.6,                       // narration time-stretch; 1.6 is the house tempo
  "gap_ms": 190,                      // breath between lines
  "tail_ms": 1400,                    // film ends this long after the last measured line
  "duration_ms": 300000,              // optional: pin an exact runtime instead
  "canvas": { "width": 1920, "height": 1080 },
  "narration": { "chars_per_second": 28 },   // measured rate at this tempo
  "captions": {
    "anchor": "bottom", "sizePt": 25, "offsetY": -280, "weight": "medium",
    "font": "be-vietnam-pro", "color": "#F4F6FA",
    "background": "#0B0D11", "backgroundPadding": 8000, "backgroundOpacity": 120,
    "chunk": { "max_words": 12, "max_chars": 72, "min_cue_ms": 900, "lead_ms": 110 }
  },
  "music": { "file": "work/music/bed-minus6.wav", "duckDb": null, "levelDb": -6 },
  "beats": [
    { "id": "open", "composer": "coldOpen", "start_ms": 0, "lines": ["mo1", "mo2"] },
    { "id": "editor", "composer": "editorBuild", "lines": ["ui1", "ui2"] },
    { "id": "close", "composer": "close", "lines": ["ket1"] }
  ]
}
```

* `lines` are `script.json` ids. A line's `start_ms` is where its voice sits. Set
  `start_ms` on a chapter opener; leave it off elsewhere and the voice places the
  line end to end. The beat window is derived from its lines, so **do not
  hand-write `end_ms`** — the film's length comes from the voice, or from
  `duration_ms`.
* `composer` is a key in `beats/beat-library.json`. A beat without a composer the
  kit knows is refused, not silently skipped.
* `duckDb: null` is deliberate. A documentary keeps a constant bed under the
  voice; ducking is for a showcase with big music beats. Set a negative number
  only when the bed is an event, not a floor.

## The reference arc (12 chapters, 5:00)

| Window | Chapter marker | Beats | What carries it |
|---|---|---|---|
| 0:00.5–25.6 | 01 · Mở | coldOpen, layerFlow, layersAlign | a shot becomes four layers, the layers become a timeline |
| 25.6–34.1 | 02 · OpenCut Reinforced | reveal | the product is named once |
| 34.1–68.0 | 03 · Dựng phim | editorBuild, editActions, cutRipple, previewExport | the real editor, exact-frame edits, a deliverable |
| 68.0–117.7 | 04 · Chat Assistant | assistantScope, assistantRole, chatAsk, acpBridge, harnessRail, chatResult | the assistant reads the project; ACP keeps keys on the near side |
| 117.7–142.4 | 05 · Motion Scene | motionCode, motionBuild, motionMorph, motionTimeline | code becomes geometry, geometry becomes a clip |
| 142.4–170.1 | 06 · Giọng đọc local | ttsScript, ttsWave, whisperListen, whisperCues | local voice, local transcript, reviewed captions |
| 170.1–179.9 | 07 · Quy trình khép kín | workflowLoop | the loop closes on one machine |
| 179.9–208.0 | 08 · Miễn phí và sở hữu | mitDoc, localCost, ownership | what free means, what still costs, who owns the files |
| 208.0–233.3 | 09 · Thực tế và giới hạn | reality, releaseCard, platformGap | the honest limits of this release |
| 233.3–254.7 | 10 · Bài toán mobile | mobileQuestion, providerTrap, notarize | the mobile gap and the install reality |
| 254.7–261.8 | 11 · Cộng đồng | community | the invitation |
| 261.8–300.0 | 12 · Cùng xây | contribPath, checks, commitment, close | how to contribute, what is checked, the end card |

Vary the viewpoint between beats: wide shot, user view, close-up, inside-view.
Two dense UI beats in a row is a fail even if both are polished.

## The subtitle method

Captions are a **real caption track**, never baked into the scene. The method has
three parts:

1. **Timing from the measured audio, not the text.** `narration.mjs` speaks and
   stretches each line, then records `start_ms`, `end_ms` and `duration_ms`.
   `build-captions.mjs` re-chunks the *caption text* over that measured interval.
   No model, no transcription, no TTS call in the re-chunk step.
2. **Readable cards, never leading the voice.** At most 12 words / 72 characters,
   at least 900 ms on screen, later chunks start 110 ms after their character
   position, and no cue may overlap the next. The reference film averages 9 words
   and 1.4 s per cue — 191 cues for 65 lines.
3. **One styled layer.** `assemble.mjs` imports the SRT and runs `timeline
   set-text` on every cue: bottom anchor, 25 pt, −280 bp, Be Vietnam Pro,
   `#F4F6FA`, `#0B0D11` at 120 alpha, 8000 milli-point padding. Because
   `set-text` only searches the first 100 clips, the styling runs in windows of
   96 on a temporary caption track and moves the spill back. The finished project
   has one caption track with every cue styled.

## Rules the template enforces

* **The windows tile the film.** Every beat is non-empty, the first starts at 0,
  each starts where the previous ended, the last ends at the film length.
* **No line is silently cut.** A line under 80 % of its expected duration is
  re-spoken sentence by sentence and stitched.
* **A line that overruns its space is reported.** `narration.mjs` prints the
  slack; fix the wording, never speed the voice up to fit.
* **Every chapter is a marker.** Markers are rebuilt from the script's `marker`
  fields, so the timeline outline and the film agree.
* **Renders are revision-safe.** `render.mjs` writes files only; every project
  write happens later, sequentially, in `assemble.mjs`.
* **Every frame moves.** A composer animates a camera over the whole beat and
  keeps two or three secondary motions running.

## Pitfalls that cost time

* **`timeline set-text` searches 100 clips.** A five-minute film has ~190 cues.
  Style 96 at a time on a temporary caption track, then move them back (both
  `assemble.mjs` and `style-captions.mjs` do this).
* **A media item caches the duration it was imported with.** Re-rendering a file
  in place keeps the old length; remove the media item when a render's length
  changes.
* **`insert-clip --duration` must not exceed the media duration.** Floor to
  milliseconds.
* **Keep rationals reduced** (`591/500`, not `1182000/1000000`).
* **Node ids are unique across a whole scene.** Prefix every piece (`imSun`,
  `auWave`, `c1`); a duplicate id fails validation.
* **Scene text lives inside a 5 % margin.** Keep type at 14 px or larger and
  lines short; everything readable stays above `SAFE_BOTTOM`.
* **Camera zoom stays 0.94–1.5** except for a deliberate dive; the backdrop is
  oversized so a zoom never shows the canvas edge.
* **Render and assemble are separate steps.** Editing the project while renders
  run causes revision conflicts.
* **Three parallel workers** is the sweet spot; more adds contention.
* **The TTS server must be running** at the endpoint in `narration-settings.json`.
  The voice name comes from Settings → Narration; `script.voice` is the one-film
  override.
* **Clip volume is app-only.** Bake the −6 dB bed into the file with ffmpeg; the
  CLI has no gain verb.

## A worked example

`videos/example-story/` is the reference film's script and beat map: 65 lines,
37 beats, 12 chapters, 5:00. It expects `work/music/bed-minus6.wav` (step 4) and
runs the six steps unchanged. The script leaves `voice` empty on purpose — the
reference machine's voice was Ngọc Huyền; on another machine the measured timing
shifts, and `build-scenes.mjs` will say so until `duration_ms` is adjusted.

## Provenance

Built from project `11` in the OR workspace: 37 MotionScene beats, 65 narration
lines at tempo 1.6 with 190 ms gaps (285.9 s spoken), 191 caption cues on one
track, a 120 bpm bed at a constant −6 dB, 12 chapter markers, 30 fps, 1920×1080.
The kit, the UI primitives and the 38 composers are those beats with the
project-specific narration removed.
