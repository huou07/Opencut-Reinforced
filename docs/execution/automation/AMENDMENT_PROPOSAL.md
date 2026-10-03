# Exact adoption amendment — proposed, not applied

This architecture freeze does **not** alter permanent invariants, PLAN, STATE,
evidence policy, phase specifications, supervisor code, protected workflows,
validators, or the existing amendment marker. ADR 0008 authorizes its recorded
9B quality amendment and guarded follow-ups, not an arbitrary new control-plane
system. V2 needs its own separate architecture/control-plane adoption task.

The current `_validate_amendment_baseline` only accepts a schema-1 marker with
five exact fields, 9B, parent implementation identity, original failed run
`37043830370`, retained state, and its hardcoded exact control-path set. New
orchestrator code/policy/ADR paths are absent from that set. Do not fabricate a
failure run, reuse the old marker unchanged, or claim this spec commit is an
activated trusted baseline. The plan validator only accepts evidence versions
1/2; this proposal intentionally preserves them and the 11 evidence classes.

## 1. Proposed permanent invariant text

Add the following to the Product acceptance section only in the separately
reviewed adoption amendment. Retain all existing invariant IDs and wording.

### INV-PRODUCT-VALUE — Real product capability is the completion unit

A user-visible checkpoint is complete only when its intended capability is
usable by real users through the actual production path under its supported
runtime, packaging, permission, persistence, and resource conditions. Passing
existing tests is necessary but never sufficient. Required evidence must name
the real user action, observable outcome, shipped capability and environment,
failure journey, and any performance or resource bound. Supporting static,
unit, synthetic, bridge, and development-runtime evidence cannot substitute
for a required product-boundary journey or packaged capability.

### INV-NO-SATISFICE — Acceptance preserves its measurement floor

An implementation is invalid if it obtains acceptance by weakening the
measurement or reducing product fidelity. Required assertions, cases, evidence
classes, environment constraints, permissions, dependency/package boundaries,
supported scope, and performance/resource bounds cannot be removed, skipped,
mocked, silently bypassed, or made non-fatal to obtain acceptance. Retries and
timeout increases require exact failure evidence, a falsifiable cause, a
discriminating test, and a causal repair explanation; they are not root-cause
repairs by themselves. Legitimate acceptance-contract changes require an
explicit separately authorized architecture amendment preserving the product
goal. A model verdict or skipped test is never completion evidence.

### INV-PERF-CORRECTNESS — Optimization is part of correctness

For realtime, media, rendering, input, export, and persistence paths,
production correctness includes bounded resources and predictable sustained
performance. Predictable severe blocking or latency, unbounded memory/queues,
unnecessary repeated reconstruction or avoidable copies with material cost,
resource leaks, failed cancellation or stale-work invalidation, or an
architecture demonstrably unsuitable for sustained supported use prevents
production completion even if functional assertions pass. Relevant checkpoints
declare measurable budgets, structural ownership/reuse constraints, workload
and environment, comparison baselines, and evidence. No worker may waive these
obligations as an optimization to be deferred.

These add durable names for the product principle while strengthening existing
INV-PRODUCT/PACKAGE/CAPABILITY/PERF/ACCEPT/VERIFY, INV-JOB-001 and related rules.
They neither reclassify historic DONE evidence nor retroactively certify Phase 8.

## 2. Protection and source authority amendment

Change supervisor protection to retain all current explicit paths/prefixes and
add these exact paths:

```text
DESIGN.md
docs/UX_ACCEPTANCE.md
docs/ARCHITECTURE.md
docs/TECHNICAL_PLAN.md
docs/PRODUCT.md
docs/SECURITY_LICENSING.md
docs/execution/AGENT_EXECUTION.md
docs/execution/PHASE_SPEC_TEMPLATE.md
opencode.json
opencode.jsonc
```

Add protected prefixes:

```text
scripts/model_orchestrator/
docs/execution/automation/
.opencode/
docs/adr/
```

Protection of a contract does not authorize edits. Ordinary feature documentation
uses existing implementation/user guides; if changing a protected product/design
contract is necessary, obtain a separate scoped architecture authorization.
Never make an affected product checkpoint implicitly able to grant that change.
Do not create PROTECTED_PATHS.json as another authority. The frozen supervisor
predicate, extended as above, is the sole protection source. Candidate changes
cannot reclassify their own paths. Freeze all other referenced policy/config and
test-harness inputs as well, including their imported/executed closure.

## 3. Supervisor and evidence amendment

Add the task-bound V2 resume interface to the existing supervisor, using explicit
trusted code location and candidate/main repository root. Required input is
task ID, checkpoint goal, promoted implementation SHA, controller runtime root
derived from bootstrap (never model path), authority and promotion receipt
digests. Validate original immutable task, state sequence, guards, executed test
receipts, independent review, remote-promotion receipt, clean aligned main, and
NEXT before hosted verification. Retain the current local workspace test gate;
if safely reused, its exact immutable receipt must establish the same command,
tree, environment and result. Do not silently remove or waive it.

The supervisor's actual candidate workspace test execution must also use the
isolated controller verifier. A trusted Python supervisor invoking candidate
Cargo build scripts/tests on the controller host would reintroduce arbitrary
candidate write authority over runtime memory. Keep host-side collection and
authorization separate from all candidate execution, including this retained
pre-host workspace test gate and completion repository checks that execute code.

In activated V2 mode, unrestricted `--runner`, receipt-less `--resume-sha`, and
receipt-less `--repair-resume-from` are refused. Prepare/preflight remain
read-only. Keep legacy historical validation fixtures; compatibility is for
reading old evidence and provenance, not for generating new unauthorized tasks.
Manual or imported candidates use an explicit controller admission path and
the same verification/review authorization. Repair tasks retain the original
failed SHA and invariant fingerprint; they cannot authorize product/control
mixing under the old maintenance bypass.

Extend EVIDENCE_POLICY with an `orchestration_v2` contract selecting mandatory
task-bound mode, approved adoption identity, acceptance harness/catalog digest
sources, and `control_plane_receipt` requirement for **new** completions under
that adopted release. Leave existing gate/workflow/job/step mappings, class
taxonomy, preview rules, token handling and historic records intact.

For schema-2 records generated under V2, add a required `control_plane_receipt`:

```text
schema_version = 1                 # nested receipt version; outer remains 2
task_id, checkpoint_id
task_contract_digest, authority_digest, adoption_manifest_digest
base_sha, promoted_implementation_sha, promotion_authorization_digest
remote_promotion_receipt_digest
guard_receipt_digest, verification_receipt_digests, reviewer_receipt_digest
acceptance_contract_digest, production_acceptance_receipts[]
```

Each production receipt includes required evidence class and case IDs, exact
run/job/attempt/step/artifact identity and digest, authority/harness SHA+digest,
package SHA+digest, capability/permission/environment identity, executed/skipped
case counts and IDs, measurement samples/bounds/baseline, observations and
result. All required cases execute and pass; no required skips or unverified
metric. Fetch facts/artifacts from GitHub and compare with trusted contract, not
model-provided success. Keep the existing schema-2 `evidence_classes` named-step
proofs unchanged. The same hosted step may satisfy several classes only when
its approved harness produces the corresponding observations.

Historic schema-1/2 records without a nested receipt are valid only before the
adoption boundary, established by immutable Git ancestry/adoption provenance,
never a caller's `legacy=true`. Existing DONE records are not rewritten. New
active V2 completion without receipt fails offline plan/evidence validation.
No new evidence version or checkpoint graph field is needed.

The hosted workflow must build the exact candidate, run the approved harness
from a separately materialized frozen authority, and let a trusted collector
associate observations with the job/package. Candidate code and tests run
without signing/publishing/controller write credentials. A small separate
trusted publishing step may publish verified artifacts under existing release
policy. Exact step names do not permit changing their semantic evidence class.
Only the minimum existing workflow changes needed for the frozen harness are
authorized; no new fake journey or non-fatal failure path.

Add exactly `.github/workflows/control-plane-acceptance.yml` as the immutable
reusable acceptance workflow. The manifest pins its reviewed workflow SHA and
catalog/harness digest; existing main-push wrappers call that literal SHA.
Verify GitHub run `referenced_workflows` and successful called-job provenance
as well as existing class proofs. Candidate-controlled artifact text or wrapper
steps cannot prove that the collector executed. Required package processes run
without write authority over collector/harness/receipt storage; certify that
actual boundary per platform. The approval may pin a reviewed workflow ancestor
alongside the final adoption SHA so no commit contains a reference to itself.

Add durable supervisor completion intent and recovery as defined in README §16.
Final state-only commit still changes exactly STATE and the checkpoint evidence;
its exact hygiene run is required before another task is admitted.

## 4. New adoption baseline, preserving old provenance

Introduce a schema-2 **control amendment marker** via a new explicitly approved
branch/commit. Update AMENDMENT_BASELINE and validators only in that task.
Proposed marker exact fields:

```text
schema_version = 2
kind = "model-orchestrator-v2-adoption"
checkpoint_id = "9B"              # only if still NEXT at actual adoption
parent_sha = <exact authoritative pre-adoption main>
architecture_spec_sha = <this reviewed frozen architecture commit>
authority_manifest_digest = <manifest excluding this marker's own digest>
changed_paths = <sorted exact adoption diff paths>
legacy_quality_amendment = {
  marker_sha: <original validated schema-1 marker commit>,
  prior_implementation_sha: "f2b737ba1ac9d7fbaf6c8cb25217d06b14aa9480",
  prior_failed_run_id: 37043830370
}
state_digest = <unchanged pre-adoption STATE bytes>
plan_digest = <unchanged pre-adoption PLAN bytes>
verified_contract_versions = {project_schema:7,recovery_schema:1,ipc_protocol:1}
```

The marker does not contain its own commit SHA; the operator pins the resulting
release SHA externally in controller bootstrap after review/acceptance. No
self-authored marker proves adoption. The manifest's exclusion avoids a hash
cycle; all marker bytes still belong to the externally pinned Git release.
No historical failed run is claimed to have run V2. Existing schema-1 markers
retain their original validation path in Git history.

Validators require operator-approved release, direct main ancestry, exact
control-only adoption diff, unchanged PLAN/STATE/checkpoint statuses/versions,
unchanged existing evidence, preserved legacy provenance, and the complete
control-plane acceptance suite. The next product task's resume baseline becomes
that validated adoption commit. A different current NEXT at adoption requires
a refreshed separate architecture amendment; workers may not reuse 9B fields.

The adoption changed-path allowlist is exactly the file inventory in
IMPLEMENTATION_PLAN's M0–M5 plus the four documentation index/link files in this
architecture commit. Product source/manifests/assets, completion evidence,
PLAN and STATE are forbidden. Any changed file outside that inventory stops
adoption. Intermediate V2 implementation stays on its architecture-derived
branch, disabled; no partially trusted live controller is published to main.
Normal subsequent product tasks cannot edit any newly protected control path.

## 5. Exact governance changes required

The adoption task may change only the listed control surfaces for their stated
purpose:

| Existing surface | Required change |
| --- | --- |
| AGENTS.md | Record proposed invariant IDs, freeze/control ownership, separate architecture-task admission; retain execution evidence lock and CI-first native rule |
| ARCHITECTURE_INVARIANTS.md | Add the three exact invariant clauses above; retain all old text |
| AGENT_EXECUTION.md | Document task-bound candidate/promotion/supervisor path, isolated workers, attempts and recovery; prevent legacy V2 bypass |
| AMENDMENT_BASELINE.json | Schema-2 adoption marker with real parent and preserved schema-1 provenance |
| architecture-policy.json | Add required V2 documents/source/contract checks; retain version ownership and core dependency prohibitions |
| EVIDENCE_POLICY.json | Enable V2 task/receipt contract; preserve existing classes, bindings and workflow requirements |
| PHASE_SPEC_TEMPLATE.md | Require explicit product/negative journey, production boundary, measurement floor and performance applicability; no existing locked phase edited |
| agent_supervisor.py | Protection additions, validated adoption baseline, mandatory receipts, frozen-root execution and completion recovery |
| execution_plan.py / execution_evidence.py | Adoption-bound nested receipt validation; current schema-1/2 history and graph stay valid |
| check_execution_plan.py / check_architecture_policy.py | Verify new control source/contract closure and adoption identity; preserve existing guards |
| test_execution_infra.py | Add negative authorization/adoption/completion-recovery tests without weakening existing scenarios |
| .github/workflows/repo-hygiene.yml | Run the bounded deterministic V2 acceptance subset; preserve fatal existing gates |
| .github/workflows/platform-verification.yml | Pinned isolated controller/adapter/harness acceptance and minimum class artifact collection; preserve all product assertions/jobs |
| .github/workflows/developer-preview.yml | Carry frozen packaged-acceptance harness and artifact identity only where required; preserve packaging/licensing/release obligations |
| .github/workflows/control-plane-acceptance.yml | New immutable reusable acceptance job with separate candidate execution/collector authority and exact-source/case/measurement receipts |
| docs/TESTING.md / docs/TOOLING.md | Explain V2 verification layers and operator sandbox prerequisite; no silent tool installation or native-local permission |

PLAN, STATE, all existing phases, product code and existing evidence stay byte-for-byte
unchanged in adoption. Task-specific numerical criteria for later product work
are approved task-template inputs; if locked product contracts must change,
that is a separate plan amendment, not this adoption task.

## 6. Build permission precedes operational adoption (M0-R2)

The explicit correction task authorizes building the disabled M0-R2 candidate
on its exact branch. Future M phases require separately scoped build permission.
This is BUILD_AUTHORIZED_DISABLED, not AMENDMENT_ADOPTED or runtime authority.
The final adoption marker is prepared and tested while disabled. External review
and operator pinning follow complete implementation and pre-adoption certification
of the exact final release; a candidate marker never grants its own authority.
The corrected release lifecycle and semantic/Git closure requirements in README
§19 supersede the old M0 adoption-before-implementation schema. No existing global
invariant, historical evidence or product checkpoint is weakened or advanced.
