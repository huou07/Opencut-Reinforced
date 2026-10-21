# Repository Policy

Last reviewed: 2026-10-21

This document holds OR's durable implementation, data-protection and evidence
rules. [AGENTS.md](../AGENTS.md) is the short entry point;
[docs/INDEX.md](INDEX.md) routes task-specific authorities. Host/session rules
and the owner's direct instructions take precedence. The current mission scope
and local permissions are recorded in [AGENT-AUTHORITY.md](AGENT-AUTHORITY.md).

## Product and architecture

Implement only the current owner-selected work in
[PRODUCT_ROADMAP.md](PRODUCT_ROADMAP.md). Architectural direction is not an
implemented capability or permission to scaffold future systems.

- Rust and the shared application/command path own canonical project state.
  Flutter and other clients may keep presentation state and bounded read-model
  caches, but never a second editable project.
- Mutations validate identity and expected revision, update history, and preserve
  explicit save, recovery and exact-base conflict behavior. Queries and playback
  do not mutate a project or increment its revision.
- Preserve the existing project format and its compatibility unless a scoped,
  reviewed change requires otherwise. Never edit project files behind the
  application path to simulate a user journey.
- MotionScene is declarative, bounded and non-executable. Evaluation,
  materialisation and rendering require no AI provider or network access.
- Keep domain logic out of presentation code. Extend existing commands and
  components before adding abstractions or dependencies.

## User data, security and licensing

Preserve existing projects, user data, branches, worktrees and unrelated edits.
Use a separate test profile and generated or lawfully reusable small fixtures for
repeatable runs. Do not remove caches or build products unless they are verified
disposable outputs created by this task.

Never put credentials in tracked files, fixtures, logs, screenshots or CLI
output. Keep generated media, models, caches and build outputs out of tracked
source. Before adopting a dependency or asset, inspect its source, maintenance,
integration cost and licence. Do not add GPL/AGPL or other distribution-restrictive
material to this MIT product without explicit owner approval and a recorded
licensing decision.

## Implementation and repair

Before editing, establish the current Git/worktree state, read the owning product
and architecture references, inspect actual source, and identify affected callers
and regression checks. Use CodeGraph first for indexed code. Keep changes small,
complete and compatible with the existing command, project, undo and recovery
contracts.

For user-visible behavior, exercise the real macOS application and verify the
resulting project/output. A widget test, CLI call, thumbnail, screenshot or
playback counter alone cannot prove the full journey. For media output, inspect
it independently with suitable media tools. For performance claims, record the
workload, source/build, codec/backend, machine and measured result.

After two speculative repairs for one failure, stop editing and state the
observed result, expected result, evidence, attempts and a falsifiable hypothesis.
Run a check that distinguishes the hypothesis before another repair.

## Verification and reporting

Use the cheapest check that answers the question, then exercise the affected
real-app path for user-visible work. The current mission authorizes local macOS
builds and runtime dogfooding; see [AGENT-AUTHORITY.md](AGENT-AUTHORITY.md) and
[or-dev-loop](../.agents/skills/or-dev-loop/SKILL.md). Do not run unrelated
platform matrices or remote CI.

Report each claim at its actual evidence level:

- **IMPLEMENTED** — source change exists.
- **VERIFIED_SOURCE** — relevant source checks or automated tests passed.
- **VERIFIED_BEHAVIORAL** — real app interaction and resulting canonical state
  were observed.
- **VERIFIED_VISUAL** — a named real-app view was inspected at its stated size and
  state.
- **VERIFIED_NATIVE** — a built macOS app was launched on this host; name whether
  it was a debug app or local package.
- **UNVERIFIED**, **BLOCKED**, **FAILED** and **NOT RUN** must remain explicit.
  No evidence class promotes another.

A local run for this mission is useful evidence for the MacBook Air, not hosted
cross-platform acceptance. Hosted claim records under
[docs/evidence/claims/](evidence/claims/README.md) remain a separate future
workflow; never fabricate or edit one to represent local evidence.

## Git and external actions

Make one local commit for each coherent, verified increment. Do not amend or
rewrite existing history, discard unrelated changes, or clean other worktrees.
For the current owner-authorized mission, do not push, open pull requests, trigger
remote CI, publish releases or deploy. Do not modify protected workflows or
historical evidence unless the owner explicitly scopes that change.

## Documentation

Update the existing source of truth when behavior, scope or a durable contract
changes. Keep [AGENTS.md](../AGENTS.md) as a concise map; put details in the owning
document, not repeated copies. Do not create a document or framework layer just
to record an ordinary implementation detail.
