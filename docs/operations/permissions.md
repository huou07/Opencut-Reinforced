# Agent Authority — Current Local Mission

Last reviewed: 2026-11-07. Source: the owner's direct mission request. This
records a task permission boundary; it grants no system privilege and does not
replace OS or session security controls.

The 2026-11-07 mission supersedes the 2026-11-02 release permission: work is
local only. Nothing is pushed, no pull request, tag, GitHub release or CI run is
created, and no remote branch is touched.

## Authorized

- Work in the existing local checkout and current branch. Inspect repository state
  and other worktrees, but preserve them and all unrelated user data.
- Build and launch the source-matched macOS application, exercise its real UI,
  inspect project state and generated exports, and run focused local checks.
- Create isolated test projects, profiles and small fixtures for repeatable
  verification.
- Make coherent local commits after a change and its relevant checks are complete.
- Build local release artifacts and install them on the three mission hosts: a
  macOS DMG built and signed locally (ad-hoc, not notarized), a Windows installer
  and portable archive, and a Linux `.deb`. These are local files; nothing is
  published.
- Run the local lanes and packaging scripts, and install the resulting packages
  on each host as the user's own account. Installing a native build tool or a
  language runtime on the mission hosts is part of this task; the mission hosts
  are dedicated to it.
- Inspect public source and documentation for the open-source editor references
  named in the mission.

These permissions apply to the local macOS target; they do not establish hosted
or cross-platform acceptance.

## Out of scope

No push, pull request, tag, release, CI run or any other remote action, and no
GitHub settings change. No contact with third parties. Do not discard, reset,
rebase, amend or clean user work or other worktrees. Do not overwrite existing
projects or application data, or replace a user's installed app before its
project data is safe. Do not change machine security settings or accept platform
licences as a workaround.

## Host snapshot

| Fact | Value |
|---|---|
| Machine | MacBook Air, Mac17,3, Apple M5, 10 cores, 16 GB memory |
| OS | macOS 27.0.1, build 26A434 |
| Python | system 3.9.6; Homebrew 3.14.6 available for repository checks |
| Checkout | existing local repository, branch `main` |

Tool versions, free disk/memory and FFmpeg capabilities change; recheck before a
build or measurement. Do not reuse dated facts from older records as facts about
this Mac.

## Related

- [git.md](git.md)
- [local-macos.md](local-macos.md)
