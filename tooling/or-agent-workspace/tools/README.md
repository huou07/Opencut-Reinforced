# Workspace tools

Scope: the local commands this workspace exposes. They complement the
`mcp__or__*` tools; they never mutate the project.

| Command | Use |
|---|---|
| `tools/or <args>` | The OR CLI bound to the running app build. `motion validate`, `inspect`, `from-code`, `render`, `media probe`. App-generated; if missing, see `docs/troubleshooting.md`. |
| `node tools/motion-scaffold.mjs` | Write a valid V1 `.ormotion.json` skeleton into `scenes/`. |
| `tools/or motion asset ...` | Validate, save, list or instantiate a reusable motion asset against a library folder. |

## Examples

```sh
tools/or motion validate --file scenes/title.ormotion.json --json
tools/or motion inspect --file scenes/title.ormotion.json --json
tools/or motion from-code --source work/sample.rs --output scenes/sample.ormotion.json
tools/or motion render --file scenes/title.ormotion.json --at 1/2 --output work/title.png
tools/or motion asset list --library work/motion-library --json
```

`motion materialize` needs a live app, so prefer `mcp__or__motion_insert_scene`
for insertion. Use the CLI verbs above only to check or preview a scene first.

## Bounds

- `tools/or` is written by the app on connect; editing it is pointless.
- Scaffold and CLI write only the files you name; nothing is project state.
