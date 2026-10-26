---
name: or-edit-feature
description: Add or fix ordinary OR editing behavior—projects, media-library commands, timeline, clips, undo, save and recovery—through the shared Rust-owned command path. Do NOT use for UI review (or-ui-ux), media decoding/render/export pipeline work (or-media-render), deterministic MotionScene (or-motionscene), or local build and verification mechanics (or-dev-loop).
---

# OR editing feature

Keep an edit in the existing canonical project model. Read the owning document
from [AGENTS.md](AGENTS.md): `docs/features/editing.md`,
`docs/features/project-lifecycle.md` or `docs/architecture/control-plane.md`,
plus the existing tests. Use CodeGraph before exploring indexed source.

## Invariants

- Rust and the shared command path own canonical project state. Flutter, CLI and
  agents never create another editable project.
- Mutations validate project identity and expected revision and preserve undo,
  explicit save, recovery and exact-base conflict behaviour.
- Queries, playback, decode and presentation do not mutate project state.
- Keep time exact in the domain; refresh the UI from canonical query results and
  never treat a presentation preview as committed state.
- `scripts/check_or_core_deps.py` enforces the core dependency boundary; a
  failure needs an architecture decision, not a workaround.

## Procedure

1. State the user-visible operation and the project state that proves it worked.
2. Extend an existing command/query and caller; identify the undo, save/reopen and
   recovery paths it touches.
3. Add a focused regression guard for the domain behaviour.
4. Route visible interaction review to `or-ui-ux`, media pipeline changes to
   `or-media-render`, MotionScene to `or-motionscene`, local execution to
   `or-dev-loop`.
5. Update the owning document only when its contract or status changes.

Pair this with the session's shared skills for general process; this file owns
only the OR-specific contract.
