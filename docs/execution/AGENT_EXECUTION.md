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

## Trust boundary

The runner owns one locked checkpoint's source implementation, ordinary
feature documentation, tests, local headless verification, implementation
commit, and normal push. It does not own execution state, completion evidence,
hosted CI truth, Developer Preview truth, or the successor checkpoint.

The supervisor owns the exact implementation SHA, hosted evidence
verification, optional preview verification, evidence record, `STATE.json`
transition, state/evidence commit, and the decision to launch a fresh runner.
An LLM saying `DONE` is never repository-authoritative completion.

## Required sequence

Before work, the supervisor requires `main`, a clean worktree, `HEAD ==
origin/main`, and a valid plan/state graph. It captures the bytes of `PLAN.json`,
`STATE.json`, and every protected execution-control surface.

The runner then receives one generated prompt and one checkpoint. It must:

1. implement only that checkpoint;
2. leave the plan, state, policy, phase specs, validators, supervisor,
   protected workflows, and evidence unchanged;
3. run local headless checks allowed by repository policy;
4. commit and push implementation changes only; and
5. return `IMPLEMENTED — AWAITING SUPERVISOR EVIDENCE` with the detailed
   handoff contract below.

After the runner exits, the supervisor fetches `origin`, requires a clean
worktree and `HEAD == origin/main`, and rejects any protected-surface or
`STATE.json` mutation. The resulting `HEAD` is the implementation SHA. The
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

## Resume

If the supervisor is interrupted after implementation push, `--resume-sha`
resumes hosted verification without rerunning the implementation. The SHA must
be a lowercase 40-character value equal to `HEAD` and `origin/main`; the
current `NEXT` must still be the same checkpoint; the worktree must be clean;
the SHA must descend from the prior state baseline; and its history must not
modify protected execution-control surfaces. The supervisor never guesses a
SHA from arbitrary history.

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

## Failure behavior

If implementation CI fails, the checkpoint remains `NEXT`, no completion
evidence is written, and no successor runner starts. The supervisor reports
the implementation SHA, failed workflow, run ID/URL, and failed or missing job
names, then stops. It does not start an arbitrary repair loop in the same
context. If a required preview cannot be dispatched with an appropriate
authenticated token, it stops with `STATE.json` still unchanged so a manual
release or a later resume can be verified.
