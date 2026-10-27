# Control Plane — Commands, Queries, Persistence, Sessions

Last reviewed: 2026-10-27

Scope: how project state changes, how it is read, how it is saved, and how
clients share one live session. Every editing feature goes through this path.

## The command path

Only this path mutates canonical project state:

```
client → validate identity + expected revision → apply → ChangeSet
       → one history entry → revision + 1 → events → clients refresh queries
```

- Commands carry `ProjectId`, `ProjectInstanceId` and the expected
  `ProjectRevision`. A stale revision is rejected so the client can re-query and
  revalidate; the rejected command changes nothing.
- One successful mutating command increments the revision exactly once. Failed
  commands and reads increment nothing.
- Undo and redo are themselves canonical mutations and increment the revision.
- A transaction groups several commands; a failure rolls the group back.
- GUI, CLI, agents, renderers, decoders, jobs and AI workers never write the
  project document directly.

Media history stores the exact item and its original insertion index instead of
whole-document snapshots.

## Runtime-only requests

Two requests run on the same application and IPC path without touching the
project revision: export (`ExportRequest`) and MotionScene materialisation
(`MotionMaterializeRequest`). Both are handled by an application runtime that the
headless host does not have; without one they return an explicit unavailable
error. Materialisation writes a persistent file, and the caller registers it
through ordinary media and timeline commands.

`or mcp` is an MCP stdio server that exposes these commands and queries to an
external agent. It holds no project state: every tool call attaches through the
live descriptor, so the CLI, the GUI and the assistant share one session.

## Queries

Queries are read-only and never mutate a project or bump its revision. They
expose project identity, timeline pages, media, captions, commands and
capabilities. Paginated timeline queries are bounded (a page is well under the
1 MiB IPC limit); clients request the next page explicitly rather than loading a
whole timeline.

## Persistence and recovery

- The canonical file is a versioned `.orproj` document; see
  [project-format.md](project-format.md).
- Save is explicit. It performs a bounded atomic replacement after an exact-base
  check, so a concurrent writer cannot silently clobber a newer file.
- A dirty project periodically updates a **recovery sidecar** (30 s in the
  Flutter shell). Autosave never overwrites the canonical project file.
- Recovery inspection, apply and discard are explicit user actions.
- Clean open preserves the on-disk bytes; an explicit save writes the current
  schema version.

## Session and IPC

The Flutter application and a developer/headless `or session serve` can both
host a session. The app's opaque bridge handle and the IPC worker share exactly
one `ProjectFileSession`; there is no second copy of the project.

Protocol v1 (local only):

- Unix-domain sockets on macOS/Linux, named pipes on Windows.
- Four-byte big-endian length prefix, 1 MiB request limit, versioned JSON
  envelopes, request ID, and a random per-server authentication token in an
  explicit endpoint descriptor.
- One request per connection. Unix runtime directories, sockets and descriptors
  are owner-only; the Windows runtime directory, descriptor and pipe use
  protected owner-only DACLs and reject remote clients.
- The server exposes describe, shared application requests, explicit save and
  guarded shutdown. It accepts no arbitrary paths and no shell commands.
- No TCP, HTTP, WebSocket or network fallback exists.

This transport isolates against other OS users, not against malicious code
running as the same user.

## CLI parity

The CLI is a first-class semantic client, not UI automation. It supports
headless operation and attachment to either the Flutter host or a headless host,
using the same command/query dispatch. `--attach` takes the endpoint descriptor
or a session pointer that names it in `descriptor_path` — the workspace
`project.json` and the assistant `session-pointer.json` are both pointers — and
re-reads the pointer on every call, so one command line survives a project switch
without going stale. Parity tests launch the real CLI against a real live host
and compare IDs, revisions, undo/redo, dirty state, save and event order.

## Where this lives

- Domain, commands, history, persistence: `crates/or_core/`
- Bridge API: `crates/or_app_bridge/src/api/`
- IPC: `crates/or_ipc/`
- CLI: `crates/or_cli/`
- Flutter gateway: `apps/or_app/lib/` (`ProjectGateway` and its callers)

## Related

- [system-overview.md](system-overview.md)
- [realtime-plane.md](realtime-plane.md)
- [project-format.md](project-format.md)
- [features/editing.md](../features/editing.md)
