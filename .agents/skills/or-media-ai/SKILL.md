---
name: or-media-ai
description: Add an AI, media or audio-processing capability to OR inside the provider and declarative-motion boundaries — speech-to-text captions, noise reduction and audio cleanup, and code-to-illustration video built from a declarative MotionScene. Use when work touches AI providers, model assets, transcription, audio DSP, or generated/illustrative motion. Do NOT use for ordinary editing behavior with no AI or model component (or-edit-feature), for lane mechanics (or-dev-loop), or for a credential, dependency or licence review (security-review).
---

# OR AI and media capability

You add a capability that may call a provider or a model, without letting it into
canonical project state or making rendering depend on a service.

## Preconditions

- Read [ADR 0005](../../../docs/adr/0005-ai-provider-boundary.md) (provider boundary) and
  [ADR 0007](../../../docs/adr/0007-declarative-motion-scenes-and-procedural-isolation.md)
  (declarative MotionScene and procedural isolation) before designing anything.
- Read the owning [PRODUCT.md](../../../docs/PRODUCT.md) section and
  [SECURITY_LICENSING.md](../../../docs/SECURITY_LICENSING.md). Model weights carry their
  own licences, independently of the runtime library licence.
- The capability must be selected by
  [PRODUCT_ROADMAP.md](../../../docs/PRODUCT_ROADMAP.md).

## Boundaries that are not negotiable

- **AI output is a proposal, never an instruction.** Recognition, translation,
  analysis and generation produce an editable result that a human or agent
  applies through the normal revision-checked command path. If the project
  changed while the proposal was produced, reject, re-query and revalidate.
- **Rendering never calls a provider.** MotionScene validation, inspection,
  preview, rendering and materialisation make zero provider/LLM calls, and no
  per-frame planner or critic loop.
- **Canonical motion is declarative and non-executable.** An `.ormotion.json`
  MotionScene is versioned, seekable and inspectable. No JavaScript, shell,
  network, arbitrary expressions or provider calls belong in canonical scene
  data. Arbitrary web code belongs only to the future isolated WebMotion sidecar,
  which never runs in `or_core`, Flutter or normal project open.
- **Credentials stay out of reach.** Provider credentials live in OS secure
  storage. The app may report whether a provider is configured; the CLI, agents
  and logs never receive plaintext secrets. Offline Mode and network permissions
  are enforced below the UI controls so no client can bypass them.
- **Audio DSP belongs in the audio layer.** Gain, pan, fades and later noise
  reduction or voice isolation belong in `or_audio`/`or_media`, never in Flutter
  widgets, and never on the audio callback's realtime path with unbounded work.
- **Do not bundle models by default.** A model download requires explicit
  confirmation, reports progress, and can be cancelled.

## Code-to-illustration video

The owner's direction (2026-10-18) has two tracks, in this order:

- **Track A — canonical and primary.** Turn a code artifact into a declarative
  MotionScene (highlighted code, callouts, arrows, captions, narration) and render
  it through the normal deterministic path. Optional AI supplies the *scene*, not
  the pixels. This is the track to design and implement first, and it needs no
  model download to render.
- **Track B — optional and later.** An external generative-video provider may
  produce illustrative footage under the ADR 0005 boundary. It is a later,
  costed, licence-reviewed addition; results arrive as reviewable imported media,
  never as canonical scene data.

## Procedure

1. Classify the work: recognition, analysis, generation, DSP, or rendering.
2. Decide whether the capability must work with no provider at all. If it cannot,
   say why in the roadmap entry.
3. Define the proposal or asset contract, its provenance, and its apply path
   through a validated command with a revision precondition.
4. Implement provider access behind the ADR 0005 boundary; keep provider-specific
   code out of `or_core` and out of the render path.
5. Add tests for the boundary itself: no provider call in render, proposal
   rejection on a stale revision, credential non-exposure, cancellation and
   bounded resource use.
6. Verify with skill `or-dev-loop`, then inspect the hosted jobs. A local pass is
   developer evidence only.
7. Update the owning source-of-truth document and the roadmap with the exact
   source, evidence class and licence note.

## Stop conditions

- A capability that cannot respect the provider boundary, the revision check or
  the licence constraints is reported as `BLOCKED` with the exact conflict, not
  implemented around them.
- An unmeasured quality or cost claim stays `UNVERIFIED` with the command that
  would settle it.

## Report

Capability, provider boundary respected, proposal/asset contract, provenance and
licence position, the apply path, tests, exact commands and results, commit,
evidence class, and what remains unverified.
