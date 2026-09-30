# Phase 9 — Android

## Status

Phase 9 is planned and follows the Desktop MVP gate. Android reuses the same
Rust command/state model and runtime contracts; it does not create a second
editing engine or a second canonical project document.

## 9A0 — Android native media and package gate

After 8F and before Android storage or product work, prove Android native
prerequisites only. Reuse the exact FFmpeg 8.1.3 software configuration
approved by the completed desktop path, including its required software
decode/export components. Use Android NDK 28.2.13676358 and Rust targets for
`arm64-v8a`, `armeabi-v7a`, and `x86_64`; x86_64 is required for hosted emulator
conformance.

Hosted verification must prove a shared FFmpeg build for every supported ABI,
Rust cross-linking, APK packaging, runtime library loading, Flutter/Rust bridge
loading with those libraries present, and the x86_64 Android
emulator/instrumentation path. Do not require a physical device, MediaCodec,
AHardwareBuffer, Vulkan zero-copy, or hardware encoding. Software FFmpeg
remains the correctness path. Physical GPU/device coverage may be recorded as
`ANDROID_HARDWARE_MEDIA = UNVERIFIED` without blocking Phase 9.

This gate does not implement Android product features or publish a preview. It
may modify exactly `.github/workflows/platform-verification.yml` as authorized
by `PLAN.json`; it has no other protected-workflow allowance.

Affected invariants: `INV-MEDIA-001`, `INV-MEDIA-002`, `INV-RENDER-002`,
`INV-PERSIST-002`, `INV-DEP-001`.

## 9A — Android SAF and project/media I/O abstraction

Own the Android project-model gate for source references and the storage
boundary. Generalize persistent media identity to a strict typed
`MediaSourceRef` with at least `FileUri` and `AndroidSafDocumentUri` variants;
keep existing `file:` behavior unchanged. Never turn a `content://` URI into a
fake filesystem path. A SAF document URI may be persisted as source identity,
but its OS persistable-permission grant is runtime/application metadata, not
`ProjectDocument` data. Tokens, file descriptors, and native handles are never
serialized. A SAF reference loaded on a non-Android platform remains valid
project data and resolves offline/unavailable until relinked. Project loading
validates the project without opening source media. Retain existing URI
size/security bounds unless a documented migration requires a stricter limit.

Android uses an app-private managed working copy as the canonical durability
anchor for an active project session; SAF is the selected import/export and
synchronization boundary. Opening an external `.orproj` uses a bounded SAF
read, strict project validation, a safe app-private working copy, then the
normal Rust `ProjectFileSession`. Saving first performs ordinary atomic local
save and recovery on that working copy, then explicitly synchronizes/exports
through SAF, with readback verification where the provider permits it. Failure
to write the external document never destroys the valid local working project.
Do not claim filesystem atomic-replace guarantees for Android
`DocumentProvider` operations, and do not weaken desktop atomic-save/recovery.

The runtime media boundary accepts a transient opaque seekable media-I/O
capability. The Android adapter may use a duplicated file descriptor or
bounded custom FFmpeg I/O. The URI remains the source reference; the live FD or
native handle remains runtime-only. Do not copy every imported video to app
storage just to obtain a path. A non-seekable provider or a bounded component
that genuinely requires a local file may use a bounded, cancellable, disposable
managed materialization/cache; it is never canonical project state. 9D export
uses this same SAF/storage abstraction and retains the mandatory software
FFV1 + PCM S16LE correctness encoder.

Affected invariants: `INV-STATE-001`, `INV-STATE-002`, `INV-STATE-003`,
`INV-CACHE-001`, `INV-PERSIST-001`, `INV-PERSIST-002`, `INV-SEC-001`,
`INV-DEP-001`.

## 9B — Android media and render surface

Evaluate optional Android `MediaCodec` decode/encode, `Surface`/native buffer
ownership, hardware-buffer interop, Vulkan, and wgpu presentation paths behind
the Phase 7 runtime interfaces. Software FFmpeg and the bounded pixel-buffer
path remain authoritative fallbacks. No hardware API or zero-copy path is
required for Phase 9 completion; platform handles remain runtime-only. Record
device API-level and GPU-matrix evidence, queue budgets, cancellation, and
fallback telemetry. Missing physical-device coverage is reported as
`ANDROID_HARDWARE_MEDIA = UNVERIFIED`.

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
approved runtime and the 9A SAF/storage abstraction. The mandatory correctness
encoder remains software FFV1 + PCM S16LE through linked FFmpeg. Android
hardware encoders are optional and must retain this correctness fallback.
Surface permission, storage, progress, cancellation, and failure states
without exposing provider or credential details.

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
