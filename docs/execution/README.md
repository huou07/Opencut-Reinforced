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

Plan IDs remain the roadmap traceability inventory. `next_checkpoint_relation`
preserves its display order; `technical_dependency_checkpoint_ids` records the
actual correctness dependencies used by the supervisor. Historical
`prerequisite_checkpoint_ids` do not by themselves block independent product
work.

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
implementation commit. It cannot edit the plan, state, phase contracts,
validators, supervisor, workflows, or completion evidence. It may only add
named evidence proof bindings for selected requirements while preserving all
existing policy fields and bindings; the supervisor validates each addition.
Historical resume also checks that any candidate-carried additive evidence
hook leaves existing authorization and verification logic unchanged.
Changes to `scripts/test_execution_infra.py` are accepted on historical resume
only when AST validation proves that module-level guards, test classes, and all
existing test methods remain unchanged and the candidate only adds new test
methods. The exact candidate's hosted hygiene gate still executes those tests.

One explicitly authorized long-running product delegation may pin a trusted
control baseline in `DELEGATION.json`. The supervisor accepts that baseline
only for the exact recorded goal and while the current STATE baseline remains
before the delegation marker. A candidate before the marker must descend from
the pinned control baseline; a candidate after the marker uses the marker
commit itself. Exact SHA ancestry is checked, the marker is protected from
runner edits, and STATE advancement disables the override. This preserves one
delegation across coherent requirement batches without making plan, evidence,
architecture, acceptance, or verification controls mutable.

The supervisor is the only authority that can mark requirements `DONE`. A
single coherent implementation may cover multiple plan IDs. After a runner
pushes, the supervisor independently verifies exact-SHA push-triggered GitHub
Actions runs and every required job for each selected ID, verifies required
Developer Previews, writes one evidence record per ID, and creates one
state/evidence completion commit. `NEXT` is a progress cursor, not a mandatory
implementation boundary. A successful model message is never
repository-authoritative completion evidence.

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

ADR 0008 adds product acceptance invariants, evidence classes, and the 9B1
packaged-runtime hardening gate. `AMENDMENT_BASELINE.json` records the guarded
in-flight 9B contract reset; it is not 9B completion evidence. Evidence
classes and successful exact-SHA job-step proofs are enforced from 9B onward.

The approved future motion direction is recorded in ADR 0007 and the Phase 11,
14, and 16 contracts: canonical MotionScene is declarative, exact-time,
seekable, AI-optional, and materialization-first; arbitrary web motion is only
the later isolated WebMotion sidecar gate. These documents do not imply that
MotionScene, rendering, a browser runtime, or a live MotionClip exists.
