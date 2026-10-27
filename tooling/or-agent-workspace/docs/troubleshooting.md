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

## A motion MCP tool is not available

`mcp__or__motion_insert_scene` and the motion asset tools come from the app's MCP
server. If the session does not expose them, the CLI can still validate and
render a scene file to disk, but it cannot import or insert the result. Tell the
user that insertion needs the app's assistant connection, and stop there.

## Materialisation is slow

`motion_insert_scene` encodes every frame with the app's encoder. A short scene
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

## A command is refused

See [project-safety.md](project-safety.md). Re-read state, retry once with real
ids and times, then report the refusal if it repeats.
