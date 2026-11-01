---
name: or-video-production
description: Produce a complete multi-scene video — beat sheet, parallel scene renders, assembly, caption pass and delivery export — with the workspace pipeline and style packs.
---

# OR video production

Use when the user asks for a whole video, explainer, montage or any piece built
from several motion scenes. For one scene use `or-motion-scene`; for narration
alone use `or-script-production`.

## Read first

- `docs/motion-production.md` — splitting, parallel renders, assembly.
- `templates/video-pipeline/README.md` — the machinery and its config.
- `templates/styles/showcase/README.md` — the default style numbers.
- `docs/captions.md` — cue rules and styling.

Read the style pack only for the decision in front of you; do not preload all six
files.

## Procedure

1. Fix length, language, audience and voice with the user. Do not invent a
duration.
2. Write the beat sheet to `work/<slug>/beats.json`: one beat per scene, 4–8 s,
one headline, the proof it shows.
3. Write the script and narrate it (`tools/voiceover.mjs --plan`, then `--apply`).
   Scene duration comes from the measured narration.
4. Generate the music bed and set it under speech.
5. Author each scene from the scaffold; validate every file.
6. Render all scenes in parallel — `render.mjs --workers 3` or one background job
   per stream. Renders are file-only and safe to run together.
7. Assemble sequentially: import, place in order, style captions, save once.
8. Export once the user asked for a file: `tools/or export start --attach
   project.json --destination work/exports/<slug>.mp4 --json`; add
   `captions export --out work/exports/<slug>.srt` when a sidecar is wanted.
   Poll with `export status` and report the job id and path.
9. Report rendered/failed counts, total duration, the export path, and the saved
   revision.

## Rules

- Project mutations are serial; only renders run in parallel.
- Read composition with `motion layout --at`, not screenshots.
- Export is the last step and renders every frame; never use it as a smoke test.
  A `succeeded` job with `completed == total` is the only export evidence.
- One style pack per video; do not mix packs.
- Package method only: when the user praises the result, promote the numbers to a
  style pack, never the story text.

## Report

Say what was produced: beat count, runtime, scene count, caption cue count, the
style pack used, and any beat that failed or was skipped.
