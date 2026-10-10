# Technical Plan

## Execution lock and implementation order

The active outcome-based work plan is
[docs/PRODUCT_ROADMAP.md](PRODUCT_ROADMAP.md), with evidence-backed adoption
decisions in [docs/OPEN_SOURCE_CONVERGENCE.md](OPEN_SOURCE_CONVERGENCE.md).
`docs/execution/PLAN.json`, `STATE.json`, and locked phase contracts preserve
the old roadmap requirements and evidence; their `NEXT` cursor is not active
after the 2026 operator pivot. This document records architecture and subsystem
contracts. Retain real schema, security, persistence, platform, and API
dependencies; do not create speculative crates or preserve artificial phase
ordering.

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

Phase 3 implemented the bootstrap subset: a Rust workspace and `or_core`, semantic CLI commands, a Flutter shell, and typed `flutter_rust_bridge` 2.13 bindings for application info, health, and capabilities. Phase 4A–4F and 4UI-2 provide the project/application, persistence, recovery, IPC, CLI, and desktop live-host foundations. Phase 5A–5F provide typed media/jobs, external `ffprobe`, disposable previews, indexed cache, and core-only file-backed Proxy V1. Phase 6 provides canonical tracks and clips, exact trim/split/ripple editing, persistent markers, marker-aware snapping, CLI parity, and the corresponding Flutter UI. Phase 7A provides the standalone dependency-free `or_runtime` contracts for immutable snapshot identity, exact-time frame descriptors and leases, bounded cancellation/backpressure, render/audio/decode budgets, and centralized software-first capability selection. Phase 7B adds the headless `or_render` wgpu spine, deterministic synthetic offscreen rendering, readback normalization, and a handle-only viewer contract. Phase 7C establishes linked software decode; 7F0 locks exact sequence timing, 7F1 proves desktop FFmpeg packaging and texture adapters, 7F delivers desktop video preview and transport, and 7G adds deterministic runtime budgets. The 7H checkpoint hardens cross-platform conformance and the first Developer Preview gate. Checkpoint 8C adds bounded transform, crop, and opacity editing plus preview evaluation. The 8D implementation path adds typed title/manual-caption editing and bundled-font preview rendering. The 8E implementation adds a bounded cpal desktop output path, typed gain/pan/fades, and a closed visual effect/transition set evaluated in preview; 8F owns export parity and release hardening. Hardware decode remains unapproved. See [docs/execution/STATE.json](execution/STATE.json) for the mutable execution status, [ARCHITECTURE.md](ARCHITECTURE.md), and [ROADMAP.md](ROADMAP.md) for design and human roadmap context.

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
23. [Frozen decisions and owned open details](#23-frozen-decisions-and-owned-open-details)
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

The current `.orproj` schema is v7. Its envelope keeps the format marker and project identity/revision/name, then stores ordered `media`, `timeline.tracks`, global `timeline.markers`, and the required nullable `timeline.sequence_frame_rate`. `ProjectDocument` owns a typed UUIDv4 `ProjectId`, persistent `ProjectRevision`, UTF-8 name, ordered media items, and canonical `ProjectTimeline`. Private versioned DTOs and explicit conversion keep domain changes from silently changing the file contract. The decoder strictly accepts v1–v7; v6 validates closed track/content enums, exact clip timing, track state, and bounded typed settings; v7 adds the typed `FileUri` / `AndroidSafDocumentUri` source union. All versions reject unknown fields. The Rust encoder emits deterministic pretty v7 JSON with a trailing newline; this is not a cross-implementation canonical JSON standard. `ProjectInstanceId` is runtime-only and is never persisted. The version probe rejects unsupported versions before decoding. Bounded load remains 64 MiB. Migrations from v1–v4 set the sequence rate to `null` and preserve project ID, revision, name, and existing media/timeline state. V5 preserves its nullable sequence rate and migrates media clips with their exact source duration and default typed settings. A clean open never rewrites the file, and the next explicit save writes v7 without a conversion-only revision increment.

7F0 implements schema v5 with one strict `timeline.sequence_frame_rate` field
encoded as either `null` or an exact `RationalRate`. Strict migrations from
v1–v4 set the rate to `null`; new projects also begin unset. Recovery remains
envelope v1 and accepts nested v5 snapshots. Opening a migrated project stays
clean and leaves disk bytes unchanged until explicit save.

The Phase 8A project-schema gate starts from the verified v5 baseline and increments it exactly once. It generalizes the one canonical Rust timeline
to closed Video, Audio, Text, and Caption track kinds and closed Media, Text,
and Caption clip content. Every clip retains stable `ClipId`, exact start, and
positive exact duration; media clips preserve `MediaId` and exact source range,
with migrated duration equal to the old source-range duration. Text and
caption clips have no synthetic media reference or source range. Persistent
typed state also includes lock, visibility, mute, solo, transforms, crop,
opacity, basic text formatting, audio gain/pan/fades, and basic transition and
effect references. Transient selection, hover, zoom, viewport, and drag state
remain presentation-only. Desktop playback speed stays 1x through MVP; 13B
owns later speed and rate mapping.

Checkpoint 8C reuses the persisted visual settings without changing schema v6.
The Inspector edits integer fixed-point values through the typed bridge and the
existing application update command; it preserves clip content, timing,
effects, and transitions. The renderer evaluates position, scale, rotation,
anchor, crop, and opacity from each clip's typed settings. Invalid numeric
ranges and crop sums are rejected before project mutation; Reset restores the
identity settings.

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

The recovery format is a separate versioned sidecar beside the `.orproj` file. It prefixes the project filename with `.` and appends `.or-recovery` (for example, `/projects/movie.orproj` uses `/projects/.movie.orproj.or-recovery`). Its strict envelope uses format marker `opencut-reinforced-recovery`, schema version 1, and contains two `.orproj` snapshots (v1–v7): the exact saved base at revision N and the newer unsaved recovery snapshot at revision M, where M > N. The recovery file is bounded to 136 MiB; each nested project is decoded and validated by the project codec and remains subject to the 64 MiB project limit. Cross-schema base/recovery combinations are supported and tested. This snapshot checkpoint is the initial recovery representation and may evolve after real scale measurements.

Writing a checkpoint requires matching `ProjectId` values, a newer recovery revision, and an on-disk canonical `ProjectDocument` exactly equal to the supplied base. The write uses the same same-directory atomic replacement primitive as project saves and does not mutate the canonical project. The recovery envelope remains v1; nested project snapshots can use project schema v1–v7, including persistent markers, sequence settings, and typed timeline content. Runtime `ProjectInstanceId` and session history are not stored.

Inspection reads and classifies without changing files or project state. If the canonical project exactly equals base N, the recovery is a candidate. If it equals the recovery snapshot or has the same project ID with a revision newer than M, the checkpoint is stale. A different project lineage or other state that cannot prove the exact base is a conflict; a missing canonical file is an orphaned conflict. Project loading does not inspect or apply recovery automatically, and conflicts never select a winner silently.

Applying is explicit and re-inspects the current canonical file before saving. It atomically saves recovery M through the existing project storage API, keeps revision M unchanged, then removes the sidecar. A save failure preserves the checkpoint; a cleanup failure after a successful save is reported as cleanup pending. Explicit discard removes the sidecar, including a malformed one, without changing the canonical project. Phase 8F uses this snapshot foundation for periodic autosave and explicit recovery UI; it is not event sourcing, command replay, or persistent history.

The UI passes a separate recovery-discard policy with shutdown. Under the host lock, it removes a candidate only when its contents still match that session's last autosaved snapshot; replaced, unrelated, or conflicting sidecars are preserved, and cleanup failure keeps the session open. Host shutdown's `discard_unsaved` permission alone only allows dirty session teardown and preserves recovery data, including during process-death simulation.

### Phase 4F file-backed session and shared dispatch

`ApplicationRequest` carries an existing `CommandEnvelope`, `QueryEnvelope`, or `TransactionEnvelope`; `ApplicationResponse` carries the corresponding result or the existing `OperationError`. `ProjectSession::handle_application_request` delegates to the existing command, query, and transaction methods. This is the common semantic path for the headless CLI and IPC server; it does not introduce a second editing implementation.

`ProjectFileSession` owns a project path, a live `ProjectSession`, and the exact `ProjectDocument` last known to be saved. Opening loads the canonical `.orproj`, inspects recovery, then creates a fresh runtime instance without incrementing revision. `NONE` and `STALE` allow opening; candidate, conflict, and invalid recovery require explicit attention. Save re-inspects recovery and reloads the disk document; the save proceeds only when the disk document exactly equals the remembered saved base. It then uses the existing atomic save path and does not increment revision. Dirty state is the in-memory project compared with that exact saved document; neither dirty state, instance ID, nor history is persisted.

### Project revisions

`ProjectRevision` is a persistent canonical project-state value backed by an unsigned 64-bit integer. Phase 4A implements its initial value, zero, and checked increment; the v1–v7 codecs preserve the stored revision during encode/decode. A changed rename, media add/remove, marker edit, sequence-rate edit, or transaction increments once; an exact no-op, failed operation, net-no-op transaction, and read-only query leave the revision unchanged. Undo and redo restore content through new canonical mutations, so they increment from the current revision rather than moving it backward. Schema migration during explicit save does not increment revision. Project ID and revision survive save/reopen; each fresh runtime open receives a new ephemeral `ProjectInstanceId`, which is excluded from the project document.

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
or media relink --project PROJECT_PATH --id MEDIA_ID --source MEDIA_PATH [--json]
or media relink --attach DESCRIPTOR --id MEDIA_ID --source MEDIA_PATH [--json]
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
or timeline add-track --project PATH --kind video|audio|text|caption [--id TRACK_ID] [--json]
or timeline add-track --attach DESCRIPTOR --kind video|audio|text|caption [--id TRACK_ID] [--json]
or timeline set-track-state --project PATH --track TRACK_ID --locked BOOL --visible BOOL --muted BOOL --solo BOOL [--json]
or timeline set-track-state --attach DESCRIPTOR --track TRACK_ID --locked BOOL --visible BOOL --muted BOOL --solo BOOL [--json]
or timeline remove-track --project PATH --track TRACK_ID [--json]
or timeline remove-track --attach DESCRIPTOR --track TRACK_ID [--json]
or timeline insert-clip --project PATH --track TRACK_ID --media MEDIA_ID --at NUM/DEN --source-start NUM/DEN --duration NUM/DEN [--id CLIP_ID] [--json]
or timeline insert-clip --attach DESCRIPTOR --track TRACK_ID --media MEDIA_ID --at NUM/DEN --source-start NUM/DEN --duration NUM/DEN [--id CLIP_ID] [--json]
or timeline insert-text --project PATH --track TRACK_ID --at NUM/DEN --duration NUM/DEN --text TEXT [--id CLIP_ID] [--json]
or timeline insert-text --attach DESCRIPTOR --track TRACK_ID --at NUM/DEN --duration NUM/DEN --text TEXT [--id CLIP_ID] [--json]
or timeline insert-caption --project PATH --track TRACK_ID --at NUM/DEN --duration NUM/DEN --text TEXT [--id CLIP_ID] [--json]
or timeline insert-caption --attach DESCRIPTOR --track TRACK_ID --at NUM/DEN --duration NUM/DEN --text TEXT [--id CLIP_ID] [--json]
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

Headless commands and queries use `ProjectFileSession` and the shared application path. A real headless mutation saves through exact-base checked atomic persistence; a no-op does not rewrite the file. Timeline commands do not edit project JSON in the CLI. `--id` is optional for add-track and clip insertion; omitted IDs are generated as UUIDv4 before command construction and returned in success output, while supplied IDs must be canonical lowercase UUIDv4. Timeline track and clip queries return the typed schema-v2 read models. Rational input is exact `NUM/DEN` (`i64` numerator and positive `u32` denominator); decimal seconds, timecode, and frame shortcuts are rejected. Recovery status reports `none`, `candidate`, `stale`, or `conflict`; apply and discard call the existing recovery APIs. Unresolved candidate/conflict/invalid recovery blocks mutable file-session opening.

Attached summary, rename, timeline commands/queries, undo/redo, save, describe, and shutdown require an explicit descriptor via `--attach`; there is no endpoint scanning. Mutations use the same application envelope and revision preconditions as headless operations, do not retry stale commands, and leave the shared live session dirty until explicit `project save`. Undo/redo require attachment because history is session-local and not persisted. `session serve` remains a developer/headless host; the Flutter application can also host a session and exposes its descriptor under Settings → Advanced / Developer. The Flutter shell schedules recovery-checkpoint autosave; the headless CLI server has no periodic scheduler.

Media list is available against either a project file (`--project`) or attached descriptor (`--attach`); pagination defaults to the maximum 100-item page. Media add takes a source path. Headless add/remove/relink open `ProjectFileSession`, execute the actual command, and safely save. Relink re-probes the replacement source while preserving the media ID, library position, and timeline references; incompatible or duplicate sources fail without mutation. Attached add and relink prepare/probe sources locally, then send normal typed commands with the captured revision; a concurrent edit returns `REVISION_CONFLICT` without retry. Attached remove uses the same semantic command path. IPC does not expose arbitrary file reads; the typed export request writes only its validated destination. `or media probe` remains read-only.

JSON success responses use the core result or descriptor structures; JSON errors use stable categories and codes. Exit codes are 0 for success, 2 for usage, 3 for application operation errors, 4 for project storage/recovery/session errors, and 5 for IPC errors. Filesystem paths remain OS paths; project names must be UTF-8. No command accepts shell instructions; IPC does not expose arbitrary file reads or writes, and CLI file operations are limited to the documented project and recovery commands.

Dry-run, long-running job progress, and agent EditPlans remain future work. Do not implement editing by synthesizing mouse clicks, keystrokes, or screen coordinates.

## 8. Local IPC

Phase 4F implements `or_ipc` protocol v1 for application/control requests. Frames contain a four-byte big-endian length and strict JSON body, bounded to 1 MiB. Each request uses a UUIDv4 ID echoed by the response; one request is processed per connection. Supported requests are `Describe`, shared `ApplicationRequest` (including the Phase 8F export job request/status/cancel contract), `Save`, and guarded `Shutdown`. The descriptor is strict and versioned and contains the endpoint, project/runtime IDs, and a random per-server token. Authentication and protocol checks happen before project disclosure or dispatch.

macOS/Linux use Unix-domain sockets in a server-created private runtime directory, with 0700 directory and 0600 socket/descriptor permissions. Sandboxed macOS builds place this directory in the app-provided temporary directory and include the local server entitlement; the app code creates no TCP listener. Windows uses a named pipe configured to reject remote clients; its runtime directory, descriptor file, and pipe have protected owner-only DACLs. There is no TCP, HTTP, WebSocket, LAN listener, or fallback. The token is not printed or logged; descriptor access is the client credential. This is a same-user local automation boundary, not isolation from malicious processes running as the same OS user. `LiveProjectHost` owns one `ProjectFileSession`, serializes direct bridge and IPC requests through the same control-plane state, and does not expose arbitrary file reads or shell execution. Android constructs this host without a server worker and continues to use the same direct bridge command/state path; the IPC protocol and its supported transports are unchanged. Export validates an absolute caller-supplied `.mkv` destination and does not read arbitrary files or execute shell commands. The Flutter shell schedules sidecar autosave; the headless `or session serve` command has no periodic scheduler.

Timeout/cancellation, subscriptions, multiple simultaneous sessions, and remote clients are not part of protocol v1.

## 9. Flutter and Rust bridge

Flutter is a thin UI over the application API. Rust remains the only canonical project/timeline state. Flutter may own presentation, navigation, panel, selected-tool, temporary text/input state, and scoped cached read models/view models, but not a second authoritative editable project model.

After a command is validated and applied, Rust emits a domain change or state-invalidation event; Flutter refreshes affected scoped queries/read models and rebuilds the relevant surface. Conceptual event categories include `ProjectChanged`, `TimelineChanged`, `SelectionChanged`, `MediaChanged`, `JobChanged`, and `CapabilitiesChanged`; exact names and schema are not frozen. Events or query results carry enough project revision/order information for Flutter to ignore or requery stale state when a newer revision is known.

Hot UI paths should use scoped queries such as timeline viewport, track list, selection inspector, media bin, and job list. Do not serialize and copy the whole project into Dart or rebuild every surface for each timeline interaction. Start with simple scoped queries and invalidation; do not introduce a reactive state framework before it is needed.

The Flutter bridge uses `flutter_rust_bridge` 2.13.0 with generated typed bindings in `packages/or_app_bridge` and a thin `crates/or_app_bridge` adapter. Its opaque `ProjectHostHandle` owns a `LiveProjectHost`; it does not expose mutable `ProjectDocument` state to Dart. Typed operations cover create/open, summary, rename, undo/redo, save, close, recovery inspection/actions, bounded media-page query, import/relink/remove, media-preview request/read, explicit sequence-rate query/command, preview-state query, exact seek, play/pause/tick/frame-step, a separate ordered artifact-event stream, desktop IPC descriptor access, and project invalidation events. Android invokes the same typed host operations in-process; its project read model has no IPC descriptor. Flutter keeps an immutable active-project read model and refreshes it from Rust after project events; mutations carry the read model's expected revision and do not auto-retry conflicts. The desktop `file_selector` plugin chooses project/media paths only; Rust owns canonical file validation, preparation, commands, and persistence. The Media panel shows filename, format, duration, video/audio summary, generated video-thumbnail or audio-only waveform PNG, and source/MediaId on demand, with 50-item pages over the core's 100-item page limit. Desktop import and disposable thumbnail, waveform, and proxy generation resolve FFmpeg helpers from the application directory first, with developer overrides available for explicit development runs; viewer decode and export use the linked FFmpeg runtime. Packaged runtime resolution does not require system `ffmpeg`/`ffprobe`, developer `PATH` entries, or loader overrides. Preview frame pixels stay in Rust/native memory, and Flutter receives only exact timing, status, dimensions, and an opaque texture ID. If the packaged probe helper cannot start, Flutter shows:

```text
The packaged media inspector could not start. Check the app installation and try again.
```

Android project New/Open use SAF document URIs only at the Flutter/native storage boundary. The native adapter reads at most 64 MiB into an app-private managed working copy; Rust then validates and opens it through the normal `ProjectFileSession`. Save and recovery update that local copy first; explicit provider sync compares the remote bytes with a private baseline and verifies readback where supported. A provider failure leaves the local project intact, and no atomic-replace guarantee is made for `DocumentProvider` writes. Android media import retains its SAF URI identity and preview opens bounded seekable provider descriptors. Export uses the shared evaluated project snapshot and bounded Rust export job with the linked LGPL FFV1 + PCM S16LE software encoder, writes to app-private staging, then publishes the completed Matroska file through the selected SAF document; it does not expose provider paths to project state. Android playback uses CPAL's AAudio backend, which requires Android 8.0/API 26; the app and all Rust target linkers use API 26. Hosted Android CI verifies the user journey and resource bounds on API 36 x86_64 SwiftShader. CI builds each target and runs Flutter widget tests. Hosted macOS, Linux, and Windows jobs run the Phase 7 runtime, software decode, recovery, and stale-output conformance suites; native Flutter lifecycle, offline-media, timeline-edit, and preview transport integration runs on macOS.

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
The required pixel-buffer fallback uses BGRA8888 with premultiplied alpha unless
specific platform API evidence requires RGBA8888 for that adapter.
It must never send per-frame pixels, full-rate frame bytes, or raw OS/GPU handles
through Dart, IPC, project state, or cache identity. Retain each `FrameLease`
until the native release callback or completion fence signals. The platform
adapter uses a bounded latest-frame mailbox/in-flight set and rejects frames
from stale generations or project revisions. Pixel-buffer fallback may require
a native copy/readback; frame data never takes a copied-Dart route.

Timeline/render evaluation should publish a stable read view such as `RenderSnapshot N` for canonical `ProjectRevision N`. A project mutation produces a view associated with the next revision, which workers can adopt safely. Render workers never mutate Project, and the architecture must not require them to lock mutable Project state continuously. The snapshot may be a full immutable evaluated structure, incremental graph, structurally shared data, versioned read model, or another measured strategy; snapshot granularity must be benchmarked rather than assumed to mean cloning the whole project for every edit.

Per-frame playback/render work is runtime execution over committed state. It must not dispatch project-edit commands, open Project transactions, or increment `ProjectRevision`. The native adapter selects a platform-appropriate synchronization primitive and a small bounded in-flight depth; the latest-frame mailbox replaces obsolete preview work. 7F1 proves platform adapter builds and 7G owns bounded queue tuning against repeatable target-hardware evidence.

A frame carries explicit dimensions, pixel or texture format, color information, and timing metadata. The desktop preview path uses software-decoded RGBA layers lowered to wgpu output and the approved native pixel-buffer adapter; platform-specific interop remains optional.

Phase 7 native capability matrix:

| Platform | Decode and runtime package | Viewer presentation | Hardware path |
| --- | --- | --- | --- |
| macOS | FFmpeg 8.1.3 software decode; app-local dynamic `.dylib` libraries | Bounded BGRA8888 premultiplied Flutter pixel-buffer texture | No hardware decode or shared GPU surface approved |
| Windows | FFmpeg 8.1.3 software decode; app-local `.dll` libraries | Bounded BGRA8888 premultiplied Flutter pixel-buffer texture | No hardware decode or shared GPU surface approved |
| Linux | FFmpeg 8.1.3 software decode; app-local `.so` libraries | Bounded BGRA8888 premultiplied Flutter pixel-buffer texture | No hardware decode or shared GPU surface approved |
| Android | No linked FFmpeg or Phase 7 viewer decode | Viewer playback unavailable | Separate Phase 9 package and device gates |

The desktop preview uses the same software fallback on all three supported desktop systems. `PreviewRuntime` reuses at most four source decoder sessions per project revision and runtime generation; forward playback requests decode from the current demux position even when presentation requests were dropped, while a backward request seeks and an explicit transport seek starts a fresh generation. Each session retains at most a preceding frame and one lookahead frame, both charged to the shared decode budget. FFmpeg `EAGAIN` from `avcodec_receive_frame` marks a drained decoder on every platform; on Windows, `ffmpeg-the-third` exposes its POSIX errno and the runtime compares it as such instead of interpreting it as a Win32 error code. CI checks the runtime, software decode, cancellation, lease lifetime, stale-snapshot, fallback, and recovery contracts on those hosts. The software decode suite asserts a pre-cancelled exact seek returns `DecodeError::Cancelled` before opening media. Native Flutter integration currently exercises lifecycle, offline media, and exact preview transport on macOS.

### Phase 7F0 playback timing contract

The canonical `ProjectTimeline.sequence_frame_rate` is optional and explicit:
one exact `RationalRate` in frames per second. No project or active source
supplies an implicit default.
The schema-v6 `timeline.sequence.set_frame_rate` command accepts an exact rate
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

The approved baseline is FFmpeg 8.1.3, using matching headers and library ABI majors at build and runtime. Any update requires a new compatibility, API, MSRV, license, and hosted-link review; the later gates use 8.1.3. Checkpoint 7C0 provisions a separate Linux CI prefix built from the official 8.1.3 source with shared libraries, `--disable-autodetect`, `--disable-everything`, and no `--enable-gpl`, `--enable-nonfree`, or `--enable-version3` options, then compiles, links, loads, and checks the native library ABI/license strings through an out-of-workspace probe. The CI-only build also uses `--disable-asm` to reduce build requirements. The 7C0 probe was tooling only. The production workspace now pins `ffmpeg-the-third` 6.0.0 with only codec, format, software-resampling, and software-scaling features. Its hosted prefix keeps the LGPL-only dynamic configuration and enables the `file` protocol, Matroska, MOV, and WAV demuxers; FFV1/PCM S16LE, H.264, and AAC decoders; and H.264/AAC parsers. The additional MP4-family profile is a candidate only until packaged desktop and Android journeys pass; its library license configuration does not settle codec patent obligations. WAV remains limited to audio-only PCM S16LE. These options add no external codec library, GPL module, nonfree module, or encoder.

Dynamic linking is the selected packaging strategy. Build from the pinned FFmpeg source/configuration for each target and bundle the matching `libavcodec`, `libavformat`, `libavutil`, `libswresample`, and `libswscale` shared libraries plus their runtime dependencies. Include the corresponding headers and import metadata only in development/build environments. Linux packages ship the `.so` libraries with app-relative runtime lookup; macOS packages ship the `.dylib` libraries with app-relative loader paths and sign/notarize the complete bundle; Windows packages ship matching MSVC `.dll` files beside the app and use their `.lib` import libraries at build time. Release packages must include the exact corresponding FFmpeg source, build configuration, changes, and LGPL notices/source location; keep the FFmpeg shared libraries replaceable and preserve their library names. Android is a separate Phase 9 decision requiring NDK builds for supported ABIs and per-ABI shared-library packaging evidence.

Phase 5A uses an external `ffprobe` executable for metadata inspection. `probe_media_file(&Path)` accepts local regular files, invokes the executable through `std::process::Command` without a shell, and passes the canonical file path as one `-i` argument. It requests `-of json` and a narrow `-show_entries` set of format and stream fields, and CI records the tool with `ffprobe -version`. Output is limited to 1 MiB stdout and 64 KiB stderr, with a 15-second timeout and child cleanup. The parser ignores unknown external JSON fields, then validates the OR metadata representation. Missing, malformed, zero, or out-of-range required video dimensions and malformed or negative durations fail with a structured metadata error. Malformed optional frame rates and missing, malformed, zero, or out-of-range optional audio sample rates and channel counts become unavailable rather than invalid domain values; wrong-typed descriptive metadata such as channel layout fails validation. `RationalTime` parses decimal durations exactly with at most nine fractional digits, matching its `u32` denominator bound; valid frame rates use `RationalRate`. `MediaMetadata` is control-plane metadata and contains no path or arbitrary tags. Phase 5B's `prepare_media_import` probes and validates one selected local file, creates a canonical local `file:` URI and fresh `MediaId`, and returns a `MediaItem`; it does not mutate a project. A caller must submit it through `media.add`. Project load and `media.list` validate/display stored metadata without opening or probing sources. Neither phase links or bundles FFmpeg. Invocation syntax and fields follow the [official ffprobe documentation](https://ffmpeg.org/ffprobe.html).

Phase 5D adds library-preview generation through an external system `ffmpeg` executable. The default command is `ffmpeg`; `OR_FFMPEG_PATH` is a developer/test override and is never persisted or included in cache keys. The bridge converts the validated local `file:` URI to a native path and passes it as one direct process argument, without a shell. Thumbnail profile `Library Thumbnail V1` selects the first video stream and first decodable frame, scales within a 320-pixel maximum edge without upscaling while preserving aspect ratio, and emits a validated PNG; it does not search for a representative scene. Waveform profile `Library Waveform V1` selects the first audio stream, combines channels, and emits a white 512×96 PNG. The profile descriptors participate in parameter fingerprints. Thumbnail generation has a 20-second timeout; waveform generation has a 30-second timeout; stdout is limited to 8 MiB, stderr to 64 KiB, and the process is polled every 10 ms. Timeout, cancellation, overflow, and read failure kill and reap the child. These are disposable Media-panel images, not timeline waveform data, decode, playback, or render output. They remain on the external-executable path and do not use the linked `or_media` runtime.

The media boundary must support both software decode and hardware-surface decode. A centralized capability/provider layer should report usable paths for automatic selection and safe fallback; generic timeline/domain code must not accumulate platform-specific conditionals. Possible platform directions are examples only, not selections: VideoToolbox/platform video surfaces on macOS; platform hardware decode and D3D-compatible surfaces on Windows; VAAPI, Vulkan, or DMABUF-style interop on Linux; and MediaCodec with hardware-buffer or native-surface paths on Android. Support varies by codec, device, pixel format, driver, and backend.

Pipeline selection chooses only a supported, stable path for the actual codec, pixel format, resolution, backend, device, driver, platform, and operation. A hardware path is not presumed faster. Software/CPU paths remain correctness fallbacks; no hardware decode path is approved by the 7D evaluation. Any later hardware selection belongs to a measured hardware-capability checkpoint with build, license, interop, fallback, and target-device evidence.

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

GPU candidates include scaling, rotation, crop, appropriate color-space conversion, blending, masking, compositing, color operations, and suitable effects. Project state, command validation, serialization, metadata, scheduling/orchestration, and work unsuited to a GPU remain CPU/domain responsibilities. Profile by operation; there is no requirement that every operation run on the GPU. The mandatory export uses the software FFV1/PCM profile. Hardware encoder APIs and FFmpeg hardware-frame integration belong to a future optional delivery/hardware checkpoint and are not correctness prerequisites.

Use optimized upstream implementations and compiler auto-vectorization before considering architecture-specific intrinsics or custom SIMD. ARM NEON or x86 SIMD paths may be evaluated behind tested abstractions only after profiling identifies a meaningful bottleneck.

## 12. Audio and text

Decode audio through `or_media`; `or_audio` defines the device-neutral output boundary. Its preallocated single-producer/single-consumer interleaved buffer is bounded and non-blocking: producers receive backpressure when full, while the callback fills missing frames with silence and reports the underrun. The callback advances the audio master clock by device frames, including silence, and returns an exact clock message bound to its `RenderSnapshot` for video pacing. Timeline-to-device conversion subtracts the snapshot seek origin, multiplies exact rational seconds by the integer device sample rate, then floors to the frame at or before that time. Video drops frames more than the configured drift tolerance behind audio and waits when they are ahead; frames within the tolerance may present. Synchronization rejects clock messages from another project or revision while allowing each worker to request its own exact time range.

The callback does not allocate, lock, call Flutter or providers, or access canonical project state. Cancellation silences and drains queued data; a seek starts a fresh clock message at its exact timeline origin. `cpal` 0.18.1 stays in `or_audio`; it must not become a direct `or_core` dependency. Desktop and Android output currently request exactly 48 kHz, stereo, f32; unsupported device formats return a structured unavailable error and video playback can continue on the monotonic clock. Flutter's Android host installs a process-lifetime `ndk_context` JavaVM/application-context pair before Dart initializes the Rust bridge, as CPAL's AAudio backend requires when embedded through FFI. A bounded worker decodes 250 ms windows through `or_media`, applies each clip's gain (millidecibels converted to linear amplitude), linear stereo balance pan (basis points), and multiplicative linear fades at exact timeline sample times, then mixes and clamps the prepared interleaved blocks. The 12,000-frame SPSC ring and 256-entry/32 MiB audio decode budget bound producer backpressure. Preview uses the device clock as master and gates video against a one-frame drift tolerance. Hosted Android acceptance now exercises imported WAV on an audio track and requires the device-master clock to advance; it does not establish acoustic output or representative-device performance.

Typed preview effects are applied to bounded RGBA layers before the existing wgpu compositor. Brightness adds `amount_milli / 1000 × 255` to each byte-domain RGB channel; contrast scales around byte value 128 with 1000 milli as identity; saturation scales channel distance from Rec.709 luma with 1000 milli as identity. Gaussian Blur rounds its milli-pixel radius up to a whole pixel and uses a separable box-blur approximation. Cross Dissolve scales layer alpha, Fade Through Black scales RGB while keeping alpha, and Wipe reveals left-to-right on entry and conceals left-to-right on exit. Transition progress derives from exact rational clip time and duration. Text and media layers use the same processing helper. Phase 8F must use these typed semantics in the exporter; this checkpoint adds no arbitrary effect language or plugin boundary.

Final video text is rendered by `or_render::TextRasterizer`, which is used by preview composition after the active typed text/caption clips are evaluated on exact half-open timeline intervals. It rasterizes transparent RGBA layers with bounded dimensions and a 64 MiB aggregate active-overlay limit before the existing wgpu compositor. Exported text must not depend on Flutter widget rendering or host-installed fonts; the software exporter in 8F can reuse this rasterizer and the same typed edit semantics. `cosmic-text` 0.19.0 belongs to `or_render`, not `or_core`, and the workspace MSRV is Rust 1.89. The renderer loads only the bundled Inter 4.1 Regular, Medium, SemiBold, and Bold faces, so rasterization does not scan or depend on host fonts. The editor creates and updates text/title and manual-caption clips through the existing canonical application command path; updates preserve clip ID and start while accepting exact duration and formatting. Phase 11 reuses this shaping, font-identity, and render path.

The font source is the official [Inter v4.1 release](https://github.com/rsms/inter/releases/tag/v4.1), downloaded as [`Inter-4.1.zip`](https://github.com/rsms/inter/releases/download/v4.1/Inter-4.1.zip) (`9883fdd4a49d4fb66bd8177ba6625ef9a64aa45899767dde3d36aa425756b11e` SHA-256). The four distributed TTFs come from `extras/ttf/` in that archive; the distributed `LICENSE.txt` is the archive-root copy. Attribution is “Copyright (c) 2016 The Inter Project Authors.” The bundled license is SIL Open Font License 1.1. SHA-256 values:

| Bundled asset | SHA-256 |
| --- | --- |
| `Inter-Regular.ttf` | `40d692fce188e4471e2b3cba937be967878f631ad3ebbbdcd587687c7ebe0c82` |
| `Inter-Medium.ttf` | `97ad806f526e41546d46365bb3a393145f75b7b1568913db74549ad8b8dba872` |
| `Inter-SemiBold.ttf` | `78a843fade9d4612a5567302fb595b56976eb5fcebf4fea5a5912d638bafcde3` |
| `Inter-Bold.ttf` | `288316099b1e0a47a4716d159098005eef7c0066921f34e3200393dbdb01947f` |
| `LICENSE.txt` | `262481e844521b326f5ecd053e59b98c8b2da78c8ee1bdbb6e8174305e54935a` |

All five files are under `crates/or_render/assets/fonts/inter/`. The direct `cosmic-text` dependency is pinned to 0.19.0 with default features disabled and only `std` and `swash` enabled; Cargo reports `MIT OR Apache-2.0` and Rust 1.89 compatibility.

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

Hot media paths should avoid repeated large allocations where practical. Frame, texture, audio-buffer, decode-surface, and render-target reuse must remain bounded and must not become an unbounded cache. 7G owns queue and pool tuning, memory budgets, and performance targets using repeatable target-hardware evidence; values may vary by device class, platform, and workload, with more conservative Android policy.

Cache keys are deterministic over source fingerprint, operation, parameters, and cache schema version. Phase 5E selects `rusqlite` 0.40.1 with bundled SQLite for the disposable cache index only; this does not change `.orproj` project storage. Cache contents are disposable and never authoritative project state. Provide bounded storage, eviction and regeneration paths, and a clear-cache operation; cache presence must never be required for project correctness.

Workers may produce structured `JobResult`, `GeneratedAsset`, `AnalysisResult`, `CaptionProposal`, or `EditProposal` outputs. They may update disposable cache/job state but must not directly mutate the canonical ProjectDocument. If an output should change a project, the application layer checks its permissions and expected revision/preconditions, then applies it through a validated command/transaction. This preserves undo/redo and rejects stale analysis instead of applying it silently.

## 14. AI providers and local inference

Checkpoint 10A owns the provider boundary for later AI features. Typed tasks go
through the `or_ai` Provider Manager to managed local sidecars, optional
permissioned cloud adapters, deterministic test providers, or a typed
`Unavailable` capability. Results are reviewable proposals, analyses, or
assets and reach project state only through the normal validated command path.
No provider or inference runtime is a direct `or_core` dependency. No cloud
credential or model is required to complete the roadmap.

The V1 local sidecar uses child-process stdin/stdout with one bounded UTF-8 JSON
object per line. Stdout carries protocol only; bounded diagnostics use stderr.
Messages carry a protocol version and stable request/job ID and support
readiness/capabilities, task start, progress, result, typed error, cancel,
cancel acknowledgement, and shutdown. Control messages are at most 1 MiB
unless an existing repository bound is smaller. Large media and model artifacts
use bounded managed-file references, not base64 payloads. Launch uses explicit
argv without a shell, sanitized inherited environment, and managed per-job
scratch. A local sidecar has no implied network authority; cloud access requires
explicit application permission.

The model artifact contract is owned by 10A: a versioned manifest records model
ID/version, task capabilities, runtime/provider requirement, artifact source,
expected SHA-256, size bound, license/provenance, hardware needs, and
language/capability metadata. Managed files are checksum-verified outside
`ProjectDocument`; weights are not bundled by default. A missing provider or
model returns typed `Unavailable`. 10H owns the user-facing download,
pause/resume, verify, update, remove, storage, license, and permission flows
over that contract. The reference local ASR sidecar is whisper.cpp v1.9.4; its
runtime and all model artifacts keep their separate dependency and licensing
reviews. CI uses deterministic provider fakes and protocol fixtures.

Runtime code licenses do not establish the license or redistribution rights for
model weights, tokenizers, or associated assets. Verify each exact artifact.
Network-capable providers declare their access needs and pass through the
application permission boundary. Offline Mode applies to GUI, CLI, and agent
requests; cloud tasks receive only the minimum context they need.

## 15. Model management and secrets

Checkpoint 10A owns the versioned model manifest and bounded artifact contract:
ID/version, task capabilities, runtime/provider requirement, source, expected
SHA-256, size bound, license/provenance, hardware requirements, and
language/capability metadata. Verify managed model files before activation and
keep them outside the repository and `ProjectDocument`; weights are not
bundled by default. Missing models/providers report typed `Unavailable`.
Checkpoint 10H owns user-facing download, pause/resume, verify, update, remove,
storage, license, and permission flows over the 10A contract.

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
references, stable identities, checksums/provenance, and the Phase 8 Inter 4.1
font identity through the shared `cosmic-text` 0.19.0 path. It has no HTTP, environment,
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
runtimes or dependencies. Lottie JSON 1.0 and dotLottie 2.0 are future bounded
interchange, not the canonical format. Chromium/headless browser use is reserved for the
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

Phase 16B uses `wasmi` 1.1.0 inside a dedicated extension runtime, never as an
`or_core` dependency. General WASI is not enabled by default. Plugins receive
only explicit OR host capabilities; default authority excludes filesystems,
network, environment enumeration, credentials, process launch, and arbitrary
project mutation. Enable fuel metering and explicit `StoreLimits`, with outer
host bounds for module bytes, memory, instances/tables, I/O, host allocations,
time, and cancellation. All changes become ordinary validated OR commands.
Phase 16D owns the stable extension ABI; 16B must not create a native-plugin
ABI.

Native and OpenFX compatibility is later and has a higher trust cost. Do not treat installed plugins as unrestricted trusted code by default.

## 19. Export and interchange

Desktop export captures one project revision through the shared application
request and queues bounded background work with monotonic frame progress and
cancellation. It evaluates exact sequence frame times through the same timeline
loader, renderer, text rasterizer, transform/crop/opacity, and visual effect and
transition path as preview; audio reuses the bounded timeline mixer and the
typed gain, pan, and fade processor. Video uses Matroska + FFV1; audio is stereo
48 kHz PCM S16LE. The selected destination must be an absolute `.mkv` path.
Output is encoded inside a unique sibling staging directory (owner-only on
Unix) and published only after both streams and the trailer finish. Cancellation,
offline media, and encode failures clean staging output, preserve an existing
destination, and leave a new destination absent.
Export does not mutate canonical project state or increment its revision.
This checkpoint enables only the muxer and encoders required by this profile.
H.264, H.265/HEVC, AV1, VP9, NVENC, VideoToolbox, MediaCodec, and other delivery
or hardware paths are optional and do not gate Desktop MVP.

The Flutter shell submits a recovery-checkpoint autosave every 30 seconds while
the project is dirty. The Rust file session validates the exact saved base and
updates only the bounded recovery sidecar. Explicit Save remains the canonical
project-file action, and autosave never silently overwrites that file. Recovery
inspection, apply, and discard remain explicit user actions.

The native OR format is not OpenTimelineIO. Phase 16E uses a bounded OTIO JSON
adapter implemented directly in Rust/serde against a documented subset; it
does not add OTIO's C++ or Python runtime. Unsupported content receives
explicit diagnostics, and EDL/XML remain bounded explicit adapters. Lottie
JSON 1.0 and dotLottie 2.0 are bounded interchange baselines, not canonical
MotionScene formats. Imports do not execute expressions or fetch remote assets;
dotLottie input is treated as untrusted ZIP data with entry, expansion, path,
collision, and parsing limits.

## 20. UI feature registration and mobile

Production Flutter may use static feature descriptors containing ID, label, icon, group, availability, command IDs, panel, inspector sections, and shortcut metadata. Keep registration lightweight; do not build a speculative plugin framework around it.

Stable shell slots include App Bar, Editor Tool Rail, Left Tool Panel, Viewer, Inspector, Timeline Toolbar, Timeline, Task or Status Area, Dialog or Mobile Sheet, and Command Palette. Integrate new work in an existing slot unless a permanent new region is justified and reviewed.

Simple and Advanced modes are visibility settings over one state and command model.

Android uses the same Rust core and project model with touch-native Flutter
presentation and mobile-appropriate resource policies. Checkpoint 9A0 proves
software FFmpeg 8.1.3 shared-library build, cross-link, APK packaging, runtime
loading, Flutter/Rust bridge loading, and hosted x86_64 emulator coverage for
`arm64-v8a`, `armeabi-v7a`, and `x86_64`; hardware media is not required.

Checkpoint 9A owns typed `MediaSourceRef` values for existing `FileUri` and
`AndroidSafDocumentUri` sources. A SAF URI is never converted to a fake path;
the OS permission grant and live descriptors/handles are runtime metadata, not
project data. On non-Android platforms the typed SAF reference remains valid
but resolves offline until relinked. Android opens an external project through
a bounded SAF read into an app-private canonical working copy and the normal
Rust `ProjectFileSession`. Save/recovery is atomic to that copy first, then an
explicit SAF synchronization/export with readback where supported. Provider
write failure cannot invalidate the working copy. Media access uses a transient
opaque seekable capability, such as a duplicated descriptor or bounded custom
FFmpeg I/O; copy only to a bounded, cancellable, disposable materialization
when a provider is non-seekable or a component requires a local file. 9D uses
the same storage boundary and the mandatory software FFV1/PCM S16LE profile.
Android media import probes the granted descriptor with the packaged FFmpeg
library and sends bounded probe output through the same core metadata and
codec-matrix validator as desktop. It never passes a SAF URI through
filesystem canonicalization or path-based ffprobe.

The 9B Android preview adapter prepares an exact shared render request before
binding its evaluated active SAF video sources. The request carries cancellation
and generation identity; completion checks project identity, instance and
revision before decoding and again before publication. Timer ticks apply
bounded backpressure while source registration or completion is pending, while
explicit seeks cancel older work and serialize completion. The native adapter
checks every FD registration, rolls back partial sets, revalidates grants on
cached sets, and releases duplicated capabilities on project close/switch.
If platform source binding fails after a prepared frame, abort drains the
bounded decoder-session cache so a cached decoder cannot keep a revoked SAF
descriptor alive. Preview shutdown drains that cache after taking the render
lock, releasing duplicated SAF descriptors when the project closes.
Software decode uses the packaged FFmpeg profile and shared render path; the
decoder includes the source PTS, origin PTS, and time-base rate when an FFmpeg
timestamp cannot be represented by the project's exact rational clock; audio
range/cursor overflows identify the operation and exact operands. The
bounded BGRA lease is copied to one reusable bitmap and presented through
Flutter's SurfaceProducer on the main thread. Surface and preview epochs cover
both copy and drawing, including cleanup/recreation/release. Android output now
uses the shared `or_audio` CPAL path. Exact-SHA hosted run
[38026118020](https://github.com/huou07/Opencut-Reinforced/actions/runs/38026118020)
on `0df7ea4c9e03d8a9034766ab8e4f19e747ee44e2` passed the packaged Android
SAF WAV-track playback journey: AAudio started, the device clock advanced with
no playback error, and the package recovered and exported the project. A
prepared Play request includes active visual SAF
sources and audio SAF sources whose timeline clips extend beyond the playhead.
The bridge registers that bounded source set before starting CPAL, so the audio
worker cannot race ahead of permission-checked seekable descriptor registration.
Unchanged source sets reuse their native capabilities. Android copies the
published premultiplied BGRA frame directly into its reusable bitmap, then
uses one reusable color-matrix paint on the hardware canvas to restore the
bitmap's red/blue channel order without a per-pixel CPU loop. Android hardware
acceleration and physical-device acoustic output remain unavailable/unverified;
emulator frame skips mean Android performance remains unaccepted.
Android SAF journey diagnostics separately report native latest-frame
acquisition, CPU bitmap byte copy, bitmap allocation count and maximum
allocation time. Main-thread timing splits `SurfaceProducer` resize, canvas
lock, bitmap draw (including the red/blue channel filter), and
`unlockCanvasAndPost` within its total draw
duration. The presenter redraws every pixel on every frame, which satisfies
Android's hardware-canvas full-surface contract. It therefore uses
`Surface.lockHardwareCanvas()` for the bitmap presentation path. These bounded
counters identify the expensive stage; they are diagnostic measurements, not
performance acceptance thresholds. Hardware-canvas acquisition still requires
exact packaged measurements and representative-device profiling before any
performance claim. The Android SAF test records a resource snapshot immediately
after surface resume and preserves structured results for each imported item,
so a later journey failure does not discard the earlier render measurements or
the import error that caused it.

Exact-SHA platform run
[38036564801](https://github.com/huou07/Opencut-Reinforced/actions/runs/38036564801)
on `7d25d17cc9c4996c556a86b0d349202fedbce371` passed every hosted job,
including the complete Android SAF journey. On the API 36 x86_64 SwiftShader
emulator, the journey report measured a maximum canvas-lock time of 7,897 µs,
canvas draw of 495 µs, `unlockCanvasAndPost` of 8,290 µs, and total main draw
of 13,254 µs; all three selected SAF media sources imported successfully. The
prior software-canvas run measured a 145,009 µs maximum canvas lock on the same
hosted emulator configuration. This isolates canvas acquisition as a major
emulator bottleneck and supports retaining the hardware-canvas implementation.
The stress phase remained bounded (peak eight pending presentations, excess
requests rejected, zero in-flight frame leases at completion). These debug
emulator measurements do not establish release performance or physical-device
GPU behavior; those remain unverified.

The direct-copy/color-filter candidate was verified at exact product SHA
`02ebdddfe413099aed97aef40f2a6fed531fb76a` in hosted run
[38039913360](https://github.com/huou07/Opencut-Reinforced/actions/runs/38039913360).
All platform jobs passed after a same-SHA macOS-only rerun: the first macOS
attempt timed out while its lifecycle test emitted no output after app launch,
and the rerun passed the native lifecycle and packaged product journeys. The
Android report passed all 25 SAF checks and all three pixel samples remained
`[252, 0, 0, 255]`, including after resume and recovery. Maximum main draw was
5,039 µs and maximum canvas draw was 76 µs. CPU bitmap-copy maximum was still
21,278 µs for a 921,600-byte frame (22,964 µs total), so this run does not
establish a lower peak copy latency than the prior implementation. The
per-pixel CPU swizzle and scratch buffer are removed; the direct copy and
hardware canvas filter preserve the measured image. Treat this as a simpler,
correct presentation path, not an accepted Android performance improvement.
The emulator measurements still do not establish release or physical-device
performance.

See TOOLING.md and TESTING.md for hosted real DocumentsUI/editor journeys and
measured diagnostic limits.

Visible Flutter strings and accessibility labels use a localization-capable resource boundary when production UI work begins. Do not scatter user-facing English strings through domain/business logic. Human-readable errors may be localized at the presentation boundary; command IDs, JSON field names, and machine-readable error codes remain stable technical identifiers. Do not select a localization package or generate localization files in this planning phase.

## 21. Security

Treat project files, media, subtitles, templates, themes, downloaded assets, models, plugin output, and agent or AI output as untrusted. Validate input at every serialization, IPC, plugin, model, and community boundary. Enforce limits for file sizes, dimensions, durations, archive expansion, and job resources before implementation exposes those inputs.

Keep secrets out of logs, project files, CLI output, and agent context. Require explicit capabilities for plugins and community actions. Security and licensing constraints are part of feature design, not follow-up cleanup.

Phase 4F/4UI-2 bounds `.orproj` input to 64 MiB and recovery sidecars to 136 MiB, validates strict UTF-8/serde envelopes, and preserves the exact disk-base and recovery checks before file-session saves. IPC accepts only the versioned, bounded semantic request types; the server cannot open caller-selected paths or execute shell commands. Unix runtime directory, socket, and descriptor permissions are restricted to the current user. The Windows runtime directory, descriptor file, and named pipe use protected owner-only DACLs, and the pipe rejects remote clients. A random per-server token is stored only in the descriptor and is not logged, returned by `Describe`, or displayed in Flutter. Anyone who can read that descriptor as the same OS user can authenticate, so it is not a multi-user security boundary. No API credentials belong in the descriptor, IPC payloads, project files, or CLI output. The implementation creates no TCP listener.

Provider network capability and permission are enforced centrally by the application, including when a request originates from CLI or an agent. Offline Mode denies OR-originated optional network calls regardless of UI path; it does not claim to firewall the operating system. Send the minimum required data to each cloud task, without unrelated project context or secret values.

## 22. Implementation structure

Start with the smallest useful Rust workspace and Flutter shell when Phase 3 is explicitly started. Planned domains are conceptual boundaries, not a mandate to create one crate per domain. Split a module into a crate only when a concrete build, reuse, ownership, or dependency boundary justifies it. Never create empty future crates or modules.

## 23. Frozen decisions and owned open details

The following architecture decisions are frozen for the remaining roadmap and
are implemented only by their owning checkpoint:

- 7F0/7F1 lock the desktop viewer timing and texture contracts; 7F connects
  product transport to the Rust runtime. A bounded native pixel-buffer
  external texture remains the correctness fallback; BGRA8888 with
  premultiplied alpha is the common format. Shared GPU surfaces remain optional.
- FFmpeg 8.1.3 with `ffmpeg-the-third` 6.0.0 uses reproducible dynamic/shared
  linking and an LGPL-only configuration. 7F1 proves desktop link/load/package
  provenance; 9A0 proves Android package/load support.
- 8A owns the single v5-to-v6 project-model transition and typed media/text/
  caption timeline contract. Phase 6 owns persistent markers and snapping.
- 8D uses `cosmic-text` 0.19.0 and bundled Inter 4.1 (SIL OFL 1.1), and
  deliberately raises workspace MSRV to Rust 1.89. Phase 11 reuses that path.
- 8E uses `cpal` 0.18.1 inside `or_audio`, outside `or_core`.
- 8F requires software Matroska + FFV1 + PCM S16LE export. Autosave writes the
  recovery checkpoint; explicit Save writes the canonical project.
- 9A0 owns Android native package evidence; 9A owns typed `FileUri` and
  `AndroidSafDocumentUri`, the app-private canonical working copy, and
  transient seekable media access.
- 10A owns the provider manager, bounded local sidecar protocol, model manifest,
  and typed `Unavailable` outcome. whisper.cpp v1.9.4 is the reference ASR
  sidecar; Phase 10 and later require no cloud key or model weights.
- 13C owns SDR Rec.709/sRGB delivery assumptions, linear-light working space,
  and premultiplied-alpha compositing. HDR correctness remains deferred.
- 16B uses `wasmi` 1.1.0 in an isolated extension runtime without default WASI;
  16D owns the stable extension ABI.
- 16E uses bounded direct Rust/serde OTIO JSON interchange, Lottie 1.0, and
  dotLottie 2.0. These remain interchange formats, not canonical project or
  MotionScene formats.

Remaining open implementation details have explicit owners:

- Flutter state management and presentation event wiring: 7F/8B, within the
  shared command/state model.
- Render snapshot representation, comparison tolerance, decode/render queue
  tuning, worker scheduling, resource pools, budgets, and performance targets:
  7G and repeatable target-hardware evidence.
- Hardware decode APIs and thresholds: a future measured hardware-capability
  checkpoint; no hardware path is valid.
- Hardware encode APIs and FFmpeg hardware-frame integration: an optional
  delivery/hardware checkpoint; neither is a correctness prerequisite.
- Shared GPU-surface fast paths and per-platform synchronization details: a
  measured platform optimization; the pixel-buffer fallback remains
  authoritative.
- Exact CPU/GPU operation partition, SIMD dispatch, and performance
  instrumentation: profiling in 7G or the owning render/effect checkpoint.
- Audio callback buffer/ring tuning: 8E, with bounded prepared buffers and
  device-neutral correctness tests.
- Translation, TTS, segmentation, and generation models: optional provider/model
  manifests under 10A/10H; missing models remain `Unavailable`.
- Cloud provider/vendor: an optional provider adapter; no vendor is mandatory.
- Stable installer formats and signing: a future stable-release gate, not a
  full-roadmap completion prerequisite.
- Localization package/resource generation and any future persistence crate:
  the feature checkpoint that first requires that dependency, after its own
  build, platform, and license review.

No marker architecture remains open, and no already-frozen dependency or
behavior is deferred for a later architecture choice. Optional capabilities
remain optional; this plan does not claim they are implemented.

## 24. Upstream references

These upstream references support frozen dependencies and current interface
claims. Re-check version-specific facts when an owning checkpoint implements
or updates the dependency.

- [Flutter supported platforms](https://docs.flutter.dev/reference/supported-platforms) documents Flutter's platform matrix; OR currently selects macOS, Windows, Linux, and Android from that broader support.
- [flutter_rust_bridge](https://github.com/fzyzcjy/flutter_rust_bridge) documents generated bindings, structured values, asynchronous functions, streams, errors, and platform support. Version 2.13.0 is used by the Phase 3 bootstrap bridge; this does not settle the future command, event, or media-transport architecture.
- [wgpu supported platforms](https://github.com/gfx-rs/wgpu#supported-platforms) lists OS and graphics-backend support, including first-class and best-effort distinctions.
- [FFmpeg license and legal considerations](https://ffmpeg.org/legal.html) describes LGPL defaults, optional GPL components, dynamic-linking guidance, and source redistribution obligations. Checkpoint 7C0 pins the probe to [ffmpeg-the-third 6.0.0](https://crates.io/crates/ffmpeg-the-third/6.0.0) and [FFmpeg 8.1.3 source](https://ffmpeg.org/releases/ffmpeg-8.1.3.tar.xz); the packaged configuration and codec set still require release-specific review.
- [ffprobe documentation](https://ffmpeg.org/ffprobe.html) defines its command-line options, JSON output, field selection, input argument, and version reporting.
- [OpenTimelineIO](https://github.com/academysoftwarefoundation/opentimelineio) describes an editorial interchange format and API.
- [whisper.cpp license](https://github.com/ggml-org/whisper.cpp/blob/master/LICENSE) covers the runtime source, not every model artifact.
- [ONNX Runtime license](https://github.com/microsoft/onnxruntime/blob/main/LICENSE) covers the runtime source, not model artifacts.
- [llama.cpp license](https://github.com/ggml-org/llama.cpp/blob/master/LICENSE) covers the runtime source, not model artifacts.
