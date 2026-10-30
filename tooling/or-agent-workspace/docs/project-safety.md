# Project safety

Scope: how to change a live project without losing work. Rust owns the project,
its revision and its history; every `tools/or` mutation goes through the same
validated command path as the editor.

## The one rule

Never edit a `.orproj` file or anything inside the project directory directly.
The running app holds the authoritative document; a direct edit is invisible to
it and will be overwritten or rejected. Attach through `project.json` and let the
app apply the command.

## Reading state

`tools/or project summary --attach project.json --json` returns the project name,
its ids and the live revision. The save state is a different read:
`tools/or session describe --attach project.json --json` returns `dirty` beside
the same revision, and the human form prints `Dirty: true|false`. `timeline`
queries, `media list` and `media probe` are reads: they never mutate. A rename is
`project rename --name NEW --attach project.json --json`; it is a normal edit and
is saved like any other.

Edits are live in the app as soon as the command succeeds, but they are unsaved
until `project save` runs. `dirty: false` after an edit means something is wrong;
report it instead of claiming success.

## Save and history

- Call `tools/or project save --attach project.json --json` once after finishing
  the requested change, unless the user asked to review first.
- `tools/or history undo --attach project.json --json` and
  `tools/or history redo --attach project.json --json` are the canonical history.
  They undo one project edit, not one conversation turn, so a `--apply` voiceover
  run (one import and one insert per line, plus a caption import) is several
  steps. Tell the user how many steps you undid.
- `tools/or session describe` only reports the live session. Two other verbs are
  not yours to call: `session shutdown` closes the session, and `recovery
  status|apply|discard` works on a project file outside the app. Closing,
  shutting down or recovering is the user's action.

## Refused operations

A refused command is a real answer, not a transient error:

- A time outside the sequence or a trim that inverts a clip: re-read the
  timeline and choose a valid time.
- A locked track: read `timeline tracks`, ask before unlocking it with
  `set-track-state`, then edit.
- A revision conflict: re-read `project summary`, then repeat the edit once.
- A duplicate id, a missing media id or a clip that no longer exists: re-read the
  ids and retry with real values.
- A missing CLI verb (speed, volume, freeze, styling, export): this is the app's
  surface, not a failure. Say so and let the user make that change.

Do not loop on the same failing command. After two identical failures, stop and
report the verb, its error code and the next concrete step.

## Reporting

Say what changed: track, clip, exact time. Say what was saved. Do not claim a
change that no command confirmed; a timeline read after the edit is the minimum
evidence.
