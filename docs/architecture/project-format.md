# `.orproj` Project Format

Last reviewed: 2026-11-01

Type: reference contract for the on-disk project file, for contributors and for
tooling that reads or validates it.

Not the source of truth for the internal domain model or migration
implementation; those live in `crates/or_core/src/project_document.rs`.

## Status and stability

OR is pre-MVP. The format is **not** a stable public interchange format and no
compatibility promise is made yet.

What is guaranteed today:

- Every version ever written is readable through explicit migrations; opening a
  v1 project in the current application works.
- Writing always produces the current version. There is no downgrade path and no
  "save as older version".
- An unknown future `schema_version` is rejected rather than guessed at, because
  silently misreading a project risks data loss.

## Envelope

UTF-8 JSON:

```json
{
  "format": "opencut-reinforced-project",
  "schema_version": 9,
  "project": {
    "id": "01234567-89ab-4def-8123-456789abcdef",
    "revision": 0,
    "name": "Example Project"
  }
}
```

| Field | Type | Meaning |
|---|---|---|
| `format` | string | Always `opencut-reinforced-project` |
| `schema_version` | integer | Document layout version; current **9** |
| `project` | object | Project identity and revision |

The authoritative constant is `CURRENT_PROJECT_SCHEMA_VERSION` in
`crates/or_core/src/project_document.rs`; this document changes in the same
commit that changes it.

## Version history

| Version | Change |
|---|---|
| 1 | Initial envelope: id, revision, name |
| 2 | Persistent media library references |
| 3 | Timeline domain: tracks and clips |
| 4 | Markers and snap schema |
| 5 | Text/caption clip types |
| 6 | Transform, crop, opacity and formatting on clips |
| 7 | Strict `FileUri` / `AndroidSafDocumentUri` source identities |
| 8 | Exact rational `playback_rate` on media clips; absent legacy values mean 1× |
| 9 | Optional sequence canvas (`width`/`height`) beside the sequence frame rate, so preview and export share one composition shape |

The codec and migration tests are the authority; if this table and those tests
disagree, the tests are right.

## Invariants a reader must preserve

1. **`revision` is monotonic and meaningful.** It is the optimistic-concurrency
   base for exact-base save. Rewriting it breaks dirty-state protection and
   conflict detection.
2. **Time is exact rational, not float.** Converting timeline positions through
   binary floating point loses time and breaks exact-edge trim and split.
3. **Reads validate, they do not resolve.** Opening a project validates stored
   media URIs but never opens or probes the referenced files. A missing source is
   a relink problem, not a load failure.
4. **Snapshots and recovery are separate.** Recovery data has its own envelope
   and version (`CURRENT_RECOVERY_SCHEMA_VERSION`) and is never written back as a
   project.
5. **Playback rate maps exact source and timeline time.** `playback_rate` is the
   exact number of source-time units consumed per timeline-time unit, bounded to
   1/16×–16×, and `source_range.duration == timeline_duration × playback_rate`.

## Writing a project safely

Do not hand-edit `.orproj` files to change project state. Use the application
command path (UI, CLI or agent API): it validates, computes the change set,
increments `revision` and records history. A file written outside that path is
not a valid edit because in-memory history and revision will not match disk.

For scripted inspection, read the file and treat every field as untrusted.

## Related

- [control-plane.md](control-plane.md) — the only valid write path
- [system-overview.md](system-overview.md)
