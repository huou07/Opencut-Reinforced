# V1 audit — source, production paths, and disposition

## Immutable audit scope

Audit date: 2026-10-03. The initial worktree was clean on
`control/model-orchestrator-v1`, HEAD
`19c94f9677e0ff55ef37fb88b31824435d28e514`. Local `origin/main` and a read-only
remote ref query both returned
`915a4a8b951643e475683e4cdf56996118ec7d1e`. The remote prototype ref also matched
HEAD. The local and remote `wip/9b-candidate-beaef` ref remained
`beaef3b7dba878d705dba07d7b9232860e184f83`.

STATE reported 9B NEXT, 9B1/9C PLANNED, versions project/recovery/IPC 7/1/1.
This is an audit snapshot, not a second mutable execution-status document.
No supervisor, product worker, native application, emulator, CI poll, or
checkpoint transition was launched.

Inspected authority: AGENTS, PLAN, STATE, EVIDENCE_POLICY, permanent invariants,
architecture-policy, AMENDMENT_BASELINE, AGENT_EXECUTION, Phase 9, ADRs 0006/0008,
and the execution plan, supervisor, evidence verifier, and infrastructure tests.
The full `915a4a8...19c94f9` diff consists of 30 added files / 4,192 added lines.
All added source, test, agent, policy, and documentation surfaces were inspected.
The intermediate corrective commits are historical context, not evidence of
correctness. Source references below use line numbers at the audited SHA.

## Actual production path

`__main__.py:_engine` loads policy from its own checkout and discovers models.
`cmd_run` reads the active task and calls `Orchestrator.run_cycle`.
`run_cycle` reads pause/task/run, releases the state lock, acquires a task lease,
then freezes authority, launches an OpenCode worker, inspects Git, runs guards,
and launches review. Review PASS plus no gaming flags directly writes a
promotion authorization and `PROMOTION_READY`. There is no required-test
execution stage and no acceptance-materialization stage.

Promotion and handoff are callable functions outside that CLI flow.
`promote_task` reads runtime JSON, checks selected digests and Git state,
pushes, fetches, merges local main, then writes PROMOTED. It holds no integration
lock. `handoff_to_supervisor` optionally checks a task record, then launches the
supervisor from the mutable main checkout. `cmd_escalate` only displays state;
`Adapters.codex`, attempt helpers, interruption detection, and telemetry helpers
are not integrated into the live lifecycle.

## A–V audit

PROVEN means directly established by source or the narrow reproduction below;
it does not imply deployed/runtime certification. SUPPORTED means a source
mechanism exists but its actual external boundary has not been verified here.

| Area | Disposition | Repository evidence and consequence |
| --- | --- | --- |
| A. Worker selected model | KEEP_WITH_REDESIGN | `orchestrator.py:146,244–258` selects at dispatch and passes model/entry/effort to `worker.py:233–249`. This wiring exists. A packet `model_locked` is accepted if found anywhere in policy, without role-specific enrollment or explicit lock authorization. Discovery lists are treated as availability. Keep runtime selection; require certified role enrollment and transport receipts. |
| B. Live reviewer | KEEP_WITH_REDESIGN | `_review`, `orchestrator.py:310–345`, builds `OpenCodeReviewAdapter` when none injected. `review.py:129–149` calls a real process adapter and parses events. Prompt omits the task's acceptance, tests, invariants, and resource contract; required fields are shallow and some are controller-fabricated. Keep separate model family and immutable candidate review; replace report validation and prompt contract. |
| C. Worker reasoning variant | KEEP_WITH_REDESIGN | `worker.py:215–220,245–250` conditionally maps requested effort to declared variants; policy declares no variants, so effective effort is provider default. No proof provider honored a requested variant. Preserve honest default recording; pin capability/version and distinguish requested, sent, and confirmed effort. |
| D. Frozen authority | REPLACE | `trusted_authority.py:38–65` snapshots PLAN/STATE but hashes and freshness-checks only supervisor bytes. Supervisor imports execution modules through its checkout; `_execution_plan_module`, lines 74–80, imports from the orchestrator source root, not necessarily `trusted_repo`. Evidence, invariants, policies, schemas, adapters, and agent config are not frozen. No trusted release/base identity is established. |
| E. Trusted PLAN resolution | KEEP_WITH_REDESIGN | `resolve_checkpoint`, lines 83–99, uses captured plan/state and existing `resolve_goal`. Initial live flow compares packet allowance to that result. Keep resolver reuse; use the entire frozen tree and explicit frozen `repo_root` rather than a module's default root. Resolve before dispatch, not after implementation. |
| F. Candidate authorization | REPLACE | CLI policies come from its source checkout; guard helpers default to importing the candidate supervisor (`guards.py:29–48,119`); a normal engine passes a separate module, but unsafe callable defaults remain. `contract_digest` excludes goal, role, worktree, and authority. New orchestrator/policy/agent paths are explicitly tested as unprotected (`tests:307–327`). Task creation accepts caller scope and required tests with presence checks only. Promotion trusts a self-consistent copy of a packet inside writable JSON. |
| G. Runtime write isolation | REPLACE | `runtime_memory.py:29–46` creates records under common Git; OpenCode subprocess inherits host user/environment (`worker.py:247–250`). Shared worktrees expose the common Git directory; an arbitrary shell can access it. Agent path permissions and atomic replace do not remove OS write capability. A hash stored beside data is forgeable by the same writer. No isolation or authentic controller ownership is demonstrated. |
| H. Rename/copy protection | KEEP_WITH_REDESIGN | `guards.py:58–103` uses NUL-delimited name-status and checks both rename/copy paths, including delete/add. Keep these scenarios. Parser truncation silently breaks rather than rejecting; text decoding, path normalization, type changes, symlinks, Git config, and intermediate commits need fail-closed validation. |
| I. Task lease | KEEP_WITH_REDESIGN | `TaskLease`, `runtime_memory.py:156–184`, holds flock across a cycle. A second claimant after worker start is tested. But task/run are read before lease and not re-read after it; a delayed claimant can dispatch using stale PENDING data. Controller death releases flock while descendants may survive. POSIX `fcntl` is unconditional. Keep one-host locks; claim transaction and container liveness must be integrated. |
| J. Pause race | REPLACE | `run_cycle:197–218` and `resume_cycle:397–415` check pause before lease, with a gap before RUNNING. No second pause read in `_cycle_under_lease`. A later pause also does not prevent new review. The source docstring's atomicity claim is false. |
| K. Dispatcher permissions | REPLACE | `model-dispatcher.md:15–18` permits globbed `git log *` / `git diff --stat *`; checker tests a finite command list. Its fnmatch accepts a composed command, though this does not prove OpenCode's real command parser executes it. Git diff output options can write files. `status`/`doctor` create directories and locks; `step` may call Jev. Reviewer permits `git branch *` and candidate test execution. Replace shell-based inspection with a read-only published snapshot and no tools. |
| L. Jev routing | REMOVE | `router.py:46–76` implements a configured fixed-label subprocess; `route:89–99` always chooses deterministic routing. `next_action` can spend a router call merely to display a step, using hardcoded `attempts_remaining=True`. Remove automatic redundant calls; retain an optional no-tools advisory label protocol only when it supplies useful triage. |
| M. OpenCode event parsing | KEEP_WITH_REDESIGN | `events.py:17–38` centralizes NDJSON text extraction. It concatenates all text parts, accepts incomplete streams and ignores event errors/session identity. No bounds, terminal-success requirement, duplicate JSON-key rejection, or tool/session provenance. Keep one adapter location; certify the installed version and reject ambiguous/truncated streams. |
| N. Interrupted resume | REPLACE | `resume_cycle` preserves dirty files and branch ancestry, but freezes authority only after resumed worker output in `_finish_candidate:469–470`. It omits the initial allowance-snapshot comparison. Dead RUNNING detection is unused; a crash leaves TASK_BUSY indefinitely, even with no lease holder. Process trees are not proven dead. Resume requires a new commit even if a clean candidate was committed just before death. Separate stage recovery from worker relaunch. |
| O. Required tests | REPLACE | `required_tests` appears in packet/digest/promotion metadata only. `_cycle_under_lease:287–300` goes from guards straight to review, and `_review:357–359` writes authorization without test receipts. `local_tests` initializes empty. The supervisor's workspace tests occur later, after push, and cannot fix missing pre-promotion tests. |
| P. Anti-thrashing | REPLACE | `failures.py:26–43` counts only `evidence_backed` history and is never called by live engine. `next_action:549` sets attempts remaining true; packet budgets never enforce dispatch. Two speculative attempts and gate/subsystem-level history cannot be avoided by changing a fingerprint. Implement claim-time budget reservation and evidence-backed causal repair admission. |
| Q. Promotion authorization | REPLACE | `promotion.py:125–171` compares selected self-contained JSON digests; no test receipts, authority digest, immutable reviewer receipt, or task-original comparison. Stored allowance is used rather than compared with frozen PLAN. `promote(...precheck)` at lines 98–102 is an exported bypass. `_clean` ignores Git exit failure. Remove caller-authored assertions and legacy promotion. |
| R. Integration lock | REPLACE | `promote_task:122–193` uses the state lock for JSON reads/writes only; Git mutation happens outside any promotion lock. It neither acquires a separate integration lock nor enters PROMOTING through the transition function. The docstring's lock/TOCTOU-safe assertion has no implementation. |
| S. Remote/local recovery | REPLACE | `_push_and_verify:105–113` returns an ordinary error when fetch or local fast-forward fails after successful push. PROMOTED is written only afterward, so remote main can be advanced while the run remains PROMOTION_READY. A retry then fails its own base check. No durable intent or reconciliation distinguishes push uncertainty from local synchronization failure. |
| T. Supervisor authorization | REPLACE | `supervisor_link.py:46–66,81–86` accepts `task_id=None`, skipping task authorization. Present-ID checks are incomplete and merely bind three fields/status. Invocation executes candidate checkout supervisor code. Require task ID and controller-owned promotion receipt for every V2 resume/repair path; run frozen supervisor code. |
| U. Reviewer reasoning/evidence | REPLACE | `_review:323` constructs reviewer with model only, omitting policy entry and HIGH effort; adapter defaults to DEFAULT. `build_review_prompt:60–70` asks only verdict/findings/tests. No PROVEN/SUPPORTED/PLAUSIBLE/UNKNOWN, causal hypotheses, competing explanations, product boundary, or discrimination requirements. |
| V. Product-value enforcement | REPLACE | `acceptance` is a list of strings hashed into metadata; no journey, environment, performance bound, or artifact receipt is enforced before readiness. Anti-gaming scans only added lines in four path groups (`guards.py:142–172`), mostly `|| true`, failure tolerance, and select timeout text. Deleted assertions, mocks, host tools, skipped cases, and resource-bound removal are missed. Existing supervisor named-step proof is necessary but cannot alone prove semantic fidelity. |

## Other major subsystems

| Subsystem | Disposition | Reason |
| --- | --- | --- |
| Main plan/state/evidence taxonomy | KEEP | Reuse current authority and all 11 evidence classes; do not invent alternate DONE state. |
| Existing supervisor and exact-SHA evidence verification | KEEP_WITH_REDESIGN | Retain existing tests, run/job/step/preview checks and state-only completion. Amend task-bound invocation, trusted source execution, acceptance receipts, and completion transaction recovery. |
| Policies and packet validation | REPLACE | Presence-only fields and schema/status drift are unsuitable for authority. Policies become one strictly validated, frozen control release. |
| Runtime JSON memory | KEEP_WITH_REDESIGN | Facts in Git-common-dir remain appropriate only when workers cannot mount it. Reject malformed records; add fsync, sequence transactions, fencing, immutable receipts, and explicit initialization. |
| Escalation adapter | KEEP_WITH_REDESIGN | Preserve deferred quota state; V1 classifies by broad text and returns raw stdout, does not enforce structured decisions or wire execution. |
| Model discovery/cost telemetry | KEEP_WITH_REDESIGN | Listed ID/free suffix is neither capability nor usable authentication. Paid IDs are discovered but excluded by the availability predicate. Statistics writer is unused. Record actual outcomes; enroll capabilities before selection. |
| V1 protection metadata | REMOVE | Drift-prone PROTECTED_PATHS.json duplicates a subset of supervisor truth. Use one adopted frozen protection contract. |
| CLI and lifecycle engine | REPLACE | No create/review-retry/promote/handoff CLI lifecycle; run returns zero even for rejection; graph/schema/live mutations disagree. Implement explicit legal actions and typed exit results. |
| V1 test suite | KEEP_WITH_REDESIGN | Retain adversarial scenarios, not copied fixtures or claims. Many integration tests inject the supervisor override and bypass real PLAN freezing; many declare no tests/acceptance. Promotion tests synthesize authorizations directly. No live sandbox, crash reconciliation, pause-gap, required-test, or frozen-harness acceptance. |

## Narrow reproductions performed

Executed once, in temporary Git repositories using V1's own test helpers and
engine. No production refs, task records, native processes, or remote GitHub
writes were touched. These are defect reproductions, not product evidence.

1. Put `python3 -c "raise SystemExit(23)"` in the stored task's `required_tests`;
   run the normal engine with the existing fake worker/reviewer: observed
   `PROMOTION_READY`. There is no command execution receipt.
2. Wrap `TaskLease.__init__` to write paused under `LockedState` before the
   real lease acquisition: observed `PROMOTION_READY`. Pause precedes the claim.
3. In a real temporary bare-remote/main pair with clean main and aligned SHA,
   call `handoff_to_supervisor` without task ID: observed `ok=true` and the
   injected invocation callback was called despite no authorization record.
4. Evaluate the actual dispatcher checker for
   `git log ; touch /tmp/not-executed`: observed `allow`. The composed command
   was never executed. The checker is inadequate; actual OpenCode shell
   semantics need a separate negative capability test.

Repeat procedure: import `model_orchestrator.tests.test_orchestrator` at the
audited SHA, use `EndToEndPipelineTests._setup` for cases 1/2 and
`PromotionAuthTests._setup` for case 3. For case 3 push/fast-forward the candidate
in that temporary bare pair, then invoke handoff with a recording callback.
Restore the lease class after case 2. This documents reproducible API-level
evidence without treating injected model PASS as proof of production quality.

## Engineering decision

**CLEAN REIMPLEMENTATION**, based on main. Selected-model wiring and rename
coverage are useful lessons, but task authorization, authority ownership,
verification, process isolation, transition persistence, and recovery all need
replacement. Incremental patches would preserve unsafe entrypoints and two
incompatible lifecycle paths. The new contracts are small enough to implement
directly; no distributed scheduler, plugin framework, or daemon is needed.

The frozen 9B candidate and its recorded hosted-failure fixture remain
unmodified. The fixture's claim about run `37110360212` was inspected as fixture
content; this session did not query that hosted run and does not independently
certify its outcome. A driver disconnect before SAF assertions remains
inconclusive until discriminating evidence separates app/JNI/main-thread,
driver/VM-service, and emulator causes.
