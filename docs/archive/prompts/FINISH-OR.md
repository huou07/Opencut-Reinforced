# Finish OR

Owner authority for this work is recorded in [../AGENT-AUTHORITY.md](../AGENT-AUTHORITY.md):
you may commit, push to `origin`, open pull requests and publish a release once
the project's own gates pass. You do not need to ask for those again, and you
must not ask for host privileges that file does not grant.

## Objective

Drive OR to a state where every documented end-user journey is implemented and
carries evidence of its own class. Work from the active roadmap and the
unverified boundaries it names; do not invent a second roadmap.

## Start from the repository, not from this file

1. `AGENTS.md`, then only the sections you need of `docs/PRODUCT_ROADMAP.md`,
   `docs/TESTING.md`, `docs/UX_ACCEPTANCE.md`, `docs/TOOLING.md` and
   `docs/evidence/claims/`.
2. Establish the real state before editing: branch, HEAD, worktrees, dirty files.
   Preserve unrelated work; never discard it.
3. Create a persisted goal for this objective and continue across rounds until
   every listed journey is either accepted or explicitly reported as blocked.
   Do not stop after planning, after implementing, or after one green run.

## Lanes verified on this host

| Lane | Command | State |
|---|---|---|
| Hygiene | `bash scripts/check-repo.sh` | PASS |
| Rust | `cargo test --locked -p or_core -p or_cli`, then `-p or_app_bridge -p or_ipc -p or_runtime` | PASS |
| Linux UI journey | `python3 scripts/run_linux_ui_preflight.py` | PASS (developer evidence) |

Use the preflight rather than a raw `flutter test`: the bridge crate declares
macOS targets and rustup then tries to install them. The preflight narrows that
list to the host target for the run and restores it byte-for-byte.

## Constraints

- Resource-bounded work: keep the preflight's own limits, serialise heavy jobs,
  respect `MemAvailable`, and clean up whole process groups.
- Local passes are developer evidence. The authoritative gate is the hosted CI
  run for the exact commit SHA; Android, macOS and Windows evidence comes only
  from hosted jobs, never from this host.
- Preserve the design system (`DESIGN.md`), the accessibility contracts and the
  single shared Flutter UI with per-platform adapters. Do not add a
  platform-specific fork of the UI.
- Keep domain logic out of Flutter presentation, and keep CLI, UI and agents on
  the same canonical Rust project state.

## Sequence

1. Reproduce the next unverified journey from the roadmap with the real
   commands above and synthetic fixtures.
2. Fix the first gap you actually reproduce; add the narrow regression test it
   needs. Never invent dogfood observations or owner feedback.
3. Re-run the affected lanes, then the full local set.
4. Commit each coherent change, push, and open a pull request. Let CI decide,
   repair genuine failures, and merge through the normal workflow.
5. Continue to the next journey. Record what you verified, in which evidence
   class, and what remains unverified.
