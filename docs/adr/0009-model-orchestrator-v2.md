# ADR 0009 — Model orchestrator V2

- Status: Frozen candidate; adoption requires the separate control-plane amendment
- Date: 2026-10-03
- Decision: CLEAN REIMPLEMENTATION
- Trusted design base: `915a4a8b951643e475683e4cdf56996118ec7d1e`
- Audited prototype: `19c94f9677e0ff55ef37fb88b31824435d28e514`

## Problem

The prototype can reach promotion readiness without executing a declared
failing test. Its worker shares host write authority with its runtime records,
authority freezing covers only part of the authorization surface, and its
promotion recovery conflates remote publication with local synchronization.
These defects cross component boundaries; passing helper tests cannot establish
that the live system preserves authorization or product quality.

## Decision

Reimplement from authoritative main. Do not merge or cherry-pick the three V1
implementation commits. Preserve their history as an audit reference. Retain
the existing execution plan, evidence classes, exact-SHA hosted verifier,
supervisor completion ownership, and useful test scenarios. Reimplement the
orchestration lifecycle around an immutable authority release, controller-owned
records, isolated candidate clones, independently executed tests, and durable
promotion intents. No product state or product code changes are authorized by
this ADR.

Optimize expected total cost to reliable product acceptance. Models propose
code, classifications, findings, or decisions; they cannot issue authorization
or completion evidence. Architecture reasoning is an exceptional escalation,
never a build, formatting, polling, or implementation worker.

## Frozen contracts

The normative candidate specification is [automation/README.md](../execution/automation/README.md).
It includes the trust diagram, state machine, task and evidence contracts,
locks, pause behavior, model policy, promotion, and recovery. The accompanying
[audit](../execution/automation/V1_AUDIT.md),
[implementation and acceptance plan](../execution/automation/IMPLEMENTATION_PLAN.md),
and [amendment proposal](../execution/automation/AMENDMENT_PROPOSAL.md) are part
of the same freeze. [V2_CONTRACT.json](../execution/automation/V2_CONTRACT.json)
indexes the candidate's machine contracts; it is not active policy in V1.

## Consequences and limits

The simplest reliable worker boundary is a disposable Linux container with an
independent candidate Git directory, no controller mounts or credentials,
immutable launch configuration, and explicit resource limits. An existing
verified container/VM runtime is an operator prerequisite; missing isolation
stops dispatch. Native OR runtime verification stays on GitHub Actions.

The existing supervisor remains the completion authority. Its task-bound
entrypoint and stronger acceptance receipts require a narrowly reviewed
control-plane amendment before activation. This candidate does not silently
change permanent invariants, locked phase contracts, PLAN, STATE, workflows,
amendment provenance, or evidence. Product-value and no-satisficing additions
are proposed verbatim in the amendment document.

V2 is not implemented or production-verified by this architecture commit.
Runtime certification, model enrollment, and checkpoint-specific measurement
bounds remain activation gates with explicit fail-closed behavior, not decisions
delegated to implementation workers. Changing a frozen design decision requires
a separate architecture task.

## M0-R2 corrective decision — 2026-10-03

Separate design authority, scoped disabled-build authorization and externally
pinned operational adoption. Implement/certify M0–M5 while disabled, then adopt
the exact certified release after independent review. README §19 freezes the
corrected lifecycle, full-tree Git authority, recursive task schemas and bound
receipt semantics. Original architecture and invalid M0 history stay intact.
This correction authorizes no M1 or product execution and is itself a candidate.
