# Opencut Reinforced — Repository Policy

Authority: permanent execution, acceptance and safety rules. Host/session
security and the user's request govern first. Navigation belongs in
[INDEX.md](INDEX.md), procedures in [DEVELOPMENT_WORKFLOW.md](DEVELOPMENT_WORKFLOW.md).
Last reviewed: 2026-10-20.

## Scope and execution

An answer, analysis, review, investigation, research, plan or prompt request ends
with its answer/artifact; it authorizes no implementation, builds, gates,
commits or delegation. An explicit change/fix/build/run/deliver request authorizes
only its named scope. A roadmap, available tool or skill is no additional grant.
Resume the existing authorized outcome on continuation; only an otherwise
unbounded product continuation selects the highest-value incomplete roadmap
capability. Inspect existing code and documented facilities before adding any.

Use one writer per repository. Stop and record a previous writer before
handover. Preserve unrelated WIP, history and owner files; isolate a task when
needed. Do not create workers without task authority and a useful independent
scope. No skill/plugin overrides user intent, safety, accessibility, licensing,
correctness or the existing acceptance contract.

## Architecture and compatibility

Flutter owns presentation and bounded read models; Rust owns canonical project
and editing state. UI, headless CLI, attached CLI and agents mutate only through
the validated application/command path. A LiveProjectHost owns one shared
ProjectFileSession, including attached IPC. Do not create a Dart/worker/IPC
editable document. Queries and playback ticks never increment ProjectRevision.
Mutation proposals carry identity and expected revision; stale proposals are
rejected, re-queried and revalidated, never silently applied. Preserve exact
RationalTime, command schemas, project migrations and save/recovery safeguards.
Recovery must prove the exact saved base before applying a checkpoint.

[ARCHITECTURE.md](ARCHITECTURE.md), [TECHNICAL_PLAN.md](TECHNICAL_PLAN.md),
[PROJECT_FORMAT.md](PROJECT_FORMAT.md) and the [ADRs](adr/README.md) own subsystem
contracts. The `or_core` dependency boundary is enforced by
`scripts/check_or_core_deps.py`; a failure requires a design decision, never a
widened allowlist. No unrequested engine replacement or architecture scaffolding.

MotionScene stays declarative, non-executable and provider-independent during
validation, preview, rendering and materialisation. Arbitrary web/script content
belongs only behind its explicitly approved future sandbox boundary, never
normal project open, Flutter or canonical evaluation. AI output is a reviewable
proposal applied through revision checks. Keep provider access and cancellation
under ADR 0005; no provider or per-frame planner in rendering. Background work
must be bounded and must not block realtime decode/audio/presentation.

## Product acceptance and protected evidence

Use structured APIs for ordinary agent editing. To verify a UI claim, operate
its actual controls and observe the resulting canonical Rust state. A direct
command is not a UI test, a widget is not a package, and a thumbnail is not
playback. Preserve Focused Monochrome and approved interactions from
[DESIGN.md](../DESIGN.md) and [UX_ACCEPTANCE.md](UX_ACCEPTANCE.md).

[QUALITY.md](QUALITY.md) owns evidence classes and the quality bar;
[TESTING.md](TESTING.md) owns executable checks and independent human evaluation.
A passing check supports only its assertions. Skipped/unavailable required checks
are BLOCKED; absent observations are UNVERIFIED; observed failures stay FAIL.
Do not average severity, suppress failed attempts, weaken guards, change evidence
class, use stale outputs or redefine acceptance to manufacture a pass.

Hosted `platform-verification` remains canonical for native/package/cross-platform
acceptance. ACCEPTED requires the relevant exact-source hosted/owner evidence;
local developer evidence is recorded separately. A claim under
[evidence/claims/](evidence/claims/README.md) names the exact implementation SHA,
hosted run/job, UTC time, evidence class and limits. Never invent run IDs or edit
unrelated completion records. Supersede historical evidence through the documented
claim procedure; preserve original observations.

The frozen checkpoint specs, permanent invariants, architecture policy,
protected workflows and existing completion evidence are not ordinary feature
write scope. Amend only under explicit control-plane authorization and never to
weaken product, persistence, security, licensing or acceptance requirements. The
archived INV-PRODUCT/PACKAGE/CAPABILITY/PARITY/PERF/UX/ACCEPT/VERIFY contracts
remain binding. The archived supervisor and NEXT cursor stay paused; do not
restart them or treat their ledger as today's status.

## Runtime and host boundaries

Default to headless scoped checks. The bounded Debian Linux developer lane and
Android emulator lane are owner-granted exceptions in
[AGENT-AUTHORITY.md](AGENT-AUTHORITY.md); exact commands belong in
[TOOLING.md](TOOLING.md). Explicit task authorization for local runtime testing
also applies; do not ask again for an already authorized action. Local runtime
results remain developer evidence. Preserve headroom, shared native locking,
timeouts, process ownership, isolated test data and cleanup. Stop only resources
you started; never clear shared caches, stop unrelated daemons or alter services.

Install/repair/select toolchains, accept licences, use sudo or change privileged
host configuration only with explicit local-development authority. Prefer
installed tools. Missing required local capabilities need a concrete probe and
BLOCKED result; an equivalent hosted result may settle that lane, never an
invented local pass. Do not request broader privileges for convenience.

## Security, dependencies and licences

Reuse the standard library, platform and existing dependencies before adding
structure. Review upstream provenance, maintenance, locked versions and licences
for any new dependency. OR is MIT licensed. GPL, AGPL, SSPL, non-commercial or
other distribution-restrictive components/models need explicit owner approval
and licensing analysis. Model weights have separate rights; never bundle or
download them without the documented confirmation. Preserve FFmpeg build
configuration/provenance, codec licences and distribution notices.

Credentials stay in OS secure storage or the documented protected environment;
CLI, agents, logs, screenshots and fixtures never receive plaintext keys. Do not
read owner credentials, use another worker identity, or commit keys/certificates,
.env files, downloaded models, caches, build outputs or large generated media.
Use small lawful deterministic fixtures with checksums and provenance.

## Repair and delivery

After two speculative attempts at the same failure, require exact output,
a falsifiable hypothesis and a discriminating check before another patch.
Timeout/retry is not a diagnosis. Preserve existing regressions and add relevant
observable guards for changed behavior. Use proportionate tests; prose alone
needs diff/link review, not invented product tests. Update the existing owning
document for a substantive change. Do not duplicate priorities or a handbook in
AGENTS or skills.

Commit each coherent verified logical change using the configured identity and
a meaningful subject. Never amend, rebase shared history, force-push, discard WIP
or merge protected branches without explicit authority. Open a reviewable PR
when requested. Green checks grant no publication, release, tag or deployment
authority; [RELEASE.md](RELEASE.md) owns those requirements.

Completion names what changed, exact source/build, commands/results, retained
evidence, commit/PR and remaining failures or gaps. Infrastructure tests can pass
while diagnostic product journeys fail: report both. A model statement is never
repository-authoritative DONE. No unrun platform or human evaluation is accepted.
