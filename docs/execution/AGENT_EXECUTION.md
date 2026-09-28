# Agent Execution Contract

This document defines how an autonomous execution model selects and completes
work. It is a protocol, not a product feature.

## Goals

The supervisor accepts exactly one of:

```text
checkpoint:<ID>
phase:<N>
milestone:desktop-mvp
milestone:full-roadmap
```

The machine plan resolves the goal. A weak agent never chooses an arbitrary
checkpoint.

## Required sequence

Before work:

1. Require `main`, a clean worktree, and `HEAD == origin/main`.
2. Read `PLAN.json`, `STATE.json`, and the selected locked phase document.
3. Resolve the current `NEXT` checkpoint for the requested goal.
4. Refuse a goal whose prerequisites or current `NEXT` state do not permit it.

During work:

1. Execute only that checkpoint.
2. Do not edit its locked specification or permanent invariants.
3. Keep product behavior, dependencies, and platform scope within the spec.
4. Add tests and documentation required by the repository rules.
5. Commit one coherent change and use the repository's approved push workflow.

After work:

1. The supervisor fetches `origin`.
2. The worktree must be clean.
3. Direct-main mode requires `HEAD == origin/main`.
4. Exactly one checkpoint may transition from `NEXT` to `DONE`.
5. The plan checker must pass and the next relation must become the new
   `NEXT`, unless the goal is complete.

Only after those checks may a fresh runner process begin the next checkpoint.
One model process never implements multiple checkpoints back-to-back without
the supervisor boundary.

## Prompt contract

The runner receives a generated checkpoint prompt on stdin and runs with the
repository root as its working directory. The runner is an absolute executable
path supplied by the caller. The supervisor does not know the runner's CLI,
does not construct shell command strings, and never supplies credentials.

Exit status zero means the runner claims completion; any nonzero status stops
the supervisor.

## Stop conditions

Stop and report the exact conflict if baseline, documentation, code behavior,
dependencies, project schema, recovery, IPC, or current `NEXT` state conflicts
with the selected checkpoint. Do not repair a specification creatively.
