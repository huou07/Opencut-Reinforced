# Feature — Timeline Editing

Last reviewed: 2026-11-02

Scope: the editing core — tracks, clips, exact-time operations, selection,
snapping, undo, and the interaction surface that drives them.

## Model

One Rust-owned timeline with closed track kinds (Video, Audio, Text, Caption)
and closed clip content (Media, Text, Caption). Stable clip IDs, exact rational
start/duration, and lossless media source ranges are canonical. Text and caption
clips have no fake media IDs or ranges. Selection, viewport and drag previews are
presentation-only.

## Commands

Every operation below is a validated command on the shared path and produces one
`ChangeSet`, one history entry and one revision increment.

| Area | Commands |
|---|---|
| Tracks | add video/audio track, remove empty track, track enabled/locked/solo |
| Clips | insert, move (same or cross track), trim edge, split, ripple delete, delete, duplicate |
| Grouping | multi-clip group move as one command (one undo) |
| Time | speed/playback rate (1/16×–16×), reverse, freeze frame |
| Audio | clip volume and mute |
| Markers | add, move, rename, delete; marker-aware snapping |
| History | undo, redo, transactional groups |

Every row is reachable from the `or` CLI (`timeline ...`, `media ...`), including
the group move (`timeline move-clips`), playback rate, reverse and freeze
(`timeline set-clip-playback`), and the sequence canvas (`timeline set-canvas`),
so an agent edit and an inspector edit share one validated path.

Time is exact and half-open. A split takes an exact interior point and Rust
generates the right-clip ID. Ripple delete shifts only later clips on the same
track. Trim uses an absolute timeline edge. Freeze is one boolean on media
content that keeps the real `source_range`, so unfreezing restores footage and
trim/split behave normally; preview maps every offset to `source_start`.

## Interaction contract

- Pointer: click/select, drag to move, drag clip edges to trim, marquee select and
  marquee drag, ruler scrub, playhead drag, clip context menu (including advanced
  trim/slip/roll/move dialogs). Right-click selects the clip it acts on, so the
  menu and the inspector describe the clip that was pointed at.
- Keyboard: shortcuts resolve through one shell `Shortcuts`/`Actions` map and
  never fire while a text field holds focus. Blade tool toggles with C/V.
- Every gesture dispatches one command; no optimistic edit geometry is applied
  from Flutter.
- Snapping uses the canonical marker-aware Snap query with revision and
  project-instance guards; a snap shows visible ruler feedback before dispatch.
- Pages are bounded (at most 100 clips per track page) with explicit "load more".

## Deferred

Keyframes and graph editor, nested/compound clips, linked-clip semantics,
speed ramps, multicam, adjustment layers. Do not add these without an approved
scope and typed migration rules.

## Where

- `crates/or_core/src/` — timeline domain, commands, history
- `crates/or_app_bridge/src/api/` — typed bridge DTOs
- `apps/or_app/lib/` — timeline widgets, inspector, palette, menus

## Related

- [workspace-ui.md](workspace-ui.md) — where these controls live
- [preview-playback.md](preview-playback.md) — playback of the result
- [architecture/control-plane.md](../architecture/control-plane.md)
