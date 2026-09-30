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

## 8A — Typed project model and timeline contract gate

Own the one project-model generalization needed by 8B–8E and the post-7F0
project-schema evolution. The verified baseline is schema v5; 8A may increment
it exactly once to schema v6 under the normal architecture policy. Do not
change recovery schema v1 or IPC protocol v1. Sequence frame rate and playback
timing remain 7F0 contracts.

Keep one Rust-owned editable timeline. Replace the media-only clip assumption
with stable `ClipId`, exact `timeline_start`, exact positive
`timeline_duration`, and closed typed `ClipContent` variants for `Media`,
`Text`, and `Caption`. Track kinds are a closed typed set containing `Video`,
`Audio`, `Text`, and `Caption`; do not use arbitrary strings or separate
editable text/caption timelines. All timed items use stable IDs and exact
`RationalTime`. Media content retains `MediaId` and exact `source_range`; every
old media clip migrates losslessly with `timeline_duration` equal to its old
`source_range.duration`. Playback speed remains exactly 1x through Desktop
MVP; Phase 13B owns speed and time remapping. Text and caption items have no
fake `MediaId` or `source_range`. One canonical caption cue is one caption
clip.

Persist closed typed track state for lock, visual visibility, mute, and solo.
Lock controls editing eligibility only. Visibility controls Video, Text, and
Caption evaluation; mute controls Audio evaluation. Solo is evaluated by
medium and never destructively rewrites other track flags. Selection, hover,
timeline viewport, zoom, and temporary drag state remain presentation-only and
are not serialized into `ProjectDocument`.

Define bounded closed project structures for the surfaces consumed by 8C–8E:
transform, crop, opacity, text/caption content and basic formatting, audio
gain/pan/fade-in/fade-out, and basic transition/effect references. These are
typed data, not arbitrary JSON, executable expressions, or plugin state. 8A
also gives persistent text a typed, stable font identity; it never serializes
an accidental OS font path. 8D owns the bundled Inter identity details and
actual text shaping/rendering. 8A approves the command, query, CLI, IPC,
history, validation, and migration contracts; it does not implement UI or
render behavior.

Migration, save/reopen, recovery, and strict decoder tests are mandatory. The
gate must prove exact-time and existing media identity preservation, revision
and history behavior, closed-enum rejection, and compatibility through the
normal project persistence path.

Affected invariants: `INV-STATE-001`, `INV-STATE-002`, `INV-STATE-003`,
`INV-TIME-001`, `INV-UI-001`, `INV-TEXT-001`, `INV-PERSIST-001`,
`INV-IPC-001`, `INV-DEP-001`.

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
transcription/caption generation is explicitly deferred to Phase 10. At the
start of 8D, deliberately raise the workspace MSRV from Rust 1.85 to Rust 1.89
and record the dependency/MSRV transition. Use `cosmic-text` 0.19.0 in the
rendering/text runtime layer, never as a direct `or_core` dependency. Bundle
Inter 4.1 under the SIL Open Font License 1.1 as the deterministic baseline so
basic text export geometry does not depend on host-installed fonts. Before
distribution, pin the exact upstream release/tag and file, record its SHA-256,
license file, attribution, and provenance. Persistent text may refer to the
stable bundled Inter identity; do not serialize an accidental OS font path.
OS font browsing remains presentation-only until a selected font is
materialized/imported as a managed project asset. Flutter widget text is not
the render/export path. Phase 11 MotionScene must reuse this same shaping,
font-identity, and render path.

Affected invariants: `INV-STATE-001`, `INV-STATE-002`, `INV-STATE-003`,
`INV-RT-001`, `INV-RT-002`, `INV-TIME-001`, `INV-RENDER-001`, `INV-UI-001`,
`INV-PERSIST-001`, `INV-IPC-001`, `INV-TEXT-001`.

## 8E — Basic audio and effects

Add the locked, typed MVP set for audio gain, pan, and fades plus basic
transitions/effects whose semantics are explicitly documented. Route audio
through the Phase 7 clock/runtime and render through the shared typed snapshot.
Use `cpal` 0.18.1 for desktop device output when this checkpoint first
implements it. `cpal` belongs in `or_audio` and must never be a direct
`or_core` dependency. The device callback does not mutate project state, take
project locks, call providers/network, or perform unbounded allocation; it
consumes bounded prepared audio buffers. Use deterministic and device-neutral
tests plus hosted compile/build verification; shared-runner physical audio is
not required. Keep the effect/transition model closed and typed, preserving
the documented basic transition/audio controls. Do not add an arbitrary effect
language or plugin framework; advanced effect categories belong to Phase 13F.

Affected invariants: `INV-STATE-001`, `INV-STATE-002`, `INV-STATE-003`,
`INV-RT-001`, `INV-RT-002`, `INV-TIME-001`, `INV-RENDER-001`, `INV-UI-001`,
`INV-JOB-001`, `INV-PERSIST-001`, `INV-IPC-001`, `INV-DEP-001`.

## 8F — Export, autosave, and Desktop MVP hardening

Complete export through the same evaluated model and runtime boundaries as
preview. Add autosave, explicit recovery inspection, save/reopen, offline-media
behavior, and failure-safe progress/cancel. Verify preview/export timing,
captions, transforms, audio, effects, and marker persistence. Before first
export, use the mandatory correctness profile: Matroska container, FFV1 video,
and PCM S16LE audio through the linked software FFmpeg path. Extend the
approved FFmpeg 8.1.3 configuration only with the exact mux/encode components
required for this profile. H.264, H.265/HEVC, AV1, VP9, NVENC, VideoToolbox,
and MediaCodec encoding are not Desktop MVP requirements; optional delivery or
hardware profiles require separate build, license, patent/platform, and
hardware evidence. Preview and software export evaluate the same project
semantics.

Autosave periodically creates or updates the approved recovery checkpoint; it
never silently overwrites the canonical user project file. Explicit Save
remains the canonical project-file save action. Define the export request, job
status/progress, and cancellation contract through the existing
application/IPC path before implementing the exporter; export is runtime work
and does not mutate the project. Required hosted platform verification and
Developer Preview packaging may update only the exact workflow paths
authorized in `PLAN.json`.

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
