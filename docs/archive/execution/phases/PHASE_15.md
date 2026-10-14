# Phase 15 — Community ecosystem

## Status

Phase 15 is planned. Community distribution is static, reviewable, and
manifest-driven. It is not a backend, social graph, executable marketplace, or
fake popularity system.

## 15A — Static registry

Define a local/static registry format for declarative templates, assets, themes,
and approved model metadata. Registry entries are content-addressed and
reviewable; they do not control project state or claim popularity metrics.

Affected invariants: `INV-CACHE-001`, `INV-PERSIST-001`, `INV-SEC-001`,
`INV-DEP-001`, `INV-AI-004`.

## 15B — Manifests and checksums

Require versioned manifests, checksums, source/provenance, license/attribution,
compatibility, resource limits, and safe failure behavior. Verify content before
install/use; cache remains disposable.

Affected invariants: `INV-CACHE-001`, `INV-PERSIST-001`, `INV-SEC-001`,
`INV-DEP-001`, `INV-AI-004`.

## 15C — License, compatibility, and dependency validation

Validate package/model/asset rights, platform compatibility, dependency policy,
runtime requirements, and transitive licenses before a package is accepted.
Reject executable or distribution-restrictive content without an explicit
architecture and licensing decision.

Affected invariants: `INV-SEC-001`, `INV-DEP-001`, `INV-AI-001`, `INV-AI-004`,
`INV-CACHE-001`.

## 15D — Reviewable publishing

Provide export/review/import artifacts suitable for maintainers and users.
Publishing is reviewable and deterministic, with no backend dependency, fake
downloads/likes/rankings, hidden telemetry, or arbitrary native code.

Affected invariants: `INV-SEC-001`, `INV-DEP-001`, `INV-PERSIST-001`.

## Stop conditions

Stop for executable packages, hidden network/backend requirements, unclear
rights, popularity manipulation, unbounded assets, or unreviewed dependencies.

## Handoff

Report registry/package schemas, validation evidence, licensing review, static
artifact examples, docs, commit, and hosted packaging checks.
