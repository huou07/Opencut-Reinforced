# Deterministic task documentation

The entry point is AGENTS.md. Documentation selection is read-only and grants
no execution, build, adoption, certification, or promotion authority.

## Authority map

| Fact | One canonical authority |
| --- | --- |
| Permanent repository behavior, repair and safety rules | [AGENTS.md](../AGENTS.md) |
| Current execution state | [STATE.json](execution/STATE.json) |
| Checkpoint graph and dependencies | [PLAN.json](execution/PLAN.json) |
| Checkpoint implementation and acceptance contract | Its locked [phase specification](execution/phases) |
| Global architecture invariants | [ARCHITECTURE_INVARIANTS.md](execution/ARCHITECTURE_INVARIANTS.md) |
| Detailed feature architecture | Exact technical-plan/architecture section selected by [DOC_ROUTING.json](DOC_ROUTING.json) |
| Product scope | [PRODUCT.md](PRODUCT.md), relevant section on demand |
| Presentation | [DESIGN.md](../DESIGN.md); [UX_ACCEPTANCE.md](UX_ACCEPTANCE.md) owns interaction acceptance |
| Feature verification strategy/coverage reference | Exact [TESTING.md](TESTING.md) section; locked phase/evidence policy governs required proof |
| Historical decision rationale | [ADRs](adr/README.md) |
| Completion evidence and its policy | [Evidence records](execution/evidence/README.md), governed by [EVIDENCE_POLICY.json](execution/EVIDENCE_POLICY.json) |
| Frozen V2 architecture and machine contracts | Original [automation inventory](execution/automation/IMPLEMENTATION_PLAN.md) |
| Documentation selection and size guidance | DOC_ROUTING.json |

Summaries and historical implementation descriptions never override those
sources. A link is discovery, not a second maintained contract. Detect a
conflict before editing; do not silently resolve architecture contradictions.

## Three tiers

CORE loads permanent agent rules and the global product-acceptance invariants.
FEATURE adds only named canonical sections or small feature documents.
ON_DEMAND includes ADR/history, detailed evidence, broad product/architecture
references, full machine schemas, and deep investigation material. Manifest
`on_demand` lists explicit entry points, not an exhaustive archive inventory;
links can reach other reference documents when needed. A linked file is never
recursively injected into a context bundle.

The existing large technical/testing references remain canonical at their
original paths: V2 inventories, locked phases, and historical authority manifests
bind these surfaces. Routing exact headings decomposes their **read context**
without relocating contracts, invalidating historical evidence, or creating a
second copy. See [the baseline audit](reference/DOCUMENTATION_AUDIT.md).
New feature docs belong under [features](features/README.md) only when they
need their own durable home. No empty feature directories or redundant copies.

## Commands

```sh
python3 scripts/execution_plan.py context 7F --json
python3 scripts/execution_plan.py context 7F --features viewer --features media-runtime --docs-text
python3 scripts/execution_plan.py docs --features ui --docs-text
python3 scripts/execution_plan.py docs --features release --json
python3 scripts/execution_plan.py docs --features model-orchestrator --features model-orchestrator-m1 --docs-text
python3 scripts/check_document_routing.py
python3 -m unittest scripts.test_document_routing -v
```

Checkpoint context includes the complete selected checkpoint section plus all
shared phase text, including stop conditions and handoff. Sibling sections are
excluded only when their exact IDs belong to the same specification in PLAN.
Missing/ambiguous checkpoint headings refuse. The full phase remains available
on demand. Defaults are
explicit phase/exception bundle names in the routing manifest; these mappings
supply reading suggestions and contain no dependencies, execution status, or
permission. Explicit `--features` replaces the default reading suggestion, but
never omits core, the checkpoint contract, or shared phase rules. Select all affected features; if
the task expands, resolve the additional bundle. A documentation-only or
separately authorized control task can use `docs` without a product checkpoint.
`docs` rejects a positional task description: selection uses exact keys, not
natural-language guessing. No LLM is involved.

Manifest version 1 has exact fields, unique document IDs and authority selectors,
feature references, phase/checkpoint references, on-demand entry points, and
byte guidance. Whole-file selectors use `path`; excerpts add an exact unique
`heading`. Excerpts include child headings until the next heading of equal or
higher level, ignoring fenced code. Paths are relative regular repository files;
traversal, symlink escape, duplicate JSON keys, unknown fields/versions/features,
missing headings/targets, and unknown checkpoint IDs refuse. Shared references
reuse one selector. Overlapping excerpts are merged without duplicated bytes.
Output includes exact source lines; excerpts are not summaries or new authority.

## Context guidance

Targets: core ≤20,000 bytes, feature ≤30,000 bytes, resolved context ≤50,000
bytes. The validator reports actual counts and dominant files above guidance.
These are warnings, not quality gates: never trim required contracts to satisfy
a number. Frozen V2 phase bundles may legitimately exceed guidance. Structural
routing errors, missing local inline-link targets, orphan feature Markdown,
and explicit mutable `Status: NEXT/PLANNED/DONE/IN_PROGRESS` declarations in
active routes fail repository hygiene. Historical/on-demand prose is not a
status authority. Broad references retain historical descriptions; the audit
identifies their stale material instead of reinterpreting frozen contracts.

Link checking covers local inline Markdown file/directory destinations under
root/docs, including URL decoding and repository containment. It does not fetch
external links or claim fragment/reference-style/fenced-example validation.
Budgets count UTF-8 source excerpts, not tokens or the JSON transport envelope.
The execution resolver and validator write no repository files, initialize no
runtime, and launch no worker/model. No database, embeddings, server, or new
orchestration framework is involved.

Durable workflow/tool/host/future-routing policy lives in
[DEVELOPMENT_WORKFLOW.md](DEVELOPMENT_WORKFLOW.md#task-context-and-tool-policy).
Documentation routes are not a V2 authority loader: later execution must still
pin and validate the complete Git manifest. The certified M1 Git object and
frozen automation files are retained; this documentation branch neither
re-certifies a new release nor authorizes M2.

## Narrative integrity policy

No global or active narrative maintains current mutable implementation truth.
README is a timeless entrypoint with a fixed purpose/direction/platform/navigation
outline, never a capability inventory. AGENTS, INDEX, DOCUMENTATION and workflow
text contain rules and lookup paths. PRODUCT labels specify intended scope.
Current facts follow STATE → locked specification → supervisor evidence →
implementation SHA/source; version constants and verified state own versions.

ARCHITECTURE, TECHNICAL_PLAN, TESTING and TOOLING retain explicitly baseline-bound
historical implementation/coverage notes at their original paths. Each selected
section carries the historical notice so excerpt routing cannot drop provenance.
Frozen contracts, ADRs and evidence remain historical/on demand and unmoved.
Do not update their historical claims to imitate current status.

The validator enforces the README outline, historical notices on mixed references,
and narrow regression patterns for observed stale claims on active entrypoints.
These are deterministic guards, not English semantic analysis; semantic review
is still required for new narrative text. No narrative may confer execution authority.
