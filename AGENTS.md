# Opencut Reinforced — Agent Guide

Last reviewed: 2026-10-21

OR is an MIT-licensed video editor. This file is a short operating map; use
[docs/INDEX.md](docs/INDEX.md) to find the authority for the task, and read only
its relevant sections.

## Active owner scope

For the current delivery mission, make the existing macOS application useful for
ordinary editing, then finish deterministic code-to-video MotionScene support.
The macOS app on this machine is the only required runtime target. Cross-platform
support remains a future objective. Defer transcription, cloud or generative AI,
community features, advanced effects, and unrelated framework work.

The required editing journey is project create, import and arrange video/image/
audio, pointer trim/move/split/reorder, speed/volume, snapping, undo/redo,
composition preview, save/reopen, and playable export. The MotionScene journey
uses source code to create, preview, render and edit a short deterministic scene.

## Hard boundaries

- Rust owns canonical project state. UI and CLI changes use the shared validated
  command path, revision checks, undo and recovery; Flutter is presentation and
  bounded read-model state.
- MotionScene data is declarative and non-executable. Preview and rendering use
  no provider or network service.
- Preserve existing projects, user data, branches, worktrees and unrelated
  changes. Do not clean or overwrite user data. Use isolated test profiles.
- Keep secrets, generated media, caches and build output out of tracked source.
  Check dependency maintenance and licence compatibility before adoption.
- The owner explicitly authorizes local macOS builds, application launches,
  real UI dogfooding, focused local checks and coherent commits for this mission.
  Do not push, open PRs, trigger remote CI, publish or deploy.

## Work loop

1. Recheck branch, HEAD, worktrees and dirty state; preserve anything unrelated.
2. Select the highest-value blocker in [PRODUCT_ROADMAP.md](docs/PRODUCT_ROADMAP.md)
   and read only the relevant product, architecture and UX references.
3. Use CodeGraph before searching or reading indexed source. Reuse the existing
   command, media and rendering paths; make the smallest complete change.
4. Add the relevant regression check. For user-visible work, build and exercise
   the actual macOS app with an isolated project and inspect the result.
5. Commit a coherent verified increment locally. Report evidence and gaps without
   promoting a build, test, screenshot or model statement into another class.

After two speculative fixes for the same failure, write down the observation and
run a discriminating check before editing again. Do not introduce a new
orchestrator, service, database, editing engine or second project state.

## Routing

- Task map and authority order: [docs/INDEX.md](docs/INDEX.md)
- Active scope and priorities: [docs/PRODUCT_ROADMAP.md](docs/PRODUCT_ROADMAP.md)
- Permanent invariants: [docs/AGENT_POLICY.md](docs/AGENT_POLICY.md)
- Local permissions and machine facts: [docs/AGENT-AUTHORITY.md](docs/AGENT-AUTHORITY.md)
- Editing path: [docs/DEVELOPMENT_WORKFLOW.md](docs/DEVELOPMENT_WORKFLOW.md)
- Design and UX: [DESIGN.md](DESIGN.md), [docs/UX_ACCEPTANCE.md](docs/UX_ACCEPTANCE.md)
- Source checks and platform procedures: [docs/TESTING.md](docs/TESTING.md), [docs/TOOLING.md](docs/TOOLING.md)
- OR-specific skill routing: [docs/skills.md](docs/skills.md)

Framework artifacts are written in English; conversation follows the owner's language.
