# Workspace tools

Scope: the local commands this workspace exposes. `tools/or` is the default way
to read and change the open project; `mcp__or__*` tools are the fallback for
motion insertion and the motion library.

| Command | Use |
|---|---|
| `tools/or <verb> --attach project.json --json` | The OR CLI bound to the running app build. Reads and edits the live project through the app-written pointer. `tools/or commands` and `tools/or queries` list the verbs. |
| `node tools/motion-scaffold.mjs` | Write a valid V1 `.ormotion.json` skeleton into `scenes/`. |
| `tools/or motion ...` | Validate, inspect, evaluate, render or write a scene file; none of these inserts a clip. |
| `tools/or motion asset ...` | Inspect a motion library folder the user names. |

## Examples

```sh
tools/or project summary --attach project.json --json
tools/or timeline tracks --attach project.json --json
tools/or timeline clips --track TRACK_ID --attach project.json --json
tools/or media list --attach project.json --json
tools/or project save --attach project.json --json
tools/or motion validate --file scenes/title.ormotion.json --json
tools/or motion inspect --file scenes/title.ormotion.json --json
tools/or motion evaluate --file scenes/title.ormotion.json --at 1/2 --json
tools/or motion from-code --source work/sample.rs --output scenes/sample.ormotion.json
tools/or motion render --file scenes/title.ormotion.json --at 1/2 --output work/title.png
tools/or motion asset list --library work/motion-library --json
```

`motion materialize` needs a live app but only writes the output file. To place a
scene on the timeline, use `mcp__or__motion_insert_scene`; it renders, imports and
inserts in one call.

## Bounds

- `tools/or` is written by the app on connect; editing it is pointless.
- Scaffold and CLI write only the files you name; nothing else is project state.
