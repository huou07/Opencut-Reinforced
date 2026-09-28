# Phase 7 — Preview and playback

## Status

Phase 7 is planned. Its checkpoints are architecture-gated and must execute in
order after 6E2B. The phase creates the runtime plane around the existing
Rust-owned project/control plane; it does not move canonical editing state into
workers or Flutter.

## Architecture boundary

The control plane is `Flutter/CLI/Agent -> typed commands and queries ->
or_core -> ProjectDocument`. The runtime plane is `ProjectDocument ->
RenderSnapshot -> media decode/audio/render workers -> viewer or export`.
Runtime activity is versioned, bounded, cancellable, and disposable. A frame
does not execute a project command or increment `ProjectRevision`.

`RenderSnapshot` is an immutable evaluated view for a requested time/range and
revision. `FrameDescriptor` describes a decoded or rendered frame without
copying it through Dart. `FrameLease` carries ownership/lifetime and explicit
release semantics for software buffers and native surfaces. Native handles are
runtime-only and never enter project, IPC, or cache identity.

## 7A — Realtime architecture and capability foundation

Define and test the control/runtime boundary, snapshot revision and exact-time
semantics, `FrameDescriptor`/`FrameLease` ownership, bounded queues, cancellation
and backpressure, render/audio/decode budgets, and centralized capability and
provider selection. Define software fallback behavior before selecting hardware
paths. Lock dependency, MSRV, license, and platform evidence requirements.

No visible playback is required in 7A. No per-frame project mutation,
unbounded worker queue, copied Dart frame stream, or platform handle in
`or_core` is permitted.

Affected invariants: `INV-STATE-001`, `INV-STATE-002`, `INV-STATE-003`,
`INV-RT-001`, `INV-RT-002`, `INV-TIME-001`, `INV-UI-002`, `INV-MEDIA-001`,
`INV-MEDIA-002`, `INV-HW-001`, `INV-HW-002`, `INV-JOB-001`, `INV-CACHE-001`,
`INV-IPC-001`, `INV-DEP-001`.

## 7B — wgpu render foundation

Create the future `or_render` crate only after 7A's gate. Verify the exact
official `wgpu` version, MSRV, supported backends, license, and platform
compatibility before pinning it. Start with deterministic synthetic scenes and
offscreen/headless rendering. Establish a preview surface abstraction, render
graph inputs, resource lifetime, and readback tests. Flutter receives a viewer
surface or handle-level presentation contract, never copied frame bytes.

Keep the shared render spine in `wgpu`; isolate Metal, DX12, Vulkan, and other
native interop in runtime adapters. Do not implement a product viewer yet.

Affected invariants: `INV-RT-001`, `INV-RT-002`, `INV-UI-002`, `INV-RENDER-001`,
`INV-RENDER-002`, `INV-RENDER-003`, `INV-HW-001`, `INV-HW-002`, `INV-JOB-001`,
`INV-CACHE-001`, `INV-DEP-001`.

## 7C — Linked media runtime and software decode

Create the future `or_media` crate only after an official FFmpeg binding and
build/license/MSRV/API review. It owns demux, exact timestamp mapping, seek,
software video decode, audio decode, resampling, bounded queues, cancellation,
and `FrameLease` production. Media sources remain outside canonical domain
state; workers consume a stable `RenderSnapshot` and exact time requests.

The first complete path is software fallback: it must seek, decode, resample,
and feed bounded runtime queues before any hardware optimization. All queues
must expose cancellation, backpressure, and stale-snapshot behavior.

Affected invariants: `INV-RT-001`, `INV-RT-002`, `INV-TIME-001`, `INV-MEDIA-001`,
`INV-MEDIA-002`, `INV-JOB-001`, `INV-CACHE-001`, `INV-DEP-001`.

## 7D — Hardware decode and native-frame interop

Add measured, optional adapters behind `or_media`/`or_render` runtime
interfaces: Apple VideoToolbox and native surfaces, Windows D3D paths, Linux
Vulkan/DMABUF paths, and NVIDIA paths only when benchmark evidence shows a
required benefit and the licensing/build gate is recorded. The shared render
spine remains wgpu. Native handles never enter `or_core`, serialized project
state, IPC, or cache keys. Every path retains software/CPU fallback and clear
fallback telemetry.

Apple direction: Metal resources interoperate with wgpu through an adapter;
VideoToolbox supplies decode surfaces; MPS is an optional future compute path;
Core ML and MLX stay behind capability/provider boundaries. NVIDIA direction:
CUDA is an optional runtime boundary, NVDEC/NVENC are measured decode/encode
paths, and Vulkan/DX12 remain distinct graphics/interoperability choices.

Affected invariants: `INV-RT-001`, `INV-RT-002`, `INV-MEDIA-001`,
`INV-MEDIA-002`, `INV-RENDER-001`, `INV-RENDER-002`, `INV-RENDER-003`,
`INV-HW-001`, `INV-HW-002`, `INV-JOB-001`, `INV-DEP-001`.

## 7E — Audio clock and A/V synchronization

Create the future `or_audio` boundary. Establish one master clock, exact
timeline-to-device conversion policy, bounded audio buffers, underrun behavior,
drift correction, cancellation, and realtime-safety tests. Audio callbacks must
not allocate, block on project locks, call provider/network code, or mutate
canonical state. Audio and video workers coordinate through runtime snapshots
and clock messages, not editing commands.

Affected invariants: `INV-RT-001`, `INV-RT-002`, `INV-TIME-001`, `INV-JOB-001`,
`INV-DEP-001`.

## 7F — Real viewer and preview transport

Connect a viewer through the approved native/external texture or equivalent
zero-copy surface contract. Add play, pause, seek, scrubbing, frame step,
playhead, ruler, and transport feedback. Viewer controls request runtime work
and read presentation state; they do not mutate the project. Stale frames may
be dropped, while exact-time requests and errors remain observable. Ruler and
playhead display conversions never replace canonical `RationalTime`.

Affected invariants: `INV-RT-001`, `INV-RT-002`, `INV-TIME-001`, `INV-UI-001`,
`INV-UI-002`, `INV-MEDIA-001`, `INV-MEDIA-002`, `INV-RENDER-001`,
`INV-RENDER-002`, `INV-JOB-001`, `INV-IPC-001`.

## 7G — Performance architecture gate

Instrument and publish repeatable conformance/performance evidence for decode
throughput, seek latency, A/V drift, audio underrun, dropped frames, copy count,
RAM, GPU memory/resources, queue depths, and software/hardware fallback rate.
Use synthetic media and legally safe fixtures. Define budgets and regression
thresholds before optimization. No dependency or native path is justified by
intuition alone.

Affected invariants: `INV-RT-001`, `INV-RT-002`, `INV-MEDIA-001`,
`INV-MEDIA-002`, `INV-RENDER-001`, `INV-HW-001`, `INV-JOB-001`, `INV-CACHE-001`,
`INV-DEP-001`.

## 7H — Phase 7 hardening

Run cross-platform/runtime conformance, failure injection, seek cancellation,
resource-lifetime, offline-media, recovery, and stale-snapshot tests. Verify
software fallback on supported paths, document native capability matrices, and
make the viewer safe for the future Developer Preview gate. A Developer Preview
may be considered only when the checkpoint's hosted CI and release gates pass;
this architecture lock does not publish one.

Affected invariants: all Phase 7 invariants listed above, especially
`INV-STATE-001`, `INV-STATE-002`, `INV-STATE-003`, `INV-RT-001`, `INV-RT-002`,
`INV-MEDIA-001`, `INV-MEDIA-002`, `INV-RENDER-001`, `INV-RENDER-002`,
`INV-RENDER-003`, `INV-HW-001`, `INV-HW-002`, `INV-JOB-001`, `INV-CACHE-001`,
`INV-IPC-001`, and `INV-DEP-001`.

## Stop conditions

Stop for unresolved dependency/MSRV/license evidence, a missing software
fallback, frame copies through Dart, unbounded queues, platform handles in
domain state, a per-frame project revision change, or a native test that cannot
be represented in hosted CI.

## Handoff

Each checkpoint reports its runtime boundary, evidence, budgets, changed
dependencies, platform coverage, tests, commit, and push state. The next
relation is taken from `PLAN.json`; no runner may silently continue into Phase 8.
