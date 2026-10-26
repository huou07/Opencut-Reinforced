# Documentation Index

Last reviewed: 2026-10-21

Use this map to load only the authority needed for the current task. The owner's
direct request defines the requested outcome and may authorize changes to these
project-local instructions. Durable product and subsystem contracts remain in
their source-of-truth documents.

## Authority map

1. [AGENT_POLICY.md](AGENT_POLICY.md) — durable project, data, architecture and
   evidence constraints.
2. [PRODUCT_ROADMAP.md](PRODUCT_ROADMAP.md) — current owner-selected scope,
   priorities and product traceability.
3. [ARCHITECTURE.md](ARCHITECTURE.md), [TECHNICAL_PLAN.md](TECHNICAL_PLAN.md),
   [PROJECT_FORMAT.md](PROJECT_FORMAT.md) and [ADRs](adr/README.md) — subsystem
   contracts and recorded architecture decisions.
4. [PRODUCT.md](PRODUCT.md) — product requirements and maturity labels.
5. [DESIGN.md](../DESIGN.md) and [UX_ACCEPTANCE.md](UX_ACCEPTANCE.md) — visual
   language and interaction guards.
6. [QUALITY.md](QUALITY.md), [TESTING.md](TESTING.md) and [TOOLING.md](TOOLING.md)
   — relevant quality targets, check inventory and local procedures.

[AGENT-AUTHORITY.md](AGENT-AUTHORITY.md) records the local permissions for this
mission. Archived material is historical evidence, not a current work selector.

## Task routing

| Task | Read |
|---|---|
| Continue current product work | [AGENTS.md](../AGENTS.md), the current scope at the top of [PRODUCT_ROADMAP.md](PRODUCT_ROADMAP.md), then [DEVELOPMENT_WORKFLOW.md](DEVELOPMENT_WORKFLOW.md) |
| Editing command or project behavior | Relevant [PRODUCT.md](PRODUCT.md), [ARCHITECTURE.md](ARCHITECTURE.md), [PROJECT_FORMAT.md](PROJECT_FORMAT.md), workflow and skill or-edit-feature |
| User-visible UI or interaction | [DESIGN.md](../DESIGN.md), [UX_ACCEPTANCE.md](UX_ACCEPTANCE.md), relevant PRODUCT section and skill or-ui-ux |
| Media import, preview, audio or export | Relevant [ARCHITECTURE.md](ARCHITECTURE.md), [TECHNICAL_PLAN.md](TECHNICAL_PLAN.md), [QUALITY.md](QUALITY.md) section and skill or-media-render |
| Declarative code-to-video MotionScene | [ADR 0007](adr/0007-declarative-motion-scenes-and-procedural-isolation.md), relevant TECHNICAL_PLAN section and skill or-motionscene |
| Build, run or verify locally on this Mac | [AGENT-AUTHORITY.md](AGENT-AUTHORITY.md), relevant TOOLING/TESTING sections and skill or-dev-loop |
| Audit a hosted evidence claim | [evidence/claims/README.md](evidence/claims/README.md), relevant TESTING/QUALITY section and skill or-claim-evidence |
| Release or distribution work | [RELEASE.md](RELEASE.md), [DEVELOPER_PREVIEW_INSTALL.md](DEVELOPER_PREVIEW_INSTALL.md), [SECURITY_LICENSING.md](SECURITY_LICENSING.md) and skill or-release |
| Security, dependency or licensing review | [SECURITY.md](../SECURITY.md), [SECURITY_LICENSING.md](SECURITY_LICENSING.md), [OPEN_SOURCE_CONVERGENCE.md](OPEN_SOURCE_CONVERGENCE.md) |

## Where details live

- Operating map: [AGENTS.md](../AGENTS.md)
- Project-specific skill routes: [skills.md](skills.md)
- Contributor conventions: [CONTRIBUTING.md](../CONTRIBUTING.md)
- Prior plans, runs and receipts: [archive/](archive/execution/README.md) and
  [evidence](evidence/claims/README.md)
- Current mission brief: [prompts/OR-BUILD.md](prompts/OR-BUILD.md)

A document that states current status carries a review date and is checked by
scripts/check_doc_freshness.sh. Update that checker when adding a new
current-status document. Historical records retain their original dates.
