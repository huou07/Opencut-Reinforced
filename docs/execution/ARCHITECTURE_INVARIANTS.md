# Permanent Architecture Invariants

These invariants are stable contracts. A feature agent may not weaken or edit
them. Every checkpoint specification lists the IDs it can affect.

## Canonical state

### INV-STATE-001 — Rust owns canonical project state

Rust owns the canonical `ProjectDocument` and project editing state.

### INV-STATE-002 — Validated commands are the mutation gate

Only validated application command execution may mutate canonical project
state.

### INV-STATE-003 — No second editable project document

Flutter, CLI, agents, workers, AI providers, and render workers never maintain
a second canonical editable `ProjectDocument`.

## Runtime and time

### INV-RT-001 — Runtime work does not revise projects

Per-frame playback, render, decode, and audio work is runtime activity. It never
increments `ProjectRevision`.

### INV-RT-002 — Workers consume versioned snapshots

Render and media workers consume a stable, versioned evaluated snapshot and do
not hold a mutable project lock for the frame lifetime.

### INV-TIME-001 — Exact time remains canonical

Canonical project and timeline time remains exact `RationalTime`. Display or
device conversions require explicit policies.

## UI and transport

### INV-UI-001 — Flutter owns presentation state only

Flutter owns presentation, read-model, navigation, gesture, and temporary input
state only. It does not own canonical editing truth.

### INV-UI-002 — No copied Dart frame stream

Full-rate decoded frames are never transported as copied Dart byte arrays.

## Media and rendering

### INV-MEDIA-001 — Software and hardware frame paths are explicit

Media/decode runtime supports software frames and hardware/native surfaces
behind explicit runtime interfaces.

### INV-MEDIA-002 — Hardware retains a correctness fallback

Every hardware acceleration path retains a correctness-preserving software or
CPU fallback where the operation permits one.

### INV-RENDER-001 — wgpu is the shared render spine

wgpu is the shared cross-platform render-graph spine.

### INV-RENDER-002 — Native interop is isolated

Metal, DX12, Vulkan, CUDA, and native interop are isolated behind runtime
adapters. Platform handles never enter domain state.

### INV-RENDER-003 — Native handles are not serialized

Native GPU and resource handles are never serialized into project, IPC, or
cache identity.

### INV-TEXT-001 — Deterministic text has an explicit font identity

Text render and export geometry must not implicitly depend on whichever system
font happens to be installed. Persistent text references a stable, explicit
font identity; the rendering path resolves that identity deterministically.

### INV-MOTION-001 — Canonical motion is declarative

Canonical MotionScene, template, and project data contains no executable
JavaScript, shell commands, provider calls, arbitrary executable expressions,
network requests, or arbitrary shader programs.

### INV-MOTION-002 — Motion evaluation is exact-time and seekable

Canonical motion timing uses `RationalTime`. Evaluation at exact time `T` is
derivable from the scene, assets, evaluator version, and `T` without replaying
from time zero. It does not depend on wall-clock time or unseeded randomness,
and preview and materialization use the same semantic evaluator.

### INV-MOTION-003 — Procedural code is isolated and materialized

Arbitrary HTML/CSS/JS/Canvas/WebGL/WebGPU may exist only behind a future
explicit sandboxed procedural adapter. It does not execute inside `or_core`,
Flutter, normal canonical project evaluation, or normal project open. Its
output enters OR only through bounded derived output/media paths.

### INV-MOTION-004 — Motion generation is AI-optional

MotionScene validation, inspection, preview, rendering, materialization, and
CLI usage work without an AI provider. External agents and built-in AI are
alternative producers only.

### INV-EXT-001 — Sandboxed extensions have explicit bounded authority

Extension execution receives only explicit capabilities and bounded resources.
Default authority includes no filesystem, environment, credential, or network
access; project changes use normal validated commands.

### INV-HW-001 — Capability selection is centralized

Runtime hardware capability discovery and provider selection are centralized.

### INV-HW-002 — Domain evaluation is platform-neutral

Generic domain and render evaluation must not contain scattered platform checks.

## Product acceptance

### INV-PRODUCT — Product journey completion

A user-visible checkpoint is incomplete until its primary real user journey
works end to end through the actual product boundary.

### INV-PACKAGE — Packaged capability is self-contained

Distributed artifacts may not depend on undocumented host executables, package
managers, developer `PATH` values, build-machine environment variables, or
richer host capabilities unless explicitly declared as product prerequisites.

### INV-CAPABILITY — Tests use the shipped capability

Tests may not use a capability richer than the packaged runtime and then claim
the packaged product supports it.

### INV-PARITY — Preview and export semantics agree

Preview and export share canonical evaluated semantics, timing, canvas and
output geometry policy, text geometry, transforms, effects, and source
interpretation unless a difference is an explicit documented product contract.

### INV-PERF — Realtime work reuses bounded state

Realtime frame and audio-block paths do not repeatedly open containers,
construct codecs, spawn processes, compile shaders, open databases, or acquire
platform resources when reusable bounded runtime state is possible.

### INV-UX — Actions have observable outcomes

A user action does not silently no-op. Unsupported or unavailable behavior is
disabled or produces actionable feedback.

### INV-ACCEPT — Product acceptance has its own path

Unit, synthetic, and bridge tests remain useful lower-level evidence. They do
not substitute for a feasible real product acceptance path for a user-visible
capability.

### INV-VERIFY — Verification preserves fidelity

A failing acceptance gate may not be weakened by replacing its environment or
product path with a lower-fidelity harness. A diagnostic harness must be
labelled as such and cannot satisfy higher-fidelity acceptance evidence.

## Jobs and cache

### INV-JOB-001 — Background work is bounded

Background work remains bounded, cancellable, and backpressured.

### INV-CACHE-001 — Cache is disposable

Cache presence is never required for project correctness.

## AI

### INV-AI-001 — AI interfaces are task-oriented

AI interfaces are task-oriented and provider-independent.

### INV-AI-002 — AI workers return proposals or assets

AI workers return structured outputs, proposals, analyses, or assets. They
never directly mutate `ProjectDocument`.

### INV-AI-003 — AI application uses normal validation

Applying AI output uses normal application validation and revision preconditions.

### INV-AI-004 — Provider and model absence is valid

Provider/model absence is a valid capability state. No provider-independent
architecture checkpoint requires cloud credentials or an unlicensed model to
complete.

## Persistence, IPC, security, and dependencies

### INV-PERSIST-001 — Migrations are explicit and tested

Project schema migrations are explicit, ordered, strict, and regression-tested.

### INV-PERSIST-002 — External storage capabilities are not filesystem paths

External storage identities such as Android SAF URIs are represented by strict
typed references, not fake filesystem paths. Persistable permission grants,
file descriptors, and platform/native handles remain runtime/application
metadata and are never serialized into `ProjectDocument`.

### INV-IPC-001 — IPC is semantic control transport

IPC remains semantic control/data transport and is never a realtime frame
transport.

### INV-SEC-001 — Stored credentials stay secret

No agent or CLI/provider API exposes stored plaintext credentials.

### INV-DEP-001 — Core does not absorb runtime dependencies

Platform and runtime dependencies must not be added to `or_core` merely for
convenience.
