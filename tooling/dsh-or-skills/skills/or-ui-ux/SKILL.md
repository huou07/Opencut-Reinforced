---
name: or-ui-ux
description: Design, change or review an OR user-visible screen or interaction, including information architecture, layout, pointer and keyboard behavior, accessibility, visual consistency and interaction performance. Do NOT use for backend-only editing logic (or-edit-feature), media pipeline design (or-media-render), MotionScene data/rendering with no UI change (or-motionscene), or build mechanics (or-dev-loop).
---

# OR UI/UX

Bind interface work to OR's existing design and acceptance rules. Use the
`ui-ux-pro-max` tools for design advice; do not copy a generic review framework
into this repository.

## Authorities

- `docs/design/design-language.md` — tokens, hard visual rules, shell, motion.
- `docs/design/ux-invariants.md` — behaviour that must not regress, and the
  evidence a UI change needs.
- `docs/features/workspace-ui.md` — where controls live and interaction parity.

## OR-specific rules

- Use the existing Focused Monochrome design and shell slots; integrate into an
  existing slot instead of inventing a region.
- Every pointer action needs a keyboard path; controls expose honest labels,
  roles, states and actions — do not add semantics wrappers to satisfy a test.
- Keep visible project state grounded in Rust query results; hover/ghost states
  are presentation only.
- Preserve contrast, type-size and hit-target thresholds from the invariants.
- The current runtime target is the real macOS app; check compact layouts only
  when the task changes them.
- A new interaction model is an architecture decision: write the information
  architecture down first (panels, zones, visibility, pointer, keyboard), then
  rebuild widgets against it.

## Review

Describe the journey and its state, then inspect the real interaction with
`scripts/or_ui.py` (`tree` plus `scripts/or_tree_outline.py` answers hierarchy,
labels, rects, states and keyboard reachability). Capture a screenshot only when
it answers a visual question. Assert semantics and canonical state at their own
layers. Report uncovered viewports and states as UNVERIFIED.

Update `docs/design/ux-invariants.md` when a lasting guard or acceptance rule
changes.
