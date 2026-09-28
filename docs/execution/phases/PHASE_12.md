# Phase 12 — Dubbing and voice

## Status

Phase 12 is planned. Voice features use the Phase 10 task/provider boundary and
must remain reviewable, permissioned, cancellable, and provenance-rich.

## 12A — Voiceover

Add typed voiceover assets and reviewed placement through normal commands. Keep
source text, speaker/provider/model metadata, timing, rights, and generation
settings explicit. Generated audio is an asset/proposal until accepted.

Affected invariants: `INV-STATE-001`, `INV-STATE-002`, `INV-STATE-003`,
`INV-TIME-001`, `INV-JOB-001`, `INV-AI-001`, `INV-AI-002`, `INV-AI-003`,
`INV-SEC-001`.

## 12B — Subtitle-to-speech

Convert approved typed subtitles into bounded speech jobs with pronunciation,
timing, language, and speaker controls. Return audio assets and alignment
proposals; never silently replace project audio or captions.

Affected invariants: `INV-RT-001`, `INV-RT-002`, `INV-TIME-001`, `INV-JOB-001`,
`INV-AI-001`, `INV-AI-002`, `INV-AI-003`.

## 12C — Translation and dubbing

Compose reviewed translation with subtitle-to-speech through explicit task
boundaries. Preserve source/target lineage, timing policy, model/provider
provenance, rights, and revision checks. Cloud providers remain optional.

Affected invariants: `INV-STATE-002`, `INV-STATE-003`, `INV-TIME-001`,
`INV-JOB-001`, `INV-AI-001`, `INV-AI-002`, `INV-AI-003`, `INV-SEC-001`.

## 12D — Speaker mapping and pronunciation

Add typed speaker mappings, pronunciation dictionaries, language variants, and
reviewable overrides. Keep identities project-local and do not infer consent or
rights from a provider response.

Affected invariants: `INV-STATE-001`, `INV-STATE-002`, `INV-AI-002`,
`INV-AI-003`, `INV-PERSIST-001`, `INV-SEC-001`.

## 12E — Timing controls

Provide exact timing/rate/fit policies, visible drift decisions, and bounded
stretching or re-generation. Preview and export use the same evaluated audio
model and retain a correctness fallback when a provider cannot meet a budget.

Affected invariants: `INV-RT-001`, `INV-RT-002`, `INV-TIME-001`,
`INV-MEDIA-002`, `INV-JOB-001`, `INV-AI-003`.

## 12F — Regeneration and ducking

Add explicit regeneration, versioned asset replacement, review-before-apply,
and typed ducking/mix controls. Voice cloning is prohibited until a separate
consent, identity, rights, and abuse-prevention checkpoint is approved; no
implicit voice cloning is included here.

Affected invariants: `INV-STATE-001`, `INV-STATE-002`, `INV-STATE-003`,
`INV-RT-001`, `INV-RT-002`, `INV-JOB-001`, `INV-AI-001`, `INV-AI-002`,
`INV-AI-003`, `INV-SEC-001`, `INV-DEP-001`.

## Stop conditions

Stop for unverified voice rights/consent, hidden identity inference, direct
provider mutation, unbounded generation, missing provenance, or a request for
voice cloning without its own approved consent checkpoint.

## Handoff

Report voice task/provider boundaries, consent and rights evidence, timing and
mix tests, cancellation behavior, docs, commit, and hosted platform status.
