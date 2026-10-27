---
name: or-motion-scene
description: Author, validate and materialise a deterministic MotionScene in the open OR project through mcp__or__motion_insert_scene — code-to-illustration, animated titles, diagrams and reusable scene JSON. Use when the user asks for an illustration, motion graphic or scene from code. Do NOT use for ordinary edits (or-timeline-edit) or for changing OR's own MotionScene implementation.
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
   --json` when the CLI exists. Fix every diagnostic; do not render a scene that
   fails validation.
4. Place it with `mcp__or__motion_insert_scene {scene_path}` or `{source_path}`.
   Materialisation encodes every frame: wait for the result and never run two at
   once.
5. Verify the clip appears with `timeline_overview`, then `save` unless the user
   asked to review first.

## Rules

- The scene is declarative and bounded: no expressions, scripts, network assets
  or provider calls. Refer to assets by scene-local or project-relative path.
- Text is literal; changing the title means changing the `text` field.
- A re-render never replaces the existing clip. Insert the new render and remove
  the old clip only when the user asks.
- Keep scenes short; a long scene is slow to encode and hard to revise.
- A reusable asset is listed with `mcp__or__motion_asset_list` and inserted with
  `mcp__or__motion_asset_insert`; pass parameter values by id. Do not copy or
  edit library files yourself.

## Report

State the scene file, its duration and canvas, the track and time the clip was
inserted at, and whether the project was saved.
