
# Opencut Reinforced — Detailed Repository Policy

Purpose: retain permanent execution, acceptance, security and delivery rules.
Authority: detailed project policy referenced by the root AGENTS.md; host and
session security boundaries remain higher authority. Read task-relevant sections.
Last reviewed: 2026-10-18.

## Project

Opencut Reinforced (OR) is a public MIT-licensed, cross-platform video editor designed for both direct human editing and deep agent/CLI automation.

The legacy checkpoint ledger is archived at `docs/archive/execution/STATE.json` and
`PLAN.json`; it records historical requirement evidence and is not the active
work cursor after the 2026 open-source convergence pivot. Active product
priorities and status belong in `docs/PRODUCT_ROADMAP.md`, and completion claims
are backed by records under `docs/evidence/claims/`.

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

Project mutations must go through the documented command/application path; clients and workers must not directly mutate canonical project state.

Use the structured command/API for ordinary agent operations. To verify a UI
claim, exercise its real controls and assert the resulting shared project state;
a direct API call cannot establish UI behavior. Screenshots answer visual claims.

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

1. Read the repository AGENTS.md and the relevant sections of this policy.
2. Read documentation relevant to the task.
3. Inspect the existing implementation before designing a replacement.
4. Use CodeGraph for structural/codebase exploration when it is available and useful.
5. Reuse existing code/components before creating new abstractions or dependencies.
6. Confirm the requested scope.

Do not guess when the repository already contains the answer.

Before choosing work, read the active roadmap. Inspect the archived execution
ledger for relevant evidence and dependencies only when reconciling a past
checkpoint claim; it never selects the next product task.

When the user says “continue”, continue the highest-value incomplete capability
in `docs/PRODUCT_ROADMAP.md`. The old `NEXT` cursor is historical and must not
reactivate the paused checkpoint sequence. A bounded product delegation may
combine or reorder requirements when their real dependencies permit it;
roadmap labels are traceability, not mandatory prompt or commit boundaries.
Honor a named legacy requirement's complete product contract and preserve its
invariants. Continue without asking for a new prompt at each label.

The execution source-of-truth order is:

1. This file for permanent repository behavior, safety, and execution rules.
2. `docs/PRODUCT_ROADMAP.md` for active product priorities and traceability.
3. `docs/OPEN_SOURCE_CONVERGENCE.md` for current reuse/build/upstream decisions.
4. `docs/ARCHITECTURE.md` and `docs/TECHNICAL_PLAN.md` for architecture and
   subsystem contracts, amended only through authorized architecture decisions.
5. `docs/PRODUCT.md` for required product capabilities.
6. `docs/archive/execution/` — the preserved legacy checkpoint graph, phase
   specifications and their receipts. Archival: it answers "what was verified
   then", never "what to do now".
7. `DESIGN.md` and `docs/UX_ACCEPTANCE.md` for presentation and UX truth.

Feature agents may not edit the archived checkpoint specification or permanent
architecture invariants. If repository reality conflicts with either, stop and
report the exact conflict. Plan amendments require an explicitly authorized
control-plane task; this convergence pivot authorizes the active product
roadmap/architecture amendments but does not authorize weakening permanent
product, security, persistence, performance, or acceptance invariants.

### Execution evidence lock

Normal runners implement the exact requirement set authorized by the selected
goal. They may combine requirements into a coherent change, but may not bypass
real schema, security, persistence, platform, or API dependencies. They may
push implementation commits only. They must not edit the archived checkpoint
specifications, architecture invariants or policy, execution
validators/supervisor, protected workflows, or existing completion evidence.
A runner handoff must say `IMPLEMENTED — AWAITING VERIFICATION EVIDENCE`; it
must never claim repository-authoritative `DONE`.

A requirement becomes evidenced when a claim record under
`docs/evidence/claims/` binds the exact implementation SHA to the hosted run and
job that verified it, with a UTC timestamp. A model statement is not completion
evidence. Claim records for requirements a runner did not implement must not be
edited by that runner.

The archived supervisor remains the historical mechanism that produced the
pre-pivot receipts. It is paused and no longer advances state: `MISSION.json`
records `legacy_execution: PAUSED`. Read it when reconciling those receipts; do
not restart it and do not treat its `STATE.json` as current product status.

### Product acceptance and repair discipline

The permanent `INV-PRODUCT`, `INV-PACKAGE`, `INV-CAPABILITY`, `INV-PARITY`,
`INV-PERF`, `INV-UX`, `INV-ACCEPT`, and `INV-VERIFY` rules in
`docs/archive/execution/ARCHITECTURE_INVARIANTS.md` apply to every checkpoint. A green
unit, bridge, or synthetic test cannot replace a feasible real product journey.
Packaged capabilities must be tested with packaged dependencies and the
permission model that users receive. Do not weaken a failed acceptance test
or silently change its evidence class.

After at most two speculative corrective attempts for the same subsystem or
gate, stop and diagnose. Another repair commit requires exact failure
evidence, a falsifiable root-cause hypothesis, a reproduction or
discriminating test, and an explanation of why the patch fixes the cause.
Retries and timeouts alone are not a root-cause repair. Record measurements
for realtime performance or resource claims.

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

## Local runtime and platform verification

Local machines are for editing and headless source work; GitHub Actions is the
canonical verification environment for platform, linker, native builds,
packaged acceptance and cross-platform coverage.

Two bounded local runtime lanes are an owner-granted exception, recorded in
[AGENT-AUTHORITY.md](AGENT-AUTHORITY.md):

- the **Debian Linux developer preflight**, whose single entry point is
  `bash scripts/run_local_lane.sh linux`; a `--packaged` run adds the local
  `.deb` build and the packaged journey;
- the **local Android emulator lane**. Its single entry point is being
  consolidated into `bash scripts/run_local_lane.sh android` by Phase 0 of
  [docs/prompts/OR-BUILD.md](prompts/OR-BUILD.md); until that
  lands, `scripts/start-android-emulator.sh`,
  `scripts/run-android-preview-check.sh` and
  `scripts/stop-android-emulator.sh` are the current mechanism.

Each lane uses one heavy job, a bounded compiler/daemon footprint, small
synthetic fixtures, headroom checks, a timeout and process-tree cleanup, and the
two are serialised against each other on this host. Both produce **developer
evidence only**: hosted `platform-verification` remains the authority for
native, packaged and cross-platform acceptance, and a local preflight or guest
result never becomes `ACCEPTED`.

Outside those two lanes, agents MUST NOT launch the native OR application, a
platform emulator or simulator, or an attached physical device locally for
verification unless the user explicitly requests local runtime testing. This
also forbids native Flutter integration tests that launch OR outside the lanes.
Do not automatically open a built or downloaded Developer Preview. Missing local
Xcode, Android SDK/JDK, CocoaPods, Windows, or Linux tooling is not a blocker
when an equivalent required Actions job exists. Report an unavailable local
check as `LOCAL ENVIRONMENT BLOCKED`, distinct from `PASS`, `FAIL`, and an
intentional `NOT RUN`. Do not install, repair, select, or accept platform
toolchains or licenses, or use `sudo` for platform setup, unless the user
explicitly requests local platform development. See [TOOLING.md](TOOLING.md).

## Regression preservation rule

Do not regress behavior that has already been fixed or verified. Before changing
an existing subsystem, inspect its current behavior and relevant tests or
acceptance checks, and preserve unrelated working behavior. A regression fix is
incomplete until it adds an automated test where practical, or a reproducible
acceptance check for UI behavior that cannot reasonably be unit-tested yet.
Keep recorded guards passing unless the user explicitly changes the requirement;
never remove or weaken a guard just to make a change pass. When refactoring
conflicts with known-good behavior, preserve behavior first and refactor
incrementally. Prefer **preserve → change one subsystem → verify → commit** over
broad rewrites, and rerun checks for every touched subsystem before handoff.

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

Keep AGENTS.md concise: repository rules and a map, not a project encyclopedia. Use [INDEX.md](INDEX.md) for the authoritative documentation map. Prefer updating the existing source-of-truth document over creating unnecessary documents.

## Document routing

Read the relevant source of truth for the task:

- UI: DESIGN.md, docs/UX_ACCEPTANCE.md, and the relevant section of docs/PRODUCT.md.
- Product scope: docs/PRODUCT.md; active priorities: docs/PRODUCT_ROADMAP.md
  (`docs/ROADMAP.md` is archival).
- Architecture: docs/ARCHITECTURE.md and docs/TECHNICAL_PLAN.md.
- Feature implementation: docs/DEVELOPMENT_WORKFLOW.md.
- Testing: docs/TESTING.md.
- Security and licensing: docs/SECURITY_LICENSING.md.
- Release: docs/RELEASE.md.
- Contributors: README.md, CONTRIBUTING.md, and docs/DEVELOPMENT_WORKFLOW.md.

Do not guess project architecture when the repository documents the answer. The HTML prototype is a product and UX reference only, not production architecture.

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
