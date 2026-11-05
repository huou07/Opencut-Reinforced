# Design and type

Scope: making a generated video look deliberate rather than assembled — hierarchy,
type, colour, spacing and motion. This is the guide to read before authoring a
title card, a lower third, an infographic or a scene's animation timing. The
mechanics of the scene format are in [motion-scenes.md](motion-scenes.md); the
timeline and text clips are in [timeline-editing.md](timeline-editing.md).

## Choose the form before the tool

| Content | Form | Not this |
|---|---|---|
| A statistic or a comparison | flat vector shapes, one accent, direct labels | a 3D chart |
| A phrase the viewer must remember | type only, one line, generous margin | a decorated card with an icon set |
| A product or an object | photography, or one authored 3D object | a stock illustration |
| A process or a sequence | numbered steps that appear in reading order | everything on screen at once |
| A person talking | the footage plus a lower third | a full-screen title over their face |

One scene carries one idea. If a scene needs two focal points, it is two scenes.

## Type scale

All sizes are a fraction of the canvas **short side**, so a 1080p and a 4K render
look the same.

| Role | Size | Weight | Line height | Tracking |
|---|---|---|---|---|
| Display headline | 0.10–0.13 | Bold | 0.95–1.05 | −0.5 to −1.5 |
| Section title | 0.055–0.07 | Semibold | 1.1 | −0.25 to −0.75 |
| Lead paragraph | 0.032–0.038 | Regular or Medium | 1.35–1.5 | 0 |
| Body or list | 0.026–0.030 | Regular | 1.4–1.55 | 0 |
| Caption or label | 0.020–0.024 | Medium | 1.2 | 0.25 to 0.75 |
| Overline or eyebrow | 0.016–0.020 | Semibold, upper case | 1.1 | 1.0 to 2.0 |

Rules that make the scale read as one family of choices:

- Two sizes per scene is usually enough; three is the maximum. A headline and a
  body at 2× the body size already separate cleanly.
- Negative tracking only at display sizes. Never track body text tighter than 0.
- Keep line height between 1.0 and 1.2 for display type, 1.35–1.55 for paragraphs.
  Tighter reads as a poster; looser reads as a document.
- Short lines: 20–32 characters for a display headline, 45–60 for a paragraph.
  Break at a phrase boundary, never mid-word.
- Use one family per video, two at most, and only ever Inter and Be Vietnam Pro:
  they are the families the renderer bundles. The renderer picks Be Vietnam Pro
  automatically for non-ASCII text, so a Vietnamese title and its English
  subtitle share a voice.

In a MotionScene these are the text node's `font`, `font_weight`, `text_align`,
`line_height`, `letter_spacing` fields inside an explicit `size` box; in a text
clip they are the visible text controls. In both, the box is part of the design:
text does not float, it occupies a measured area.

## Layout

- Action-safe margin: keep meaning inside 5% of each edge. Title-safe: 10%.
  The editor's composition guides draw exactly these two insets plus thirds, so
  design with the guides on and the result matches the export.
- Use the thirds for placement, the centre only for a deliberate symmetric
  moment. A headline on the left third with a figure on the right reads faster
  than centred everything.
- One alignment per scene. Left-aligned type with a ragged right edge is easier
  to read than centred stacks of unequal lines.
- Space is a material: the gap between a headline and its support line should be
  about one line height of the support text, not an arbitrary 24 px.
- Optically centre pictures: a subject with headroom needs the frame shifted,
  not the text.

## Colour and contrast

- Start from the footage. Sample one dominant colour and use it as the accent;
  everything else is neutral (near-black, near-white, one grey).
- Text over video needs a plate: either a solid scrim at 60–80% opacity, a solid
  band, or a strong darkening gradient. White text on raw footage fails as soon
  as the footage brightens.
- Meet 4.5:1 contrast for body text and 3:1 for large display text against the
  actual backdrop, not against the intended one.
- One accent colour per video, used for one thing: the number being emphasised,
  the active step, the key word. A second accent halves the effect of the first.
- Gradients belong in the background plate, not in text or icons.

## Motion

Motion explains a change; it is not decoration.

| Move | Duration | Easing |
|---|---|---|
| Entrance (fade, rise, scale 0.96 → 1) | 180–320 ms | ease-out |
| Exit | 120–220 ms | ease-in |
| Emphasis (small scale or colour pulse) | 160–240 ms | ease-in-out |
| Camera pan or zoom | 600 ms and up | linear or very gentle ease |
| Stagger between sibling elements | 60–90 ms per item, 4 items maximum | same ease-out |

- Travel no more than 8% of the canvas width for an entrance; large slides look
  like a template, small ones look composed.
- Nothing moves while the viewer is reading. Hold the settled frame for at least
  1.2 s, longer for a two-line headline.
- At most three elements animate at once. A fourth element waits its turn.
- Match the beat: land entrances on the music or the narration's stressed
  syllable, and keep one scene's entrances within a single bar where possible.
- Camera moves are a slow push (≤ 3% scale per second) or a short pan; a rotating
  or swinging camera reads as an error.

## Captions and lower thirds

- Captions: two lines maximum, 42 characters per line at 1080p, 15–20 characters
  per second of reading speed, bottom safe area, never over a face or a chart's
  labels.
- Burned-in captions are part of the design: match the type family and the
  contrast plate rule, and keep them out of the way of scene content.
- A lower third carries name and role at the section size, on a plate, entering
  once and leaving once. It does not loop or bounce.

## Charts and infographics

- Baseline at zero, one unit label, values ordered by size unless the order is
  the story.
- Direct labels beat legends for four series or fewer; a legend is a lookup the
  viewer will not perform.
- No 3D charts, no drop shadows on data, no gradient fills that encode a value.
- Numbers animate by counting or by growing the bar, never by spinning.
- An infographic is a sequence of three to five reveals, each one idea, each
  landing on a beat.

## Checking before delivery

1. Squint test: with the frame blurred, the focal point is still obvious.
2. Safe-area check: guides on, nothing meaningful outside title-safe.
3. Contrast check: the smallest text is readable on the brightest frame it
   appears over.
4. Reading check: pause on every text frame; if you cannot read it in one pass,
   it is too much or too small.
5. Parity check: the preview and the exported file agree on type, colour, timing
   and crop. If they do not, the export is wrong, not the preview.
