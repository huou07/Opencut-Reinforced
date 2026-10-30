# Feature — Project Lifecycle

Last reviewed: 2026-10-26

Scope: create, open, save, reopen, recover, relink and recent-project behaviour
from the user's point of view, and the invariants that make it safe.

## User journey

New project → choose name/location → edit → explicit save → close → reopen with
the same state. A dirty project is protected on close, switch and exit. A crash
leaves a recovery checkpoint that the user can inspect, apply or discard.

## Contract

- A new project derives a safe `.orproj` filename suggestion from the entered
  title; the native save dialog lets the user choose another location or name.
- Save is explicit and atomic. Autosave writes only the recovery sidecar every
  30 s while dirty and never overwrites the canonical file.
- Opening validates stored media URIs without opening or probing sources.
- A stale exact-base save is rejected as a conflict; the user resolves it.
- Schema migrations run on open; opening a v1 project works and an unknown future
  version is rejected.
- A missing or offline source is a relink problem: `media.relink` re-probes and
  replaces the source under the existing `MediaId`, preserving timeline
  references, and is undoable.
- Recent projects show the saved name; renaming a project updates the recent
  display name without changing its last-opened time.

## Implemented

Create/open/save/recovery/recovery-inspection and conflict handling; recent
projects on desktop and Android SAF rebind; relink for desktop local files and
Android SAF items; folder import.

## Deferred

Pinned/archived project organisation, snapshots and version history, project
consolidation, bulk offline-media relink, workspace presets, and clean-machine
install/upgrade/uninstall verification.

## Where

- `crates/or_core/src/` — document, storage, recovery, session
- `apps/or_app/lib/` — home, project dialogs, dirty-state guards

## Acceptance

A real app run must show the interaction and the resulting canonical state
(project file bytes and revision), not only a widget test or CLI call. See
[operations/verification.md](../operations/verification.md).

## Related

- [architecture/project-format.md](../architecture/project-format.md)
- [architecture/control-plane.md](../architecture/control-plane.md)
