# Roadmap

## Status

No dates or delivery promises are implied. Phase 2 is complete as Architecture Blueprint V1 and Phase 3's executable architecture skeleton is complete. Phase 4 is next; it starts project and command foundations, not a finished editor. Later phases depend on implementation capacity, platform evidence, and licensing or security review.

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
**Status: IN PROGRESS**

Phase 4A foundational values are implemented in `or_core`:

- exact rational time, rate, and range primitives
- typed UUIDv4 persistent project identity
- ephemeral runtime project-instance identity
- checked persistent project revision

Remaining Phase 4 work:

- ProjectDocument and .orproj schema
- commands and queries
- undo and redo
- serialization and migrations
- atomic save and crash journal
- local IPC
- CLI parity

Phase 4 does not implement the real-time media pipeline, media engine, renderer, audio playback, or hardware acceleration. It establishes project state, time, commands and queries, transactions and history, serialization, and IPC while preserving control-plane/media-plane separation, a snapshot-ready project/timeline evaluation boundary, and the rule that per-frame work never mutates Project or increments `ProjectRevision`.

### Phase 5 — Media foundation
**Status: PLANNED**

- media import and probing
- metadata
- thumbnails and waveforms
- shared job system
- proxy and cache foundation

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

Phases 3 and 4 establish the command, project, and job foundations required by nearly every later feature. Media and timeline work in Phases 5 and 6 precede reliable preview and export. Desktop MVP depends on save and recovery, media ingest, timeline operations, preview, basic editing tools, and export. Android reuses those core contracts but requires dedicated storage and resource validation. AI, templates, community, and plugins depend on structured project data, safe commands, and trust boundaries.
