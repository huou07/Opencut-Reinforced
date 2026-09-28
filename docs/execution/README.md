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
python3 scripts/execution_plan.py context 6E2B
python3 scripts/execution_plan.py goal milestone:desktop-mvp
python3 scripts/execution_plan.py check
python3 scripts/check_execution_plan.py
python3 scripts/check_architecture_policy.py
```

The optional `scripts/agent_supervisor.py` runs one fresh external runner
process per valid checkpoint and refuses dirty or diverged direct-main state.
It does not know or assume a vendor agent CLI.

## Current state

Phase 5 is complete. Phase 6 is in progress through completed 6E2A, with 6E2B
as the only `NEXT` product checkpoint. All later checkpoints are planned.

This architecture checkpoint does not advance product state, implement marker
UI, start Phase 7, publish a Developer Preview, or create future runtime
crates.
