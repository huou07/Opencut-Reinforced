---
name: or-dev-loop
description: Build and verify OR on the owner-authorized local macOS machine, including focused checks, real app journeys, semantic UI driving and performance measurements. Use for local verification. Do NOT use to design a feature (or-edit-feature), review a UI (or-ui-ux), record hosted claims (or-claim-evidence), or publish (or-release).
---

# OR local macOS loop

Use the cheapest check that answers the question, then exercise the real app for
user-visible claims. Commands and host traps: `docs/operations/local-macos.md`.
Evidence classes and reporting: `docs/operations/verification.md`.

## Checks

```sh
bash scripts/check-repo.sh
cargo test -p <crate>                     # focused first
cd apps/or_app && flutter analyze --no-pub && flutter test --no-pub
```

## Drive the interface by semantics

`python3 scripts/or_ui.py` drives the real desktop app by widget key, semantics
label, visible text or role — never by screen coordinate — and writes a
step transcript plus screenshots and semantics snapshots. Its default target on
this Mac is `macos` (a debug build through the Flutter test binding).

```sh
python3 scripts/or_ui.py screens
python3 scripts/or_ui.py --viewport 1440x900 tree --name editor
python3 scripts/or_tree_outline.py <tree-*.json>
python3 scripts/or_ui.py run scripts/ui_scenarios/editor-journey.json
```

A single `act` relaunches the app; multi-step journeys belong in a scenario file
under `run`. Reach for a screenshot, MCP or vision only when semantics cannot
express the question. Details: `docs/operations/ui-harness.md`.

## Procedure

1. Record branch, HEAD, dirty state, and the source/build being checked. Use a
   separate test profile.
2. Recheck Flutter/Rust/Xcode/disk/FFmpeg availability; do not reuse old host
   facts.
3. Run the affected checks; use the repository gate only for a coherent increment.
4. Build the macOS app, copy it to a fresh ignored path with its own bundle id,
   stage the linked FFmpeg runtime (`scripts/package_local_macos_app.sh`), then
   launch the copy. Never rebuild or stage a bundle that is running.
5. Perform the affected interaction and verify canonical project state,
   save/reopen and output. For export, probe, decode and play the artifact.
6. Record revision, build mode, machine, fixture, backend, measurements, output
   path and limits; commit verified coherent work locally.

## Evidence

Classify source tests, semantics, real behaviour, visual inspection, native build
and performance separately. A local result proves only the tested source and
workflow on this Mac. The harness's debug-mode screenshots and timings are not
packaged or release claims.

Do not push, open PRs, trigger remote CI, publish or deploy; do not install
system toolchains or change machine security settings as a build workaround.
