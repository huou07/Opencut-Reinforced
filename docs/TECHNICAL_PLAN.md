# Technical Plan

## Status

Phase 3 implemented the bootstrap subset: a Rust workspace and `or_core`, semantic CLI commands, a Flutter shell, and typed `flutter_rust_bridge` 2.13 bindings for application info, health, and capabilities. Phase 4A adds foundational `or_core` values for exact time, project identity, runtime instance identity, and project revision. Phase 4B adds a minimal `ProjectDocument` and strict `.orproj` v1 JSON codec. Phase 4C adds `ProjectSession`, static command/query catalogs and versioned envelopes, `project.rename` v1, and `project.summary` v1. Phase 4D adds rename-only atomic transaction groups, normalized `ChangeSet` results, and in-memory session-local undo/redo. Phase 4E1 adds bounded filesystem load, atomic save, and race-safe no-clobber creation. Phase 4E2 adds a separate snapshot recovery checkpoint sidecar, exact saved-base validation, bounded strict inspection, and explicit apply/discard. Phase 4F adds shared application dispatch, exact-base file sessions, local IPC v1, and headless/attached semantic CLI operations. Phase 4UI-2 connects the Flutter desktop project lifecycle to one Rust-owned `LiveProjectHost` shared by its typed bridge handle and authenticated local IPC. It adds explicit recovery, dirty-state and exit guards, and event-driven read-model refresh. Phase 5A adds typed media/job identities, structured metadata, a bounded external `ffprobe` metadata adapter, and read-only CLI inspection. Phase 5 remains in progress: media import/persistence, Android SAF, autosave, migrations, timeline editing, decode, playback, and rendering remain unimplemented. See [ARCHITECTURE.md](ARCHITECTURE.md) for the current implementation status.

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

- **Control/project plane:** commands and queries, project mutations, undo/redo, canonical `ProjectRevision`, persistence, and timeline editing decisions. Only validated application commands mutate canonical project state; the current in-memory transaction path groups rename commands only.
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

The command catalog and project transaction path must not become a per-frame playback/render path. Playback ticks, decoding, audio buffering, render evaluation, presentation, and dropped-frame decisions are runtime execution and must not create Project transactions or increment `ProjectRevision`. Only canonical project mutation changes that revision.

Timeline/render evaluation should produce a stable, versioned render-facing read view. Conceptually, canonical Project revision N is evaluated into `RenderSnapshot N` for media/render workers; after a project change, a snapshot for revision N+1 is prepared and the renderer switches safely. The renderer must not mutate canonical Project state or continuously hold a heavyweight lock on mutable Project state. Snapshot representation and granularity remain open: a full immutable evaluated structure, incremental graph, structural sharing, versioned read model, or another measured solution may be appropriate. Do not assume every edit requires cloning the entire project.

The Phase 3 application-info capability query is a bootstrap capability concept only. Future runtime capability discovery may centrally describe useful CPU architecture/features, GPU adapter/backend/limits, hardware decode and encode paths, pixel formats, and external/shared texture interoperability. Expose only details needed for pipeline selection; avoid unnecessary device fingerprinting and scattered platform checks in domain code.

## 2. Time model

Phase 4A implements `RationalTime` as exact seconds with a signed `i64` numerator and positive `u32` denominator. Fractions normalize to one canonical representation. `RationalRate` is exact units per second with positive, nonzero `u32` numerator and denominator. `RationalTime::from_units` converts integer frame or sample counts through the rate without floating point.

Canonical time is never stored as `f32` or `f64`. Rational values remain exact: there is no implicit rounding. Any future conversion from exact time to integer frames, samples, or ticks must select an explicit rounding policy. `TimeRange` enforces nonnegative duration while allowing a negative start at this low-level layer. This is a foundational time layer, not full timeline behavior.

## 3. Native project format

The initial `.orproj` schema v1 contract is UTF-8 JSON with this envelope:

```json
{
  "format": "opencut-reinforced-project",
  "schema_version": 1,
  "project": {
    "id": "01234567-89ab-4def-8123-456789abcdef",
    "revision": 0,
    "name": "Example Project"
  }
}
```

The `ProjectDocument` domain type stores a typed UUIDv4 `ProjectId`, persistent `ProjectRevision`, and UTF-8 name. Its fields are not the wire schema: private v1 DTOs and explicit conversion keep internal domain changes from silently changing the file contract. V1 decoding validates IDs and required fields and rejects unknown fields; the version probe rejects unsupported versions before v1 decoding. The current Rust encoder emits deterministic pretty JSON with a trailing newline for the same document; this is not a cross-implementation canonical JSON standard. `ProjectInstanceId` is runtime-only and is never persisted. Phase 4E1 wraps this codec in filesystem load/save without changing schema version 1. Migrations remain future work. Future persistent object IDs should follow the same typed opaque-ID pattern unless evidence justifies another representation. No `TrackId`, `ClipId`, or other object IDs exist yet.

Projects reference external media. Media paths and fingerprints support relink, replace, offline state, and project collection without embedding source media by default. Cache entries never become canonical project state.

A Phase 4E1 safe-save sequence is:

1. Encode the canonical `ProjectDocument` in memory and reject output larger than `MAX_PROJECT_FILE_BYTES` (64 MiB) before creating any file.
2. Create a unique hidden sibling named `.<target-name>.or-tmp-<uuid>` with `create_new(true)`, retrying only bounded name collisions. The caller's parent directory is never created automatically.
3. Write all bytes through a buffer, flush the buffer, sync the temporary file, and close it before replacement.
4. Replace the destination without deleting it first. Unix-like targets (macOS, Linux, and direct-filesystem Android paths) use same-directory `rename` and sync the containing directory. Windows uses `MoveFileExW` with `MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH`.
5. Report success only after the replacement and platform durability step. A failure to sync the Unix directory after replacement returns a durability-uncertain error; the new file may already be present.

`load_project_file` opens the file, checks its metadata size, and reads at most `MAX_PROJECT_FILE_BYTES + 1`; it rejects larger input, validates strict UTF-8, then calls the existing `decode_project`. Neither load nor save changes `ProjectId` or `ProjectRevision`. Session instance IDs and undo/redo history are not passed to the storage API and remain absent from `.orproj`.

`ProjectStorageError` distinguishes I/O, oversize input, invalid UTF-8, codec failures, temporary-file create/write/flush/sync failures, replacement failures, and post-replace durability uncertainty.

The replacement primitive provides atomic namespace/file replacement on supported local filesystems. File sync and directory sync or write-through improve durability but do not guarantee survival of every power-loss, controller, or filesystem failure. Phase 4E1 assumes one OR application/session owns a save target at a time; it does not provide a file-locking or concurrent-writer coordinator. Future migrations must be explicit, ordered, versioned, and tested on old and malformed inputs.

### Phase 4E2 snapshot recovery checkpoint

The initial recovery format is a separate versioned sidecar beside the `.orproj` file. It prefixes the project filename with `.` and appends `.or-recovery` (for example, `/projects/movie.orproj` uses `/projects/.movie.orproj.or-recovery`). Its strict envelope uses format marker `opencut-reinforced-recovery`, schema version 1, and contains two `.orproj` v1 snapshots: the exact saved base at revision N and the newer unsaved recovery snapshot at revision M, where M > N. The recovery file is bounded to 136 MiB; each nested project is decoded and validated by the existing project codec and remains subject to the 64 MiB project limit. This snapshot checkpoint is the initial pre-MVP representation and may evolve after real scale measurements.

Writing a checkpoint requires matching `ProjectId` values, a newer recovery revision, and an on-disk canonical `ProjectDocument` exactly equal to the supplied base. The write uses the same same-directory atomic replacement primitive as project saves and does not mutate the canonical project. Neither `.orproj` v1 nor its revision semantics change; runtime `ProjectInstanceId` and session history are not stored.

Inspection reads and classifies without changing files or project state. If the canonical project exactly equals base N, the recovery is a candidate. If it equals the recovery snapshot or has the same project ID with a revision newer than M, the checkpoint is stale. A different project lineage or other state that cannot prove the exact base is a conflict; a missing canonical file is an orphaned conflict. Project loading does not inspect or apply recovery automatically, and conflicts never select a winner silently.

Applying is explicit and re-inspects the current canonical file before saving. It atomically saves recovery M through the existing project storage API, keeps revision M unchanged, then removes the sidecar. A save failure preserves the checkpoint; a cleanup failure after a successful save is reported as cleanup pending. Explicit discard removes the sidecar, including a malformed one, without changing the canonical project. This is a snapshot checkpoint foundation, not event sourcing, command replay, persistent history, autosave, or a recovery UI.

### Phase 4F file-backed session and shared dispatch

`ApplicationRequest` carries an existing `CommandEnvelope`, `QueryEnvelope`, or `TransactionEnvelope`; `ApplicationResponse` carries the corresponding result or the existing `OperationError`. `ProjectSession::handle_application_request` delegates to the existing command, query, and transaction methods. This is the common semantic path for the headless CLI and IPC server; it does not introduce a second editing implementation.

`ProjectFileSession` owns a project path, a live `ProjectSession`, and the exact `ProjectDocument` last known to be saved. Opening loads the canonical `.orproj`, inspects recovery, then creates a fresh runtime instance without incrementing revision. `NONE` and `STALE` allow opening; candidate, conflict, and invalid recovery require explicit attention. Save re-inspects recovery and reloads the disk document; the save proceeds only when the disk document exactly equals the remembered saved base. It then uses the existing atomic save path and does not increment revision. Dirty state is the in-memory project compared with that exact saved document; neither dirty state, instance ID, nor history is persisted.

### Project revisions

`ProjectRevision` is a persistent canonical project-state value backed by an unsigned 64-bit integer. Phase 4A implements its initial value, zero, and checked increment; Phase 4B's v1 codec preserves the stored revision during encode/decode. A changed rename or transaction increments once; an exact no-op, net-no-op transaction, and read-only query leave the revision unchanged. Undo and redo restore content through new canonical mutations, so they increment from the current revision rather than moving it backward. Future successful canonical mutations must each increment once. Opening/loading or saving without a canonical mutation does not increment it. Project ID and revision survive save/reopen; each fresh runtime open receives a new ephemeral `ProjectInstanceId`, which is excluded from the project document.

`CommandEnvelope` v1 uses `ProjectId` + `ProjectInstanceId` + `expected_project_revision` as live mutation preconditions. This distinguishes a stale client attached to a previous runtime session even when the same project reopens at the same revision. Revision overflow is checked and reported without mutation; it must never wrap. Restoring older snapshot content through OR is a new mutation: at current revision 100, restoring content captured at revision 20 results in revision 101, not 20.

## 4. Command system

Phases 4C–4D establish a small static command catalog without a dynamic registry framework. It contains `project.rename`, `history.undo`, and `history.redo`, each schema v1; only rename is allowed inside a transaction. `CommandEnvelope` v1 has `command_id`, `schema_version`, typed `project_id`, typed `project_instance_id`, `expected_project_revision`, and `arguments`. `TransactionEnvelope` v1 has a schema version, the same three project/session preconditions, and an ordered list of strict `CommandCall` values. The envelope uses `serde_json::Value` at the structured argument boundary; dispatch immediately decodes rename arguments into a strict private typed structure. The transport-independent contracts are now carried by the bounded, strict JSON protocol in `or_ipc` v1.

The dispatcher checks command ID, schema version, project ID, project-instance ID, and expected revision before decoding arguments or mutating state; unknown envelope and argument fields are rejected. Rename preserves its supplied UTF-8 name exactly. Renaming to the same name succeeds as a no-op (`changed = false`) without incrementing revision or changing history. A real rename checks the next revision and reserves history storage before applying the paired name/revision update. Results identify the operation, session, before/after revisions, whether state changed, and the resulting `ChangeSet`.

`ProjectSession` owns a canonical `ProjectDocument` and a runtime-only `ProjectInstanceId`. It exposes read-only project access and the command/query entry points, with no public mutable document accessor. The command/application path is the only public project-mutation path. Flutter widgets, CLI presentation code, agents, render workers, media decoders, background jobs, AI workers, and provider adapters must not directly mutate the canonical project.

The runtime transaction implementation is deliberately narrow: it stages ordered `project.rename` calls, validates every child, and reduces the group to one net name change. A changed group applies the name and increments revision once, then records one `ChangeSet` in session history. A net no-op creates no revision or history entry. Any validation, precondition, overflow, or history-storage failure occurs before canonical state changes; a failed group preserves the project and both history stacks. `history.undo` and `history.redo` apply the stored name change only when the current name matches its expected side; they increment the current revision once and move the entry between in-memory session stacks. A new real edit clears redo history. History is neither serialized nor carried across a fresh `ProjectSession::open`.

Stable errors include `UNKNOWN_COMMAND`, `UNSUPPORTED_COMMAND_SCHEMA`, `UNKNOWN_QUERY`, `UNSUPPORTED_QUERY_SCHEMA`, `UNSUPPORTED_TRANSACTION_SCHEMA`, `EMPTY_TRANSACTION`, `COMMAND_NOT_ALLOWED_IN_TRANSACTION`, `PROJECT_ID_MISMATCH`, `PROJECT_INSTANCE_MISMATCH`, `REVISION_CONFLICT`, `INVALID_ARGUMENTS`, `REVISION_OVERFLOW`, `NOTHING_TO_UNDO`, `NOTHING_TO_REDO`, `HISTORY_CONFLICT`, and `HISTORY_STORAGE_FAILURE`. The CLI, local IPC client, and Flutter bridge use these same operation errors. Permissions, dry-run, generalized transaction operations, timeline edits, and agent clients remain future work.

## 5. Query system

Phase 4C's deterministic query catalog contains exactly `project.summary` schema v1. `QueryEnvelope` v1 has `query_id`, `schema_version`, typed `project_id`, typed `project_instance_id`, and `arguments`; the current query accepts exactly an empty object. The structured result reports query ID/version and a summary containing project ID, runtime instance ID, current revision, and name. It reads current canonical state and cannot mutate it or increment revision.

Unknown queries and unsupported query schemas return `UNKNOWN_QUERY` and `UNSUPPORTED_QUERY_SCHEMA`; project/session mismatches use the corresponding shared codes, and invalid arguments return `INVALID_ARGUMENTS`. Timeline, selection, media, caption, and other query families remain future work. Queries must not start hidden destructive work or return provider credentials. Output fields and schema versions are discoverable for automation clients.

## 6. History and transactions

Phase 4D implements the first in-memory transaction and history behavior without event sourcing. The current `ChangeSet` contains only a normalized before/after project-name change. A successful single rename or changed group creates one undo entry; an undo/redo is itself a new canonical mutation and increments the current revision once. New real edits clear redo history. No-op edits, failed/rolled-back groups, and reads leave project state, revision, and history unchanged. History lives only inside `ProjectSession`: it is not saved in `.orproj` and resets when the document is opened into a new session.

The transaction envelope currently accepts only `project.rename` child calls. Grouped commands stage all intermediate names and normalize them to one net `ChangeSet`; a net no-op does not increment revision or create history. This proves bounded single-operation atomic grouping, not a general transaction framework. The separate project-storage and recovery-checkpoint APIs do not depend on keeping an unbounded event log; persistent history and migrations remain future work.

## 7. CLI

The CLI is a first-class semantic interface to the shared application and domain operations. Parity means semantic/domain operation parity for project changes and meaningful project queries, not exposure of presentation-only UI controls; see [PRODUCT.md](PRODUCT.md) for examples.

Current commands preserve the original `version`, `health`, and `capabilities` output contracts and add deterministic catalog discovery:

```text
or commands [--json]
or queries [--json]
or project summary --file PATH [--json]
or project summary --attach DESCRIPTOR [--json]
or project rename --file PATH --name NAME [--json]
or project rename --attach DESCRIPTOR --name NAME [--json]
or project save --attach DESCRIPTOR [--json]
or history undo --attach DESCRIPTOR [--json]
or history redo --attach DESCRIPTOR [--json]
or recovery status --file PATH [--json]
or recovery apply --file PATH [--json]
or recovery discard --file PATH [--json]
or session serve --file PATH [--descriptor PATH] [--json]
or session describe --attach DESCRIPTOR [--json]
or session shutdown --attach DESCRIPTOR [--discard-unsaved] [--json]
```

Headless summary and rename use `ProjectFileSession` and the `project.summary` query / `project.rename` command. A changed headless rename saves immediately through exact-base checked atomic persistence; a no-op does not rewrite the file. Recovery status reports `none`, `candidate`, `stale`, or `conflict`; apply and discard call the existing recovery APIs. Unresolved candidate/conflict/invalid recovery blocks mutable file-session opening.

Attached summary, rename, undo/redo, save, describe, and shutdown require an explicit descriptor via `--attach`; there is no endpoint scanning. Attached rename uses the revision returned by describe and does not retry a stale command. It leaves the live session dirty until explicit `project save`. Undo/redo require attachment because history is session-local and not persisted. `session serve` remains a developer/headless host; the Flutter application can also host a session and exposes its descriptor under Settings → Advanced / Developer. Neither host autosaves.

JSON success responses use the core result or descriptor structures; JSON errors use stable categories and codes. Exit codes are 0 for success, 2 for usage, 3 for application operation errors, 4 for project storage/recovery/session errors, and 5 for IPC errors. Filesystem paths remain OS paths; project names must be UTF-8. No command accepts shell instructions; IPC does not expose arbitrary file reads or writes, and CLI file operations are limited to the documented project and recovery commands.

Dry-run, long-running job progress, and agent EditPlans remain future work. Do not implement editing by synthesizing mouse clicks, keystrokes, or screen coordinates.

## 8. Local IPC

Phase 4F implements `or_ipc` protocol v1 for application/control requests only. Frames contain a four-byte big-endian length and strict JSON body, bounded to 1 MiB. Each request uses a UUIDv4 ID echoed by the response; one request is processed per connection. Supported requests are `Describe`, shared `ApplicationRequest`, `Save`, and guarded `Shutdown`. The descriptor is strict and versioned and contains the endpoint, project/runtime IDs, and a random per-server token. Authentication and protocol checks happen before project disclosure or dispatch.

macOS/Linux use Unix-domain sockets in a server-created private runtime directory, with 0700 directory and 0600 socket/descriptor permissions. Sandboxed macOS builds place this directory in the app-provided temporary directory and include the local server entitlement; the app code creates no TCP listener. Windows uses a named pipe configured to reject remote clients; its runtime directory, descriptor file, and pipe have protected owner-only DACLs. There is no TCP, HTTP, WebSocket, LAN listener, or fallback. The token is not printed or logged; descriptor access is the client credential. This is a same-user local automation boundary, not isolation from malicious processes running as the same OS user. `LiveProjectHost` owns one `ProjectFileSession`, serializes direct bridge and IPC requests through the same control-plane state, and exposes no arbitrary path or shell operation. The Flutter application and developer/headless `or session serve` command can each host one live project. Neither autosaves.

Timeout/cancellation, subscriptions, multiple simultaneous sessions, and remote clients are not part of protocol v1.

## 9. Flutter and Rust bridge

Flutter is a thin UI over the application API. Rust remains the only canonical project/timeline state. Flutter may own presentation, navigation, panel, selected-tool, temporary text/input state, and scoped cached read models/view models, but not a second authoritative editable project model.

After a command is validated and applied, Rust emits a domain change or state-invalidation event; Flutter refreshes affected scoped queries/read models and rebuilds the relevant surface. Conceptual event categories include `ProjectChanged`, `TimelineChanged`, `SelectionChanged`, `MediaChanged`, `JobChanged`, and `CapabilitiesChanged`; exact names and schema are not frozen. Events or query results carry enough project revision/order information for Flutter to ignore or requery stale state when a newer revision is known.

Hot UI paths should use scoped queries such as timeline viewport, track list, selection inspector, media bin, and job list. Do not serialize and copy the whole project into Dart or rebuild every surface for each timeline interaction. Start with simple scoped queries and invalidation; do not introduce a reactive state framework before it is needed.

The Flutter bridge uses `flutter_rust_bridge` 2.13.0 with generated typed bindings in `packages/or_app_bridge` and a thin `crates/or_app_bridge` adapter. Its opaque `ProjectHostHandle` owns a `LiveProjectHost`; it does not expose mutable `ProjectDocument` state to Dart. Typed operations cover create/open, summary, rename, undo/redo, save, close, recovery inspection/actions, descriptor path, and ordered invalidation events. Flutter keeps an immutable active-project read model and refreshes it from Rust after events; mutations carry the read model's expected revision and do not auto-retry conflicts. The desktop `file_selector` plugin chooses paths only; Rust owns canonical file validation and persistence. New/Open stay unavailable on Android pending SAF support. CI builds each target, runs Flutter widget and lifecycle tests, and exercises the native macOS bridge.

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

Phase 5A uses an external `ffprobe` executable only for metadata inspection. `probe_media_file(&Path)` accepts local regular files, invokes the executable through `std::process::Command` without a shell, and passes the canonical file path as one `-i` argument. It requests `-of json` and a narrow `-show_entries` set of format and stream fields, and CI records the tool with `ffprobe -version`. Output is limited to 1 MiB stdout and 64 KiB stderr, with a 15-second timeout and child cleanup. The parser ignores unknown external JSON fields, then validates the OR metadata representation. `RationalTime` parses decimal durations exactly with at most nine fractional digits, matching its `u32` denominator bound; valid frame rates use `RationalRate`. `MediaMetadata` is control-plane metadata and contains no path or arbitrary tags. This adapter is not a decode/render architecture selection, and Phase 5A does not link or bundle FFmpeg. Invocation syntax and fields follow the [official ffprobe documentation](https://ffmpeg.org/ffprobe.html).

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

Phase 5A implements `JobId` (validated UUIDv4), `JobKind::MediaProbe`, and `JobState` values (`Queued`, `Running`, `Succeeded`, `Failed`, and `Cancelled`). This is the job contract only: the public media probe runs synchronously, and no scheduler, Job Manager, thread pool, priority/backpressure system, progress/result API, or job persistence exists yet. The shared Job Manager remains planned for thumbnail and waveform generation, proxy creation, transcription, translation, AI work, model and asset downloads, and export.

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

Phase 4F/4UI-2 bounds `.orproj` input to 64 MiB and recovery sidecars to 136 MiB, validates strict UTF-8/serde envelopes, and preserves the exact disk-base and recovery checks before file-session saves. IPC accepts only the versioned, bounded semantic request types; the server cannot open caller-selected paths or execute shell commands. Unix runtime directory, socket, and descriptor permissions are restricted to the current user. The Windows runtime directory, descriptor file, and named pipe use protected owner-only DACLs, and the pipe rejects remote clients. A random per-server token is stored only in the descriptor and is not logged, returned by `Describe`, or displayed in Flutter. Anyone who can read that descriptor as the same OS user can authenticate, so it is not a multi-user security boundary. No API credentials belong in the descriptor, IPC payloads, project files, or CLI output. The implementation creates no TCP listener.

Provider network capability and permission are enforced centrally by the application, including when a request originates from CLI or an agent. Offline Mode denies OR-originated optional network calls regardless of UI path; it does not claim to firewall the operating system. Send the minimum required data to each cloud task, without unrelated project context or secret values.

## 22. Implementation structure

Start with the smallest useful Rust workspace and Flutter shell when Phase 3 is explicitly started. Planned domains are conceptual boundaries, not a mandate to create one crate per domain. Split a module into a crate only when a concrete build, reuse, ownership, or dependency boundary justifies it. Never create empty future crates or modules.

## 23. Technical non-decisions

The following are deliberately not permanently selected:

- exact FFmpeg Rust binding
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
- [ffprobe documentation](https://ffmpeg.org/ffprobe.html) defines its command-line options, JSON output, field selection, input argument, and version reporting.
- [OpenTimelineIO](https://github.com/academysoftwarefoundation/opentimelineio) describes an editorial interchange format and API.
- [whisper.cpp license](https://github.com/ggml-org/whisper.cpp/blob/master/LICENSE) covers the runtime source, not every model artifact.
- [ONNX Runtime license](https://github.com/microsoft/onnxruntime/blob/main/LICENSE) covers the runtime source, not model artifacts.
- [llama.cpp license](https://github.com/ggml-org/llama.cpp/blob/master/LICENSE) covers the runtime source, not model artifacts.
