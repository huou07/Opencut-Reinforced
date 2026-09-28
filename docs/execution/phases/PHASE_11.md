# Phase 11 — Templates, assets, themes, and motion scenes

## Status

Phase 11 is planned. It adds reviewable, declarative creative assets without
turning templates into executable plugins or granting them hidden project
mutation authority. Checkpoints 11A–11D retain their existing contracts. 11E
defines the non-AI canonical MotionScene foundation, and 11F makes it useful
through materialization and a vendor-neutral agent/CLI workflow.

## 11A — Declarative templates

Define versioned template manifests containing typed layout, text, timing,
media slots, and approved effect references. Templates instantiate through
normal validated commands and remain inspectable/diffable. No embedded
executable code, arbitrary shader, or provider call is allowed.

Affected invariants: `INV-STATE-001`, `INV-STATE-002`, `INV-STATE-003`,
`INV-TIME-001`, `INV-RENDER-001`, `INV-AI-003`, `INV-PERSIST-001`,
`INV-SEC-001`, `INV-DEP-001`.

## 11B — Asset manifests and rights

Add typed asset manifests with stable IDs, checksums, provenance, source/license
rights, attribution requirements, media metadata, and offline behavior. Asset
references remain portable and cacheable without making cache presence required
for project correctness.

Affected invariants: `INV-STATE-003`, `INV-MEDIA-001`, `INV-CACHE-001`,
`INV-PERSIST-001`, `INV-SEC-001`, `INV-DEP-001`.

## 11C — Themes

Add bounded, declarative themes for typography, colors, spacing, and approved
component variants while preserving the Focused Monochrome product baseline.
Themes must not introduce marketing UI, decorative clutter, or executable
behavior. Theme application is a typed, reviewable project/presentation change.

Affected invariants: `INV-STATE-001`, `INV-STATE-002`, `INV-UI-001`,
`INV-PERSIST-001`, `INV-DEP-001`.

## 11D — Static community packaging

Define a static package format and review/import checks for templates, assets,
and themes. Verify checksums, manifests, licenses, compatibility, and safe
resource limits before installation. Do not add a backend, fake popularity
metrics, executable community code, or unreviewed network behavior.

Affected invariants: `INV-CACHE-001`, `INV-SEC-001`, `INV-DEP-001`,
`INV-PERSIST-001`.

## 11E — MotionScene V1 and deterministic motion evaluation

Define the future `.ormotion.json` source contract and a machine-readable JSON
Schema mirror where practical. The source is bounded, strict, non-executable
data with stable scene, node, asset, and animation identities. It uses exact
`RationalTime`, a half-open local timeline, typed primitives (`Group`, `Text`,
`Rectangle`, `Ellipse`, `Line`/`Arrow`, renderer-gated `Polyline`, and `Image`),
typed keyframes and a small locked easing set. It has no expression language,
script, shell, provider call, network lookup, environment lookup, filesystem
glob, arbitrary shader, or hidden font download.

11E freezes resource bounds, missing-asset/font diagnostics, reproducible font
references, deterministic seeded-randomness rules, semantic content/hash/version
identity, and a random-access evaluator. The evaluator lowers into the actual
Phase 7/8 `RenderSnapshot` and `or_render` primitives available when this
checkpoint is implemented; it does not create a speculative second renderer.
Preview and materialization use the same evaluator. Charts and diagrams begin
as typed templates composed from the basic primitives.

The checkpoint is an architecture gate with no AI, browser, JavaScript,
Remotion, Motion Canvas, live `MotionClip`, or executable community code. No
browser, Node, or runtime dependency is introduced speculatively. Any small
parser, text, or vector dependency must pass the normal license, MSRV,
platform, and concrete-need gates.

Future test categories are schema strictness and unknown fields, resource
bounds, identity/canonical ordering, exact and invalid timing, random access
and seek equivalence, same-time determinism without a wall clock, no-network
evaluation, missing asset/font diagnostics, semantic hashing, RenderSnapshot
lowering, and preview/materialization semantic parity.

Affected invariants: `INV-STATE-003`, `INV-TIME-001`, `INV-RENDER-001`,
`INV-RENDER-002`, `INV-RENDER-003`, `INV-MOTION-001`, `INV-MOTION-002`,
`INV-MOTION-004`, `INV-PERSIST-001`, `INV-SEC-001`, `INV-DEP-001`.

## 11F — Motion materialization and agent/CLI workflow

Add a vendor-neutral semantic workflow for human files, templates, Codex,
Claude Code, OpenCode, and other external harnesses. The future CLI may expose
`or motion validate`, `or motion inspect`, and `or motion render`; exact names
and options are frozen by 11E/11F. Validation and inspection are read-only.
Rendering is also read-only unless a separate explicit materialization or
project-application command is requested.

MotionScene UX treats the source as an editable source asset with a validation
state, duration, canvas, missing-asset diagnostics, Preview, Render/Rerender,
and Add rendered media to project/timeline. The flow is materialization first:
render to persistent generated media, associate source/provenance through the
approved asset-manifest system, register through normal media commands, and
use ordinary timeline clip operations. Re-rendering creates new content
identity rather than silently replacing bytes behind an existing `MediaId`.

This checkpoint does not require a full node graph, keyframe curve editor,
After Effects clone, browser IDE, or code editor. It does not add arbitrary
code, a live `MotionClip`, or an AI provider requirement. TTS, music, and SFX
remain ordinary OR audio/media/timeline concerns, coordinated by an agent or
`EditPlan` when needed. Normal OR encoding is used; browser WebCodecs is not a
canonical encoder.

Future tests cover CLI validation/inspection/render, JSON diagnostics,
external-agent fixtures, persistent generated media, cache-independent project
references, new identity on re-render, normal `media.add` and timeline paths,
save/reopen, recovery, offline sources, and cancellation/failure handling.

Affected invariants: `INV-STATE-001`, `INV-STATE-002`, `INV-STATE-003`,
`INV-MEDIA-001`, `INV-MEDIA-002`, `INV-MOTION-001`, `INV-MOTION-002`,
`INV-MOTION-004`, `INV-AI-002`, `INV-AI-003`, `INV-PERSIST-001`, `INV-SEC-001`.

## Stop conditions

Stop for executable package contents, unclear asset rights, arbitrary effect
payloads, hidden network access, unbounded resource consumption, a proposal to
make community metadata authoritative project state, arbitrary code in
MotionScene, provider calls during rendering, a live `MotionClip` without a
new gate, or a browser/Node dependency in the canonical path.

## Handoff

Report manifest/package schemas, rights checks, compatibility tests, safe import
behavior, MotionScene schema/evaluator/materialization and agent contract
evidence when applicable, docs, commit, and hosted packaging evidence.
