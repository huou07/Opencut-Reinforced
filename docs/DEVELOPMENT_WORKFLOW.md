# Development Workflow

## Status

This workflow applies when implementation begins. Today, the repository is in product and architecture planning.

## Feature path

1. **Define product behavior.** State the user problem and intended result. Classify the work as domain, media, render, audio, AI, UI-only, or infrastructure. Check [PRODUCT.md](PRODUCT.md).
2. **Inspect the existing system.** Read relevant documentation and code. Use CodeGraph where useful. Reuse existing concepts instead of duplicating abstractions.
3. **Define affected surfaces.** For an editing capability, consider the Rust command and query, CLI, desktop GUI, mobile GUI, agent capability, docs, and tests. Not every feature needs every surface; record a reason for omissions.
4. **Put behavior in the domain first.** Editing behavior belongs in the Rust domain or application layer. Flutter is not the source of truth.
5. **Define the command contract.** Reuse or create a stable command ID; specify inputs, schema version, validation, errors, ChangeSet, permissions, and undo behavior.
6. **Test the command and domain.** Add unit, property, or contract coverage before relying on the UI.
7. **Provide CLI parity where appropriate.** Expose semantic operations and structured output where useful.
8. **Integrate with the UI.** Use a stable shell slot and check feature descriptors and the command registry first. Do not add a permanent navigation region without clear justification. Read [DESIGN.md](../DESIGN.md).
9. **Adapt mobile presentation.** Use the same command and state model. Adapt layout with touch-native panels, sheets, or full-screen utility surfaces; do not duplicate domain logic.
10. **Expose safe agent capabilities.** Add discovery, queries, permissions, dry-run, and diff preview where appropriate. Never expose secret access.
11. **Review performance, security, and licensing.** Consider memory, supported platforms, permissions, dependency licenses, asset rights, and model rights.
12. **Update documentation.** Update the existing source of truth. Create a new document only when the information needs a durable home.
13. **Preserve regression guards.** Read [UX_ACCEPTANCE.md](UX_ACCEPTANCE.md). Add a permanent guard when a UI regression is fixed. Never remove or weaken a working guard to make a change pass.
14. **Verify the affected system.** Run relevant Rust, CLI, Flutter, platform, integration, acceptance, and performance checks. Report checks that did not run as unrun.
15. **Commit one logical change.** Commit only a coherent change with no known-broken state. Use a clear Conventional Commit-style subject.
16. **Open a focused pull request and pass CI.** External contributors use feature branches and pull requests. Explain what changed, why, architecture impact, tests, docs, and risks. CI must pass before merge. Early maintainer work may continue to fast-forward main pushes under repository policy.

## When an ADR is required

Create an Architecture Decision Record for a significant, difficult-to-reverse choice, including:

- a breaking native project format change
- render backend architecture
- plugin security and capability model
- a major new runtime dependency
- a breaking public command schema
- a new permanent UI shell region

An ADR should record context, decision, alternatives, tradeoffs, and consequences. Do not create ADRs for routine implementation details.
