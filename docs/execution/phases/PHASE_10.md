# Phase 10 — Captions, transcript, and AI assist

## Status

Phase 10 is planned. AI is an optional task-oriented service boundary. Providers
return structured proposals, analyses, or assets; only validated application
commands mutate the Rust-owned project.

## AI task boundary

The supported task vocabulary is `Transcribe`, `Translate`, `TextToSpeech`,
`Segment`, `DetectScene`, `PlanEdit`, `GenerateImage`, `GenerateVideo`,
`GenerateMotionScene`, and `GenerateAudio`. `GenerateVideo` returns an opaque
raster/video asset; the future `GenerateMotionScene` task returns an editable,
declarative MotionScene proposal. Neither task is implemented by Phase 10.
Task requests carry explicit project/revision/input identity,
permissions, cancellation, and resource budgets. Providers are selected by a
central capability/policy layer and are never called directly from `or_core`,
Flutter widgets, or arbitrary agent code.

MotionScene generation is optional. Validation, inspection, preview,
rendering, and materialization of a MotionScene must not make a provider call,
and a renderer must never invoke an LLM per frame or require a planner/critic
loop. See [ADR 0007](../../adr/0007-declarative-motion-scenes-and-procedural-isolation.md)
for the future declarative contract.

Model manifests describe model identity, version/checksum, license, runtime,
hardware requirements, language/capability coverage, and provenance. Provider
or model absence is a valid typed `Unavailable` capability state; no checkpoint
requires cloud credentials or an unlicensed model merely to complete the
provider-independent architecture. Stored credentials remain in OS/user-secure
storage; no CLI or agent query returns plaintext secrets.

## 10A — AI runtime and provider foundation

Define the complete provider boundary needed by every later AI checkpoint:
typed `AiTask`, provider manager, jobs, permissions, progress, cancellation,
typed errors, proposals, analyses, assets, and normal command application. The
path is OR application → typed task → Provider Manager → managed local
sidecar, optional cloud adapter, deterministic test provider, or unavailable
capability → proposal/analysis/asset → review → normal Command Executor.
`or_core` must never directly link Whisper, TTS engines, diffusion or
video-generation runtimes, MLX, CUDA, ONNX Runtime, or cloud SDKs. The future
`or_ai/provider` layer owns those boundaries.

Freeze local sidecar protocol V1 as a child process using stdin/stdout with one
bounded UTF-8 JSON object per line. stdout carries protocol only; diagnostics
go to bounded stderr. Use explicit argv and no shell invocation, sanitize the
inherited environment, and give each provider/job a managed scratch directory.
Every message carries a protocol version and stable request/job ID. Support
ready/capabilities, start task, progress, result, typed error, cancel, cancel
acknowledgement, and shutdown. Bound each control message to 1 MiB unless a
smaller existing repository limit is authoritative. Do not send frame-rate
binary media or large base64 blobs through control JSON; use bounded approved
managed file/artifact references for large media, model, and output data. A
local sidecar has no implied network authority. Cloud network access requires
an explicitly permissioned provider. Stored secrets stay outside project
files, CLI output, and agent-readable interfaces.

10A also owns the versioned model-artifact contract needed by 10B: a
`ModelManifest` with model ID/version, task and capabilities, runtime/provider
requirement, artifact source, expected SHA-256, size bound, license/provenance,
hardware requirements, and language/capability metadata. Managed model files
are checksum-verified and live outside `ProjectDocument`. A missing model
returns typed `Unavailable`; do not bundle weights by default. 10H later owns
the user-facing download, pause/resume, verify, update, remove, storage-use,
license-presentation, and permission experience, reusing this exact artifact
contract.

The Phase 10 reference local ASR is whisper.cpp v1.9.4 in a provider sidecar,
never an `or_core` dependency. Do not bundle Whisper model weights. CI uses
deterministic fake providers and protocol fixtures; real sidecar testing is
optional only where a lawfully obtained model fixture is explicitly available.
No cloud API key or real model is a checkpoint prerequisite.

Provide local-first selection where available and optional cloud providers
only with user-supplied credentials. Gate any exact runtime/model used by an
available provider on upstream, license, platform, and maintenance evidence.

Affected invariants: `INV-STATE-001`, `INV-STATE-002`, `INV-STATE-003`,
`INV-JOB-001`, `INV-CACHE-001`, `INV-AI-001`, `INV-AI-002`, `INV-AI-003`,
`INV-AI-004`, `INV-SEC-001`, `INV-DEP-001`.

## 10B — Local transcription

Add bounded transcription jobs that consume media snapshots or approved audio
artifacts and return timestamped transcript proposals with confidence and
provenance. Automatic captions are first introduced here, not in Desktop MVP.
Workers never write captions directly.

Affected invariants: `INV-RT-001`, `INV-RT-002`, `INV-JOB-001`,
`INV-CACHE-001`, `INV-AI-001`, `INV-AI-002`, `INV-AI-003`, `INV-AI-004`,
`INV-SEC-001`, `INV-DEP-001`.

## 10C — Caption proposal and apply workflow

Present transcript-to-caption proposals for review, diff, revision/precondition
validation, and explicit apply through normal typed commands. Preserve manual
edits, stable IDs, exact timing, undo/redo, and rejection on stale project
revision.

Affected invariants: `INV-STATE-001`, `INV-STATE-002`, `INV-STATE-003`,
`INV-TIME-001`, `INV-AI-002`, `INV-AI-003`, `INV-AI-004`, `INV-PERSIST-001`,
`INV-IPC-001`.

## 10D — Transcript editing

Add a typed transcript read model and editing workflow with speaker/segment
metadata, search, correction, timing review, and explicit caption regeneration
proposals. Transcript edits remain distinct from canonical captions until
applied through validated commands.

Affected invariants: `INV-STATE-001`, `INV-STATE-002`, `INV-STATE-003`,
`INV-TIME-001`, `INV-UI-001`, `INV-AI-002`, `INV-AI-003`, `INV-AI-004`,
`INV-PERSIST-001`.

## 10E — Translation

Use the `Translate` task boundary for reviewed transcript/caption translation.
Preserve source/target language, provider/model provenance, timing policy,
revision preconditions, and explicit apply. Cloud use is optional and
permission-gated.

Affected invariants: `INV-STATE-002`, `INV-STATE-003`, `INV-TIME-001`,
`INV-JOB-001`, `INV-AI-001`, `INV-AI-002`, `INV-AI-003`, `INV-AI-004`,
`INV-SEC-001`.

## 10F — Scene, silence, and filler analysis

Add `Segment` and `DetectScene` analysis for scene boundaries, silence, and
filler proposals. Results are inspectable structured analyses and never direct
timeline mutations. Any accepted edit becomes a normal `EditPlan` or command
with dry-run/diff and stale revision checks.

Affected invariants: `INV-RT-001`, `INV-RT-002`, `INV-JOB-001`,
`INV-CACHE-001`, `INV-AI-001`, `INV-AI-002`, `INV-AI-003`, `INV-AI-004`.

## 10G — EditPlan, dry run, and diff

Define a typed `EditPlan` for agent and AI-assisted multi-step proposals.
Validate permissions, expected revision, object IDs, exact times, and command
compatibility before presenting a dry run and semantic diff. Applying a plan
executes normal validated commands; there is no hidden direct project writer.

Affected invariants: `INV-STATE-001`, `INV-STATE-002`, `INV-STATE-003`,
`INV-TIME-001`, `INV-AI-002`, `INV-AI-003`, `INV-AI-004`, `INV-IPC-001`,
`INV-SEC-001`.

## 10H — Provider permissions and model manager

Complete provider permissions, model install/remove/update, manifest
verification, checksums, license display, cache disposal, resource budgets,
and failure/fallback behavior. Make local/cloud choice explicit and auditable.
Provider secrets remain inaccessible through project/agent interfaces.

Affected invariants: `INV-JOB-001`, `INV-CACHE-001`, `INV-AI-001`,
`INV-AI-002`, `INV-AI-003`, `INV-AI-004`, `INV-SEC-001`, `INV-DEP-001`.

## Stop conditions

Stop for direct provider calls from core/UI, model/license ambiguity, hidden
project mutation, missing revision checks, plaintext credentials, unbounded AI
jobs, a request for autonomous application without review/permission, or a
proposal to make MotionScene rendering provider-dependent.

## Handoff

Report task/provider/model contracts, proposal and apply evidence, model rights,
secret handling, cancellation/resource tests, docs, commit, and hosted runtime
status. Automatic captions are not complete until 10C and the relevant caption
conformance checks pass.
