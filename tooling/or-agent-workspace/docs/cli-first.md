# CLI-first operation

Scope: which surface answers a read or a change, in what order, and what to do
when a call fails. Layout is [workspace.md](workspace.md); timeline mechanics are
[timeline-editing.md](timeline-editing.md).

## Default surface

`tools/or` is the default for every project read and mutation. It goes through the
same validated command path as the editor, so it cannot write a state the app
would reject.

| Need | Surface |
|---|---|
| Project state, tracks, clips, markers, media | `tools/or project summary`, `session describe`, `timeline ...`, `media list` |
| Any timeline or library mutation | `tools/or ... --attach project.json` |
| Render a scene to a file | `tools/or motion materialize` (no project change) |
| Render, import and insert a scene | `tools/or motion publish` |
| Motion asset library | `tools/or motion asset ...` |
| Transcription of existing audio | `tools/or captions transcribe` |
| Read scene composition | `tools/or motion layout --file --at` |
| Offline narration, music, scaffold | `tools/voiceover.mjs`, `tools/music.mjs`, `tools/motion-scaffold.mjs` |
| A verb the CLI does not have | `mcp__or__*`, then report which surface ran |

`tools/or commands` and `tools/or queries` print the exact surface; trust them
over a remembered verb list.

## Order of work

1. Confirm the pointer: `tools/or project summary --attach project.json --json`.
2. Read the smallest state the task needs (tracks, clips, media, scene layout).
3. Apply one mutation and read the result.
4. Save once at the end with `project save`, unless the user asked to review.

Do not write blind: every mutation needs real ids and times from a read in this
session. Do not chain mutations whose inputs depend on the previous result in one
parallel burst.

## Parallelism

Reads and file-only renders can run together. Project mutations are serial because
each one advances the project revision; two concurrent mutations race and one is
refused. Run renders in parallel, then import and insert sequentially.

## MCP fallback

Use `mcp__or__*` only when one of these is true:

- `tools/or` is missing or cannot attach because the workspace has no
  `project.json`.
- The verb is absent from `tools/or commands` (older app build).
- The CLI returns an error the guides do not explain, and the MCP tool has the
  exact operation.

Do not use MCP for an operation the CLI already performs. When a fallback runs,
say which surface produced the change; the project history should not be a
mystery.

## Failures

A refused command is an answer, not a transient error:

1. Read the error code and message.
2. Check [troubleshooting.md](troubleshooting.md) for the known cause.
3. Fix the input (id, time, track state) and retry once.
4. If it fails again, stop and report the verb, the code and the current state.

Never loop on the same call, never fake an edit with a different verb, and never
claim a change no read confirmed.

## Evidence

After a mutation, read the affected track or clip back. After a save,
`session describe` must report `dirty: false`. Report ids, tracks and exact times,
not just "done".
