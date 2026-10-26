# OR Skill Routing

Last reviewed: 2026-10-25

OR-specific skills are no longer project files. They ship as the local DeepSeek
Harness plugin [tooling/dsh-or-skills](../tooling/dsh-or-skills/README.md), which
registers a `SkillProvider` over its `skills/` directory, so any session in this
repository receives them as `dsh-or-skills` skills. Edit a skill in
`tooling/dsh-or-skills/skills/<name>/SKILL.md`, never in a project-local
`.agents/skills` copy. After editing, refresh the installed copies:

```sh
dsh plugin --profile desktop remove dsh-or-skills
dsh plugin --profile desktop add "file:$PWD/tooling/dsh-or-skills"
```

The plugin resolves at rank 20, ahead of filesystem roots, so an OR skill name
comes from `tooling/dsh-or-skills/skills/` even if a stale copy exists elsewhere.
Changes reach the harness after an application restart, because bundles are
composed at boot.

These skills contain only OR-specific contracts and paths. They defer general
planning, coding, UI review and persistence practice to the session skills the
host exposes (superpowers, `ui-ux-pro-max`, and the rest). Load a skill only when
its trigger matches.

## Active development routes

| Skill | Use for |
|---|---|
| or-edit-feature | Canonical editing behavior: timeline, media/project commands, clips, undo, save/reopen and recovery. |
| or-ui-ux | Any user-visible layout, information architecture, pointer/keyboard interaction, accessibility, visual review or UI performance work. |
| or-media-render | Media import, decoding/playback, audio, compositing, FFmpeg, export and media performance. |
| or-motionscene | Deterministic code-to-video scenes: source parsing/highlighting, scene authoring, preview, rendering and insertion into an editing project. |
| or-dev-loop | Local builds, focused checks, real macOS app runs, semantic UI driving and evidence classification for this mission. |

Editing remains the entry point for ordinary user operations. Add or-ui-ux when
the surface changes; add or-media-render when the media pipeline changes.
MotionScene uses or-motionscene and, when its screen changes, or-ui-ux. It does
not route through an AI/provider workflow.

## Future-only routes

| Skill | Use for |
|---|---|
| or-claim-evidence | Record or audit an exact-SHA hosted claim when that work is explicitly in scope. |
| or-release | Package, publish or distribute OR when explicitly authorized. |

Claim and release procedures are preserved for future work. They are not part
of the current local Mac mission. Do not load them for ordinary implementation.

The former or-media-ai route is retired from active development: its combined
provider, DSP and MotionScene scope was misleading. Provider/model work is
deferred. If it becomes authorized later, follow ADR 0005, ADR 0007 and
SECURITY_LICENSING.md; deterministic MotionScene rendering stays provider-free.

## Shared procedures

Use the general skills exposed in the current session for general coding,
documentation, investigation or UI review. Do not copy their generic procedures
into this repository. If a shared skill is unavailable, follow the owning OR
document linked by [docs/INDEX.md](INDEX.md).

Framework artifacts are written in English; conversation follows the owner's
language.
