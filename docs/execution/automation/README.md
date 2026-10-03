# Model orchestration — operator guide

Deterministic control plane over free/cheap models. Models are workers and
advisers, never truth. Truth remains: PLAN, STATE, invariants, phase
contracts, exact SHA, tests, CI, evidence policy, and `agent_supervisor.py`.

## A. CLI unattended workflow

```sh
python3 scripts/model_orchestrator/__main__.py status    # task/run memory
python3 scripts/model_orchestrator/__main__.py doctor    # policies + memory health
python3 scripts/model_orchestrator/__main__.py models    # discovered model IDs per role
python3 scripts/model_orchestrator/__main__.py plan      # supervisor: current NEXT
python3 scripts/model_orchestrator/__main__.py step      # deterministic next action
python3 scripts/model_orchestrator/__main__.py run --auto   # opt-in worker dispatch
python3 scripts/model_orchestrator/__main__.py resume    # resume interrupted task
python3 scripts/model_orchestrator/__main__.py pause      # stop dispatching
python3 scripts/model_orchestrator/__main__.py escalate  # pending escalation state
python3 scripts/model_orchestrator/__main__.py explain   # sources of truth
```

Without `--auto`, `run` refuses to dispatch. Nothing here advances
checkpoints; the supervisor (`--resume-sha <exact-SHA>`) still owns hosted
verification, evidence, and STATE transitions.

## B. OpenCode Desktop dispatcher workflow

Open Desktop in the repository and ask `continue the current roadmap`. The
`model-dispatcher` subagent (`.opencode/agents/model-dispatcher.md`,
Nemotron 3 Ultra Free, HIGH) runs `status` then `step`, reports blockers and
the next legal action, and stops. It cannot edit, commit, push, or advance
state.

## C. Resume after model interruption

State lives in `<git-common-dir>/opencut-automation/` (shared across
worktrees, never committed). A dead worker leaves its run `RUNNING` with a
dirty worktree; resume marks it `INTERRUPTED`, preserves every dirty file
(no reset, no clean), reuses the same task packet, and continues that exact
work on the same branch/worktree.

## D. When a free model disappears

`models` re-discovers via `opencode models` on every call. A missing ID
becomes `UNAVAILABLE` and the next allowed candidate for that role is
selected. No redesign, no failure. If no reviewer of a different family
exists, candidates stay `REVIEW_PENDING` — never self-approved.

## E. When Codex quota is exhausted

Escalation state becomes `ESCALATION_DEFERRED_QUOTA`; the exact packet is
preserved under `escalations/` and the task stops safely. This is deferred
availability, not repository failure. Resume with `resume` when quota
returns. Never probe in a loop, never substitute a weaker model.

## F. Inspecting task/run state

`status` prints `current.json`; per-task packets live in `tasks/`,
run records in `runs/`, telemetry in `model-stats.json` — all JSON,
schema version 1, atomic writes, under the common git dir.

## G. Disabling autonomous operation

Omit `--auto` (default), or run `pause`. The dispatcher agent has no
dispatch capability at all.

## H. Supervisor authority

The orchestrator emits implementation candidate SHAs plus local results and
reviewer reports, then hands the exact SHA to
`scripts/agent_supervisor.py --resume-sha`. Hosted CI, evidence records,
`NEXT → DONE`, and completion commits remain supervisor-owned. An
implementation commit alone is never DONE.
