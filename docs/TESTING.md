# Testing Strategy

## Status

Phase 3 has executable tests for the bootstrap core, CLI, and native bridge. Phase 4UI-1 adds structural widget regression coverage for the Flutter visual foundation. Phase 4F adds file-session, local IPC, and semantic CLI contracts. Phase 4UI-2 adds fake-gateway widget coverage, a native Flutter lifecycle test, and a real attached-CLI process test against the same shared live host. Phase 5A adds media identity, metadata, bounded external-probe, and CLI contract coverage plus a real generated-media `ffprobe` test on hosted Linux CI. OR remains pre-MVP and is not a usable video editor. The test layers below distinguish implemented coverage from future product tests.

## Current Phase 3 checks

- Rust unit tests cover core bootstrap values and bridge DTO mapping; CLI contract tests execute the real binary and verify human output, JSON, help, and invalid input.
- A macOS integration test initializes the native Rust library, calls app info, health, and capability discovery through the typed bridge, and compares the results with the CLI snapshot.

These checks prove the bootstrap architecture only. They do not demonstrate editing, media, or release behavior.

## Current Phase 4UI-1 coverage

Structural tests in `apps/or_app/test/widget_test.dart` cover:

- wide desktop shell, OR branding, top bar, and route selection
- compact shell navigation across all five primary destinations without overflow
- a medium-width desktop layout, centralized design tokens, and responsive breakpoints
- Home's empty recent-project state, Projects navigation, and honest New / Open feedback
- Settings diagnostics provided by a fake `CoreGateway`, including Advanced / Developer
- Ctrl/Cmd+K command palette navigation to the Editor Shell Preview
- desktop and compact Editor Shell Preview regions, unavailable tools, and absence of fake media

The frozen prototype is guarded separately by its before/after SHA-256 and an empty `git diff -- prototypes/or-ui-demo.html`; this is not a pixel-golden test. No pixel-perfect parity claim is made.

## Current CI gates

GitHub Actions runs Rust formatting, Clippy, and the full workspace test suite; Flutter dependency, formatting, analysis, and widget checks; storage, recovery, real local IPC, shared-host/attached-CLI parity, and Windows endpoint ACL tests on macOS and Windows; native builds for macOS, Linux, Windows, and Android; and native macOS Flutter bridge/lifecycle integration tests. The Linux Rust job installs FFmpeg tooling for CI only, logs `ffmpeg -version` and `ffprobe -version`, and explicitly runs the generated-media real-probe integration test. Android CI builds the Rust bridge and APK but does not run IPC on an Android device.

## CI-first verification status

GitHub-hosted Actions is canonical for platform correctness, linker-dependent builds, and the native bridge runtime smoke test. Local inability to run a platform test does not remove its verification requirement; it moves the evidence source to the equivalent required Actions job.

Local native/runtime execution is disallowed by default: do not launch OR, an emulator/simulator, or an attached device for verification; do not run native Flutter integration tests that launch OR, manually smoke-test the GUI, or open a built/downloaded Developer Preview. Local Rust/CLI and other headless tests remain allowed, as do Flutter formatting, analysis, and widget tests that do not launch a native application. If Actions cannot provide genuinely required runtime evidence, explain why and ask the user before local launch.

- `PASS`: the check ran and succeeded.
- `FAIL`: the check ran and found a defect that must be addressed.
- `LOCAL ENVIRONMENT BLOCKED`: local execution was prevented by unavailable or intentionally unconfigured platform tooling. This is neither pass nor fail.
- `NOT RUN — LOCAL NATIVE EXECUTION DISALLOWED BY POLICY`: local native/runtime verification was intentionally skipped under the default execution policy. This is distinct from an environment limitation and is not a pass.

For example, a blocked local macOS native check plus a passing GitHub macOS native job is verified. A blocked local check with its remote job not run is not verified. Never weaken or omit a test because a local platform tool is unavailable.

## Current Phase 4A coverage

- `ProjectId` and `ProjectInstanceId`: UUIDv4 generation, canonical display and parse round trips, serde round trips, and invalid/non-v4 project ID rejection.
- `ProjectRevision`: initial zero, checked increments, overflow rejection, and serde round trip.
- `RationalTime` and `RationalRate`: normalization, invalid denominator/rate rejection, serde validation and normalization, and exact 24, 24000/1001, 30000/1001, and 48000/1 unit conversions.
- Checked exact addition/subtraction and overflow, rational ordering, and `TimeRange` duration validation including serde rejection of negative duration.

Phase 4A tests foundational values only; later sections record project and application coverage.

## Current Phase 4B coverage

- New `ProjectDocument` ID, initial revision, exact name, and domain round trip.
- V1 format marker, schema version, envelope fields, deterministic pretty output, and trailing newline.
- Exact Unicode name preservation and unchanged nonzero/maximum revision round trips.
- Malformed JSON, missing fields, wrong marker, invalid/missing/noninteger or unsupported schema version, and unknown-field rejection at the envelope and project levels.
- Malformed, nil, and non-v4 project ID rejection through the existing `ProjectId` invariant.
- Runtime-instance ID and field leakage guard; decoding returns only canonical project state.

Filesystem save/load tests are recorded under Phase 4E1. Schema migrations remain unimplemented.

## Current Phase 4C coverage

- `ProjectSession::open` preserving the document while owning a valid runtime instance ID, plus discovery of the then-implemented command and query contracts.
- Strict command/query envelope serde round trips, unknown-field rejection, and validation order for operation ID, schema, project, instance, revision, and arguments.
- `project.rename`: exact Unicode preservation, one revision increment per real rename, same-name no-op, stale revision/current-revision reporting, project/session mismatch, unknown/unsupported command rejection, malformed/extra arguments, and overflow without partial mutation.
- `project.summary`: current IDs, revision, and name; read-only behavior; visibility of renamed state; project/session mismatch; unknown/unsupported query rejection; and empty-object-only arguments.
- Rename → `.orproj` encode/decode preserves the changed name and revision without persisting the runtime instance ID.

## Current Phase 4D coverage

- Exact command/query catalogs including undo/redo discovery; strict command-call and transaction-envelope serde round trips, unknown-field rejection, and typed-ID validation.
- Atomic rename groups, net `ChangeSet` normalization, one revision increment/history entry for changed groups, and no revision/history/redo clearing for net no-ops.
- Rollback and history preservation for invalid, unknown, unsupported, disallowed, empty, stale, or wrong-project/session transactions.
- `history.undo` and `history.redo`: inverse/forward changes, one new revision each, grouped undo, redo invalidation after a real edit, no-op/failed-edit redo preservation, empty-stack errors, history conflicts, and overflow without partial mutation.
- Transaction and history effects round-trip through the `.orproj` v1 codec as canonical name/revision only; runtime instance ID and history remain absent, and a reopened session starts with empty history.

Filesystem save/load tests are recorded under Phase 4E1. Migration remains unimplemented; Phase 4F records IPC and client-integration coverage.

## Current Phase 4E1 coverage

- Bounded load accepts canonical `.orproj` v1 through the existing codec and rejects files over 64 MiB, invalid UTF-8, invalid codec data, and missing files.
- Save/load preserves `ProjectId`, nonzero `ProjectRevision`, and exact project name without mutating the source session; runtime instance ID and undo/redo history are not persisted.
- New and existing destinations, Unicode paths, and the caller-owned parent-directory rule are covered.
- Failed replacement preserves the existing destination and removes the temporary sibling; oversized encoded state is rejected before temporary creation and leaves an existing destination unchanged.
- Unit checks confirm the temp file is a same-directory sibling and a failed platform replacement leaves the destination intact.
- The storage integration test runs in the Linux workspace Rust checks and on macOS and Windows in Platform Verification. Android CI builds the Rust bridge for Android but does not run storage tests on an Android device.

## Current Phase 4E2 coverage

- Candidate inspection requires exact saved-base equality; inspection also covers `NONE`, stale checkpoints, same-revision content mismatch, intermediate revisions, rollback below base, foreign projects, and orphaned sidecars.
- Strict recovery-envelope marker, schema, and unknown-field validation; nested `.orproj` codec failures; bounded oversize rejection; and invalid UTF-8 rejection.
- Checkpoint creation validates project identity, newer revision, and exact disk base; repeated checkpoints replace the old sidecar atomically without changing the canonical project.
- Explicit apply revalidates after an earlier inspection, saves the recovery snapshot without a revision increment, and covers save failure preservation plus cleanup-pending behavior. Explicit discard covers valid and malformed checkpoints and leaves the canonical project unchanged.
- Persistence guards confirm runtime `ProjectInstanceId` and session history are absent. Unicode paths and temporary-file cleanup are covered.
- Recovery integration tests run in Linux Rust checks and in dedicated macOS and Windows Platform Verification steps. Android CI builds the Rust bridge but does not run recovery tests on an Android device.

Recovery UI behavior is covered under Phase 4UI-2 below. Autosave remains unimplemented and is not tested.

## Current Phase 4F coverage

- `ProjectSession::handle_application_request` dispatches commands, queries, and transactions through the existing semantic methods; operation errors remain the core `OperationError` values.
- `ProjectFileSession` tests cover open without revision change, dirty state, exact saved-base comparison including same-revision external replacement, recovery candidate blocking, stale recovery allowance, and safe save behavior.
- `or_ipc` unit tests cover big-endian framing, empty/oversized/truncated frames, strict endpoint descriptors and redacted debug output, request-ID matching, malformed UTF-8/JSON without mutation, and authentication/protocol rejection before dispatch.
- Real local transport tests cover semantic query/command/transaction requests, stale revision and instance protection, explicit save, no autosave, external disk conflict, recovery appearing during a live session, authentication, guarded/discard shutdown, descriptor permissions/collision, endpoint rotation, and cleanup. They use Unix sockets on Linux/macOS and named pipes on Windows.
- The Windows `or_ipc` unit suite reads back the ACLs from the created runtime directory, descriptor file, and named pipe and verifies that only the owner-rights ACE is present. The pipe creation keeps the remote-client rejection flag enabled.
- CLI contract tests preserve the bootstrap commands and cover catalog JSON, headless summary/rename, no-op rename, all recovery statuses and explicit actions, attached summary/rename/undo/redo/save/describe/shutdown, semantic parity, dirty shutdown rejection, exact disk conflict protection, clean JSON output, token non-disclosure, OS-path handling, and UTF-8 project-name validation.
- CLI integration tests run in the Linux Rust workspace job. The macOS and Windows jobs also run the real attached-CLI contracts directly; Windows additionally runs the `or_ipc` unit suite for endpoint ACL checks.

## Current Phase 4UI-2 coverage

- Widget tests inject fake project gateways and file pickers. They cover desktop create/open, cancellation, exact project-name preservation, revision-zero creation, close and switch Save/Discard/Cancel decisions, save failures, stale-revision refresh without retry, event-sequence invalidation, recovery candidate/stale/conflict/invalid handling, Android's unavailable New/Open state, keyboard shortcuts, and exit cancellation.
- The macOS native integration test uses the generated Rust bridge to create and open a real `.orproj`, check revision and runtime identity, rename/undo/redo/save through the Flutter workspace, inspect the Advanced / Developer descriptor surface, verify descriptor cleanup, and reopen with the persistent project ID but a fresh runtime instance ID.
- `crates/or_cli/tests/semantic_cli.rs` launches the actual `or` executable against a `LiveProjectHost`. Direct host commands model the bridge path while attached CLI commands query and mutate the same host. It verifies GUI-style rename → CLI summary, CLI rename and undo → direct summary, direct redo → CLI summary, shared identity/history/revision/dirty state, explicit save, and ordered change/save events.
- The `LiveProjectHost` tests separately exercise direct access and `LocalIpcClient` access to the same session, revision-conflict behavior, dirty shutdown protection, and endpoint cleanup on Unix and Windows transports.
- The native UI test does not spawn an external CLI process from the sandboxed application. Process-level CLI parity is exercised by the Rust integration test outside the app sandbox.
- Project creation uses the Rust no-clobber storage tests; project bytes are never decoded or written by Dart. Recovery inspection/apply/discard contracts remain covered by the Phase 4E2 Rust tests.

## Current Phase 5A coverage

- `MediaId` and `JobId`: UUIDv4 generation, canonical display/parse, serde round trips, and rejection of malformed and non-v4 IDs.
- Exact duration parsing: `0`, whole seconds, decimal fractions, normalization, excess precision, invalid signs/text, and overflow. Rates cover `24/1`, `24000/1001`, `30000/1001`, zero values, invalid syntax, and overflow.
- External JSON conversion: video-only, audio-only, combined streams, subtitle/other streams, missing optional fields, unknown external fields, malformed documents, invalid dimensions, invalid durations, and invalid optional frame rates/audio values. Invalid optional rates and audio sample-rate/channel-count values become unavailable; invalid required dimensions or durations fail with a structured metadata error.
- OR metadata serde: stable round trip and rejection of zero dimensions or negative duration.
- Probe process tests: missing executable, directory input, short injected timeout with child cleanup, oversized stdout and stderr, nonzero exit with bounded sanitized diagnostic, and a path containing spaces and Unicode.
- CLI contracts: human and OR JSON output, Unicode input path, structured backend-unavailable and missing-file errors, and bounded probe-failure details.
- Real integration: the hosted Linux Rust job generates a tiny video/audio Matroska file with CI-installed FFmpeg and probes it with the real `ffprobe`. The fixture is generated in a temporary directory and no media sample is committed. The integration test is ignored in the default local suite and is run explicitly on hosted Linux.
- Local native/runtime verification remains `NOT RUN — LOCAL NATIVE EXECUTION DISALLOWED BY POLICY`; the macOS bridge/lifecycle evidence comes from hosted CI.

## Test pyramid

### Rust domain and application unit tests

Test deterministic commands, queries, project rules, timeline operations, errors, and undo behavior close to the domain code.

Property and invariant tests should cover:

- valid timeline ranges and ordering
- stable opaque persistent IDs and reference integrity
- undo and redo restoring equivalent state
- rational time conversions and frame boundaries
- transactional rollback on failure
- one project revision increment per successful mutating transaction, and none for reads or failed/rolled-back transactions
- rejection of stale expected revisions without applying the command

### Serialization and command contracts

- project serialization round trips
- forward migrations from supported prior versions
- malformed, truncated, oversized, and corrupt project input
- command schema and validation contracts
- query schemas, capability discovery, and structured errors
- atomic save and crash journal recovery

### CLI and agent contracts

- stable command discovery and exit-code categories
- JSON output shape and versioning
- headless and attached behavior
- dry-run does not mutate state
- EditPlan schema, unknown command rejection, permissions, diff, apply, and undo
- secrets never appear in CLI, agent output, logs, or errors

### Flutter and UX

- Flutter widget tests for controls, panels, focus, and state rendering
- [UX_ACCEPTANCE.md](UX_ACCEPTANCE.md) for preserved interaction invariants
- keyboard navigation and accessibility semantics
- mobile sheet, touch target, and timeline interaction checks

### Render, audio, media, and export integration

- Exact domain/CPU reference determinism where applicable: timeline evaluation and time math, command behavior, serialization, effect parameter evaluation, and frame scheduling decisions.
- GPU/image output comparisons must not assume bit-identical pixels across Metal, Vulkan, D3D12, GPU vendors, or shader compiler versions. Use operation-appropriate comparisons such as per-channel tolerance, maximum error, percentage of differing pixels, or SSIM/perceptual thresholds; select no single metric in advance. Golden tests must fail clearly on critical semantic regressions.
- Preview and export golden cases should verify the same edit semantics, while allowing different resolution, proxies, quality, scheduling, and encoders.
- audio/video clock synchronization and seek behavior
- export output verification for container, streams, duration, dimensions, and expected frames
- decode, encode, and packaged FFmpeg configuration checks; metadata-probe coverage is recorded under Phase 5A
- platform-specific GPU and texture fallback coverage

### Data, platform, and security

- Android storage and media integration
- macOS integration
- Windows and Linux CI coverage
- plugin sandbox and capability tests when plugins exist
- template and theme schema validation, including rejection of executable content
- untrusted project, subtitle, media metadata, archive, model, and agent-output tests
- permission and secret-boundary tests
- crash recovery, memory, leak, and performance tests

## Performance instrumentation and benchmarks

Performance work is planned. Local diagnostic instrumentation should eventually measure decode latency, render CPU time, render GPU time where available, present latency, dropped frames, audio underruns, queue depth, memory use, GPU/resource memory where measurable, cache hit/miss, and export throughput. This is local performance diagnosis, not telemetry or network reporting.

Before claiming a hardware optimization, use controlled, repeatable media fixtures and benchmark scenarios. Representative workload classes may include:

- 1080p H.264 playback and 1080p high-frame-rate playback
- 4K H.264/H.265 playback and 4K high-frame-rate playback where hardware permits
- multiple composited layers
- transform, color, and effect workloads
- seek and scrub workloads
- export throughput

These are benchmark examples, not permanent product resolution or codec requirements. Use tiny self-created or legally safe fixtures with recorded provenance; do not download copyrighted benchmark media. Compare software and hardware paths on known hardware where possible, and record enough device/backend and workload context to make results interpretable. Measure latency, throughput, copies/transfers, memory, and fallback correctness rather than assuming a hardware path is faster.

GitHub-hosted CI is authoritative for build correctness, automated tests, platform compatibility, and native bridge verification. Shared hosted runners are not stable authoritative hardware-performance machines: do not set hard FPS or performance-regression thresholds from ordinary hosted-runner timings. If performance regression checks become necessary, use known dedicated hardware, a self-hosted runner, or a repeatable local benchmark machine.

## Fixtures and results

Use tiny, self-created or legally safe media fixtures. Keep fixture provenance and rights clear. Avoid shipping downloaded models or copyrighted media as test data.

A check that did not run must never be reported as passing. Report its exact status and reason. Keep a failing or unavailable check visible rather than silently omitting it. Prototype simulation tests are not production application tests.
