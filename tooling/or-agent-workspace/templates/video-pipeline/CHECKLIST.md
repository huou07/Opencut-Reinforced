# Review gates

Run these before rendering and again before saving. Each gate is a question, not
a checkbox you can tick by habit. The standard is the 59 s showcase, not "good
enough for a rough cut".

## 1. Frame gate — does every moment move?

Sample the beat at five times, including the middle.

* Is there at least one camera change and one secondary motion in *every* sample?
  A frame where only opacity is fading is a still frame — fail it.
* Is the beat a title card, a paragraph, or a diagram that replaces another
  diagram? Then it is not in the showcase style: rebuild it as a transformation.
* Does anything change form (a line becomes geometry, a clip becomes a layer, a
  code line becomes a shape), or does content merely appear and disappear?
* Does the shot hand off to the next one — a match cut, a dive, a fold, a wipe —
  rather than ending on an empty frame?

## 2. Composition gate

* Is all readable content above the caption band (`SAFE_BOTTOM` in `kit.mjs`)?
* Are there at most six or seven words in one line, at 14 px or larger?
* Does any shape or text leave its panel and sit over unrelated UI? (Rotated
  rectangles are the usual culprit.)
* Is the palette the same three-to-five flat colours as the rest of the film?
  Gradients, stacked bands that read as a gradient, and glow soup all fail.
* Does the camera ever expose the canvas edge? Keep zoom ≥ 0.94 except mid-dive.

## 3. Motion gate

* Does the camera move for the whole beat, not only at the cut?
* Do two elements interact — particles following a beam, a clip landing where the
  cursor pointed, a waveform and a caption tightening together?
* Is the beat's own idea visible in the first 0.8 s, before any text explains it?
* Does the motion read with the sound off? Mute the film and look again.

## 4. Sound gate

* Does each caption appear at or after the voice reaches that phrase?
* Does any line overrun the space before the next line? `narration.mjs` reports
  the slack; fix the wording rather than speeding the voice up.
* Is the bed a beat under every cut? 120 bpm at 0.5 s per beat is the default.
* Is the music −9 dB under the voice and still present in the gaps?
* Does the spoken audio fill 45–60 % of the runtime, with deliberate silence
  after the big moments?

## 5. Duration gate

* Does `build-scenes.mjs` report "windows tile exactly"?
* Does the video track run 0 → `duration_ms` with no gap and no overlap?
* Is the last frame the end of the last beat — no trailing black?
* Is the frame count `duration_ms / 1000 × fps` exactly (59 s × 30 fps = 1770)?
* If you exported, does the file report exactly `00:00:59.000` at its timebase?

## 6. Story and honesty gate

* If a beat were cut, would the viewer lose information, understanding or a
  memorable moment?
* Does the narration name what is on screen at that moment?
* Is every integration shown one the build actually supports? Do not show a
  harness, provider or model that is not in the app.
* Is a paid service described as paid, and a local/free path as local/free?
  Never imply paid services are free.
* Say which steps were verified in this session and which were not. Never claim
  the narration was listened to, or the export watched, if it was not.
