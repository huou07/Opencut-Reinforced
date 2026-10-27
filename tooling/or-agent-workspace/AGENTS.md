# OR Agent Workspace

This is the operating workspace the Opencut Reinforced assistant uses by default.
It is not the OR source checkout. The open project is recorded in `project.json`,
and the `mcp__or__*` tools are the only way to read or change that project.

## Rule

Read the smallest entry that answers the current task. Do not preload the guides:
each line below names the situation that makes it the right thing to read.

## Routing

### Guides

- **Timeline editing** — tracks, clips, markers and sequence commands with exact times.
  When: the user asks to add, move, trim, split, delete, reorder, re-time or mark something on the timeline.
  Path: `docs/timeline-editing.md`
- **Media and import** — media ids, importing, paging and source relinking.
  When: the task needs a file in the library before it can be placed, or needs an existing media id.
  Path: `docs/media-and-import.md`
- **Motion scenes** — code-to-video scenes, `.ormotion.json`, validation and materialisation.
  When: the user asks for an illustration, motion graphic, animated title or scene from code.
  Path: `docs/motion-scenes.md`
- **Project safety** — revisions, save, undo/redo and refused operations.
  When: deciding whether to save, undoing a change, or recovering from a failed tool call.
  Path: `docs/project-safety.md`
- **Troubleshooting** — missing tools, no open project, slow or failed renders.
  When: a tool call fails and the error is not self-explanatory.
  Path: `docs/troubleshooting.md`

### Skills

The session catalog carries these. Load one only when its task matches.

- `or-timeline-edit` — orient, act, verify and save one timeline change.
- `or-motion-scene` — author, validate and materialise a deterministic scene.
- `or-project-safety` — protect revisions and report edits honestly.

### Tools

- `tools/or` — the OR CLI bound to the app build. Use it for
  `motion validate`, `inspect`, `from-code` and `render` when a scene must be
  checked without starting a materialisation. If it is missing, see
  `docs/troubleshooting.md`.
- `tools/motion-scaffold.mjs` — writes a valid V1 `.ormotion.json` skeleton:
  `node tools/motion-scaffold.mjs --name "Title" --out scenes/title.ormotion.json`.

## Working files

- `scenes/` — scene sources you author. `mcp__or__motion_insert_scene` reads them.
- `work/` — scratch files, notes and copies. Nothing here is project state.
- `project.json`, `or-cli.json` and `tools/or` are written by the app; leave them alone.

## Bounds

- Never edit a `.orproj` file, the project directory, or any path listed in `project.json`.
- Project mutations happen only through `mcp__or__*` calls; imports read local files,
  so a file you copy into `work/` can be imported from there.
- Make one requested change at a time, confirm it with `timeline_overview`, then save
  unless the user asked to review first.
