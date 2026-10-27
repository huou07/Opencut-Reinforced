# Project safety

Scope: how to change a live project without losing work. Rust owns the project,
its revision and its history; every `mcp__or__*` mutation goes through the same
validated command path as the editor.

## The one rule

Never edit a `.orproj` file or anything inside the project directory directly.
The running app holds the authoritative document; a direct edit is invisible to
it and will be overwritten or rejected. Use the MCP tools.

## Reading state

`mcp__or__project_status` returns the project name, file, revision and dirty
state. `timeline_overview` and `media_list` are reads: they never mutate.

Edits are live in the app as soon as the tool succeeds, but they are unsaved
until `save` runs. `dirty: false` after an edit means something is wrong; report
it instead of claiming success.

## Save and history

- Call `mcp__or__save` once after finishing the requested change, unless the user
  asked to review first.
- `mcp__or__undo` and `mcp__or__redo` are the canonical history. They undo one
  project edit, not one conversation turn; tell the user how many steps you
  undid.
- Never call any project, session or recovery shutdown. Closing the app is the
  user's action.

## Refused operations

A refused command is a real answer, not a transient error:

- A time outside the sequence or a trim that inverts a clip: re-read the
  timeline and choose a valid time.
- A revision conflict: re-read `project_status`, then repeat the edit once.
- A duplicate id or a missing media id: re-read ids and retry with real values.

Do not loop on the same failing call. After two identical failures, stop and
report the tool, its error and the next concrete step.

## Reporting

Say what changed: track, clip, exact time. Say what was saved. Do not claim a
change that no tool confirmed; a timeline read after the edit is the minimum
evidence.
