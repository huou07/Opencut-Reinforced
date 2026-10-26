# `.orproj` project format

Last reviewed: 2026-10-22

Type: Reference

Purpose: the on-disk contract for OR project files, for contributors and for
third-party tooling that wants to read or validate them.

Not a source of truth for: the internal domain model, migration implementation,
or the sequence in which fields were introduced. Those live in
[ARCHITECTURE.md](ARCHITECTURE.md), [TECHNICAL_PLAN.md](TECHNICAL_PLAN.md)
section 3, and `crates/or_core/src/project_document.rs`.

## Status and stability

OR is pre-MVP. The format is **not** a stable public interchange format yet, and
no compatibility promise is made. It changes with the product.

Work stream G in [PRODUCT_ROADMAP.md](PRODUCT_ROADMAP.md) covers making command
and interchange contracts stable enough to depend on from outside this
repository. Until that work lands, treat the format as internal: readable and
writable by this codebase and its tests, and nothing more.

What *is* guaranteed today:

- Every version ever written is readable by the current code, through explicit
  migrations. Opening a v1 project in the current application works.
- Writing always produces the current version. There is no downgrade path, and
  no "save as older version" feature.
- An unknown future `schema_version` is rejected rather than guessed at. This is
  deliberate: silently misreading a project would risk data loss.

## Envelope

UTF-8 JSON. The top level is:

```json
{
  "format": "opencut-reinforced-project",
  "schema_version": 8,
  "project": {
    "id": "01234567-89ab-4def-8123-456789abcdef",
    "revision": 0,
    "name": "Example Project"
  }
}
```

| Field | Type | Meaning |
|---|---|---|
| `format` | string | Always `opencut-reinforced-project`. A different marker is not an OR project |
| `schema_version` | integer | Version of the document layout below. Current: **8** |
| `project` | object | Project identity and revision |

The authoritative constant is `CURRENT_PROJECT_SCHEMA_VERSION` in
`crates/or_core/src/project_document.rs`. This document must be updated in the
same change that changes it.

## Version history

Each version records what changed, because a field that appeared or changed
meaning is the thing external tooling breaks on.

| Version | Change |
|---|---|
| 1 | Initial project envelope: id, revision, name |
| 2 | Persistent media library references |
| 3 | Timeline domain: tracks and clips |
| 4 | Markers and Snap schema |
| 5 | Text/caption clip types |
| 6 | Transformation and formatting on clips |
| 7 | Strict `FileUri` / `AndroidSafDocumentUri` media source identities |
| 8 | Current. Exact rational `playback_rate` on media clips; absent legacy values mean 1× |

The authoritative migration chain and per-version field detail is
[TECHNICAL_PLAN.md](TECHNICAL_PLAN.md) section 3. The implementation is the
codec and migration tests in `crates/or_core/src/project_document.rs` — if this
table and those tests ever disagree, the tests are right.

## Invariants a reader must preserve

1. **`revision` is monotonic and meaningful.** It is the optimistic-concurrency
   base for exact-base save checks. A tool that rewrites `revision` breaks
   dirty-state protection and conflict detection.
2. **Time is exact rational, not float.** Timeline positions and durations are
   exact rational values. Converting to binary floating point and back loses
   time and will fail exact-edge trim and split operations.
3. **Reads validate, they do not resolve.** Opening a project validates stored
   media URIs but never opens or probes the referenced files. A missing or
   offline source is a relink problem, not a load failure.
4. **Snapshots and recovery are separate.** Recovery data has its own envelope
   and version (`CURRENT_RECOVERY_SCHEMA_VERSION`). It is not the project
   document, and a recovery record must never be written back as a project.
5. **Playback rate maps exact source and timeline time.** A media clip's
   `playback_rate` is the exact number of source-time units consumed per
   timeline-time unit, bounded to 1/16×–16×. Its source range duration equals
   `timeline_duration × playback_rate`. Files from v1–v7 omit the field and
   decode it as the identity rate, 1/1.

## Writing a project safely

Do not hand-edit `.orproj` files to change project state. Use the application
command path — the CLI, the agent command API, or the UI. The command path is
what validates, computes the change set, increments `revision`, and records
undo history. A file written outside it is not a valid edit, because the
in-memory history and the revision counter will not match what is on disk.

For scripted inspection, read the file and treat every field as untrusted.

## Related

- [TECHNICAL_PLAN.md](TECHNICAL_PLAN.md) section 3 — native project format and
  subsystem data contracts
- [ARCHITECTURE.md](ARCHITECTURE.md) — subsystem boundaries and current status
- [SECURITY_LICENSING.md](SECURITY_LICENSING.md) — asset and fixture rights
