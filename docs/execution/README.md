# OR Execution Lock

This directory is the repository's machine-readable execution control plane.
It makes the approved architecture, checkpoint order, and current execution
state inspectable without a bespoke prompt.

## Authority

The authority order is:

1. `AGENTS.md` — permanent safety, repository, and execution rules.
2. `PLAN.json` — immutable checkpoint graph and milestone definitions.
3. `STATE.json` — mutable `DONE`, `NEXT`, and `PLANNED` state.
4. `phases/*.md` — locked implementation contracts.
5. `docs/ARCHITECTURE.md` — human architecture source of truth.
6. `docs/TECHNICAL_PLAN.md` — subsystem design detail.
7. `docs/PRODUCT.md` — capability scope.
8. `docs/ROADMAP.md` — human roadmap and status.
9. `DESIGN.md` and `docs/UX_ACCEPTANCE.md` — presentation and UX truth.

`PLAN.json` never contains mutable completion state. `STATE.json` never adds
checkpoints that are absent from the plan.

## Commands

```sh
python3 scripts/execution_plan.py status
python3 scripts/execution_plan.py next
python3 scripts/execution_plan.py context <checkpoint-id>
python3 scripts/execution_plan.py goal milestone:desktop-mvp
python3 scripts/execution_plan.py check
python3 scripts/check_execution_plan.py
python3 scripts/check_architecture_policy.py
python3 scripts/test_execution_infra.py
```

The optional `scripts/agent_supervisor.py` runs one fresh external runner
process per valid checkpoint and refuses dirty or diverged direct-main state.
It does not know or assume a vendor agent CLI. A runner may push only its
implementation commit. It cannot edit the plan, state, evidence policy,
phase contracts, validators, supervisor, workflows, or completion evidence.

The supervisor is the only authority that can turn `NEXT` into `DONE`. After a
runner pushes, it independently verifies exact-SHA push-triggered GitHub
Actions runs and every required job, verifies a Developer Preview when the
selected checkpoint requires one, writes the supervisor-owned evidence record,
and then creates the state/evidence completion commit. A successful model
message is never repository-authoritative completion evidence.

Evidence enforcement begins at the checkpoint named by
`EVIDENCE_POLICY.json`. Historical checkpoints before that boundary remain
valid without fabricated evidence. See [evidence/README.md](evidence/README.md)
for the record contract.

## Current state

`STATE.json` is the only mutable execution-status source. `PLAN.json` supplies
the immutable checkpoint graph; documentation must not copy the current
checkpoint or phase status.

An architecture/plan amendment does not advance product state, start a product
checkpoint, publish a Developer Preview, or create future runtime crates.

The approved future motion direction is recorded in ADR 0007 and the Phase 11,
14, and 16 contracts: canonical MotionScene is declarative, exact-time,
seekable, AI-optional, and materialization-first; arbitrary web motion is only
the later isolated WebMotion sidecar gate. These documents do not imply that
MotionScene, rendering, a browser runtime, or a live MotionClip exists.
