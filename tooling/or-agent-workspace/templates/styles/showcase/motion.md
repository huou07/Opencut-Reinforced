# Motion

## Frame

- Canvas 1920×1080 at 30/1. One canvas for the whole film.
- A scene runs 4–8 s and carries one headline. Longer means split it.

## Palette

- Three to five flat fills: background, ink, one accent, one muted tone.
- Reuse the same fills across the film. No gradients, no stacked bands, no
  per-beat palette.
- Background is the darkest or lightest value; text never sits on the accent.

## Type

- Titles are 6–7 words per line, at least 28 px at 1080p, inside a 5 % margin.
- One type scale: headline, support, label. Do not add a fourth size.
- Let the renderer pick the face; write NFC text and keep diacritics.

## Camera and animation

- A slow push-in from zoom 1.0 to 1.08, or a pan of at most 32 px, keeps every
  frame alive. Keep zoom ≥ 1.
- Two or three animations run at once: drift, pulse, wipe. Entrance fades alone
  read as a slideshow.
- Entrance 300–500 ms, ease-out; exit 200–300 ms. Nothing snaps.
- No dead frame: at every second, something moves.

## Check

`tools/or motion layout --file SCENE --at 1/2 --json` — read the boxes, not a
screenshot. Text must stay inside the canvas and clear of the safe margin.
