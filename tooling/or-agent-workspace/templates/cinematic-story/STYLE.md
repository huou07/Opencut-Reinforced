# Style — the cinematic documentary grammar

The look and the rhythm this template packages, stated as rules a composer and a
script can be judged against. The reference is the 5:00 OpenCut Reinforced film in
project `11`. Read this before writing a beat or a line; read `README.md` for the
process.

## 1. The frame

* 1920×1080, 30 fps. The sequence rate is set once; every rendered beat is
  `duration × 30` frames exactly.
* The palette is five flat colours, not a gradient: near-black ground
  (`#08090B`), panel (`#111317`), edge (`#2A2F37`), ink (`#EDF0F5`), and one
  accent per chapter (amber `#F4B64E`, cyan `#4FD1C5`, blue `#6094E8`, violet
  `#A78BFA`). A sixth colour is a decision, not a default.
* `SAFE_BOTTOM = 900` in `kit.mjs`. Everything readable lives above it; the
  caption band sits under it. A shape may cross the line, a word may not.
* The backdrop is oversized and the camera never exposes a canvas edge. Zoom
  stays 0.94–1.5 except for a deliberate dive.

## 2. Camera grammar

Every beat has one camera move over its whole length, not a push at the cut.

* **Wide** — establish a world or a workspace. Slow drift, 1.0–1.1.
* **User view** — the frame is the app window; the camera settles, then tracks a
  cursor or a playhead.
* **Close-up** — one control, one word, one waveform. 1.2–1.4.
* **Inside view** — the camera enters the thing it was looking at (a viewer, a
  scene, a code pane). A dive is a handoff, not decoration: it carries the cut.
* **Return** — pull back to the wide for the chapter close, or hand off directly
  to the next beat's first frame. Never end on an empty frame.

Vary the viewpoint between consecutive beats. Two UI beats in a row is a fail
even when both are polished.

## 3. Beat anatomy

A beat is 4–13 s and carries exactly one idea. In the first 0.8 s the idea is
visible before any text explains it. Through the middle, two or three secondary
motions keep running (particles following a beam, a waveform tightening, a clip
landing where the cursor pointed). At the end, the beat hands off: a match cut, a
fold, a wipe, a dive.

Content must **change form**, not merely appear: a line becomes geometry, a clip
becomes a layer, a code line becomes a shape, a layer becomes a timeline. A title
card, a paragraph, or a diagram that replaces another diagram is out of style.

Text rules: six or seven words per line at 14 px or larger, short lines, one
accent word at most. A number on screen is a fact, not a texture.

## 4. Chapters

A chapter is 3–5 beats and one numbered marker (`01 · Mở`, `02 · OpenCut
Reinforced`, …). The marker sits on the first line of the chapter, not on the
first frame of a scene.

* The marker label is what the viewer would call that section, not an internal
  beat id.
* A chapter lasts as long as its argument. In the reference, chapters run 7–50 s;
  the average is 25 s. Do not pad a chapter to hit a cadence.
* A chapter opens with a new viewpoint and closes by naming what it established.
* The last chapter is the end card: mark, one-line value, the honest licence
  line, one action.

## 5. Writing the narration

The voice is the film. Write it before the scenes.

* **One idea per line**, 25–45 words, 3.5–5.5 s spoken. If a line needs a comma
  splice to hold two ideas, it is two lines.
* **Name what is on screen at that moment.** "Đây là timeline" while a timeline
  assembles; not "công cụ mạnh mẽ" over a generic panel.
* **Concrete over adjectival.** "Cắt đúng khung" beats "chỉnh sửa chính xác".
* **Numbers are checked facts.** Version, date, platform, duration, price. If it
  was not verified in this session, it does not go on screen.
* **Honesty is a beat, not a disclaimer.** Say what is missing (mobile, notarisation,
  a paid provider) in its own chapter, in the same confident voice as the wins.
* **No feature lists.** A list is a beat that failed to become a transformation.
* **The close is one action.** "Mở dự án mẫu", "thử một cảnh" — not five links.

## 6. Reading rhythm

* Tempo **1.6** on the local TTS voice. 1.0 reads like an audiobook; above 1.8
  the consonants start to blur.
* **190 ms** between lines. That is a breath. A chapter change may take 400–600 ms,
  but the gap is written into the plan, not left to chance.
* **≈28 characters/second** at 1.6 on the reference voice. Measure it: speak one
  known line, divide characters by seconds, put the number in
  `narration.chars_per_second`. The fit check uses it to catch a line that lost
  words.
* A line that overruns the space before the next line is **rewritten shorter**,
  never sped up. `narration.mjs` reports the slack; a slack under 180 ms is a
  warning, not a style.
* Speech fills **90–96 %** of the runtime. The reference: 285.9 s spoken in
  300.0 s. Silence is reserved for a chapter turn or the final card, not sprinkled
  between lines.
* The film ends 1.4 s after the last measured line (or at an exact `duration_ms`).
  No trailing black.

## 7. Caption craft

* Captions are a **track**, never baked into a scene. The camera moves underneath
  a fixed subtitle.
* Timing comes from the measured audio: first chunk at the measured start, later
  chunks at their character position **+110 ms**, clamped to the next cue.
* Readability: **≤12 words, ≤72 characters, ≥900 ms** on screen. The reference
  averages 9 words and 1.4 s across 191 cues for 65 spoken lines.
* Never overlap the next cue; fold a two-word orphan into the previous card when
  it still fits.
* Look: Be Vietnam Pro, 25 pt, medium, bottom anchor, −280 basis points,
  `#F4F6FA` on `#0B0D11` at 120 alpha, 8000 milli-point padding. One layer, every
  cue styled.
* A caption that leads the voice by even half a second changes the feel of the
  film. Treat the +110 ms as a floor, not a target.

## 8. Music

* **120 bpm**, so every 0.5 s is a beat and every whole-second cut lands on the
  grid.
* **Constant −6 dB** under the voice for a documentary. `duckDb: null` is the
  house setting; per-sentence ducking is a showcase move, not a bed.
* Bake the level into the file (`ffmpeg -filter:a volume=-6dB`). Clip volume is
  app-only; the CLI has no gain verb.
* The bed runs the full film, including the gaps. It is a floor, not an event.
* One bed per film. A second music cue is a chapter decision, not a loop.

## 9. Assembly shape

The finished timeline is four tracks and a marker row:

| Track | Holds |
|---|---|
| video | one clip per beat, 0 → film length, no gaps |
| audio (voice) | one clip per line at its measured start |
| caption | every cue, one track, one style |
| audio (music) | one bed clip, 0 → film length |

Markers are rebuilt from the script, so the outline and the film never disagree.
Renders are files only; assembly is the one sequential pass that writes the
project. Review, then save once.

## 10. What "cinematic" is not

* Not a dark background with white text.
* Not a slow zoom on a still.
* Not a stack of glows standing in for depth.
* Not a feature list read aloud.
* Not a caption that arrives before the voice.
* Not a chapter that exists only to hold a marker.
