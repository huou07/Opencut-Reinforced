# Development Workflow

## Status

This workflow applies to the Phase 3 architecture skeleton and all later implementation. Phase 4A–4F provide project/application foundations, shared command/query/transaction dispatch, exact-base file sessions, bounded `.orproj` v1 storage, recovery, local IPC, and semantic CLI operations. Phase 4UI-1 provides the Flutter visual shell; Phase 4UI-2 connects desktop create/open/save, explicit recovery, dirty-state guards, and Flutter-hosted IPC to one Rust live project host. Timeline and media behavior have not started. See [ROADMAP.md](ROADMAP.md) for the current phase boundary.

Never silently auto-apply a recovery checkpoint over a canonical project whose exact saved base cannot be proven.

For project mutations, keep Flutter, headless, and attached CLI operations on the shared `ApplicationRequest` path. `LiveProjectHost` must own exactly one `ProjectFileSession`; its opaque Rust bridge handle and IPC server share the same state. Do not add a Dart-editable document or a second IPC-owned project session. Headless rename saves a changed result through exact-base checks; Flutter and attached rename/undo/redo leave the live session dirty until explicit save. The running Flutter application and developer/headless `or session serve` can each host a live project. Preserve recovery inspection and exact-disk-base conflict checks on every open/save path. Android project file access stays unavailable until SAF is implemented; never send content URIs to Rust path APIs.

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

For Flutter project lifecycle changes, run both the fake-gateway widget tests and the native Rust-bridge lifecycle test. Verify attached CLI parity in a process-level test against the same `LiveProjectHost`; a sandboxed macOS UI integration process cannot launch an external CLI binary itself. Confirm that IPC protocol v1 remains unchanged and that the Flutter read model refreshes from Rust summaries after ordered events.

### CI-first verification ladder

Local verification is headless by default. Do not launch the native OR application, platform emulator/simulator, or attached physical device locally for verification; do not run native Flutter integration tests that launch OR or open a built/downloaded Developer Preview. GitHub Actions is canonical for native/runtime evidence. If local runtime interaction is genuinely required and Actions cannot provide the evidence, explain why and ask the user before launching it. This policy does not block local Rust/CLI/headless tests or Flutter format, analysis, and widget tests that do not launch a native application.

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

An ADR should record context, decision, alternatives, tradeoffs, and consequences. Do not create ADRs for routine implementation details.
