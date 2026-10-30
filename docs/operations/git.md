# Git and External Actions

Last reviewed: 2026-10-28

Scope: repository history rules and the actions that are out of bounds for the
current mission.

## Commits

- One local commit per coherent, verified increment.
- Commit each verified increment before reporting it: never leave verified work
  uncommitted when the turn ends.
- Do not amend, rebase or rewrite existing history.
- Do not discard unrelated changes, clean other worktrees, or reset user work.
- Do not commit generated media, caches, build output, secrets or model weights.
- Keep historical evidence records and protected workflows untouched unless the
owner explicitly scopes that change.

## Not authorized in this mission

No push, no pull request, no remote CI trigger, no release publication, no
deploy, no GitHub settings change. See [permissions.md](permissions.md).

## Local branches and worktrees

Work in the existing local checkout and current branch. Other worktrees and
branches are preserved, not cleaned or reset.

## Related

- [permissions.md](permissions.md)
- [verification.md](verification.md)
