# Troubleshooting

Scope: the failures the assistant can meet inside this workspace, and the next
concrete step for each. Report the cause, never a guessed success.

## `tools/or` is missing or not executable

The app writes `tools/or` when it connects to the app build. If it is absent, the
assistant cannot reach the project through the CLI. Check `or-cli.json` for the
path the app recorded, and tell the user to reconnect the assistant. Do not write
your own bridge or edit the project files.

## `--attach project.json` fails

The CLI reports that no project is open or that the endpoint is unavailable. The
app has no open project, the project.json pointer is stale, or this session is
not using the agent workspace (a project-folder or chosen-folder workspace has no
pointer to attach through).

- Ask the user to open or create a project in OR, then retry once.
- When this workspace has no `project.json`, use the `mcp__or__*` tools for the
  edit instead, and say which surface you used.

## The CLI has no verb for what was asked

Speed, reverse, freeze, clip volume, mute, text/caption styling, group move,
effects, export, recording and cloud narration are app-only. This is not a bug
and not a reason to fake the edit with a trim: tell the user which surface owns it
(the timeline strip, the inspector, the Edit Title/Caption dialog, the Audio tool,
the Export button), let them do it, then re-read the project.

## `no motion library is recorded by this session`

The CLI looked for `motion_library_path` in the pointer it attached through and
found none. The app writes that path into `project.json`, so the usual cause is a
pointer written by an older app build or a `project.json` from before this
install: reconnect the assistant (or update OR) so the app rewrites the marker,
then retry. Until then, name the library for one call:

```sh
LIB="<application support>/motion-library"   # beside the agent-workspace folder
tools/or motion asset list --library "$LIB" --json
```

`motion asset list`, `validate`, `save`, `instantiate`, `remove` and `import` take
`--attach project.json` and resolve the library from it; `--library` always
overrides. `motion asset insert` needs `--attach`, because it renders through the
live app.

## `publish` refuses a scene

`motion publish` accepts vector scenes only. A scene with image `assets` is
published from the app's Motion panel, which copies the images into the library.
A scene that fails validation reports `MOTION_SCENE_INVALID` with the diagnostic
codes; fix the scene, validate again, then publish once.

Without `--no-library`, `publish` saves the scene to the library the pointer
records and then renders and inserts it. Pass `--no-library` when the library copy
is not wanted, so the result is predictable.

## A motion CLI verb is not available

`tools/or motion publish` and the motion asset `insert` verb need a running OR app
with this workspace's `project.json`. If the verb itself is missing, the app build
is older than the workspace: ask the user to update OR, and use the
`mcp__or__motion_*` fallback only until then. If the app is not connected, the
CLI can still validate a scene and render a file to disk, but nothing can be
imported or inserted; tell the user insertion needs the open app and stop.

## Materialisation is slow

`motion publish` encodes every frame with the app's encoder. A short scene
still takes seconds to minutes. Wait for the tool result; do not start a second
render and do not retry after a timeout without checking whether the clip
appeared. If a render fails, read the returned code, fix the scene, and retry
once.

## Scene validation codes

Common codes and their fix:

| Code | Fix |
|---|---|
| `MOTION_INVALID_SCENE_ID` | `scene_id` must be a fresh UUIDv4. |
| `MOTION_INVALID_DURATION` | `duration` must be greater than zero. |
| `MOTION_MISSING_TEXT` / `MOTION_MISSING_SIZE` | Add the field the node kind requires. |
| `MOTION_KEYFRAMES_UNSORTED` / `MOTION_KEYFRAME_OUT_OF_RANGE` | Sort keyframes and keep them inside the duration. |
| `MOTION_ANIMATION_UNKNOWN_NODE` | The `node_id` must exist in `nodes`. |
| `MOTION_REMOTE_ASSET_REFUSED` | Use a scene-local or project-relative asset path. |

## Transcription fails

- `local transcription needs the running app; pass --attach project.json` — the
  verb was given `--project FILE`. Transcription is a host service; attach the
  live session.
- `TRANSCRIPTION_MODEL_UNAVAILABLE` — no model is installed, or this build has no
  whisper runtime. Send the user to **Audio → Speech model…** (or
  `scripts/install_local_transcription_helpers.sh` for a local debug build) and
  stop. Never download weights from the workspace.
- `captions transcribe` is slow on long media; one job at a time, then review the
  cues before importing them.

## `media clean-audio` or `media probe` reports a helper error

Both run the packaged FFmpeg helper. A plain debug CLI may not find it: say the
helper is missing and point the user at the app, which imports and cleans audio
through the bridge. Do not retry with guessed helper paths.

## A command is refused

See [project-safety.md](project-safety.md). Re-read state, retry once with real
ids and times, then report the refusal if it repeats.
