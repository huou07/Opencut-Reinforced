# V2 implementation, migration, and acceptance plan

This is the executor contract for the frozen architecture. Implementation phases
are control-plane work packages, **not** OR product checkpoints. They confer no
authority to advance 9B, start 9B1, run the roadmap, or alter a locked product
contract. Each phase requires its own explicit architecture-task authorization,
one coherent committed change, and the specified gates. No executor decides
architecture by improvising around a blocked gate.

## Migration from 19c94f9

1. Preserve `control/model-orchestrator-v1` at
   `19c94f9677e0ff55ef37fb88b31824435d28e514`; preserve the initial/corrective
   history. Retain `wip/9b-candidate-beaef` at
   `beaef3b7dba878d705dba07d7b9232860e184f83`. Never reset, amend, merge, or
   cherry-pick them into V2.
2. This architecture branch is based directly on authoritative main
   `915a4a8b951643e475683e4cdf56996118ec7d1e`. The implementation branch starts
   from the reviewed architecture commit descended from that main, after the
   separate build authorization binds the disabled M0–M5 scope. Use a new descriptive branch such
   as `codex/model-orchestrator-v2`; never run the prototype as a trusted
   implementation controller.
3. Export V1 runtime facts read-only for operator inspection if needed. Never
   import its task, review, attempt, or promotion JSON as authorization. Keep
   the old directory untouched; V2 uses a separate `opencut-automation-v2/`
   directory initialized explicitly by the controller.
4. Implement staged, disabled V2 on that branch. Test against isolated fixture
   repositories and bare remotes, not the OR product or GitHub main. Keep all
   product and execution statuses/versions unchanged.
5. Adopt only after the full suite below, independent semantic review, and the
   proposed protected-surface amendment are verified. Publish one reviewed
   control-only adoption sequence with a truthful marker/manifest. The operator
   pins its final control release SHA; a candidate cannot adopt itself.
6. Prepare a fresh product task from the post-adoption authoritative main. The
   old 9B candidate cannot be directly promoted from its old base: adoption has
   changed main. An explicitly authorized import task may examine/reapply its
   product diff into a fresh isolated clone, preserving original SHA provenance
   and acquiring new tests/review/authorization. This architecture session does
   not perform that work or infer that the old candidate works.

No migration rewrites historic completion evidence or treats a chat report as
memory. Unexpected remote advancement stops for a refreshed contract. No auto
rebase, force push, or silent change to existing 9B acceptance.

## Exact future file inventory

All paths are repository-relative. Each phase's allowlist consists only of the
listed files, plus its listed documentation updates; changing the architecture
spec itself requires another architecture task. No V1 file is copied wholesale.
Use simple functions and small records, not speculative plugin/adapter factories.

| Phase | Files to add/change | Purpose and exit gate |
| --- | --- | --- |
| M0: adoption contract and schema | `AGENTS.md`; `docs/execution/ARCHITECTURE_INVARIANTS.md`; `docs/execution/AGENT_EXECUTION.md`; `docs/execution/architecture-policy.json`; `docs/execution/AMENDMENT_BASELINE.json`; `docs/execution/EVIDENCE_POLICY.json`; `docs/execution/PHASE_SPEC_TEMPLATE.md`; `docs/execution/automation/V2_CONTRACT.json`; `docs/execution/automation/MODEL_POLICY.json`; `docs/execution/automation/PROTOCOL_SCHEMAS.json`; `docs/execution/automation/SANDBOX_POLICY.json`; `docs/execution/automation/TASK_TEMPLATES.json`; `docs/execution/automation/CHECKS.json`; `scripts/model_orchestrator/__init__.py`; `scripts/model_orchestrator/contracts.py`; `scripts/model_orchestrator/tests/__init__.py`; `scripts/model_orchestrator/tests/test_contracts.py`; `scripts/check_execution_plan.py`; `scripts/check_architecture_policy.py`; `scripts/test_execution_infra.py` | Apply exactly the proposed separate amendment on the implementation branch, disabled. Formal strict record schemas, complete authority manifest and root resolution; legacy/current validators still pass. No worker dispatch. M0 build authorization is distinct from the proposed operational adoption marker; no active authority before final external adoption. CP01–05, 33. |
| M1: state and isolation | `scripts/model_orchestrator/store.py`; `scripts/model_orchestrator/sandbox.py`; `scripts/model_orchestrator/workspace.py`; `scripts/model_orchestrator/tests/test_isolation_and_state.py`; `scripts/model_orchestrator/tests/test_live_workspace_recovery.py`; `docs/TOOLING.md` | Durable single-snapshot transactions, separate locks/epochs, independent clone/container ownership and effective config overlays. The controller-owned reserve/bind/preserve/restore launch-workspace lifecycle and its real crash acceptance belong to M1 (CP07 × CP10 × CP11). Missing runtime blocks. CP06–11, 27–29, 34. |
| M2: guards and verification | `scripts/model_orchestrator/guards.py`; `scripts/model_orchestrator/verification.py`; `scripts/model_orchestrator/tests/test_verification.py` | Trusted quarantine import, all-commit path guards, floor detection, real required-check execution and acceptance readiness receipts. No review/promotion until test PASS. CP12–18, 30. |
| M3: model lifecycle | `scripts/model_orchestrator/adapters.py`; `scripts/model_orchestrator/orchestrator.py`; `scripts/model_orchestrator/__main__.py`; `scripts/model_orchestrator/tests/test_lifecycle.py`; `.opencode/agents/model-dispatcher.md`; `.opencode/agents/orch-worker.md`; `.opencode/agents/orch-reviewer.md` | One CLI path for admission/claims/recovery/review/escalation. Strict versioned transport, role/cost/effort enrollment, pause/attempt ledger, no-tools published Desktop snapshot. CP19–26, 31–32, 35. |
| M4: promotion and supervisor | `scripts/model_orchestrator/promotion.py`; `scripts/model_orchestrator/push_guard.py`; `scripts/model_orchestrator/tests/test_promotion_and_handoff.py`; `scripts/agent_supervisor.py`; `scripts/execution_plan.py`; `scripts/execution_evidence.py`; `scripts/test_execution_infra.py` | Durable authorization/intent; exact advertised-base non-force push; remote/local reconciliation; mandatory task handoff; adopted schema-2 receipts and state-only completion recovery. No optional bypass. CP36–43. |
| M5: real acceptance and release | `scripts/model_orchestrator/tests/test_live_acceptance.py`; `.github/workflows/control-plane-acceptance.yml`; `.github/workflows/repo-hygiene.yml`; `.github/workflows/platform-verification.yml`; `.github/workflows/developer-preview.yml`; `docs/TESTING.md`; `docs/TOOLING.md`; `docs/execution/AGENT_EXECUTION.md`; `docs/execution/automation/CHECKS.json`; `docs/execution/automation/TASK_TEMPLATES.json`; `docs/execution/automation/SANDBOX_POLICY.json`; `docs/execution/automation/MODEL_POLICY.json`; `docs/execution/AMENDMENT_BASELINE.json` | Certify pinned actual sandbox/OpenCode/Codex protocol and hosted trusted-harness receipts. Preserve existing product jobs/cases. Independently review the entire live path; adopt final disabled-to-enabled release only after all mandatory acceptance. CP44–48 and full regression suite. |

Documentation index/link paths allowed for the adoption are exactly
`docs/INDEX.md`, `docs/adr/README.md`, `docs/execution/README.md`,
`docs/TESTING.md`, plus the new `docs/adr/0009-model-orchestrator-v2.md` and the
six Markdown/JSON candidate-contract documents in this directory as listed
in the architecture commit. The four frozen Markdown documents and ADR are
read-only implementation inputs; changes to their design need a new architecture
review. MODEL_POLICY and V2_CONTRACT may be updated only under M0/M5 authority
for strict schema/enrollment/adoption metadata, never to loosen the freeze.

`contracts.py` includes authority loading/manifest and strict protocol checks;
`adapters.py` holds OpenCode/Codex/discovery transport; `orchestrator.py` holds
the legal state/routing/attempt/escalation transitions; `promotion.py` includes
handoff/reconciliation. Do not recreate V1's separate router, scope,
trusted-authority, failures, packets, policies, review, escalation, dispatcher
checker, and supervisor-link implementations alongside the new path. Necessary
logic remains, duplicate/unsafe entrypoints do not.

No product runtime or native platform install is part of these phases. Build
tools in the pinned verifier image are developer tooling, not product
dependencies. Verify image/tool licenses and sources, record pinned digests,
and do not distribute restrictive components as part of OR.

## Exact forbidden execution surfaces

For every M0–M5 implementation phase, forbid:

```text
docs/execution/PLAN.json
docs/execution/STATE.json
docs/execution/phases/**
docs/execution/evidence/**
docs/adr/0001-control-and-realtime-planes.md
docs/adr/0002-wgpu-render-spine-and-native-interop.md
docs/adr/0003-frame-memory-domain-and-ownership.md
docs/adr/0004-runtime-capabilities-and-provider-selection.md
docs/adr/0005-ai-provider-boundary.md
docs/adr/0006-autonomous-agent-execution-contract.md
docs/adr/0007-declarative-motion-scenes-and-procedural-isolation.md
docs/adr/0008-product-acceptance-and-execution-quality.md
DESIGN.md
docs/UX_ACCEPTANCE.md
docs/ARCHITECTURE.md
docs/TECHNICAL_PLAN.md
docs/PRODUCT.md
docs/ROADMAP.md
docs/SECURITY_LICENSING.md
Cargo.toml
Cargo.lock
rust-toolchain.toml
crates/**
apps/**
packages/**
assets/**
prototype/**
```

Also forbid every tracked path not explicitly allowed by that phase, all
controller/runtime records from worker access, all host Git refs/credentials,
the V1 branch, and the old 9B candidate branch. `.github/workflows/**` is
forbidden except the four exact M5 paths, and active control files are
forbidden except the exact M0/M4/M5 allowances granted by the separate amendment.
No cheap feature executor may change the design documents or invent an allowance.

## Mandatory control-plane acceptance suite

UNIT/fixture passes support the suite, but do not replace OS/CLI/hosted boundaries.
Tests should call the actual CLI/lifecycle and real Git/container processes
where named; process-boundary fakes are appropriate for deterministic fault
injection, never proof of live isolation/provider semantics. Fixtures create
their own independent authority trees and state; they must not depend on live
9B NEXT or copy an unrestricted injected supervisor override into integration
tests. Report exact version/profile/commands/cases/results and artifact digests.

| ID | Required adversarial/positive observation |
| --- | --- |
| CP01 | Candidate modifies supervisor, imported plan/evidence module, PLAN, STATE, invariant, model policy, schema, agent config independently: frozen authority and guard refuse each before invocation/promotion. Test complete source closure, not one file hash. |
| CP02 | Authority/code/schema absent, tampered, wrong mode/digest, mixed base/release, wrong repository, changed NEXT or default `repo_root`: admission refuses; zero model calls. |
| CP03 | Task scope/checkpoint/base/goal/role/required check/resource bound/allowance forged, wrong type, unknown field, duplicate JSON key, bool-as-int, path traversal and corrupt receipt: fail closed. |
| CP04 | Candidate changes config after authority freeze or during interrupted resume; resume retains original authority and refuses drift. No post-worker re-freeze. |
| CP05 | Old schema-1/2 evidence remains valid at its historical boundary; new adopted completion without nested task/acceptance receipt fails. No caller legacy override. |
| CP06 | Actual worker shell tries write/read into host common Git/runtime/controller/home/credentials, mount/socket access, `/proc` host reach, symlink escape: denied or absent; host records/refs unchanged. Explicitly test task, review, authorization, attempt, escalation forgery. |
| CP07 | Actual reviewer/router/architecture/dispatcher boundaries deny shell/edit/subagents/custom/MCP tools and control mutations. Reviewer source tree is OS read-only. Candidate/global/project config cannot override installed effective policy. |
| CP08 | Missing/unverified container profile, root/privileged/host namespace, unbounded memory/PID/output, or image mismatch: zero workers; ISOLATION_UNAVAILABLE. |
| CP09 | Candidate clone's `.git` is independent; attempts to access shared worktree refs/alternates/push origin fail. Controller import uses sanitized Git, never candidate hooks/config/filters. |
| CP10 | Kill controller while worker subprocess descendants live: new controller reconciles container before any lease reuse. PID/boot/nonce mismatch cannot be treated as dead proof. |
| CP11 | Interrupt after dirty edit and after clean commit: preserve files and branch; resume dirty task in same contract, import clean commit without another worker; no reset/clean/discard. |
| CP12 | Rename/copy both directions, protected delete/add, intermediate protected edit later reverted, malformed/truncated NUL output, non-UTF path, symlink/type/mode/submodule/replace-ref tricks: safe rejection where unsupported, both paths checked. |
| CP13 | Required command exits nonzero while worker/reviewer claim PASS: actual CLI does not reach PROMOTION_READY; exact command/exit/log receipt stored. |
| CP14 | Required command timeout, signal, skip, empty-case result, missing executable/environment, malformed output, candidate mutation or forged PASS artifact: no promotion. Environment block remains availability. |
| CP15 | Required harness/check catalog/case/assertion deleted or weakened, mocked real boundary, host dependency, permission disable, `continue-on-error`, `|| true`, retry/timeout increase, scope/resource-bound removal or silent fallback: deterministic veto/flag remains unresolved until authorized evidence disposition; real lowering never waived by model PASS. |
| CP16 | Candidate removes required tests but returns identical test-count metadata: frozen harness/case inventory detects omission. Candidate test additions cannot replace required baseline cases. |
| CP17 | Controller launches checks against exact imported candidate in fresh verifier; monkeypatch worker Git index/config/ignored cache or helper shims cannot forge cleanliness/results. |
| CP18 | Unit/bridge/local-file success cannot satisfy packaged/SAF/permission/relaunch/performance classes. Missing actual journey/harness/bounds blocks task admission or completion, not a lower class substitution. |
| CP19 | Role-qualified selected/fallback model ID reaches the **actual** OpenCode process argv/config; unavailable promotional model does not create failure or guessed ID. Packet preference/lock cannot bypass role enrollment. |
| CP20 | Reviewer model/family and policy reasoning reach actual transport; same or unknown family leaves REVIEW_PENDING. Requested/sent/confirmed/default effort recorded honestly; required unsupported effort blocks. |
| CP21 | Strict reviewer report requires full task coverage, classification and exact citations/discriminating evidence; wrong SHA, prose, missing field, invalid types, stale flags or PASS with blocking unknown cannot authorize promotion. |
| CP22 | OpenCode adapter fixtures for supported real event versions: multiline payload, tools/earlier text, multiple sessions/turns, error events, duplicate keys, truncated terminal stream, oversized/invalid UTF-8, nonzero process. Only one proven final payload accepted. |
| CP23 | Optional router fixed label cannot override deterministic scope/stop/attempt/evidence rule; unavailable router works without model. Status/next-action never dispatches it. |
| CP24 | Two attempted speculative corrections, including failed tool launches/model changes/fingerprint/retry changes, exhaust gate/subsystem episode budget in live claim path; third worker cannot start without admitted causal packet. |
| CP25 | Historical disconnect alone -> UNKNOWN/inconclusive, never infrastructure PASS. Require competing hypotheses and discriminatory observations; no automatic rerun-until-green route. |
| CP26 | Codex quota result preserves identical packet/attempt facts as ESCALATION_DEFERRED_QUOTA, does not fail repository or poll. Malformed decision/auth/tool errors cannot become an architecture contract. |
| CP27 | Two real controller processes contending for one task: exactly one worker launch. Delay second process before lease; after first completes it re-reads state and cannot redispatch stale READY. |
| CP28 | Pause-before-claim interleaving: zero workers. Pause-after-claim: current bounded stage settles, no new reviewer/task/push/supervisor. Unpause alone starts nothing. Test resume and review claims too. |
| CP29 | Crash between immutable receipt creation and snapshot replacement, truncated snapshot, lost fsync error, stale epoch/result or corrupt record: orphan objects confer no authority; recovery never silently creates empty ready state. |
| CP30 | Sustained workload violates queue/memory/FD/latency/cancellation/reuse bound while functional checks pass: candidate/hosted acceptance rejected. Measurements include package/environment/baseline and preserved negative cases. |
| CP31 | Desktop reads only published immutable facts: reading missing runtime does not create directories/locks, discover models, change refs, resume, pause, escalate, promote, supervise or invoke arbitrary shell. |
| CP32 | Telemetry records failures/repairs/reviews/acceptance with unknown cost null; model list/free suffix cannot auto-enroll or change safety policy. Disappearing model leaves safe availability state. |
| CP33 | Amendment adoption changes only exact allowed controls, keeps PLAN/STATE/versions/evidence/product unchanged and retains real old provenance. Forged/self-appointed release/marker or product/control mixed adoption fails. |
| CP34 | Controller/Linux/macOS supported filesystem/profile locks and persistence pass; unsupported native Windows/network filesystem refuses. No claims based on `fcntl` import alone. |
| CP35 | Every state/action pair is either a defined transition with prerequisite receipts or a typed refusal. CLI exit/result distinguishes rejection, availability, conflict and success; no successful no-op or generic retry loop. |
| CP36 | Promotion without task/authorization, with caller `{ok:true}`, forged/stale reviewer/test/guard/authority/allowance/packet, wrong base/candidate/NEXT, or dirty main is refused before push. No legacy export bypass. |
| CP37 | Two tasks/processes promoting to one real bare remote: integration lock serializes them and exact base permits one; second stops. Runtime lock is not held through network wait. |
| CP38 | Remote advances before fetch, between fetch/advertisement (including a candidate ancestor), and after advertisement/before server update: trusted pre-push old-OID check/receive-pack rejects unexpected state. No force flag or auto rebase. |
| CP39 | Real non-force remote push succeeds, then injected local ff-only failure/dirty main: REMOTE_PROMOTED/LOCAL_SYNC_PENDING persisted, remote SHA unchanged, authorization preserved; recovery syncs once without a second push. |
| CP40 | Kill controller before push, after remote commit before acknowledgement, after ack before receipt, after ff before record; post-push fetch unavailable; unexpected remote advance: durable intent reconciles exactly and no double push/authorization rollback. |
| CP41 | Handoff missing task ID, fake promotion record, stale tracking ref, wrong SHA/NEXT/main or dirty tree: supervisor not invoked. Positive handoff binds original task plus all receipts and frozen supervisor code. |
| CP42 | Exact-SHA hosted run/job/step/preview/class-artifact mismatch, expired artifact, skipped required case, synthetic echo with successful named step, host-rich dependency or wrong entitlement: completion refused and STATE stays NEXT. |
| CP43 | Supervisor crash before state write, after dirty evidence/state write, after completion commit, after remote push/ack uncertainty, before state-commit hygiene: reconcile preserved exact completion intent; no second completion commit or false ACCEPTED. |
| CP44 | Live small task through real installed OpenCode worker and independent reviewer in certified containers, actual controller check, full CLI state/receipts and bare-remote promotion. No external process fake in this case; no OR native application. |
| CP45 | Live OpenCode config precedence, model/variant, permissions, event-stream terminal/error behavior match pinned adapter; replace/upgraded CLI invalidates certification until reverified. |
| CP46 | Live read-only Codex architecture adapter returns strict factual decision, writes no source, launches no builds/tests/models; mock quota/error fixtures plus an authentic captured versioned error validate deferral without quota probing. |
| CP47 | Hosted trusted-harness receipt collection on an isolated control-plane fixture proves package identity/cases/measurements, GitHub provenance of the SHA-pinned reusable workflow, and package-process isolation from collector storage. Fake wrapper receipts, duplicate artifacts or omitted/changed pinned calls fail. Existing native OR verification remains on Actions; do not launch it locally. |
| CP48 | End-to-end independent source review traces prepare->claim->real worker->guards->required checks->full review->authorization->normal push->local failure recovery->task handoff->hosted receipt->state-only completion. Existing plan/policy/evidence/regression guards remain passing. |

Before production merge/activation, all mandatory cases must pass at their
declared boundary, with exact implementation SHA/runtime certification. Fake
adapters alone do not satisfy CP06/07/09/10/27/28/37–40/44–47. Unavailable tools,
quota, platform, or CI are unverified availability blockers, never skipped
passes. Do not spend architecture quota waiting on them; executor/controller
performs verification and reports factual blockers.

Required source checks include `git diff --check`, repository hygiene,
`scripts/check_execution_plan.py`, `scripts/check_architecture_policy.py`,
the unchanged existing `scripts/test_execution_infra.py` suite plus its newly
authorized tests, and the V2 suites above. Python syntax/static checks use the
repository's configured tooling; do not introduce a new linter just for V2.
No Rust/Flutter product suite is required for this architecture-only commit.
Future activation keeps all existing supervisor/hosted product gates.

## Specific executor prompt

Supply this template with immutable controller-generated M-phase contract. It
is a bounded implementation instruction, not permission to choose a phase:

```text
Implement only authorized phase <M0..M5> of model orchestrator V2.
Architecture authority: <reviewed architecture SHA>; disabled-build architecture-task
authorization: <controller-provided record>. Base: <exact SHA>. Branch and
workspace: <isolated controller-created locator>.

Read ADR 0009, automation README, V1_AUDIT, AMENDMENT_PROPOSAL,
IMPLEMENTATION_PLAN, V2_CONTRACT and the frozen task. Use only the exact phase
allowlist in that task. Preserve all frozen decisions and forbidden paths.
Do not copy/merge/cherry-pick V1, modify product code or PLAN/STATE/evidence,
change locked phases, start a product checkpoint, weaken assertions or retry
until green. No native OR launch or toolchain install.

Implement the actual CLI/production path and required CP cases for this phase;
helpers or fake adapters alone do not satisfy boundary tests. Required tests
run by the controller after your commit. Your report cannot assert authority,
test truth, acceptance, promotion, or DONE. Commit one internally consistent
change with relevant documentation/tests. Preserve dirty work if interrupted.

Stop on a contradiction, missing authority/isolation/harness/budget, uncertain
permission/lifetime, failing required gate, or exhausted corrective budget.
Report exact source/failure facts, competing hypotheses and discriminating
check. Do not invent a replacement architecture, change fidelity, rebase,
force push, promote, or invoke the supervisor. Controller owns all of those
authorization/evidence operations. Return candidate commit and factual blockers.
```

Mechanical models can implement schema validation and fixtures; qualified
implementation models can implement state/sandbox/verification/lifecycle/push
mechanics. Independent investigators review each phase's actual wiring and
failure cases. Architecture is invoked only if the frozen contract is
contradictory or impossible, with a concise repository-backed packet.

## Remaining activation inputs, not open design choices

No trust-boundary/state/promotion design question is delegated to executors.
Activation still needs operator adoption, verified existing sandbox/tool image
digests, actual CLI conformance, enrolled model/family/cost/reasoning data,
frozen required-test/acceptance catalog, and numerical product task budgets.
Those facts are not available from a prototype PASS report and were deliberately
not fabricated here. Missing inputs cause explicit blocked admission. Product
contracts requiring new scope/budgets beyond the locked specifications need a
separate architecture task. This candidate is frozen and stops at architecture.

## M0-R2 correction and independent audit gate

README §19 is normative for design authority, disabled build authorization and
operational adoption. Build/test work precedes operational adoption; M5 fixture
certification does not invoke the OR product or require an active V2 release.
The M0-R2 candidate must independently pass CP01–05/33 contract boundaries and
all lifecycle, real-Git authority, nested task and receipt negative matrices.
These M0 results do not certify later OS/CLI/hosted boundaries. An independent
subsequent audit decides admission to M1; this correction stops at M0-R2.

### M0-R3 closure gate

R3 is a corrective successor of R2, preserving its published history. It closes
CP01–05/33 contracts only: deterministic lifecycle derivation, independent
Git-pinned build/certification/adoption provenance, positive exact authority
references and pinned CHECKS phase/case ownership. README §19 specifies the
host bootstrap boundary. Synthetic pinned attestations are contract examples,
not M5 certification. The focused and existing infrastructure suites plus the
clean actual-candidate authority probe must pass; independent review decides
M1 admission. R3 itself authorizes no M1, product execution or full-auto.

## M3 admission amendment: phase-aware shared control primitives

Reproduced contradiction: `scripts/model_orchestrator/store.py` (`_task`
requires `checkpoint_id == M1`; `_authority` requires `M1` in the authorized
phases with an `M0` prefix), `scripts/model_orchestrator/sandbox.py`
(`_authority` requires `M1` build authorization/prerequisite semantics), and
`scripts/model_orchestrator/guards.py` (`Floor.verify` requires an authorized
`M2` with an `M0,M1` prefix and `checkpoint_id == M2`) prevent M3 admission:
an M1 task is refused by the M2 floor, an M2 task is refused by the store, and
an M3 task is refused by both, although the frozen design expects one
immutable task packet moving through shared primitives. The admission
semantics are corrected in README §19 (phase-aware shared control primitives);
this section grants the minimum implementation permission.

The amendment authorizes exactly:

- `docs/execution/automation/README.md`, `docs/execution/automation/IMPLEMENTATION_PLAN.md`
  (this amendment text; no other design-document change);
- `scripts/model_orchestrator/contracts.py` (one shared phase-admission helper
  next to the validated build authority);
- `scripts/model_orchestrator/store.py`, `scripts/model_orchestrator/sandbox.py`,
  `scripts/model_orchestrator/guards.py` (phase-aware correction of the shared
  admission checks only; no new capability, transition, or authority);
- `scripts/model_orchestrator/tests/test_contracts.py`,
  `scripts/model_orchestrator/tests/test_isolation_and_state.py`,
  `scripts/model_orchestrator/tests/test_verification.py` (regression and the
  permanent negative admission matrix where the correction requires it; the M2
  module-inventory exception is extended so each phase commit may list exactly
  the modules that phase's own allowlist lands, e.g. M3's `adapters.py`,
  `orchestrator.py`, `__main__.py` and M4's `promotion.py`, `push_guard.py`).

It is not permission to redesign V2, weaken task/authority binding, modify
product PLAN/STATE, or self-authorize runtime. Later M3/M4/M5 work reuses the
shared primitives under the amended rule and adds only its own phase files.
