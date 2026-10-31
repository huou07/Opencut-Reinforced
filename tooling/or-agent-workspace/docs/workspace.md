# Workspace and bounds

Scope: what this folder is, which files the app owns, where work goes, and what
an agent must not touch. Project mechanics are in
[project-safety.md](project-safety.md); surface choice is in
[cli-first.md](cli-first.md).

## What this folder is

The app installs this workspace at `<application support>/agent-workspace`
(`~/Library/Application Support/io.github.huou07.orApp/agent-workspace` on macOS)
when the assistant uses the **Agent workspace** mode. `AGENTS.md`, `docs/`,
`.agents/`, `tools/` and `templates/` are framework files: they come from the app
bundle and are replaced when its version changes. `work/` and `scenes/` are yours.

## Layout

| Path | Holds |
|---|---|
| `AGENTS.md` | the routing index; the only rule file |
| `docs/` | one guide per operating concern |
| `.agents/skills/` | task skills the harness can load |
| `tools/` | `or`, the CLI bound to this build, and the offline helper scripts |
| `templates/` | the video pipeline, the long-form story and the style packs |
| `scenes/` | `.ormotion.json` sources you author |
| `work/` | scripts, voiceover, music, captions, renders, notes; never project state |
| `project.json` | app-written pointer to the live project and app folders |
| `or-cli.json` | app-written path to the `or` binary |
| `.or-workspace.json` | installed bundle version and file manifest |

## Runtime files

`project.json` carries `version`, `project_name`, `project_path`,
`descriptor_path`, `app_support_path` and `motion_library_path`.
`tools/or ... --attach project.json` follows `descriptor_path` to the running app,
so the same command keeps working when another project opens. `or-cli.json`
records the executable `tools/or` wraps. `.or-workspace.json` is the installer
manifest.

These files are written by the app on connection. Do not edit, move or delete
them; reconnect the assistant to have them rewritten. `tools/or` is regenerated
on every connection, so never copy it into a script.

## Working folders

- `work/scripts/` — `or-assistant-script` JSON.
- `work/voiceover/` — WAVs, `manifest.json`, `captions.srt`.
- `work/music/` — generated beds.
- `work/captions/` — SRTs from transcription or editing.
- `work/renders/` — materialised scene files before insertion.
- `scenes/` — scene sources, not renders.

Files under `work/` are not project state until imported. Pass absolute paths to
`media add`; do not move or delete a source after importing it.

## Workspace modes

- **Agent workspace** (default) — the installed folder above, with
  `project.json` and `tools/or`.
- **Project folder** — the framework is installed beside the project so file
  tools see the media. It may have no `project.json` or `tools/or`.
- **Choose a folder** — a path the user picks; it gets no runtime files.

A workspace without `project.json` or `tools/or` cannot attach the CLI. Use the
`mcp__or__*` tools for project work and tell the user that switching to **Agent
workspace** in the assistant settings restores the CLI. This is a known gap, not
a broken install.

## Bounds

- Never edit a `.orproj` file, anything inside the project directory, or a path
  named in `project.json`.
- Never read, ask for or store a cloud provider key. Keys live in the macOS
  keychain and belong to the app's Settings.
- App-only: playback speed, reverse, freeze, clip volume and mute, group move,
  effects and transitions, export, microphone recording, cloud voices and the
  speech-model picker. Title and caption styling is not app-only;
  `timeline set-text` sets it.
- Project mutations are one stream at a time. File-only renders may run in
  parallel; see [motion-production.md](motion-production.md).
- Framework files are app-owned and can be replaced. Keep durable notes and
  outputs in `work/`.

## Refresh and drift

A bundle version change refreshes every framework file, restores missing ones and
deletes nothing. If `AGENTS.md` or a guide looks older than this install, compare
`.or-workspace.json` with the app build and reconnect; do not hand-patch a
framework file as a long-term fix.
