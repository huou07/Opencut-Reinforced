---
name: or-ui-ux
description: Design, change or review an OR user-visible screen or interaction, including layout, pointer and keyboard behavior, accessibility, visual consistency and interaction performance. Do NOT use for backend-only editing logic (or-edit-feature), media pipeline design (or-media-render), MotionScene data/rendering with no UI change (or-motionscene), or build mechanics (or-dev-loop).
---

# OR UI/UX

This skill binds interface work to OR's existing design and acceptance rules. Use
the shared UI-review method if the current session exposes it; do not copy a
generic review framework into this repository.

## Authorities

Read [DESIGN.md](../../../DESIGN.md) and the relevant section of
[UX_ACCEPTANCE.md](../../../docs/UX_ACCEPTANCE.md). Read PRODUCT and QUALITY
sections only when they own behavior or a measured target.

## OR-specific rules

- Use the existing Focused Monochrome design and shell slots. Common editing
  actions should be direct and discoverable.
- Pointer actions need a usable keyboard path. Controls expose honest labels,
  roles, states and actions; do not add semantics wrappers just to satisfy a
  test.
- Keep visible project state grounded in Rust query results. A ghost or hover
  state is presentation only.
- Preserve the established contrast, type-size and hit-target thresholds in
  UX_ACCEPTANCE.md.
- The current runtime target is the real macOS app. Check compact layouts only
  when the task changes them or the owner selects mobile work.

## Review

Describe the journey and its state, then inspect the actual app interaction.
Capture a screenshot only when it answers a visual question; inspect it for
clipping, hierarchy, alignment, focus and readability. Assert semantics and
canonical project state at their appropriate layers. Measure interaction or
frame performance when that path changes. Report uncovered viewports and states
as UNVERIFIED.

Use or-dev-loop for local builds and checks. Update UX_ACCEPTANCE.md when a
lasting regression guard or acceptance rule changes.
