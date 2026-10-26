# Change Workflow

Last reviewed: 2026-10-26

Scope: the path from a requested outcome to a verified local increment. Read the
feature and architecture document that owns the area, not all of them.

## Work loop

1. **Inspect the checkout.** Record branch, HEAD, dirty files and worktrees.
   Preserve unrelated work and user data.
2. **Pick the blocker.** Use [reference/roadmap.md](../reference/roadmap.md) and the
   user's requested journey; open only the owning feature document.
3. **Inspect the real implementation.** Use CodeGraph before searching or reading
   indexed source; trace callers and reuse the existing command, media and render
   paths.
4. **Make the smallest complete change.** Keep project mutations in the
   Rust-owned validated command path, preserve revisions/undo/recovery, and add
   the smallest useful regression guard.
5. **Exercise the affected path.** For user-visible work, build and run the real
   macOS app with an isolated profile, observe the interaction and the resulting
   project state, and inspect exports independently. See
   [local-macos.md](local-macos.md) and [ui-harness.md](ui-harness.md).
6. **Report and commit.** State checks, build, source revision, artifacts and
   anything unverified; commit one coherent verified increment. See [git.md](git.md).

## Repair discipline

After two speculative fixes for the same failure, stop editing. Write down the
observed result, the expected result, the evidence and the attempts, then run a
check that distinguishes the hypothesis before touching code again.

## Editing-state rules

- Rust owns canonical project state and history; Flutter owns presentation and
  bounded read models.
- Queries, playback and rendering are runtime activity, not mutations.
- Preserve exact timeline time, revision checks, explicit save, recovery and
  compatibility with existing project files.
- Before changing an interaction, identify its command, UI callers, undo
  behaviour and existing tests.

## Architecture change discipline

Do not introduce a second editing engine, service, database, orchestration layer
or agent hierarchy. Significant, hard-to-reverse changes belong in an ADR under
`docs/adr/` with measured evidence.

## Related

- [verification.md](verification.md)
- [permissions.md](permissions.md)
- [reference/roadmap.md](../reference/roadmap.md)
