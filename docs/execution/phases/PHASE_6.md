# Phase 6 — Timeline MVP completion

## Status

Phase 6 is active. Checkpoints 6A through 6E2A are complete in the current
repository state. Checkpoint 6E2B is the only `NEXT` checkpoint. This document
is a locked contract; mutable completion state belongs in `STATE.json`.

## Scope and boundary

Phase 6 completes the Rust-owned timeline editing foundation and the remaining
marker presentation work. It does not implement playback, decode, rendering,
media-to-timeline drag insertion, track reorder, multi-select, linked clips,
timeline zoom, or any Phase 7 runtime. The existing Phase 6 domain, command,
query, CLI, IPC, recovery, and Flutter bridge behavior remains authoritative.

The canonical state remains `ProjectDocument` in Rust. Flutter receives typed,
bounded read models and sends validated commands; it never becomes a second
editable project document. Exact timeline values remain `RationalTime`.

## Completed checkpoints

### 6A — Timeline domain and persistence foundation

Phase 6A added the Rust-owned `ProjectTimeline`, `.orproj` schema v3, strict
v1/v2-to-v3 migration, bounded track/clip decoding, and the existing recovery
compatibility. It added no timeline commands, IPC changes, Flutter timeline, or
playback. Media remains offline-tolerant; project loading validates stored
references without probing files.

### 6B — Basic track and clip operations

Phase 6B added validated `timeline.track.add`, `timeline.track.remove`,
`timeline.clip.insert`, `timeline.clip.move`, and `timeline.clip.delete`
through the shared application path, with bounded queries, semantic history,
undo/redo, CLI parity, overlap and same-kind validation, and stable error
codes. Track removal is empty-only; clip moves preserve identity and exact
source ranges; edit failures leave project, revision, history, and redo
unchanged.

### 6C — Real Flutter timeline foundation

Phase 6C added typed Flutter bridge wrappers for existing track/clip queries
and commands, immutable Dart read models, exact signed-64/unsigned-32 time
transport, bounded clip paging, stale snapshot guards, ordered
`project_changed` refresh, and a real track/clip workspace. The UI exposes
track creation/removal, exact-time clip insertion, same-kind moves, explicit
delete, and read-only timing details without optimistic geometry. It added no
drag/drop, snap, playback, decode, rendering, thumbnails, or waveforms.

### 6D — Trim, split, and track-local ripple delete

Phase 6D added exact trim, interior split, and track-local ripple-delete
commands, compact history recipes, CLI parity, typed bridge methods, and
action dialogs. Ripple deletion shifts only later clips on the selected track;
global markers and other tracks are not moved. Timeline commands remain
disallowed inside grouped transactions.

### 6E1 — Pointer timeline editing and clip-edge snapping

Phase 6E1 added pointer move/trim editing, a fixed-threshold canonical Snap V1
query, same-kind lane targeting, nearest-millisecond gesture quantization,
stale-result guards, a presentation-only ghost, and CLI snap inspection. Rust
retains the original exact time, quantizes only the total gesture delta, and
validates the final existing move/trim command. Flutter still uses Snap V1.

### 6E2A — Persistent marker foundation

Phase 6E2A added persistent global markers, schema-v4 migration, marker
commands/history/query, marker-aware Snap V2, and headless/attached CLI parity.
The marker model is Rust-owned and marker edits use the existing validated
application path. Flutter intentionally remains on Snap V1 until 6E2B.

## 6E2B — Persistent marker UI and Snap V2 GUI integration

### Goal

Expose the already-persistent marker model in the desktop Flutter workspace and
finish the GUI-side pointer integration without redefining marker semantics or
creating a parallel UI-owned marker document.

### Prerequisites

6E2A is `DONE`; project schema v4, recovery schema v1, and IPC protocol v1 are
unchanged unless a separate plan amendment proves otherwise.

### Allowed work

- Add typed Flutter marker read models, bounded marker query paging, and typed
  Rust bridge/gateway operations for the existing marker commands and query.
- Add marker ruler presentation and Add, Move, Rename, and Delete actions.
- Switch the Flutter pointer path from Snap V1 to the existing canonical Snap V2
  query, with marker-aware feedback and stale-result handling.
- Refresh the attached Flutter workspace after CLI `project_changed` events.
- Harden save, reopen, and recovery UI flows for marker-bearing projects while
  preserving exact-base conflict checks and explicit recovery behavior.
- Update Phase 6 documentation and focused tests/acceptance guards.

### Invariants that may be affected

`INV-STATE-001`, `INV-STATE-002`, `INV-STATE-003`, `INV-TIME-001`, `INV-UI-001`,
`INV-MEDIA-002`, `INV-PERSIST-001`, and `INV-IPC-001`.

### Required verification and documentation

Run the Rust marker/domain tests, Flutter bridge/widget tests that do not launch
the native application, CLI parity checks, repository hygiene, formatting, and
diff checks. Update the relevant product, workflow, roadmap, and UX acceptance
documents. Hosted CI remains authoritative for native/runtime checks.

### Explicitly out of scope

Do not add playback, frame decode, render surfaces, audio, timeline zoom,
media-to-timeline drag insertion, multi-select, track reorder, linked clips,
automatic captions, a new project schema version, an IPC protocol version, a
new runtime crate, or a Developer Preview.

## Stop conditions

Stop if marker behavior in the existing Rust model conflicts with this contract,
if a proposed UI path mutates canonical state directly, if the exact-base
recovery guard would be weakened, or if completing the checkpoint needs a schema,
IPC, dependency, or runtime boundary not described here.

## Handoff

The handoff must identify 6E2B, changed files, tests and checks, commit, push
state, and remaining hosted-CI evidence. The next relation is `7A` from
`PLAN.json`; Phase 7 must not be started in the same runner process.
