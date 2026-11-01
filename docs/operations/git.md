# Git and External Actions

Last reviewed: 2026-11-02

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

## Authorized external actions

The owner's 2026-11-02 instruction authorizes one release path. The
`main-safety` ruleset requires the `release-gate` check on `main`, so the release
commit lands through a release branch and PR whose checks pass; then push a `v*`
tag and let `.github/workflows/macos-release.yml` build and publish the DMG,
checksum file and notes for that tag. No other pull request, deployment or GitHub
settings change. See [permissions.md](permissions.md).

## Local branches and worktrees

Work in the existing local checkout and current branch. Other worktrees and
branches are preserved, not cleaned or reset.

## Related

- [permissions.md](permissions.md)
- [verification.md](verification.md)
