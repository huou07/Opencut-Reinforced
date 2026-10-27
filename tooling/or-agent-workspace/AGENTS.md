# OR Agent Workspace

This is the operating workspace the Opencut Reinforced assistant uses by default.
It is not the OR source checkout. The open project is recorded in `project.json`,
and the `or` CLI attached through that pointer is the default way to read or
change it. An `mcp__or__*` tool is a fallback for the few operations the CLI
cannot express.

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
- **Project safety** — revisions, save, undo/redo and refused commands.
  When: deciding whether to save, undoing a change, or recovering from a failed command.
  Path: `docs/project-safety.md`
- **Troubleshooting** — missing tools, no open project, slow or failed renders.
  When: a command fails and the error is not self-explanatory.
  Path: `docs/troubleshooting.md`

### Skills

The session catalog carries these. Load one only when its task matches.

- `or-timeline-edit` — orient, act, verify and save one timeline change.
- `or-motion-scene` — author, validate and materialise a deterministic scene.
- `or-project-safety` — protect revisions and report edits honestly.

### Tools

- `tools/or` — the OR CLI bound to this app build and the default project surface.
  Run it from this workspace: `tools/or <verb> --attach project.json --json`.
  `project.json` is the app-written session pointer; the CLI follows its
  `descriptor_path`, so the same command line keeps working when another project
  opens. `tools/or commands` and `tools/or queries` print the exact verb surface.
- `tools/motion-scaffold.mjs` — writes a valid V1 `.ormotion.json` skeleton:
  `node tools/motion-scaffold.mjs --name "Title" --out scenes/title.ormotion.json`.
- `mcp__or__motion_insert_scene`, `mcp__or__motion_asset_list`,
  `mcp__or__motion_asset_insert`, `mcp__or__motion_asset_save` — the app's MCP
  fallback. Use them only for motion insertion and the motion library, where the
  CLI can render a file but cannot import or place a clip. A CLI failure whose
  error does not explain itself is the other fallback; report it before switching.

## Working files

- `scenes/` — scene sources you author. `mcp__or__motion_insert_scene` reads them.
- `work/` — scratch files, notes and copies. Nothing here is project state.
- `project.json`, `or-cli.json` and `tools/or` are written by the app; leave them
  alone. `project.json` names the live descriptor the CLI attaches through.

## Bounds

- Never edit a `.orproj` file, the project directory, or any path listed in `project.json`.
- Project mutations go through `tools/or ... --attach project.json`; the app applies
  them and owns history. The motion MCP tools are the only other write path.
- Make one requested change at a time, confirm it with a timeline read, then save
  unless the user asked to review first.
