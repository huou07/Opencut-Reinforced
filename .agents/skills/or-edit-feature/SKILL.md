---
name: or-edit-feature
description: Add or fix ordinary OR editing behavior—projects, media-library commands, timeline, clips, undo, save and recovery—through the shared Rust-owned command path. Do NOT use for UI review (or-ui-ux), media decoding/render/export pipeline work (or-media-render), deterministic MotionScene (or-motionscene), or local build and verification mechanics (or-dev-loop).
---

# OR editing feature

Keep an edit in the existing canonical project model. This skill supplies OR
state-ownership constraints; the general implementation process is in
[DEVELOPMENT_WORKFLOW.md](../../../docs/DEVELOPMENT_WORKFLOW.md).

## Read for the change

Read only the owning PRODUCT, ARCHITECTURE, TECHNICAL_PLAN and PROJECT_FORMAT
sections, plus existing tests and UI references when applicable. Use CodeGraph
before exploring indexed source.

## Invariants

- Rust and the shared application command path own canonical project state.
  Flutter, CLI and agents must not create another editable project.
- Mutations validate project identity and expected revision. Preserve undo,
  explicit save, recovery and exact-base conflict behavior.
- Queries, playback, decode and presentation do not mutate project state.
- Keep time exact in the domain. Refresh UI from canonical query results after
  edits; do not treat a presentation-only preview as committed state.
- Motion content remains declarative and non-executable.
- The existing core-dependency boundary is enforced by
  scripts/check_or_core_deps.py; a failure needs an architecture decision.

## Procedure

1. State the user-visible operation and the project state that proves it worked.
2. Extend an existing command/query and caller where possible; identify affected
   undo, save/reopen and recovery paths.
3. Add a focused regression guard for the domain behavior.
4. Route visible interaction review to or-ui-ux, media pipeline changes to
   or-media-render, MotionScene work to or-motionscene, and local execution to
   or-dev-loop.
5. Update the owning product or architecture document only when its contract or
   status changes.

Do not broaden platform parity work unless the owner selects it. During the
current mission, make and verify local changes on macOS and commit coherent work
locally; do not push or trigger remote CI.
