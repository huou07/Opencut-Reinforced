# Opencut Reinforced — Agent Guide

Last reviewed: 2026-10-14

OR is an MIT-licensed cross-platform video editor for human editing and structured
CLI/agent automation. Architectural directions are not implemented capabilities
or permission to scaffold future subsystems.

## Authority and document map

Host security and tool permissions govern this repository. This file is the
entrypoint; [detailed repository policy](docs/AGENT_POLICY.md) retains the permanent
rules and acceptance restrictions. Read its relevant sections before changes to
execution evidence, architecture, platform tooling, licensing or release.

| Task | Source of truth |
|---|---|
| Active priorities and traceability | docs/PRODUCT_ROADMAP.md |
| Current reuse/upstream/build decisions | docs/OPEN_SOURCE_CONVERGENCE.md |
| Architecture and subsystem contracts | docs/ARCHITECTURE.md, docs/TECHNICAL_PLAN.md |
| Product requirements | docs/PRODUCT.md |
| UI and UX | DESIGN.md, docs/UX_ACCEPTANCE.md |
| Development and tests | docs/DEVELOPMENT_WORKFLOW.md, docs/TESTING.md |
| Platform tooling | docs/TOOLING.md |
| Security/licensing and release | docs/SECURITY_LICENSING.md, docs/RELEASE.md |
| Documentation index | docs/INDEX.md |
| Skill routing | docs/skills.md |
| What was verified, and what it does not prove | docs/evidence/claims/ |

`docs/ROADMAP.md` and everything under `docs/archive/` are archival. They
preserve requirements, checkpoints and receipts. They never select active work
after the convergence pivot. Read them only when reconciling past evidence.

This project is registered in the host control workspace (AEH) under the slug
`or`. From the workspace root, `python3 scripts/hub.py status or` reads Git and
recorded verification without executing project code; `hub.py verify or` runs
`bash scripts/check-repo.sh` explicitly. AEH holds no product or task state —
the authoritative progress source is `docs/PRODUCT_ROADMAP.md`.

## Investigate and implement

1. Establish branch, HEAD and working tree. Preserve unrelated changes.
2. Read task-relevant authorities and inspect existing implementation. When a
   valid CodeGraph index is available, use it before broad exploration and
   verify what it reports against source; without one, explore normally.
3. Define observable completion and change only the requested scope. Reuse code,
   standard/platform facilities and existing dependencies before adding structure.
4. Preserve validated behaviour, schema, security, persistence, performance,
   packaging, capability parity, accessibility and acceptance guards.
5. Run meaningful authorized checks, repair from evidence and update the existing
   relevant documentation. Prose-only changes need no invented product test.
6. Commit one coherent internally consistent change; do not amend, rewrite shared
   history, force-push or discard work without explicit authorization.

A continuation request selects the most valuable incomplete capability in the
active product roadmap. Labels provide traceability; they are not mandatory
prompt/commit boundaries. Preserve real dependencies and the complete contract of
any named requirement. Do not reactivate the historical NEXT checkpoint sequence.

## Permanent architecture and UX

- Rust core/media logic, FFmpeg, wgpu, Flutter, shared structured command API,
  local-first AI and optional user-supplied cloud credentials are intended
  directions. Implement only an authorized scope.
- UI, CLI and agents use the shared application/command path; clients and workers
  must not directly mutate canonical project state. Domain logic stays outside
  presentation where practical.
- Canonical future motion content is declarative and non-executable. Arbitrary
  procedural HTML/JS/GPU content requires its explicit sandbox boundary.
- Read DESIGN.md before UI changes. Preserve Focused Monochrome conventions,
  hierarchy and terminology. Avoid marketing decoration, fake metrics and
  unrelated redesigns. Real journey, states, keyboard and relevant viewport
  evidence accompany UI work; a green build is insufficient.
- The HTML prototype is product/UX reference, not production architecture.

## Evidence and protected state

Feature runners must not edit archived checkpoint specifications, permanent
invariants, phase specs, architecture policy, protected workflows, or claim
records for requirements they did not implement. Report an exact conflict rather
than weakening a contract. Architecture/plan amendments require an explicitly
authorized control-plane task.

A runner reports IMPLEMENTED — AWAITING VERIFICATION EVIDENCE, never
authoritative DONE. A requirement becomes evidenced when a record under
`docs/evidence/claims/` binds the exact implementation SHA to the hosted run and
job that verified it, names its evidence class, and states what it does not
establish. `scripts/check_claims.py` validates those records; a model statement
cannot establish completion.

`scripts/check_or_core_deps.py` enforces the `or_core` dependency boundary in
repository hygiene: GPU, media, audio, text-shaping, scripting and AI-runtime
layers must not be reachable from the crate that owns canonical project state.
Adding a dependency that trips it is a gated decision, not a formatting change —
record an architecture decision rather than widening the pattern list.

Execution-plan graph validation was lost with the paused framework and has not
been reinstated. It guarded a frozen ledger, so there is nothing live to
validate; the equivalent live rule is that `PRODUCT_ROADMAP.md` must not restate
a checkpoint status, which `docs/DEVELOPMENT_WORKFLOW.md` states and
`scripts/check_doc_freshness.sh` keeps current.

## Verification and platform boundaries

Use configured source checks locally. `bash scripts/check-repo.sh` is the
repository gate: tracked-file hygiene, plus Markdown link resolution, document
review freshness, and claim-record validity. GitHub Actions is canonical for
native and runtime acceptance. Do not launch OR, native Flutter integration
tests, local emulators/simulators or physical devices unless the user explicitly
requests local runtime testing. Do not install/repair/select toolchains, accept
licences or use sudo for platform setup without explicit local-development
authorization.

Report PASS, FAIL, NOT RUN and LOCAL ENVIRONMENT BLOCKED accurately. A skipped or
synthetic check does not prove a feasible real packaged product journey. Test
packaged dependencies and permissions as users receive them. Never weaken a
failed test or silently change its evidence class.

After two speculative repairs for the same failure, require actual output, a
falsifiable hypothesis, a discriminating reproduction and a cause-based patch.
Measure realtime/resource claims; retries and timeouts are not root causes.

## Security, licensing and delivery

Keep credentials out of tracked files, logs, fixtures, screenshots and CLI
output. The future agent interface must not expose plaintext application keys.
Use ignored local storage, OS secure storage or repository secrets as appropriate.
Keep downloaded models, generated media, caches and build outputs untracked.

Review dependency provenance, maintenance and licences; pin normal ecosystem
versions. Restrictive distributed dependencies/models need explicit owner approval
and licensing analysis. Model weights and FFmpeg build options have independent
obligations. Preserve notices and lawful tiny fixtures.

Public publication, releases and production deployment require explicit owner
authority. Readiness and a clean scanner do not grant it. Author repository
instructions and framework documents in English; converse in the owner's language.

## Finish

Report changes, source identity, relevant documentation, commands/results, commit
and unverified items. Distinguish implementation, testing and product acceptance.
No skill can override user intent, permanent invariants or host security.
