---
name: or-timeline-edit
description: Make one timeline change in the open OR project through the workspace or CLI — add, insert, move, trim, slip, roll, slide, split, delete, re-time or mark clips, tracks and the sequence. Use when the user asks for an edit to the timeline. Do NOT use for media import (read docs/media-and-import.md), MotionScene authoring (or-motion-scene), or developing OR itself.
---

# Timeline edit

Workspace guide: `docs/timeline-editing.md`.

## Procedure

1. `tools/or project summary --attach project.json --json` — confirm the project
   and read the revision.
2. `tools/or timeline tracks|settings|markers --attach project.json --json` and
   `tools/or timeline clips --track TRACK_ID` — read rates, tracks with their
   state, clips and markers. Use only ids and times from these results. Clips and
   markers page: pass `--offset` while the result reports more.
3. Choose the smallest command. If the target track does not exist, create it with
   `timeline add-track`; text and caption clips need a text or caption track. A
   locked track refuses edits: ask before clearing the lock with
   `timeline set-track-state`.
4. Run it with `--attach project.json --json` and exact `NUM/DEN` or decimal times.
5. Re-read the track when placement matters, then `tools/or project save
   --attach project.json --json` unless the user asked to review first.

## Rules

- One requested change per turn; do not bundle unrequested edits.
- Never invent ids or times from the prompt. The panel context names the current
  clip and playhead only when they exist.
- `--mode insert` moves later clips right; `--mode overwrite` clears the range.
  Say which one you used.
- Slip moves the source window inside a fixed clip, roll moves the cut to its
  neighbour, slide moves the clip and lets the neighbours absorb it. They take a
  signed `--delta`, not an absolute time.
- `--ripple` on `trim-clip` is accepted on the end edge only.
- Speed, reverse, freeze, clip volume, mute, title/caption styling, group move,
  effects and export have no CLI verb. Say the app owns them (timeline strip,
  inspector, dialog, Export) and let the user make that change; never fake it with
  a trim or a split.
- When this workspace has no `project.json` to attach through, use the matching
  `mcp__or__*` tool and say so.
- A refused edit is information: re-read, retry once with valid values, then
  report.

## Report

Name the track, the clip and the exact time that changed, and whether the project
was saved.
