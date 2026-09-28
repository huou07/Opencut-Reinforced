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

### INV-HW-001 — Capability selection is centralized

Runtime hardware capability discovery and provider selection are centralized.

### INV-HW-002 — Domain evaluation is platform-neutral

Generic domain and render evaluation must not contain scattered platform checks.

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

## Persistence, IPC, security, and dependencies

### INV-PERSIST-001 — Migrations are explicit and tested

Project schema migrations are explicit, ordered, strict, and regression-tested.

### INV-IPC-001 — IPC is semantic control transport

IPC remains semantic control/data transport and is never a realtime frame
transport.

### INV-SEC-001 — Stored credentials stay secret

No agent or CLI/provider API exposes stored plaintext credentials.

### INV-DEP-001 — Core does not absorb runtime dependencies

Platform and runtime dependencies must not be added to `or_core` merely for
convenience.
