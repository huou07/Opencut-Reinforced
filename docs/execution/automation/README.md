# Model orchestration — operator guide

Deterministic control plane over free/cheap models. Models are workers and
advisers, never truth. Truth remains: PLAN, STATE, invariants, phase
contracts, exact SHA, tests, CI, evidence policy, and `agent_supervisor.py`.

## A. CLI unattended workflow

```sh
python3 scripts/model_orchestrator/__main__.py status    # task/run memory
python3 scripts/model_orchestrator/__main__.py doctor    # policies + memory + parity + dispatcher
python3 scripts/model_orchestrator/__main__.py models    # discovered model IDs per role
python3 scripts/model_orchestrator/__main__.py plan      # supervisor: current NEXT
python3 scripts/model_orchestrator/__main__.py step      # deterministic next action
python3 scripts/model_orchestrator/__main__.py run --auto   # ONE bounded engine cycle
python3 scripts/model_orchestrator/__main__.py resume    # real relaunch of interrupted task
python3 scripts/model_orchestrator/__main__.py pause      # block new dispatch
python3 scripts/model_orchestrator/__main__.py unpause    # re-allow dispatch
python3 scripts/model_orchestrator/__main__.py escalate  # pending escalation state
python3 scripts/model_orchestrator/__main__.py explain   # sources of truth
```

Without `--auto`, `run` refuses to dispatch. With `--auto`, one bounded
cycle executes: pause check, task-lease claim (second claimant gets
TASK_BUSY), packet validation, runtime model selection, trusted-authority
freeze, precondition check, exactly ONE worker launch, git re-inspection,
scope/protection/anti-gaming guards against the FROZEN authority,
ONE independent reviewer of a different family, persist, and stop at
PROMOTION_READY or another explicit safe state. The selected model (never
stale packet metadata) reaches the worker command; reasoning `--variant`
is passed only when declared supported, else `DEFAULT_PROVIDER` is
recorded. Promotion to main and supervisor handoff are separate explicit
gates bound to the runtime authorization record — never automatic. Nothing
here advances checkpoints; the supervisor (`--resume-sha <exact-SHA>`)
still owns hosted verification, evidence, and STATE transitions.

## B. OpenCode Desktop dispatcher workflow

Open Desktop in the repository and ask `continue the current roadmap`. The
`model-dispatcher` subagent (`.opencode/agents/model-dispatcher.md`,
Nemotron 3 Ultra Free, HIGH) runs `status` then `step`, reports blockers and
the next legal action, and stops. It cannot edit, commit, push, or advance
state. Read-only operation is machine-enforced by its `permission:` block
(`edit: deny`, `task: deny`, `external_directory: deny`, bash default-deny
with an inspection-only allowlist) and verified by `doctor` via
`dispatcher_safety.check_dispatcher`, which fails closed as
`READ_ONLY_ENFORCEMENT_UNAVAILABLE` if the block is missing or weakened.
Worker (`orch-worker`) and reviewer (`orch-reviewer`) agents likewise deny
nested task spawning and external-directory access; the reviewer additionally
denies edits. Scope and protection are always re-verified deterministically
after every run, so agent prose is never the boundary — git inspection is.

## C. Resume after model interruption

State lives in `<git-common-dir>/opencut-automation/` (shared across
worktrees, never committed). A dead worker leaves its run `RUNNING` with a
dirty worktree; the next cycle (or explicit `resume`) marks it
`INTERRUPTED`, preserves every dirty file (no reset, no clean), and
`resume` relaunches exactly one worker on the SAME task/branch with an
explicit resume context (existing HEAD, dirty paths, do-not-restart),
then re-applies classification, guards, and review. `pause` blocks NEW
dispatch only (checked before task claim); it never kills a legally
claimed RUNNING worker. `unpause` re-allows dispatch. One task lease
(`leases/<task>.lock`, flock-held across the whole cycle) guarantees a
single writer; a second claimant receives TASK_BUSY.

## Trust boundaries (enforced, not prose)

- Protection truth is the frozen `scripts/agent_supervisor.py` predicate
  from the trusted checkout — never candidate code. Freshness is
  re-verified after the worker; drift fails closed (TRUSTED_POLICY_MISMATCH).
- Protected-workflow authorization comes from trusted PLAN resolution
  (`runner_allowed_protected_paths`); the packet snapshot is audit-only and
  any mismatch fails closed (TASK_CONTRACT_MISMATCH).
- Rename/copy changes check BOTH old and new paths.
- Promotion reads its authorization record from runtime memory itself and
  revalidates digests, trees, review, NEXT, and remote before a
  fast-forward-only push; handoff additionally requires a PROMOTED record.
- Model output is untrusted: worker success is git state, reviewer verdicts
  are schema-validated event payloads, Jev is fixed-label advisory only.

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

## Routing, Jev, and reasoning effort

Jev is not installed here, so routing is deterministic; the adapter
(`router.JevAdapter`) can invoke a configured Jev model through
`opencode run` if one appears, accepts exactly one fixed label, and falls
back on any failure. Advisory output never overrides hard safety rules.
Reasoning effort is truthful: the policy `reasoning` value is recorded as
requested, and `--variant` is passed only when the model entry declares
support — otherwise `reasoning_effective = DEFAULT_PROVIDER`.

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
