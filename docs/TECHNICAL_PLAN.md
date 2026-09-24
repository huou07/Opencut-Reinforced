# Technical Plan

## Status

Phase 3 implemented the bootstrap subset: a Rust workspace and `or_core`, semantic CLI commands, a Flutter shell, and typed `flutter_rust_bridge` 2.13 bindings for application info, health, and capabilities. Phase 4A adds only foundational `or_core` values for exact time, project identity, runtime instance identity, and project revision. A project document, command/query system, editing, media, rendering, and automation remain unimplemented. See [ARCHITECTURE.md](ARCHITECTURE.md) for the current implementation status.

## Contents

1. [Architecture boundaries](#1-architecture-boundaries)
2. [Time model](#2-time-model)
3. [Native project format](#3-native-project-format)
4. [Command system](#4-command-system)
5. [Query system](#5-query-system)
6. [History and transactions](#6-history-and-transactions)
7. [CLI](#7-cli)
8. [Local IPC](#8-local-ipc)
9. [Flutter and Rust bridge](#9-flutter-and-rust-bridge)
10. [Preview rendering and frame model](#10-preview-rendering-and-frame-model)
11. [Media and render graph](#11-media-and-render-graph)
12. [Audio and text](#12-audio-and-text)
13. [Background jobs and cache](#13-background-jobs-and-cache)
14. [AI providers and local inference](#14-ai-providers-and-local-inference)
15. [Model management and secrets](#15-model-management-and-secrets)
16. [EditPlan and automation recipes](#16-editplan-and-automation-recipes)
17. [Templates, themes, and community](#17-templates-themes-and-community)
18. [Plugins](#18-plugins)
19. [Export and interchange](#19-export-and-interchange)
20. [UI feature registration and mobile](#20-ui-feature-registration-and-mobile)
21. [Security](#21-security)
22. [Implementation structure](#22-implementation-structure)
23. [Technical non-decisions](#23-technical-non-decisions)
24. [Upstream references](#24-upstream-references)

## 1. Architecture boundaries

Flutter is the presentation layer. Rust owns canonical project and editing state. A shared application layer exposes validated commands and read-only queries to the GUI, semantic CLI, and agent clients. Media, render, audio, jobs, and project storage connect through explicit interfaces.

No client keeps an independent editing engine. The browser prototype is not the model for production internals.

### Control/project plane and real-time media plane

The architecture has two related but distinct planes:

- **Control/project plane:** commands and queries, project mutations, undo/redo, canonical `ProjectRevision`, persistence, and timeline editing decisions. Only validated application transactions change canonical project state.
- **Real-time media plane:** transient decoded frames, audio buffers, playback clock, frame queues, render resources and GPU textures, frames in flight, render scheduling, dropped-frame decisions, and transient playback position.

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

The command registry and project transaction path must not become a per-frame playback/render path. Playback ticks, decoding, audio buffering, render evaluation, presentation, and dropped-frame decisions are runtime execution and must not create Project transactions or increment `ProjectRevision`. Only canonical project mutation changes that revision.

Timeline/render evaluation should produce a stable, versioned render-facing read view. Conceptually, canonical Project revision N is evaluated into `RenderSnapshot N` for media/render workers; after a project change, a snapshot for revision N+1 is prepared and the renderer switches safely. The renderer must not mutate canonical Project state or continuously hold a heavyweight lock on mutable Project state. Snapshot representation and granularity remain open: a full immutable evaluated structure, incremental graph, structural sharing, versioned read model, or another measured solution may be appropriate. Do not assume every edit requires cloning the entire project.

The Phase 3 application-info capability query is a bootstrap capability concept only. Future runtime capability discovery may centrally describe useful CPU architecture/features, GPU adapter/backend/limits, hardware decode and encode paths, pixel formats, and external/shared texture interoperability. Expose only details needed for pipeline selection; avoid unnecessary device fingerprinting and scattered platform checks in domain code.

## 2. Time model

Phase 4A implements `RationalTime` as exact seconds with a signed `i64` numerator and positive `u32` denominator. Fractions normalize to one canonical representation. `RationalRate` is exact units per second with positive, nonzero `u32` numerator and denominator. `RationalTime::from_units` converts integer frame or sample counts through the rate without floating point.

Canonical time is never stored as `f32` or `f64`. Rational values remain exact: there is no implicit rounding. Any future conversion from exact time to integer frames, samples, or ticks must select an explicit rounding policy. `TimeRange` enforces nonnegative duration while allowing a negative start at this low-level layer. This is a foundational time layer, not full timeline behavior.

## 3. Native project format

The native project is planned as a versioned structured .orproj document. It is agent-readable and will use stable opaque persistent IDs for projects and other persistent objects. Phase 4A selects typed UUID version 4 for `ProjectId`; it is independent of names, paths, and collection indexes. Future persistent object IDs should follow the same typed opaque-ID pattern unless evidence justifies another representation. No `TrackId`, `ClipId`, or other object IDs exist yet. Exact .orproj serialization syntax and schema remain undecided; serde support does not select JSON or another file format.

Projects reference external media. Media paths and fingerprints support relink, replace, offline state, and project collection without embedding source media by default. Cache entries never become canonical project state.

A safe-save sequence is:

1. Validate the in-memory project and target path.
2. Write a complete temporary file in the destination filesystem.
3. Flush buffered data and fsync where appropriate.
4. Atomically replace the prior project where the platform supports it.
5. Report success only after the replacement is complete.

A crash journal records recoverable changes between durable checkpoints. Startup recovery validates journal data before offering recovery. Migrations are explicit, ordered, versioned, and tested on old and malformed inputs.

### Project revisions

`ProjectRevision` is a persistent canonical project-state value backed by an unsigned 64-bit integer. Phase 4A implements its initial value, zero, and checked increment; command mutation is not implemented yet. Once canonical mutation exists, each successful mutation will increment the revision exactly once. Opening/loading or saving without a canonical mutation does not increment it. Project ID and revision survive save/reopen; each fresh runtime open receives a new ephemeral `ProjectInstanceId`, which is not part of the future project document.

Future live mutation preconditions conceptually identify state by `ProjectId` + `ProjectInstanceId` + `ProjectRevision`. This distinguishes a stale client attached to a previous runtime session even when the same project reopens at the same revision. The command wire schema remains deferred. Revision overflow is checked and must never wrap. Restoring older snapshot content through OR is a new mutation: at current revision 100, restoring content captured at revision 20 results in revision 101, not 20.

## 4. Command system

A Command Registry describes stable command IDs, schema versions, arguments, target IDs, preconditions, permissions, and availability. A Command Envelope contains:

- command ID
- schema version
- arguments
- target IDs
- preconditions
- expected project revision (conceptually `expected_project_revision`) when the operation depends on previously inspected state; exact wire/schema naming is not frozen

Only the command/application execution path may mutate canonical Project state. Flutter widgets, CLI presentation code, agents, render workers, media decoders, background jobs, AI workers, and provider adapters may submit commands, results, proposals, events, generated assets, or analysis, but must not directly mutate the canonical project.

Validate shape, permissions, object existence, revision preconditions, and domain invariants before mutation. Apply a valid command as a transaction and return a structured result and ChangeSet. If the expected project revision is stale, reject with a stable `REVISION_CONFLICT` error and make no change. Attached CLI commands, agent EditPlans, long-running UI workflows, and background analysis proposals use this protection when based on inspected state. The caller must query current state and revalidate, dry-run again, or regenerate its proposal; it must not silently apply an old plan to new state. A caller already holding the active mutation transaction need not redundantly provide this precondition for every internal operation. Errors are machine-readable and do not leak secrets or sensitive file contents.

Commands can be grouped atomically. An agent's multi-command edit can therefore preview and apply as one undoable transaction.

## 5. Query system

Queries are read-only and return structured data. Initial query families cover project summary and metadata, timeline and selection inspection, media and offline state, captions, supported commands, and capabilities.

Queries must not mutate state, start hidden destructive work, or return provider credentials. Output fields and schema versions are discoverable for automation clients.

## 6. History and transactions

Do not require full event sourcing. Use command transactions and ChangeSets with enough inverse information to support reliable undo and redo. Define what a command contributes to history and how a failed transaction is rolled back.

Group a multi-command agent edit into one history entry and one atomic commit; it increments the project revision once. Any successful project-mutating transaction increments the revision exactly once. Read-only queries and failed or rolled-back transactions leave it unchanged. Persistence and crash recovery do not depend on keeping an unbounded event log.

## 7. CLI

The CLI is a first-class semantic interface to the shared application and domain operations. Parity means semantic/domain operation parity for project changes and meaningful project queries, not exposure of presentation-only UI controls; see [PRODUCT.md](PRODUCT.md) for examples.

Planned contract:

- machine-readable JSON output where appropriate
- stable command and object identifiers
- stable exit-code categories
- command, query, and capability introspection
- dry-run support for destructive or complex operations
- structured errors and progress for long-running jobs
- no plaintext secrets in arguments, logs, or output

Support headless operation and attachment to an active project through local IPC. Do not implement editing by synthesizing mouse clicks, keystrokes, or screen coordinates.

## 8. Local IPC

IPC is local-only by default. Use Unix domain sockets on Unix-like desktop platforms and an equivalent named pipe on Windows. Authenticate or otherwise constrain local clients using operating-system facilities where available, validate every message, and do not open a public network listener by default.

Define protocol versioning, connection lifecycle, command timeout and cancellation, job subscriptions, and stale-client behavior before enabling external clients.

## 9. Flutter and Rust bridge

Flutter is a thin UI over the application API. Rust remains the only canonical project/timeline state. Flutter may own presentation, navigation, panel, selected-tool, temporary text/input state, and scoped cached read models/view models, but not a second authoritative editable project model.

After a command is validated and applied, Rust emits a domain change or state-invalidation event; Flutter refreshes affected scoped queries/read models and rebuilds the relevant surface. Conceptual event categories include `ProjectChanged`, `TimelineChanged`, `SelectionChanged`, `MediaChanged`, `JobChanged`, and `CapabilitiesChanged`; exact names and schema are not frozen. Events or query results carry enough project revision/order information for Flutter to ignore or requery stale state when a newer revision is known.

Hot UI paths should use scoped queries such as timeline viewport, track list, selection inspector, media bin, and job list. Do not serialize and copy the whole project into Dart or rebuild every surface for each timeline interaction. Start with simple scoped queries and invalidation; do not introduce a reactive state framework before it is needed.

The Phase 3 bootstrap uses `flutter_rust_bridge` 2.13.0 with generated typed bindings in the `packages/or_app_bridge` Dart package and a thin `crates/or_app_bridge` adapter that calls `or_core`. Its native-assets hook builds the Rust library for the consuming Flutter target. The demonstrated API is limited to app info, health, and capabilities; the future command/query/event model and media transport remain planned. CI verifies target builds and exercises the real macOS bridge.

Keep high-volume media transport separate from ordinary bridge messages. The bridge remains a control and ordinary structured-data path; do not send full-rate decoded video frames or large frame buffers as copied Dart objects. The render path should use a native/external display resource where supported and retain a correctness fallback.

## 10. Preview rendering and frame model

Rust and wgpu are intended to own the render graph and preview rendering. Flutter should consume a native or external texture handle when supported. Evaluate platform-specific fast paths and retain a correctness fallback. Do not copy full-resolution frames through Dart at playback frame rate.

Prefer zero-copy where platform/backend interoperability safely permits it; otherwise minimize copies across hot media paths. This is not a universal zero-copy promise. Avoid a forced route of decoder to CPU RGBA copy to Rust bytes to Dart bytes to Flutter GPU upload. The intended fast direction is compressed media to decoder to a CPU or hardware frame to a GPU-compatible/shared surface where available, then through the render graph to a native/external display texture.

The frame boundary must eventually represent distinct memory domains such as a CPU frame, GPU texture, hardware decoder surface, or external/shared platform surface. Do not force every hardware-decoded frame to round-trip through CPU memory. Exact frame structures are not selected here. Resource ownership, synchronization, color format, and lifecycle need platform prototypes before choosing the Flutter texture integration.

Timeline/render evaluation should publish a stable read view such as `RenderSnapshot N` for canonical `ProjectRevision N`. A project mutation produces a view associated with the next revision, which workers can adopt safely. Render workers never mutate Project, and the architecture must not require them to lock mutable Project state continuously. The snapshot may be a full immutable evaluated structure, incremental graph, structurally shared data, versioned read model, or another measured strategy; snapshot granularity must be benchmarked rather than assumed to mean cloning the whole project for every edit.

Per-frame playback/render work is runtime execution over committed state. It must not dispatch project-edit commands, open Project transactions, or increment `ProjectRevision`. The exact synchronization primitive, frame queue, buffering mode, and number of frames in flight are implementation choices. Double buffering, triple buffering, or other bounded depths may suit different playback, scrubbing, paused/frame-step, export, or low-latency preview modes; measure the tradeoff among latency, throughput, memory, and GPU occupancy.

A frame should carry explicit dimensions, pixel or texture format, color information, and timing metadata. The exact representation remains implementation work.

## 11. Media and render graph

FFmpeg is the intended baseline for media probing, demux, decode, encode, mux, conversion, and resampling. The exact Rust binding is undecided. Packaged FFmpeg configuration, linked libraries, and codecs require an explicit distribution license audit.

The media boundary must support both software decode and hardware-surface decode. A centralized capability/provider layer should report usable paths for automatic selection and safe fallback; generic timeline/domain code must not accumulate platform-specific conditionals. Possible platform directions are examples only, not selections: VideoToolbox/platform video surfaces on macOS; platform hardware decode and D3D-compatible surfaces on Windows; VAAPI, Vulkan, or DMABUF-style interop on Linux; and MediaCodec with hardware-buffer or native-surface paths on Android. Support varies by codec, device, pixel format, driver, and backend.

Pipeline selection should choose the best supported and stable path for the actual codec, pixel format, resolution, backend, device, driver, platform, and operation. A hardware path is not presumed faster. Prefer hardware decode and minimal-copy GPU processing where they benefit the workload, and preserve software/CPU paths as correctness fallbacks when decode, GPU interop, or drivers are unavailable or unstable.

The render evaluation order is:

    Timeline evaluation
    -> source resolution and decode
    -> transforms
    -> effects
    -> compositing
    -> color processing
    -> output frame

Preview and export share the same edit semantics: clip timing, transforms, effects, compositing, text, keyframe evaluation, and color intent. This does not require identical implementation scheduling or bit-identical pixels. Preview may use lower resolution, proxies, reduced quality, or different scheduling; export may use full-quality sources, offline evaluation, and a different encoder. Equivalent source and quality conditions must still represent the same edit.

GPU candidates include scaling, rotation, crop, appropriate color-space conversion, blending, masking, compositing, color operations, and suitable effects. Project state, command validation, serialization, metadata, scheduling/orchestration, and work unsuited to a GPU remain CPU/domain responsibilities. Profile by operation; there is no requirement that every operation run on the GPU. Export should prefer render output in a GPU/native-compatible surface to a hardware encoder when supported, avoiding GPU readback followed by upload when interop allows. Otherwise use a CPU frame and a supported software or platform encoder. Exact hardware encoder APIs and FFmpeg hardware-frame integration remain undecided.

Use optimized upstream implementations and compiler auto-vectorization before considering architecture-specific intrinsics or custom SIMD. ARM NEON or x86 SIMD paths may be evaluated behind tested abstractions only after profiling identifies a meaningful bottleneck.

## 12. Audio and text

Decode audio through the media layer. Define a low-latency audio output abstraction and use an audio clock as a playback synchronization master where appropriate. Core gain, pan, fades, and future DSP live in the audio engine, not in Flutter presentation code.

The future audio output callback/realtime path should avoid network calls, Flutter calls, JSON parsing, heavy locks, unnecessary allocation, Project mutation, and blocking background jobs. A conceptual direction is decode/resample to a bounded audio buffer or ring, then low-latency audio output and a playback clock. Video presentation synchronizes to that clock where appropriate. Exact audio library and buffer design are not selected.

Final video text is rendered by the render core. Exported text must not depend on Flutter widget rendering. Select a font shaping and rendering dependency only after cross-platform behavior and licensing are evaluated.

## 13. Background jobs and cache

Use a shared Job Manager for thumbnail and waveform generation, proxy creation, transcription, translation, AI work, model and asset downloads, and export. Each job exposes stable identity, status, progress, cancellation, result or structured error, and pause or priority only where the operation supports it.

Scheduling must support bounded concurrency, cancellation, backpressure, and deliberate CPU and memory budgets without unbounded worker creation. Keep conceptual classes for latency-sensitive realtime work (audio output, immediately needed playback decode, render/present), interactive work (commands, timeline queries, scrubbing, inspector updates), and background work (thumbnails, waveforms, proxies, indexing, AI analysis, downloads). Playback-critical work must be able to take priority over opportunistic background work, and AI/background work must not starve playback; exact priority names and implementation are not fixed.

Producers must not outrun consumers indefinitely. Frame/decode and export queues, thumbnail work, and AI/background work must stay bounded. When a consumer is slower, the system may pause production, reduce queue depth, drop obsolete preview work where safe, or block a background producer appropriately. The exact queue type and policy are workload decisions.

Hot media paths should avoid repeated large allocations where practical. Bounded frame, texture, audio-buffer, decode-surface, and render-target reuse are candidates, not selected implementations; pools must remain bounded and must not turn into an unbounded cache. Establish system-level budgets for RAM, GPU memory or equivalent render resources, decoded-frame cache, thumbnail cache, waveform cache, and proxy/cache storage. Budgets may vary by device class, available memory, platform, and workload, with more conservative policy on Android; fixed percentages and values are deferred.

Cache keys are deterministic over source fingerprint, operation, parameters, and cache schema version. Cache storage may use a local database or index, but the exact database crate is not selected. Cache contents are disposable and never authoritative project state. Provide bounded storage, eviction and regeneration paths, and a clear-cache operation; cache presence must never be required for project correctness.

Workers may produce structured `JobResult`, `GeneratedAsset`, `AnalysisResult`, `CaptionProposal`, or `EditProposal` outputs. They may update disposable cache/job state but must not directly mutate the canonical ProjectDocument. If an output should change a project, the application layer checks its permissions and expected revision/preconditions, then applies it through a validated command/transaction. This preserves undo/redo and rejects stale analysis instead of applying it silently.

## 14. AI providers and local inference

Define capability-oriented adapters for transcription, translation, text-to-speech, LLM planning, segmentation, image generation, video generation, and audio generation. Local and optional cloud implementations plug into the same task-oriented contracts. Do not hard-code an editor workflow to one provider.

Candidate runtime categories for evaluation:

- whisper.cpp for local automatic speech recognition
- ONNX Runtime for suitable specialized models
- a llama.cpp-compatible provider for local language-model inference

These are candidates, not required MVP components or final dependency decisions. Keep heavyweight Python-centric generation stacks behind a supervised sidecar or provider boundary rather than making them core Rust dependencies by default.

Runtime code licenses do not establish the license or redistribution rights for a model's weights, tokenizer, or associated assets. Verify each exact model artifact independently.

Network-capable providers/services declare whether a task is local-only or requires network access; those are conceptual capability categories, not frozen enum/API names. Provider calls go through an application-controlled permission boundary. Offline Mode is enforced below UI controls, so GUI, CLI, and agent clients cannot bypass it. Cloud tasks receive only the minimum project-derived context needed for that task; do not serialize the whole project into prompts by default.

## 15. Model management and secrets

A model manifest records ID, version, task, source, cryptographic hash, size, runtime, hardware needs, language coverage, license, and install state. Verify SHA-256 before activation. Store weights outside the repository and do not bundle them by default.

Store user provider secrets in operating-system secure storage. Internal provider calls may access a credential through a narrowly scoped application service. Agent and CLI interfaces expose only configured or not-configured state and never return plaintext stored credentials. Do not log request headers or secret-bearing configuration.

## 16. EditPlan and automation recipes

The planned agent edit flow is:

    User prompt
    -> agent proposal
    -> strict EditPlan schema validation
    -> permission validation
    -> domain validation
    -> dry run
    -> ChangeSet and human-readable diff
    -> explicit apply as one transaction

Treat all model output as untrusted input. Refuse unknown commands, target IDs, fields, and unsupported operations. Applying an EditPlan uses the same command registry and checks as GUI and CLI actions.

Automation recipes are declarative OR command sequences with schema and permission validation. They do not execute arbitrary shell commands.

## 17. Templates, themes, and community

Templates are declarative packages with stable editable slot IDs, dependency manifests, checksums, and license metadata. They contain project structure and values, not arbitrary executable code.

Themes are declarative semantic token sets. They cannot include JavaScript or arbitrary CSS, execute code, redefine application behavior, or replace the layout architecture.

Initial community distribution can use a GitHub-first static registry: manifest repository, versioned entries, release or download assets, automated validation, and pull-request-based publishing. Do not build a community backend now; early phases do not require one.

## 18. Plugins

Prefer a sandbox and explicit capability permissions. A WASM/WASI-style runtime is a candidate for future plugin work, not a permanent selection. Refresh runtime support, escape analysis, permissions, and dependency research before plugin implementation.

Native and OpenFX compatibility is later and has a higher trust cost. Do not treat installed plugins as unrestricted trusted code by default.

## 19. Export and interchange

Export uses the same timeline and render evaluation as preview. The intended flow is offscreen render frames to a media encoder and muxer, managed as a background job with progress and cancellation. Where supported, prefer a GPU/native-compatible render surface into a hardware encoder; otherwise use CPU frames with a supported software or platform encoder. Do not require a GPU readback/upload cycle when a stable shared-surface path is available. Codec and hardware options depend on platform support and licensing review, and correctness fallback remains first-class.

The native OR format is not OpenTimelineIO. OTIO is an import/export interchange format and API for editorial cut information, not the native project database and not a media container. Select adapters and supported OTIO fields when an interchange implementation is scoped.

## 20. UI feature registration and mobile

Production Flutter may use static feature descriptors containing ID, label, icon, group, availability, command IDs, panel, inspector sections, and shortcut metadata. Keep registration lightweight; do not build a speculative plugin framework around it.

Stable shell slots include App Bar, Editor Tool Rail, Left Tool Panel, Viewer, Inspector, Timeline Toolbar, Timeline, Task or Status Area, Dialog or Mobile Sheet, and Command Palette. Integrate new work in an existing slot unless a permanent new region is justified and reviewed.

Simple and Advanced modes are visibility settings over one state and command model.

Android uses the same Rust core and project model with touch-native Flutter presentation, Android Storage Access Framework or platform storage abstraction, and mobile-appropriate resource and proxy policies.

Visible Flutter strings and accessibility labels use a localization-capable resource boundary when production UI work begins. Do not scatter user-facing English strings through domain/business logic. Human-readable errors may be localized at the presentation boundary; command IDs, JSON field names, and machine-readable error codes remain stable technical identifiers. Do not select a localization package or generate localization files in this planning phase.

## 21. Security

Treat project files, media, subtitles, templates, themes, downloaded assets, models, plugin output, and agent or AI output as untrusted. Validate input at every serialization, IPC, plugin, model, and community boundary. Enforce limits for file sizes, dimensions, durations, archive expansion, and job resources before implementation exposes those inputs.

Keep secrets out of logs, project files, CLI output, and agent context. Require explicit capabilities for plugins and community actions. Security and licensing constraints are part of feature design, not follow-up cleanup.

Provider network capability and permission are enforced centrally by the application, including when a request originates from CLI or an agent. Offline Mode denies OR-originated optional network calls regardless of UI path; it does not claim to firewall the operating system. Send the minimum required data to each cloud task, without unrelated project context or secret values.

## 22. Implementation structure

Start with the smallest useful Rust workspace and Flutter shell when Phase 3 is explicitly started. Planned domains are conceptual boundaries, not a mandate to create one crate per domain. Split a module into a crate only when a concrete build, reuse, ownership, or dependency boundary justifies it. Never create empty future crates or modules.

## 23. Technical non-decisions

The following are deliberately not permanently selected:

- exact FFmpeg Rust binding
- exact .orproj serialization syntax
- exact Flutter/native texture implementation
- exact Flutter state management framework and state-change event schema, event bus/library, and transport
- exact text shaping library
- exact audio output library
- exact SQLite or storage crate
- exact Flutter localization package and generated resource format
- exact GPU image-comparison tolerance metric
- exact immutable render snapshot representation, granularity, and update strategy
- exact hardware decode API on each platform
- exact hardware encode API on each platform
- exact FFmpeg hardware-frame integration
- exact CPU, GPU, decoder-surface, and external/shared frame representation
- exact external texture/interoperability path on each platform
- exact synchronization primitive between project evaluation, media workers, GPU, and presentation
- exact frame queue depth, frames in flight, and buffering strategy for each mode
- exact worker scheduler/runtime, thread-pool implementation, and priority API
- exact frame, texture, audio-buffer, decode-surface, and render-target pool implementation
- exact RAM, GPU/resource, and cache budget values and adaptation policy
- exact performance thresholds and benchmark hardware
- exact centralized runtime hardware capability schema/provider
- exact hardware/software path selection thresholds by codec, format, device, driver, and operation
- exact CPU/GPU operation partition, based on profiling and task characteristics
- exact CPU SIMD/intrinsic implementations and feature dispatch
- exact performance instrumentation implementation
- exact audio callback buffer/ring design and buffering policy
- exact cache eviction policy
- exact translation model
- exact segmentation model
- exact text-to-speech model or runtime
- exact diffusion or video-generation runtimes
- exact WASM plugin runtime
- exact cloud provider/vendor
- exact release package formats

Choose these when the relevant phase begins, using implementation prototypes, target-platform benchmarks, security review, and license analysis. Do not pin versions here without an implementation need.

## 24. Upstream references

These official upstream references support the current candidate descriptions. Re-check them when selecting versions or packaging dependencies.

- [Flutter supported platforms](https://docs.flutter.dev/reference/supported-platforms) documents Flutter's platform matrix; OR currently selects macOS, Windows, Linux, and Android from that broader support.
- [flutter_rust_bridge](https://github.com/fzyzcjy/flutter_rust_bridge) documents generated bindings, structured values, asynchronous functions, streams, errors, and platform support. Version 2.13.0 is used by the Phase 3 bootstrap bridge; this does not settle the future command, event, or media-transport architecture.
- [wgpu supported platforms](https://github.com/gfx-rs/wgpu#supported-platforms) lists OS and graphics-backend support, including first-class and best-effort distinctions.
- [FFmpeg license and legal considerations](https://ffmpeg.org/legal.html) describes LGPL defaults and optional GPL components. The packaged configuration determines the review required.
- [OpenTimelineIO](https://github.com/academysoftwarefoundation/opentimelineio) describes an editorial interchange format and API.
- [whisper.cpp license](https://github.com/ggml-org/whisper.cpp/blob/master/LICENSE) covers the runtime source, not every model artifact.
- [ONNX Runtime license](https://github.com/microsoft/onnxruntime/blob/main/LICENSE) covers the runtime source, not model artifacts.
- [llama.cpp license](https://github.com/ggml-org/llama.cpp/blob/master/LICENSE) covers the runtime source, not model artifacts.
