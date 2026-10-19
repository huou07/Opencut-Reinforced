---
name: or-edit-feature
description: Add or extend a basic OR editing capability — timeline, clips, media, markers, text, captions, undo, project lifecycle — through the shared command path so the desktop and compact UIs, the CLI and agents stay one model. Use when implementing or changing editing behavior, a command or query, or its GUI/CLI surface. Do NOT use for generic planning (task-planning), diagnosing a failing gate (verify-debug), AI/provider or media-codec work (or-media-ai), or local lane mechanics (or-dev-loop).
---

# OR editing feature

You add one editing capability to Opencut Reinforced so that every client of the
same project state sees the same behavior. You do not invent a second editable
model, and you do not put domain logic in Flutter.

## Preconditions

- Read [AGENTS.md](../../../AGENTS.md), then the [PRODUCT.md](../../../docs/PRODUCT.md)
  section that owns the capability, [ARCHITECTURE.md](../../../docs/ARCHITECTURE.md) and
  [DEVELOPMENT_WORKFLOW.md](../../../docs/DEVELOPMENT_WORKFLOW.md). The feature path in
  that last document is the procedure; this skill only fixes OR-specific traps.
- The capability must already be selected by
  [PRODUCT_ROADMAP.md](../../../docs/PRODUCT_ROADMAP.md). A roadmap label is traceability,
  not permission to expand scope.
- Confirm branch, HEAD and a clean-or-explained tree before editing.

## Invariants that break a review if violated

- Only the command/application path mutates canonical project state. Flutter,
  CLI, workers, renderers and agents never write it directly.
- Every mutation carries a matching `ProjectId`, `ProjectInstanceId` and
  expected `ProjectRevision`; a stale request is rejected for re-query, never
  silently applied. Undo/redo are canonical mutations and increment the revision.
- Queries are read-only and increment nothing. Playback ticks, decode, audio and
  presentation are runtime activity: they must not create transactions or bump
  `ProjectRevision`.
- Flutter keeps presentation-local state and bounded read-model caches only. If a
  design creates a second editable project in Dart, stop and redesign.
- `scripts/check_or_core_deps.py` enforces the canonical-state dependency
  boundary. A failure there is a design decision, never a widened pattern list.
- Motion content stays declarative and non-executable. Do not add JavaScript,
  shell, network or provider calls to canonical project data.

## Procedure

1. State the user-visible problem and the observable result. Classify the work
   (domain, media, render, audio, AI, UI-only, infrastructure).
2. Inspect the existing implementation before designing a replacement, and
   prefer extending an existing command over adding an abstraction.
3. Define the affected surfaces and record a reason for each omission: Rust
   command and query, CLI parity, desktop GUI, compact/mobile GUI, agent
   capability, docs, tests.
4. Define the command contract: command id, inputs, schema version, validation,
   revision preconditions, errors, `ChangeSet`, permissions, undo behavior.
5. Implement domain behavior first, then tests at the command/domain layer
   before relying on the UI.
6. Integrate the UI through an existing shell slot (App Bar, Tool Rail, Left Tool
   Panel, Viewer, Inspector, Timeline Toolbar, Timeline, Status Area, Command
   Palette, Dialog/Mobile Sheet). Read
   [DESIGN.md](../../../DESIGN.md) and check the feature descriptor and command
   registry first.
7. Adapt the compact presentation with the same command and state model — sheets,
   touch targets and layout change, semantics do not. Desktop and mobile share
   one canonical meaning; only presentation differs.
8. Keep the semantics tree honest: a control an agent must drive exposes a
   semantics label/role. Do not wrap everything, and never add a wrapper to force
   a test to pass.
9. Add or update the permanent UX regression guard when a UI defect is fixed. A
   regression fix is incomplete until a guard covers it.
10. Update the existing source of truth — usually PRODUCT.md or the roadmap —
    rather than creating a new document.
11. Verify with the local ladder in skill `or-dev-loop`, then push and inspect
    the required hosted jobs. Hosted CI is canonical for native and packaged
    behavior.
12. Commit one coherent change with a Conventional Commit subject, and report
    what changed, which checks ran, the commit, and what remains unverified.

## Report

Name the capability, the command id, the surfaces you changed, the guard you
added, the exact commands and results, the commit hash, and every surface or lane
you did not verify.
