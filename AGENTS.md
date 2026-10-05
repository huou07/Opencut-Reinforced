
# Opencut Reinforced — Agent Guide

## Project

Opencut Reinforced (OR) is a public MIT-licensed, cross-platform video editor designed for both direct human editing and deep agent/CLI automation.

The repository execution state is maintained in `docs/execution/STATE.json` and
selected by `docs/execution/PLAN.json`. Do not maintain current-phase status in
this file.

Do not assume unfinished features already exist.

Do not scaffold major subsystems unless the user explicitly requests them.

## Engineering principles

Understand before editing. Plan enough to avoid obvious mistakes, then act.
Work smart, not merely hard. Persistent, not stubborn. Finish the outcome, not
the attempt. Evidence beats claims. Complexity has to earn its keep.

[docs/ENGINEERING_PRINCIPLES.md](docs/ENGINEERING_PRINCIPLES.md) is the detailed
durable policy: memory model, evidence and repair discipline, untrusted input,
anti-slop, context/quota lifecycle, delegation, and what does or does not block
work. It is the `engineering` routing bundle
(`python3 scripts/execution_plan.py docs --features engineering`). Read it
before substantial multi-step implementation or debugging work. It never
overrides the OR-specific authority named in this file.

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

Project mutations must go through the documented command/application path; clients and workers must not directly mutate canonical project state.

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

Canonical future motion content is declarative and non-executable. Arbitrary
HTML/CSS/JS/Canvas/WebGL/WebGPU motion belongs only behind the future explicit
sandboxed procedural boundary; it is not normal project state or the canonical
renderer.

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

Before choosing work, run:

    python3 scripts/execution_plan.py status
    python3 scripts/execution_plan.py context <checkpoint-id>

When the user says “continue”, execute only the checkpoint currently marked
`NEXT`. When the user names a phase, follow the machine plan and its locked
phase specification. When the user says “finish desktop MVP”, follow the
`desktop-mvp` milestone graph. If no external fresh-process supervisor is
active, execute one checkpoint and stop rather than chaining checkpoints in a
single model context.

The execution source-of-truth order is:

1. This file for permanent repository behavior, safety, and execution rules.
2. `docs/execution/PLAN.json` for the immutable checkpoint graph.
3. `docs/execution/STATE.json` for mutable `DONE`, `NEXT`, and `PLANNED` state.
4. `docs/execution/phases/*.md` for locked checkpoint contracts.
5. `docs/ARCHITECTURE.md` for human-readable architecture.
6. `docs/TECHNICAL_PLAN.md` for detailed subsystem design.
7. `docs/PRODUCT.md` for product capability scope.
8. `docs/ROADMAP.md` for historical/human-readable roadmap context.
9. `DESIGN.md` and `docs/UX_ACCEPTANCE.md` for presentation and UX truth.

Feature agents may not edit the locked checkpoint specification or permanent
architecture invariants. If repository reality conflicts with either, stop and
report the exact conflict. Plan amendments require a separate architecture
plan task.

### Execution evidence lock

Normal checkpoint runners implement exactly one resolved `NEXT` checkpoint and
may push implementation commits only. They must not edit `PLAN.json`,
`STATE.json`, `EVIDENCE_POLICY.json`, phase specifications, architecture
invariants or policy, execution validators/supervisor, protected workflows, or
completion evidence. A runner handoff must say `IMPLEMENTED — AWAITING
SUPERVISOR EVIDENCE`; it must never claim repository-authoritative `DONE`.

The supervisor independently verifies the implementation SHA against the
required hosted workflow runs and jobs, verifies a Developer Preview when the
plan requires one, writes the evidence record, advances `STATE.json`, and
pushes the state/evidence-only completion commit. A model statement is not
completion evidence. Use `scripts/agent_supervisor.py --resume-sha` only for
an already-pushed exact SHA that still matches the current `NEXT` state.

### Product acceptance and repair discipline

The permanent `INV-PRODUCT`, `INV-PACKAGE`, `INV-CAPABILITY`, `INV-PARITY`,
`INV-PERF`, `INV-UX`, `INV-ACCEPT`, and `INV-VERIFY` rules in
`docs/execution/ARCHITECTURE_INVARIANTS.md` apply to every checkpoint. A green
unit, bridge, or synthetic test cannot replace a feasible real product journey.
Packaged capabilities must be tested with packaged dependencies and the
permission model that users receive. Do not weaken a failed acceptance test
or silently change its evidence class.

Repairs follow the root-cause discipline in
[docs/ENGINEERING_PRINCIPLES.md](docs/ENGINEERING_PRINCIPLES.md) §6: after at
most two speculative attempts against one subsystem or gate, stop and diagnose.
Record measurements for realtime performance or resource claims.

### Execution discipline

Difficulty, length, complexity, inconvenience, computational expense within
authorized limits, or estimates of hours/days/weeks never justify reducing,
deferring, or abandoning an authorized task. Attempt the complete authorized
scope now and continue until it is achieved or a concrete legitimate blocker is
observed. Decompose only for dependencies, atomic/reviewable commits, crash
recovery, isolation, independent verification, genuine parallelism, or bounded
destructive operations.

Existing checkpoint boundaries and explicit token/cost/resource/security limits
still apply. Fail closed for missing authority; genuinely unavailable
credentials, resources, hardware, tools, quota or platforms; destructive work
beyond authorization; proven frozen-architecture contradictions; security,
legal or safety boundaries; or required evidence impossible in the authorized
environment.

For every new persistent/mutable object or state boundary, verify its
interaction with existing invariants, including ownership, durable identity,
lifecycle, crash behavior, recovery, isolation, cleanup conditions and
authority semantics. Local acceptance alone is insufficient.

## Model orchestrator V2 (proposed, disabled)

ADR 0009 and `docs/execution/automation/` freeze a candidate control-plane
design for isolated multi-model development. It is **proposed, not adopted**:
the architecture freeze alone confers no execution authority and no full-auto
eligibility.

- `docs/execution/automation/PROTOCOL_SCHEMAS.json` defines the adoption
  lifecycle and distinguishes ARCHITECTURE_FROZEN, AMENDMENT_PROPOSED,
  BUILD_AUTHORIZED_DISABLED, disabled implementation progress, pre-adoption
  certification and externally pinned operational adoption.
- Activation requires the separate control-plane amendment, every M0–M5
  implementation phase, and the full CP01–CP48 acceptance suite; until all of
  those hold, `full_auto_eligible` is false and `scripts/model_orchestrator/`
  must not dispatch product workers. Explicitly authorized disabled fixture
  certification never grants production authority.
- Adoption changes only the exact control inventory. It may not change
  `PLAN.json`, `STATE.json`, product source, contract versions, or existing
  evidence, and it requires an externally operator-pinned release SHA.
- V1 history (`control/model-orchestrator-v1`) and the preserved
  `wip/9b-candidate-beaef` product candidate are audit inputs only. Neither
  confers V2 authority or may be imported implicitly.

## Minimal implementation rule

Prefer the smallest correct implementation: does this need to exist at all, does
the repository already provide it, does the language standard library or the
target platform provide it, does an existing dependency provide it, can it be one
line — and only then add the minimum new implementation required.

Never remove necessary validation, reliability, security, accessibility, or data-loss protection merely to reduce code size.

## Git discipline

Every completed logical change must have a corresponding Git commit:

- one coherent change = one atomic commit
- commit only after the change is internally consistent
- do not intentionally commit known-broken intermediate states
- use meaningful Conventional Commit-style messages where practical
- never amend a previous commit unless the user explicitly requests it
- never rebase shared history unless explicitly requested
- never force-push unless explicitly requested
- do not discard unrelated user changes
- leave the worktree clean at task completion unless clearly explained

## Documentation rule

Every substantive code, behavior, architecture, configuration, workflow, or user-facing change must update the relevant Markdown documentation in the same logical change.

Prefer updating the existing source-of-truth document rather than creating a new document for every small change.

Do not create meaningless documentation churn.

If documentation genuinely does not apply, state that in the task report.

Keep documentation synchronized with implementation.

Global and active narrative documents must not maintain mutable implementation
truth independently: capability availability/maturity, current schema/protocol
versions, phase/checkpoint status, and release/runtime availability belong to
STATE → locked specification → supervisor evidence → implementation SHA/source.
Entrypoints remain timeless. Explicit baseline history and frozen historical
contracts may retain old observations; every routed historical excerpt must keep
its provenance notice. Preserve historical regression guards without turning
those lists into current-status inventories. See [docs/DOCUMENTATION.md](docs/DOCUMENTATION.md).
## Test rule

Every substantive implementation change must update or add relevant automated
tests when behavior is added or changed. Before handing work back, run the
relevant tests plus the lint/static, formatting, build/type and
acceptance/integration checks that are configured. All relevant checks must
pass.

If a test cannot run because of a real environment limitation, do not claim it
passed: document exactly what was not run, explain why, and provide the
strongest available alternative verification.

For documentation-only or local-tooling-only changes where an automated product
test is not meaningful, do not invent a fake test merely to satisfy this rule.
Report `tests: N/A` with the reason.

## CI-first platform verification

Use local machines for editing and headless source checks; GitHub Actions is canonical for native/runtime verification. Agents MUST NOT launch the native OR application, a platform emulator or simulator, or an attached physical device locally for verification unless the user explicitly requests local runtime testing. This also forbids native Flutter integration tests that launch OR. Do not automatically open a built or downloaded Developer Preview. Missing local Xcode, Android SDK/JDK, CocoaPods, Windows, or Linux tooling is not a blocker when an equivalent required Actions job exists. Report an unavailable local check as `LOCAL ENVIRONMENT BLOCKED`, distinct from `PASS`, `FAIL`, and an intentional `NOT RUN`. Do not install, repair, select, or accept platform toolchains or licenses, or use `sudo` for platform setup, unless the user explicitly requests local platform development. See [docs/TOOLING.md](docs/TOOLING.md).

## Regression preservation rule

Do not regress behavior that has already been fixed or verified. Inspect the
current behavior and its tests or acceptance checks before changing a subsystem,
and preserve unrelated working behavior. A regression fix is incomplete until it
adds an automated test where practical, or a reproducible acceptance check for UI
behavior that cannot reasonably be unit-tested yet.

Keep recorded guards passing unless the user explicitly changes the requirement;
never remove or weaken a guard just to make a change pass. When refactoring
conflicts with known-good behavior, preserve behavior first and refactor
incrementally: **preserve → change one subsystem → verify → commit**, and rerun
checks for every touched subsystem before handoff.

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

Keep AGENTS.md concise: repository rules and a map, not a project encyclopedia. Use [docs/INDEX.md](docs/INDEX.md) for the authoritative documentation map. Prefer updating the existing source-of-truth document over creating unnecessary documents.

## Document routing

Always read this file. Resolve the checkpoint contract and relevant documentation
with `python3 scripts/execution_plan.py context <checkpoint-id>`; use repeatable
`--features <key>` for explicitly scoped feature bundles. For separately
authorized non-checkpoint work use `python3 scripts/execution_plan.py docs
--features <key>`. `--docs-text` emits canonical excerpts with source locations.
Unknown bundle keys refuse; routing never grants checkpoint/phase authority.

[docs/DOC_ROUTING.json](docs/DOC_ROUTING.json) defines CORE, FEATURE, and
ON_DEMAND selection. [docs/DOCUMENTATION.md](docs/DOCUMENTATION.md) explains the
authority map and budgets. Do not automatically read full technical/testing
plans, roadmap, PLAN, all ADRs, or evidence. Follow links on demand when the task
requires the detailed contract, trust boundary, or historical evidence.
UI tasks include `ui` (DESIGN and UX authority); release tasks use `release`;
control-plane tasks use `model-orchestrator` plus the authorized phase bundle.

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
