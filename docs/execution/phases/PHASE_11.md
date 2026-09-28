# Phase 11 — Templates, assets, and themes

## Status

Phase 11 is planned. It adds reviewable, declarative creative assets without
turning templates into executable plugins or granting them hidden project
mutation authority.

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

## Stop conditions

Stop for executable package contents, unclear asset rights, arbitrary effect
payloads, hidden network access, unbounded resource consumption, or a proposal
to make community metadata authoritative project state.

## Handoff

Report manifest/package schemas, rights checks, compatibility tests, safe import
behavior, docs, commit, and hosted packaging evidence.
