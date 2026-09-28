# Phase Specification Template

Every locked phase document should use this shape.

## Status

State whether the phase is complete, active, or planned. Do not put mutable
checkpoint status in the plan document; that belongs in `STATE.json`.

## Scope

State what the phase may implement and what it must not implement.

## Checkpoints

For every checkpoint, record:

- checkpoint ID and goal
- prerequisites
- affected invariant IDs
- allowed product/schema/IPC/dependency categories
- required tests and documentation
- acceptance gate
- explicit out-of-scope items

## Stop conditions

List repository contradictions, missing prerequisites, dependency/license
uncertainty, or platform evidence that require a separate decision.

## Handoff

The final handoff names the checkpoint, files, checks, implementation commit,
push state, and remaining unverified items. It begins with
`IMPLEMENTED — AWAITING SUPERVISOR EVIDENCE`; it never claims repository
authoritative `DONE`. It reports the baseline, local checks, explicit
out-of-scope work, native-runtime policy result, and leaves the checkpoint
`NEXT` with its successor `PLANNED` until the supervisor verifies hosted
evidence. The next relation is taken from `PLAN.json`.

The supervisor owns exact-SHA hosted CI and Developer Preview verification,
completion evidence, the `NEXT -> DONE` transition, the state/evidence-only
commit, and the final verified report. Normal runners may not edit `PLAN.json`,
`STATE.json`, this evidence policy, phase specifications, execution
validators/supervisor, protected workflows, or `docs/execution/evidence/**`.
