---
name: or-motionscene
description: Build or fix OR's deterministic code-to-video MotionScene authoring, preview, rendering, materialisation or insertion into an ordinary timeline. Use for declarative scene work; add or-ui-ux when its visible interface changes. Do NOT use for AI/provider workflows (deferred), ordinary editing commands (or-edit-feature), general media pipelines (or-media-render), or local build mechanics (or-dev-loop).
---

# OR MotionScene

MotionScene is the focused creative feature in the current product scope. Read
[ADR 0007](docs/adr/0007-declarative-motion-scenes-and-procedural-isolation.md)
and the relevant section of [TECHNICAL_PLAN.md](docs/TECHNICAL_PLAN.md)
before changing its contract.

## Boundaries

- A scene is versioned, validated, bounded and declarative. Treat source code as
  untrusted text: parse it; never execute it or embed scripts, expressions,
  network calls or provider instructions in the canonical scene.
- Reuse crates/or_motion, crates/or_render, the existing bridge and FFmpeg
  materialisation path. Do not create a second text or animation engine.
- Preview and materialisation must evaluate the same scene deterministically.
  Rendering does not call an AI provider or network service.
- Insert rendered output through the normal media and timeline commands so it
  behaves like other project media.

## Journey evidence

Use a real small source-code fixture. In the macOS app, create or import the
scene, preview it, render it, insert the result into a normal edit and export
that edit. Inspect the rendered clip and final export independently. Add domain
guards for parsing, bounds, determinism and seek behavior; use or-ui-ux for
visible controls and or-dev-loop for local build/run evidence.

## Session skills

This plugin supplies only the OR-specific contract. Pair it with the session
skills the host exposes: `brainstorming` before changing an interaction model,
`writing-plans` when a change spans several commits, `test-driven-development`
for the regression guard, `systematic-debugging` when behavior is wrong, and
`verification-before-completion` before any done claim. `ui-ux-pro-max`
(`design_recommend` / `design_review` / `design_search`) owns visual and UX
decision advice. Do not re-create those procedures here.
