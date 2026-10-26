# Agent Authority — Current Local Mission

Last reviewed: 2026-10-26. Source: the owner's direct mission request. This
records a task permission boundary; it grants no system privilege and does not
replace OS or session security controls.

## Authorized

- Work in the existing local checkout and current branch. Inspect repository state
  and other worktrees, but preserve them and all unrelated user data.
- Build and launch the source-matched macOS application, exercise its real UI,
  inspect project state and generated exports, and run focused local checks.
- Create isolated test projects, profiles and small fixtures for repeatable
  verification.
- Make coherent local commits after a change and its relevant checks are complete.
- Inspect public source and documentation for the open-source editor references
  named in the mission.

These permissions apply to the local macOS target; they do not establish hosted
or cross-platform acceptance.

## Out of scope

No push or sync, no pull requests, no remote CI, no publishing or deployment, no
GitHub settings changes, no contact with third parties. Do not discard, reset,
rebase, amend or clean user work or other worktrees. Do not overwrite existing
projects or application data. Do not install system tooling, change machine
security settings, use sudo, or accept platform licences as a workaround.

## Host snapshot

| Fact | Value |
|---|---|
| Machine | MacBook Air, Mac17,3, Apple M5, 10 cores, 16 GB memory |
| OS | macOS 27.0.1, build 26A434 |
| Python | system 3.9.6; Homebrew 3.14.6 available for repository checks |
| Checkout | existing local repository, branch `codex/local-macos-dev` |

Tool versions, free disk/memory and FFmpeg capabilities change; recheck before a
build or measurement. Do not reuse dated facts from older records as facts about
this Mac.

## Related

- [git.md](git.md)
- [local-macos.md](local-macos.md)
