---
name: or-motionscene
description: Build or fix OR's deterministic code-to-video MotionScene authoring, preview, rendering, materialisation or insertion into an ordinary timeline. Use for declarative scene work; add or-ui-ux when its visible interface changes. Do NOT use for AI/provider workflows (deferred), ordinary editing commands (or-edit-feature), general media pipelines (or-media-render), or local build mechanics (or-dev-loop).
---

# OR MotionScene

Read `docs/features/motionscene.md` and
[ADR 0007](docs/adr/0007-declarative-motion-scenes-and-procedural-isolation.md)
before changing the contract.

## Boundaries

- A scene is versioned, validated, bounded and declarative. Treat source code as
  untrusted text: parse it; never execute it or embed scripts, expressions,
  network calls or provider instructions in the canonical scene.
- Reuse the existing render, bridge and FFmpeg materialisation paths. Do not add
  a second text or animation engine.
- Preview and materialisation evaluate the same scene deterministically.
  Rendering calls no AI provider and no network service.
- Insert rendered output through the normal media and timeline commands so it
  behaves like other project media; a re-render never silently replaces an
  existing `MediaId`.

## Journey evidence

Use a real small source-code fixture: create or import the scene, preview it,
render it, insert it into a normal edit, export that edit. Inspect the rendered
clip and the final export independently. Add domain guards for parsing, bounds,
determinism and seek behaviour; use `or-ui-ux` for visible controls and
`or-dev-loop` for local build/run evidence.
