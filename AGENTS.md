# Opencut Reinforced — Agent Guide

Last reviewed: 2026-10-18

OR is an MIT-licensed cross-platform video editor for human and structured
CLI/agent use. Architectural directions are not implemented capabilities. Read
only the task-relevant references; this file is an operating map, and
[docs/INDEX.md](docs/INDEX.md) is the full documentation router.

## Product focus

The owner's active priorities, in dependency order:

1. **Complete basic editing** — timeline, tracks, clips, move/trim/split/ripple,
   undo/redo, zoom/scroll, snapping, markers, clip properties, save/reopen and
   recovery, usable by pointer, keyboard and touch.
2. **Code-to-illustration video** — turn a code artifact into a declarative
   MotionScene (highlighted code, callouts, captions, narration) and render it
   deterministically. An external generative-video provider is a later, optional
   second track.
3. **Automatic subtitles** — local speech-to-text into reviewable caption
   proposals, editable and exportable to SRT/WebVTT.
4. **Noise filtering and audio cleanup** — denoise, voice enhancement and
   loudness work in the audio layer.

Desktop (macOS, Linux, Windows) must present one coherent product with the same
concepts, terminology, layout intent and command semantics; input and layout
adapt to platform and screen size. Android shares the same model with adapted
presentation. Framing, gaps and open decisions are in
[docs/PRODUCT_ROADMAP.md](docs/PRODUCT_ROADMAP.md); this list selects work and
does not restate it.

## Non-negotiables

The permanent execution, acceptance, security and dependency rules are in
[docs/AGENT_POLICY.md](docs/AGENT_POLICY.md). The short version:

- UI, CLI and agents mutate canonical Rust project state through the shared
  command/application path. Keep domain logic outside presentation, and never
  create a second editable project state in Dart, CLI code or a worker.
- Canonical motion is declarative and non-executable; rendering makes no
  provider call. Do not scaffold unrequested architecture.
- AI output is a reviewable proposal applied through a revision-checked command,
  never an instruction. Credentials stay in OS secure storage and never reach the
  CLI, agents, logs or fixtures.
- Keep protected specifications, invariants, workflows and claims intact unless
  the owner explicitly authorizes their control-plane change.
- `scripts/check_or_core_deps.py` enforces the canonical-state dependency
  boundary. A failure needs a decision, never a widened pattern list.
- Keep credentials, models, caches and generated media out of tracked source.
  Preserve licences and lawful small fixtures.

## Work loop

1. Establish branch, HEAD, worktrees and dirty changes; preserve unrelated work.
2. Read the active roadmap and the authority that owns the capability. Reuse
   existing components, command paths and tests before adding abstractions.
3. Define the observable outcome and the evidence class it will need.
4. Implement the smallest correct change and its regression guard.
5. Verify through the ladder below, commit one coherent change, and report
   honestly.

After two speculative repairs for the same check, record observed/expected/
evidence/attempts, a falsifiable hypothesis and a discriminating next check
before another edit. Never discard WIP, amend shared history, force-push, merge
protected branches, or infer publication authority from a green gate.

## Build, test and release

Local work is headless by default. Exactly two bounded local runtime lanes are
owner-granted ([docs/AGENT-AUTHORITY.md](docs/AGENT-AUTHORITY.md)):

```sh
bash scripts/run_local_lane.sh linux      # hygiene, Flutter suite, preflight, bench
bash scripts/run_local_lane.sh linux --packaged   # adds the local .deb and journey
bash scripts/run_local_lane.sh android    # local emulator lane: apk, emulator, preview, saf
```

`bash scripts/check-repo.sh` is the source gate and `bash scripts/acceptance.sh`
adds the Rust and Flutter lanes, `gitleaks` and the local performance budget.
`scripts/acceptance.sh` writes the machine-readable report
`.hub/gate-report.json`; `scripts/check-repo.sh` is read-only and writes no
report. **GitHub Actions is the only
authority for native, packaged and cross-platform acceptance**; a local preflight
or emulator result is developer evidence and never becomes `ACCEPTED`. Release,
packaging and distribution follow [docs/RELEASE.md](docs/RELEASE.md) and are
owner-gated. Mechanics are in [docs/TOOLING.md](docs/TOOLING.md) and
[docs/TESTING.md](docs/TESTING.md); the working procedure is skill `or-dev-loop`.

## Evidence and reporting

Report `IMPLEMENTED`, each verified evidence class, `ACCEPTED`, and
`BLOCKED`/`UNVERIFIED`/`NOT RUN` separately. A skipped or unavailable check is
never `PASS`. A completion claim is a record under
[docs/evidence/claims/](docs/evidence/claims/README.md) that binds the exact
implementation SHA to the hosted run and job that verified it; an implementation
statement never establishes DONE. A Linux debug app with a fixture picker is not
an installed-package or native OS-picker claim.

## Document routing

| Need | Authority |
|---|---|
| Full documentation router, authority order, current work selection | [docs/INDEX.md](docs/INDEX.md) |
| Permanent execution, acceptance, security, dependency rules | [docs/AGENT_POLICY.md](docs/AGENT_POLICY.md) |
| What to do now, priorities, traceability | [docs/PRODUCT_ROADMAP.md](docs/PRODUCT_ROADMAP.md) |
| What this host authorizes without asking | [docs/AGENT-AUTHORITY.md](docs/AGENT-AUTHORITY.md) |
| Required product capabilities and maturity labels | [docs/PRODUCT.md](docs/PRODUCT.md) |
| Architecture, subsystem contracts, `.orproj` format, decisions | [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md), [docs/TECHNICAL_PLAN.md](docs/TECHNICAL_PLAN.md), [docs/PROJECT_FORMAT.md](docs/PROJECT_FORMAT.md), [ADRs](docs/adr/README.md) |
| Feature path from definition to merged pull request | [docs/DEVELOPMENT_WORKFLOW.md](docs/DEVELOPMENT_WORKFLOW.md) |
| Quality bar, evidence classes, gates and known gaps | [docs/QUALITY.md](docs/QUALITY.md), [docs/TESTING.md](docs/TESTING.md) |
| Local tooling, hosted verification, approved agent tools | [docs/TOOLING.md](docs/TOOLING.md) |
| UI conventions and acceptance | [DESIGN.md](DESIGN.md), [docs/UX_ACCEPTANCE.md](docs/UX_ACCEPTANCE.md) |
| Reuse, upstream, dependency and licence decisions | [docs/OPEN_SOURCE_CONVERGENCE.md](docs/OPEN_SOURCE_CONVERGENCE.md), [docs/SECURITY_LICENSING.md](docs/SECURITY_LICENSING.md) |
| Release and installation | [docs/RELEASE.md](docs/RELEASE.md), [docs/DEVELOPER_PREVIEW_INSTALL.md](docs/DEVELOPER_PREVIEW_INSTALL.md) |
| Current work selection | [docs/prompts/OR-BUILD.md](docs/prompts/OR-BUILD.md), [docs/prompts/README.md](docs/prompts/README.md) |
| History and archived receipts | [docs/ROADMAP.md](docs/ROADMAP.md), [docs/PAUSE_MIGRATION_HANDOFF.md](docs/PAUSE_MIGRATION_HANDOFF.md), [docs/archive/execution/README.md](docs/archive/execution/README.md) |
| Contributing and vulnerability reporting | [CONTRIBUTING.md](CONTRIBUTING.md), [SECURITY.md](SECURITY.md), [README.md](README.md) |

## Skill routing

Project-local skills live in [.agents/skills/](.agents/skills/) and are catalogued
with their triggers and exclusions in [docs/skills.md](docs/skills.md):
`or-edit-feature`, `or-dev-loop`, `or-media-ai`, `or-claim-evidence`,
`or-release`. Load one only when its trigger matches; defer to the shared
procedures the current session exposes rather than copying one into this
repository.

Author framework artifacts in English and converse in the owner's language.
