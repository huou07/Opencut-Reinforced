# Phase 8 — Desktop MVP

## Status

Phase 8 is planned and depends on the completed Phase 7 runtime gate. It is the
optimized Desktop MVP milestone. The milestone includes the existing Phase 6
timeline foundation, Phase 6E2B marker UI, Phase 7 preview/playback, and the
checkpoints below.

## MVP product map

The Desktop MVP must provide selection, duplicate, basic track enabled/locked/
solo behavior, timeline zoom, direct media-to-timeline insertion, playhead,
scrubbing, frame step, basic text/manual captions, basic audio gain/pan/fades,
basic transitions/effects, transform/crop/opacity, export, autosave/recovery,
save/reopen, and preview/playback. Automatic captions belong to Phase 10.

Complex linked-clip semantics, grouping, nested timelines, multicamera, and
other advanced editing semantics are later Phase 13 work unless a future
architecture amendment explicitly re-promotes them.

## 8A — Project sequence settings and typed timeline evolution

Define the smallest explicit typed model for sequence settings, selection,
zoom/view state boundaries, enabled/locked/solo track state, and any additional
MVP timeline concepts. Audit migration, recovery, query, command, CLI, and IPC
contracts before changing them. Keep display-only viewport state out of the
canonical project unless a product decision makes it persistent. No arbitrary
JSON effect or plugin state is accepted.

Gate: schema and contract review, explicit migration tests, exact time
preservation, revision/history behavior, and architecture-policy update if a
version changes.

Affected invariants: `INV-STATE-001`, `INV-STATE-002`, `INV-STATE-003`,
`INV-TIME-001`, `INV-UI-001`, `INV-PERSIST-001`, `INV-IPC-001`, `INV-DEP-001`.

## 8B — Timeline usability completion

Implement bounded, typed selection and duplicate actions; track enabled,
locked, and solo presentation/commands; timeline zoom and viewport behavior;
direct media-to-timeline insertion; and the keyboard/pointer paths needed for
the MVP. Reuse existing media and timeline commands. Keep selection and zoom
read models separate from canonical clip truth. Validate insertion through Rust,
refresh from Rust, and preserve exact media duration and track-kind rules.

Do not add linked-clip semantics, unbounded multi-select state, or playback
logic here; playback is Phase 7 and advanced relationships are Phase 13.

Affected invariants: `INV-STATE-001`, `INV-STATE-002`, `INV-STATE-003`,
`INV-TIME-001`, `INV-UI-001`, `INV-JOB-001`, `INV-IPC-001`.

## 8C — Video transform foundation

Add typed transform, crop, and opacity values with explicit validation and
deterministic evaluation through the render snapshot. Keep project values exact
and serializable; keep GPU resources and native handles runtime-only. Provide
reset/default behavior and bounded UI feedback. No arbitrary shader/effect
JSON or plugin API is introduced.

Affected invariants: `INV-STATE-001`, `INV-STATE-002`, `INV-STATE-003`,
`INV-RT-001`, `INV-RT-002`, `INV-TIME-001`, `INV-RENDER-001`,
`INV-RENDER-002`, `INV-RENDER-003`, `INV-IPC-001`.

## 8D — Basic text and manual captions

Add a typed text/title and manual-caption model, editing commands, bounded
layout/evaluation, and preview/export parity. Captions must remain ordinary
validated project edits, with stable IDs and exact timing. Automatic
transcription/caption generation is explicitly deferred to Phase 10.

Affected invariants: `INV-STATE-001`, `INV-STATE-002`, `INV-STATE-003`,
`INV-RT-001`, `INV-RT-002`, `INV-TIME-001`, `INV-RENDER-001`, `INV-UI-001`,
`INV-PERSIST-001`, `INV-IPC-001`.

## 8E — Basic audio and effects

Add the locked, typed MVP set for audio gain, pan, and fades plus basic
transitions/effects whose semantics are explicitly documented. Route audio
through the Phase 7 clock/runtime and render through the shared typed snapshot.
Use a bounded, reviewable effect set; do not create a plugin framework or
arbitrary effect payloads.

Affected invariants: `INV-STATE-001`, `INV-STATE-002`, `INV-STATE-003`,
`INV-RT-001`, `INV-RT-002`, `INV-TIME-001`, `INV-RENDER-001`, `INV-UI-001`,
`INV-JOB-001`, `INV-PERSIST-001`, `INV-IPC-001`, `INV-DEP-001`.

## 8F — Export, autosave, and Desktop MVP hardening

Complete export through the same evaluated model and runtime boundaries as
preview. Add autosave, explicit recovery inspection, save/reopen, offline-media
behavior, and failure-safe progress/cancel. Verify preview/export timing,
captions, transforms, audio, effects, and marker persistence. Record codec and
packaging license choices before release.

This is the Desktop MVP completion gate. It may require a future Developer
Preview release decision, but it does not authorize publishing by itself.

Affected invariants: all Phase 8 invariants, especially `INV-STATE-001`,
`INV-STATE-002`, `INV-STATE-003`, `INV-RT-001`, `INV-RT-002`, `INV-TIME-001`,
`INV-UI-001`, `INV-UI-002`, `INV-MEDIA-001`, `INV-MEDIA-002`, `INV-RENDER-001`,
`INV-RENDER-002`, `INV-RENDER-003`, `INV-JOB-001`, `INV-CACHE-001`,
`INV-PERSIST-001`, `INV-IPC-001`, `INV-SEC-001`, `INV-DEP-001`.

## Stop conditions

Stop if a feature requires linked-clip semantics, arbitrary effect/plugin JSON,
direct UI mutation, a second project model, a new runtime boundary, an
unreviewed schema/IPC break, or a codec/model/license decision without a gate.

## Handoff

Each checkpoint must report MVP requirement coverage, migration and recovery
evidence, preview/export parity, tests, docs, commit, and hosted CI status. The
milestone is complete only at 8F.
