# Opencut Reinforced Architecture

## Status

This is the canonical high-level architecture. Phase 3's executable bootstrap skeleton and Phase 4A–4C project/application foundations are implemented; planned and future components below do not imply implemented code. See [ROADMAP.md](ROADMAP.md) for phases and [TECHNICAL_PLAN.md](TECHNICAL_PLAN.md) for subsystem detail.

## Implemented today

The repository contains a minimal Rust workspace with `or_core`, a semantic `or` CLI, and a Flutter shell at `apps/or_app`. `or_core` provides application info, health, and capability discovery; Phase 4A values for project identity, runtime project-instance identity, project revision, exact rational time and rate, and time ranges; a minimal `ProjectDocument` with a strict `.orproj` v1 JSON encoder/decoder; and Phase 4C's `ProjectSession`, static command/query catalogs, versioned envelopes, `project.rename`, and `project.summary`. The CLI and Flutter obtain bootstrap values from the same core. Flutter uses generated typed bindings from `flutter_rust_bridge` 2.13 through the `crates/or_app_bridge` adapter and `packages/or_app_bridge` Dart package.

GitHub Actions checks Rust and Flutter code, builds the macOS, Windows, Linux, and Android targets, and runs a macOS integration smoke that calls the native bridge and compares its results with the CLI. The project document codec is in-memory; there is no filesystem save/load or migration implementation. The Flutter shell is not an editor. There is no general command/query registry framework, ChangeSet, transaction/history layer, timeline engine, media engine, renderer, FFmpeg, wgpu, IPC service, or AI system. Prototype behavior is simulated in browser-side code and is not evidence of production architecture.

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

Only the command/application execution path may mutate canonical Project state. Phase 4C currently implements one validated `project.rename` operation; it requires matching `ProjectId`, `ProjectInstanceId`, and expected `ProjectRevision`. A real rename increments once, a same-name no-op and reads do not, and stale requests are rejected for re-query and revalidation. GUI, CLI, agents, renderers, decoders, jobs, and AI workers must not write canonical project state directly. This command path is for project edits, never a per-frame playback or render execution path.

## Planned boundaries

### Presentation and feature integration

Flutter presents the product, handles interaction, accessibility, navigation, panels, inspectors, and timeline presentation. It sends user intent through the application command boundary and renders returned state. It does not own canonical project state or exported video text. Flutter may keep navigation, panel, selection, tool, and temporary input state, plus scoped read-model caches; it does not maintain a second editable project model. Rust change/invalidation events lead Flutter to refresh affected queries. Events carry revision/order information so stale updates are ignored or requeried. Hot UI paths use scoped views rather than copying and rebuilding the whole project.

New UI features should use existing shell slots: App Bar, Editor Tool Rail, Left Tool Panel, Viewer, Inspector, Timeline Toolbar, Timeline, Task or Status Area, Command Palette, and Dialog or Mobile Sheet. A lightweight static feature descriptor may register an ID, label, icon, group, availability, command IDs, panel, inspector sections, and shortcut metadata. This is a boundary for integration, not a reason to build a runtime framework now.

Simple and Advanced modes are visibility preferences over the same state and command model. Hiding an advanced control must not remove or fork the underlying project data.

### Application and command/query APIs

The application layer owns command discovery, validation, authorization, transactions, job coordination, and structured errors. A command is validated before mutation and returns a ChangeSet. Read-only queries expose project, timeline, media, captions, command, and capability information.

The CLI is a first-class semantic client of this API. Its parity is for domain operations and meaningful project queries, not presentation-only controls. It supports headless operation and, when an application is open, attached operation over local IPC. It does not automate the UI by clicking coordinates. Agents are also clients of structured queries and commands; generated EditPlans pass the same permission and domain validation as human-initiated work.

### Rust domain

Rust is intended to own project, timeline, media identity and metadata, command behavior, history, deterministic editing operations, and render evaluation. UI widgets and agent sessions are not sources of domain truth. The first implementation should remain a small workspace; planned domains do not require empty crates.

### Media, render, and audio

FFmpeg is the intended media layer for probing, demuxing, decoding, encoding, muxing, and conversion or resampling. Its exact Rust binding and packaged configuration are undecided and require a licensing review.

The render core evaluates a versioned timeline view into sources, transforms, effects, compositing, color, and output. Preview and export use the same edit semantics, while scheduling and quality may differ; cross-GPU pixels are not required to be bit-identical. wgpu is the preferred GPU abstraction candidate; backend support and performance must be checked on every target platform.

OR's intended media policy is zero-copy where platform/backend interoperability safely permits it, and otherwise to minimize copies across hot media paths. This is not a universal zero-copy promise: software and CPU-frame fallbacks remain first-class. Future frame boundaries must be able to represent CPU frames, GPU textures, hardware-decoder surfaces, and external/shared platform surfaces without forcing hardware-decoded frames through CPU memory. Avoid a design that copies every decoded frame through Rust byte buffers, Dart objects, and a Flutter GPU upload.

The decoder boundary must support software decode and hardware-surface decode, with automatic capability-based selection and a correctness fallback. Export should prefer a GPU/native-compatible surface into a hardware encoder where supported, and otherwise use a CPU frame with a supported software or platform encoder. Hardware paths are not assumed to be faster or available for every device, codec, or format. Platform-specific interop belongs behind narrow media/render boundaries; project and timeline semantics stay platform-independent.

GPU work is a candidate for scaling, rotation, crop, color conversion where appropriate, blending, masking, compositing, color operations, and suitable effects. Project state, command validation, serialization, metadata, scheduling/orchestration, and unsuitable operations remain CPU/domain responsibilities. Profile the workload; not every operation belongs on the GPU. Preview can prioritize latency with lower resolution, proxies, reduced-quality effects, and bounded work, while export can prioritize quality and throughput. Both preserve the same timing, transform, effect, compositing, text, keyframe, and color intent.

Audio decoding belongs in the media layer. A low-latency output abstraction and an audio playback clock are planned. Core gain, pan, fades, and later DSP belong in the audio engine rather than Flutter widgets.

### Preview bridge

Rust and wgpu are intended to own rendered preview frames. Flutter should eventually consume a native or external texture handle, with platform-specific fast paths and a correctness fallback. The Flutter/Dart bridge remains a control and structured-data path; full-rate decoded video frames must not travel through ordinary Dart/Rust messages as copied objects. Frame/resource transport must support a correct fallback when external texture interoperability is unavailable.

### Project, cache, and jobs

The native project is a versioned, structured .orproj document with stable IDs, external media references, and migrations. Project data is canonical; thumbnails, waveforms, proxies, render intermediates, and indexes are disposable cache data. Future caches use deterministic keys, bounded storage and eviction, and a clear-cache operation; cache data can be regenerated and is never required for project correctness.

A shared Job Manager is planned for thumbnails, waveforms, proxies, transcription, translation, AI work, model and asset downloads, and export. Jobs report progress and structured results or errors, support cancellation, and support pause and priority where appropriate. Scheduling must use bounded concurrency, backpressure, and deliberate CPU and memory budgets; it must not create unbounded workers or queues. Playback-critical decode, audio, and render work must be able to take priority over opportunistic work such as thumbnail and waveform generation, proxy creation, and AI analysis. Stale preview work may be dropped where safe. Workers do not mutate canonical project state; results that affect a project return through validated application commands.

### Local IPC and platform boundary

IPC is local by default: Unix domain sockets on Unix-like desktop platforms and a named-pipe equivalent on Windows. OR must not listen on a public network port by default. Android uses platform storage integration, including the Storage Access Framework where appropriate, through a platform storage abstraction.

The domain model should avoid platform lock-in while matching current product targets: macOS, Windows, Linux, and Android. iOS and web are not current release targets.

### Secrets and trust

OS secure storage is the intended home for provider credentials. The application may report whether a provider is configured, but CLI and agents never receive plaintext stored secrets. Network permissions and Offline Mode are enforced at the application/provider boundary, below individual UI controls, so GUI, CLI, and agents cannot bypass them. Imported projects, media, subtitles, models, templates, themes, community content, plugin output, and AI output are untrusted inputs and must be validated at each boundary.

## Future directions

Local and optional cloud AI providers, declarative templates and themes, a GitHub-first static community registry, and sandboxed plugins are future extensions. Early community distribution does not require an OR-hosted backend. A WASM/WASI-style plugin sandbox is only a candidate until plugin work starts and security research is refreshed. Native and OpenFX compatibility is later and higher trust.
