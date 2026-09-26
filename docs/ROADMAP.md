# Roadmap

## Status

No dates or delivery promises are implied. Phase 2 is complete as Architecture Blueprint V1, Phase 3's executable architecture skeleton is complete, and Phase 4 is complete as a project/application foundation, not a finished editor. Phase 5 media foundation is in progress: Phase 5A and 5B are done, and Phase 5C is next. Later phases depend on implementation capacity, platform evidence, and licensing or security review.

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
**Status: IN PROGRESS**

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

Phase 5C — NEXT:

- bounded background Job Manager
- thumbnail cache foundation
- waveform cache foundation
- cancellation and backpressure basics

### Phase 6 — Timeline MVP
**Status: PLANNED**

- tracks and clips
- insert and move
- trim, split, ripple, and delete
- snap and markers
- undo and redo
- save and load
- CLI parity

### Phase 7 — Preview and playback
**Status: PLANNED**

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

- basic text
- basic audio
- basic transitions and effects
- export
- autosave and recovery
- macOS, Windows, and Linux validation

#### MVP definition

A user can create a project, import media, edit a multitrack timeline, preview it, perform basic transforms and text and audio edits, undo and redo, save and reopen, and export a usable video. The GUI and CLI use the same core operations.

AI generation and community features are not MVP requirements. AI features may be added after the MVP through validated commands and explicit permissions.

### Phase 9 — Android
**Status: PLANNED**

Use the same project and core model with mobile-native UI, Android storage integration, and resource-aware editing, playback, and export.

### Phase 10 — Captions, transcript, and AI assist
**Status: PLANNED**

Add transcription, caption editing and export, translation, dubbing foundations, and selected assist workflows after the command and job foundations.

### Phase 11 — Templates, assets, and themes
**Status: PLANNED**

Add declarative project templates, asset metadata and rights, and safe token-based themes.

### Phase 12 — Dubbing and voice
**Status: PLANNED**

Add voiceover, subtitle-to-speech, translated dubbing, speaker mapping, pronunciation, and timing workflows. Voice cloning remains deferred pending an explicit consent and safety design.

### Phase 13 — Advanced editing, color, and audio
**Status: PLANNED**

Evaluate and add advanced keyframe, speed, color, effects, transition, audio, and multicamera capabilities as scoped.

### Phase 14 — AI generation
**Status: PLANNED**

Evaluate image, video, music, sound-effect, and voice generation only after runtime, model license, hardware, and provider boundaries are understood.

### Phase 15 — Community ecosystem
**Status: PLANNED**

Start with a validated static registry and reviewable publishing. An OR-hosted backend is not a prerequisite.

### Phase 16 — Plugins, advanced interchange, and OpenFX evaluation
**Status: PLANNED**

Revisit sandboxed plugin capabilities, native or OpenFX compatibility, and advanced EDL or XML interchange with current security and platform research.

## Dependencies

Phase 4's project lifecycle foundation is complete. Phase 5A established bounded read-only metadata inspection, and Phase 5B added persistent project media identity, source references, migration, shared media commands/query, CLI parity, and desktop library integration. Phase 5C is the next checkpoint for a bounded background Job Manager and thumbnail/waveform cache foundations. Timeline and playback work remain later. Phases 3 and 4 establish the command, project, and job foundations required by nearly every later feature. Media and timeline work in Phases 5 and 6 precede reliable preview and export. Desktop MVP depends on save and recovery, media ingest, timeline operations, preview, basic editing tools, and export. Android reuses those core contracts but requires dedicated storage and resource validation. AI, templates, community, and plugins depend on structured project data, safe commands, and trust boundaries.
