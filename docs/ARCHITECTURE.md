# Opencut Reinforced Architecture

## Status

The active product architecture follows the 2026 convergence decision in
[OPEN_SOURCE_CONVERGENCE.md](OPEN_SOURCE_CONVERGENCE.md); coherent user
capabilities and real technical dependencies are planned in
[PRODUCT_ROADMAP.md](PRODUCT_ROADMAP.md). The older phase contracts below remain
historical product requirements and evidence, not an instruction to build each
planned subsystem independently. In particular, media-engine replacement is
not approved: OR keeps its current FFmpeg/wgpu path while a bounded MLT/GES
comparison is triggered only by a measured packaged-product gap. OpenCut's
rewrite is monitored until its promised core/API ships and can be tested.

This is the canonical high-level architecture. It documents the implemented
project/application, Flutter shell, media, and timeline foundations alongside
planned components. The
Flutter application creates and opens real projects, owns one Rust live host
shared with local IPC, presents explicit recovery and dirty-state workflows,
and provides desktop media-library import/list/remove plus read-only generated
thumbnail and waveform previews. `.orproj` schema v7 persists validated media references, metadata, canonical
ordered timeline state, global point markers, one nullable sequence frame rate,
typed media/text/caption clips, and persistent track state while loading v1–v6. Schema v7 adds strict local-file and Android SAF source identities. Phase
5D–5F provide bounded fingerprints, disposable indexed cache, and core-only
file-backed proxies.
Phase 6 provides canonical tracks, clips, exact editing, persistent markers,
marker-aware snapping, CLI parity, and the corresponding Flutter UI without a
second editable UI state. Phase 7A adds the dependency-free `or_runtime`
contract for immutable render-snapshot identity, exact-time frame metadata and
leases, bounded cancellation-aware queues, separate render/audio/decode
budgets, and centralized capability selection with software fallback. 7B adds
the headless `or_render` wgpu spine, 7C adds linked software decode, 7E adds the
audio clock, and the shared 7F0 contract defines explicit sequence timing
and viewer presentation. The 7F implementation connects a desktop preview
runtime to the product viewer and transport controls; hosted verification is
required before the checkpoint can advance. For authoritative current checkpoint and phase status,
see [docs/execution/STATE.json](execution/STATE.json); this document does not
copy mutable `NEXT` state. Planned and future components below do not imply
implemented code. See [ROADMAP.md](ROADMAP.md) and
[TECHNICAL_PLAN.md](TECHNICAL_PLAN.md) for human-readable design detail.

## Implemented today

The Rust workspace owns the versioned `.orproj` document, validated commands and queries, exact timeline time, edit history, media identity, and project persistence. Schema v7 stores media references, typed video/audio/text/caption clips, track and clip settings, markers, and an optional sequence rate. Save performs bounded atomic replacement after exact-base checks. Dirty projects periodically update a recovery sidecar; explicit Save updates the canonical project, and recovery is an explicit user action.

Flutter is the human editing surface. Desktop and Android use the same Rust project/session and command semantics; the desktop CLI can attach to the application’s live project host over authenticated local IPC, while Android uses the shared bridge in-process. Current UI capabilities include project create/open/save/recovery, multi-file media import, timeline track/clip edits, markers/snapping, text and caption clips, selected transform/audio/effect controls, preview transport, and a limited export workflow. Android file access uses SAF grants and an app-private working copy; it does not reinterpret provider URIs as paths.

Media operations use packaged FFmpeg 8.1.3 libraries and packaged-first helper executables in supported app bundles. The verified profile is Matroska with FFV1 video and PCM S16LE audio plus standalone PCM WAV import; export uses the Matroska profile. A candidate extends the same probe/decode path to MP4-family H.264/AAC, pending exact-SHA packaged desktop and Android journeys and distribution patent review. The app has bounded software decode, wgpu rendering, audio output on the currently supported desktop path, and Android software preview/export paths. Proxies and generated previews are disposable cache data, not canonical project state. Broader format coverage, effects and audio workflows, AI assistance, and production-signed releases remain product gaps.

GitHub Actions provides the platform build and runtime evidence recorded with exact source SHAs in the active product roadmap. Those checks cover specified journeys and boundaries; they do not imply that every feature is complete or that a debug Developer Preview is ready for general release. The frozen HTML prototype remains a UX reference, not production architecture.

## Planned full target architecture

    Flutter GUI
        | typed bridge and events
        v
    Application Layer <--- local IPC <--- CLI
        ^                                  Agents
        | structured commands and queries
    Command and Query Registry
        |
    Rust Domain Core
        +-- Project
        +-- Timeline -----------------------> Render Graph ---> wgpu
        +-- Media ---> FFmpeg ---> decoded sources --/
        +-- Jobs ---> AI tasks / downloads
                 +--> Export job -----------> Render Graph ---> FFmpeg encode/mux

The diagram describes a target. Exact bridge, bindings, and rendering integration remain subject to implementation-time evaluation. Flutter is the presentation layer; Rust owns project and editing truth. GUI, CLI, and agents submit the same validated domain operations and query the same structured state. They must not become independent editing engines.

### Control/project plane and real-time media plane

The planned architecture separates project decisions from time-sensitive playback and rendering work:

    Flutter / CLI / Agent
             |
             v
      Commands / Queries
             |
             v
     Canonical Project State
             |
      evaluated snapshot
             |
             v
    -------------------------
      REAL-TIME MEDIA PLANE
    -------------------------
             |
      Decode / Audio / Render
             |
             v
           Output

The control/project plane owns project mutations, commands, queries, undo and redo, `ProjectRevision`, persistence, and timeline editing decisions. The real-time media plane owns transient decoded frames, audio buffers, playback clock, frame queues, render resources and GPU textures, frames in flight, render scheduling, dropped-frame decisions, and transient playback position.

Per-frame playback and rendering must not create Project transactions or increment `ProjectRevision`. The revision changes only when canonical project state changes. Playback ticks, decoding, audio buffering, render evaluation, presentation, and dropping an obsolete preview frame are runtime activity, not project edits.

Render workers consume a stable, versioned evaluated view of project/timeline state. The renderer must not mutate canonical Project state or require a heavyweight lock on mutable Project state for its lifetime. Snapshot representation and granularity remain open; a full immutable evaluated structure, incremental graph, structural sharing, versioned read model, or another measured design may satisfy this boundary.

### Canonical mutation and concurrency

Only the command/application execution path may mutate canonical Project state. Phase 4D implements `project.rename`, `history.undo`, `history.redo`, and atomic groups containing one or more `project.rename` calls. Phase 5B adds non-transactional `media.add` and `media.remove`; `media.relink` replaces a re-probed source under the existing MediaId and preserves timeline references. Commands require matching `ProjectId`, `ProjectInstanceId`, and expected `ProjectRevision`; a stale request is rejected for re-query and revalidation. Each successful media command increments revision once, while failed commands and reads do not. Each changed forward operation creates one `ChangeSet` and one session-local history entry; media history stores the exact item and original insertion index instead of full `ProjectDocument` snapshots. Undo/redo are new canonical mutations and increment revision. GUI, CLI, agents, renderers, decoders, jobs, and AI workers must not write canonical project state directly. This command path is for project edits, never a per-frame playback or render execution path.

## Planned boundaries

### Presentation and feature integration

Flutter presents the product, handles interaction, accessibility, navigation, panels, inspectors, and timeline presentation. It sends user intent through the application command boundary and renders returned state. It does not own canonical project state or exported video text. Flutter may keep navigation, panel, selection, tool, and temporary input state, plus scoped read-model caches; it does not maintain a second editable project model. Rust change/invalidation events lead Flutter to refresh affected queries. Events carry revision/order information so stale updates are ignored or requeried. Hot UI paths use scoped views rather than copying and rebuilding the whole project.

New UI features should use existing shell slots: App Bar, Editor Tool Rail, Left Tool Panel, Viewer, Inspector, Timeline Toolbar, Timeline, Task or Status Area, Command Palette, and Dialog or Mobile Sheet. A lightweight static feature descriptor may register an ID, label, icon, group, availability, command IDs, panel, inspector sections, and shortcut metadata. This is a boundary for integration, not a reason to build a runtime framework now.

Simple and Advanced modes are visibility preferences over the same state and command model. Hiding an advanced control must not remove or fork the underlying project data.

### Application and command/query APIs

The application layer owns command discovery, validation, authorization, transactions, job coordination, and structured errors. A command is validated before mutation and returns a ChangeSet. Read-only queries expose project, timeline, media, captions, command, and capability information.

The CLI is a first-class semantic client of this API. Its parity is for domain operations and meaningful project queries, not presentation-only controls. It supports headless operation and attachment through an explicit descriptor to either the Flutter application host or the developer/headless `or session serve` host. The same-host parity test launches the real CLI against a `LiveProjectHost` while exercising its direct application path, covering IDs, revisions, undo/redo, dirty state, save, and event order. The CLI does not automate the UI by clicking coordinates. Agents are also intended to be clients of structured queries and commands; generated EditPlans must pass the same permission and domain validation as human-initiated work.

### Rust domain

Rust is intended to own project, timeline, media identity and metadata, command behavior, history, deterministic editing operations, and render evaluation. UI widgets and agent sessions are not sources of domain truth. The first implementation should remain a small workspace; planned domains do not require empty crates.

### Media, render, and audio

FFmpeg is the media layer for demuxing, software decoding, encoding, muxing, and conversion or resampling. The Phase 7 software path uses `ffmpeg-the-third` 6.0.0 with FFmpeg 8.1.3 shared libraries built from the official source archive under the approved LGPL-only profile. Supported app bundles carry the linked runtime and the packaged-first helpers required by import and preview-artifact workflows; unpackaged development may use explicit overrides or PATH.

`or_core::probe_media_file(&Path)` accepts a local regular file and invokes the packaged-first `ffprobe` helper directly, without a shell. It requests a bounded JSON field set; timeout, output limits, process cleanup, and metadata validation remain enforced. Android SAF imports probe granted descriptors through the in-process FFmpeg boundary instead of converting URIs into paths. Prepared candidates enter canonical project state only through `media.add`. Project load and `media.list` display persisted metadata without opening or probing sources. See the [official ffprobe documentation](https://ffmpeg.org/ffprobe.html).

The render core evaluates a versioned timeline view into sources, transforms, effects, compositing, color, and output. Preview and export use the same edit semantics, while scheduling and quality may differ; cross-GPU pixels are not required to be bit-identical. Phase 7B establishes `or_render` on the pinned wgpu 25.0.2 spine; backend support and performance remain subject to hosted platform verification and later measurement.

The future canonical simple-motion path is a versioned, non-executable
`MotionScene` source. It uses exact `RationalTime`, typed bounded primitives,
random-access evaluation, and stable asset/font provenance. Evaluation lowers to
the same `RenderSnapshot` and wgpu render spine used by the rest of OR; preview
and materialization share the evaluator. MotionScene validation, inspection,
rendering, materialization, and CLI use do not require an AI provider. The
Phase 11 path is materialization-first: render persistent generated media,
register it through normal media commands, and use ordinary timeline clips.
See [ADR 0007](adr/0007-declarative-motion-scenes-and-procedural-isolation.md).

OR's intended media policy is zero-copy where platform/backend interoperability safely permits it, and otherwise to minimize copies across hot media paths. This is not a universal zero-copy promise: software and CPU-frame fallbacks remain first-class. Future frame boundaries must be able to represent CPU frames, GPU textures, hardware-decoder surfaces, and external/shared platform surfaces without forcing hardware-decoded frames through CPU memory. Avoid a design that copies every decoded frame through Rust byte buffers, Dart objects, and a Flutter GPU upload.

The decoder boundary must support software decode and hardware-surface decode, with automatic capability-based selection and a correctness fallback. Export should prefer a GPU/native-compatible surface into a hardware encoder where supported, and otherwise use a CPU frame with a supported software or platform encoder. Hardware paths are not assumed to be faster or available for every device, codec, or format. Platform-specific interop belongs behind narrow media/render boundaries; project and timeline semantics stay platform-independent.

GPU work is a candidate for scaling, rotation, crop, color conversion where appropriate, blending, masking, compositing, color operations, and suitable effects. Project state, command validation, serialization, metadata, scheduling/orchestration, and unsuitable operations remain CPU/domain responsibilities. Profile the workload; not every operation belongs on the GPU. Preview can prioritize latency with lower resolution, proxies, reduced-quality effects, and bounded work, while export can prioritize quality and throughput. Both preserve the same timing, transform, effect, compositing, text, keyframe, and color intent.

Audio decoding belongs in the media layer. `or_audio` owns the device-neutral bounded output buffer, exact audio master clock, and video pacing decisions. Clock messages carry a `RenderSnapshot`; synchronization rejects a different project or revision while allowing audio and video to request different time ranges. The callback consumes preallocated samples without waiting or locking; underruns emit silence and still advance device time. Phase 8E uses `cpal` 0.18.1 inside `or_audio`, never as a direct `or_core` dependency. Core gain, pan, fades, and later DSP belong in the audio engine rather than Flutter widgets.

### Preview bridge

Phase 7F0 locks a shared viewer boundary for all platforms; 7F1 proves its
desktop native adapter and FFmpeg package/link/load path on macOS, Linux, and
Windows. `or_render`/wgpu
produces rendered output; `or_runtime` supplies immutable snapshot identity,
frame metadata, a bounded latest-frame mailbox, and `FrameLease` lifetime,
cancellation, and release behavior. A narrow native platform adapter owns
Flutter external-texture registration, GPU resources, synchronization, and
texture lifetime. Flutter sends playback, seek, scrub, and frame-step controls
through the app/bridge path and receives structured transport state plus an
opaque registered texture identifier. A bounded BGRA8888 pixel-buffer
presentation path with premultiplied alpha is required on supported desktop
targets unless platform API evidence requires RGBA8888 for an adapter. Shared
GPU surfaces remain an optional fast path.

The Flutter/Dart and IPC paths must never carry per-frame pixels, full-rate
frame bytes, or raw OS/GPU handles. Retain each `FrameLease` until the native
release callback or completion fence signals. The platform adapter uses a
bounded latest-frame mailbox/in-flight set and rejects frames from stale
generations or project revisions. 7F1 proves the adapter builds and is
packaged; the product viewer and playback behavior remain future 7F work.

### Project, cache, and jobs

The native project is a versioned, structured `.orproj` document with stable IDs, external media references, and migrations. Current schema v7 persists the ordered library of `MediaItem` values, an ordered `ProjectTimeline` of tracks and clips, bounded global markers, a required nullable exact sequence frame rate, typed media/text/caption content, and persistent typed track/clip settings. V1–v4 leave the sequence rate unset; v5 preserves its nullable rate and migrates media clips with exact duration and default typed state/settings. Clean open preserves disk bytes, and explicit save converts to v7 without a conversion-only revision increment. V2 media entries retain UUIDv4 `MediaId`, validated local `file:` URIs (maximum 8,192 bytes), and bounded `MediaMetadata`; schema v7 adds the strict `FileUri` / `AndroidSafDocumentUri` source union without persisting permission grants or native handles. Media bytes do not enter the project. Project data is canonical; thumbnails, waveforms, proxies, render intermediates, and indexes are disposable cache data. Phase 5C provides the thumbnail and waveform namespaces; Phase 5D adds bounded source-fingerprint v1, fixed generation profiles, and cached PNG previews for the Media panel. Phase 5E adds a separate disposable SQLite index v1, reconciles recognized Phase 5D files and repairs stale or drifted rows, and applies persistent sequence-based LRU eviction only when needed to satisfy the global artifact budget. The index stores only kind, opaque cache key, byte size, and access sequence; it stores no source path or project identity and does not count against the artifact budget. SQLite metadata and filesystem changes are not one atomic transaction; startup reconciliation and targeted cache repair rebuild or correct mismatches. V1 loading preserves identity, revision, and name and supplies empty media and timeline state; v2 and v3 loading preserve the existing project state and supply empty marker state. V2 rejects duplicate media IDs and duplicate source URIs and preserves insertion order. Cache data can be regenerated and is never required for project correctness.

The Phase 8A contract starts from the verified schema-v5 baseline and increments the project schema exactly once. It keeps one Rust-owned timeline with closed Video,
Audio, Text, and Caption track kinds and closed Media, Text, and Caption clip
content. Stable clip IDs, exact start/duration, and lossless media source ranges
remain canonical; text and caption clips have no fake media IDs or ranges.
Typed track flags, transforms, crop, opacity, text formatting, audio controls,
and basic transition/effect references are part of that model. UI selection and
viewport state remain presentation-only. Playback speed remains 1x through
Desktop MVP; Phase 13B owns speed mapping.

Checkpoint 8C applies the existing schema-v6 visual settings through the shared
`timeline.clip.update` command. The desktop Inspector reads and updates bounded
fixed-point transform, crop, and opacity values while retaining exact project
values. The preview queries typed timeline-v2 clips and applies transform,
anchor, crop, and opacity in the wgpu layer pipeline; GPU uniforms and textures
remain runtime-only. The Flutter timeline keeps its media-only clip read model
for editing, with visual settings read through the typed v2 bridge path.

Phase 5A defines `JobId`, `JobKind::MediaProbe`, and the `Queued`, `Running`, `Succeeded`, `Failed`, and `Cancelled` states as a small shared job boundary. Public media probing still runs synchronously. Phase 5C adds a bounded `JobManager` over Rust standard-library threads: callers pass an explicit non-zero worker, queue, and record configuration; the manager creates exactly that many worker threads and never one per job; submission is non-blocking and returns a structured backpressure error when the bounded pending queue is full; tracked records are bounded and only terminal records are reclaimed oldest-first by a manager-local sequence; cancellation is cooperative for queued and running jobs; a panicking task is contained and cannot kill the pool; and shutdown stops submissions, skips queued work, signals running work, and joins workers. Phase 5D adds concrete thumbnail and waveform job kinds. A `MediaArtifactService` uses `JobManagerConfig(2, 32, 128)` and `CacheStoreConfig(8 MiB, 256 MiB)` in the desktop bridge. It returns `Ready`, `Queued`, `Running`, `NotApplicable`, or `Failed` request states; queue/record pressure is surfaced, identical in-flight cache keys share work, terminal artifact events carry an independent monotonic sequence, and cancellation/close kills and reaps the child process before workers join. The service does not mutate project state. It is not a realtime media scheduler and targets no playback or per-frame work. There is still no job progress API, job persistence, or priority system. Playback-critical decode, audio, and render work must eventually take priority over opportunistic work such as thumbnail and waveform generation, proxy creation, and AI analysis. Workers do not mutate canonical project state; results that affect a project must return through validated application commands.

### Local IPC and platform boundary

Phase 4F implements protocol v1 over Unix-domain sockets on macOS/Linux and Windows named pipes. Requests use a four-byte big-endian length prefix with a 1 MiB limit, strict versioned JSON envelopes, a request ID, and a random per-server authentication token in an explicit endpoint descriptor. Each connection carries one request. Unix runtime directories, sockets, and descriptors have owner-only permissions. The Windows runtime directory, descriptor file, and named pipe use protected owner-only DACLs, and the pipe rejects remote clients. There is no TCP, HTTP, WebSocket, or network fallback. The server exposes describe, shared application requests, explicit save, and guarded shutdown; it does not accept arbitrary paths or shell commands. This transport is intended for local same-user automation and does not establish a boundary against malicious code running as that same OS user.

Both the Flutter application and developer/headless `or session serve` can host a session. The app's opaque bridge handle and IPC worker share exactly one `ProjectFileSession`; protocol v1 is unchanged. On macOS the app places its private runtime directory in the sandbox-provided temporary directory and the packaged app has the local server entitlement; the application code still binds only a Unix-domain socket. The current 9A implementation opens and creates Android projects through SAF, stores each active project in an app-private canonical working copy, and explicitly synchronizes after local save or recovery. The Rust media model accepts typed `FileUri` and `AndroidSafDocumentUri` sources; SAF references are never converted into filesystem paths, and seekable media access is a runtime-only owned-descriptor capability. Hosted evidence is still required before the checkpoint can advance. Phase 9A0 proves software FFmpeg 8.1.3 package/link/load support for `arm64-v8a`, `armeabi-v7a`, and `x86_64`, including the hosted x86_64 emulator path.

The domain model should avoid platform lock-in while matching current product targets: macOS, Windows, Linux, and Android. iOS and web are not current release targets.

### Secrets and trust

OS secure storage is the intended home for provider credentials. The application may report whether a provider is configured, but CLI and agents never receive plaintext stored secrets. Network permissions and Offline Mode are enforced at the application/provider boundary, below individual UI controls, so GUI, CLI, and agents cannot bypass them. Imported projects, media, subtitles, models, templates, themes, community content, plugin output, and AI output are untrusted inputs and must be validated at each boundary.

## Future directions

Local and optional cloud AI providers, declarative templates, themes, and
MotionScenes, a GitHub-first static community registry, and sandboxed plugins
are future extensions. Early community distribution does not require an
OR-hosted backend. Phase 10A freezes the provider manager, bounded local
sidecar protocol, model-manifest contract, and typed Unavailable outcome; no
model weights or cloud credentials are required for roadmap completion. Phase
13C freezes SDR Rec.709/sRGB delivery assumptions, linear-light working space,
and premultiplied-alpha compositing; HDR remains deferred. Phase 16B uses
`wasmi` 1.1.0 in a dedicated capability-limited runtime without default WASI;
Phase 16E uses bounded Rust/serde OTIO JSON plus Lottie 1.0 and dotLottie 2.0
interchange. Native and OpenFX compatibility is later and higher trust.
Arbitrary HTML/CSS/JS/Canvas/WebGL/WebGPU motion belongs only to the future
explicit, isolated, bounded WebMotion sidecar gate; it is not canonical project
data or the default renderer.

## Architecture execution lock

The machine-readable authority is [docs/execution/README.md](execution/README.md),
with permanent invariants in [ARCHITECTURE_INVARIANTS.md](execution/ARCHITECTURE_INVARIANTS.md),
the immutable checkpoint graph in [PLAN.json](execution/PLAN.json), and mutable
progress in [STATE.json](execution/STATE.json). The lock separates a semantic
control plane from a realtime runtime plane and does not claim that future
runtime crates or platform bindings already exist. The 7A `or_runtime`
contract is implemented without platform bindings; later runtime crates remain
checkpoint-gated.

The control plane is:

    Flutter / CLI / Agent
              |
       typed commands and queries
              v
          or_core
              |
       ProjectDocument + history + persistence

The runtime plane is:

    ProjectDocument -> evaluated RenderSnapshot
                    -> media decode / audio / render workers
                    -> preview viewer or export

`ProjectDocument` and `ProjectRevision` remain canonical. A `RenderSnapshot` is
an immutable, versioned evaluated view for a requested exact time/range and
revision. Per-frame playback, decode, audio, render, and export work consumes
that snapshot and never runs a project command or increments `ProjectRevision`.

`FrameDescriptor` describes a software frame, GPU texture, decoder surface, or
external/shared surface: format, dimensions, exact timestamp, and access mode.
`FrameLease` owns its lifetime and explicit release path. The bridge must not
copy full-rate decoded frames into Dart byte arrays. Native handles remain
runtime-only and are never serialized into projects, IPC, or cache identity.

The crate boundaries are checkpoint-gated: `or_core` owns domain/project/application
contracts; `or_runtime` owns the 7A capability, queue, budget, and runtime
coordination contracts; `or_media` owns linked software demux/decode/seek and
software frame/audio output; `or_render` owns the wgpu render spine and graph;
`or_audio` owns clocks, buffers, and realtime audio; and `or_ai` owns
task/provider/model boundaries. Hardware decode, the audio engine, AI, and
native interop remain checkpoint-gated.

wgpu is the shared render spine. Metal, DX12, Vulkan, CUDA, VideoToolbox,
MediaCodec, DMABUF, hardware buffers, and other native interop belong behind
runtime adapters. Apple direction is Metal/wgpu interop, VideoToolbox decode,
optional MPS compute, and Core ML/MLX through capability/provider boundaries.
NVIDIA CUDA, NVDEC, and NVENC are optional measured runtime paths; Vulkan and
DX12 remain explicit graphics/interoperability choices. Windows ML/provider
selection is capability-gated. Android uses SAF for storage, MediaCodec and
hardware-buffer paths where proven, Vulkan/wgpu for rendering, and a software
fallback; mobile AI runtime selection is likewise provider/capability based.

AI tasks are provider-independent (`Transcribe`, `Translate`, `TextToSpeech`,
`Segment`, `DetectScene`, `PlanEdit`, `GenerateImage`, `GenerateVideo`,
`GenerateMotionScene`, and `GenerateAudio`). `GenerateVideo` returns an opaque
video asset; future `GenerateMotionScene` returns an editable declarative
proposal. Providers return structured proposals, analyses, or assets
with model manifests and provenance. Normal validated application commands,
revision preconditions, and explicit permissions are required to apply a
result. Stored credentials remain outside project, agent, and CLI data.

MotionScene rendering makes zero provider/LLM calls, including no per-frame AI.
Metal, CUDA, MLX, Core ML, NVIDIA, Android acceleration, and cloud AI may
accelerate future implementations but cannot change MotionScene semantics.
