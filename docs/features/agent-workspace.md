# Feature — Agent Workspace

Last reviewed: 2026-10-27

Scope: the workspace the assistant opens by default, its `AGENTS.md` index,
guides, skills and tools, and how the app installs and binds it to the open
project. The assistant transport itself is [assistant.md](assistant.md).

## Contract

The assistant session starts in an app-installed **agent workspace**, not in the
project folder. The workspace is a normal directory the harness can read and
write, organised like this repository's own routing:

| Path | Holds |
|---|---|
| `AGENTS.md` | the index: one routing entry per guide, skill and tool, with the task trigger and the exact path |
| `docs/` | one guide per operating concern: timeline, media, motion scenes, project safety, script/narration/music, troubleshooting |
| `.agents/skills/` | task skills discovered by the harness skill provider (`or-timeline-edit`, `or-motion-scene`, `or-project-safety`, `or-script-production`) |
| `tools/` | `or`, the app's CLI bound to this build, `motion-scaffold.mjs`, `voiceover.mjs` and `music.mjs` |
| `scenes/` | scene sources the agent authors |
| `work/` | scratch files and imported copies |
| `project.json` | app-written marker: project name, project file, session descriptor |
| `or-cli.json` | app-written marker: the `or` executable used by `tools/or` |

The agent reads `AGENTS.md` automatically as its session instruction file. The
index says which single guide answers a task, so the agent does not preload the
whole workspace. `.agents/skills` is the harness's project skill root; the app
does not copy skills into a user-global directory.

## Installation

- The source tree is `tooling/or-agent-workspace/`. `node build.mjs` packs it into
  the checked-in Flutter asset `apps/or_app/assets/agent_workspace/bundle.json`;
  `node --test` fails when the two drift apart.
- On assistant initialisation the app installs the bundle into
  `<application support>/agent-workspace/`. Bundle paths are validated at parse
  time: absolute paths, `..` segments and platform separators are refused, so an
  asset can never write outside that folder.
- A bundle carries a version. While the version is unchanged, a missing
  framework file is restored and an existing one is left alone, so an agent (or
  the user) may edit the instalment. When the version changes, every framework
  file is refreshed from the bundle; `work/` and any file the bundle does not
  list survive. Framework files are app-owned; notes and outputs belong in
  `work/`.
- `tools/or` is regenerated on every connection, so moving the checkout or
  rebuilding the CLI does not leave a stale path. It is quoted for paths with
  spaces and made executable where the platform supports it.

## Workspace choice

Assistant settings offer three modes:

- **Agent workspace** (default) — the installed folder above. Its whole point is
  the routing index.
- **Project folder** — the open project's directory, for a user who wants the
  agent's file tools to see media beside the project.
- **Choose a folder** — a path the user picks; it is restarted into a new
  conversation.

The choice is a local preference. Settings written before modes existed keep
their chosen folder and otherwise adopt the agent workspace. The workspace does
not follow the open project: opening another project updates `project.json` and
the MCP pointer, not the session folder.

## Project binding

The workspace never holds canonical project state. `project.json` is the marker
the agent names the project by and attaches the CLI through: it carries
`descriptor_path`, and `--attach project.json` follows it to the running app on
every call. The session pointer in the assistant directory is what `or mcp`
reads. Reads and edits go through the `or` CLI by default; the `mcp__or__*` tools
remain for motion insertion, the motion library and a CLI failure, and every
guide repeats the rule that a `.orproj` file or the project directory is never
edited directly. Importing an absolute path is the one file read outside the
workspace, and it is a project command.

## Bounds

- The bundle is compiled into the app; installing it writes only inside
  `<application support>/agent-workspace/`.
- No secret is written into the workspace. `project.json` and `or-cli.json`
  contain paths, not credentials; `or mcp` resolves the descriptor and the
  harness resolves its own credentials.
- The workspace adds no tool surface: the `or` CLI, the agent's `or` MCP server
  and the harness skill catalog are the ones the assistant already had.
- Android has no assistant, so it has no installed workspace.

## Verification

- `cd tooling/or-agent-workspace && node --test test/bundle.test.mjs test/tools.test.mjs`
  — the checked-in bundle matches the source, every guide and skill is routed,
  skill frontmatter matches its directory, the scaffold emits a scene the real
  validator accepts, `music.mjs` writes a valid PCM WAV, `voiceover.mjs --plan`
  is side-effect free, and a macOS `say` run derives exact caption timings.
- `cd apps/or_app && flutter test test/agent_workspace_test.dart
  test/assistant_workspace_test.dart test/assistant_settings_test.dart` — path
  safety, versioned installation, runtime files, default resolution and settings
  migration.
- With a project open, `tools/or project summary --attach project.json --json`
  run from the installed workspace resolves the pointer and reads the live
  revision (`crates/or_cli/tests/semantic_cli.rs` covers the pointer form).
- The real-app journey: open the assistant in a packaged build, confirm the
  session workspace is the installed folder, and confirm the harness reports the
  workspace `AGENTS.md` and the three skills.

## Gaps

- No in-app editor or "reveal in Finder" for the workspace; it is a folder the
  user opens from the filesystem.
- Framework files are replaced on a bundle version bump; user edits to a guide
  do not merge.
- The installed skills are model-invocable only; there is no separate
  user-invocable workspace command list.
- No per-project workspace or per-project library; the workspace is shared by
  every project this app opens.
- The production tools are model-free except for transcription; narration uses
  the system `say` voice and music is synthesised, so quality is bounded by
  those local providers.
- A project-folder or chosen-folder workspace has no `project.json` pointer, so
  the CLI attach guidance only applies to the default agent workspace; those
  sessions fall back to the `mcp__or__*` tools.

## Where

- `tooling/or-agent-workspace/` — source tree, bundle build, production tools and tests
- `tooling/or-agent-workspace/tools/voiceover.mjs` — script narration, timing and captions
- `tooling/or-agent-workspace/tools/music.mjs` — procedural background bed
- `apps/or_app/assets/agent_workspace/bundle.json` — packed bundle
- `apps/or_app/lib/assistant/agent_workspace.dart` — parse, install, runtime files
- `apps/or_app/lib/assistant/assistant_controller.dart` — default and mode resolution
- `apps/or_app/lib/assistant/assistant_settings.dart` — workspace mode preference

## Related

- [assistant.md](assistant.md)
- [motionscene.md](motionscene.md)
- [skills.md](../operations/skills.md)
- [security-boundaries.md](../architecture/security-boundaries.md)
