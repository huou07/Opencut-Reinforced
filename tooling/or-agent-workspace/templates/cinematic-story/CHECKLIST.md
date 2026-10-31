# Review gates — the documentary standard

Run these before rendering and again before saving. Each gate is a question, not
a checkbox. The reference is the 5:00 film in project `11`; "good enough for a
rough cut" is not a pass.

## 1. Chapter gate

* Does every chapter open a new viewpoint, and close by naming what it
  established?
* Is each marker on the first line of its chapter, labelled the way a viewer
  would name the section — not an internal beat id?
* Would cutting a chapter lose information, understanding or a memorable moment?
  If not, cut it.
* Is the order an argument (hook → tool → proof → limits → invitation), not a
  feature tour?
* Does the film end on the last frame of the last beat, with no trailing black?

## 2. Frame gate — does every moment move?

Sample the beat at five times, including the middle.

* Is there at least one camera change and one secondary motion in *every* sample?
  A frame where only opacity is fading is a still frame — fail it.
* Does anything change form, or does content merely appear and disappear?
* Does the shot hand off to the next one — a match cut, a dive, a fold, a wipe —
  rather than ending on an empty frame?
* Is the beat's idea visible in the first 0.8 s, before any text explains it?
* Does the motion read with the sound off?

## 3. Composition gate

* Is all readable content above the caption band (`SAFE_BOTTOM = 900`)?
* Are there at most six or seven words in one line, at 14 px or larger?
* Does any shape or text leave its panel and sit over unrelated UI?
* Is the palette the same three-to-five flat colours as the rest of the film? A
  gradient, a stack of bands that reads as a gradient, or glow soup fails.
* Does the camera ever expose the canvas edge? Keep zoom ≥ 0.94 except mid-dive.

## 4. Voice gate

* Does each caption appear at or after the voice reaches that phrase?
* Does any line overrun the space before the next line? `narration.mjs` prints the
  slack; rewrite the line, never speed the voice up.
* Is the spoken rate within 10 % of `narration.chars_per_second`? A line far
  under it was re-spoken and stitched; check the seam.
* Are the 190 ms gaps intact, with a longer breath only at chapter turns?
* Does speech fill 90–96 % of the runtime, with silence reserved for a chapter
  turn or the final card?
* Is the voice the one in Settings → Narration (or the deliberate one-film
  `script.voice` override), and was the local TTS server actually running?

## 5. Caption gate

* One caption track, every cue styled: Be Vietnam Pro 25 pt, medium, bottom,
  −280 bp, `#F4F6FA` on `#0B0D11` @120, padding 8000.
* ≤12 words and ≤72 characters per cue; ≥900 ms on screen; no cue overlaps the
  next.
* Does any cue lead the voice? The first chunk starts at the measured start;
  later chunks at character position +110 ms.
* Are there roughly two to three cues per spoken line (the reference: 191 cues
  for 65 lines)?
* After a re-import, did `style-captions.mjs` run over **all** cues, including
  the ones past the 100-clip search window?

## 6. Sound gate

* Is the bed 120 bpm, constant, and present in the gaps?
* Is it at −6 dB, baked into the file rather than a clip setting?
* Is there exactly one bed for the whole film?
* Does a whole-second cut land on the 0.5 s beat grid?

## 7. Duration and assembly gate

* Does `build-scenes.mjs` report "windows tile exactly"?
* Does the video track run 0 → film length with no gap and no overlap?
* Is the frame count `duration_ms / 1000 × 30` exactly (300 s × 30 = 9000)?
* Are the marker count and labels exactly the script's chapters?
* Did `assemble.mjs` clear only the tracks it owns, and leave other material
  alone?

## 8. Honesty gate

* Is every integration shown one the build actually supports? Do not show a
  harness, provider or model that is not installed.
* Is a paid service described as paid, and a local/free path as local/free? Never
  imply paid services are free.
* Are version, date, platform and signing status the ones verified for this
  build?
* Say which steps were verified in this session and which were not. Never claim
  the narration was listened to, or the export watched, if it was not.
