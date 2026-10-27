# Troubleshooting

Scope: the failures the assistant can meet inside this workspace, and the next
concrete step for each. Report the cause, never a guessed success.

## No tools are available

If the session has no `mcp__or__*` tools, the `or` MCP server did not attach.
The assistant panel shows a setup error for a missing harness or CLI. Stop
editing and tell the user to reconnect the assistant; do not try to reach the
project another way.

## `project_status` fails

The app has no open project, or the session pointer is stale. Ask the user to
open or create a project in OR, then retry once. `workspace` does not have to be
the project directory; the pointer carries the real project path.

## `tools/or` is missing or not executable

The app writes `tools/or` when it connects to the app build. If it is absent,
run the scene check another way: `mcp__or__motion_insert_scene` validates the
scene during materialisation and returns the diagnostic codes. Do not write your
own renderer.

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

## A tool call is refused

See [project-safety.md](project-safety.md). Re-read state, retry once with real
ids and times, then report the refusal if it repeats.
