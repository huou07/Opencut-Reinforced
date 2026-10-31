# Style packs

Scope: reusable method for motion, storytelling, script, voice, editing and
captions. A pack records how a video was made, never what it said. The packs
live under `templates/styles/`.

## What a pack is

A pack is six short method files plus a README. It carries timing, palette,
camera, beat shape, cue rules and voice settings — measured numbers, not
adjectives. It carries no narration text, names, claims, logos, footage or
brand-specific content.

## When to promote

Promote a finished video into a pack only when all of these hold:

- the user judged the result good;
- the approach is repeatable on a different topic;
- the content has been stripped out and replaced by rules;
- the numbers were measured, not guessed.

Do not promote a one-off experiment or a video the user has not reviewed.

## Pack layout

| File | Holds |
|---|---|
| `README.md` | what the pack is for, its numbers, and the order to read it |
| `motion.md` | canvas, palette, camera, animation density, type scale |
| `story.md` | beat count and arc, length, speech ratio |
| `script.md` | words per second, sentence shape, caption length |
| `voice.md` | rate, gaps, voice choice, listen-before-apply |
| `edit.md` | cut rhythm, gaps, music level, hold and transition rules |
| `captions.md` | size, anchor, offset, cue length and reading speed |

## Shipped packs

| Pack | Use | Path |
|---|---|---|
| Showcase | one-minute product or feature film | `templates/styles/showcase/README.md` |
| Documentary | long-form chaptered story | `templates/cinematic-story/STYLE.md` |

The video pipeline and the cinematic story template hold the machinery; a style
pack holds only the method. Read the machinery when producing, the pack when
deciding how it should look and sound.

The documentary entry predates this contract: it lives in the cinematic template
and will be split into the six files when it is next revised. The showcase pack is
the reference shape.

## Use a pack

1. Read the pack README and only the file the current decision needs.
2. Keep the canvas, frame rate and structural numbers; adapt speeds and lengths
   to the language, topic and target duration.
3. Write new content. Never reuse the pack's example sentences.
4. Check the result with `motion layout`, a caption read and a first/middle/last
   cue playback.

## Add a pack

1. `mkdir templates/styles/<slug>` and copy the six files from `showcase/`.
2. Replace every number with a measured one and delete anything topic-specific.
3. Add a row to the table in `templates/styles/README.md` and an entry under
   `Templates` in `AGENTS.md`.
4. Rebuild the bundle and run `node --test`; the routing test must pass.

Names are lowercase kebab-case. A pack that only renames another pack is not a
pack.
