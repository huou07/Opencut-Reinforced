# Opencut Reinforced Architecture

## Status

This is the canonical high-level architecture. Phase 3's executable bootstrap skeleton, Phase 4A–4F project/application foundations, Phase 4UI-1's Flutter visual foundation, Phase 4UI-2's desktop project lifecycle, and Phase 5A–5C media foundations are implemented. The Flutter application creates and opens real projects, owns one Rust live host shared with local IPC, presents explicit recovery and dirty-state workflows, and provides desktop media-library import/list/remove. `.orproj` schema v2 persists validated media references and metadata while still loading v1. Phase 5C adds a bounded background Job Manager and a disposable thumbnail/waveform cache foundation that are not yet integrated with the application, IPC, or CLI. Phase 5 remains in progress; actual thumbnail and waveform generation, timeline editing, decode, playback, and rendering remain future work. Planned and future components below do not imply implemented code. See [ROADMAP.md](ROADMAP.md) for phases and [TECHNICAL_PLAN.md](TECHNICAL_PLAN.md) for subsystem detail.

## Implemented today

The repository contains a minimal Rust workspace with `or_core`, `or_ipc`, a semantic `or` CLI, and the native Flutter application at `apps/or_app`. `or_core` provides application info, health, and capability discovery; Phase 4A values for project identity, runtime project-instance identity, project revision, exact rational time and rate, and time ranges; a `ProjectDocument` with strict `.orproj` v1/v2 decoding, v2 encoding, and bounded filesystem load/atomic-save APIs; a separate snapshot recovery checkpoint format with bounded read, ancestry inspection, and explicit apply/discard APIs; a `ProjectSession` with static command/query catalogs, versioned envelopes, rename-only transaction groups, media add/remove commands, paginated media query, and session-local undo/redo; and Phase 5A typed `MediaId`/`JobId`, a minimal job-kind/state contract, validated control-plane media metadata, and a synchronous read-only `probe_media_file(&Path)` API. The probe uses an external `ffprobe` executable and does not create a `MediaId` or touch project state. Prepared import canonicalizes and probes one selected local file outside the live-host lock, then adds its `MediaItem` through `media.add`. `ProjectSession` dispatches the transport-independent `ApplicationRequest` to implemented command, query, and transaction paths. `ProjectFileSession` owns one project path, live session, exact last-saved document, dirty state, recovery policy, and external-change-checked save. Phase 5C adds a standard-library bounded `JobManager` (explicit non-zero worker/queue/record bounds, a fixed worker pool, non-blocking submission backpressure, cooperative cancellation, panic containment, and deterministic shutdown) and a disposable `CacheStore` (thumbnail and waveform namespaces, a deterministic SHA-256 `CacheKey` over a schema version and source/parameters fingerprints, bounded atomic storage, and explicit remove/clear paths). Both are unintegrated infrastructure; neither mutates canonical project state nor affects `ProjectRevision`.

The semantic CLI uses that same dispatch. Headless summary and rename open `ProjectFileSession`; a changed rename uses atomic save. Attached commands use the `or_ipc` client and an explicit endpoint descriptor. `LiveProjectHost` owns one shared `ProjectFileSession` behind a short-lived control-plane lock. A Rust-owned opaque `ProjectHostHandle` gives Flutter direct typed access, while the local IPC worker receives the same shared state; neither client has a second editable project model. The application host emits ordered invalidation events, and Flutter refreshes the Rust summary after events rather than trusting event payloads. `or_ipc` v1 uses bounded framed JSON, per-server authentication, Unix-domain sockets on macOS/Linux, and Windows named pipes with remote clients rejected. Its `Describe`, `Application`, `Save`, and guarded `Shutdown` requests do not accept arbitrary file paths or shell commands. The CLI can attach to either the Flutter host or the developer/headless `or session serve` host. Flutter uses generated typed bindings from `flutter_rust_bridge` 2.13 through the `crates/or_app_bridge` adapter and `packages/or_app_bridge` Dart package.

Production Flutter implements the Focused Monochrome shell and real desktop project workflows. The native `file_selector` picker chooses project and one media file at import; Rust validates and reads or writes project data. The UI supports create/open, project summary, rename, undo/redo, explicit save, close, recovery inspection/apply/discard, dirty close/switch/exit guards, Advanced / Developer descriptor access, and a desktop Media panel with bounded listing, Load more, import, and confirmed removal. The UI keeps an immutable read model; canonical media mutations go through the gateway and Rust command path, and invalidation events refresh the query. Import requires system-provided `ffprobe`; unavailable-backend errors are shown directly. Android builds, but project create/open remain unavailable until SAF integration. The Editor Shell Preview remains a clearly non-functional workspace. The viewer remains `No media loaded`, and media does not play or enter the timeline.

GitHub Actions checks Rust and Flutter code, runs project-storage, recovery, real local IPC, shared-host plus attached-CLI media parity tests on macOS and Windows, runs the complete Rust workspace (including the Phase 5C job/cache suites) and a generated-media real-`ffprobe` integration test on Linux, builds the macOS, Windows, Linux, and Android targets, and runs native Flutter bridge, project lifecycle, and offline-media persistence tests on macOS. The current `.orproj` schema is v2 and accepts files up to 64 MiB; the strict decoder also loads v1 into a v2 in-memory document with an empty media library while preserving project ID, revision, and name. A clean v1 open does not rewrite the file. Its next explicit save writes v2 without incrementing revision for schema conversion alone. New-project creation uses a race-safe no-clobber install. Recovery uses a separate bounded sidecar containing the exact saved base and a newer `ProjectDocument` snapshot; its v1 envelope can contain either supported project schema. Inspection is read-only, and load never applies a checkpoint automatically. `ProjectFileSession` checks recovery before opening/saving and compares the exact on-disk document with its saved base before replacement. Applying a recovery candidate revalidates the saved base and atomically saves the snapshot without incrementing its revision. A conflict does not select a winner. This initial snapshot representation may evolve after real scale measurements. Session history remains in-memory; autosave and Android SAF remain unimplemented. There is no general command/query registry framework; the static catalogs contain only the implemented operations. Phase 5B does not implement thumbnails, waveforms, proxies, a cache database, a scheduler, a timeline, decoder, playback, renderer, linked or bundled FFmpeg, wgpu, TCP/network listener, or AI system. Phase 5C adds a bounded background Job Manager and a disposable thumbnail/waveform cache store as unintegrated infrastructure; it generates no thumbnails or waveforms and adds no cache database, index, or automatic eviction. Prototype behavior is simulated in browser-side code and is not evidence of production architecture.

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

The render core evaluates a versioned timeline view into sources, transforms, effects, compositing, color, and output. Preview and export use the same edit semantics, while scheduling and quality may differ; cross-GPU pixels are not required to be bit-identical. wgpu is the preferred GPU abstraction candidate; backend support and performance must be checked on every target platform.

OR's intended media policy is zero-copy where platform/backend interoperability safely permits it, and otherwise to minimize copies across hot media paths. This is not a universal zero-copy promise: software and CPU-frame fallbacks remain first-class. Future frame boundaries must be able to represent CPU frames, GPU textures, hardware-decoder surfaces, and external/shared platform surfaces without forcing hardware-decoded frames through CPU memory. Avoid a design that copies every decoded frame through Rust byte buffers, Dart objects, and a Flutter GPU upload.

The decoder boundary must support software decode and hardware-surface decode, with automatic capability-based selection and a correctness fallback. Export should prefer a GPU/native-compatible surface into a hardware encoder where supported, and otherwise use a CPU frame with a supported software or platform encoder. Hardware paths are not assumed to be faster or available for every device, codec, or format. Platform-specific interop belongs behind narrow media/render boundaries; project and timeline semantics stay platform-independent.

GPU work is a candidate for scaling, rotation, crop, color conversion where appropriate, blending, masking, compositing, color operations, and suitable effects. Project state, command validation, serialization, metadata, scheduling/orchestration, and unsuitable operations remain CPU/domain responsibilities. Profile the workload; not every operation belongs on the GPU. Preview can prioritize latency with lower resolution, proxies, reduced-quality effects, and bounded work, while export can prioritize quality and throughput. Both preserve the same timing, transform, effect, compositing, text, keyframe, and color intent.

Audio decoding belongs in the media layer. A low-latency output abstraction and an audio playback clock are planned. Core gain, pan, fades, and later DSP belong in the audio engine rather than Flutter widgets.

### Preview bridge

Rust and wgpu are intended to own rendered preview frames. Flutter should eventually consume a native or external texture handle, with platform-specific fast paths and a correctness fallback. The Flutter/Dart bridge remains a control and structured-data path; full-rate decoded video frames must not travel through ordinary Dart/Rust messages as copied objects. Frame/resource transport must support a correct fallback when external texture interoperability is unavailable.

### Project, cache, and jobs

The native project is a versioned, structured `.orproj` document with stable IDs, external media references, and migrations. Current schema v2 persists an ordered library of `MediaItem` values, each with UUIDv4 `MediaId`, a `MediaSourceRef::LocalFile` containing a validated `file:` URI (maximum 8,192 bytes), and bounded `MediaMetadata`. Media bytes do not enter the project. Project data is canonical; thumbnails, waveforms, proxies, render intermediates, and indexes are intended to be disposable cache data, and Phase 5C implements the first such store for thumbnail and waveform artifacts (deterministic keys, bounded storage, explicit clear paths) with no generator yet. V1 loading preserves identity, revision, and name and supplies an empty library; open alone leaves disk bytes untouched, and explicit save converts to v2 without a conversion-only revision increment. V2 rejects duplicate media IDs and duplicate source URIs and preserves insertion order. Future caches add automatic eviction and a database or index; cache data can be regenerated and is never required for project correctness.

Phase 5A defines `JobId`, `JobKind::MediaProbe`, and the `Queued`, `Running`, `Succeeded`, `Failed`, and `Cancelled` states as a small shared job boundary. Public media probing still runs synchronously. Phase 5C adds a bounded `JobManager` over Rust standard-library threads: callers pass an explicit non-zero worker, queue, and record configuration; the manager creates exactly that many worker threads and never one per job; submission is non-blocking and returns a structured backpressure error when the bounded pending queue is full; tracked records are bounded and only terminal records are reclaimed oldest-first by a manager-local sequence; cancellation is cooperative for queued and running jobs; a panicking task is contained and cannot kill the pool; and shutdown stops submissions, skips queued work, signals running work, and joins workers. `JobKind::MediaProbe` remains the only concrete kind; the manager is generic infrastructure. It is not a realtime media scheduler and targets no playback or per-frame work. There is still no progress/result API, job persistence, or priority system. Playback-critical decode, audio, and render work must eventually take priority over opportunistic work such as thumbnail and waveform generation, proxy creation, and AI analysis. Workers do not mutate canonical project state; results that affect a project must return through validated application commands.

### Local IPC and platform boundary

Phase 4F implements protocol v1 over Unix-domain sockets on macOS/Linux and Windows named pipes. Requests use a four-byte big-endian length prefix with a 1 MiB limit, strict versioned JSON envelopes, a request ID, and a random per-server authentication token in an explicit endpoint descriptor. Each connection carries one request. Unix runtime directories, sockets, and descriptors have owner-only permissions. The Windows runtime directory, descriptor file, and named pipe use protected owner-only DACLs, and the pipe rejects remote clients. There is no TCP, HTTP, WebSocket, or network fallback. The server exposes describe, shared application requests, explicit save, and guarded shutdown; it does not accept arbitrary paths or shell commands. This transport is intended for local same-user automation and does not establish a boundary against malicious code running as that same OS user.

Both the Flutter application and developer/headless `or session serve` can host a session. The app's opaque bridge handle and IPC worker share exactly one `ProjectFileSession`; protocol v1 is unchanged. On macOS the app places its private runtime directory in the sandbox-provided temporary directory and the packaged app has the local server entitlement; the application code still binds only a Unix-domain socket. Android project files remain unavailable until a platform storage abstraction uses the Storage Access Framework; content URIs are never passed to Rust path APIs.

The domain model should avoid platform lock-in while matching current product targets: macOS, Windows, Linux, and Android. iOS and web are not current release targets.

### Secrets and trust

OS secure storage is the intended home for provider credentials. The application may report whether a provider is configured, but CLI and agents never receive plaintext stored secrets. Network permissions and Offline Mode are enforced at the application/provider boundary, below individual UI controls, so GUI, CLI, and agents cannot bypass them. Imported projects, media, subtitles, models, templates, themes, community content, plugin output, and AI output are untrusted inputs and must be validated at each boundary.

## Future directions

Local and optional cloud AI providers, declarative templates and themes, a GitHub-first static community registry, and sandboxed plugins are future extensions. Early community distribution does not require an OR-hosted backend. A WASM/WASI-style plugin sandbox is only a candidate until plugin work starts and security research is refreshed. Native and OpenFX compatibility is later and higher trust.
