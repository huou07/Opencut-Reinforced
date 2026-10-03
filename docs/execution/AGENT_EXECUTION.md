# Agent execution contract

This document defines the autonomous execution protocol. It is a control-plane
contract, not a product feature.

## Goal selection

The supervisor accepts exactly one of:

```text
checkpoint:<ID>
phase:<N>
milestone:desktop-mvp
milestone:full-roadmap
```

`PLAN.json` and `STATE.json` resolve the goal to the only checkpoint currently
marked `NEXT`. A runner never chooses an arbitrary checkpoint.

Before a multi-checkpoint phase is eligible for unattended execution, every
remaining checkpoint must receive its required architecture, model, and
dependency contracts from an earlier checkpoint or explicitly own them itself.
A checkpoint must not consume a contract whose first owner occurs later in the
execution graph.

PLAN project-schema and IPC effect categories are distinct namespaces. A
checkpoint must not use one field's category family in the other field.

## Contract versions and recovery

`STATE.json` records `verified_contract_versions`, the versions proven by the
latest completed checkpoint. `architecture-policy.json` defines transition
rules, not mutable current versions. Before each token-using runner, the
supervisor runs a read-only preflight; explicit model and contract gates may
retain their version or increment it by one, while downstream `*-only`
checkpoints must retain the verified version. Recovery schema remains fixed
until an explicit recovery owner is planned. Completion derives `phase_status`
from checkpoint statuses and snapshots the exact verified source versions.

`--repair-resume-from` is a strict control-plane recovery path, not a runner
bypass. It checks the original implementation and active checkpoint contract,
accepts only trusted maintenance changes, and records the repaired HEAD as the
verified implementation SHA with the failed implementation SHA as provenance.
Its state baseline is the oldest first-parent revision in the contiguous run
before the failed SHA that keeps the checkpoint NEXT under the same active
contract fingerprint. Missing historical `STATE.json` ends that walk;
malformed execution state fails closed.
Repair history may also contain corrections confined to crate `tests/`
directories or the OR app's `test/` and `integration_test/` directories; runtime
source, manifests, workflows, evidence policy, and active checkpoint contracts
remain protected. The supervisor runs `cargo test --workspace` on the exact
candidate before hosted verification, including after a repair. A local failure
stops before GitHub polling and leaves state unchanged.

## Trust boundary

The runner owns one locked checkpoint's source implementation, ordinary
feature documentation, tests, local headless verification, implementation
commit, and normal push. It does not own execution state, completion evidence,
hosted CI truth, Developer Preview truth, or the successor checkpoint.

The supervisor owns the exact implementation SHA, hosted evidence
verification, optional preview verification, evidence record, `STATE.json`
transition, state/evidence commit, and the decision to launch a fresh runner.
An LLM saying `DONE` is never repository-authoritative completion.

The generated runner prompt names required evidence classes and states that
passing existing tests with a smaller implementation is not the objective.
It forbids substituting bridge calls for in-scope UI journeys, host tools for
packaged dependencies, in-process checks for persistent reopen, disabled
permissions for packaged permission behavior, or synthetic tests for failed
acceptance. After two speculative fixes to one gate, the runner pauses for
failure evidence, a falsifiable hypothesis, a discriminating test, and a
causal repair explanation. Unit and bridge tests still support lower-level
claims.

A checkpoint introducing a native/system dependency must not be asked to prove
hosted compatibility with CI infrastructure it is forbidden to establish. If
a protected CI/build change is required, use a preceding architecture-gate
checkpoint with an explicit exact-path protected-workflow allowance. A
downstream implementation checkpoint may also receive that allowance when it
needs to operationalize a dependency strategy its predecessor already approved.

## Manual/Desktop checkpoint preparation

Prepare the authoritative prompt for the current checkpoint:

```sh
python3 scripts/agent_supervisor.py \
  --goal checkpoint:<ID> \
  --prepare
```

On macOS, the output can be copied directly to the clipboard:

```sh
python3 scripts/agent_supervisor.py \
  --goal checkpoint:<ID> \
  --prepare | pbcopy
```

Paste the generated prompt into one fresh Codex Desktop conversation opened on
the repository. Desktop performs exactly the generated checkpoint: source
implementation, local headless verification, implementation commit, and normal
push. It must not edit `STATE.json` or completion evidence.

After obtaining the exact implementation SHA, resume deterministically:

```sh
python3 scripts/agent_supervisor.py \
  --goal checkpoint:<ID> \
  --resume-sha <40-character implementation SHA>
```

The supervisor owns hosted CI verification, evidence, the `STATE.json`
transition, the completion commit, and state-commit hygiene. `--prepare` does
not mean the checkpoint has started or completed. Desktop chat history is
human-readable execution history only, not repository-authoritative evidence.

## Required sequence

Before work, the supervisor requires `main`, a clean worktree, `HEAD ==
origin/main`, and a valid plan/state graph. It captures the bytes of `PLAN.json`,
`STATE.json`, and every protected execution-control surface.

The runner then receives one generated prompt and one checkpoint. It must:

1. implement only that checkpoint;
2. leave `PLAN.json`, `STATE.json`, policy, phase specs, validators, supervisor,
   and evidence unchanged; leave protected workflows unchanged except for
   exact paths authorized by the checkpoint's PLAN entry;
3. run local headless checks allowed by repository policy;
4. commit and push implementation changes only; and
5. return `IMPLEMENTED — AWAITING SUPERVISOR EVIDENCE` with the detailed
   handoff contract below.

After the runner exits, the supervisor fetches `origin`, requires a clean
worktree and `HEAD == origin/main`, and rejects any protected-surface change
outside the checkpoint's exact-path allowance or any `STATE.json` mutation.
The resulting `HEAD` is the implementation SHA. The
supervisor then verifies, in order:

1. Repository hygiene on that exact SHA;
2. Platform verification on that exact SHA;
3. every policy-required job in both workflows;
4. the exact Developer Preview when `PLAN.json` requires it;
5. the evidence record and offline record validation;
6. the single `NEXT -> DONE` plus planned-successor transition;
7. local plan, policy, and repository checks;
8. a state/evidence-only completion commit; and
9. Repository hygiene on the exact state-commit SHA.

Only after the final hygiene gate succeeds may a new fresh runner process
start. A phase or milestone therefore means a sequence of fresh runners,
hosted verification, and state commits—not one model context implementing
multiple checkpoints.

A successful direct runner invocation must produce a new implementation
commit. If `HEAD` is unchanged, stop immediately; it is not an implementation
SHA and hosted polling must not begin. For a checkpoint whose locked contract
makes an optimization optional, missing qualifying evidence should produce a
repository-recorded "not enabled / software fallback retained" decision when
that is an allowed outcome, rather than speculative implementation just to
advance.

## Hosted evidence

`docs/execution/EVIDENCE_POLICY.json` is deterministic policy. Product gates
must be push-triggered runs on `main` whose `head_sha` equals the implementation
SHA, whose status is `completed`, whose conclusion is `success`, and whose
workflow file/name matches policy. Workflow success alone is insufficient:
every required job must also be completed successfully. Pull requests,
`workflow_dispatch`, another SHA, pending runs, and non-success conclusions do
not satisfy a product gate.

The supervisor uses Python's standard library REST client. `GH_TOKEN` is
preferred over `GITHUB_TOKEN`; when neither is present, public unauthenticated
read verification is allowed. Authenticated polling is 15 seconds,
unauthenticated polling is 90 seconds, and the total default wait is 7200
seconds. HTTP 403/429 or exhausted rate limits stop execution. Tokens are
never printed, passed as subprocess arguments, stored, or written to evidence.

The policy boundary begins at 7A. Earlier completed checkpoints remain
grandfathered and do not receive fabricated evidence. A `DONE` checkpoint at
or after the boundary is invalid without its supervisor-generated evidence
file.

Quality evidence classes begin at 9B. `PLAN.json` declares required classes;
`EVIDENCE_POLICY.json` binds each class to named hosted job steps. The
supervisor queries those steps on the exact implementation SHA and writes
schema-2 class proofs. A missing or unsuccessful step prevents completion.
Historic schema-1 evidence remains unchanged. The 9B quality amendment is a
control-plane-only baseline with explicit failed-run provenance and an exact
changed-path allowlist; fresh 9B implementation commits are measured from it.
The old failed 9B history is preserved, and ordinary protected-path checks
continue after the amendment.
The amendment marker must change in each trusted control-plane commit; an
inherited marker on a later 9B state commit is not a new trusted baseline.
A follow-up must directly follow a validated amendment, preserve the locked
plan, and contain only its exact control-path allowlist. The supervisor uses
the latest validated marker commit when it follows the state baseline, so
fixture corrections need no fabricated state change. All retained state and
verified schema versions are preserved. Proof-step bindings for 9B
and 9B1 are part of this amendment. A later checkpoint without approved
bindings fails preparation and completion; its bindings require a separate
control-plane amendment before a feature runner can start.

## Resume

If the supervisor is interrupted after implementation push, `--resume-sha`
resumes hosted verification without rerunning the implementation. The SHA must
be a lowercase 40-character value equal to `HEAD` and `origin/main`; the
current `NEXT` must still be the same checkpoint; the worktree must be clean;
the SHA must descend from the prior state baseline; and its history must not
modify protected execution-control surfaces outside the checkpoint's
PLAN-listed exact-path allowance. The supervisor never guesses a SHA from
arbitrary history.

## Runner handoff report

The final runner message must begin exactly with:

```text
IMPLEMENTED — AWAITING SUPERVISOR EVIDENCE
```

It must contain these headings and actual values:

```text
## CHECKPOINT
ID:
title:

## BASELINE
starting SHA:
branch:
ahead/behind:

## IMPLEMENTATION
files changed:
behavior implemented:
architecture/invariants preserved:
explicit out-of-scope:

## TESTS
new/updated tests:
results:

## LOCAL VERIFICATION
repo hygiene:
format:
static analysis:
Rust:
Flutter:
other:
Native Flutter runtime: NOT RUN — LOCAL NATIVE EXECUTION DISALLOWED BY POLICY

## IMPLEMENTATION COMMIT
SHA:
subject:
origin/main:
worktree:

## HOSTED EVIDENCE
AWAITING SUPERVISOR

## STATE
checkpoint remains: NEXT
successor remains: PLANNED

## BLOCKERS
...
```

## Supervisor verified report

After finalization the supervisor prints a deterministic report sourced from
the evidence record. It includes the checkpoint/title, implementation SHA and
subject, both workflow run IDs/URLs/head SHAs/conclusions and required jobs,
preview status/tag/source/release/assets when applicable, evidence path, exact
state transition, state commit, state-commit hygiene run, Git alignment, and
the next checkpoint/phase/spec. The runner cannot fabricate this report.

## V2 control plane (proposed, disabled)

ADR 0009 defines a task-bound controller that moves work through admitted
tasks, isolated candidates, controller-executed verification, independent
review, durable promotion, and the existing supervisor. It is **not active**:

- the adoption lifecycle state is `AMENDMENT_PROPOSED` and `full_auto_eligible`
  is false in `docs/execution/automation/V2_CONTRACT.json`;
- the task, candidate, verification, review, promotion, and completion receipt
  schemas are frozen in
  `docs/execution/automation/PROTOCOL_SCHEMAS.json` and validated by
  `scripts/model_orchestrator/contracts.py`;
- a schema-2 completion created under an adopted release must carry a nested
  `control_plane_receipt`; historical schema-1/2 records remain valid at their
  boundary and a caller cannot re-declare itself legacy;
- unrestricted `--runner`, receipt-less `--resume-sha`, and receipt-less
  `--repair-resume-from` remain available only until the separate adoption
  amendment disables them for activated V2.

An explicit bounded build authorization permits disabled fixture work before
operational adoption; it permits no product autonomy. No worker, model, router,
or Desktop process may treat this section as operational authority to run, and V1 prototype history is not V2 authority.

### M0-R2 candidate contract verification

The corrected design distinguishes disabled build permission from final
operational adoption. M0 implements contract validation only: no worker, store,
sandbox, promotion or supervisor entrypoint exists. Its manifest covers every
pinned Git tree entry plus known absent configuration names; Git modes/object
bytes, repository-anchor ancestry and clean materialization are validated.
Absent optional OpenCode/Rust toolchain configuration is pinned as absent; it
must not become an invented required repository file or appear after freezing.
Product source remains baseline input rather than a protected edit allowance.

Task and receipt validators require separately supplied frozen template/attempt
bindings. Recursively checked resources, argv/environment, acceptance and
performance floors cannot be changed by an output record. Test PASS, case
completion and product acceptance remain separate facts. Historical evidence
comes from actual Git blobs/ancestry and the existing evidence verifier, never
a caller legacy flag. Missing controller context refuses validation.

M0-owned regression groups are GitAuthorityMatrixTests (CP01/02/04),
TaskMatrixTests and ReceiptMatrixTests (CP03), CompletionBoundaryTests (CP05),
and LifecycleMatrixTests/ControlAmendmentMarkerTests (CP33). Run them with:
`python3 scripts/model_orchestrator/tests/test_contracts.py -v`, then the
existing infrastructure, plan/policy, hygiene, JSON and Python syntax checks.
They verify M0 contract boundaries, not future container/CLI/hosted certification.
An independent subsequent audit must approve M1 admission. Docker unavailability
is `M1_ISOLATION_ENVIRONMENT_UNAVAILABLE`, not a reason to relax isolation.
Git object reads follow the [Git object manual](https://git-scm.com/docs/git-cat-file).

## Failure behavior

If implementation CI fails, the checkpoint remains `NEXT`, no completion
evidence is written, and no successor runner starts. The supervisor reports
the implementation SHA, failed workflow, run ID/URL, and failed or missing job
names, then stops. It does not start an arbitrary repair loop in the same
context. If a required preview cannot be dispatched with an appropriate
authenticated token, it stops with `STATE.json` still unchanged so a manual
release or a later resume can be verified.
