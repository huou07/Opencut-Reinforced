# Development Workflow

Last reviewed: 2026-10-21

This is the short path for a scoped OR change. The active outcome is in
[PRODUCT_ROADMAP.md](PRODUCT_ROADMAP.md); read only the contract that owns the
work. Historical phases and old host reports are not a current work selector.

## Work loop

1. **Inspect the checkout.** Record branch, HEAD, dirty files and worktrees.
   Preserve unrelated work and user data.
2. **Pick the blocker.** Use the active roadmap and the user's requested journey.
   Read the relevant sections of PRODUCT, ARCHITECTURE, TECHNICAL_PLAN,
   PROJECT_FORMAT, DESIGN, UX_ACCEPTANCE or QUALITY as applicable.
3. **Inspect the real implementation.** Use CodeGraph before searching or
   reading indexed source. Trace the callers and reuse the existing command,
   gateway, media and render paths.
4. **Implement the smallest complete increment.** Keep project mutations in the
   Rust-owned validated command path, preserve revisions/undo/recovery, and add
   the smallest useful regression guard.
5. **Exercise the affected product path.** Build and run the real macOS app for
   user-visible work. Use an isolated test profile. Observe both the interaction
   and resulting project state; inspect visual behavior where the change is
   visible. For export, inspect and decode the output independently.
6. **Record and commit.** Report exact checks, runtime build, source revision,
   artifact and any unverified states. Commit a coherent, verified increment
   locally. Do not push or trigger remote CI during the current mission.

Do not use direct project-file edits, hidden commands or synthetic widget tests
as substitutes for a normal UI journey. They may support diagnosis. A screenshot
establishes visual evidence only; a test establishes only the behavior it runs.

## Editing state

All user-facing editing operations must use the shared validated application
command path. Rust owns canonical project state and history. Flutter owns only
presentation and bounded read models. Queries and playback are runtime activity,
not project mutations. Preserve exact timeline time, revision checks, explicit
save, recovery and compatibility with existing project files.

Before changing an interaction, identify its command, UI callers, undo behavior,
and existing tests. Keep timeline drag/trim/split/reorder intuitive and use
contextual pointer and keyboard controls where available.

## Media, rendering and MotionScene

Media import, preview and export use the existing OR/FFmpeg path. Inspect the
actual packaged/helper capabilities before claiming a codec or acceleration
path. Keep decode, audio and render work bounded; measure a demonstrated
bottleneck before changing architecture.

MotionScene is a focused deterministic feature. It uses declarative validated
scene data, the existing render path and normal media/timeline commands.
Rendering, preview and materialisation make no provider or network calls. See
[ADR 0007](adr/0007-declarative-motion-scenes-and-procedural-isolation.md).

## Verification

Choose the cheapest check that answers the question, then run the real local
journey needed for the claim. The current host's permitted build and verification
procedure is in [TOOLING.md](TOOLING.md) and skill
[or-dev-loop](../.agents/skills/or-dev-loop/SKILL.md). Do not run unrelated
platform matrices. See [TESTING.md](TESTING.md) for test inventory and
[QUALITY.md](QUALITY.md) for existing measured budgets and their limits.

Classify results honestly: IMPLEMENTED, VERIFIED_SOURCE,
VERIFIED_BEHAVIORAL, VERIFIED_VISUAL, VERIFIED_NATIVE, FAILED, BLOCKED,
UNVERIFIED or NOT RUN. Local Mac evidence is not a hosted cross-platform claim.

## Boundaries for larger changes

Do not introduce a second editing engine, service, database, orchestration layer
or agent hierarchy without measured evidence and explicit scope. Significant,
hard-to-reverse changes belong in an ADR. Preserve historical receipts; do not
edit them to make current implementation appear verified.
