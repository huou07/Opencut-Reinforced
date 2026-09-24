# Opencut Reinforced Architecture

## Status

This is the canonical high-level architecture. It records intended boundaries; planned and future components do not imply implemented code. See [ROADMAP.md](ROADMAP.md) for phases and [TECHNICAL_PLAN.md](TECHNICAL_PLAN.md) for subsystem detail.

## Implemented today

The repository contains public-project documentation and safeguards, hygiene checks and CI, the approved [DESIGN.md](../DESIGN.md), [UX acceptance guards](UX_ACCEPTANCE.md), and the frozen [interactive HTML prototype](../prototypes/or-ui-demo.html).

There is no production Flutter application, Rust editing core, media engine, CLI, IPC service, or AI system. Prototype behavior is simulated in browser-side code and is not evidence of production architecture.

## Planned target architecture

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

### Canonical mutation and concurrency

Only the command/application execution path may mutate canonical Project state. GUI, CLI, agents, renderers, decoders, jobs, and AI workers submit validated commands or structured results and proposals; none writes the canonical project directly. Each active project exposes a monotonically increasing revision. A successful transaction increments it once; reads and failed or rolled-back work do not. A client acting on inspected state supplies its expected revision, and stale work is rejected for re-query and revalidation rather than silently applied.

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

The render core evaluates timeline state into sources, transforms, effects, compositing, color, and output. Preview and export use the same edit semantics, while scheduling and quality may differ; cross-GPU pixels are not required to be bit-identical. wgpu is the preferred GPU abstraction candidate; backend support and performance must be checked on every target platform.

Audio decoding belongs in the media layer. A low-latency output abstraction and an audio playback clock are planned. Core gain, pan, fades, and later DSP belong in the audio engine rather than Flutter widgets.

### Preview bridge

Rust and wgpu are intended to own rendered preview frames. Flutter should eventually consume a native or external texture handle, with platform-specific fast paths and a correctness fallback. Decoded real-time video frames must not travel through the ordinary Dart/Rust message bridge as copied objects.

### Project, cache, and jobs

The native project is a versioned, structured .orproj document with stable IDs, external media references, and migrations. Project data is canonical; thumbnails, waveforms, proxies, render intermediates, and indexes are disposable cache data.

A shared background Job Manager is planned for thumbnails, waveforms, proxies, transcription, translation, AI work, model and asset downloads, and export. Jobs report progress and structured results or errors, support cancellation, and support pause and priority where appropriate. Workers do not mutate canonical project state; results that affect a project return through validated application commands.

### Local IPC and platform boundary

IPC is local by default: Unix domain sockets on Unix-like desktop platforms and a named-pipe equivalent on Windows. OR must not listen on a public network port by default. Android uses platform storage integration, including the Storage Access Framework where appropriate, through a platform storage abstraction.

The domain model should avoid platform lock-in while matching current product targets: macOS, Windows, Linux, and Android. iOS and web are not current release targets.

### Secrets and trust

OS secure storage is the intended home for provider credentials. The application may report whether a provider is configured, but CLI and agents never receive plaintext stored secrets. Network permissions and Offline Mode are enforced at the application/provider boundary, below individual UI controls, so GUI, CLI, and agents cannot bypass them. Imported projects, media, subtitles, models, templates, themes, community content, plugin output, and AI output are untrusted inputs and must be validated at each boundary.

## Future directions

Local and optional cloud AI providers, declarative templates and themes, a GitHub-first static community registry, and sandboxed plugins are future extensions. Early community distribution does not require an OR-hosted backend. A WASM/WASI-style plugin sandbox is only a candidate until plugin work starts and security research is refreshed. Native and OpenFX compatibility is later and higher trust.
