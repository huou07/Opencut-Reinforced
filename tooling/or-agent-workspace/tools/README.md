# Workspace tools

Scope: the local commands this workspace exposes. `tools/or` is the default way
to read and change the open project; `mcp__or__*` tools are the fallback for
motion insertion, the motion library, and a CLI failure. Some operations belong to
the app alone — speed, volume, group move, effects, export, recording, cloud
voices — and have no verb here on purpose. Title and caption styling does have a
verb: `timeline set-text`.

| Command | Use |
|---|---|
| `tools/or <verb> --attach project.json --json` | The OR CLI bound to the running app build. Reads and edits the live project through the app-written pointer. `tools/or commands` and `tools/or queries` list the verbs. |
| `tools/or media clean-audio --source IN --out OUT` | Offline noise reduction; writes a new file that you import to use. |
| `tools/or captions transcribe` | Transcribe one media item locally (Vietnamese/English auto-detected); needs the running app. |
| `tools/or timeline import-captions` | Apply a reviewed SRT/WebVTT file as a new caption track. |
| `tools/or motion ...` | Validate, inspect, evaluate, layout, render, from-code, materialize or publish a scene. `publish` saves it to the library, renders, imports **and** inserts in one call. |
| `tools/or motion asset ...` | List, validate, save, instantiate, import, remove or insert from the app's motion library; pass `--attach project.json` and the CLI resolves the folder from the pointer. |
| `node tools/motion-scaffold.mjs` | Write a valid V1 `.ormotion.json` skeleton into `scenes/`. |
| `node tools/voiceover.mjs` | Speak an `or-assistant-script` with the macOS voice, write exact timings and SRT, optionally import and place the result. |
| `node tools/music.mjs` | Synthesise a quiet, royalty-free background music bed offline, from a prompt or explicit flags. |

The app keeps its motion library at `<application support>/motion-library`, and the
workspace `project.json` records that path, so every motion verb resolves it from
`--attach project.json`. `--library PATH` overrides it for one call.

## Examples

```sh
tools/or project summary --attach project.json --json
tools/or timeline tracks --attach project.json --json
tools/or timeline clips --track TRACK_ID --attach project.json --json
tools/or timeline markers --attach project.json --json
tools/or media list --attach project.json --json
tools/or media clean-audio --source /absolute/in.wav --out /absolute/out.wav --json
tools/or project save --attach project.json --json
tools/or motion validate --file scenes/title.ormotion.json --json
tools/or motion inspect --file scenes/title.ormotion.json --json
tools/or motion evaluate --file scenes/title.ormotion.json --at 1/2 --json
tools/or motion layout --file scenes/title.ormotion.json --at 1/2 --json
tools/or motion from-code --source work/sample.rs --output scenes/sample.ormotion.json
tools/or motion render --file scenes/title.ormotion.json --at 1/2 --output work/title.png
tools/or motion publish --attach project.json --scene scenes/title.ormotion.json --json
tools/or motion asset list --attach project.json --json
tools/or motion asset insert --attach project.json --asset-id ASSET_ID --json
node tools/voiceover.mjs --script work/scripts/demo.json --plan --json
node tools/voiceover.mjs --script work/scripts/demo.json --out work/voiceover/demo --apply --attach project.json --json
node tools/music.mjs --out work/music/bed.wav --prompt "nhạc nền nhẹ nhàng 30 giây, không trống" --json
tools/or captions transcribe --attach project.json --media MEDIA_ID --srt-out work/captions/clip.srt --json
tools/or timeline import-captions --attach project.json --file work/captions/clip.srt --json
```

`motion publish` needs the running app: it validates the scene, saves it as a
reusable library asset, renders it, imports the video and inserts an ordinary clip
through validated commands. Pass `--no-library` to skip the asset save on purpose.
`motion materialize` only writes a file; use it when a file is the goal, not as an
insertion path. The MCP motion tools are a fallback for a CLI failure, not the
default.

## Bounds

- `tools/or` is written by the app on connect; editing it is pointless.
- Scaffold and CLI write only the files you name; nothing else is project state.
- No workspace tool reads a cloud key. Narration keys live in the macOS keychain
  and belong to the app's Settings → Narration panel.
