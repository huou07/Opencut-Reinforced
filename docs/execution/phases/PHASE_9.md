# Phase 9 — Android

## Status

Phase 9 is planned and follows the Desktop MVP gate. Android reuses the same
Rust command/state model and runtime contracts; it does not create a second
editing engine or a second canonical project document.

## 9A — Android SAF and project/media I/O abstraction

Implement Android Storage Access Framework permission and document handling at
the platform boundary. Keep content URIs out of Rust path-only APIs until an
explicit abstraction accepts them. Define durable permission behavior,
project/media import/export streams, offline sources, cancellation, and
recovery. Preserve exact-base save checks and secret boundaries.

Affected invariants: `INV-STATE-001`, `INV-STATE-002`, `INV-STATE-003`,
`INV-CACHE-001`, `INV-PERSIST-001`, `INV-SEC-001`, `INV-DEP-001`.

## 9B — Android media and render surface

Evaluate Android `MediaCodec` decode/encode, `Surface`/native buffer ownership,
hardware buffer interop, Vulkan, and wgpu presentation paths behind the Phase 7
runtime interfaces. Prefer zero-copy when safe, but retain a software/CPU
fallback. Platform handles remain runtime-only. Record device API-level and GPU
matrix evidence, queue budgets, cancellation, and fallback telemetry.

Affected invariants: `INV-RT-001`, `INV-RT-002`, `INV-MEDIA-001`,
`INV-MEDIA-002`, `INV-RENDER-001`, `INV-RENDER-002`, `INV-RENDER-003`,
`INV-HW-001`, `INV-HW-002`, `INV-JOB-001`, `INV-DEP-001`.

## 9C — Mobile editor UX

Adapt the editor to touch-native panels, sheets, compact transport, and
playback-like mobile interaction while retaining the shared commands, exact
time, typed read models, permissions, and recovery behavior. Do not claim the
prototype's mobile playback-like behavior is production until this checkpoint
passes hosted/device acceptance.

Affected invariants: `INV-STATE-001`, `INV-STATE-002`, `INV-STATE-003`,
`INV-TIME-001`, `INV-UI-001`, `INV-UI-002`, `INV-IPC-001`.

## 9D — Android export

Add bounded, cancellable export using the shared evaluated snapshot and
approved runtime. Android encoders are optional and must retain a correctness
fallback. Surface permission, storage, progress, cancellation, and failure
states without exposing provider or credential details.

Affected invariants: `INV-RT-001`, `INV-RT-002`, `INV-MEDIA-001`,
`INV-MEDIA-002`, `INV-JOB-001`, `INV-CACHE-001`, `INV-SEC-001`, `INV-DEP-001`.

## 9E — Android resource and device hardening

Test lifecycle interruption, memory pressure, GPU resource loss, background
limits, SAF revocation, offline media, save/reopen/recovery, playback, and
export across the supported device matrix. Document AI runtime direction as
capability/provider selection; do not embed a device-specific model into core.

Affected invariants: all Phase 9 invariants, especially `INV-MEDIA-002`,
`INV-RENDER-002`, `INV-HW-001`, `INV-HW-002`, `INV-JOB-001`, `INV-PERSIST-001`,
`INV-SEC-001`, and `INV-DEP-001`.

## Stop conditions

Stop for unavailable SAF permission semantics, a native buffer lifetime that
cannot be proven, absent software fallback, device-only behavior with no
conformance path, or a proposal to fork project state or editing commands.

## Handoff

Report platform/API matrix, permission and resource evidence, fallback path,
tests, hosted device status, commit, and remaining device-lab gaps.
