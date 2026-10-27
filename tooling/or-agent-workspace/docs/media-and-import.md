# Media and import

Scope: getting a file into the project library and finding the id that the
timeline commands take. The library is project state; importing is a project
mutation and is saved like any other edit.

## Order of operations

1. `mcp__or__media_list` — check whether the file is already imported. The list
   is paged: pass `offset` while `has_more` is true.
2. `mcp__or__add_media {source_path}` — import an **absolute** path. The result
   carries the new `media_id` and its metadata.
3. Place it with `insert_clip` (see [timeline-editing.md](timeline-editing.md)).
4. `mcp__or__save` after the edit, unless the user asked to review first.

## What can be imported

- Video, audio and image files that the app's FFmpeg build decodes.
- A file must exist and be readable when `add_media` runs; the library records a
  source path, not a copy.
- Importing the same path twice is allowed; use the id the new call returns.

## Paths

- `source_path` must be absolute. The workspace sits outside the project, so a
  path relative to the project is not automatically correct.
- Files the agent downloaded or wrote live under `work/`; copy or keep them there
  and pass that absolute path.
- Do not move or delete a source file after importing it; the project would need
  a relink, and no MCP tool performs relink yet.

## Reading metadata

`media_list` returns ids, source paths and duration metadata. Use the returned
duration when you need a clip length; do not round it into a different time base.
`project_status` reports the project revision and dirty state.
