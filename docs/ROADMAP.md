# Roadmap

## Status

No dates or delivery promises are implied. Phase 2 is complete as Architecture Blueprint V1, Phase 3's executable architecture skeleton is complete, and Phase 4 is complete as a project/application foundation, not a finished editor. Phase 5 is DONE / FOUNDATION COMPLETE: Phase 5A–5F are complete. Phase 6 — Timeline MVP is DONE: 6A, 6B, 6C, 6D, 6E1, 6E2A, and 6E2B are complete. Phase 7 is the next planned phase; later phases depend on implementation capacity, platform evidence, and licensing or security review.

## Phases

### Phase 0 — Repository and safety
**Status: DONE**

Public repository setup, license, contribution and security guidance, repository safeguards, and hygiene CI.

### Phase 1 — Product and interactive UX prototype
**Status: DONE / REFERENCE FROZEN**

Approved design language, interactive product reference, and UX acceptance invariants. The HTML prototype is a product and UX reference only.

### Phase 2 — Product and technical blueprint
**Status: DONE / ARCHITECTURE BLUEPRINT V1**

Complete product scope, architecture, implementation workflow, roadmap, testing, security and licensing, and release documentation before application implementation.

### Phase 3 — Executable architecture skeleton
**Status: DONE**

- minimal Rust workspace
- or_core
- or_cli
- Flutter shell
- typed bridge
- version, health, and capability commands
- cross-platform Rust and Flutter CI, including macOS runtime bridge verification

### Phase 4 — Project and command foundation
**Status: DONE / FOUNDATION COMPLETE**

Phase 4A — DONE:

- exact rational time, rate, and range primitives
- typed UUIDv4 persistent project identity
- ephemeral runtime project-instance identity
- checked persistent project revision

Phase 4B — DONE:

- minimal `ProjectDocument`
- `.orproj` UTF-8 JSON schema v1 and strict in-memory codec
- explicit domain/wire conversion and schema-version dispatch

Phase 4C — DONE:

- runtime `ProjectSession` and stale project/session/revision protection
- versioned command/query envelopes and deterministic discovery catalogs
- `project.rename` v1 and `project.summary` v1
- structured errors, checked revision mutation, and read-only query proof

Phase 4D — DONE:

- rename-only atomic transaction groups with one net `ChangeSet`
- one persistent revision increment per changed transaction; no increment for net no-ops or failed groups
- in-memory, per-session undo/redo history that is not part of `.orproj`

Phase 4E1 — DONE:

- bounded 64 MiB `.orproj` filesystem load with strict UTF-8 and v1 codec validation
- same-directory temporary writes, file sync, atomic replacement, and post-replace durability reporting
- macOS, Linux, and Windows storage integration verification

Phase 4UI-1 — DONE:

- production-direction Flutter visual foundation using OR Focused Monochrome tokens
- primary app shell and Home, Projects, Templates, Asset Library, and Settings surfaces
- Advanced / Developer diagnostics preserved through the Rust gateway
- responsive desktop and compact/mobile shell
- non-functional Editor Shell Preview for visual evaluation only

Phase 4E2 — DONE:

- crash-recovery snapshot checkpoint v1 in a separate bounded sidecar
- exact saved-base ancestry validation
- candidate, stale, and conflict inspection
- explicit apply and discard with atomic replacement and cleanup semantics
- cross-platform recovery tests

Phase 4F — DONE:

- transport-independent `ApplicationRequest` / `ApplicationResponse` dispatch and exact-base `ProjectFileSession`
- bounded, versioned, strictly parsed local IPC with per-server authentication
- Unix-domain sockets on macOS and Linux; Windows named pipes with protected owner-only DACLs that reject remote clients; no TCP fallback
- headless project summary, rename, and recovery status/apply/discard commands
- attached summary, rename, undo/redo, save, describe, and guarded shutdown commands
- developer/headless `or session serve` host; Flutter live-host integration is completed in Phase 4UI-2
- Linux, macOS, and Windows IPC integration coverage plus CLI contract tests

Phase 4UI-2 — DONE:

- desktop New/Open Project through the official Flutter file selector and Rust-owned no-clobber/file-session APIs
- one `LiveProjectHost` and one `ProjectFileSession`, shared by the opaque Flutter bridge handle and authenticated local IPC
- real project summary, rename, undo/redo, explicit save, close, and ordered invalidation-driven Flutter refresh
- explicit recovery candidate/stale/conflict/invalid handling, dirty close/switch/exit guards, and macOS sandbox-safe IPC endpoints
- attached CLI parity against the same host, including project/runtime IDs, revision, history, dirty state, save, and event ordering
- Android project New/Open remain unavailable pending Storage Access Framework integration

The first real project migration, schema v1 to v2, is implemented and tested. Further schema migrations remain future work and must stay explicit, ordered, and tested.

Phase 4 is complete. Both the Flutter application and `or session serve` can host projects; one Rust `LiveProjectHost` shares a single session between direct typed bridge access and attached CLI requests. Android project file access still awaits SAF. Phase 4 does not implement the real-time media pipeline, media engine, renderer, audio playback, or hardware acceleration. It establishes project state, time, shared commands and queries, transactions and history, serialization, bounded filesystem persistence, recovery, local IPC, and semantic CLI operations while preserving control-plane/media-plane separation and the rule that per-frame work never mutates Project or increments `ProjectRevision`.

### Phase 5 — Media foundation
**Status: DONE / FOUNDATION COMPLETE**

Phase 5A — DONE:

- typed UUIDv4 `MediaId` and `JobId`
- bounded read-only local media probe with structured errors
- validated format, duration, file-size, video, audio, and other-stream metadata
- exact decimal duration and rational frame-rate parsing
- external system-provided `ffprobe` metadata adapter; no linked or bundled FFmpeg
- minimal `MediaProbe` job kind and lifecycle states, without a scheduler
- `or media probe --file PATH` human and OR JSON output
- tiny generated synthetic media fixture probed by hosted Linux CI

Still not implemented:

- project media library or media import mutation
- media persistence in `.orproj`, schema v2, or v1-to-v2 migration
- media picker or media library UI
- thumbnails, waveforms, proxies, or cache database
- job manager, scheduler, thread pool, priority, or backpressure system
- timeline, decoder, playback, rendering, or export

Phase 5B — DONE:

- persistent project media library in `.orproj` schema v2
- schema v1 loads into an empty in-memory media library; the next explicit save writes v2 without incrementing revision solely for conversion
- validated local-file source references and bounded persisted metadata; source bytes remain external
- prepared import, `media.add`, and `media.remove` through the shared command path
- undo/redo with item identity and insertion order preserved
- bounded, paginated `media.list`
- headless and attached CLI list/add/remove parity
- desktop Flutter media import, list, and remove through the Rust-owned project host
- save/reopen and recovery compatibility, including an offline source reference

Still not implemented:

- thumbnails, waveforms, proxies, or cache storage
- a background Job Manager or scheduler
- decode, timeline editing, playback, rendering, or export

Phase 5C — DONE:

- bounded background Job Manager with explicit non-zero worker, queue, and record bounds
- fixed worker pool (never one thread per job), bounded pending queue, and non-blocking submit backpressure
- bounded tracked records with oldest-terminal reclamation; queued and running records are never evicted
- cooperative cancellation for queued and running jobs, and panic containment that keeps workers alive
- deterministic shutdown that stops submissions, skips queued work, signals running work, and joins workers
- disposable thumbnail and waveform cache namespaces with a caller-provided root and explicit entry/total budgets
- deterministic SHA-256 cache-key foundation over a schema version, artifact kind, source fingerprint, and parameters fingerprint
- bounded atomic cache storage with explicit remove, clear-namespace, and clear-all paths; Phase 5E later adds indexed automatic eviction

Not included in Phase 5C (the cache index and automatic eviction are added in Phase 5E):

- persistent cache index and automatic cache eviction
- proxy generation

Phase 5D — DONE:

- production bounded source-fingerprint v1 for disposable cache invalidation
- actual `Library Thumbnail V1` PNG generation from the first video stream's first decodable frame, scaled to a maximum 320-pixel edge without upscaling
- actual `Library Waveform V1` white 512×96 PNG generation from the first audio stream with channels combined
- bounded Job Manager and CacheStore integration, including same-key in-flight deduplication, backpressure, cancellation, and artifact events
- typed Rust bridge requests, reads, and events, plus desktop Flutter read-only Media-panel preview consumption
- system-provided `ffmpeg` execution; no linking or bundling

These are read-only library images; Phase 5D adds no proxy generation, timeline, playback, or render graph.

Phase 5E — DONE:

- persistent disposable SQLite cache index schema v1 at `cache-index.sqlite3`
- lazy startup/reopen reconciliation upgrades existing Phase 5D artifacts and repairs missing rows/files and size drift
- corrupt or incompatible index metadata is disposable and rebuilt from bounded scans of managed artifacts
- persistent access-sequence LRU ordering and indexed artifact-byte accounting
- automatic eviction removes only the minimum oldest set needed under the global artifact budget; the index database and journal are excluded from that budget
- cache artifacts and index metadata remain outside canonical project state and do not affect `ProjectRevision`

Phase 5F — DONE:

- disposable, file-backed `OR Library Proxy V1` generation through the existing `MediaArtifactService` and bounded `JobManager`
- Matroska `.mkv`, native FFmpeg `mpeg4`, first video stream only, bounded to 960×540 without upscaling, even dimensions, square pixels, and `yuv420p`
- preserves relative frame timing and normalizes the first presentation timestamp to zero; disables audio, subtitles, data, source metadata, and chapters
- proxy artifacts use the existing source fingerprint, schema-v1 cache key/index, shared global LRU, and total cache budget; preview byte reads stay bounded and proxies remain file-backed
- core-only foundation with cancellation, duration-aware timeout, staged-output size monitoring, atomic cache installation, and hosted real FFmpeg/ffprobe verification
- no project, revision, recovery, UI, Flutter API, CLI, IPC, playback, or Android proxy-generation changes

Phase 5 — DONE / FOUNDATION COMPLETE. Phase 6 — Timeline MVP is DONE. Phase 6A through 6E2B are complete, including the persistent marker UI and marker-aware Snap V2 GUI integration. Decode and playback remain Phase 7.

### Phase 6 — Timeline MVP
**Status: DONE**

Phase 6A — Timeline domain and persistence foundation — DONE. This adds canonical timeline state and `.orproj` v3 persistence. It does not add user-visible timeline behavior.

Phase 6B — Basic track/clip operations — DONE:

- append-only track add and empty-track-only remove
- clip insert in canonical start-time order, same-kind move, and explicit delete
- same-track overlap rejection, adjacency acceptance, and allowed cross-track overlap
- shared schema-v1 application commands and read-only, bounded timeline queries
- existing session-local ChangeSet history with undo/redo and monotonic revisions
- headless and attached semantic CLI parity using exact rational times
- `.orproj` remains schema v3; save/reopen and recovery preserve timeline state
- no grouped timeline transactions or Flutter timeline UI

Phase 6C — Real Flutter timeline foundation — DONE:

- typed Rust bridge for the existing bounded timeline queries and Phase 6B commands, using the same live host and Rust-generated track/clip IDs
- immutable exact-rational Dart read models with persisted media stream/container duration presentation data
- real project track/clip visualization in canonical order, with derived V#/A# labels, a time ruler, and position/width proportional clip blocks
- add Video/Audio tracks, remove empty tracks, insert media through an exact-time dialog, move between same-kind tracks through a dialog, and explicitly delete clips
- bounded first pages of 100 clips per populated track, on-demand Load more, revision/identity consistency checks, event-driven refresh, and read-snapshot clearing on project close/switch
- no new commands, queries, IPC version, dependency, project schema, or independent Flutter history/editing state
- no drag/drop, snap, markers, playback, decode, rendering, or timeline thumbnail/waveform strips

Phase 6D — Trim, split, and ripple editing — DONE:

- three schema-v1 commands: `timeline.clip.trim`, `timeline.clip.split`, and `timeline.clip.ripple_delete`; all are mutating, not transaction-allowed, and keep project schema v3/recovery v1/IPC v1 unchanged
- absolute exact-rational trim with known source-bound validation, strict interior exact split with Rust-generated UI IDs, and track-local ripple delete that preserves gaps and leaves other tracks unchanged
- compact semantic ChangeSets and undo/redo with revision overflow, capacity, overlap, duplicate-ID, and atomic history-conflict guards
- headless/attached CLI forms with changed-only headless saves and explicit attached saves, plus typed Rust bridge/gateway methods
- Focused Monochrome action menu and exact dialogs for Move, Trim, Split, Delete, and Ripple Delete; no optimistic geometry, drag/drop, snapping, markers, playback, decode, rendering, or timeline thumbnails

Phase 6E1 — Pointer timeline editing and clip-edge snapping — DONE:

- one schema-v1 read-only `timeline.snap` query; the command catalog, IPC v1, `.orproj` schema v3, and recovery v1 remain unchanged
- fixed exact `1/8`-second threshold over timeline zero plus every other canonical clip start/end on all tracks; active clip boundaries are excluded
- deterministic move start/end-anchor and trim-edge resolution with O(total clips) scanning and an O(1) result; no markers or pagination-dependent candidate set
- exact nearest-1-ms pointer deltas from the original canonical time, same-kind lane targeting, pointer-priority start/end handles, presentation-only ghosts, and drop-time-only snap queries
- existing Rust move/trim commands remain the only mutations; revision/session checks reject stale query results and attached CLI invalidation cancels active gestures without retry
- default-on non-persisted Snap toggle, temporary cyan snap guide, typed bridge/gateway access, and headless/attached `or timeline snap` inspection
- exact Move/Trim dialogs remain arbitrary `NUM/DEN`; no live continuous snap loop

Phase 6E2A — Persistent marker foundation — DONE:

- global point-marker domain with UUIDv4 `MarkerId`, exact rational time, exact labels, bounded validation, and canonical time/ID ordering
- strict `.orproj` schema v4 encoder with v1/v2/v3 decoders, explicit-save migration, no conversion revision increment, and bounded marker deserialization
- four schema-v1 marker commands with semantic undo/redo, no-op/redo preservation, transaction rejection, and clip-edit marker stability
- bounded read-only `timeline.markers` pages and unchanged existing query wire shapes
- Snap V1 preservation plus marker-aware Snap V2; the catalog advertises both compatibility scopes and the GUI now uses V2
- headless and attached CLI marker list/add/move/rename/delete parity, exact rational parsing, generated IDs, and existing save/dirty semantics
- storage, recovery, local IPC, frame-bound, migration, and CLI regression coverage; marker presentation remains the separate 6E2B scope

Phase 6E2B — Persistent marker UI and Snap V2 GUI integration — DONE:

- typed marker read models, bounded marker paging, and generated bridge/gateway methods
- Focused Monochrome marker ruler with accessible Add, Move, Rename, and Delete actions backed by the existing Rust commands
- Flutter pointer editing switched from Snap V1 to canonical marker-aware Snap V2 with revision/session stale guards and marker feedback
- attached project-change refresh, save/reopen/recovery UI guards, exact rational presentation, widget checks, and bridge mapping coverage

Phase 6 completion is recorded after 6E2B. Media-to-timeline drag insertion, track reorder, multi-select, linked clips, zoom, playhead/scrubbing, playback, decode, rendering, and export remain outside this phase.

### Phase 7 — Preview and playback
**Status: PLANNED**

Locked contract: [PHASE_7.md](execution/phases/PHASE_7.md).

- **7A:** realtime architecture, `RenderSnapshot`, `FrameDescriptor`/
  `FrameLease`, budgets, capability selection, fallback, and dependency gates.
- **7B:** wgpu render spine and synthetic/offscreen foundation.
- **7C:** linked media runtime, FFmpeg-gated software decode, seek, audio, and
  bounded queues.
- **7D:** measured hardware decode and native-frame interop with fallback.
- **7E:** `or_audio`, master clock, bounded buffers, and A/V synchronization.
- **7F:** viewer/external texture, play/pause, seek, scrubbing, playhead, ruler,
  and frame step.
- **7G:** performance metrics and budgets.
- **7H:** conformance, failure injection, and runtime hardening.

- wgpu and render graph foundation
- viewer
- native or external texture path
- video decode
- audio playback and A/V synchronization
- transform, crop, and opacity

#### Performance Architecture Gate

Before the hardware/media pipeline architecture is considered settled, complete this gate once enough Phase 5/7 prototypes and implementation exist to measure real behavior. It must evaluate software versus hardware decode, frame memory domains, CPU/GPU transfer count, native/external texture interoperability, render synchronization, buffering, hardware encode, correctness fallbacks, memory budgets, and scheduler/backpressure behavior. Use representative prototypes and benchmarks, not assumptions about platform capabilities. This gate does not delay Phase 4; it belongs when Phase 5/7 evidence can inform the choices.

### Phase 8 — Desktop MVP
**Status: PLANNED**

Locked contract: [PHASE_8.md](execution/phases/PHASE_8.md).

- **8A:** typed sequence settings and model/contract gate.
- **8B:** selection, duplicate, enabled/locked/solo tracks, zoom, direct media
  insertion, and bounded timeline usability.
- **8C:** transform, crop, and opacity.
- **8D:** basic text and manual captions.
- **8E:** gain, pan, fades, basic transitions, and effects.
- **8F:** export, autosave, recovery, save/reopen, and MVP hardening.

- basic text
- basic audio
- basic transitions and effects
- export
- autosave and recovery
- macOS, Windows, and Linux validation

#### MVP definition

A user can create a project, import media, edit a multitrack timeline, preview it, perform basic transforms and text and audio edits, undo and redo, save and reopen, and export a usable video. The GUI and CLI use the same core operations. Automatic captions are Phase 10; complex linked-clip semantics are advanced Phase 13 scope.

AI generation and community features are not MVP requirements. AI features may be added after the MVP through validated commands and explicit permissions.

### Phase 9 — Android
**Status: PLANNED**

Locked contract: [PHASE_9.md](execution/phases/PHASE_9.md). Checkpoints are 9A
(SAF and I/O), 9B (MediaCodec/native buffer/Vulkan/wgpu surface), 9C (mobile
UX), 9D (export), and 9E (resource/device hardening).

Use the same project and core model with mobile-native UI, Android storage integration, and resource-aware editing, playback, and export.

### Phase 10 — Captions, transcript, and AI assist
**Status: PLANNED**

Locked contract: [PHASE_10.md](execution/phases/PHASE_10.md). Checkpoints are
10A (AI/provider foundation), 10B (transcription), 10C (caption proposal/apply),
10D (transcript editing), 10E (translation), 10F (scene/silence/filler), 10G
(EditPlan/dry-run/diff), and 10H (permissions/model manager).

Add transcription, caption editing and export, translation, dubbing foundations, and selected assist workflows after the command and job foundations.

### Phase 11 — Templates, assets, and themes
**Status: PLANNED**

Locked contract: [PHASE_11.md](execution/phases/PHASE_11.md): 11A declarative
templates, 11B asset manifests/rights, 11C themes, and 11D static community
packaging.

Add declarative project templates, asset metadata and rights, and safe token-based themes.

### Phase 12 — Dubbing and voice
**Status: PLANNED**

Locked contract: [PHASE_12.md](execution/phases/PHASE_12.md): 12A voiceover, 12B
subtitle-to-speech, 12C translation/dubbing, 12D speaker mapping/pronunciation,
12E timing, and 12F regeneration/ducking. Voice cloning remains prohibited
pending its own consent checkpoint.

Add voiceover, subtitle-to-speech, translated dubbing, speaker mapping, pronunciation, and timing workflows. Voice cloning remains deferred pending an explicit consent and safety design.

### Phase 13 — Advanced editing, color, and audio
**Status: PLANNED**

Locked contract: [PHASE_13.md](execution/phases/PHASE_13.md): 13A keyframes,
13B speed, 13C transforms/color, 13D LUTs, 13E audio/mixer, 13F effects, 13G
linked/group semantics, and 13H nested timelines/multicamera.

Evaluate and add advanced keyframe, speed, color, effects, transition, audio, and multicamera capabilities as scoped.

### Phase 14 — AI generation
**Status: PLANNED**

Locked contract: [PHASE_14.md](execution/phases/PHASE_14.md): 14A generated
image, 14B video, 14C music/SFX, 14D voice, 14E provider/model/jobs, and 14F
provenance/review-before-timeline.

Evaluate image, video, music, sound-effect, and voice generation only after runtime, model license, hardware, and provider boundaries are understood.

### Phase 15 — Community ecosystem
**Status: PLANNED**

Locked contract: [PHASE_15.md](execution/phases/PHASE_15.md): 15A static
registry, 15B manifests/checksums, 15C license/compatibility/dependency
validation, and 15D reviewable publishing.

Start with a validated static registry and reviewable publishing. An OR-hosted backend is not a prerequisite.

### Phase 16 — Plugins, advanced interchange, and OpenFX evaluation
**Status: PLANNED**

Locked contract: [PHASE_16.md](execution/phases/PHASE_16.md): 16A plugin
security/capabilities, 16B sandbox/declarative-first, 16C permissions/resources/
network, 16D versioned APIs, 16E OTIO/EDL/XML evaluation, and 16F optional
high-trust OpenFX/native evaluation. Arbitrary native loading remains prohibited
by default.

Revisit sandboxed plugin capabilities, native or OpenFX compatibility, and advanced EDL or XML interchange with current security and platform research.

## Dependencies

Phase 4's project lifecycle foundation is complete. Phase 5A established bounded read-only metadata inspection, Phase 5B added persistent project media identity, source references, migration, shared media commands/query, CLI parity, and desktop library integration, Phase 5C added the bounded background Job Manager and disposable thumbnail/waveform cache foundations, Phase 5D added production cache-invalidation fingerprints and generated library PNG previews, Phase 5E added a persistent disposable cache index with automatic LRU eviction, and Phase 5F added the disposable file-backed Proxy V1 generation foundation. Phase 5 is DONE / FOUNDATION COMPLETE; Phase 6 — Timeline MVP is DONE through 6E2B. Phase 6B establishes application-owned basic timeline operations and semantic CLI parity; Phase 6C adds a real project track/clip view through the same Rust commands, bounded queries, and live host; Phase 6D adds exact trim, split, and track-local ripple-delete editing through the same Rust-owned path; Phase 6E1 adds pointer move/trim editing and canonical drop-time snapping; Phase 6E2A adds persistent markers and core/CLI Snap V2; Phase 6E2B adds the marker UI, bridge/gateway integration, and GUI Snap V2. Media-to-timeline drag insertion, track reorder, multi-select, zoom, playhead/scrubbing, playback, decode, rendering, and export remain future work until the linked Phase 7/8 checkpoints. Phases 3 and 4 establish the command, project, and job foundations required by nearly every later feature. Media and timeline work in Phases 5 and 6 precede reliable preview and export. Desktop MVP depends on save and recovery, media ingest, timeline operations, preview, basic editing tools, and export. Android reuses those core contracts but requires dedicated storage and resource validation. AI, templates, community, and plugins depend on structured project data, safe commands, and trust boundaries. See the machine-readable [execution plan](execution/README.md).
