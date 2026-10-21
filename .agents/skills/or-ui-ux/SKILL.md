---
name: or-ui-ux
description: Implement or review an authorized OR visible interaction, screen, navigation or accessibility change using Focused Monochrome. Use for direct manipulation and GUI journey acceptance. Exclude backend-only editing (or-edit-feature), pure performance diagnosis or failing builds (verify-debug), single-value cosmetic tweaks, release bookkeeping and documentation maintenance.
---

# OR interaction acceptance

[DESIGN.md](../../../DESIGN.md) owns Focused Monochrome;
[UX_ACCEPTANCE.md](../../../docs/UX_ACCEPTANCE.md) owns interaction and accessibility
acceptance. Read their affected sections, not every product document.
[DEVELOPMENT_WORKFLOW.md](../../../docs/DEVELOPMENT_WORKFLOW.md) owns implementation.

1. State the user goal, entry point, intended input and target viewport/platform.
   Review-only requests stop at findings; they authorize no edits or runtime gates.
2. Inspect the current controls, semantics and behavior. Keep established shell
   slots, terminology and shared command semantics across desktop and Android.
3. Test discovery, direct move/trim, split at playhead, focus/keyboard alternatives,
   touch targets, feedback, empty/error states and undo where affected. Numeric
   dialogs do not substitute for appropriate direct manipulation.
4. Reuse or_ui semantic actions/gestures and Flutter tests. Pair visible actions
   with Rust observations. A missing/inaccessible control or failed preview is
   FAIL, never a reason to inject state or use an API as UI evidence.
5. Retain relevant screenshots/tree/transcript; inspect visuals separately.
   Source, semantic, behavioral, visual and native evidence remain distinct.
   Measure affected latency via QUALITY, with environment and harness overhead.

Preserve the existing 4.5:1 contrast, 11 px type floor, 24/44 px hit targets,
keyboard access and compact viewport guards. Do not lower requirements or change
the visual identity. Stop at scoped observations/fixes. Report source/build,
journey, input/viewport, assertions, evidence, defects and unrun targets.
Human usability and exact-source hosted acceptance remain separate.
