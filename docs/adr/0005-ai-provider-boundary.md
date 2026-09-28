# ADR 0005 — AI provider boundary

- Status: Accepted
- Date: 2026-09-28

## Context

AI features may use local or user-selected cloud providers, but project state,
secrets, model rights, and reproducible edits must remain under application
control.

## Decision

Expose task-oriented, provider-independent operations such as `Transcribe`,
`Translate`, `TextToSpeech`, `Segment`, `DetectScene`, `PlanEdit`,
`GenerateImage`, `GenerateVideo`, and `GenerateAudio`. Providers return
structured analyses, proposals, or assets with model manifests and provenance.
Only normal validated commands apply accepted results, with revision
preconditions and explicit permissions. Credentials remain in secure storage and
are never exposed through project, CLI, or agent interfaces.

## Alternatives rejected

- Direct provider calls from `or_core`, widgets, or arbitrary agent scripts.
- Letting an AI worker mutate `ProjectDocument`.
- Treating a provider response as trusted executable edit instructions.

## Consequences

AI work needs job identity, cancellation, resource bounds, model/license
metadata, review/diff flows, and stale-result rejection. Local-first and cloud
providers can coexist without changing the project model.
