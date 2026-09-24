# OR UI/UX Acceptance Invariants

## Purpose

Any behavior listed here has previously been verified and must remain working
unless the product requirement is intentionally changed.

Before modifying a related area, inspect this checklist. Before handing off,
re-run affected checks.

## Editor entry

- Home recent project can open Editor.
- Projects screen can open Editor.
- New Project → Create enters Editor.
- Template → Use Template enters Editor.
- Import Media can enter the intended project/editor context.
- Exiting Editor returns to the intended destination.

## Project state

- Switching between projects preserves each project's independent in-memory
  demo timeline.
- Opening a new empty project must not inherit another project's timeline.

## Desktop Editor

- Viewer remains the dominant upper workspace region.
- Left panel and Inspector remain collapsible/resizable.
- Timeline remains vertically resizable.
- Play/Pause occupies a stable fixed position.
- Rapid Play/Pause clicks do not move the hit target.
- Timeline ruler seeking works.
- Timeline playhead dragging works.
- Previous/Next Frame works using project FPS.
- Common editing toolbar commands show consistent icons.
- More opens a visible advanced-command menu.
- More menu commands remain reachable and clickable.
- No menu may be visually clipped by its toolbar/container.

## Mobile Editor

- Preview is visible.
- Transport is visible.
- Timeline is visible.
- Timeline clips are visible.
- Timeline can scroll horizontally.
- Timeline ruler/playhead can be scrubbed by touch/pointer.
- Editor tool dock is visible.
- Opening a tool uses the mobile panel/sheet pattern without removing the
  timeline permanently.
- Closing a tool sheet returns to the same editor/timeline state.

## Feature preservation

- All registered product screens remain reachable.
- All registered Editor panels remain reachable.
- Developer features remain discoverable through Settings → Advanced /
  Developer.
- Simple Mode must not remove major core workflows.

## Themes

- Theme switching does not alter navigation or editor structure.
- Custom theme import accepts valid declarative theme JSON.
- Unsafe/arbitrary CSS/script input is rejected.

## Runtime

- Prototype self-test passes.
- Browser console contains no unexpected errors.
- Browser console contains no unexpected warnings caused by the prototype.

Add future regression guards to this file whenever a real UI/UX regression is
fixed. Keep this a practical invariant checklist, not a changelog or test report.
