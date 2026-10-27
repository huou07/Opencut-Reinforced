---
name: or-project-safety
description: Protect the open OR project while editing — check revision and dirty state, save explicitly, undo or redo canonical history, handle refused commands and report honestly. Use before any save, undo or recovery decision, or when a project tool call fails unexpectedly. Do NOT use for designing OR's history or command model.
---

# Project safety

Workspace guide: `docs/project-safety.md`.

## Procedure

1. Before changing anything, read `mcp__or__project_status`.
2. After the requested change, read the affected state (`timeline_overview` or
   `media_list`).
3. Save once with `mcp__or__save` unless the user asked to review first.
4. If the user wants a change undone, use `mcp__or__undo` and say how many edits
   it covered. Never close, shut down or reset the project.

## Rules

- Never write a `.orproj` file or any project path directly.
- A revision conflict or refused edit: re-read, retry once with real values, then
  stop and report.
- `dirty` is evidence. If an edit succeeded but the project is not dirty, report
  the inconsistency instead of claiming success.
- Do not invent tool success. Every claim names the tool and the state you read.
