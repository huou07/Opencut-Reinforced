# Media and import

Scope: getting a file into the project library and finding the id that the
timeline commands take. The library is project state; importing is a project
mutation and is saved like any other edit.

## Order of operations

1. `tools/or media list --attach project.json --json` — check whether the file is
   already imported. The list is paged: pass `--offset` while the page reports
   more items.
2. `tools/or media add --attach project.json --source PATH --json` — import an
   **absolute** path. The result carries the new media id and its metadata.
3. Place it with `timeline insert-clip` (see
   [timeline-editing.md](timeline-editing.md)); use the duration from this result.
4. `tools/or project save --attach project.json --json` after the edit, unless the
   user asked to review first.

## What can be imported

- Video, audio and image files that the app's FFmpeg build decodes.
- A file must exist and be readable when `media add` runs; the library records a
  source path, not a copy.
- Importing the same path twice is allowed; use the id the new call returns.

## Paths

- `--source` must be absolute. The workspace sits outside the project, so a path
  relative to the project is not automatically correct.
- Files the agent downloaded or wrote live under `work/`; copy or keep them there
  and pass that absolute path.
- Do not move or delete a source file after importing it. If it moves, relink it:
  `tools/or media relink --attach project.json --id MEDIA_ID --source PATH --json`.
- `tools/or media remove --attach project.json --id MEDIA_ID --json` removes a
  library entry when the user asks for it.

## Reading metadata

`media list` returns ids, source paths and duration metadata. Use the returned
duration when a clip needs a length; do not round it into a different time base.
`project summary` reports the project revision and dirty state.
