---
name: or-motion-scene
description: Author, validate and materialise a deterministic MotionScene in the open OR project — code-to-illustration, animated titles, diagrams and reusable scene JSON. Use when the user asks for an illustration, motion graphic or scene from code. Do NOT use for ordinary edits (or-timeline-edit) or for changing OR's own MotionScene implementation.
---

# Motion scene

Workspace guide: `docs/motion-scenes.md`.

## Procedure

1. Decide the input: a source file (code illustration) or an authored
   `.ormotion.json` (title, diagram, reusable scene).
2. For a scene, start from `node tools/motion-scaffold.mjs --name "Title"
   --out scenes/title.ormotion.json`, then edit nodes, text, colours, sizes and
   keyframes.
3. Validate with `tools/or motion validate --file scenes/title.ormotion.json
   --json`. Fix every diagnostic; do not render a scene that fails validation.
   Read the composition with `tools/or motion layout --file ... --at 1/2
   --json`; the node boxes are the layout, so do not ask for a screenshot.
4. Publish it with `tools/or motion publish --attach project.json --scene
   scenes/title.ormotion.json --json` (or `--source`). One call validates, saves
   the scene to the app's motion library, renders it, imports the video and
   inserts an ordinary clip. Materialisation encodes every frame: wait for the
   result and never run two at once.
5. Verify the clip appears with `tools/or timeline clips --track TRACK_ID
   --attach project.json --json`, then `tools/or project save --attach
   project.json --json` unless the user asked to review first.

## Rules

- The scene is declarative and bounded: no expressions, scripts, network assets
  or provider calls. Refer to assets by scene-local or project-relative path.
- Flat colour only (3–5 fills), at least two animations running across the whole
  scene, short Vietnamese titles with their diacritics intact, and no empty
  static middle. The scaffold starts from that shape. Read the boxes with
  `motion layout` instead of trusting a still.
- Text is literal; changing the title means changing the `text` field.
- A re-render never replaces the existing clip. Insert the new render and remove
  the old clip only when the user asks.
- Keep scenes short; a long scene is slow to encode and hard to revise.
- A reusable asset is listed with `tools/or motion asset list --attach
  project.json --json`, inserted with `tools/or motion asset insert --attach
  project.json --asset-id ID [--values JSON] --json`, and saved from an authored
  scene file by `motion publish` itself. Do not copy or edit library files
  yourself. Use the `mcp__or__motion_*` tools only if the CLI call fails and its
  error does not explain why.

## Report

State the scene file, its duration and canvas, the track and time the clip was
inserted at, and whether the project was saved.
