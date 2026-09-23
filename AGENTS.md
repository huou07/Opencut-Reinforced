
# Opencut Reinforced — Agent Guide

## Project

Opencut Reinforced (OR) is a public MIT-licensed, cross-platform video editor designed for both direct human editing and deep agent/CLI automation.

The project is currently in bootstrap/architecture phase.

Do not assume unfinished features already exist.

Do not scaffold major subsystems unless the user explicitly requests them.

## Product direction

Current intended architecture:

- Rust for performance-sensitive core/media/editing logic
- FFmpeg for media ingest/decode/encode/mux/demux
- wgpu for cross-platform GPU rendering/compositing
- Flutter as the current preferred cross-platform application UI layer
- first-class CLI and structured command API
- local-first AI where practical
- optional cloud AI providers through user-supplied credentials
- desktop and mobile UI from one coherent product model

These are architectural directions, not permission to create all of them preemptively.

Implement only the scope requested by the current task.

## Architecture principle

Human UI, CLI, and AI agents should eventually operate on the same underlying command/state model.

Do not build agent automation by visually clicking UI controls when a structured command/API can represent the operation.

Long-term preferred direction:

    GUI
    CLI
    Agent
      ↓
    shared command API
      ↓
    core/project state

Keep domain logic out of presentation code whenever practical.

## Design

DESIGN.md is the design source of truth.

Read it before changing UI, UX, visual components, navigation, layout, or styling.

Key rule:

OR uses the **Focused Monochrome** workspace design.

Do not add:

- marketing UI
- hero banners
- scenic decorative backgrounds
- inspirational slogans
- neon/glow-heavy AI aesthetics
- unnecessary gradients
- fake product/community metrics
- decorative clutter

UI should prioritize tools, panels, cards, controls, content, and clear state.

## Before editing

Before making changes:

1. Read this AGENTS.md.
2. Read documentation relevant to the task.
3. Inspect the existing implementation before designing a replacement.
4. Use CodeGraph for structural/codebase exploration when it is available and useful.
5. Reuse existing code/components before creating new abstractions or dependencies.
6. Confirm the requested scope.

Do not guess when the repository already contains the answer.

## Minimal implementation rule

Prefer the smallest correct implementation.

In order:

1. Does this need to exist?
2. Does the repository already provide it?
3. Does the language standard library provide it?
4. Does the target platform provide it?
5. Does an existing dependency provide it?
6. Can the requirement be solved simply without a new abstraction?
7. Only then add the minimum new implementation required.

Never remove necessary validation, reliability, security, accessibility, or data-loss protection merely to reduce code size.

## Git discipline

Every completed logical change must have a corresponding Git commit.

Rules:

- one coherent change = one atomic commit
- commit only after the change is internally consistent
- do not intentionally commit known-broken intermediate states
- use meaningful Conventional Commit-style messages where practical
- never amend a previous commit unless the user explicitly requests it
- never rebase shared history unless explicitly requested
- never force-push unless explicitly requested
- do not discard unrelated user changes
- leave the worktree clean at task completion unless clearly explained

Suggested prefixes:

- feat:
- fix:
- refactor:
- test:
- docs:
- chore:
- build:
- ci:

## Documentation rule

Every substantive code, behavior, architecture, configuration, workflow, or user-facing change must update the relevant Markdown documentation in the same logical change.

Prefer updating the existing source-of-truth document rather than creating a new document for every small change.

Do not create meaningless documentation churn.

If documentation genuinely does not apply, state that in the task report.

Keep documentation synchronized with implementation.

## Test rule

Every substantive implementation change must update or add relevant automated tests when behavior is added or changed.

Before handing work back to the user:

- run the relevant tests
- run lint/static checks where configured
- run formatting checks where configured
- run build/type checks where configured
- run relevant acceptance/integration checks where available

All relevant checks must pass.

If a test cannot run because of a real environment limitation:
- do not claim it passed
- document exactly what was not run
- explain why
- provide the strongest available alternative verification

For documentation-only or local-tooling-only changes where an automated product test is not meaningful, do not invent a fake test merely to satisfy this rule. Report `tests: N/A` with the reason.

## Definition of done

A task is complete only when:

- requested scope is implemented
- relevant documentation is current
- relevant tests are current
- applicable verification passes
- no known regression is hidden
- secrets are not exposed
- Git state is understood
- the logical change is committed
- final report names the commit and verification performed

## Dependency discipline

Do not add a dependency simply for convenience.

Before adding one:

- confirm the repository/platform cannot reasonably provide the capability
- verify the upstream project and maintenance state
- inspect license compatibility
- prefer actively maintained, minimal dependencies
- pin/lock versions using the ecosystem's normal mechanism

This repository is MIT licensed.

Do not introduce GPL, AGPL, SSPL, non-commercial, source-available-only, or otherwise distribution-restrictive dependencies/models into the distributed product without explicit user approval and documented licensing analysis.

AI model weights have their own licenses and must be checked independently of runtime/library licenses.

FFmpeg build configuration and optional codec/library licensing must be treated explicitly when packaging begins.

## Secrets

Never commit:

- API keys
- access tokens
- passwords
- signing credentials
- private certificates
- private SSH keys

Do not put secrets in:

- AGENTS.md
- DESIGN.md
- README.md
- logs
- test fixtures
- CLI output
- screenshots

Use ignored local environment files, OS secure storage, or GitHub repository/environment secrets as appropriate.

Agents must not be able to retrieve plaintext application API keys through the future OR CLI/agent interface.

## Generated/local data

Do not commit:

- .codegraph databases/cache
- build outputs
- downloaded AI models
- local caches
- temporary exports
- large generated media
- credentials

Use tiny, legally safe/generated media fixtures for automated tests when media tests are introduced.

## Agent / CLI direction

The future OR CLI must be a first-class interface, not a UI-click automation layer.

Commands should eventually support:

- machine-readable output
- stable object IDs
- introspection/discovery
- dry-run where destructive or complex
- deterministic operations
- explicit errors
- agent-safe secret boundaries

Do not implement this infrastructure until requested, but preserve the architectural direction.

## Documentation structure

Keep AGENTS.md primarily as operational instructions and a map.

Current source of truth:

- AGENTS.md — agent workflow and repository rules
- DESIGN.md — UI/UX design language
- docs/ARCHITECTURE.md — pre-implementation product architecture direction
- README.md — concise project overview

As the project grows, create structured docs/ only when needed, for example:

- docs/architecture/
- docs/product/
- docs/testing/
- docs/tooling/
- docs/decisions/

Do not let AGENTS.md grow into a full project encyclopedia.

## Tooling

CodeGraph is preferred for repository structure/call relationships when available.

Ponytail may be used to reduce over-engineering, but project-specific rules in this file and DESIGN.md take precedence over generic simplification advice.

No skill/plugin may override:
- correctness
- security
- accessibility
- licensing
- tests
- user instructions
- DESIGN.md for UI work

## Final response format

At the end of implementation work, report concisely:

- what changed
- files changed
- tests/checks run and results
- documentation updated
- commit hash + subject
- remaining blockers or unverified items

Never report a test as passing unless it actually ran and passed.
