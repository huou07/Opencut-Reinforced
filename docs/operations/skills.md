# OR Skills

Last reviewed: 2026-10-27

Scope: how OR-specific agent skills are packaged, installed and edited. The skill
catalogue itself is routed from [AGENTS.md](../../AGENTS.md).

## Packaging

OR skills are not project-local files. They ship as the local DeepSeek Harness
plugin [`tooling/dsh-or-skills`](../../tooling/dsh-or-skills/README.md), which
registers a `SkillProvider` over its `skills/` directory at rank 20 — ahead of
filesystem roots — so every session in this repository receives them.

These are **development** skills: they carry how to change OR. The assistant's
installed agent workspace ships a separate, narrower set for **operating** OR
through the `or` CLI, with the `or mcp` tools as the fallback (see
[agent-workspace.md](../features/agent-workspace.md)); do not merge the two
catalogues.

| Skill | Contract it carries |
|---|---|
| `or-edit-feature` | canonical editing: timeline, media/project commands, clips, undo, save/reopen, recovery |
| `or-ui-ux` | layout, information architecture, pointer/keyboard interaction, accessibility, visual review |
| `or-media-render` | import, decode/playback, audio, compositing, FFmpeg, export, media performance |
| `or-motionscene` | deterministic code-to-video scenes: authoring, preview, render, insertion |
| `or-dev-loop` | local builds, focused checks, real macOS runs, semantic UI driving, evidence classes |
| `or-claim-evidence` | exact-SHA hosted claim records (future work only) |
| `or-release` | packaging, publishing, distribution (future work only) |

## Install and refresh

Bundles compose at boot, so changes need an application restart. The install
hard-links files, so an in-place edit does not propagate by itself:

```sh
dsh plugin --profile desktop remove dsh-or-skills
dsh plugin --profile desktop add "file:$PWD/tooling/dsh-or-skills"
```

## Editing a skill

- Edit `tooling/dsh-or-skills/skills/<name>/SKILL.md`; keep the frontmatter `name`
  equal to the directory name.
- Skills contain only OR-specific contracts and paths. General planning, coding
  and UI-review procedure comes from the session's shared skills; do not copy it
  here.
- Keep a skill short: what it owns, the paths it needs, and the checks it
  requires. Link to the owning document instead of restating it.
- After editing, refresh the installed copy and run
  `cd tooling/dsh-or-skills && node --test test/provider.test.mjs`.

## Related

- [docs-maintenance.md](docs-maintenance.md)
- [reference/tooling.md](../reference/tooling.md)
