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
| Clip transforms, effects, audio, speed, canvas, group moves | `tools/or timeline set-clip-visual\|set-clip-audio\|set-clip-playback`, `set-canvas`, `move-clips` |
| Render the sequence to a file | `tools/or export start\|status\|cancel` |
| Subtitle sidecar | `tools/or captions export --out FILE.srt\|.vtt` |
| One media item's metadata | `tools/or media get --id MEDIA_ID` |
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

The MCP server mirrors the CLI: first-class tools for the common edits and motion
work, plus `run_cli`, which runs any `or` verb with the same attach and JSON
flags. Prefer `tools/or` while it works — one process instead of a tool
round-trip — and use `mcp__or__*` only when one of these is true:

- `tools/or` is missing or cannot attach because the workspace has no
  `project.json`.
- The verb is absent from `tools/or commands` (older app build); `run_cli` still
  reaches it if the app accepts it.
- The CLI returns an error the guides do not explain, and an MCP tool names the
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
