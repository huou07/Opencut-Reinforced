---
name: or-timeline-edit
description: Make one timeline change in the open OR project through the mcp__or__ tools — add, insert, move, trim, split, delete, re-time or mark clips, tracks and the sequence. Use when the user asks for an edit to the timeline. Do NOT use for media import (read docs/media-and-import.md), MotionScene authoring (or-motion-scene), or developing OR itself.
---

# Timeline edit

Workspace guide: `docs/timeline-editing.md`.

## Procedure

1. `mcp__or__project_status` — confirm the project and read the revision.
2. `mcp__or__timeline_overview` — read rates, tracks, clips and markers. Use only
   ids and times from this result.
3. Choose the smallest command. If the target track does not exist, create it
   with `add_track`; text and caption clips need a text or caption track.
4. Call the tool with exact `NUM/DEN` or decimal times.
5. `timeline_overview` again when placement matters, then `mcp__or__save` unless
   the user asked to review first.

## Rules

- One requested change per turn; do not bundle unrequested edits.
- Never invent ids or times from the prompt. The panel context names the current
  clip and playhead only when they exist.
- `insert` moves later clips right; `overwrite` clears the range. Say which one
  you used.
- A refused edit is information: re-read, retry once with valid values, then
  report.

## Report

Name the track, the clip and the exact time that changed, and whether the project
was saved.
