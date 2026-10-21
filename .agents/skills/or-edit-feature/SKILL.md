---
name: or-edit-feature
description: Implement an authorized OR Rust editing command/query or project-lifecycle change through the shared application path. Use for canonical editing semantics, undo, persistence or CLI parity. Exclude UI-only work (or-ui-ux), failing-check diagnosis (verify-debug), codec/provider work (or-media-ai), planning and status reports.
---

# OR editing contracts

Read only the owning PRODUCT section and existing command/query callers.
[DEVELOPMENT_WORKFLOW.md](../../../docs/DEVELOPMENT_WORKFLOW.md) owns the journey
procedure; [ARCHITECTURE.md](../../../docs/ARCHITECTURE.md) and
[PROJECT_FORMAT.md](../../../docs/PROJECT_FORMAT.md) own state and compatibility.

1. Identify the user's editing goal and the existing validated command path.
2. Preserve ProjectId/ProjectInstanceId/revision preconditions, exact time,
   ChangeSet/history behavior, read-only queries and stale-result rejection.
   LiveProjectHost and attached CLI share one ProjectFileSession.
3. Implement the smallest scoped change. Flutter keeps presentation/read caches;
   queries, playback and workers never create canonical edits. Do not widen the
   core dependency boundary or change the format to simplify a test.
4. Test command validation, errors, undo and persistence as affected. If the
   request includes a GUI result, demonstrate the actual interaction and inspect
   Rust state; command tests alone leave that surface UNVERIFIED.
5. Update the existing authority, choose checks from TESTING, and stop at the
   requested capability. Do not add unrelated surfaces or invoke more skills
   merely because a checklist mentions them.

Stop on an architecture/compatibility conflict rather than bypassing it.
Report the command, actual surfaces, source/build, tests, observed journey,
evidence and remaining gaps. An implementation is not ACCEPTED without the
project's exact-source hosted/owner evidence.
