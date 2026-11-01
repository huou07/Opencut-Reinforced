# OR Agent Workspace

Working folder for the Opencut Reinforced assistant. The open project is recorded
in `project.json`; `tools/or` is the default way to read or change it. `mcp__or__*`
is the fallback when the CLI cannot act. This file is an index, not a handbook.

## Rules

1. Choose what to read from this index by the needs of the task; do not read
   material the task does not need.
2. When adding or changing routing in this file or in `docs/`, keep exactly this
   index template: a short name and function, when to use it, and the exact file
   path.
3. Do not add any other rule to this file. Operational rules live in `docs/`.

## Routing

### Guides

- **Workspace and bounds** — layout, runtime files, working folders, workspace modes, app-only operations.
  When: first task here, or before touching a runtime file, the project folder or a credential.
  Path: `docs/workspace.md`
- **CLI-first operation** — read and mutation order, the CLI/MCP split, failure policy.
  When: choosing how to read or change the project.
  Path: `docs/cli-first.md`
- **Timeline editing** — tracks, clips, exact times, trims, splits, markers, canvas, clip settings, frame rate.
  When: changing anything on the timeline.
  Path: `docs/timeline-editing.md`
- **Media and import** — probe, import, relink, remove, paths, supported formats.
  When: bringing a file in, or finding a media id.
  Path: `docs/media-and-import.md`
- **Motion scenes** — scene format, validation, render, publish, motion assets.
  When: authoring or rendering one deterministic scene.
  Path: `docs/motion-scenes.md`
- **Motion production** — splitting scenes into parallel streams and background render jobs.
  When: producing several scenes or a multi-scene sequence.
  Path: `docs/motion-production.md`
- **Captions** — cue sources, timing, styling, review, import and subtitle export.
  When: adding, editing, restyling or delivering subtitles.
  Path: `docs/captions.md`
- **Export and delivery** — render the sequence, job status and cancel, subtitle sidecars.
  When: producing a finished file, or checking an export job.
  Path: `docs/export-and-delivery.md`
- **Script and voice** — script schema, offline narration, voice selection, music bed.
  When: writing narration or generating a voiceover.
  Path: `docs/script-and-voice.md`
- **Audio prep** — recording, noise cleanup, speech models, transcription.
  When: the user supplies audio, or asks for cleanup or transcription.
  Path: `docs/audio-prep.md`
- **Project safety** — live revision, save, undo, refused operations.
  When: mutating a project, or recovering from a refusal.
  Path: `docs/project-safety.md`
- **Troubleshooting** — known failures and the next concrete step.
  When: a command fails, or a surface is missing.
  Path: `docs/troubleshooting.md`
- **Style packs** — reusable method packs for motion, story, script, voice, edit and captions.
  When: starting a video in a proven style, or promoting a finished video's method.
  Path: `docs/style-packs.md`
- **Docs maintenance** — the routing template and rules for this file and `docs/`.
  When: adding, moving, renaming or reviewing a guide.
  Path: `docs/docs-maintenance.md`

### Skills

- **`or-timeline-edit`** — timeline and project edit procedure with checks.
  When: performing a timeline edit end to end.
  Path: `.agents/skills/or-timeline-edit/SKILL.md`
- **`or-motion-scene`** — scene authoring, validation, render and insertion procedure.
  When: building or publishing a MotionScene.
  Path: `.agents/skills/or-motion-scene/SKILL.md`
- **`or-script-production`** — script, narration, captions and music procedure.
  When: producing a narrated video from a script.
  Path: `.agents/skills/or-script-production/SKILL.md`
- **`or-video-production`** — multi-scene production with parallel render streams.
  When: producing a full video from several scenes.
  Path: `.agents/skills/or-video-production/SKILL.md`
- **`or-project-safety`** — mutation, save and history procedure.
  When: any change to a live project.
  Path: `.agents/skills/or-project-safety/SKILL.md`

### Tools

- **OR CLI** — the attached `or` binary; every project read and mutation.
  When: any project work.
  Path: `tools/or`
- **Voiceover** — script to WAVs, manifest and SRT; optional import and insert.
  When: narrating a script offline.
  Path: `tools/voiceover.mjs`
- **Music** — prompt or flags to a local music bed.
  When: generating background music.
  Path: `tools/music.mjs`
- **Motion scaffold** — starter `.ormotion.json`.
  When: starting a scene from scratch.
  Path: `tools/motion-scaffold.mjs`

### Templates

- **Video pipeline** — beat-driven short-video machinery: build, render, assemble.
  When: producing a one-minute style video.
  Path: `templates/video-pipeline/README.md`
- **Cinematic story** — long-form documentary machinery and style.
  When: producing a multi-minute narrated story.
  Path: `templates/cinematic-story/README.md`
- **Style packs** — portable method packs and the promotion contract.
  When: reusing a proven style, or packaging a new one.
  Path: `templates/styles/README.md`
