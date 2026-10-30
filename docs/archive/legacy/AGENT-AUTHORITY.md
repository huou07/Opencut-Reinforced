# Agent Authority — Current Mac Mission

Last reviewed: 2026-10-21. Source: the owner's direct mission request in this
session. This file records the task-specific permission boundary; it grants no
system privilege and does not replace OS or session security controls.

## Authorized for this mission

- Work in the existing local checkout and current branch. Inspect repository
  state and other worktrees, but preserve them and all unrelated user data.
- Build and launch the source-matched macOS application, exercise its real UI,
  inspect project state and generated exports, and run focused local checks.
- Create isolated test projects, profiles and small fixtures for repeatable
  verification.
- Make coherent local commits after the change and its relevant checks are
  complete.
- Inspect public source and documentation for the open-source editor references
  named in the mission.

These permissions apply to the local macOS target. They do not establish hosted
or cross-platform acceptance.

## Explicitly out of scope

Do not push or sync Git, open pull requests, trigger remote CI, publish releases,
deploy, alter GitHub settings, or contact third parties. Do not discard, reset,
rebase, amend or clean user work or other worktrees. Do not overwrite existing
projects or application data. Do not install system tooling, change machine
security settings, use sudo, or accept platform licenses as a workaround.

## Verified host snapshot

| Fact | Value |
|---|---|
| Machine | MacBook Air, Mac17,3, Apple M5, 10 cores, 16 GB memory |
| OS | macOS 27.0.1, build 26A434 |
| Python | system Python 3.9.6; installed Homebrew Python 3.14.6 is available for repository checks |
| Checkout | Existing local repository, branch codex/local-macos-dev at the start of this mission |

Tool versions, available disk and memory headroom, FFmpeg codecs, and build
artifacts can change. Recheck them before a measurement or build; do not reuse
the dated Debian/Linux/Android facts in older records as facts about this Mac.

## Evidence

A real local run establishes behavior only for the tested source, build, host,
profile and journey. Record those details and state what remains untested.
Remote CI, publication and cross-platform claims are not authorized in this
mission.
