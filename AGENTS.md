# Opencut Reinforced — Agent Guide

Last reviewed: 2026-10-18

OR is an MIT-licensed cross-platform video editor for human and structured
CLI/agent use. Architectural directions are not implemented capabilities.
Read only task-relevant references; this is an operating map.

| Need | Authority |
|---|---|
| Permanent execution/security/acceptance rules | [Repository policy](docs/AGENT_POLICY.md) |
| Active priorities and source/run traceability | [Product roadmap](docs/PRODUCT_ROADMAP.md) |
| Architecture and subsystem boundaries | [Architecture](docs/ARCHITECTURE.md), [technical plan](docs/TECHNICAL_PLAN.md) |
| Reuse and current product direction | [Convergence](docs/OPEN_SOURCE_CONVERGENCE.md), [product](docs/PRODUCT.md) |
| UI conventions and acceptance | [DESIGN](DESIGN.md), [UX acceptance](docs/UX_ACCEPTANCE.md) |
| Checks and runtime evidence | [Development](docs/DEVELOPMENT_WORKFLOW.md), [testing](docs/TESTING.md) |
| Platform tools | [Tooling](docs/TOOLING.md) |
| Licensing, dependencies and release | [Security/licensing](docs/SECURITY_LICENSING.md), [release](docs/RELEASE.md) |
| Navigation and workflow routing | [Docs index](docs/INDEX.md), [skills](docs/skills.md) |
| Exact-SHA authoritative claims | [Claim records](docs/evidence/claims/README.md) |

## Work

1. Establish branch, HEAD, worktrees and dirty changes; preserve unrelated work.
2. Use a valid CodeGraph index before broad exploration and confirm against source.
3. Define the observable outcome and evidence needed. Reuse existing components,
   command paths, libraries and tests; keep the change within the owner's task.
4. Run the cheapest relevant check, then the actual journey when its claim needs it.
   After two speculative repairs, record observed/expected/evidence/attempts,
   hypotheses and a discriminating next check before another edit.
5. Update existing task/evidence authorities; keep commits coherent. Never discard
   WIP, amend/rewrite shared history, force-push, merge protected branches or infer
   publication/release authority from a green gate.

The active roadmap selects ordinary continuation work. Archived NEXT/checkpoint
ledgers retain history and never reactivate the paused execution framework.

## Preserve the product boundaries

- UI, CLI and agents mutate canonical Rust project state through the shared
  command/application path. Keep domain logic outside presentation.
- Canonical motion is declarative/non-executable; procedural content needs its
  explicit future sandbox. Do not scaffold unrequested architecture.
- DESIGN and UX acceptance own Focused Monochrome, accessible controls, keyboard
  alternatives, feedback and recovery. Inspect actual semantics before adding
  wrappers. Semantic/component, behavior, visual and native evidence are distinct.
- The HTML prototype is a reference, not production architecture.
- Keep protected specifications, workflows, invariants and claims intact unless
  the owner explicitly authorizes their control-plane change.
- `scripts/check_or_core_deps.py` enforces the canonical-state dependency
  boundary. A dependency failure needs a decision, never a widened pattern list.
- Keep credentials, models, caches and generated media out of tracked source.
  Preserve licences and lawful small fixtures.

## Verify and hand off

`bash scripts/check-repo.sh` is the repository hygiene/document/claim gate;
`bash scripts/acceptance.sh` adds the documented Rust lanes, the Flutter suite
and the local performance budget. GitHub Actions remains canonical for
native/package/cross-platform acceptance. The owner permits two bounded local
runtime lanes on this host: the Debian Linux developer preflight via
`python3 scripts/run_linux_ui_preflight.py`, and, since 2026-10-18, the local
Android emulator lane granted in `docs/AGENT-AUTHORITY.md`. Both are developer
evidence with one heavy job, one compiler worker, fresh headroom checks, timeout
and process-tree cleanup; neither authorizes cross-platform packaging, a long
media render, a second writer in this tree, or sudo, privilege changes or
toolchain substitution.

Report IMPLEMENTED, verified evidence classes, ACCEPTED and BLOCKED/UNVERIFIED
separately. A Linux debug app with a fixture picker is not an installed-package
or native OS-picker claim. Existing claim records require exact source and
hosted run/job identity; an implementation statement never establishes DONE.
Author framework artifacts in English and converse in the owner's language.
