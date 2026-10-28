---
name: or-project-safety
description: Protect the open OR project while editing — check revision and dirty state, save explicitly, undo or redo canonical history through the workspace or CLI, handle refused commands and report honestly. Use before any save, undo or recovery decision, or when a project command fails unexpectedly. Do NOT use for designing OR's history or command model.
---

# Project safety

Workspace guide: `docs/project-safety.md`.

## Procedure

1. Before changing anything, read `tools/or project summary --attach project.json
   --json` for the revision, and `tools/or session describe --attach project.json
   --json` for `dirty` when the save state matters.
2. After the requested change, read the affected state (`timeline clips` or
   `media list`).
3. Save once with `tools/or project save --attach project.json --json` unless the
   user asked to review first.
4. If the user wants a change undone, use `tools/or history undo --attach
   project.json --json` and say how many edits it covered. A `--apply` voiceover
   run is several edits, not one.
5. Never close, shut down, recover or reset the project: `session shutdown` and
   `recovery status|apply|discard` are the user's actions. `session describe` is a
   read and is fine.

## Rules

- Never write a `.orproj` file or any project path directly.
- A revision conflict or refused command: re-read, retry once with real values,
  then stop and report.
- A locked track is a refusal with a cause: read `timeline tracks`, ask before
  clearing the lock, then edit.
- A missing CLI verb for speed, volume, styling or export is the app's surface,
  not a failure; say so instead of working around it.
- `dirty` is evidence. If an edit succeeded but the project is not dirty, report
  the inconsistency instead of claiming success.
- Do not invent command success. Every claim names the verb and the state you read.
