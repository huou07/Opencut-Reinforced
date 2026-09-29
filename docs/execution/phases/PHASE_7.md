# Phase 7 — Preview and playback

## Status

Phase 7 is in progress. Its checkpoints are architecture-gated and must execute
in order after 6E2B. The phase creates the runtime plane around the existing
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

## 7C0 — FFmpeg dependency, packaging, and CI gate

Evaluate and document exactly one Rust integration strategy for FFmpeg:
high-level binding, low-level binding, or direct FFI. Select it from evidence
covering upstream provenance and maintenance, exact FFmpeg and Rust package
versions, API/ABI compatibility and supported-version policy, MSRV, build and
toolchain requirements (including bindgen, clang, pkg-config, and CMake where
applicable), API suitability, and the binding license.

Approve a dynamic or static linking and packaging strategy that records the
development headers/libraries, runtime shared libraries, and macOS, Linux, and
Windows implications. Assess Android separately and stage it for Phase 9 only
when the documented evidence justifies that boundary. Document the FFmpeg build
configuration, LGPL/GPL effects, redistribution obligations, and any
prohibited or nonfree configuration. Record the decision in the existing
technical or security/licensing documentation, or one small dedicated
dependency decision document if that is clearer.

Provision the selected development environment in hosted CI and prove actual
compile/link capability with a bounded probe; merely installing or finding an
`ffmpeg` executable is insufficient. Preserve software decode as 7C's first
correctness path. This gate does not create `or_media` or implement demux,
seek, decode, resampling, `FrameLease` production, playback, or hardware
acceleration. Do not add a media crate or select the binding before this
checkpoint executes.

Affected invariants: `INV-MEDIA-001`, `INV-MEDIA-002`, `INV-HW-001`,
`INV-HW-002`, `INV-DEP-001`.

## 7C — Linked media runtime and software decode

Consume the dependency, linking, packaging, and build strategy approved by 7C0.
The approved strategy is ffmpeg-the-third 6.0.0 with FFmpeg 8.1.x, dynamically
linked against shared libraries from an LGPL-only FFmpeg build. Do not
reconsider or silently switch the dependency from scratch. If concrete
compatibility, build, or licensing evidence proves the approved strategy
invalid, stop and report that evidence instead of replacing it.

7C may modify exactly `.github/workflows/platform-verification.yml`, only as
needed to make the approved strategy available to production workspace build
and test verification. When required, this may include exposing the approved
FFmpeg development headers and libraries to workspace Cargo checks and tests,
persisting the approved prefix environment to later relevant CI steps, enabling
only the minimum FFmpeg components needed for deterministic software decode and
resample fixtures, and adding bounded production-workspace compile, link, and
decode verification.

7C must not switch bindings or the approved FFmpeg major/minor strategy, enable
GPL or nonfree components, change dynamic/shared packaging policy, weaken hosted
verification, modify another workflow, or implement hardware decode. If
concrete evidence proves the 7C0-approved strategy invalid, stop rather than
silently replacing it.

Then create the future `or_media` crate. It owns demux, exact timestamp
mapping, seek, software video decode, audio decode, resampling, bounded queues,
cancellation, and `FrameLease` production. Media sources remain outside
canonical domain state; workers consume a stable `RenderSnapshot` and exact
time requests.

The first complete path is software fallback: it must seek, decode, resample,
and feed bounded runtime queues before any hardware optimization. All queues
must expose cancellation, backpressure, and stale-snapshot behavior.

Affected invariants: `INV-RT-001`, `INV-RT-002`, `INV-TIME-001`, `INV-MEDIA-001`,
`INV-MEDIA-002`, `INV-JOB-001`, `INV-CACHE-001`, `INV-DEP-001`.

## 7D — Hardware decode and native-frame interop evaluation and optional implementation

Evaluate currently eligible platform hardware paths using capability evidence,
build and license evidence, and repeatable target-hardware measurements when a
performance benefit is claimed. Hardware acceleration is optional and is not
required to complete Phase 7. Implement only paths with sufficient evidence.

If no hardware path has sufficient evidence, this is a valid checkpoint
outcome: keep software decode as the authoritative correctness path, record
that no hardware path is currently approved, preserve centralized
capability/provider selection and deterministic software fallback, and add
repository-visible tests or documentation for that decision. Commit the
decision, make no hardware performance claim, and defer adapters until evidence
exists. No hardware path approved is a decision, not a blocker; do not invent
benchmark results or implement hardware speculatively merely to advance.

A genuine blocker is a broken software fallback, an architecture invariant that
cannot be preserved, build or license evidence contradicting an already-enabled
path, or a required correctness path that cannot build or test. Native handles
never enter `or_core`, serialized project state, IPC, or cache keys. Every
enabled path retains software/CPU fallback and clear fallback telemetry.

The shared render spine remains wgpu. Possible paths include Apple
VideoToolbox/native surfaces, Windows D3D, Linux Vulkan/DMABUF, and NVIDIA
adapters, but each remains unapproved until its evidence qualifies.

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

7E may modify exactly `.github/workflows/platform-verification.yml` only as
needed to provision the selected audio backend's hosted build requirements or
run bounded deterministic audio and synchronization verification. Keep timing
correctness tests deterministic and do not depend on shared-runner audio
hardware or performance claims.

Affected invariants: `INV-RT-001`, `INV-RT-002`, `INV-TIME-001`, `INV-JOB-001`,
`INV-DEP-001`.

## 7F0 — Playback timing and viewer transport contract gate

Lock the one canonical sequence timing model and the smallest shared viewer
presentation boundary needed by 7F. This is an architecture/model checkpoint;
it does not implement a product viewer or playback UI. Do not infer decisions
from active media, platform defaults, or undocumented implementation choices.

The canonical `ProjectTimeline.sequence_frame_rate` is one optional exact
`RationalRate` in frames per second. A project created without an explicit rate
remains unconfigured; migrations from project schemas v1–v4 also produce an
unset rate. There is no guessed or source-derived default. Authorize the
required project schema evolution and strict migrations through the existing
persistence path, and include the field in recovery snapshots. Add the typed
`timeline.sequence.set_frame_rate`
command and `timeline.sequence.settings` query through the existing validated
application boundary and generic `ApplicationRequest` route. A real change is
one validated project command, history entry, and revision increment; setting
the existing value is a no-op. Migration preserves revision and does not dirty
the project before explicit save. Keep the recovery envelope and IPC protocol
versions unchanged.

For sequence frame index `n`, the exact presentation time is `n / rate` from
timeline zero. All multiplication, division, boundary selection, and conversion
use checked exact integer/rational arithmetic; floating point is display-only.
At nonnegative exact playhead time `t`, next frame selects
`floor(t × rate) + 1`; previous selects `ceil(t × rate) - 1`, clamped to frame
zero and the last valid sequence frame. A frame is valid only when its exact
time is before the half-open content end. Previous at content end selects the
last valid frame; next at or beyond content end is a no-op. Empty timelines
have no playable frames. The content end is the greatest exact clip end across
audio and video tracks; markers do not extend it and playback does not loop.

Seek and scrub accept exact nonnegative `RationalTime` values and do not snap or
round them to sequence frames. They remain available while the sequence rate is
unset. Play and frame-step require an explicit sequence rate and otherwise
return a typed unavailable/validation result. Seeking into a gap or beyond the
content end retains the requested exact playhead and presents a blank/neutral
frame; playback starting at or beyond the content end completes without
advancing.

The sequence rate defines one global output-frame lattice for all source rates.
For an output time inside a clip, source time is exactly
`source_range.start + (output_time - clip.timeline_start)`. Use the preceding
source presentation timestamp, holding that source frame until the next one;
do not interpolate. This applies to mixed and variable source frame rates. A
still image holds for its clip's explicit duration. Audio-only clips
contribute to content duration and the audio clock but do not synthesize video;
the viewer remains blank where no video contributes. Frame-step always follows
the global sequence lattice. While audio output is active, the 7E audio clock
drives playback; otherwise use a monotonic runtime clock anchored to the exact
seek/play origin. Scrub position maps to exact timeline time, not a source or
sequence-frame index.

Lock one shared semantic viewer contract: `or_render`/wgpu owns render output;
`or_runtime` owns snapshot identity, `FrameLease`, cancellation, and release
coordination; a native adapter owns platform texture registration, native
resources, synchronization, and Flutter texture lifetime; Flutter displays the
registered external texture and sends controls through the application/bridge
path. Use platform-specific adapters behind this contract. The supported
desktop fallback is a bounded pixel-buffer presentation path on each platform;
use shared GPU/native surfaces only where that platform/backend interop is
validated. Keep copies out of Dart and minimize copies on the presentation
path; universal zero-copy is not a requirement.

The bridge may carry an opaque registered Flutter texture identifier, frame
dimensions/format, exact presentation time, transport state, errors, and user
controls. It must never carry per-frame pixel buffers or frame-rate frame bytes
through Dart. Raw OS/GPU handles never cross Dart and stay out of `or_core`, IPC,
project files, and cache identity.
Adapters retain each `FrameLease` until the platform's release callback or
completion fence signals. Use a bounded latest-frame mailbox/in-flight set;
reject stale generation/revision frames so old work cannot replace newer
presentation. A bounded hosted proof may be added only when needed to establish
that the selected transport builds on supported desktop targets.

Affected invariants: `INV-STATE-001`, `INV-STATE-002`, `INV-STATE-003`,
`INV-RT-001`, `INV-RT-002`, `INV-TIME-001`, `INV-UI-001`, `INV-UI-002`,
`INV-PERSIST-001`, `INV-RENDER-001`, `INV-RENDER-002`, `INV-IPC-001`,
`INV-DEP-001`.

## 7F — Real viewer and preview transport

Consume the timing and viewer contracts locked by 7F0. Connect the product
viewer and add play, pause, exact seek, scrubbing, frame step, playhead, ruler,
and transport feedback. Provide the minimal explicit sequence-rate control
needed to configure a new/unset project through the 7F0 command; never choose a
rate implicitly. Viewer controls request runtime work and read presentation
state; they do not mutate the project. Drop stale frames while keeping
exact-time requests and errors observable. Ruler and playhead display
conversions never replace canonical `RationalTime`.

Do not invent or infer a frame rate, redesign project timing, silently switch
the approved viewer transport, or add another canonical state engine. If
concrete build or runtime evidence proves a 7F0-approved contract invalid, stop
and report that evidence for a separate architecture-plan decision.

7F may modify exactly `.github/workflows/platform-verification.yml` only as
needed to provision viewer/native-surface build requirements or run bounded
hosted viewer and transport verification on supported platforms. Preserve the
existing hosted build and test gates.

Affected invariants: `INV-RT-001`, `INV-RT-002`, `INV-TIME-001`, `INV-UI-001`,
`INV-UI-002`, `INV-MEDIA-001`, `INV-MEDIA-002`, `INV-RENDER-001`,
`INV-RENDER-002`, `INV-JOB-001`, `INV-IPC-001`.

## 7G — Performance architecture gate

Implement deterministic instrumentation and publish repeatable correctness and
conformance measurements where hosted CI is appropriate. Use synthetic media
and legally safe fixtures. Define deterministic, platform-independent semantic
or resource budgets where justified.

When no known dedicated or self-hosted benchmark machine is available, record
`DEDICATED_HARDWARE_PERFORMANCE = UNVERIFIED`. This status does not block Phase
7 completion. Do not set hard FPS thresholds, claim GPU performance, or justify
a hardware path from shared hosted-runner timings. Retain software fallback and
instrumentation for future target-hardware measurements; do not fabricate
benchmark evidence.

Affected invariants: `INV-RT-001`, `INV-RT-002`, `INV-MEDIA-001`,
`INV-MEDIA-002`, `INV-RENDER-001`, `INV-HW-001`, `INV-JOB-001`, `INV-CACHE-001`,
`INV-DEP-001`.

## 7H — Phase 7 hardening

Run cross-platform/runtime conformance, failure injection, seek cancellation,
resource-lifetime, offline-media, recovery, and stale-snapshot tests. Verify
software fallback on supported paths, document native capability matrices, and
make the Phase 7 viewer/playback path safe for the Developer Preview gate. The
preview may include only capabilities delivered by Phase 7; it must not depend
on Phase 8 transforms, captions, effects, export, or other later work. A
Developer Preview may be considered only when the checkpoint's hosted CI and
release gates pass; this architecture lock does not publish one. Missing
dedicated-hardware performance evidence remains `UNVERIFIED` and is not a
hardening blocker.

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
