# ADR 0006 — Autonomous agent execution contract

- Status: Accepted
- Date: 2026-09-28

## Context

Long-running autonomous work must not guess the current phase, silently skip
architecture gates, or let one model context chain unrelated checkpoints.

## Decision

Keep an immutable JSON checkpoint graph in `PLAN.json` and mutable completion
state in `STATE.json`. A supervisor accepts a checkpoint, phase, or milestone
goal, resolves only the current `NEXT` checkpoint, runs one fresh external
runner process, and verifies exactly one `NEXT -> DONE` transition against a
clean direct-main baseline. The runner is an absolute executable supplied by
the caller; the supervisor has no vendor CLI assumptions and supplies no
credentials.

## Alternatives rejected

- A prose-only phase status in `AGENTS.md`.
- Letting the model choose any later checkpoint after reading a roadmap.
- Chaining multiple checkpoints in one model process without a supervisor
  boundary.
- Hard-coding a particular agent product or CLI.

## Consequences

The plan/state files and validators are part of repository infrastructure.
Checkpoint completion requires clean Git state, exact baseline checks, docs,
tests, commit, and push evidence. A plan amendment is a separate architecture
decision and cannot be hidden inside feature work.
