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

The final handoff names the checkpoint, files, checks, commit, push state, and
remaining unverified items. The next relation is taken from `PLAN.json`.
