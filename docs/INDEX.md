# Documentation index

Last reviewed: 2026-10-20

This file is the routing layer. Pick the row that matches your task, then read
the authority that owns the answer. There is no need to read every document.

## Authority order

Host/session rules and the user's request govern first. The following map names
responsibilities; a roadmap does not override architecture or UX acceptance.
Resolve conflicts in the owning authority and report them, rather than choosing
a lower standard:

1. [AGENT_POLICY.md](AGENT_POLICY.md) — permanent execution, safety, acceptance
   and delivery rules.
2. [PRODUCT_ROADMAP.md](PRODUCT_ROADMAP.md) — active product priorities and
   traceability.
3. [OPEN_SOURCE_CONVERGENCE.md](OPEN_SOURCE_CONVERGENCE.md) — current
   reuse/build/upstream decisions.
4. [ARCHITECTURE.md](ARCHITECTURE.md), [TECHNICAL_PLAN.md](TECHNICAL_PLAN.md),
   [PROJECT_FORMAT.md](PROJECT_FORMAT.md) and the [ADRs](adr/README.md) —
   architecture and subsystem contracts.
5. [PRODUCT.md](PRODUCT.md) — required product capabilities and maturity labels.
6. [docs/archive/](archive/execution/README.md) — historical traceability only.
   It answers "what was planned or verified, and when", never "what to do now".
7. [DESIGN.md](../DESIGN.md) and [UX_ACCEPTANCE.md](UX_ACCEPTANCE.md) —
   presentation and UX truth.

The operating loop is in DEVELOPMENT_WORKFLOW; QUALITY owns the quality/evidence
contract, TESTING owns check selection and human acceptance, TOOLING owns command
usage, RELEASE owns publication requirements. AGENTS is the concise entry map;
skills add narrow procedures, not duplicate authorities.

Host and session security boundaries in [AGENT-AUTHORITY.md](AGENT-AUTHORITY.md)
and the agent harness remain higher authority than anything in this repository.

## Task routing

| Your task | Read, in order |
|---|---|
| First contact with the repository | [README.md](../README.md), [AGENTS.md](../AGENTS.md), this file |
| Continue product work ("continue", "next task") | [AGENTS.md](../AGENTS.md), [PRODUCT_ROADMAP.md](PRODUCT_ROADMAP.md), the current work selection below, then [DEVELOPMENT_WORKFLOW.md](DEVELOPMENT_WORKFLOW.md) |
| Add or extend an editing capability | [AGENTS.md](../AGENTS.md), [PRODUCT.md](PRODUCT.md), [ARCHITECTURE.md](ARCHITECTURE.md), [DEVELOPMENT_WORKFLOW.md](DEVELOPMENT_WORKFLOW.md), skill `or-edit-feature` |
| UI, UX or design | [DESIGN.md](../DESIGN.md), [UX_ACCEPTANCE.md](UX_ACCEPTANCE.md), relevant [PRODUCT.md](PRODUCT.md) section |
| Rust, domain, project format or commands | [ARCHITECTURE.md](ARCHITECTURE.md), [TECHNICAL_PLAN.md](TECHNICAL_PLAN.md), [PROJECT_FORMAT.md](PROJECT_FORMAT.md) |
| Media, audio, render or export | [ARCHITECTURE.md](ARCHITECTURE.md), [TECHNICAL_PLAN.md](TECHNICAL_PLAN.md), [QUALITY.md](QUALITY.md) |
| AI, provider or model work | [TECHNICAL_PLAN.md](TECHNICAL_PLAN.md), [ADR 0005](adr/0005-ai-provider-boundary.md), [ADR 0007](adr/0007-declarative-motion-scenes-and-procedural-isolation.md), [SECURITY_LICENSING.md](SECURITY_LICENSING.md), skill `or-media-ai` |
| Declarative motion, explainer or MotionScene work | [ADR 0007](adr/0007-declarative-motion-scenes-and-procedural-isolation.md), [TECHNICAL_PLAN.md](TECHNICAL_PLAN.md), skill `or-media-ai` |
| Build and test locally on this host | [TOOLING.md](TOOLING.md), [TESTING.md](TESTING.md), [AGENT-AUTHORITY.md](AGENT-AUTHORITY.md), skill `or-dev-loop` |
| Release, packaging or distribution | [RELEASE.md](RELEASE.md), [DEVELOPER_PREVIEW_INSTALL.md](DEVELOPER_PREVIEW_INSTALL.md), [TOOLING.md](TOOLING.md), skill `or-release` |
| Evidence, claims or acceptance | [TESTING.md](TESTING.md), [QUALITY.md](QUALITY.md), [evidence/claims/README.md](evidence/claims/README.md), skill `or-claim-evidence` |
| Security, dependencies or licensing | [SECURITY.md](../SECURITY.md), [SECURITY_LICENSING.md](SECURITY_LICENSING.md), [OPEN_SOURCE_CONVERGENCE.md](OPEN_SOURCE_CONVERGENCE.md) |
| Contribution or pull request | [CONTRIBUTING.md](../CONTRIBUTING.md), [DEVELOPMENT_WORKFLOW.md](DEVELOPMENT_WORKFLOW.md) |
| Reconcile a past checkpoint claim | [archive/execution/README.md](archive/execution/README.md), then the matching receipt under [archive/execution/evidence/](archive/execution/evidence/README.md) |

## Current work selection

One prompt file is current. It is not a second roadmap; it selects and bounds
work the roadmap already names.

| Document | Purpose |
|---|---|
| [prompts/OR-BUILD.md](prompts/OR-BUILD.md) | **Current.** The single execution prompt for the whole program: owner intent, verified starting state, phases 0–5, boundaries, completion criteria and recovery |
| [prompts/README.md](prompts/README.md) | Prompt-artifact convention: what a prompt must contain, and how it is retired |
| [archive/prompts/](archive/prompts/OR-LOCAL-FINISH.md) | Archived prompts (OR-LOCAL-FINISH, CONTINUE-OR, FINISH-OR, NEXT-OR). Read for method, never for current facts |

## Documents by area

### Agent operating surface

| Document | Purpose |
|---|---|
| [AGENTS.md](../AGENTS.md) | Repository rules, product boundaries and the agent document map |
| [AGENT-AUTHORITY.md](AGENT-AUTHORITY.md) | What this host authorizes without asking, and what it does not |
| [AGENT_POLICY.md](AGENT_POLICY.md) | Permanent execution, evidence, design, dependency and delivery rules |
| [skills.md](skills.md) | Skill routing: OR project-local skills and the shared procedures they defer to |
| [.agents/skills/](../.agents/skills/) | Project-local skill packages (`or-edit-feature`, `or-dev-loop`, `or-media-ai`, `or-claim-evidence`, `or-release`) |

### Product

| Document | Purpose |
|---|---|
| [PRODUCT.md](PRODUCT.md) | Full product vision, requirement inventory and maturity labels |
| [PRODUCT_ROADMAP.md](PRODUCT_ROADMAP.md) | **Active** outcome roadmap, current focus and legacy requirement mapping |
| [UX_ACCEPTANCE.md](UX_ACCEPTANCE.md) | Preserved prototype behavior and permanent UI regression guards |
| [DESIGN.md](../DESIGN.md) | Approved OR Focused Monochrome design language |

### Architecture and contracts

| Document | Purpose |
|---|---|
| [ARCHITECTURE.md](ARCHITECTURE.md) | Canonical high-level architecture, boundaries and implementation status |
| [TECHNICAL_PLAN.md](TECHNICAL_PLAN.md) | Detailed subsystem boundaries, data contracts and open choices |
| [PROJECT_FORMAT.md](PROJECT_FORMAT.md) | The `.orproj` on-disk contract, version history and reader invariants |
| [adr/README.md](adr/README.md) | Architecture decision records and their status |

### Engineering and verification

| Document | Purpose |
|---|---|
| [DEVELOPMENT_WORKFLOW.md](DEVELOPMENT_WORKFLOW.md) | Required path from a feature definition to a merged pull request |
| [TESTING.md](TESTING.md) | Test and evidence layers, local lanes, report schemas and exit codes |
| [QUALITY.md](QUALITY.md) | OR's definition of "good": attributes, targets, enforcing gate and known gaps |
| [TOOLING.md](TOOLING.md) | Local development baseline, hosted verification and approved agent tooling |

### Platform, security, release

| Document | Purpose |
|---|---|
| [RELEASE.md](RELEASE.md) | Developer Preview distribution and stable-release requirements |
| [DEVELOPER_PREVIEW_INSTALL.md](DEVELOPER_PREVIEW_INSTALL.md) | Install, try, update and remove Developer Preview packages |
| [SECURITY.md](../SECURITY.md) | How to report a repository vulnerability |
| [SECURITY_LICENSING.md](SECURITY_LICENSING.md) | Product trust boundaries, dependency and content licences |
| [OPEN_SOURCE_CONVERGENCE.md](OPEN_SOURCE_CONVERGENCE.md) | Evidence-backed reuse/build/upstream decisions and licence boundaries |
| [CHANGELOG.md](../CHANGELOG.md) | Notable changes per release |
| [LICENSE](../LICENSE) | MIT licence text |

### Evidence and claims

| Document | Purpose |
|---|---|
| [evidence/claims/README.md](evidence/claims/README.md) | Claim records: what was verified, by which run, and what it does not establish |
| [evidence/2026-10-18-local-linux-lane.md](evidence/2026-10-18-local-linux-lane.md) | Local Linux lane runs, including the first full four-step PASS: commands, timings, hashes and limits |
| [evidence/2026-10-18-local-android-lane.md](evidence/2026-10-18-local-android-lane.md) | Local Android lane: the wiring repair, the staged cross-build, and the measured host BLOCKED that stops the guest drive |
| [evidence/2026-10-18-linux-deb-local-acceptance.md](evidence/2026-10-18-linux-deb-local-acceptance.md) | One local Linux `.deb` build, disposable-root install and packaged-journey run |
| [evidence/local-transcription-poc.md](evidence/local-transcription-poc.md) | Pinned whisper.cpp and model feasibility proof for local caption proposals |

### History and archive — never current work

| Document | Purpose |
|---|---|
| [ROADMAP.md](ROADMAP.md) | Original phased plan and provenance; superseded by PRODUCT_ROADMAP.md |
| [PAUSE_MIGRATION_HANDOFF.md](PAUSE_MIGRATION_HANDOFF.md) | Append-only pause and Debian-migration log; superseded facts are marked, not edited |
| [evidence/PRODUCT_ROADMAP_HISTORY.md](evidence/PRODUCT_ROADMAP_HISTORY.md) | Archived exact-run product journey investigations |
| [evidence/dont-giveup-soon-archived-skill.md](evidence/dont-giveup-soon-archived-skill.md) | Archived text of the retired project-local persistence skill |
| [archive/execution/README.md](archive/execution/README.md) | The paused checkpoint ledger, its receipts and its architecture lock |
| [archive/execution/ARCHITECTURE_INVARIANTS.md](archive/execution/ARCHITECTURE_INVARIANTS.md) | The permanent invariants the paused lock enforced; the rules still apply, the machinery does not run |
| [archive/prompts/](archive/prompts/CONTINUE-OR.md) | Archived dated handoffs |

## Two maps, one rule

`PRODUCT_ROADMAP.md` selects work. Everything under `archive/`, plus
`ROADMAP.md` and `PAUSE_MIGRATION_HANDOFF.md`, answers "what was planned or
verified, and when" — never "what to do now". A document that restates a
checkpoint or phase status as current is wrong by construction, because prose
drifts from receipts.

A document that states a current status carries a `Last reviewed:` date and is
checked by `scripts/check_doc_freshness.sh`. Adding a new current-claim document
means adding it to that script's in-scope list in the same change, or its
staleness is invisible to every gate.
