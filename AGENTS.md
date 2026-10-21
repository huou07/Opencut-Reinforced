# Opencut Reinforced — Agent Guide

Last reviewed: 2026-10-20

OR is an MIT-licensed video editor. Flutter owns presentation and interaction;
Rust owns canonical project/editing state. GUI, CLI and agents use the shared
validated command/application path. FFmpeg and the documented media/render
components retain their roles. Preserve project-format compatibility, licences,
security boundaries and release safeguards.

## Start and scope

1. Establish repository, branch, HEAD, worktrees and dirty changes. Preserve WIP;
   use one writer per repository and stop/record a previous writer before takeover.
2. Follow the user's named outcome. An answer, review, investigation, plan or
   prompt authorizes its artifact only: no edits, gates, commits or new agents.
   A continuation resumes the existing task before selecting new roadmap work.
3. Read only the authority relevant to that outcome in the map below. Inspect
   existing functionality before proposing a replacement. A roadmap or skill
   never expands the request's scope.
4. For implementation, use the user-journey loop in
   [DEVELOPMENT_WORKFLOW.md](docs/DEVELOPMENT_WORKFLOW.md). Choose proportionate
   checks; demonstrate affected user behavior in the actual target application.
5. Report source/build identity, observed outcome, commands, evidence and gaps.
   A command, widget test or screenshot proves only what it exercised.

## Authorities

[docs/INDEX.md](docs/INDEX.md) is the full router; each authority owns a different answer.

| Question | Authority |
|---|---|
| Execution scope, safety and acceptance restrictions | [AGENT_POLICY.md](docs/AGENT_POLICY.md) |
| Active product priorities and defect backlog | [PRODUCT_ROADMAP.md](docs/PRODUCT_ROADMAP.md) |
| Product requirements | [PRODUCT.md](docs/PRODUCT.md) |
| Architecture, commands, persistence and decisions | [ARCHITECTURE.md](docs/ARCHITECTURE.md), [TECHNICAL_PLAN.md](docs/TECHNICAL_PLAN.md), [PROJECT_FORMAT.md](docs/PROJECT_FORMAT.md), [ADRs](docs/adr/README.md) |
| Development procedure | [DEVELOPMENT_WORKFLOW.md](docs/DEVELOPMENT_WORKFLOW.md) |
| Quality contract and measurement limits | [QUALITY.md](docs/QUALITY.md) |
| Test selection, fixtures and human acceptance | [TESTING.md](docs/TESTING.md) |
| Tool commands and host permissions | [TOOLING.md](docs/TOOLING.md), [AGENT-AUTHORITY.md](docs/AGENT-AUTHORITY.md) |
| Focused Monochrome design and interaction acceptance | [DESIGN.md](DESIGN.md), [UX_ACCEPTANCE.md](docs/UX_ACCEPTANCE.md) |
| Dependencies, provenance and licences | [OPEN_SOURCE_CONVERGENCE.md](docs/OPEN_SOURCE_CONVERGENCE.md), [SECURITY_LICENSING.md](docs/SECURITY_LICENSING.md) |
| Hosted claims and release requirements | [claims](docs/evidence/claims/README.md), [RELEASE.md](docs/RELEASE.md) |
| Narrow project procedures and shared-skill relationship | [skills.md](docs/skills.md) |

## Verification and delivery

`bash scripts/check-repo.sh` is the source gate. `bash scripts/acceptance.sh`
adds the configured Rust/Flutter/performance lanes and writes
`.hub/gate-report.json`. Bounded local Linux/Android runtime tooling is described
in TOOLING; hosted `platform-verification` remains the acceptance authority.
Local or hosted success never substitutes for unexercised user journeys or human
usability evaluation. PASS, FAIL, BLOCKED and UNVERIFIED are distinct; required
skips block the corresponding claim. Historical evidence remains historical.

After two speculative repairs, require a falsifiable hypothesis and a
new discriminating observation before editing again. Never weaken acceptance
for green. Commit coherent verified infrastructure; preserve genuine diagnostic
product failures. Publishing, merging, tagging and deploying need explicit
separate authority. Never restart the archived checkpoint supervisor.

Author docs and skills in English; converse in the owner's language.
