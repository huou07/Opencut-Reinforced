# Phase 10 — Captions, transcript, and AI assist

## Status

Phase 10 is planned. AI is an optional task-oriented service boundary. Providers
return structured proposals, analyses, or assets; only validated application
commands mutate the Rust-owned project.

## AI task boundary

The supported task vocabulary is `Transcribe`, `Translate`, `TextToSpeech`,
`Segment`, `DetectScene`, `PlanEdit`, `GenerateImage`, `GenerateVideo`, and
`GenerateAudio`. Task requests carry explicit project/revision/input identity,
permissions, cancellation, and resource budgets. Providers are selected by a
central capability/policy layer and are never called directly from `or_core`,
Flutter widgets, or arbitrary agent code.

Model manifests describe model identity, version/checksum, license, runtime,
hardware requirements, language/capability coverage, and provenance. Stored
credentials remain in OS/user-secure storage; no CLI or agent query returns
plaintext secrets.

## 10A — AI runtime and provider foundation

Define task, provider, model-manifest, job, permission, progress, cancellation,
error, proposal, and asset contracts. Provide local-first selection where
available and optional cloud providers only with user-supplied credentials.
Keep provider adapters outside canonical state and gate exact runtimes/models by
official license, platform, and maintenance evidence.

Affected invariants: `INV-STATE-001`, `INV-STATE-002`, `INV-STATE-003`,
`INV-JOB-001`, `INV-CACHE-001`, `INV-AI-001`, `INV-AI-002`, `INV-AI-003`,
`INV-SEC-001`, `INV-DEP-001`.

## 10B — Local transcription

Add bounded transcription jobs that consume media snapshots or approved audio
artifacts and return timestamped transcript proposals with confidence and
provenance. Automatic captions are first introduced here, not in Desktop MVP.
Workers never write captions directly.

Affected invariants: `INV-RT-001`, `INV-RT-002`, `INV-JOB-001`, `INV-CACHE-001`,
`INV-AI-001`, `INV-AI-002`, `INV-AI-003`, `INV-SEC-001`, `INV-DEP-001`.

## 10C — Caption proposal and apply workflow

Present transcript-to-caption proposals for review, diff, revision/precondition
validation, and explicit apply through normal typed commands. Preserve manual
edits, stable IDs, exact timing, undo/redo, and rejection on stale project
revision.

Affected invariants: `INV-STATE-001`, `INV-STATE-002`, `INV-STATE-003`,
`INV-TIME-001`, `INV-AI-002`, `INV-AI-003`, `INV-PERSIST-001`, `INV-IPC-001`.

## 10D — Transcript editing

Add a typed transcript read model and editing workflow with speaker/segment
metadata, search, correction, timing review, and explicit caption regeneration
proposals. Transcript edits remain distinct from canonical captions until
applied through validated commands.

Affected invariants: `INV-STATE-001`, `INV-STATE-002`, `INV-STATE-003`,
`INV-TIME-001`, `INV-UI-001`, `INV-AI-002`, `INV-AI-003`, `INV-PERSIST-001`.

## 10E — Translation

Use the `Translate` task boundary for reviewed transcript/caption translation.
Preserve source/target language, provider/model provenance, timing policy,
revision preconditions, and explicit apply. Cloud use is optional and
permission-gated.

Affected invariants: `INV-STATE-002`, `INV-STATE-003`, `INV-TIME-001`,
`INV-JOB-001`, `INV-AI-001`, `INV-AI-002`, `INV-AI-003`, `INV-SEC-001`.

## 10F — Scene, silence, and filler analysis

Add `Segment` and `DetectScene` analysis for scene boundaries, silence, and
filler proposals. Results are inspectable structured analyses and never direct
timeline mutations. Any accepted edit becomes a normal `EditPlan` or command
with dry-run/diff and stale revision checks.

Affected invariants: `INV-RT-001`, `INV-RT-002`, `INV-JOB-001`, `INV-CACHE-001`,
`INV-AI-001`, `INV-AI-002`, `INV-AI-003`.

## 10G — EditPlan, dry run, and diff

Define a typed `EditPlan` for agent and AI-assisted multi-step proposals.
Validate permissions, expected revision, object IDs, exact times, and command
compatibility before presenting a dry run and semantic diff. Applying a plan
executes normal validated commands; there is no hidden direct project writer.

Affected invariants: `INV-STATE-001`, `INV-STATE-002`, `INV-STATE-003`,
`INV-TIME-001`, `INV-AI-002`, `INV-AI-003`, `INV-IPC-001`, `INV-SEC-001`.

## 10H — Provider permissions and model manager

Complete provider permissions, model install/remove/update, manifest
verification, checksums, license display, cache disposal, resource budgets,
and failure/fallback behavior. Make local/cloud choice explicit and auditable.
Provider secrets remain inaccessible through project/agent interfaces.

Affected invariants: `INV-JOB-001`, `INV-CACHE-001`, `INV-AI-001`, `INV-AI-002`,
`INV-AI-003`, `INV-SEC-001`, `INV-DEP-001`.

## Stop conditions

Stop for direct provider calls from core/UI, model/license ambiguity, hidden
project mutation, missing revision checks, plaintext credentials, unbounded AI
jobs, or a request for autonomous application without review/permission.

## Handoff

Report task/provider/model contracts, proposal and apply evidence, model rights,
secret handling, cancellation/resource tests, docs, commit, and hosted runtime
status. Automatic captions are not complete until 10C and the relevant caption
conformance checks pass.
