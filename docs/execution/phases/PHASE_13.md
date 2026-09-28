# Phase 13 — Advanced editing, color, and audio

## Status

Phase 13 is planned. These are advanced, typed editing capabilities after the
Desktop MVP. They must extend the shared command/state model rather than add a
second editor or arbitrary effect language.

## 13A — Typed keyframes

Add typed keyframe tracks, interpolation policies, exact time, validation,
history, and deterministic snapshot evaluation. Keyframes are project data;
runtime caches and GPU resources are not serialized.

Affected invariants: `INV-STATE-001`, `INV-STATE-002`, `INV-STATE-003`,
`INV-RT-001`, `INV-RT-002`, `INV-TIME-001`, `INV-RENDER-001`,
`INV-PERSIST-001`, `INV-IPC-001`.

## 13B — Speed

Add typed speed/rate and time-remapping semantics with explicit source-to-
timeline mapping, audio policy, exact boundaries, preview/export parity, and
bounded validation. No floating-point-only canonical time is accepted.

Affected invariants: `INV-RT-001`, `INV-RT-002`, `INV-TIME-001`,
`INV-MEDIA-001`, `INV-MEDIA-002`, `INV-RENDER-001`, `INV-PERSIST-001`.

## 13C — Transforms and color

Extend typed transforms, color adjustments, and deterministic evaluation. Keep
color-management policy explicit and backend-neutral; runtime GPU/native state
remains outside project serialization.

Affected invariants: `INV-RT-002`, `INV-RENDER-001`, `INV-RENDER-002`,
`INV-RENDER-003`, `INV-PERSIST-001`, `INV-DEP-001`.

## 13D — LUTs

Add reviewed LUT assets/manifests, checksums, color-space metadata, rights, and
fallback behavior. LUT application is typed and validated; unknown or unsafe
assets fail closed without breaking project readability.

Affected invariants: `INV-RENDER-001`, `INV-RENDER-002`, `INV-CACHE-001`,
`INV-PERSIST-001`, `INV-SEC-001`, `INV-DEP-001`.

## 13E — Audio and mixer

Extend typed audio gain/pan/fades into a bounded mixer with buses, automation,
metering, and deterministic preview/export behavior. Realtime paths remain
allocation-free and do not hold project locks or call providers.

Affected invariants: `INV-RT-001`, `INV-RT-002`, `INV-TIME-001`,
`INV-JOB-001`, `INV-PERSIST-001`, `INV-DEP-001`.

## 13F — Effects

Expand the reviewed typed effect set through explicit schemas and capability
checks. Do not accept arbitrary effect JSON or an executable plugin path under
this checkpoint.

Affected invariants: `INV-STATE-002`, `INV-RT-002`, `INV-RENDER-001`,
`INV-RENDER-002`, `INV-RENDER-003`, `INV-PERSIST-001`, `INV-DEP-001`.

## 13G — Linked and group semantics

Define linked clips, groups, and relationship identities explicitly, with
command validation, undo/redo, stale revision checks, migration, and clear
cross-track/trim/ripple semantics. This is the advanced home for complex linked
clip behavior deferred from the Desktop MVP.

Affected invariants: `INV-STATE-001`, `INV-STATE-002`, `INV-STATE-003`,
`INV-TIME-001`, `INV-PERSIST-001`, `INV-IPC-001`.

## 13H — Nested timelines and multicamera

Add typed nested sequence and multicamera models with bounded evaluation,
resource budgets, source switching semantics, and preview/export conformance.
Keep nested runtime snapshots versioned and disposable.

Affected invariants: `INV-STATE-001`, `INV-STATE-002`, `INV-STATE-003`,
`INV-RT-001`, `INV-RT-002`, `INV-TIME-001`, `INV-MEDIA-001`,
`INV-RENDER-001`, `INV-JOB-001`, `INV-PERSIST-001`, `INV-IPC-001`.

## Stop conditions

Stop for arbitrary effect JSON, hidden relationship semantics, nondeterministic
time mapping, project/runtime ownership mixing, unbounded nested evaluation, or
native handles in serialized state.

## Handoff

Report typed schemas, migration/recovery, preview/export parity, performance
budgets, advanced editing acceptance, docs, commit, and hosted evidence.
