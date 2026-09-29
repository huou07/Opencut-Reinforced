# Opencut Reinforced Architecture

## Status

This is the canonical high-level architecture. The executable bootstrap,
project/application, Flutter shell, media, and timeline foundations described
below are implemented through the completed Phase 6 timeline foundation. The
Flutter application creates and opens real projects, owns one Rust live host
shared with local IPC, presents explicit recovery and dirty-state workflows,
and provides desktop media-library import/list/remove plus read-only generated
thumbnail and waveform previews. `.orproj` schema v4 persists validated media
references, metadata, canonical ordered timeline state, and global point
markers while still loading v1, v2, and v3. Phase 5D–5F provide bounded
fingerprints, disposable indexed cache, and core-only file-backed proxies.
Phase 6 provides canonical tracks, clips, exact editing, persistent markers,
marker-aware snapping, CLI parity, and the corresponding Flutter UI without a
second editable UI state. Phase 7A adds the dependency-free `or_runtime`
contract for immutable render-snapshot identity, exact-time frame metadata and
leases, bounded cancellation-aware queues, separate render/audio/decode
budgets, and centralized capability selection with software fallback. Visible
playback, decoding, and export remain future Phase 7/8 work; 7B adds the
headless `or_render` wgpu spine without a product viewer. For authoritative current checkpoint and phase status,
see [docs/execution/STATE.json](execution/STATE.json); this document does not
copy mutable `NEXT` state. Planned and future components below do not imply
implemented code. See [ROADMAP.md](ROADMAP.md) and
[TECHNICAL_PLAN.md](TECHNICAL_PLAN.md) for human-readable design detail.

## Implemented today

The repository contains a minimal Rust workspace with `or_core`, `or_ipc`, `or_runtime`, `or_render`, a semantic `or` CLI, and the native Flutter application at `apps/or_app`. `or_core` provides application info, health, and capability discovery; Phase 4A values for project identity, runtime project-instance identity, project revision, exact rational time and rate, and time ranges; a `ProjectDocument` with strict `.orproj` v1/v2/v3/v4 decoding, v4 encoding, and bounded filesystem load/atomic-save APIs; a separate snapshot recovery checkpoint format with bounded read, ancestry inspection, and explicit apply/discard APIs; a `ProjectSession` with static command/query catalogs, versioned envelopes, rename-only transaction groups, media/timeline/marker commands, bounded media/timeline/marker queries, Snap V1/V2 resolution, and session-local undo/redo; and Phase 5A typed `MediaId`/`JobId`, validated control-plane media metadata, and a synchronous read-only `probe_media_file(&Path)` API. The probe uses an external `ffprobe` executable and does not create a `MediaId` or touch project state. Prepared import canonicalizes and probes one selected local file outside the live-host lock, then adds its `MediaItem` through `media.add`. `ProjectSession` dispatches the transport-independent `ApplicationRequest` to implemented command, query, and transaction paths. `ProjectFileSession` owns one project path, live session, exact last-saved document, dirty state, recovery policy, and external-change-checked save. Phase 5C adds a standard-library bounded `JobManager` and disposable thumbnail/waveform `CacheStore`; Phase 5D adds bounded source-fingerprint v1 and generated PNG previews through `MediaArtifactService`; Phase 5E adds a lazy SQLite index (`cache-index.sqlite3`, schema v1) and persistent sequence-based LRU eviction; Phase 5F adds `ProxyGenerate` and file-backed Matroska proxy artifacts. Jobs, artifacts, and index metadata do not mutate canonical project state or affect `ProjectRevision`.

The semantic CLI uses that same dispatch. Headless commands open `ProjectFileSession`; changed operations save through its existing atomic, exact-base-checked path. Attached commands use the `or_ipc` client and an explicit endpoint descriptor. `LiveProjectHost` owns one shared `ProjectFileSession` behind a short-lived control-plane lock. A Rust-owned opaque `ProjectHostHandle` gives Flutter direct typed access, while the local IPC worker receives the same shared state; neither client has a second editable project model. The application host emits ordered invalidation events, and Flutter refreshes Rust read models after events rather than trusting event payloads. `or_ipc` v1 uses bounded framed JSON, per-server authentication, Unix-domain sockets on macOS/Linux, and Windows named pipes with remote clients rejected. Its `Describe`, generic `Application`, `Save`, and guarded `Shutdown` requests do not accept arbitrary file paths or shell commands. The CLI can attach to either the Flutter host or the developer/headless `or session serve` host. Flutter uses generated typed bindings from `flutter_rust_bridge` 2.13 through the `crates/or_app_bridge` adapter and `packages/or_app_bridge` Dart package.

Production Flutter implements the Focused Monochrome shell and real desktop project workflows. The native `file_selector` picker chooses project and one media file at import; Rust validates and reads or writes project data. The UI supports create/open, project summary, rename, undo/redo, explicit save, close, recovery inspection/apply/discard, dirty close/switch/exit guards, Advanced / Developer descriptor access, and a desktop Media panel with bounded listing, Load more, import, confirmed removal, generated video thumbnails, and audio-only waveform previews. Real project workspaces also present canonical track order and clip blocks from bounded Rust queries. Video/Audio tracks can be added, empty tracks removed, and clips inserted, moved, trimmed, split, explicitly deleted, or ripple-deleted through exact-rational action dialogs. Phase 6E1 adds pointer body moves, start/end trim handles, same-kind lane targeting, a default-on non-persisted Snap toggle, and a cyan drop-time guide. Pointer gestures retain canonical exact start/end values, quantize only the raw pointer delta to the nearest millisecond, and commit through the existing move/trim commands after Rust resolves a snap; the timeline remains canonical only after the command and refresh. Trim shows only the selected edge, current timeline timing, and timeline edge; split accepts an exact interior timeline point and leaves right-clip ID generation to Rust. Ripple delete confirms that only later clips on the selected track move. Flutter stores only disposable read pages, gesture-local ghosts, and dialog/selection state, never optimistic clip geometry. All canonical edits use existing command envelopes through the same Rust host; ordered project invalidation events refresh the current revision, including edits made by an attached CLI. Clip pages are limited to 100 and loaded on demand, while Flutter continues to request Snap V1 and `timeline.snap` itself scans the full canonical clip timeline. Artifact requests use a separate ordered event stream and affect only presentation state. Import requires system-provided `ffprobe`; preview generation requires system-provided `ffmpeg`. Android builds, but project create/open and the media-artifact service remain unavailable until platform storage support is implemented. The non-project Editor Shell Preview remains a layout preview. Phase 6E2A deliberately adds no marker read model, bridge method, marker ruler, marker dialog, marker pointer interaction, or visual marker; those are Phase 6E2B, and the viewer remains `No media loaded`.

GitHub Actions checks Rust and Flutter code, runs project-storage, recovery, real local IPC, shared-host plus attached-CLI media parity tests on macOS and Windows, runs the complete Rust workspace and generated-media real-`ffprobe` plus real-`ffmpeg` artifact integrations on Linux, builds the macOS, Windows, Linux, and Android targets, and runs native Flutter bridge, project lifecycle, offline-media persistence, and timeline edit/history/save/reopen tests on macOS. Cache and file-backed proxy API tests run on Linux, macOS, and Windows; Linux also runs real Proxy V1 FFmpeg/ffprobe coverage. The current `.orproj` schema is v4 and accepts files up to 64 MiB; the strict decoder loads v1 with empty media/timeline/markers, v2 with existing media and empty timeline/markers, and v3 with existing media/tracks/clips and empty markers, preserving project ID, revision, and name. Clean opens do not rewrite older files. The next explicit save writes v4 without incrementing revision for schema conversion alone. New-project creation uses a race-safe no-clobber install. Recovery uses a separate bounded sidecar containing the exact saved base and a newer `ProjectDocument` snapshot; its v1 envelope accepts nested v1/v2/v3/v4 project snapshots, including markers in v4. Inspection is read-only, and load never applies a checkpoint automatically. `ProjectFileSession` checks recovery before opening/saving and compares the exact on-disk document with its saved base before replacement. Applying a recovery candidate revalidates the saved base and atomically saves the snapshot without incrementing its revision. A conflict does not select a winner. Session history remains in-memory; autosave and Android SAF remain unimplemented. There is no general command/query registry framework; the static catalogs contain only the implemented operations. The schema-v1 cache index covers disposable thumbnail, waveform, and proxy artifacts under one global budget; proxy artifacts are file-backed and are never read wholly into memory. The preview service does not alter the frozen HTML prototype, `.orproj` schema, IPC protocol version, or canonical project state. Prototype behavior is simulated in browser-side code and is not evidence of production architecture. The Phase 7A `or_runtime` contract is Rust-only and is not connected to Flutter playback.

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

Only the command/application execution path may mutate canonical Project state. Phase 4D implements `project.rename`, `history.undo`, `history.redo`, and atomic groups containing one or more `project.rename` calls. Phase 5B adds non-transactional `media.add` and `media.remove`. Commands require matching `ProjectId`, `ProjectInstanceId`, and expected `ProjectRevision`; a stale request is rejected for re-query and revalidation. Each successful media command increments revision once, while failed commands and reads do not. Each changed forward operation creates one `ChangeSet` and one session-local history entry; media history stores the exact item and original insertion index instead of full `ProjectDocument` snapshots. Undo/redo are new canonical mutations and increment revision. GUI, CLI, agents, renderers, decoders, jobs, and AI workers must not write canonical project state directly. This command path is for project edits, never a per-frame playback or render execution path.

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

FFmpeg is the intended media layer for probing, demuxing, decoding, encoding, muxing, and conversion or resampling. Its exact Rust binding and packaged configuration are undecided and require a licensing review.

Phase 5A implements metadata inspection. `or_core::probe_media_file(&Path)` accepts a local regular file and calls a system-provided `ffprobe` executable directly, without a shell. It requests a narrow JSON field set through `-show_entries`, uses `-of json`, and passes the canonical input path as the `-i` argument; `ffprobe -version` reports the executable version. The adapter has a 15-second timeout, a 1 MiB stdout limit, and a 64 KiB stderr limit, and it kills/reaps the child on timeout or output/read failure. The source path and arbitrary tags do not enter `MediaMetadata`. Phase 5B's `prepare_media_import` probes and validates one selected local file, creates a canonical local `file:` URI and fresh `MediaId`, and returns a `MediaItem`; it does not mutate a project. A caller must submit that prepared item through `media.add`. This adapter does not select a decode/render backend or link or bundle FFmpeg. Project load and `media.list` validate/display stored metadata without opening or probing source files. See the [official ffprobe documentation](https://ffmpeg.org/ffprobe.html).

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

Audio decoding belongs in the media layer. A low-latency output abstraction and an audio playback clock are planned. Core gain, pan, fades, and later DSP belong in the audio engine rather than Flutter widgets.

### Preview bridge

Rust and wgpu are intended to own rendered preview frames. Flutter should eventually consume a native or external texture handle, with platform-specific fast paths and a correctness fallback. The Flutter/Dart bridge remains a control and structured-data path; full-rate decoded video frames must not travel through ordinary Dart/Rust messages as copied objects. Frame/resource transport must support a correct fallback when external texture interoperability is unavailable.

### Project, cache, and jobs

The native project is a versioned, structured `.orproj` document with stable IDs, external media references, and migrations. Current schema v4 persists the ordered library of `MediaItem` values and an ordered `ProjectTimeline` of tracks, clips, and bounded global markers. V2 media entries retain UUIDv4 `MediaId`, validated local `file:` URIs (maximum 8,192 bytes), and bounded `MediaMetadata`. Media bytes do not enter the project. Project data is canonical; thumbnails, waveforms, proxies, render intermediates, and indexes are disposable cache data. Phase 5C provides the thumbnail and waveform namespaces; Phase 5D adds bounded source-fingerprint v1, fixed generation profiles, and cached PNG previews for the Media panel. Phase 5E adds a separate disposable SQLite index v1, reconciles recognized Phase 5D files and repairs stale or drifted rows, and applies persistent sequence-based LRU eviction only when needed to satisfy the global artifact budget. The index stores only kind, opaque cache key, byte size, and access sequence; it stores no source path or project identity and does not count against the artifact budget. SQLite metadata and filesystem changes are not one atomic transaction; startup reconciliation and targeted cache repair rebuild or correct mismatches. V1 loading preserves identity, revision, and name and supplies empty media and timeline state; v2 and v3 loading preserve the existing project state and supply empty marker state. Open alone leaves disk bytes untouched, and explicit save converts older projects to v4 without a conversion-only revision increment. V2 rejects duplicate media IDs and duplicate source URIs and preserves insertion order. Cache data can be regenerated and is never required for project correctness.

Phase 5A defines `JobId`, `JobKind::MediaProbe`, and the `Queued`, `Running`, `Succeeded`, `Failed`, and `Cancelled` states as a small shared job boundary. Public media probing still runs synchronously. Phase 5C adds a bounded `JobManager` over Rust standard-library threads: callers pass an explicit non-zero worker, queue, and record configuration; the manager creates exactly that many worker threads and never one per job; submission is non-blocking and returns a structured backpressure error when the bounded pending queue is full; tracked records are bounded and only terminal records are reclaimed oldest-first by a manager-local sequence; cancellation is cooperative for queued and running jobs; a panicking task is contained and cannot kill the pool; and shutdown stops submissions, skips queued work, signals running work, and joins workers. Phase 5D adds concrete thumbnail and waveform job kinds. A `MediaArtifactService` uses `JobManagerConfig(2, 32, 128)` and `CacheStoreConfig(8 MiB, 256 MiB)` in the desktop bridge. It returns `Ready`, `Queued`, `Running`, `NotApplicable`, or `Failed` request states; queue/record pressure is surfaced, identical in-flight cache keys share work, terminal artifact events carry an independent monotonic sequence, and cancellation/close kills and reaps the child process before workers join. The service does not mutate project state. It is not a realtime media scheduler and targets no playback or per-frame work. There is still no job progress API, job persistence, or priority system. Playback-critical decode, audio, and render work must eventually take priority over opportunistic work such as thumbnail and waveform generation, proxy creation, and AI analysis. Workers do not mutate canonical project state; results that affect a project must return through validated application commands.

### Local IPC and platform boundary

Phase 4F implements protocol v1 over Unix-domain sockets on macOS/Linux and Windows named pipes. Requests use a four-byte big-endian length prefix with a 1 MiB limit, strict versioned JSON envelopes, a request ID, and a random per-server authentication token in an explicit endpoint descriptor. Each connection carries one request. Unix runtime directories, sockets, and descriptors have owner-only permissions. The Windows runtime directory, descriptor file, and named pipe use protected owner-only DACLs, and the pipe rejects remote clients. There is no TCP, HTTP, WebSocket, or network fallback. The server exposes describe, shared application requests, explicit save, and guarded shutdown; it does not accept arbitrary paths or shell commands. This transport is intended for local same-user automation and does not establish a boundary against malicious code running as that same OS user.

Both the Flutter application and developer/headless `or session serve` can host a session. The app's opaque bridge handle and IPC worker share exactly one `ProjectFileSession`; protocol v1 is unchanged. On macOS the app places its private runtime directory in the sandbox-provided temporary directory and the packaged app has the local server entitlement; the application code still binds only a Unix-domain socket. Android project files remain unavailable until a platform storage abstraction uses the Storage Access Framework; content URIs are never passed to Rust path APIs.

The domain model should avoid platform lock-in while matching current product targets: macOS, Windows, Linux, and Android. iOS and web are not current release targets.

### Secrets and trust

OS secure storage is the intended home for provider credentials. The application may report whether a provider is configured, but CLI and agents never receive plaintext stored secrets. Network permissions and Offline Mode are enforced at the application/provider boundary, below individual UI controls, so GUI, CLI, and agents cannot bypass them. Imported projects, media, subtitles, models, templates, themes, community content, plugin output, and AI output are untrusted inputs and must be validated at each boundary.

## Future directions

Local and optional cloud AI providers, declarative templates, themes, and
MotionScenes, a GitHub-first static community registry, and sandboxed plugins
are future extensions. Early community distribution does not require an
OR-hosted backend. A WASM/WASI-style plugin sandbox is only a candidate until
plugin work starts and security research is refreshed. Native and OpenFX
compatibility is later and higher trust. Arbitrary HTML/CSS/JS/Canvas/WebGL/
WebGPU motion belongs only to the future explicit, isolated, bounded WebMotion
sidecar gate; it is not canonical project data or the default renderer.

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
coordination contracts; `or_media` owns
demux/decode/seek and software or hardware frame sources; `or_render` owns the
wgpu render spine and graph; `or_audio` owns clocks, buffers, and realtime
audio; and `or_ai` owns task/provider/model boundaries. Phase 7B creates only
`or_render`; the media, audio, AI, and native interop boundaries remain
checkpoint-gated.

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
