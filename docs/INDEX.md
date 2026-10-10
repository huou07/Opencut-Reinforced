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
| [ARCHITECTURE.md](ARCHITECTURE.md) | Canonical high-level architecture and current implementation status |
| [TECHNICAL_PLAN.md](TECHNICAL_PLAN.md) | Detailed subsystem boundaries, data contracts, and open choices |
| [DEVELOPMENT_WORKFLOW.md](DEVELOPMENT_WORKFLOW.md) | Required path for defining and implementing a feature |
| [ROADMAP.md](ROADMAP.md) | Phased plan, dependencies, and MVP boundary |
| [PRODUCT_ROADMAP.md](PRODUCT_ROADMAP.md) | Active outcome-based product roadmap and legacy requirement mapping |
| [evidence/PRODUCT_ROADMAP_HISTORY.md](evidence/PRODUCT_ROADMAP_HISTORY.md) | Archived exact-run product journey investigations; historical status snapshots only |
| [OPEN_SOURCE_CONVERGENCE.md](OPEN_SOURCE_CONVERGENCE.md) | Evidence-backed reuse/build/upstream decisions and license boundaries |
| [TESTING.md](TESTING.md) | Current bootstrap checks, planned verification layers, and reporting rules |
| [SECURITY_LICENSING.md](SECURITY_LICENSING.md) | Product trust boundaries, dependency and content licenses |
| [RELEASE.md](RELEASE.md) | Future application release process; no app binaries are released today |
| [TOOLING.md](TOOLING.md) | Local development baseline, hosted platform verification, optional tools, and repository safeguards |
| [UX_ACCEPTANCE.md](UX_ACCEPTANCE.md) | Preserved prototype behavior and future UI regression guards |
| [execution/README.md](execution/README.md) | Machine-readable architecture lock, checkpoint graph, and execution contract |
| [adr/README.md](adr/README.md) | Architecture decision records |

## Suggested task routing

- **UI work:** AGENTS.md, DESIGN.md, UX_ACCEPTANCE.md, and the relevant PRODUCT.md section.
- **Rust or domain work:** AGENTS.md, ARCHITECTURE.md, TECHNICAL_PLAN.md, and DEVELOPMENT_WORKFLOW.md.
- **AI work:** AGENTS.md, ARCHITECTURE.md, TECHNICAL_PLAN.md, and SECURITY_LICENSING.md.
- **Release work:** AGENTS.md, RELEASE.md, TESTING.md, and TOOLING.md.
- **Contributors:** README.md, CONTRIBUTING.md, and DEVELOPMENT_WORKFLOW.md.
