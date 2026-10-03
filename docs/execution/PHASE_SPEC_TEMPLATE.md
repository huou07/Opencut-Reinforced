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
- primary user journey through the real product boundary
- failure journey with explicit user-visible error or recovery behavior
- packaged runtime and clean-environment assumptions
- performance and bounded-resource expectations
- required acceptance evidence classes and the hosted proof path for each
- acceptance gate, including real UI and process relaunch where feasible
- explicit out-of-scope items

Synthetic, unit, and bridge checks support but do not replace the declared
product journey. A lower-fidelity diagnostic harness is labelled as such.

## Acceptance floor

State the production boundary explicitly: the real user action, the shipped
artifact or runtime that must provide it, and the supported environment,
permission, persistence, and resource conditions. State the measurement floor
that must not be weakened: the required tests/cases, evidence classes, packaged
dependency and permission boundaries, supported scope, and any numeric bound.

State the performance applicability decision. If the checkpoint touches a
realtime, media, render, input, export, or persistence path, declare measurable
budgets, the comparison baseline, the fixture and measurement method, and the
supported workload/device matrix. `N/A` requires an explicit trusted rationale;
it may not waive a realtime obligation. Unit, synthetic, and bridge checks
support but never replace the declared product and negative journeys.

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
