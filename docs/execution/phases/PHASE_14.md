# Phase 14 — AI generation

## Status

Phase 14 is planned. Generation builds on the Phase 10 provider/task contract
and returns reviewable assets or proposals before any timeline mutation.
Checkpoint 14G adds optional AI production of declarative MotionScene
proposals; MotionScene rendering remains provider-independent and requires no
AI call.

## 14A — Generated image

Add `GenerateImage` jobs with model manifests, input/provenance/rights, bounded
resources, cancellation, checksums, and reviewable asset results. Generated
images are not inserted into a project without an explicit validated command.

Affected invariants: `INV-JOB-001`, `INV-CACHE-001`, `INV-AI-001`,
`INV-AI-002`, `INV-AI-003`, `INV-SEC-001`, `INV-DEP-001`.

## 14B — Generated video

Add `GenerateVideo` behind the same provider boundary with explicit duration,
resolution, frame-rate, storage, budget, provenance, rights, and cancellation.
Heavy runtimes may run in a sidecar process/service boundary; they do not enter
`or_core` or bypass permissions.

Affected invariants: `INV-RT-001`, `INV-RT-002`, `INV-MEDIA-001`,
`INV-MEDIA-002`, `INV-JOB-001`, `INV-CACHE-001`, `INV-AI-001`, `INV-AI-002`,
`INV-AI-003`, `INV-DEP-001`.

## 14C — Generated music and SFX

Add `GenerateAudio` results for music and sound effects with duration, stems or
mix metadata, rights, provenance, checksum, and review. Generated audio follows
the existing `or_audio` clock and fallback rules.

Affected invariants: `INV-RT-001`, `INV-RT-002`, `INV-JOB-001`, `INV-CACHE-001`,
`INV-AI-001`, `INV-AI-002`, `INV-AI-003`, `INV-SEC-001`.

## 14D — Generated voice

Use the Phase 12 voice/consent policy for generated voice assets. No identity or
voice cloning is implied by a provider capability; consent and rights remain
explicit and reviewable.

Affected invariants: `INV-JOB-001`, `INV-AI-001`, `INV-AI-002`, `INV-AI-003`,
`INV-SEC-001`, `INV-DEP-001`.

## 14E — Provider/model selection and jobs

Complete provider/model selection, capability discovery, progress, cancel,
retry, failure, cost/resource budgets, sidecar isolation, manifests, and
secret boundaries. Keep heavy runtimes optional and disposable.

Affected invariants: `INV-HW-001`, `INV-HW-002`, `INV-JOB-001`, `INV-CACHE-001`,
`INV-AI-001`, `INV-AI-002`, `INV-SEC-001`, `INV-DEP-001`.

## 14F — Provenance and review-before-timeline

Require provenance and asset rights in generated results and provide review,
diff/metadata inspection, and explicit apply. The application path validates
the current project revision before placing an asset on the timeline.

Affected invariants: `INV-STATE-001`, `INV-STATE-002`, `INV-STATE-003`,
`INV-RT-001`, `INV-RT-002`, `INV-AI-002`, `INV-AI-003`, `INV-PERSIST-001`,
`INV-SEC-001`.

## 14G — AI MotionScene generation and review

Add `GenerateMotionScene` through the existing provider and capability
boundary. Local or cloud providers may return a structured MotionScene
proposal with model, source-asset, and rights provenance. OR performs local
deterministic validation, preview, explicit human review, materialization, and
normal media/timeline application through the existing command path.

Provider selection may later choose MLX/Core ML on Apple, a CUDA-capable local
provider on NVIDIA, other local runtimes, or cloud providers, but none is a
direct `or_core` dependency and MotionScene semantics never depend on the
provider. Rendering a valid scene makes no provider or per-frame model call.

Future tests cover provider-independent task contracts, valid and invalid scene
proposals, local/cloud selection, model and provider provenance, stale revision
rejection on apply, no model call during render, explicit review,
materialization, and normal project mutation.

Affected invariants: `INV-STATE-001`, `INV-STATE-002`, `INV-STATE-003`,
`INV-MOTION-001`, `INV-MOTION-002`, `INV-MOTION-004`, `INV-JOB-001`,
`INV-AI-001`, `INV-AI-002`, `INV-AI-003`, `INV-SEC-001`, `INV-DEP-001`.

## Stop conditions

Stop for hidden generation, absent model/license provenance, direct provider
mutation, plaintext credentials, no cancellation/budget, unapproved voice
identity behavior, or a design that makes MotionScene validation/rendering
depend on AI.

## Handoff

Report generated asset/task contracts, sidecar decision, provider/model
manifests, rights/provenance, MotionScene proposal/validation/review/
materialization evidence when applicable, review/apply tests, docs, commit, and
hosted runtime evidence.
