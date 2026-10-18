# Development Workflow

Last reviewed: 2026-10-18

## Status

This workflow applies to the current implementation and every later one. The
phased build history and its completion receipts are preserved in
[ROADMAP.md](ROADMAP.md) (archival) and
[execution/](archive/execution/README.md) (archival machine ledger). Active
priorities and traceability live in [PRODUCT_ROADMAP.md](PRODUCT_ROADMAP.md);
this document does not restate them and must never copy a checkpoint status.

## Roadmap-first workflow

Before any feature, architecture, or automation work, establish what is actually
next:

1. Read [PRODUCT_ROADMAP.md](PRODUCT_ROADMAP.md) and identify the most valuable
   incomplete capability. A continuation request ("continue", "next task")
   selects that capability, not a historical checkpoint.
2. Check the current machine and repository state:

```sh
git status --short
git log --oneline -5
```

3. Read the task-relevant authorities named in the table in `../AGENTS.md`.

The legacy checkpoint graph in `docs/archive/execution/PLAN.json` and its `NEXT` cursor
are **history, not a work selector**. They are preserved because their receipts
are real evidence of past work, and they are read only when reconciling that
evidence or answering a question about a past checkpoint. Do not restart the
checkpoint sequence, and do not reintroduce the old rule that one model context
executes exactly one checkpoint.

Where the roadmap and the archived ledger disagree about what happened, both are
true about different things: the ledger records what was verified at the time,
the roadmap records what the product still needs. Report the disagreement rather
than editing either to match.

### Evidence discipline after the pivot

A feature runner implements and pushes; it does not declare a requirement done.
A completion claim needs a claim record under
[evidence/claims/](evidence/claims/README.md) naming the exact implementation
SHA, the hosted run and job that verified it, and the UTC time. A model
statement is not evidence, and a green local check is not product acceptance.

Feature runners must not edit archived execution specifications, permanent
architecture invariants, the architecture policy, or claim records for
requirements they did not implement. Report an exact conflict rather than
weakening a contract. Architecture or plan amendments require an explicitly
authorized control-plane task.

Never silently auto-apply a recovery checkpoint over a canonical project whose exact saved base cannot be proven.

Keep the Phase 5A `media probe` operation read-only: it accepts a local filesystem `Path`, returns validated metadata, and never creates a `MediaId`, opens or mutates a project, or increments `ProjectRevision`. Phase 5B import prepares one selected file with the existing probe, canonicalizes its path, builds a validated local `file:` URI and fresh `MediaId`, then submits `media.add` through the application path. Project loading validates stored URIs but never opens or probes referenced sources. Desktop imports and disposable media-artifact generation use the packaged-first FFmpeg helpers; a normal packaged app must not depend on a system `PATH` executable or loader override.

For project mutations, keep Flutter, headless, and attached CLI operations on the shared `ApplicationRequest` path. `LiveProjectHost` must own exactly one `ProjectFileSession`; its opaque Rust bridge handle and IPC server share the same state. Do not add a Dart-editable document or a second IPC-owned project session. Headless mutations execute shared commands through `ProjectFileSession` and save changed state with exact-base checks; attached commands use the existing `LiveProjectHost` and leave its live state dirty until explicit save. The Phase 6C/6D/6E1/6E2B Flutter timeline keeps only bounded disposable query pages and uses those same commands; project invalidation events refresh the open view so attached CLI edits appear there. Phase 6E2A marker commands and queries use the same Rust-owned path, and 6E2B exposes them through typed bridge/gateway methods without creating a second document. Timeline edits use exact `RationalTime`, append tracks, remove only empty tracks, sort inserted/moved clips by timeline start, reject same-track overlap, permit adjacency and cross-track overlap, and permit moves only between tracks of the same kind. Trim uses absolute start/end edges, split requires an exact interior point, and ripple delete shifts only later clips on the selected track without moving global markers. Pointer edits retain the original canonical exact time, quantize only the total delta to the nearest 1 ms, show a presentation-only ghost, resolve the appropriate canonical Snap query once at drop, and dispatch the existing move/trim command; the fixed snap threshold is `1/8` second over timeline zero, clip boundaries, and persistent markers. Rust validates final state; successful edits refresh from Rust and never optimistically change geometry. Timeline and marker commands remain disallowed inside grouped transactions. The running Flutter application and developer/headless `or session serve` can each host a live project. Preserve recovery inspection and exact-disk-base conflict checks on every open/save path. Android project access crosses SAF through an app-private managed working copy; retain provider URIs and permission grants as runtime metadata, and never send `content://` URIs to Rust path APIs. Canonical project mutations still use the shared `ApplicationRequest` and normal `ProjectFileSession` path.

### Future MotionScene authoring

MotionScene is future non-executable declarative source, not a product feature
of the current execution checkpoint. Keep exact `RationalTime`, random-access
evaluation, typed/bounded primitives, stable asset/font provenance, and the
shared RenderSnapshot/wgpu path. Preview and materialization must share the
evaluator, and materialization must register persistent output through normal
media/timeline commands. Rendering makes no required AI/provider call.

External agents and templates may produce the same scene contract. Do not add
JavaScript, shell, network, arbitrary expressions, or provider calls to
canonical scene data; do not add a live MotionClip without a new architecture
gate. A future WebMotionBundle is allowed only in the explicit Phase 16G
least-privilege sidecar boundary with network off, no credentials, bounded
inputs/outputs, cancellation, and crash containment. It never runs in
`or_core`, Flutter, normal project open, or canonical evaluation.

## Feature path

1. **Define product behavior.** State the user problem and intended result. Classify the work as domain, media, render, audio, AI, UI-only, or infrastructure. Check [PRODUCT.md](PRODUCT.md).
2. **Inspect the existing system.** Read relevant documentation and code. Use CodeGraph where useful. Reuse existing concepts instead of duplicating abstractions.
3. **Define affected surfaces.** For an editing capability, consider the Rust command and query, CLI, desktop GUI, mobile GUI, agent capability, docs, and tests. Not every feature needs every surface; record a reason for omissions.
4. **Put behavior in the domain first.** Editing behavior belongs in the Rust domain or application layer. Flutter is not the source of truth.
5. **Preserve state ownership.** Before implementing stateful editing behavior, identify canonical state in the Rust/domain/application layer. Flutter state must be presentation-local or a read-model/cache. If a proposal creates a second independently editable project state in Dart, CLI code, worker state, or an agent session, stop and redesign.
6. **Define the command contract.** Reuse or create a stable command ID; specify inputs, schema version, validation, revision/preconditions, errors, ChangeSet, permissions, and undo behavior. Only the command/application execution path mutates canonical project state.
7. **Test the command and domain.** Add unit, property, or contract coverage before relying on the UI.
8. **Provide CLI parity where appropriate.** Expose semantic project operations and meaningful project queries where representable; presentation-only controls do not require CLI exposure. See [PRODUCT.md](PRODUCT.md).
9. **Integrate with the UI.** Use a stable shell slot and check feature descriptors and the command registry first. Do not add a permanent navigation region without clear justification. Read [DESIGN.md](../DESIGN.md).
10. **Adapt mobile presentation.** Use the same command and state model. Adapt layout with touch-native panels, sheets, or full-screen utility surfaces; do not duplicate domain logic.
11. **Expose safe agent capabilities.** Add discovery, queries, permissions, dry-run, and diff preview where appropriate. Never expose secret access.
12. **Review performance where it applies, plus security and licensing.** Apply the hot-path review below primarily to media, render, audio, timeline evaluation, export, AI processing, and large background jobs. Ordinary UI-only work does not need an unnecessary media-performance review. Consider supported platforms, permissions, dependency licenses, asset rights, and model rights for all relevant work.
13. **Localize at the presentation boundary.** Do not scatter user-facing English strings through domain/business logic. Use localization-capable Flutter resources when production UI starts; keep command IDs, JSON field names, and machine-readable error codes stable.
14. **Update documentation.** Update the existing source of truth. Create a new document only when the information needs a durable home.
15. **Preserve regression guards.** Read [UX_ACCEPTANCE.md](UX_ACCEPTANCE.md). Add a permanent guard when a UI regression is fixed. Never remove or weaken a working guard to make a change pass.
16. **Verify the affected system.** Follow the CI-first verification ladder below. Report checks that did not run with their actual status.
17. **Commit one logical change.** Commit only a coherent change with no known-broken state. Use a clear Conventional Commit-style subject.
18. **Open a focused pull request and pass CI.** External contributors use feature branches and pull requests. Explain what changed, why, architecture impact, tests, docs, and risks. CI must pass before merge. Early maintainer work may continue to fast-forward main pushes under repository policy.

For Flutter project lifecycle changes, run the fake-gateway widget tests and include the native Rust-bridge lifecycle test in hosted CI. Verify attached CLI parity in a process-level test against the same `LiveProjectHost`; a sandboxed macOS UI integration process cannot launch an external CLI binary itself. Confirm that IPC protocol v1 remains unchanged and that the Flutter read model refreshes from Rust summaries after ordered events.

The real media-probe and media-artifact integration tests run on hosted Linux with CI-installed FFmpeg tooling and generated synthetic fixtures. They verify the real `ffprobe` and `ffmpeg` paths; do not install FFmpeg locally or run these integration tests locally. The native Flutter lifecycle and offline-media bridge integration tests run in hosted macOS CI. Keep the permanent local policy: do not launch OR or run native Flutter integration tests locally.

### CI-first verification ladder

Local verification is headless by default. Do not launch the native OR application, platform emulator/simulator, or attached physical device locally for verification; do not run native Flutter integration tests that launch OR or open a built/downloaded Developer Preview. GitHub Actions is canonical for native/runtime evidence. If local runtime interaction is genuinely required and Actions cannot provide the evidence, explain why and ask the user before launching it. This policy does not block local Rust/CLI/headless tests or Flutter format, analysis, and widget tests that do not launch a native application.

Two bounded local runtime lanes are an explicit, dated exception recorded in [AGENT-AUTHORITY.md](AGENT-AUTHORITY.md): the Debian Linux developer preflight and the local Android emulator lane. The Linux lane has one entry point, `bash scripts/run_local_lane.sh linux` (`--packaged` adds the local `.deb` build and the packaged journey), which runs the hygiene gate, the Flutter widget suite, the bounded preflight and the performance budget, writes one machine-readable report, and exits non-zero unless every selected step passed. Both lanes are developer evidence only: they are serialised on this host, they never authorise a second writer or cross-platform packaging, and hosted `platform-verification` stays the acceptance authority. Step scope, report schema, exit codes and limits are in [TESTING.md](TESTING.md#one-local-lane-entry-point).

1. Run repository hygiene, diff checks, formatting, and static/source checks locally.
2. Run checks scoped to the affected package or subsystem, such as `cargo check -p or_core --all-targets`, plus relevant tests and linting where available.
3. Attempt stronger workspace checks when useful; do not let an unrelated missing platform tool prevent source work.
4. Classify each check as `PASS`, `FAIL`, `LOCAL ENVIRONMENT BLOCKED`, or `NOT RUN`. Use `NOT RUN — LOCAL NATIVE EXECUTION DISALLOWED BY POLICY` for a local native/runtime check intentionally skipped by policy. Use `LOCAL ENVIRONMENT BLOCKED` only when an actual missing or unconfigured local tool prevented execution; neither status is a pass.
5. Push through normal Git and inspect the actual required GitHub Actions jobs for the commit. Hosted CI is canonical for platform/linker/native verification.
6. Fix implementation failures and rerun relevant local checks before pushing a focused fix. Do not disable or weaken required workflows to get a green result.
7. Complete only after required remote jobs pass. A local block with no equivalent remote result remains unverified and must be reported. Do not install, repair, select, or accept platform toolchains or licenses, or use `sudo` for platform setup, unless the user explicitly asks for local platform development.

### Real-time and hot-path review

For applicable work, ask:

- Does this run per project edit, per frame, or per audio callback?
- Does it allocate on the hot path or copy large frames/buffers?
- Does it require CPU↔GPU transfer or block the UI/application mutation path?
- Is background concurrency bounded, and can stale preview work be cancelled or dropped safely?
- Does it preserve a correctness fallback?

If a proposed implementation routes per-frame playback/render work through the project mutation path, stop and redesign it. Playback ticks, frame decode, and presentation are runtime work; they must not create Project transactions or increment `ProjectRevision`.

## State ownership, concurrency, and background results

For every stateful feature, check the command ID, validation, expected project revision/preconditions, ChangeSet, undo behavior, background work, stale-result handling, and read-model/event notification. Queries are read-only. A long-running UI workflow, attached CLI, agent EditPlan, or background analysis based on inspected state must carry its expected revision when applying a change; if the revision changed, reject it, re-query and revalidate or dry-run again, then regenerate the proposal if needed. Do not silently apply stale work.

Workers return structured results, proposals, analysis, or generated assets. If a result should change a project, ask: “What happens if the project changes before this result is applied?” The safe default is a revision/precondition check followed by rejection and revalidation or rerun. Workers never write canonical Project state directly; the application applies accepted results through a validated command/transaction.

## When an ADR is required

Create an Architecture Decision Record for a significant, difficult-to-reverse choice, including:

- a breaking native project format change
- render backend architecture
- plugin security and capability model
- a major new runtime dependency
- a breaking public command schema
- a new permanent UI shell region
- the canonical MotionScene contract or an isolated procedural execution boundary

An ADR should record context, decision, alternatives, tradeoffs, and consequences. Do not create ADRs for routine implementation details.
