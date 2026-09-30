# Technical Plan

## Execution lock and implementation order

The machine-readable architecture and execution authority is
[docs/execution/README.md](execution/README.md), with the immutable graph in
[PLAN.json](execution/PLAN.json), mutable state in [STATE.json](execution/STATE.json),
and locked phase contracts in [execution/phases](execution/phases). This document records architecture and subsystem contracts. For authoritative
current checkpoint and phase status, see `docs/execution/STATE.json`; this
document does not copy mutable `NEXT` state or authorize skipping it. Further
runtime crates are not created ahead of their checkpoint gates.

The implementation order preserves the control-plane/runtime-plane boundary:
Rust `or_core` owns canonical project/application state; `or_runtime` now
coordinates the Phase 7A capability, snapshot, queue, budget, and fallback
contracts; `or_render` now owns the Phase 7B wgpu spine; `or_media` provides the linked software decode foundation; `or_audio` owns
clocked realtime audio; and `or_ai` owns task/provider/model boundaries. Native
media acceleration, the audio engine, and AI remain checkpoint-gated.

Runtime work consumes immutable `RenderSnapshot` values and uses
`FrameDescriptor`/`FrameLease` ownership. Full-rate frames do not travel as
copied Dart byte arrays. Native Metal, DX12, Vulkan, CUDA, VideoToolbox,
MediaCodec, hardware-buffer, and DMABUF paths stay behind adapters with a
software correctness fallback. Apple MLX/MPS/Core ML, NVIDIA CUDA/NVDEC/NVENC,
Windows ML/providers, and Android AI runtimes are capability/provider choices,
not direct `or_core` dependencies.

See the [ADR set](adr/README.md) for durable decisions and the phase documents
for checkpoint-specific tests, dependency gates, and stop conditions.

## Status

Phase 3 implemented the bootstrap subset: a Rust workspace and `or_core`, semantic CLI commands, a Flutter shell, and typed `flutter_rust_bridge` 2.13 bindings for application info, health, and capabilities. Phase 4A–4F and 4UI-2 provide the project/application, persistence, recovery, IPC, CLI, and desktop live-host foundations. Phase 5A–5F provide typed media/jobs, external `ffprobe`, disposable previews, indexed cache, and core-only file-backed Proxy V1. Phase 6 provides canonical tracks and clips, exact trim/split/ripple editing, persistent markers, marker-aware snapping, CLI parity, and the corresponding Flutter UI. Phase 7A provides the standalone dependency-free `or_runtime` contracts for immutable snapshot identity, exact-time frame descriptors and leases, bounded cancellation/backpressure, render/audio/decode budgets, and centralized software-first capability selection. Phase 7B adds the headless `or_render` wgpu spine, deterministic synthetic offscreen rendering, readback normalization, and a handle-only viewer contract. Phase 7C establishes linked software decode; visible playback and export remain future Phase 7/8 work. See [docs/execution/STATE.json](execution/STATE.json) for the mutable execution status, [ARCHITECTURE.md](ARCHITECTURE.md), and [ROADMAP.md](ROADMAP.md) for design and human roadmap context.

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
17. [Templates, themes, MotionScene, and community](#17-templates-themes-motionscene-and-community)
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

The original `.orproj` schema v1 contract is UTF-8 JSON with this envelope:

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

The current `.orproj` schema is v5. Its envelope keeps the format marker and project identity/revision/name, then stores ordered `media`, `timeline.tracks`, global `timeline.markers`, and the required nullable `timeline.sequence_frame_rate`. `ProjectDocument` owns a typed UUIDv4 `ProjectId`, persistent `ProjectRevision`, UTF-8 name, ordered media items, and canonical `ProjectTimeline`. Private versioned DTOs and explicit conversion keep domain changes from silently changing the file contract. The decoder strictly accepts v1–v5; v5 requires `sequence_frame_rate` to be either `null` or an exact positive `RationalRate`, and all versions validate required fields and reject unknown fields. The Rust encoder emits deterministic pretty v5 JSON with a trailing newline; this is not a cross-implementation canonical JSON standard. `ProjectInstanceId` is runtime-only and is never persisted. The version probe rejects unsupported versions before decoding. Bounded load remains 64 MiB. Migrations from v1–v4 set the sequence rate to `null` and preserve project ID, revision, name, and existing media/timeline state. A clean open never rewrites the file, and the next explicit save writes v5 without a conversion-only revision increment.

7F0 implements schema v5 with one strict `timeline.sequence_frame_rate` field
encoded as either `null` or an exact `RationalRate`. Strict migrations from
v1–v4 set the rate to `null`; new projects also begin unset. Recovery remains
envelope v1 and accepts nested v5 snapshots. Opening a migrated project stays
clean and leaves disk bytes unchanged until explicit save.

V1 contains only project ID, revision, and name. V2 adds ordered media entries containing a UUIDv4 `MediaId`, validated local `file:` URI, and bounded `MediaMetadata`; source bytes remain external. V3 adds an ordered timeline. Each track stores a UUIDv4 ID, `video` or `audio` kind, and ordered clips. Each clip stores exactly its UUIDv4 ID, `MediaId`, `timeline_start`, and `source_range`; playback speed is implicitly 1×. V3 media IDs, source URIs, track IDs, and clip IDs must be unique in their respective project-wide scopes. Media metadata bounds remain as documented in [SECURITY_LICENSING.md](SECURITY_LICENSING.md).

Projects reference external media. Media paths and fingerprints support relink, replace, offline state, and project collection without embedding source media by default. Cache entries never become canonical project state.

A Phase 4E1 safe-save sequence is:

1. Encode the canonical `ProjectDocument` in memory and reject output larger than `MAX_PROJECT_FILE_BYTES` (64 MiB) before creating any file.
2. Create a unique hidden sibling named `.<target-name>.or-tmp-<uuid>` with `create_new(true)`, retrying only bounded name collisions. The caller's parent directory is never created automatically.
3. Write all bytes through a buffer, flush the buffer, sync the temporary file, and close it before replacement.
4. Replace the destination without deleting it first. Unix-like targets (macOS, Linux, and direct-filesystem Android paths) use same-directory `rename` and sync the containing directory. Windows uses `MoveFileExW` with `MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH`.
5. Report success only after the replacement and platform durability step. A failure to sync the Unix directory after replacement returns a durability-uncertain error; the new file may already be present.

`load_project_file` opens the file, checks its metadata size, and reads at most `MAX_PROJECT_FILE_BYTES + 1`; it rejects larger input, validates strict UTF-8, then calls the existing `decode_project`. Neither load nor save changes `ProjectId` or `ProjectRevision`. Session instance IDs and undo/redo history are not passed to the storage API and remain absent from `.orproj`.

`ProjectStorageError` distinguishes I/O, oversize input, invalid UTF-8, codec failures, temporary-file create/write/flush/sync failures, replacement failures, and post-replace durability uncertainty.

The replacement primitive provides atomic namespace/file replacement on supported local filesystems. File sync and directory sync or write-through improve durability but do not guarantee survival of every power-loss, controller, or filesystem failure. Phase 4E1 assumes one OR application/session owns a save target at a time; it does not provide a file-locking or concurrent-writer coordinator. Future migrations must be explicit, ordered, versioned, and tested on old and malformed inputs; the implemented v1-to-v4, v2-to-v4, and v3-to-v4 migrations preserve project identity, revision, and name, with older timelines receiving an empty marker list.

### Phase 4E2 snapshot recovery checkpoint

The recovery format is a separate versioned sidecar beside the `.orproj` file. It prefixes the project filename with `.` and appends `.or-recovery` (for example, `/projects/movie.orproj` uses `/projects/.movie.orproj.or-recovery`). Its strict envelope uses format marker `opencut-reinforced-recovery`, schema version 1, and contains two `.orproj` snapshots (v1–v5): the exact saved base at revision N and the newer unsaved recovery snapshot at revision M, where M > N. The recovery file is bounded to 136 MiB; each nested project is decoded and validated by the project codec and remains subject to the 64 MiB project limit. Cross-schema base/recovery combinations are supported and tested. This snapshot checkpoint is the initial pre-MVP representation and may evolve after real scale measurements.

Writing a checkpoint requires matching `ProjectId` values, a newer recovery revision, and an on-disk canonical `ProjectDocument` exactly equal to the supplied base. The write uses the same same-directory atomic replacement primitive as project saves and does not mutate the canonical project. The recovery envelope remains v1; nested project snapshots can use project schema v1–v5, including persistent markers and the optional sequence rate in v5. Runtime `ProjectInstanceId` and session history are not stored.

Inspection reads and classifies without changing files or project state. If the canonical project exactly equals base N, the recovery is a candidate. If it equals the recovery snapshot or has the same project ID with a revision newer than M, the checkpoint is stale. A different project lineage or other state that cannot prove the exact base is a conflict; a missing canonical file is an orphaned conflict. Project loading does not inspect or apply recovery automatically, and conflicts never select a winner silently.

Applying is explicit and re-inspects the current canonical file before saving. It atomically saves recovery M through the existing project storage API, keeps revision M unchanged, then removes the sidecar. A save failure preserves the checkpoint; a cleanup failure after a successful save is reported as cleanup pending. Explicit discard removes the sidecar, including a malformed one, without changing the canonical project. This is a snapshot checkpoint foundation, not event sourcing, command replay, persistent history, autosave, or a recovery UI.

### Phase 4F file-backed session and shared dispatch

`ApplicationRequest` carries an existing `CommandEnvelope`, `QueryEnvelope`, or `TransactionEnvelope`; `ApplicationResponse` carries the corresponding result or the existing `OperationError`. `ProjectSession::handle_application_request` delegates to the existing command, query, and transaction methods. This is the common semantic path for the headless CLI and IPC server; it does not introduce a second editing implementation.

`ProjectFileSession` owns a project path, a live `ProjectSession`, and the exact `ProjectDocument` last known to be saved. Opening loads the canonical `.orproj`, inspects recovery, then creates a fresh runtime instance without incrementing revision. `NONE` and `STALE` allow opening; candidate, conflict, and invalid recovery require explicit attention. Save re-inspects recovery and reloads the disk document; the save proceeds only when the disk document exactly equals the remembered saved base. It then uses the existing atomic save path and does not increment revision. Dirty state is the in-memory project compared with that exact saved document; neither dirty state, instance ID, nor history is persisted.

### Project revisions

`ProjectRevision` is a persistent canonical project-state value backed by an unsigned 64-bit integer. Phase 4A implements its initial value, zero, and checked increment; the v1–v5 codecs preserve the stored revision during encode/decode. A changed rename, media add/remove, marker edit, sequence-rate edit, or transaction increments once; an exact no-op, failed operation, net-no-op transaction, and read-only query leave the revision unchanged. Undo and redo restore content through new canonical mutations, so they increment from the current revision rather than moving it backward. Migration from v1–v4 to v5 during explicit save does not increment revision. Project ID and revision survive save/reopen; each fresh runtime open receives a new ephemeral `ProjectInstanceId`, which is excluded from the project document.

`CommandEnvelope` v1 uses `ProjectId` + `ProjectInstanceId` + `expected_project_revision` as live mutation preconditions. This distinguishes a stale client attached to a previous runtime session even when the same project reopens at the same revision. Revision overflow is checked and reported without mutation; it must never wrap. Restoring older snapshot content through OR is a new mutation: at current revision 100, restoring content captured at revision 20 results in revision 101, not 20.

### Phase 6A timeline domain and persistence

Phase 6A adds a persistence/domain foundation only. `TrackId` and `ClipId` are canonical lowercase UUIDv4 values. `TrackKind` is only Video or Audio, and a new project has no default tracks. Track order is persisted. A clip contains only its ID, media ID, timeline start, and source range, with implicit 1× speed. Timeline and source starts must be nonnegative, duration must be positive, and timeline/source ends use checked arithmetic. Clips are ordered by timeline start; same-track overlap is rejected while adjacency and cross-track overlap are allowed. Track IDs and clip IDs are unique across the project.

Every clip must reference an existing media item and a compatible stream; the first matching stream in metadata order is used. If that stream has a duration, it is enforced; otherwise the container duration is used when available, and an unknown duration is unbounded. Loading does not check the source filesystem, so offline media is allowed. The decoder bounds tracks at 256, clips at 100,000 project-wide, and clips at 100,000 per track.

The timeline is canonical project state. Phase 6A adds no timeline commands, queries, CLI syntax, IPC changes, or Flutter timeline. Phase 6B adds the application commands and queries described below; it does not add a Flutter timeline or playback. `media.remove` refuses referenced media with `MEDIA_IN_USE` before revision/history mutation and does not cascade. Track labels/settings, mute/solo/lock, compositing precedence, linking/grouping, alternate-stream selection, speed changes, transitions/effects, transforms/crop/opacity, audio gain/pan, markers, snap behavior, UI interactions, playback/decode/render behavior remain outside Phase 6B.

### Phase 6B basic track and clip operations

`ProjectTimeline` remains owned by `ProjectDocument` and publicly read-only. The five schema-v1 commands enter through the existing validated application boundary; each declares `mutates_project = true` and `allowed_in_transaction = false`. Track add appends a caller-supplied UUIDv4 ID. Track remove accepts only an empty track and restores its exact index through history. Clip insert accepts exact `RationalTime` values, validates persisted media metadata and stream compatibility, and inserts by ascending `timeline_start`. Clip IDs are unique project-wide. Clip delete is explicit. Clip move preserves clip ID, media ID, and source range; it changes only the track and timeline start, and may cross tracks only when their `TrackKind` matches. Same-track overlap is rejected, adjacency is allowed, and overlap on other tracks is irrelevant. Media sources may be offline; edits never probe files. No media item is cascade-deleted. The project encoder remains schema v3, the decoder remains v1/v2/v3, and recovery continues to use its existing schema-v1 envelope.

The command argument objects reject unknown fields. `timeline.track.add` takes `{track_id, kind}`; `timeline.track.remove` takes `{track_id}`; `timeline.clip.insert` takes `{clip_id, track_id, media_id, timeline_start, source_range}`; `timeline.clip.move` takes `{clip_id, track_id, timeline_start}`; and `timeline.clip.delete` takes `{clip_id}`. Rational values use `{numerator: i64, denominator: u32}`; `TimeRange` contains `start` and positive `duration`. Malformed numeric/range values and cross-kind moves use `INVALID_ARGUMENTS`; stable domain codes cover duplicate/missing IDs, non-empty track removal, incompatible media, overlap, and limits. Track capacity is 256, clips are bounded to 100,000 project-wide and per track.

Every real timeline edit checks revision overflow and reserves required project/history storage before mutation, increments `ProjectRevision` exactly once, records one semantic `ProjectChange` in one `ChangeSet`, and clears redo. Failures leave project, revision, and both stacks unchanged. A same-track move to the same start succeeds with `changed = false` and preserves revision, history, and redo. Undo and redo each apply one history transition and increment the current revision once; revision never rewinds. Timeline edits are not children of `TransactionEnvelope`, and no transaction semantics are generalized in this phase.

### Phase 6C real Flutter timeline foundation

The Flutter bridge exposes typed wrappers for the existing `timeline.tracks` and bounded `timeline.clips` queries and all Phase 6B/6D commands. Track and clip IDs are generated in Rust for UI add/insert/split actions. `RationalTimeView` crosses the bridge as an exact signed-64 numerator and positive unsigned-32 denominator; media presentation views also carry persisted container, first-video-stream, and first-audio-stream durations without changing the core media or query contracts. The Dart gateway maps these to immutable read models with `BigInt` numerators. Display-only pixel and ruler calculations may use `double`; edit inputs remain exact `NUM/DEN` values.

`AppShell` stores disposable track results and at most the first 100 clips per populated track. Empty tracks do not cause a clip query. Load more requests the next bounded page only on demand. Track and clip pages are combined only when project ID, runtime instance ID, revision, track ID, page offset, and page bounds agree; an inconsistent refresh is retried once, then the project remains open with a timeline error. Ordered `project_changed` events refresh from the Rust summary and queries, so attached CLI edits become visible. Project close/switch clears the read snapshot. Flutter does not own canonical track/clip state or optimistically change it.

The project workspace displays tracks in persisted order with derived V#/A# labels, visible clip blocks positioned and sized from exact values converted only for layout, and exact rational details in tooltips. Users can append Video/Audio tracks, remove empty tracks, insert a media clip through an exact-time dialog, move a clip between same-kind tracks, explicitly delete it, trim its absolute start or end, split at an exact interior time, and ripple-delete it with later clips on that track shifted left by the deleted duration. Insert duration defaults to the matching first stream duration, then container duration, or remains blank when both are unknown. Trim and split dialogs show current timeline timing only and never generate optimistic geometry; Rust validates all final state. Phase 6D adds no drag/drop, snap, markers, playback, decode, rendering, or timeline thumbnails/waveforms. It adds no command/query, IPC version, dependency, or project-schema change; `.orproj` remains v3 and session history remains Rust-owned.

### Phase 6E1 pointer editing and clip-edge snapping

Phase 6E1 adds exactly one schema-v1 read-only query, `timeline.snap`; the command catalog remains unchanged, IPC remains v1, `.orproj` remains schema v3, and recovery remains v1. Its strict arguments are `{operation, clip_id, target_track_id, target_time}`, where `operation` is `move`, `trim_start`, or `trim_end`; moves require a same-kind target track and trims reject a target track. The resolver uses a fixed exact `1/8`-second threshold and scans timeline zero plus every other canonical clip start and end across all tracks, excluding the active clip. A move evaluates the proposed new start and new end anchors; a trim evaluates only its selected edge. It returns only the raw/resolved exact times, snap state, moving anchor, target kind, and optional target IDs. Tie-breaking is deterministic: smallest absolute adjustment, earliest candidate time, start anchor before end anchor, canonical track index, canonical clip index, and start boundary before end boundary, with timeline zero at position zero ahead of clip metadata. The scan is O(total clips), the response is O(1), and the query has no marker, media, filesystem, pagination, index, cache, or spatial-tree inputs.

The typed Rust bridge and Dart gateway carry project ID, project-instance ID, and revision with the snap result. Flutter validates those values and the active gesture token before committing. Pointer movement retains the original canonical exact clip start/end and rounds only the total logical pixel delta to the nearest 1 ms (`1/1000`); it never accumulates rounded updates or stores a second editable timeline. A body drag can target only same-kind lanes; an opposite-kind lane is invalid and a drop outside lanes retains the source track. Start/end handles have pointer priority. During a gesture Flutter shows only a temporary ghost/edge preview; canonical geometry changes only when the existing `timeline.clip.move` or `timeline.clip.trim` command succeeds. Snap is queried only once at drop/release, never in a live high-frequency loop; a successful snap may show a temporary cyan guide before the command completes. Revision conflicts, project/session switches, disposal, or attached CLI invalidation reject stale results without retry and refresh the canonical view. The Snap toggle defaults on, is gesture-local UI state, and is not persisted. Exact Move/Trim dialogs continue to accept arbitrary exact `NUM/DEN` values. `or timeline snap` exposes the same read-only result headlessly and when attached without saving, marking dirty state, or changing revision.

### Phase 6E2A persistent marker foundation

Phase 6E2A adds a global point-marker domain to `ProjectTimeline`: `MarkerId` is a persistent canonical UUIDv4, and `TimelineMarker` contains only `MarkerId`, nonnegative exact `RationalTime`, and an exact UTF-8 label. Labels must contain non-whitespace text and remain at most 256 UTF-8 bytes; the project contains at most 10,000 markers. Markers may share times and labels, do not share an ID namespace with tracks/clips/media, and are stored in strict `(timeline_time, MarkerId)` order. The timeline remains publicly read-only through `markers()`, and marker validation does not inspect media or the filesystem.

The project encoder emits strict schema v4 with only `id`, `timeline_time`, and `label` marker fields. The decoder accepts v1/v2/v3/v4; old versions receive an empty marker list, old files remain clean and unchanged until explicit save, and conversion writes v4 without a revision increment. Recovery remains envelope schema v1 and accepts nested v1–v4 project snapshots. Marker commands are exactly `timeline.marker.add`, `.move`, `.rename`, and `.delete`, all schema v1, mutating, and transaction-disallowed. They use bounded semantic `ChangeSet` recipes, check revision/storage capacity before mutation, preserve redo on no-ops, and return only the two marker-specific errors `TIMELINE_MARKER_ID_ALREADY_EXISTS` and `TIMELINE_MARKER_NOT_FOUND`; existing `INVALID_ARGUMENTS` and `TIMELINE_LIMIT_EXCEEDED` cover validation and capacity.

The read-only `timeline.markers` query is schema v1 with offset/limit paging capped at 100 and canonical ordering. `QueryResult` adds only an optional marker page, so existing result shapes remain unchanged. Snap V1 remains accepted, clip-only, and wire-compatible; Snap V2 is explicitly marker-aware, keeps the exact `1/8` threshold and existing tie rules, returns `target_marker_id` only for marker winners, and is advertised as the latest `timeline.snap` catalog schema. `OPERATION_SCHEMA_VERSION`, recovery, IPC v1, clip editing, project summary, media behavior, and the Flutter bridge/UI remain unchanged. Headless and attached CLI marker commands reuse the existing rational parser and file-session/live-host save semantics; Flutter remains on Snap V1 until Phase 6E2B.

### Phase 6D trim, split, and ripple editing

The three schema-v1 commands are `timeline.clip.trim`, `timeline.clip.split`, and `timeline.clip.ripple_delete`. All mutate canonical state, are disallowed in grouped transactions, use strict argument objects, and leave the project, revision, and history stacks unchanged on failure. Only `project.rename` remains transaction-allowed; the complete command catalog contains 13 commands.

`timeline.clip.trim` takes `{clip_id, edge: "start"|"end", timeline_time}`. The edge is absolute timeline time: start trim preserves the old timeline end and source end while allowing a nonnegative earlier extension when the resulting source start remains nonnegative; end trim preserves the timeline/source starts and changes duration to `timeline_time - start`. Final-state validation enforces same-track overlap, positive duration, checked ends, and known stream/container source bounds. Exact equality is a no-op with no revision or history change.

`timeline.clip.split` takes `{clip_id, new_clip_id, timeline_time}` and accepts only a strict interior point. The original ID remains on the left clip and the caller-provided project-wide unique ID becomes the right clip. The split is an exact source/timeline partition and checks total/per-track capacity before mutation. `timeline.clip.ripple_delete` takes `{clip_id}` and deletes only that clip, shifting later clips on the same track left by its duration while preserving IDs, media, source ranges, other tracks, and existing gaps. Its public `ChangeSet` stores only the deleted clip, track/index, shifted count, and shift duration; history validates a compact state guard before applying undo/redo.

Each real edit increments `ProjectRevision` once, reserves required storage before mutation, records one semantic `ChangeSet`, clears redo, and supports one-step undo/redo with a new monotonic revision. Headless CLI forms are `timeline trim-clip --clip ... --edge start|end --to NUM/DEN`, `timeline split-clip --clip ... --at NUM/DEN [--id NEW_RIGHT_CLIP_ID]`, and `timeline ripple-delete-clip --clip ...`; headless changed operations save, while attached operations remain dirty until explicit save. Flutter exposes the same operations through typed gateway methods and an exact action menu; it refreshes canonical read pages after success or revision conflict and never retries stale actions.

## 4. Command system

Phases 4C–4D establish a small static command catalog without a dynamic registry framework. The catalog includes `project.rename`, `history.undo`, `history.redo`, `media.add`, `media.remove`, `timeline.track.add`, `timeline.track.remove`, `timeline.clip.insert`, `timeline.clip.move`, `timeline.clip.delete`, `timeline.clip.trim`, `timeline.clip.split`, `timeline.clip.ripple_delete`, `timeline.marker.add`, `timeline.marker.move`, `timeline.marker.rename`, `timeline.marker.delete`, and `timeline.sequence.set_frame_rate`, all schema v1. Only `project.rename` is allowed inside a transaction; media and timeline commands are not. `CommandEnvelope` v1 has `command_id`, `schema_version`, typed `project_id`, typed `project_instance_id`, `expected_project_revision`, and `arguments`. `TransactionEnvelope` v1 has a schema version, the same three project/session preconditions, and an ordered list of strict `CommandCall` values. The envelope uses `serde_json::Value` at the structured argument boundary; dispatch immediately decodes arguments into strict private typed structures. The transport-independent contracts are carried by the bounded, strict JSON protocol in `or_ipc` v1.

The dispatcher checks command ID, schema version, project ID, project-instance ID, and expected revision before decoding arguments or mutating state; unknown envelope and argument fields are rejected. Rename preserves its supplied UTF-8 name exactly. Renaming to the same name succeeds as a no-op (`changed = false`) without incrementing revision or changing history. `media.add` accepts one fully validated `MediaItem` and rejects duplicate IDs and source URIs. `media.remove` accepts a `MediaId` and returns `MEDIA_NOT_FOUND` if absent. Each successful add/remove checks the next revision and reserves history storage before mutation; failures change no project, revision, or history. Timeline and marker commands use strict typed arguments and the Phase 6B/6D/6E2A behavior above. `timeline.sequence.set_frame_rate` strictly accepts a nullable positive exact rational rate, participates in session-local undo/redo, increments revision once when changed, and preserves redo and revision on a no-op. Results identify the operation, session, before/after revisions, whether state changed, and the resulting semantic `ChangeSet`.

`ProjectSession` owns a canonical `ProjectDocument` and a runtime-only `ProjectInstanceId`. It exposes read-only project access and the command/query entry points, with no public mutable document accessor. The command/application path is the only public project-mutation path. Flutter widgets, CLI presentation code, agents, render workers, media decoders, background jobs, AI workers, and provider adapters must not directly mutate the canonical project.

The runtime transaction implementation is deliberately narrow: it stages ordered `project.rename` calls, validates every child, and reduces the group to one net name change. A changed group applies the name and increments revision once, then records one `ChangeSet` in session history. A net no-op creates no revision or history entry. Any validation, precondition, overflow, or history-storage failure occurs before canonical state changes; a failed group preserves the project and both history stacks. `history.undo` and `history.redo` apply the stored name change only when the current name matches its expected side; they increment the current revision once and move the entry between in-memory session stacks. A new real edit clears redo history. History is neither serialized nor carried across a fresh `ProjectSession::open`.

Stable errors include `UNKNOWN_COMMAND`, `UNSUPPORTED_COMMAND_SCHEMA`, `UNKNOWN_QUERY`, `UNSUPPORTED_QUERY_SCHEMA`, `UNSUPPORTED_TRANSACTION_SCHEMA`, `EMPTY_TRANSACTION`, `COMMAND_NOT_ALLOWED_IN_TRANSACTION`, `PROJECT_ID_MISMATCH`, `PROJECT_INSTANCE_MISMATCH`, `REVISION_CONFLICT`, `INVALID_ARGUMENTS`, `REVISION_OVERFLOW`, `MEDIA_ID_ALREADY_EXISTS`, `MEDIA_SOURCE_ALREADY_EXISTS`, `MEDIA_NOT_FOUND`, `TIMELINE_TRACK_ID_ALREADY_EXISTS`, `TIMELINE_TRACK_NOT_FOUND`, `TIMELINE_TRACK_NOT_EMPTY`, `TIMELINE_CLIP_ID_ALREADY_EXISTS`, `TIMELINE_CLIP_NOT_FOUND`, `TIMELINE_MEDIA_INCOMPATIBLE`, `TIMELINE_OVERLAP`, `TIMELINE_LIMIT_EXCEEDED`, `TIMELINE_MARKER_ID_ALREADY_EXISTS`, `TIMELINE_MARKER_NOT_FOUND`, `NOTHING_TO_UNDO`, `NOTHING_TO_REDO`, `HISTORY_CONFLICT`, and `HISTORY_STORAGE_FAILURE`. Probe/preparation errors remain separate media-probe/media-import errors. The CLI, local IPC client, and Flutter bridge use the shared operation errors. Permissions, dry-run, generalized transaction operations, later timeline editing tools, and agent clients remain future work.

## 5. Query system

The deterministic query catalog is `project.summary`, `media.list`, `media.get`, `timeline.tracks`, `timeline.clips`, `timeline.snap`, `timeline.markers`, and `timeline.sequence.settings`. Existing queries remain schema v1; `timeline.snap` advertises schema v2 while accepting schema v1 and v2, and `timeline.markers` and `timeline.sequence.settings` are schema v1. `QueryEnvelope` v1 has `query_id`, `schema_version`, typed `project_id`, typed `project_instance_id`, and `arguments`. `project.summary` accepts an empty object and retains its existing wire shape: project ID, runtime instance ID, current revision, and name. `media.list` accepts unsigned `offset` and `limit`, rejects zero or limits above 100, returns a bounded page in project insertion order, and includes items, total count, offset, limit, and `next_offset` when more items exist. An offset beyond the end returns a valid empty page. `timeline.tracks` returns canonical track order with ID, kind, and clip count. `timeline.clips` takes a track ID, offset, and limit from 1 through 100, and returns only clip ID, media ID, exact timeline start, and source range in canonical order. An offset equal to or beyond the clip count produces an empty page; `next_offset` appears only when another page exists. `timeline.markers` takes strict unsigned `offset` and `limit` arguments with a maximum page size of 100, returns canonical marker state and ordering, and produces an empty page beyond the end. `timeline.sequence.settings` accepts an empty argument object and returns the current nullable exact sequence rate. `timeline.snap` keeps the Phase 6E1 clip-only resolver for schema v1 and adds marker candidates only for explicit schema v2; both versions scan canonical state independently of paginated pages. Timeline DTOs are application-facing types, not persistence codec structures. All queries read canonical state without mutation or revision changes; listing does not open/probe source files, and offline sources remain listable.

Unknown queries and unsupported query schemas return `UNKNOWN_QUERY` and `UNSUPPORTED_QUERY_SCHEMA`; project/session mismatches use the corresponding shared codes, and invalid arguments return `INVALID_ARGUMENTS`. Queries must not start hidden destructive work or return provider credentials. Output fields and schema versions are discoverable for automation clients. `QueryResult` omits absent optional result fields so existing `project.summary`, `media.list`, and `media.get` wire shapes remain unchanged. IPC protocol v1 and the shared `ApplicationRequest` route are unchanged.

## 6. History and transactions

Phase 4D implements the first in-memory transaction and history behavior without event sourcing. `ChangeSet` supports normalized project-name changes, indexed media-added/removed changes, compact Phase 6B/6D timeline changes, and compact Phase 6E2A marker changes: track add/remove stores ID, kind, and index; clip insert/delete stores the clip, track, and index; clip move stores the clip/media/source range and exact old/new track, index, and timeline start; trim stores before/after clip state; split stores the original and exact left/right states; ripple delete stores only the deleted clip, track/index, shifted count, and shift duration; marker add/delete stores the exact marker and canonical index; marker move stores ID/label and exact old/new times and indices; marker rename stores ID/time and before/after labels. Ripple history keeps an internal constant-size state guard rather than a per-shifted-clip list. Entries do not store full `ProjectDocument` or marker-collection snapshots. Undo/redo validate the expected canonical identity and state before mutation and return `HISTORY_CONFLICT` without partial changes on mismatch. Each undo/redo is a new canonical mutation and increments the current revision once. New real edits clear redo history. No-op edits, failed operations/groups, and reads leave project state, revision, and history unchanged. History lives only inside `ProjectSession`: it is not saved in `.orproj` and resets when the document is opened into a new session. Required project/history storage is reserved before timeline mutations; allocation failure uses `HISTORY_STORAGE_FAILURE` and leaves state and both stacks unchanged.

The transaction envelope still accepts only `project.rename` child calls. Grouped commands stage all intermediate names and normalize them to one net `ChangeSet`; a net no-op does not increment revision or create history. Timeline commands are explicitly rejected inside a transaction. This proves bounded single-operation atomic grouping, not a general transaction framework. The separate project-storage and recovery-checkpoint APIs do not depend on keeping an unbounded event log; persistent history remains out of scope. IPC remains protocol v1 and carries timeline operations through the existing generic `ApplicationRequest` envelope; no timeline-specific transport is added.

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
or media probe --file MEDIA_PATH [--json]
or media list --project PROJECT_PATH [--offset N] [--limit N] [--json]
or media list --attach DESCRIPTOR [--offset N] [--limit N] [--json]
or media add --project PROJECT_PATH --source MEDIA_PATH [--json]
or media add --attach DESCRIPTOR --source MEDIA_PATH [--json]
or media remove --project PROJECT_PATH --id MEDIA_ID [--json]
or media remove --attach DESCRIPTOR --id MEDIA_ID [--json]
or timeline tracks --project PATH [--json]
or timeline tracks --attach DESCRIPTOR [--json]
or timeline clips --project PATH --track TRACK_ID [--offset N] [--limit N] [--json]
or timeline clips --attach DESCRIPTOR --track TRACK_ID [--offset N] [--limit N] [--json]
or timeline markers --project PATH [--offset N] [--limit N] [--json]
or timeline markers --attach DESCRIPTOR [--offset N] [--limit N] [--json]
or timeline snap --project PATH --clip CLIP_ID --operation move|trim-start|trim-end --at NUM/DEN [--track TRACK_ID] [--json]
or timeline snap --attach DESCRIPTOR --clip CLIP_ID --operation move|trim-start|trim-end --at NUM/DEN [--track TRACK_ID] [--json]
or timeline add-marker --project PATH --at NUM/DEN --label LABEL [--id MARKER_ID] [--json]
or timeline add-marker --attach DESCRIPTOR --at NUM/DEN --label LABEL [--id MARKER_ID] [--json]
or timeline move-marker --project PATH --id MARKER_ID --to NUM/DEN [--json]
or timeline move-marker --attach DESCRIPTOR --id MARKER_ID --to NUM/DEN [--json]
or timeline rename-marker --project PATH --id MARKER_ID --label LABEL [--json]
or timeline rename-marker --attach DESCRIPTOR --id MARKER_ID --label LABEL [--json]
or timeline delete-marker --project PATH --id MARKER_ID [--json]
or timeline delete-marker --attach DESCRIPTOR --id MARKER_ID [--json]
or timeline add-track --project PATH --kind video|audio [--id TRACK_ID] [--json]
or timeline add-track --attach DESCRIPTOR --kind video|audio [--id TRACK_ID] [--json]
or timeline remove-track --project PATH --track TRACK_ID [--json]
or timeline remove-track --attach DESCRIPTOR --track TRACK_ID [--json]
or timeline insert-clip --project PATH --track TRACK_ID --media MEDIA_ID --at NUM/DEN --source-start NUM/DEN --duration NUM/DEN [--id CLIP_ID] [--json]
or timeline insert-clip --attach DESCRIPTOR --track TRACK_ID --media MEDIA_ID --at NUM/DEN --source-start NUM/DEN --duration NUM/DEN [--id CLIP_ID] [--json]
or timeline move-clip --project PATH --clip CLIP_ID --track TRACK_ID --at NUM/DEN [--json]
or timeline move-clip --attach DESCRIPTOR --clip CLIP_ID --track TRACK_ID --at NUM/DEN [--json]
or timeline delete-clip --project PATH --clip CLIP_ID [--json]
or timeline delete-clip --attach DESCRIPTOR --clip CLIP_ID [--json]
or timeline trim-clip --project PATH --clip CLIP_ID --edge start|end --to NUM/DEN [--json]
or timeline trim-clip --attach DESCRIPTOR --clip CLIP_ID --edge start|end --to NUM/DEN [--json]
or timeline split-clip --project PATH --clip CLIP_ID --at NUM/DEN [--id NEW_RIGHT_CLIP_ID] [--json]
or timeline split-clip --attach DESCRIPTOR --clip CLIP_ID --at NUM/DEN [--id NEW_RIGHT_CLIP_ID] [--json]
or timeline ripple-delete-clip --project PATH --clip CLIP_ID [--json]
or timeline ripple-delete-clip --attach DESCRIPTOR --clip CLIP_ID [--json]
or recovery status --file PATH [--json]
or recovery apply --file PATH [--json]
or recovery discard --file PATH [--json]
or session serve --file PATH [--descriptor PATH] [--json]
or session describe --attach DESCRIPTOR [--json]
or session shutdown --attach DESCRIPTOR [--discard-unsaved] [--json]
```

Headless commands and queries use `ProjectFileSession` and the shared application path. A real headless mutation saves through exact-base checked atomic persistence; a no-op does not rewrite the file. Timeline commands do not edit project JSON in the CLI. `--id` is optional for add-track and insert-clip; omitted IDs are generated as UUIDv4 before command construction and returned in success output, while supplied IDs must be canonical lowercase UUIDv4. Rational input is exact `NUM/DEN` (`i64` numerator and positive `u32` denominator); decimal seconds, timecode, and frame shortcuts are rejected. Recovery status reports `none`, `candidate`, `stale`, or `conflict`; apply and discard call the existing recovery APIs. Unresolved candidate/conflict/invalid recovery blocks mutable file-session opening.

Attached summary, rename, timeline commands/queries, undo/redo, save, describe, and shutdown require an explicit descriptor via `--attach`; there is no endpoint scanning. Mutations use the same application envelope and revision preconditions as headless operations, do not retry stale commands, and leave the shared live session dirty until explicit `project save`. Undo/redo require attachment because history is session-local and not persisted. `session serve` remains a developer/headless host; the Flutter application can also host a session and exposes its descriptor under Settings → Advanced / Developer. Neither host autosaves.

Media list is available against either a project file (`--project`) or attached descriptor (`--attach`); pagination defaults to the maximum 100-item page. Media add takes a source path. Headless add/remove open `ProjectFileSession`, execute the actual command, and safely save. Attached add first describes the live host, prepares/probes the source locally, then sends a normal `media.add` command with the captured revision; a concurrent edit returns `REVISION_CONFLICT` without retry. Attached remove uses the same semantic command path. IPC does not gain arbitrary filesystem access. `or media probe` remains read-only.

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

The Flutter bridge uses `flutter_rust_bridge` 2.13.0 with generated typed bindings in `packages/or_app_bridge` and a thin `crates/or_app_bridge` adapter. Its opaque `ProjectHostHandle` owns a `LiveProjectHost`; it does not expose mutable `ProjectDocument` state to Dart. Typed operations cover create/open, summary, rename, undo/redo, save, close, recovery inspection/actions, bounded media-page query, import, remove, media-preview request/read, a separate ordered artifact-event stream, descriptor path, and project invalidation events. Flutter keeps an immutable active-project read model and refreshes it from Rust after project events; mutations carry the read model's expected revision and do not auto-retry conflicts. The desktop `file_selector` plugin chooses project/media paths only; Rust owns canonical file validation, preparation, commands, and persistence. The Media panel shows filename, format, duration, video/audio summary, generated video-thumbnail or audio-only waveform PNG, and source/MediaId on demand, with 50-item pages over the core's 100-item page limit. Import requires system `ffprobe`; preview generation requires system `ffmpeg`. If the probe backend is missing, Flutter shows:

```text
Media probe backend is unavailable.
This Developer Preview currently requires a system-provided ffprobe.
```

New/Open and media-file path integration remain unavailable on Android pending SAF support. CI builds each target, runs Flutter widget and lifecycle tests, and exercises the native macOS offline-media bridge behavior.

Keep high-volume media transport separate from ordinary bridge messages. The bridge remains a control and ordinary structured-data path; do not send full-rate decoded video frames or large frame buffers as copied Dart objects. The render path should use a native/external display resource where supported and retain a correctness fallback.

## 10. Preview rendering and frame model

Rust and wgpu own render evaluation and output. Flutter presents a registered
external texture through one shared semantic viewer contract. The runtime owns
snapshot identity and `FrameLease`; a platform adapter owns texture registration,
native-resource lifetime, and synchronization. Use a bounded pixel-buffer path
as the supported desktop fallback and optional shared GPU surfaces only where
the platform/backend interop is validated. Minimize copies on the hot path;
universal zero-copy is not promised. The selected Flutter texture boundary is
consistent with the platform APIs for [Windows](https://api.flutter.dev/windows-embedder/flutter__windows__texture__registrar_8h_source.html),
[Linux](https://api.flutter.dev/linux-embedder/flutter__texture__registrar_8h_source.html),
and [Apple platforms](https://api.flutter.dev/macos-embedder/_flutter_external_texture_8mm_source.html),
while allowing separate native adapters.

The bridge may send an opaque registered Flutter texture identifier, dimensions,
pixel format, exact presentation time, transport state/errors, and controls.
It must never send per-frame pixels, full-rate frame bytes, or raw OS/GPU handles
through Dart, IPC, project state, or cache identity. Retain each `FrameLease`
until the native release callback or completion fence signals. The platform
adapter uses a bounded latest-frame mailbox/in-flight set and rejects frames
from stale generations or project revisions. Pixel-buffer fallback may require
a native copy/readback; frame data never takes a copied-Dart route.

Timeline/render evaluation should publish a stable read view such as `RenderSnapshot N` for canonical `ProjectRevision N`. A project mutation produces a view associated with the next revision, which workers can adopt safely. Render workers never mutate Project, and the architecture must not require them to lock mutable Project state continuously. The snapshot may be a full immutable evaluated structure, incremental graph, structurally shared data, versioned read model, or another measured strategy; snapshot granularity must be benchmarked rather than assumed to mean cloning the whole project for every edit.

Per-frame playback/render work is runtime execution over committed state. It must not dispatch project-edit commands, open Project transactions, or increment `ProjectRevision`. The native adapter selects a platform-appropriate synchronization primitive and a small bounded in-flight depth; the latest-frame mailbox replaces obsolete preview work. Measure the latency, throughput, memory, and GPU-occupancy tradeoffs when tuning that bound.

A frame should carry explicit dimensions, pixel or texture format, color information, and timing metadata. The exact representation remains implementation work.

### Phase 7F0 playback timing contract

The canonical `ProjectTimeline.sequence_frame_rate` is optional and explicit:
one exact `RationalRate` in frames per second. No project or active source
supplies an implicit default.
The schema-v5 `timeline.sequence.set_frame_rate` command accepts an exact rate
or `null`; the read-only `timeline.sequence.settings` query returns it. Both
use the existing validated application boundary and generic
`ApplicationRequest` IPC route, with operation schema v1 and no IPC protocol
version change. A real mutation is one validated project command, history
entry, and revision increment; an unchanged value is a no-op. Migration
preserves revision and does not dirty the project before explicit save. The 7F
viewer exposes a minimal explicit control to set or clear the rate, without
selecting a default. The recovery envelope stays v1.

For output index `n`, time is the exact rational `n / sequence_rate` from
timeline zero. All frame-index conversion uses checked integer/rational
arithmetic. At exact nonnegative playhead time `t`, next selects
`floor(t × rate) + 1`; previous selects `ceil(t × rate) - 1`, clamped to zero
and the last valid frame. Valid frame times are nonnegative and strictly before
the maximum exact clip end across audio/video tracks. The interval is
half-open; markers do not extend it. An empty timeline has no playable frames,
and playback does not loop. Previous at the end selects the last valid frame;
next at or beyond the end does not advance.

Seek and scrub preserve any exact nonnegative `RationalTime`, including
positions between sequence frames, in gaps, or beyond content end. Play and
frame-step require an explicit rate and return a typed unavailable result if it
is unset; seeking and scrubbing remain available. A gap or beyond-end seek
presents blank/neutral output. Play started at or beyond content end completes
without advancing. Scrub maps pointer position to exact timeline time, not a
frame or source index.

The output lattice is global across mixed source rates. At output time inside a
clip, map to `source_range.start + offset`, where the exact offset is
`output_time - clip.timeline_start`. Choose the preceding source presentation
timestamp, and hold that frame until the next timestamp; do not interpolate.
Apply the same rule to variable frame-rate sources. A still image holds across
its explicit clip duration. Audio-only clips extend content time and
participate in the 7E audio clock but do not create video frames. Active audio
output drives playback time through the 7E clock; without active output, use a
monotonic runtime clock anchored to the exact play/seek time.

### Phase 7A runtime foundation

The workspace `or_runtime` crate is the first runtime-plane implementation. It
depends only on the existing `or_core` path dependency and the Rust standard
library; it adds no wgpu, FFmpeg, audio, platform, native-interoperability, or
provider dependency. It inherits workspace MSRV Rust 1.85 and MIT licensing.
Its public contracts are runtime-only and are not serialized into `.orproj`,
IPC, cache keys, or Dart frame messages.

`RenderSnapshot` records the `ProjectId`, exact `ProjectRevision`, and exact
requested `TimeRange` captured from a `ProjectDocument`. It contains no mutable
project document and has no operation that can mutate canonical state. A
`FrameDescriptor` records memory domain, dimensions, pixel format, color
metadata, exact `RationalTime` timing, and access mode. `FrameLease` owns a
software byte buffer or a runtime-only release callback and releases exactly
once, including when dropped; platform handles remain outside this crate.

`BoundedQueue` has a fixed positive capacity. Nonblocking producers receive an
explicit `Backpressure` result, blocking push/pop operations are cancellable,
and close drains existing items before reporting `Closed`. `RuntimeBudgets`
keeps independent render, audio, and decode in-flight/byte reservations with
RAII release. `CapabilityRegistry` registers typed operation/provider/path
capabilities, starts with stable software providers for video decode, audio
decode, and render, and selects only available stable hardware paths when
preferred; otherwise it reports an observable software fallback. Hardware-only
selection is opt-in and fails when no stable hardware path exists.

7A stops at these contracts and deterministic unit coverage. It does not add
visible playback, a worker scheduler, decode/render/audio backends, native
surface interop, or platform capability discovery. Any later runtime or
platform dependency must carry official upstream version, MSRV, license/build,
and hosted platform evidence before it is pinned or used.

### Phase 7B render foundation

`or_render` owns the shared wgpu render spine while keeping project state,
media decode, and viewer integration outside this checkpoint. It exposes a
validated `PreviewSurface` abstraction, an opaque `ViewerSurfaceContract`,
immutable `RenderGraphInput` values tied to the 7A `RenderSnapshot`, and an
owned `RenderedFrame` whose wgpu texture is released by Rust RAII. The first
graph is a deterministic synthetic solid-color scene rendered to an offscreen
`Rgba8Unorm` target. `OffscreenReadback` is an explicit diagnostic/test and
future-encoder path; it is not the viewer transport and is never a Dart frame
message.

The dependency gate pins `wgpu = 25.0.2` exactly. The official package metadata
records MSRV Rust 1.84 and `MIT OR Apache-2.0` licensing, so it fits the OR
workspace MSRV Rust 1.85 and MIT distribution. The crate enables the native
`vulkan`, `gles`, `metal`, `dx12`, and `wgsl` features: Vulkan covers Windows,
Linux, and Android; GLES covers Windows, Linux, and Android; Metal covers
Apple platforms; and DX12 covers Windows. These platform/backend claims follow
the [official wgpu platform matrix](https://github.com/gfx-rs/wgpu#supported-platforms)
and the [pinned upstream manifest](https://github.com/gfx-rs/wgpu/blob/v25.0.2/wgpu/Cargo.toml).
The [published crate metadata](https://crates.io/crates/wgpu/25.0.2) is the
version/MSRV/license source used for the pin. Metal, DX12, Vulkan portability,
and other native interop remain runtime-adapter work for later checkpoints.

## 11. Media and render graph

FFmpeg is the intended baseline for media probing, demux, decode, encode, mux, conversion, and resampling. Checkpoint 7C0 selects the current high-level `ffmpeg-the-third` Rust binding, package version `6.0.0+ffmpeg-9.0`, with its paired `ffmpeg-sys-the-third` 6.0.0 binding layer. The maintained upstream release adds FFmpeg 9 support while retaining FFmpeg 5.1 through 8.1 compatibility, and exposes Rust wrappers for format/demux, codec/decode, frame, software-resampling, and software-scaling APIs. Its declared Rust 1.80 MSRV is below the workspace's Rust 1.85 floor. Both Rust packages declare WTFPL. The system-link path needs FFmpeg headers and shared libraries, `pkg-config`, a C compiler, and `clang`/`libclang` for runtime bindgen; the binding uses vcpkg discovery for MSVC. The `ffmpeg-sys` link step does not directly require CMake; the Linux/macOS FFmpeg source build uses `configure`/`make`. CMake/tool versions for a Windows vcpkg port are port-specific and must be fixed with that platform's later build gate. See the [published 6.0.0 package](https://crates.io/crates/ffmpeg-the-third/6.0.0), [upstream release history](https://github.com/shssoichiro/ffmpeg-the-third/blob/master/CHANGELOG.md), and [current build manifest](https://github.com/shssoichiro/ffmpeg-the-third/blob/master/ffmpeg-sys-the-third/Cargo.toml).

The approved baseline is FFmpeg 8.1.3 and the supported update line is 8.1.x, using matching headers and library ABI majors at build and runtime. Patch updates require the hosted binding link probe; changing FFmpeg minor/major lines or the Rust package requires a new compatibility, API, MSRV, and license review. FFmpeg 9 support exists in the binding, but the integration baseline follows the official stable 8.1 release line. Checkpoint 7C0 provisions a separate Linux CI prefix built from the official 8.1.3 source with shared libraries, `--disable-autodetect`, `--disable-everything`, and no `--enable-gpl`, `--enable-nonfree`, or `--enable-version3` options, then compiles, links, loads, and checks the native library ABI/license strings through an out-of-workspace probe. The CI-only build also uses `--disable-asm` to reduce build requirements. The 7C0 probe was tooling only. The production workspace now pins `ffmpeg-the-third` 6.0.0 with only codec, format, software-resampling, and software-scaling features. Its hosted prefix keeps the LGPL-only dynamic configuration and enables only the `file` protocol, Matroska demuxer, and FFV1/PCM S16LE decoders needed by the generated fixture; this small fixture codec set does not select the product codec set.

Dynamic linking is the selected packaging strategy. Build from the pinned FFmpeg source/configuration for each target and bundle the matching `libavcodec`, `libavformat`, `libavutil`, `libswresample`, and `libswscale` shared libraries plus their runtime dependencies. Include the corresponding headers and import metadata only in development/build environments. Linux packages ship the `.so` libraries with app-relative runtime lookup; macOS packages ship the `.dylib` libraries with app-relative loader paths and sign/notarize the complete bundle; Windows packages ship matching MSVC `.dll` files beside the app and use their `.lib` import libraries at build time. Release packages must include the exact corresponding FFmpeg source, build configuration, changes, and LGPL notices/source location; keep the FFmpeg shared libraries replaceable and preserve their library names. Android is a separate Phase 9 decision requiring NDK builds for supported ABIs and per-ABI shared-library packaging evidence.

Phase 5A uses an external `ffprobe` executable for metadata inspection. `probe_media_file(&Path)` accepts local regular files, invokes the executable through `std::process::Command` without a shell, and passes the canonical file path as one `-i` argument. It requests `-of json` and a narrow `-show_entries` set of format and stream fields, and CI records the tool with `ffprobe -version`. Output is limited to 1 MiB stdout and 64 KiB stderr, with a 15-second timeout and child cleanup. The parser ignores unknown external JSON fields, then validates the OR metadata representation. Missing, malformed, zero, or out-of-range required video dimensions and malformed or negative durations fail with a structured metadata error. Malformed optional frame rates and missing, malformed, zero, or out-of-range optional audio sample rates and channel counts become unavailable rather than invalid domain values; wrong-typed descriptive metadata such as channel layout fails validation. `RationalTime` parses decimal durations exactly with at most nine fractional digits, matching its `u32` denominator bound; valid frame rates use `RationalRate`. `MediaMetadata` is control-plane metadata and contains no path or arbitrary tags. Phase 5B's `prepare_media_import` probes and validates one selected local file, creates a canonical local `file:` URI and fresh `MediaId`, and returns a `MediaItem`; it does not mutate a project. A caller must submit it through `media.add`. Project load and `media.list` validate/display stored metadata without opening or probing sources. Neither phase links or bundles FFmpeg. Invocation syntax and fields follow the [official ffprobe documentation](https://ffmpeg.org/ffprobe.html).

Phase 5D adds library-preview generation through an external system `ffmpeg` executable. The default command is `ffmpeg`; `OR_FFMPEG_PATH` is a developer/test override and is never persisted or included in cache keys. The bridge converts the validated local `file:` URI to a native path and passes it as one direct process argument, without a shell. Thumbnail profile `Library Thumbnail V1` selects the first video stream and first decodable frame, scales within a 320-pixel maximum edge without upscaling while preserving aspect ratio, and emits a validated PNG; it does not search for a representative scene. Waveform profile `Library Waveform V1` selects the first audio stream, combines channels, and emits a white 512×96 PNG. The profile descriptors participate in parameter fingerprints. Thumbnail generation has a 20-second timeout; waveform generation has a 30-second timeout; stdout is limited to 8 MiB, stderr to 64 KiB, and the process is polled every 10 ms. Timeout, cancellation, overflow, and read failure kill and reap the child. These are disposable Media-panel images, not timeline waveform data, decode, playback, or render output. They remain on the external-executable path and do not use the linked `or_media` runtime.

The media boundary must support both software decode and hardware-surface decode. A centralized capability/provider layer should report usable paths for automatic selection and safe fallback; generic timeline/domain code must not accumulate platform-specific conditionals. Possible platform directions are examples only, not selections: VideoToolbox/platform video surfaces on macOS; platform hardware decode and D3D-compatible surfaces on Windows; VAAPI, Vulkan, or DMABUF-style interop on Linux; and MediaCodec with hardware-buffer or native-surface paths on Android. Support varies by codec, device, pixel format, driver, and backend.

Pipeline selection should choose the best supported and stable path for the actual codec, pixel format, resolution, backend, device, driver, platform, and operation. A hardware path is not presumed faster. Prefer hardware decode and minimal-copy GPU processing where they benefit the workload, and preserve software/CPU paths as correctness fallbacks when decode, GPU interop, or drivers are unavailable or unstable.

Phase 7D evaluation: no hardware decode or native-frame interop path is approved. The current `or_media` decoder creates a software FFmpeg decoder from stream parameters and emits owned CPU RGBA frames. Its manifest enables codec/format plus software resampling and scaling; the bounded hosted FFmpeg fixture build enables only file input, Matroska, FFV1, and PCM S16LE. `or_render` has no native viewer-surface adapter, and the runtime capability/lease contracts have no platform adapter or native handle implementation. The repository contains no repeatable target-hardware comparison or platform-specific build, license, and packaging evidence for a candidate path. Keep software decode as the correctness path and make no hardware performance claim. Reconsider a candidate only with platform/device/driver/codec/backend coverage, build and license/package evidence, native resource lifetime and synchronization validation, software fallback coverage, and repeatable measurements on target hardware.

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

Decode audio through `or_media`; `or_audio` defines the device-neutral output boundary. Its preallocated single-producer/single-consumer interleaved buffer is bounded and non-blocking: producers receive backpressure when full, while the callback fills missing frames with silence and reports the underrun. The callback advances the audio master clock by device frames, including silence, and returns an exact clock message bound to its `RenderSnapshot` for video pacing. Timeline-to-device conversion subtracts the snapshot seek origin, multiplies exact rational seconds by the integer device sample rate, then floors to the frame at or before that time. Video drops frames more than the configured drift tolerance behind audio and waits when they are ahead; frames within the tolerance may present. Synchronization rejects clock messages from another project or revision while allowing each worker to request its own exact time range.

The callback does not allocate, lock, call Flutter or providers, or access canonical project state. Cancellation silences and drains queued data; a seek starts a fresh clock message at its exact timeline origin. No device library or platform output adapter is selected in 7E, so the crate adds no third-party audio dependency. A hardware backend requires a separate platform, build, MSRV, and license gate before selection. Core gain, pan, fades, and future DSP live in the audio engine, not in Flutter presentation code.

Final video text is rendered by the render core. Exported text must not depend on Flutter widget rendering. Select a font shaping and rendering dependency only after cross-platform behavior and licensing are evaluated.

## 13. Background jobs and cache

Phase 5A implements `JobId` (validated UUIDv4), `JobKind::MediaProbe`, and `JobState` values (`Queued`, `Running`, `Succeeded`, `Failed`, and `Cancelled`). `JobKind::MediaProbe` remains synchronous; Phase 5D adds concrete thumbnail and waveform job kinds for generated library previews, and Phase 5F adds `JobKind::ProxyGenerate` (serialized as `proxy_generate`).

Phase 5C implements a bounded `JobManager` over Rust standard-library threads (`crates/or_core/src/jobs/manager.rs`). The caller passes an explicit, non-zero `JobManagerConfig` (`max_workers`, `queue_capacity`, `max_records`); the manager spawns exactly `max_workers` worker threads and never one thread per job. The pending queue is bounded by `queue_capacity`, and `submit` is non-blocking, returning `JobSubmitError::QueueFull` when it is full. Up to `max_records` job records are tracked; when capacity is needed, only terminal (`Succeeded`/`Failed`/`Cancelled`) records are evicted oldest-first by a manager-local monotonic `sequence` (no wall-clock ordering); if every record is non-terminal, a submit returns `JobSubmitError::RecordCapacityExceeded`. Cancellation is cooperative through `JobContext::is_cancelled`: a queued job is removed before execution and never runs, and a running job is signalled and becomes `Cancelled` when it acknowledges. A task that returns `Err(JobFailure)` is `Failed`; a panicking task is caught at the job boundary and marked `Failed` without killing the worker. `shutdown` (also invoked on `Drop`) stops accepting submissions, cancels queued records, signals running records, and joins every worker thread. Reporting is read-only `JobSnapshot` (`id`, `kind`, `state`, `sequence`). Phase 5C adds no realtime scheduler, priority API, progress/result API, or job persistence, and the manager has no access to project state or `ProjectRevision`.

Phase 5C implements a disposable filesystem `CacheStore` (`crates/or_core/src/cache.rs`). The caller supplies an explicit root and a non-zero `CacheStoreConfig` (`max_entry_bytes`, `max_total_bytes`). `CacheArtifactKind::{Thumbnail, Waveform}` remain bounded byte artifacts; Phase 5F adds `Proxy` as a file-backed artifact. A `CacheKey` is a deterministic SHA-256 digest over `CACHE_SCHEMA_VERSION = 1`, the artifact kind, a `SourceFingerprint`, and a `ParametersFingerprint`, displayed as 64 lowercase hexadecimal characters; `DefaultHasher` is not used. Existing thumbnail/waveform tags and keys remain unchanged; Proxy uses a distinct stable tag. Preview paths remain `<root>/<namespace>/<first-two-hex>/<key>.cache`; the proxy path is `<root>/proxy/<first-two-hex>/<key>.mkv`. No caller path segment or source filename enters a final path. Thumbnail/waveform entries over `max_entry_bytes` are rejected before filesystem mutation, and their byte reads stay bounded to `max_entry_bytes + 1`; the generic byte API rejects Proxy. Phase 5E's indexed global-budget behavior is described below. Preview writes use a same-directory `create_new` temp file, flush, file sync, and rename; explicit `remove`, `clear_namespace`, and `clear_all` work across artifact kinds. Phase 5D acquires bounded source-fingerprint v1 from a native regular file's size, available modified time, and sampled bytes under the versioned domain marker `opencut-reinforced-source-fingerprint-v1`. Files up to 1 MiB are sampled completely in 256 KiB windows; larger files use up to four 256 KiB windows at the start, one-third, two-thirds, and tail, for a 1 MiB maximum. File size, modification-time availability/value, and sample offsets/bytes enter the SHA-256 digest. This is for cache invalidation only, not full-file identity or an integrity guarantee. `MediaArtifactService` deduplicates same-key in-flight work and stores validated PNG bytes through the same CacheStore. The desktop bridge configures two workers, queue capacity 32, 128 job records, an 8 MiB byte-entry limit, and a 256 MiB total cache budget; Phase 5F does not change these defaults and Android does not initialize this desktop artifact service. Desktop cache roots are `~/Library/Caches/Opencut-Reinforced/media-artifacts` on macOS, `$XDG_CACHE_HOME/opencut-reinforced/media-artifacts` (or `~/.cache/opencut-reinforced/media-artifacts`) on Linux, and `%LOCALAPPDATA%/Opencut Reinforced/Cache/media-artifacts` on Windows. Cache-service initialization is opportunistic: project create/open succeeds when a cache root is unavailable, and artifact requests report cache unavailability. Cache contents are never canonical project state and are never stored in `.orproj`, the recovery sidecar, or history. Phase 5C did not yet include automatic eviction or a persistent cache index; Phase 5E adds both below.

Phase 5E keeps the existing `CacheStore` API and artifact paths, adding a private, lazy SQLite index at `<cache-root>/cache-index.sqlite3`. `CACHE_INDEX_SCHEMA_VERSION = 1` sets `PRAGMA user_version = 1`; schema v1 has `cache_meta(id, next_access_sequence)` and `cache_entries(kind, cache_key, size_bytes, last_access_sequence)` with a primary key on kind/key and an LRU index on sequence, kind, and key. The index stores no source path, `MediaId`, project ID, or project content. It recognizes thumbnail/waveform `.cache` paths and Proxy `.mkv` paths and reconciles them on first use, preserving known sequence values, discovering legacy artifacts at sequence zero, removing stale rows, and repairing size drift. Corrupt, unsupported, or inconsistent metadata is discarded and rebuilt from a bounded scan of at most 100,000 managed artifacts; symlinks are not followed. A successful byte `get`/`put`, proxy typed hit, or artifact commit touches a positive signed-64-bit access sequence, which persists across normal restarts and never uses wall clock time. Ties sort by kind then cache key. Indexed artifact-byte totals apply one global budget and evict only the minimum oldest set needed for space, protecting the replacement target; an entry larger than the budget returns `BudgetExceeded` without clearing existing entries. Proxy effective generation size is bounded by `min(PROXY_MAX_ARTIFACT_BYTES, max_total_bytes)` before and during FFmpeg execution. Database and journal bytes are not budgeted. SQLite uses ordinary rollback journaling, no WAL, a one-second busy timeout, and immediate write transactions for cache mutations. SQLite and filesystem changes are not one atomic transaction: startup reconciliation and targeted reads/writes repair mismatches because the cache and index are disposable and project correctness never depends on either.

Phase 5F adds `MediaArtifactService::request_proxy` and a file-backed `CacheStore` path; this remains core-only and does not request generation during import or expose proxy requests through Flutter, CLI, or IPC. A Proxy V1 key uses the unchanged `SourceFingerprint` with this exact parameters descriptor, which participates in `ParametersFingerprint`:

```text
opencut-reinforced-proxy-v1
container=matroska
video_codec=mpeg4
max_width=960
max_height=540
upscale=false
square_pixels=true
pixel_format=yuv420p
qscale=6
gop=12
bframes=0
fps_mode=passthrough
pts=start_at_zero
audio=none
metadata=none
```

The first video stream (`0:v:0`) is encoded by the system `ffmpeg` executable using its native MPEG-4 Part 2 `mpeg4` encoder and Matroska muxer, with no codec fallback, linked library, bundled executable, or hardware path. The filter normalizes PTS with `setpts=PTS-STARTPTS`, preserves input frame cadence with `-fps_mode passthrough`, scales within 960×540 without upscaling, preserves display aspect ratio, makes dimensions even, resets sample aspect ratio to square pixels, and converts to `yuv420p`. It uses `-q:v 6`, `-g 12`, `-bf 0`, disables audio/subtitle/data streams, and clears global/stream metadata and chapters. Scale options follow the [official FFmpeg filters documentation](https://ffmpeg.org/ffmpeg-filters.html); in particular, systems whose FFmpeg build predates `scale=reset_sar` report a generation failure rather than using a different profile. The service invokes the executable through direct process arguments, with stdin/stdout null and stderr bounded to 64 KiB. It polls the same-directory hidden `.key.or-proxy-tmp-<uuid>.mkv` staging file while FFmpeg runs, terminates and reaps the process when cancellation, timeout, or the effective output-size maximum is reached, validates non-empty Matroska output, then atomically renames the file into its derived cache path and updates the shared index. Staging is removed on normal success, failure, timeout, cancellation, and oversize; an abrupt process or OS crash may leave a hidden staging file, which reconciliation ignores and does not remove because another process may own an active stage. The timeout is three times the ceiling of known source duration in whole seconds, clamped to 120–7200 seconds, or 1800 seconds when duration is unknown. Proxy output is a disposable future decode source, not canonical/project/export media or a current playback source; Phase 5F adds no decoder, timeline, playback, audio, or render graph.

Scheduling must support bounded concurrency, cancellation, backpressure, and deliberate CPU and memory budgets without unbounded worker creation. Keep conceptual classes for latency-sensitive realtime work (audio output, immediately needed playback decode, render/present), interactive work (commands, timeline queries, scrubbing, inspector updates), and background work (thumbnails, waveforms, proxies, indexing, AI analysis, downloads). Playback-critical work must be able to take priority over opportunistic background work, and AI/background work must not starve playback; exact priority names and implementation are not fixed.

Producers must not outrun consumers indefinitely. Frame/decode and export queues, thumbnail work, and AI/background work must stay bounded. When a consumer is slower, the system may pause production, reduce queue depth, drop obsolete preview work where safe, or block a background producer appropriately. The exact queue type and policy are workload decisions.

Hot media paths should avoid repeated large allocations where practical. Bounded frame, texture, audio-buffer, decode-surface, and render-target reuse are candidates, not selected implementations; pools must remain bounded and must not turn into an unbounded cache. Establish system-level budgets for RAM, GPU memory or equivalent render resources, decoded-frame cache, thumbnail cache, waveform cache, and proxy/cache storage. Budgets may vary by device class, available memory, platform, and workload, with more conservative policy on Android; fixed percentages and values are deferred.

Cache keys are deterministic over source fingerprint, operation, parameters, and cache schema version. Phase 5E selects `rusqlite` 0.40.1 with bundled SQLite for the disposable cache index only; this does not change `.orproj` project storage. Cache contents are disposable and never authoritative project state. Provide bounded storage, eviction and regeneration paths, and a clear-cache operation; cache presence must never be required for project correctness.

Workers may produce structured `JobResult`, `GeneratedAsset`, `AnalysisResult`, `CaptionProposal`, or `EditProposal` outputs. They may update disposable cache/job state but must not directly mutate the canonical ProjectDocument. If an output should change a project, the application layer checks its permissions and expected revision/preconditions, then applies it through a validated command/transaction. This preserves undo/redo and rejects stale analysis instead of applying it silently.

## 14. AI providers and local inference

Define capability-oriented adapters for transcription, translation, text-to-speech, LLM planning, segmentation, image generation, video generation, declarative MotionScene generation, and audio generation. Local and optional cloud implementations plug into the same task-oriented contracts. `GenerateVideo` returns an opaque raster/video asset; future `GenerateMotionScene` returns an editable declarative scene proposal. Do not hard-code an editor workflow to one provider.

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

## 17. Templates, themes, MotionScene, and community

Templates are declarative packages with stable editable slot IDs, dependency manifests, checksums, and license metadata. They contain project structure and values, not arbitrary executable code.

Themes are declarative semantic token sets. They cannot include JavaScript or arbitrary CSS, execute code, redefine application behavior, or replace the layout architecture.

Initial community distribution can use a GitHub-first static registry: manifest repository, versioned entries, release or download assets, automated validation, and pull-request-based publishing. Do not build a community backend now; early phases do not require one.

### Declarative MotionScene

The future `.ormotion.json` source is a versioned, non-executable data format
for simple motion graphics. Its conceptual V1 envelope contains `format`,
`schema_version`, UUIDv4 `scene_id`, `name`, exact `duration`, `canvas`, typed
`assets`, `nodes`, and `animations`. Phase 11E freezes exact names and bounds
after the Phase 7/8 renderer exists; this planning document does not add codec
types or a project schema.

MotionScene uses nonnegative finite `RationalTime`, a half-open `[0, duration)`
local timeline, exact-time random-access evaluation, typed local asset
references, stable identities, checksums/provenance, and reproducible font
references or a documented bundled fallback. It has no HTTP, environment,
glob, shell, provider, script, arbitrary shader, or hidden font lookup. The
initial primitive set is `Group`, `Text`, `Rectangle`, `Ellipse`, `Line`/
`Arrow`, renderer-gated `Polyline`, and `Image`; animation is limited to typed
translation, scale, rotation, opacity, bounded geometry, and supported
fill/stroke values with a small locked easing set. Charts and diagrams start as
typed templates over primitives. Any later randomness is explicit and seeded
from scene/source identity; V1 should otherwise be deterministic.

The canonical path is:

```text
MotionScene -> validate -> evaluate exact time
            -> typed instructions / RenderSnapshot -> existing or_render/wgpu
            -> preview or normal OR materialization/export
```

Preview and materialization share the evaluator. Phase 11F is
materialization-first: render persistent generated media, preserve source and
asset provenance, register it via normal media commands, and use ordinary
timeline clips. A re-render never silently replaces an existing canonical
`MediaId`. Cache identity may include source/asset hashes, evaluator/render
version, dimensions, frame rate, and color settings, but a timeline-referenced
materialized asset is not cache-only. There is no live `MotionClip` without a
separate architecture/schema gate.

External agents and templates are first-class producers. Future semantic
commands may be `or motion validate`, `inspect`, and `render`; validation,
inspection, and render are read-only unless an explicit materialization/project
command is requested. Rendering requires zero LLM/provider calls. TTS, music,
and SFX remain normal OR audio/media/timeline concerns, coordinated by an
agent or `EditPlan` rather than embedded in MotionScene.

Remotion and Motion Canvas are useful conceptual references, not canonical
runtimes or dependencies. Lottie/dotLottie are future bounded interchange,
not the canonical format. Chromium/headless browser use is reserved for the
optional Phase 16G WebMotion adapter, never the normal renderer. WebCodecs is
not the encoding authority; the normal OR encoder remains the path for
materialization.

### Optional procedural WebMotion boundary

Future `WebMotionBundle` content may support custom HTML/CSS, Canvas, SVG,
WebGL/WebGPU, Three.js-style scenes, or simulations only after an explicit
request. It runs in an isolated sidecar/browser with an OR-owned
least-privilege outer boundary, network off, read-only bounded approved inputs,
bounded scratch/output, no credentials or arbitrary project writes, resource
and output limits, cancellation, crash containment, and structured diagnostics.
It never runs in `or_core`, Flutter, normal project open, or canonical motion
evaluation, and it returns only bounded output for materialization. CSP and a
browser sandbox are defense in depth; production must not disable the browser
sandbox as a GPU workaround. A pinned browser, if later selected, is
capability-managed and recorded in provenance but is not sufficient isolation
by itself.

## 18. Plugins

Prefer a sandbox and explicit capability permissions. A WASM/WASI-style runtime is a candidate for future plugin work, not a permanent selection. Refresh runtime support, escape analysis, permissions, and dependency research before plugin implementation.

Native and OpenFX compatibility is later and has a higher trust cost. Do not treat installed plugins as unrestricted trusted code by default.

## 19. Export and interchange

Export uses the same timeline and render evaluation as preview. The intended flow is offscreen render frames to a media encoder and muxer, managed as a background job with progress and cancellation. Where supported, prefer a GPU/native-compatible render surface into a hardware encoder; otherwise use CPU frames with a supported software or platform encoder. Do not require a GPU readback/upload cycle when a stable shared-surface path is available. Codec and hardware options depend on platform support and licensing review, and correctness fallback remains first-class.

Checkpoint 8F owns and locks the export request, job status/progress, and
cancellation contract through the existing application/IPC path before the
exporter implementation begins. Export is runtime work and does not mutate the
canonical project. The container/codec profile and packaging license decision
are also selected by 8F before first use; hardware encoding remains optional.

The native OR format is not OpenTimelineIO. OTIO is an import/export interchange format and API for editorial cut information, not the native project database and not a media container. Lottie and dotLottie may be evaluated as bounded motion interchange in Phase 16, but neither is the canonical MotionScene format. Select adapters and supported fields when an interchange implementation is scoped.

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

- exact software codec/demuxer set within the approved FFmpeg 8.1.x dynamic-link strategy; export codecs and packaging are selected by 8F before first use
- native implementation details within the 7F0 external-texture contract; its bounded pixel-buffer fallback is required on supported desktop targets, with shared GPU surfaces optional behind platform adapters
- exact Flutter state management framework and state-change event schema, event bus/library, and transport
- text-shaping dependency, selected and verified by 8D before first text render
- audio-output backend, selected and verified by 8E before first device output
- exact storage crate for any future persistence subsystem beyond the Phase 5E cache index
- exact Flutter localization package and generated resource format
- exact GPU image-comparison tolerance metric
- exact immutable render snapshot representation, granularity, and update strategy
- exact hardware decode API on each platform
- exact hardware encode API on each platform
- exact FFmpeg hardware-frame integration
- concrete per-platform CPU/GPU/decoder-surface resource wrappers and native handle types; semantic `FrameDescriptor`/`FrameLease` ownership is fixed
- validated optional shared-surface interoperability fast paths; Flutter external-texture presentation and the pixel-buffer fallback are fixed by 7F0
- platform-specific synchronization primitive selected by the adapter; 7F0 fixes synchronization ownership and lease lifetime
- numeric queue depth and per-mode buffering tuning; 7F0 requires a bounded latest-frame mailbox/in-flight set and stale-frame rejection
- exact worker scheduler/runtime, thread-pool implementation, and priority API
- exact frame, texture, audio-buffer, decode-surface, and render-target pool implementation
- exact RAM, GPU/resource, and cache budget values and adaptation policy
- exact performance thresholds and benchmark hardware
- exact centralized runtime hardware capability schema/provider
- timeline track naming and track mute/solo/lock behavior
- video compositing precedence and clip linking/grouping
- alternate stream selection and clip speed changes
- transitions, effects, transforms, crop, opacity, audio gain, and pan semantics
- marker schema/details, snap algorithm, and timeline UI interactions
- decode/render scheduling and quality details beyond 7F0's exact sequence timing, source-sampling, viewer lifetime, and stale-frame contract
- exact hardware/software path selection thresholds by codec, format, device, driver, and operation
- exact CPU/GPU operation partition, based on profiling and task characteristics
- exact CPU SIMD/intrinsic implementations and feature dispatch
- exact performance instrumentation implementation
- exact audio callback buffer/ring design and buffering policy
- future proxy-cache weighting, pinning, and namespace-quota policy
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
- [FFmpeg license and legal considerations](https://ffmpeg.org/legal.html) describes LGPL defaults, optional GPL components, dynamic-linking guidance, and source redistribution obligations. Checkpoint 7C0 pins the probe to [ffmpeg-the-third 6.0.0](https://crates.io/crates/ffmpeg-the-third/6.0.0) and [FFmpeg 8.1.3 source](https://ffmpeg.org/releases/ffmpeg-8.1.3.tar.xz); the packaged configuration and codec set still require release-specific review.
- [ffprobe documentation](https://ffmpeg.org/ffprobe.html) defines its command-line options, JSON output, field selection, input argument, and version reporting.
- [OpenTimelineIO](https://github.com/academysoftwarefoundation/opentimelineio) describes an editorial interchange format and API.
- [whisper.cpp license](https://github.com/ggml-org/whisper.cpp/blob/master/LICENSE) covers the runtime source, not every model artifact.
- [ONNX Runtime license](https://github.com/microsoft/onnxruntime/blob/main/LICENSE) covers the runtime source, not model artifacts.
- [llama.cpp license](https://github.com/ggml-org/llama.cpp/blob/master/LICENSE) covers the runtime source, not model artifacts.
