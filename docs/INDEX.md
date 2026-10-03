# Documentation Index

This map points to the current source of truth. Read the documents relevant to your task; there is no need to read every file for every change.

## Project and contributor entry points

| Document | Purpose |
| --- | --- |
| [README.md](../README.md) | Public project overview, status, product direction, platforms, and links |
| [AGENTS.md](../AGENTS.md) | Repository rules, safety constraints, and agent document routing |
| [DESIGN.md](../DESIGN.md) | Approved OR Focused Monochrome application design language |
| [CONTRIBUTING.md](../CONTRIBUTING.md) | Contributor workflow and pull request expectations |
| [SECURITY.md](../SECURITY.md) | How to report a repository vulnerability |

## Product and engineering references

| Document | Purpose |
| --- | --- |
| [PRODUCT.md](PRODUCT.md) | Full product vision, feature scope, and maturity labels |
| [ARCHITECTURE.md](ARCHITECTURE.md) | High-level architecture reference; execution status comes from STATE.json |
| [TECHNICAL_PLAN.md](TECHNICAL_PLAN.md) | Detailed subsystem boundaries, data contracts, and open choices |
| [DEVELOPMENT_WORKFLOW.md](DEVELOPMENT_WORKFLOW.md) | Required path for defining and implementing a feature |
| [ROADMAP.md](ROADMAP.md) | Phased plan, dependencies, and MVP boundary |
| [TESTING.md](TESTING.md) | Canonical testing sections, historical coverage, and reporting rules |
| [SECURITY_LICENSING.md](SECURITY_LICENSING.md) | Product trust boundaries, dependency and content licenses |
| [RELEASE.md](RELEASE.md) | Future application release process; no app binaries are released today |
| [TOOLING.md](TOOLING.md) | Local development baseline, hosted platform verification, optional tools, and repository safeguards |
| [UX_ACCEPTANCE.md](UX_ACCEPTANCE.md) | Preserved prototype behavior and future UI regression guards |
| [execution/README.md](execution/README.md) | Machine-readable architecture lock, checkpoint graph, and execution contract |
| [adr/README.md](adr/README.md) | Architecture decision records |
| [execution/automation/README.md](execution/automation/README.md) | Frozen V2 model-orchestration candidate: trust, verification, recovery, and adoption requirements; not activated |

## Deterministic task routing

Use [DOC_ROUTING.json](DOC_ROUTING.json) through the existing execution resolver:
`python3 scripts/execution_plan.py context <checkpoint-id>` or
`python3 scripts/execution_plan.py docs --features <key> --docs-text`.
See [DOCUMENTATION.md](DOCUMENTATION.md) for the authority map, feature keys,
section selection, guidance budgets, and on-demand references. Full engineering
references above are available for investigation, not always-loaded context.
[The audit](reference/DOCUMENTATION_AUDIT.md) records legacy duplication and
frozen documents retained rather than relocated.
