# Technical Plan

## Status

This document describes a planned implementation boundary, not current production code. It avoids selecting dependencies where practical until a concrete implementation can be benchmarked and checked for license, security, and platform fit.

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

## 2. Time model

Do not use floating-point seconds as canonical timeline time. Use rational or integer time suitable for video frame rates, audio sample rates, source rates, and timeline rates. A time value must carry or resolve through an explicit rate/timebase.

Conversions to display seconds are derived and non-canonical. Define overflow, rounding, frame-boundary, and invalid-rate behavior in domain tests. Timeline math must remain exact for common rational frame rates and sample positions.

## 3. Native project format

The native project is a versioned UTF-8 structured .orproj document. It is agent-readable and uses stable opaque persistent IDs for projects, tracks, clips, effects, markers, and other persistent objects. IDs must remain stable for the project lifetime, be unique in their required scope, serializable, comparable, and safe for CLI/API references; they must not encode mutable display names or array indexes. The ID representation, exact serialization syntax, and schema are not permanently selected here.

Projects reference external media. Media paths and fingerprints support relink, replace, offline state, and project collection without embedding source media by default. Cache entries never become canonical project state.

A safe-save sequence is:

1. Validate the in-memory project and target path.
2. Write a complete temporary file in the destination filesystem.
3. Flush buffered data and fsync where appropriate.
4. Atomically replace the prior project where the platform supports it.
5. Report success only after the replacement is complete.

A crash journal records recoverable changes between durable checkpoints. Startup recovery validates journal data before offering recovery. Migrations are explicit, ordered, versioned, and tested on old and malformed inputs.

### Project revisions

Canonical Project state exposes a monotonically increasing `ProjectRevision`, conceptually an unsigned integer. Each successful project-mutating transaction increments the revision exactly once, including a multi-command atomic group; read-only queries and failed or rolled-back commands do not increment it.

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

Hot UI paths use scoped queries such as timeline viewport, track list, selection inspector, media bin, and job list. Do not serialize and copy the whole project into Dart or rebuild every surface for each timeline interaction. Start with simple scoped queries and invalidation; do not introduce a reactive state framework before it is needed. The current preferred structured call and event bridge candidate is flutter_rust_bridge. It generates Flutter/Dart-to-Rust bindings and supports structured types, errors, asynchronous calls, and stream-style results. Confirm exact generator and native-build workflows on macOS, Windows, Linux, and Android when implementation starts.

Keep high-volume media transport separate from ordinary bridge messages. Do not send decoded real-time video frames as copied Dart objects.

## 10. Preview rendering and frame model

Rust and wgpu are intended to own the render graph and preview rendering. Flutter should consume a native or external texture handle when supported. Evaluate platform-specific fast paths and retain a correctness fallback. Do not copy full-resolution frames through Dart at playback frame rate.

Define CPU and GPU frame abstractions at the media/render boundary. Allow later hardware decoding and zero- or minimal-copy paths without changing timeline or project APIs. Resource ownership, synchronization, color format, and lifecycle need platform-specific prototypes before choosing the exact Flutter texture integration.

A frame should carry explicit dimensions, pixel or texture format, color information, and timing metadata. The exact representation remains implementation work.

## 11. Media and render graph

FFmpeg is the intended baseline for media probing, demux, decode, encode, mux, conversion, and resampling. The exact Rust binding is undecided. Packaged FFmpeg configuration, linked libraries, and codecs require an explicit distribution license audit.

The render evaluation order is:

    Timeline evaluation
    -> source resolution and decode
    -> transforms
    -> effects
    -> compositing
    -> color processing
    -> output frame

Preview and export share the same edit semantics: clip timing, transforms, effects, compositing, text, keyframe evaluation, and color intent. This does not require identical implementation scheduling or bit-identical pixels. Preview may use lower resolution, proxies, reduced quality, or different scheduling; export may use full-quality sources, offline evaluation, and a different encoder. Equivalent source and quality conditions must still represent the same edit.

## 12. Audio and text

Decode audio through the media layer. Define a low-latency audio output abstraction and use an audio clock as a playback synchronization master where appropriate. Core gain, pan, fades, and future DSP live in the audio engine, not in Flutter presentation code.

Final video text is rendered by the render core. Exported text must not depend on Flutter widget rendering. Select a font shaping and rendering dependency only after cross-platform behavior and licensing are evaluated.

## 13. Background jobs and cache

Use a shared Job Manager for thumbnail and waveform generation, proxy creation, transcription, translation, AI work, model and asset downloads, and export. Each job exposes stable identity, status, progress, cancellation, result or structured error, and pause or priority only where the operation supports it.

Cache keys are deterministic over source fingerprint, operation, parameters, and cache schema version. Cache storage may use a local database or index, but the exact database crate is not selected. Cache contents are disposable and never authoritative project state. Provide bounded storage and a clear-cache operation.

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

Export uses the same timeline and render evaluation as preview. The intended flow is offscreen render frames to a media encoder and muxer, managed as a background job with progress and cancellation. Codec and hardware options depend on platform support and licensing review.

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
- exact .orproj serialization syntax and persistent ID representation
- exact Flutter/native texture implementation
- exact Flutter state management framework and state-change event schema, event bus/library, and transport
- exact text shaping library
- exact audio output library
- exact SQLite or storage crate
- exact Flutter localization package and generated resource format
- exact GPU image-comparison tolerance metric
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
- [flutter_rust_bridge](https://github.com/fzyzcjy/flutter_rust_bridge) documents generated bindings, structured values, asynchronous functions, streams, errors, and platform support. It remains the preferred bridge candidate, not a locked dependency.
- [wgpu supported platforms](https://github.com/gfx-rs/wgpu#supported-platforms) lists OS and graphics-backend support, including first-class and best-effort distinctions.
- [FFmpeg license and legal considerations](https://ffmpeg.org/legal.html) describes LGPL defaults and optional GPL components. The packaged configuration determines the review required.
- [OpenTimelineIO](https://github.com/academysoftwarefoundation/opentimelineio) describes an editorial interchange format and API.
- [whisper.cpp license](https://github.com/ggml-org/whisper.cpp/blob/master/LICENSE) covers the runtime source, not every model artifact.
- [ONNX Runtime license](https://github.com/microsoft/onnxruntime/blob/main/LICENSE) covers the runtime source, not model artifacts.
- [llama.cpp license](https://github.com/ggml-org/llama.cpp/blob/master/LICENSE) covers the runtime source, not model artifacts.
