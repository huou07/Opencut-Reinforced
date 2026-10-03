# Testing Strategy

The [model orchestrator V2 candidate](execution/automation/README.md) has a
separate [control-plane acceptance plan](execution/automation/IMPLEMENTATION_PLAN.md).
Its 48 cases require actual process, isolation, Git, transport and hosted
boundaries where declared; helper or fake-adapter success cannot replace them.
This is a frozen architecture candidate, not implemented test coverage or an
activated change to product evidence policy.

## Status

Phase 3 has executable tests for the bootstrap core, CLI, and native bridge. Phase 4UI-1 adds structural widget regression coverage for the Flutter visual foundation. Phase 4F adds file-session, local IPC, and semantic CLI contracts. Phase 4UI-2 adds fake-gateway widget coverage, a native Flutter lifecycle test, and a real attached-CLI process test against the same shared live host. Phase 5A adds media identity, metadata, bounded external-probe, and CLI contract coverage plus a real generated-media `ffprobe` test on hosted Linux CI. Phase 5B adds project-format migration/recovery, media-command/history/query, headless/attached CLI parity, Flutter media-panel, and native offline-media bridge coverage. Phase 5C adds bounded Job Manager and disposable cache foundation coverage. Phase 5D adds production source-fingerprint and artifact-service unit coverage, Flutter preview widgets, native bridge checks, and a hosted real-`ffmpeg` thumbnail/waveform integration test. Phase 5E adds persistent cache-index, reconciliation, LRU eviction, concurrency, project-independence, and artifact-regeneration coverage on hosted Linux, macOS, and Windows. Phase 6A adds timeline-domain validation, strict `.orproj` v3 codec and v1/v2 migration, recovery and storage compatibility, query-wire regression, and referenced-media removal guard coverage. Phase 6B adds application command/history/query coverage, headless and attached semantic CLI tests, save/reopen and undo-save storage checks, recovery snapshot/apply/reload checks, and transaction/precondition regression coverage. Phase 6C adds bridge DTO tests, Flutter widget coverage for read-only track/clip presentation and command interactions, bounded and stale page handling, and a hosted native Rust-host timeline lifecycle test through save/reopen. Phase 6D adds exact trim/split/ripple core invariants, compact history/conflict/overflow/capacity guards, CLI changed-only save and attached dirty-state parity, Flutter action/dialog acceptance, and hosted typed-bridge save/reopen coverage. Phase 6E2A adds marker ID/domain/order/bounds tests, strict v4 codec and v1/v2/v3 migration tests, clean-open/explicit-save storage tests, v1 recovery with v3 and v4 nested snapshots, semantic marker command/history/conflict/transaction/revision tests, bounded marker query and sub-1 MiB page tests, Snap V1/V2 schema/tie/read-only tests, headless/attached CLI parity, and local IPC coverage without changing Flutter marker APIs or UI. Phase 7A adds deterministic `or_runtime` coverage for exact snapshot revision/time identity, frame descriptor and lease ownership, bounded queue pressure/close/cancellation, separate render/audio/decode budget release, and software-first capability/provider selection. Phase 7B adds deterministic `or_render` contract coverage, offscreen wgpu synthetic rendering, row-padded readback normalization, resource-owned frame lifetime, and handle-only viewer presentation checks. Phase 7C0 adds a locked, hosted Linux FFmpeg 8.1.3 shared-library compile/link/load probe under an LGPL-only configuration. 7F1 adds deterministic viewer-mailbox tests and hosted macOS/Linux/Windows builds that compile a desktop packaging probe against production `or_media`, stage official FFmpeg 8.1.3 shared libraries beside that probe in the app bundle, verify loading without developer FFmpeg search paths, and preserve source/build provenance. The shared Flutter bridge remains FFmpeg-independent; Phase 9A0 separately builds and packages the approved Android shared libraries and validates the native runtime on an x86_64 emulator. It does not add FFmpeg dependencies to Android product code or to Flutter widget tests. Phase 7F builds the native Flutter pixel-buffer texture adapters but does not add a product viewer or playback controls. The 7C implementation adds bounded/stale/cancellable `or_media` queues, exact timestamp/seek checks, owned software RGBA frame tests, and a linked audio resample/range test with a tiny generated FFV1/PCM fixture. CI runs the production crate against the approved FFmpeg 8.1.3 shared prefix. Checkpoint 8F adds local export/autosave coverage; its authoritative milestone status remains gated on hosted supervisor evidence. The test layers below distinguish implemented coverage from future product tests.

## Current Phase 3 checks

- Rust unit tests cover core bootstrap values and bridge DTO mapping; CLI contract tests execute the real binary and verify human output, JSON, help, and invalid input.
- The dedicated `apps/or_app/integration_test/native_bridge_diagnostics_test.dart` initializes the native Rust library, calls app info, health, and capability discovery through the typed bridge, and compares the results with the CLI snapshot.

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

GitHub Actions runs Rust formatting, Clippy, and the full workspace test suite; a focused deterministic `or_audio` test; Flutter dependency, formatting, analysis, and widget checks; storage, v1/v2/v3 recovery, real local IPC, shared-host/attached-CLI media parity, and Windows endpoint ACL tests on macOS and Windows; native builds for macOS, Linux, Windows, and Android; native macOS Flutter bridge, project lifecycle, and offline-media integration tests; and Android x86_64 emulator checks. The Flutter static and widget job has a bounded 40-minute timeout: run 36897619300 spent 17m29s installing FFmpeg development libraries and hit the previous 20-minute job timeout before widget tests could run. The Ubuntu 26.04 Rust job installs system FFmpeg tooling for CI-only generated-media tests, logs `ffmpeg -version` and `ffprobe -version`, and explicitly runs the generated-media real-probe and real-artifact integration tests. It separately builds FFmpeg 8.1.3 as LGPL-only shared libraries and runs the bounded Rust binding compile/link/load probe described under Phase 7C0. The macOS, Linux, and Windows desktop jobs independently build that official FFmpeg release from a SHA-512-verified upstream archive using its LGPL 2.1-or-later defaults, without enabling GPL, version3, or nonfree options. Each job records the compiler/toolchain, source tag and archive identity, configure arguments, components, license posture, patch status, and runtime library names; it preserves the source archive and installed shared libraries as a hosted artifact. Desktop Flutter builds compile the texture adapter and FFmpeg-independent bridge with `PKG_CONFIG_PATH` empty. A separate probe that calls production `or_media` is staged beside the bridge with FFmpeg libraries; it checks binding ABI majors and the LGPL license with ambient runtime search paths cleared. The macOS bridge integration tests run after the libraries are packaged. This runner supplies the locked Proxy V1 scale-filter option `reset_sar`; older system FFmpeg versions fail proxy generation without changing the profile. Android CI cross-builds the production FFmpeg probe and software preview bridge against the approved shared profile for all three ABIs, verifies the viewer JNI and FFmpeg libraries in the APK, runs the x86_64 probe with libraries extracted from the APK, and drives the bridge diagnostics plus `apps/or_app/integration_test/android_preview_surface_test.dart` on API 36 x86_64 SwiftShader. It records the API/GPU path, queue budgets, cancellation and fallback telemetry, and `ANDROID_HARDWARE_MEDIA=UNVERIFIED`. The emulator step keeps its 30-minute timeout, bounds `adb wait-for-device` to 120 seconds, and preserves the emulator log. It does not run IPC or the project-storage integration test on Android.

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

Filesystem save/load tests are recorded under Phase 4E1. The initial v1-to-v2 project migration and current-format codec coverage are recorded under Phase 5B below; further migrations remain future work.

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

Filesystem save/load tests are recorded under Phase 4E1. Phase 5B migration and media-query coverage is recorded below; Phase 4F records IPC and client-integration coverage.

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
- The native lifecycle test captures project-creation completion before the UI action, pumps once after confirmation, and awaits the Rust bridge operation through `WidgetTester.runAsync` before waiting for the loaded empty media and timeline panels. It uses the same bounded panel wait after reopening the project; the native preview transport test uses it after creation. This checks workspace loading without allowing indeterminate indicators to hold hosted tests indefinitely, and the native creation timeout includes bridge-error diagnostics.
- Hosted macOS native Flutter tests pass Flutter's `--ci` flag so Flutter builds the test app with sandboxing disabled. Flutter documents that an unsigned sandboxed app can trigger a permission prompt and stall CI; the `Failed to foreground app; open returned 1` diagnostic is a separate post-attach foreground attempt.
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

## Current Phase 5B coverage

- `.orproj` codec tests cover v1-to-v2 migration with exact project ID/revision/name preservation and an empty library, v2 empty and media round trips, strict unknown-field rejection, malformed/oversized URI and metadata values, duplicate IDs/sources, and deterministic media order. A clean v1 open is checked not to rewrite disk; explicit save emits v2 without a conversion-only revision increment.
- Recovery tests cover the v1 recovery envelope with old/new nested project formats, including cross-schema saved-base and recovery snapshots, while preserving exact-base conflict checks.
- Media domain/import tests cover local `file:` URI validation and byte bounds, metadata persistence bounds, source canonicalization before probing, generated UUIDv4 identity, missing/directory paths, and source loss after import. A tiny generated-media integration fixture verifies import/save/reopen when the source is offline.
- `media.add`/`media.remove` tests cover exactly one revision increment on success, zero changes on malformed/stale/duplicate/missing failures, transaction rejection, semantic error codes, `ChangeSet` contents, and undo/redo restoring item identity and original order without full-project snapshots.
- `media.list` tests cover deterministic insertion order, read-only behavior, page sizes up to 100, all pages and `next_offset`, valid out-of-range empty pages, invalid bounds/argument shapes, offline source listing, and unchanged `project.summary` result shape.
- `crates/or_cli/tests/semantic_cli.rs` covers headless media add/list/remove and save, plus attached list/add/remove against one live host, shared history/revision, explicit save, and stale-revision rejection without retry. CLI output uses OR-structured values rather than raw ffprobe JSON.
- Flutter fake-gateway widget tests cover Media panel fields, import and invalidation refresh, page offsets 0 and 50, exact backend-unavailable text, and confirmed removal. The widget suite contains 31 passing tests.
- Hosted macOS runs `native media bridge persists offline media through undo and save` alongside the existing native project lifecycle test. It verifies native bridge list/remove/undo, explicit save, reopen persistence, and success when the referenced source is absent. Hosted Rust/CLI jobs also cover migration and attached shared-host parity.
- Local native/runtime verification remains `NOT RUN — LOCAL NATIVE EXECUTION DISALLOWED BY POLICY`; hosted macOS Actions supplies the native evidence.

## Current Phase 5C coverage

Job Manager unit tests (`crates/or_core/src/jobs/manager.rs`) use synchronization primitives rather than sleeps and cover:

- a fixed worker bound: with `max_workers = 2`, exactly two jobs are `Running` and the other submitted jobs remain `Queued`, so concurrency never exceeds the configured count
- queue backpressure: a full bounded queue returns `JobSubmitError::QueueFull` without blocking or spawning a thread
- queued cancellation: a queued job cancelled before execution never runs and is `Cancelled`
- running cooperative cancellation: a running job observes `JobContext::is_cancelled` and finishes `Cancelled`; no unsafe thread termination is used
- success (`Queued → Running → Succeeded`) and failure (`Err(JobFailure) → Failed`) with the worker continuing afterward
- panic containment: a panicking task is `Failed` and the pool survives to run a later job
- record bound: completed jobs beyond `max_records` reclaim the oldest terminal record; `record_count` never exceeds the bound
- active record capacity: when all records are non-terminal, a submit returns `JobSubmitError::RecordCapacityExceeded`
- monotonic manager-local `sequence` (no wall-clock ordering)
- shutdown: running jobs observe cancellation, workers are joined, and later submissions return `JobSubmitError::Shutdown`

Cache unit tests (`crates/or_core/src/cache.rs`) cover:

- cache-key determinism; separation by artifact kind, source fingerprint, parameters fingerprint, and schema version; and a 64-character lowercase-hex path-safe format
- exact round trip; normal miss; `EntryTooLarge` rejected before any file is created; an externally oversized/corrupt entry reading as a controlled error with a bounded read
- `BudgetExceeded` when an artifact itself cannot fit the total budget, with no unrelated eviction; indexed replacement accounting that counts an existing key once, not twice
- idempotent remove; clear-namespace preserving the other namespace; clear-all leaving the store usable
- atomic failure leaving no partial final entry; concurrent same-key writers producing one complete payload
- a Unicode and spaced root path; and a project-independence check confirming job and cache activity does not change `ProjectRevision` and writes no `.orproj` file

The Rust workspace job runs the cache tests on Linux; Phase 5E also runs `cache::tests` explicitly on hosted macOS and Windows. No local native/runtime test is used for cache verification. Local native/runtime verification remains `NOT RUN — LOCAL NATIVE EXECUTION DISALLOWED BY POLICY`.

## Phase 5D coverage

- Source-fingerprint tests cover stable small-file content hashing, changed bytes in a large file, bounded large-file sampling, and errors for unavailable or non-regular sources. The fingerprint includes file size, available modification time, and sampled bytes; it is used only for disposable cache invalidation.
- `MediaArtifactService` tests cover locked profile-key separation, valid cached PNG generation for both profiles, terminal event ordering, ready cache hits without a second job, same-key in-flight deduplication, unsupported streams without spawning `ffmpeg`, queue and record backpressure, cache-budget failure, timeout/cancellation, bounded output, malformed PNG rejection, and child cleanup.
- Bridge tests cover desktop cache-root policy. The hosted macOS offline-media bridge test checks thumbnail and waveform requests return `notApplicable`, artifact work does not change the project revision, and malformed cache keys are rejected.
- Flutter widget tests check supported video/audio requests, unsupported media, cached previews, success events loading PNGs, neutral failure placeholders, media removal clearing the preview row, and unchanged project revision.
- The ignored `crates/or_core/tests/media_artifacts_integration.rs` test runs only on hosted Linux CI. It generates a tiny video/audio Matroska fixture with the CI-installed system `ffmpeg`, verifies real thumbnail and waveform PNG dimensions, confirms cache hits do not create another job, and changes sampled source bytes while preserving size and modification time to verify cache-key invalidation. It also checks that artifact generation leaves project revision unchanged. Do not install FFmpeg locally or run this real-media integration test locally.
- Hosted macOS CI runs the native Flutter project lifecycle and offline-media bridge tests. Local native/runtime verification remains `NOT RUN — LOCAL NATIVE EXECUTION DISALLOWED BY POLICY`.

## Current Phase 5F and cache-index coverage

- Lazy index creation and exact schema-v1 table/index shape; existing thumbnail and waveform cache files and canonical proxy `.mkv` files are discovered without regeneration.
- Reopen reconciliation preserves access sequences, repairs size drift, adds orphan files, removes missing-file rows, and rebuilds corrupt, unsupported-version, or inconsistent index metadata without deleting artifacts.
- Bounded artifact scans return `IndexTooLarge`; symlinked cache entries are not followed. Unknown files and the SQLite index do not consume the managed artifact-byte budget.
- Successful `get` and `put` access ordering persists across restarts, uses no clock, and breaks sequence ties by kind then cache key. Eviction removes only the oldest minimum set required, protects the replacement target, and leaves newer unrelated entries intact.
- An artifact larger than the total budget returns `BudgetExceeded` without evicting existing entries. Remove and clear operations update rows, preserve unrelated namespaces and unknown files, and leave the index usable.
- Cloned stores serialize budget changes; independent stores coordinate through SQLite; a held database lock returns within the one-second busy bound. Sequence exhaustion is controlled and does not wrap.
- Proxy file API tests cover same-directory reserved staging, files larger than the 8 MiB preview byte limit, byte API rejection, index repair and LRU touch after reopen, remove/clear, atomic replacement with target protection, global LRU participation, budget rejection, and active-stage preservation during clear.
- Proxy service tests cover the exact profile descriptor and stable existing thumbnail/waveform cache keys, no-video handling, file-backed success and cache hit without a new job, same-key in-flight deduplication, shared queue backpressure, cancellation/reap, duration-aware timeout bounds, output growth limit, non-zero exit, invalid EBML output, and normal staging cleanup.
- Cache/index and artifact operations leave `ProjectDocument` and `ProjectRevision` unchanged. Existing thumbnail/waveform tests remain in the same suite and continue to verify their event, cache, cancellation, and error behavior.
- The ignored `system_ffmpeg_generates_v1_video_only_proxy_and_preserves_vfr_timing` integration runs only on hosted Linux CI. It generates a tiny VFR video/audio source, requests a proxy through production `MediaArtifactService`, checks Matroska/`mpeg4`/video-only/`yuv420p`/dimensions/duration with `ffprobe`, compares frame ordering and relative timestamps, confirms a ready cache hit without a job, and changes the source to verify a new key and generation. The existing ignored thumbnail/waveform integration remains in that same hosted Linux command. Do not install FFmpeg locally or run these real-media integration tests locally.
- Hosted macOS and Windows run the file-backed cache and proxy fake-executable tests through `cache::tests` and `media_artifacts::tests`; Android verifies the Rust-backed APK build and does not run proxy generation. Local native/runtime verification remains `NOT RUN — LOCAL NATIVE EXECUTION DISALLOWED BY POLICY`.

## Phase 6A, 6B, 6C, 6D, and 6E1 coverage

- Timeline unit tests cover UUIDv4 `TrackId` and `ClipId` parsing/serde, Video/Audio-only kinds, empty default state, track and clip bounds, globally unique IDs, media existence and kind compatibility, nonnegative starts, positive duration, checked end arithmetic, known source-duration bounds, unknown-duration acceptance, ordered clips, same-track overlap rejection, adjacency, allowed cross-track overlap, and valid reuse of one media item across tracks.
- Strict project codec tests cover v5 round trips for empty and populated timelines, markers, and exact sequence rates; v1–v4 migrations set the rate to `null` and preserve identity, revision, media, and timeline state. V5 requires the nullable rate field and rejects malformed or extra rational fields.
- `ProjectFileSession` tests verify v1–v4 opens remain clean and do not rewrite source bytes; explicit save writes v5 without a conversion-only revision change. Storage tests retain bounded-load and atomic-save regressions.
- Recovery tests retain the v1 envelope and cover legacy v1–v5 nested snapshots, including the exact sequence rate, inspection/apply, timeline preservation, and revision integrity; the Phase 9A coverage below adds current schema-v7 snapshots.
- Application/bridge tests verify referenced `media.remove` returns stable `MEDIA_IN_USE` without changing project, revision, undo history, or redo history; existing unreferenced removal behavior remains covered. Query regressions assert `project.summary`, `media.list`, and `media.get` do not expose timeline fields. Phase 6B extends the catalogs while preserving the existing IPC v1 route and project/media command contracts.
- Phase 6B application tests in `crates/or_core/src/application.rs` cover track add/remove ordering and limits, clip insert ordering/range/media checks/overlap/adjacency, project-wide clip IDs and clip limits, same-kind and cross-kind moves, move no-op and redo preservation, delete and `MEDIA_IN_USE`, bounded read-only queries, strict arguments, transaction rejection, stale preconditions, undo/redo chains, redo invalidation, history conflicts, and revision overflow.
- Phase 6D application tests in `crates/or_core/src/application.rs` cover absolute exact trim, earlier extension, known source bounds, no-op history preservation, split partitioning and project-wide ID uniqueness, capacity-before-mutation, track-local ripple shifts and gap preservation, compact public ChangeSets, history conflicts, strict arguments, transaction rejection, and revision overflow. `crates/or_core/tests/project_storage.rs` verifies advanced timeline edits and sequence settings save/reopen exactly as schema v5 without persisting history. It also verifies saving after undo persists the current canonical state and revision.
- Phase 6E1 application tests in `crates/or_core/src/application.rs` cover the exact `timeline.snap` argument contract, fixed `1/8` threshold, timeline-zero and all-canonical-boundary candidates, active-clip exclusion, move start/end anchors, trim-edge resolution, deterministic tie-breaking, unpaged canonical scans, same-kind move preconditions, read-only revision/history behavior, and stale/invalid argument rejection.
- `crates/or_core/tests/project_recovery.rs` verifies dirty timeline commands survive the existing v1 recovery envelope, snapshot inspection, apply, and reload without changing the envelope version.
- `crates/or_cli/tests/semantic_cli.rs` verifies headless trim/split/ripple and sequence-setting commands/queries use the application path and v5 storage, `timeline snap` works in headless and attached read-only forms, exact rational parsing rejects rounded or malformed values, changed-only headless saves preserve exact no-op revisions, and attached CLI operations share the live host's instance, revision, history, dirty state, and explicit save behavior.
- `crates/or_ipc/tests/live_host.rs` retains shared-host command, history, dirty state, save, invalidation, and stale-revision contracts. It verifies sequence settings use generic `ApplicationRequest` over IPC v1; catalog tests cover the schema-v1 timeline operations and transaction tests assert timeline commands are rejected.
- `crates/or_app_bridge/src/api/project.rs` tests ensure bridge track/clip views preserve identity, kind, page metadata, and exact rational numerator/denominator values; the bridge adapter and generated bindings expose typed trim/split/ripple methods with Rust-side split ID generation and typed `timeline.snap` operation/result metadata.
- `apps/or_app/test/widget_test.dart` covers the real project empty state, canonical Video/Audio track labels/order and empty-track removal, clip insertion and proportional visualization, exact tooltip values, stream-specific duration defaults, unknown-duration entry, no-compatible-track guidance, exact rational parsing, same-kind move target filtering, explicit delete, media preservation, failed mutation behavior, and the absence of toolbar placeholder controls. It also covers the exact Move/Trim/Split/Delete/Ripple Delete action menu, current-timing trim/split dialogs, edge switching without overwriting intentional input, strict interior split input, ripple confirmation wording, pointer move drop-time snap and toggle-off behavior, original-time nearest-ms quantization without accumulated rounding, trim-handle priority and exact edge operations, same-kind lane targeting/opposite-kind rejection, attached-session stale snap invalidation, Rust-gateway callbacks, dirty project events, attached-session refresh, stale dialog rejection, bounded 100-clip pagination, stale-page discard/refresh, project close/switch clearing, and compact populated layout.
- The hosted-only native test in `apps/or_app/integration_test/core_bridge_test.dart` exercises the Rust bridge against one live project host: Rust-generated v4 track/clip IDs, exact rational queries, insert, undo/redo, move/no-op, trim, split, track-local ripple delete with undo/redo, explicit save, reopen, and empty history after reopen. It is compiled by local analysis but native Flutter integration execution remains CI-only.
- Hosted Linux CI continues to run the generated-media real-`ffprobe` test and real-`ffmpeg` thumbnail, waveform, and Proxy V1 integrations. Hosted macOS, Linux, Windows, and Android jobs provide the repository's canonical platform/build verification.
- Local native/runtime verification remains `NOT RUN — LOCAL NATIVE EXECUTION DISALLOWED BY POLICY`.

## Phase 6E2A coverage

- `crates/or_core/src/timeline.rs` covers generated/canonical UUIDv4 `MarkerId` values, malformed and non-v4 rejection, empty/default marker state, nonnegative time, no upper time bound, exact surrounding whitespace, Unicode labels at the 256-byte boundary, blank/oversized labels, duplicate IDs, same-time duplicate labels, canonical time/ID ordering, and the 10,000-marker bound.
- `crates/or_core/src/project_document.rs` covers strict v5 marker and sequence-rate fields, exact rate round trips, v1–v4 migrations with an unset rate, bounded marker deserialization, and unknown/invalid data. `crates/or_core/src/project_file_session.rs` verifies clean v1–v4 opens, no immediate rewrite, and explicit v5 saves without conversion-only revision increments.
- `crates/or_core/src/application.rs` covers all four marker commands, canonical insertion/reordering, exact labels, duplicate/not-found/invalid/limit errors, missing-marker and revision-overflow atomicity, semantic `ChangeSet` recipes, add/move/rename/delete undo/redo, history conflicts, no-op move/rename redo preservation, transaction rejection, clip-edit marker stability, bounded canonical marker paging, strict page arguments, unchanged existing `QueryResult` shapes, and a 100-marker 256-byte serialized page below 1 MiB.
- The same application suite proves Snap V1 ignores markers and omits marker-only wire fields, Snap V2 accepts marker candidates for move/trim, preserves the 1/8 threshold and clip/zero tie rules, orders same-time markers canonically, scans markers independently of query pages, remains read-only, accepts schemas 1 and 2, rejects schema 3, and leaves other query schema rules unchanged. Catalog tests advertise `timeline.snap` v2 and `timeline.markers` v1.
- `crates/or_core/tests/project_storage.rs` covers marker and sequence-rate save/reopen with v5 output; `crates/or_core/tests/project_recovery.rs` covers v5 sequence-rate checkpoints and older nested project migrations while retaining recovery schema v1. `crates/or_ipc/tests/local_transport.rs` covers generic marker commands/query/save and the 100-marker page frame bound without changing IPC v1.
- `crates/or_cli/tests/semantic_cli.rs` covers headless and attached marker list/add/move/rename/delete, generated IDs, exact rational parsing, changed-only headless saves, attached dirty-until-save behavior, and CLI Snap V2 marker reporting. No Flutter marker read model, bridge API, or Snap V2 GUI coverage is expected in this phase.

## Current Phase 7A coverage

`crates/or_runtime` unit tests remain headless and standard-library-only. They
verify that `RenderSnapshot` preserves the exact requested rational range and
captured project revision without mutating `ProjectDocument`; descriptors carry
explicit memory domain, dimensions, pixel format, color, access, and exact
timing; software leases own buffers and runtime leases release exactly once;
bounded queues return backpressure, cancel blocked work, and drain on close;
render/audio/decode budgets enforce independent in-flight and byte limits with
release-on-drop; and the centralized capability registry prefers only stable
available hardware while reporting software fallback or a deliberate
hardware-only failure. No full-rate frame is copied through Dart, and no
platform/native runtime is launched locally. Phase 9B adds hosted Android
surface presentation coverage; local native Flutter runtime remains prohibited.

The workspace dependency gate is also covered by source review: `or_runtime`
depends only on `or_core`, uses the workspace MSRV Rust 1.89 and MIT license,
and adds no wgpu, FFmpeg, audio, provider, or platform dependency. Checkpoint
7C0 separately approves the FFmpeg binding/package strategy and its hosted
probe below; other future runtime dependencies require official version, MSRV,
license/build, and hosted platform evidence before pinning.

## Current Phase 7B coverage

`crates/or_render` keeps its tests headless and render-core scoped. They verify
handle-only viewer contracts, immutable render-graph inputs tied to
`RenderSnapshot`, deterministic wgpu synthetic solid-color rendering,
256-byte row-padded readback normalization, GPU-target ownership through the
`RenderedFrame` lifetime, and the exact `wgpu 25.0.2` dependency gate. The
offscreen readback test uses a runtime adapter when one is available and does
not launch Flutter or a native OR application. Viewer presentation, media
decode, native interop, and copied Dart frame transport remain outside 7B.

## Phase 7F0 timing and viewer contract coverage

`or_core::RationalRate` tests exact checked floor/ceiling frame-index
conversion and frame-time conversion. `ProjectTimeline` tests the global
sequence lattice, half-open content end across audio/video tracks, marker
exclusion, no-frame empty timelines, and next/previous selection at exact
boundaries. Application tests cover the nullable settings query, strict set or
clear command, revision/history behavior, no-op preservation, transaction
rejection, and generic `ApplicationRequest` dispatch. Schema-v5 codec, clean
migration/save, storage, and recovery tests retain the explicit unset rate for
v1–v4 projects. Headless/attached CLI and hosted IPC-v1 tests cover the same
command/query path. The Phase 7 viewer contract remains handle-only; Phase 9B
adds Android surface presentation behind that contract. Local native Flutter
runtime verification remains `NOT RUN — LOCAL NATIVE EXECUTION DISALLOWED BY POLICY`.

## Phase 7F1 media and texture packaging gate

`or_runtime` tests the bounded viewer mailbox, RGBA-to-premultiplied-BGRA
conversion, stale generation rejection, and the three-lease in-flight bound.
`or_app_bridge` exposes the small native C ABI used by the viewer adapters.
7F initially added the `or_media` dependency only for desktop targets. Phase
9A0 built and packaged Android FFmpeg shared libraries, and 9B enables that
approved decoder in the Android bridge. The Flutter package registers a
pixel-buffer texture adapter on macOS, Linux, Windows, and Android. Android
copies bounded software pixels to a reusable bitmap and `SurfaceProducer`;
desktop adapters retain their platform release callbacks.
The Android bridge build-hook environment test checks that each ABI selects
its matching staged FFmpeg prefix and passes the cross-compilation settings to
Cargo. Hosted Android verification checks all three resulting libraries in
the APK and runs its x86_64 software-preview bridge path.
CI grants the ephemeral runner read/write access to `/dev/kvm` for hosted
x86_64 emulator verification.
Linux converts the leased BGRA pixels into a bounded native-owned RGBA ring and
releases the source lease when the synchronous copy completes; the owned
buffers stay available to Flutter through the next render tick and are freed
only as the ring advances or when the texture unregisters.

Hosted macOS, Linux, and Windows jobs build the official FFmpeg 8.1.3 source
archive with a configure-help-verified LGPL shared profile. It disables
`libavdevice` and `libavfilter`, leaving the five shared runtime libraries
`libavcodec`, `libavformat`, `libavutil`, `libswresample`, and `libswscale`.
The configure defaults keep GPL, version3, and nonfree disabled.
They verify the source archive SHA-512 and place the production `or_media`
probe and shared libraries beside the bridge in the app bundle. The 7F desktop
app builds provide the same FFmpeg prefix to the Flutter Rust bridge; Android
keeps the bridge dependency excluded while its package gate checks native
libraries separately. The probe checks ABI majors and license with
ambient FFmpeg search paths removed. Each
job uploads the verified source archive, installed libraries, and a
target-specific record of the toolchain, exact configure arguments, components,
license posture, patch status, and runtime names. Local native Flutter runtime
verification remains `NOT RUN — LOCAL NATIVE EXECUTION DISALLOWED BY POLICY`.

## Phase 7F preview transport coverage

`or_runtime::transport` tests exact seeks, explicit-rate requirements,
half-open playback ends, frame-lattice stepping, monotonic-clock updates, and
stale generations. `or_media` verifies the preceding source presentation
timestamp, and `or_render` exercises GPU composition of decoded RGBA layers.
Flutter widget tests check exact scrubbing, frame-rate selection, frame-step
controls, and the no-playable-frames empty state against a fake gateway. The hosted macOS native integration test
checks texture registration, an exact `7/15` seek through the Rust host without
changing project revision, a rendered frame sequence/dimensions, play rejection
while the sequence rate is unset, and configuration of an explicit `24/1` rate
through the project command.
Native Flutter runtime execution remains `NOT RUN — LOCAL NATIVE EXECUTION DISALLOWED BY POLICY`.

## Phase 7C0 coverage

`tools/ffmpeg-link-probe` is an out-of-workspace CI tool with an exact
`ffmpeg-the-third` 6.0.0 package pin and locked Rust dependency graph. Hosted
Linux CI built official FFmpeg 8.1.3 shared libraries in a temporary prefix
with autodetection and media components disabled and no GPL, GPLv3, or nonfree
enabling options for the 7C0 probe.
The probe requires `pkg-config` to discover those exact libraries, compiles bindgen from
their installed headers, links the Rust binary, loads the shared libraries,
and checks the expected ABI majors plus the LGPL 2.1-or-later license string.
The build and probe steps have explicit time limits. The current 7C workflow
uses the LGPL-only dynamic configuration with only the `file` protocol, Matroska
demuxer, FFV1, and PCM S16LE enabled, then exports that prefix to production
workspace checks. A bounded linked decode test verifies software video seek,
audio seek, decoding, resampling, exact range clipping, and cross-platform
classification of FFmpeg's POSIX `EAGAIN` decoder-drain status. The test uses no
global FFmpeg serialization lock. FFmpeg source builds
and the binding probe are hosted checks; native application execution remains
disallowed locally by policy.

## Current Phase 7D evaluation coverage

The 7D review found no enabled hardware decoder or native-frame interop adapter
to verify. The software decode integration test checks an exact-time seek,
owned RGBA pixels, and budget release; `or_media` also tests that FFmpeg's
POSIX `EAGAIN` is treated as normal decoder-drain backpressure, including on
Windows. `or_runtime` tests cover centralized
stable-hardware selection, software fallback when no hardware is available,
metadata-only hardware-frame leases, and exactly-once release; these contracts
do not represent a platform decoder or native surface. No target-hardware
benchmark or hardware performance claim is recorded. Therefore no hardware
path is approved, and software decode remains the correctness path until a
candidate has platform build/license/package evidence, interop lifetime and
fallback checks, and repeatable target-hardware measurements.

## Current Phase 7E coverage

`or_audio` tests exact rational timeline-to-device flooring relative to a
`RenderSnapshot`, callback clock advancement through silence, bounded ring
backpressure and wraparound, cancellation, and exact video wait/drop/present
decisions. Synchronization rejects a clock message tied to another project or
revision while allowing separate audio and video request ranges. A cross-thread producer/callback test checks
ordered delivery, and a dedicated integration test asserts that rendering a
device block performs no allocation. The Rust CI job runs `cargo test --locked
-p or_audio`; no audio device or native output backend is selected or exercised.

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

Future MotionScene and procedural-motion coverage is checkpoint-scoped:

- **11E:** strict schema and unknown-field rejection, resource bounds, stable
  node/asset/animation identity, exact `RationalTime`, negative and
  out-of-range timing, random-access and seek equivalence, deterministic
  same-time evaluation without wall-clock dependence, no network lookup,
  missing-asset/font diagnostics, semantic hash/version identity,
  RenderSnapshot lowering, and preview/materialization semantic parity.
- **11F:** CLI validate/inspect/render, machine-readable diagnostics,
  external-agent fixtures, persistent generated media, cache-independent
  project references, new content identity on re-render, normal `media.add`
  and timeline insertion, save/reopen, recovery, offline sources, and
  cancellation/failure behavior.
- **14G:** provider-independent task contracts, valid/invalid scene proposals,
  local/cloud capability selection, provider/model/source provenance, stale
  revision rejection on apply, explicit review/materialization, normal project
  mutation, and proof that rendering makes no model call.
- **16G:** network and credential denial, bounded approved inputs and outputs,
  timeout, cancellation, crash containment, filesystem/project-write denial,
  process isolation, no execution on project open, materialization-only output,
  unsafe browser configuration rejection, and normal media validation.

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

Phase 7G adds deterministic `ResourceBudget::metrics()` snapshots for current
and peak in-flight resources/bytes, successful acquisitions, and in-flight or
byte-budget rejections. These counters use no clock and send no telemetry. The
hosted `software_decode` fixture test records one 16×16 RGBA frame (1,024
bytes), one occupied slot in a four-item queue, a peak decode budget of one
lease/1,024 bytes, and zero usage after release. Existing audio conformance
tests report exact underrun frames and sample counts. These are repeatable
resource and semantic measurements; they do not measure latency or throughput.

`DEDICATED_HARDWARE_PERFORMANCE = UNVERIFIED`: no dedicated or self-hosted
benchmark machine is documented for this checkpoint. Shared hosted-runner
timings are not used to claim FPS, GPU speed, hardware-path benefit, or a
performance regression threshold. Future local diagnostic instrumentation may
measure decode latency, render CPU/GPU time where available, present latency,
dropped frames, audio underruns, queue depth, memory, cache hit/miss, and export
throughput. This remains local performance diagnosis, not telemetry or network
reporting.

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

## Execution-plan infrastructure

The standard-library-only execution infrastructure has focused tests for valid
plan/state validation, duplicate IDs, missing dependencies, cycles, multiple
`NEXT` states, unfinished prerequisites, unknown state IDs, grandfathered
pre-boundary completion, required evidence after the 7A boundary,
dirty-worktree refusal, non-zero runner exit, protected runner-state mutation,
exact-SHA workflow and required-job matching, wrong branch/event/conclusion,
rate/token behavior, timeout behavior without real sleeps, preview release
contracts, exact resume guards, metadata-only platform filtering, and
exactly-one-checkpoint transitions. The repository hygiene check runs both the
execution-plan and architecture-policy checkers and requires the evidence
policy, evidence directory documentation, and evidence module. These checks
use fake API data and do not mutate the real plan or state.

The maintenance guard also locks the current post-7F0 transition
`7F0 -> 7F1 -> 7F` and the later `8F -> 9A0 -> 9A` Android gate, verifies the
milestone membership/order and PLANNED/NEXT state, asserts exact protected
workflow paths for platform gates, and ensures no 7F1 or 9A0 evidence is
fabricated. Architecture-policy tests ensure `or_core` remains barred from
`cosmic-text`, `wasmi`, and Whisper wrapper dependencies. Preview release
fixtures require eleven non-empty assets, including
`FFMPEG-BUILD-INFO.txt` and `ffmpeg-8.1.3-source.tar.xz`.

Hosted evidence is not inferred from workflow-level success alone. The
supervisor requires push-triggered `Repository hygiene` and `Platform
verification` runs whose `head_sha` is the implementation SHA, then checks the
policy-required job names and completed-success conclusions. Product CI
failures leave the checkpoint `NEXT` and create no completion evidence. The
state/evidence completion commit intentionally skips Platform verification via
the metadata-only push filter, but still runs Repository hygiene; the
hardening commit itself must pass both workflows.

## Phase 8A typed project model and timeline contract coverage

- Timeline model tests cover Video, Audio, Text, and Caption tracks; Media, Text, and Caption clip content; stable IDs; exact starts and positive exact durations; media source-range preservation; bounded text and typed clip settings; per-kind track-state rules; solo evaluation by medium; and locked-track mutation rejection.
- Strict project codec tests cover schema-v7 round trips, unknown-field and closed-enum rejection, typed setting bounds, SAF source validation, and v5-to-v7 migration with stable media/clip identity, exact source duration, default state/settings, and preserved project revision. The v6 decoder remains covered as a legacy input; earlier v1–v4 migration and clean-open behavior remain covered.
- Application tests cover typed insert/update commands, typed v2 track/clip queries, exact duration-based move/trim/split/ripple behavior, validation atomicity, persistent track state, undo/redo, and unchanged IPC protocol v1 dispatch.
- `crates/or_cli/tests/semantic_cli.rs` covers text and caption insertion, all four track kinds, persistent track-state changes, typed query output, schema-v7 persistence, and the shared application path.
- Recovery tests verify that the recovery sidecar remains schema v1, preserves typed text content in a nested schema-v7 snapshot, and inspects/applies/reloads it without changing project identity or revision. Legacy nested snapshots still load and save as schema v7.
- `crates/or_ipc/tests/live_host.rs` exercises typed timeline commands and queries through the existing generic IPC v1 route. The original 8A Flutter bridge view was media-only; the 8D bridge now exposes media, text, and caption clip content without changing IPC or project schema contracts.
- Local native/runtime verification remains `NOT RUN — LOCAL NATIVE EXECUTION DISALLOWED BY POLICY`.

## Phase 8B timeline usability coverage

- `crates/or_app_bridge/src/api/project.rs` tests preserve typed Video, Audio,
  Text, and Caption track kinds plus lock, visibility, mute, and solo state in
  the v2 track read model. The bridge exposes the existing typed track-state
  command without changing IPC or project schema contracts.
- `apps/or_app/test/widget_test.dart` covers persistent track visibility/mute,
  lock, and solo commands; single-clip selection, Ctrl+D, Escape, and confirmed
  Delete-key flow; duplicate insertion at the exact clip end with its source
  range retained; view-only zoom and Fit; and compatible media drag insertion
  with exact pointer time and stream duration. Incompatible drops are
  rejected. Existing timeline editing, paging, marker, stale-revision, and
  compact-layout regressions remain in the same suite.
- The hosted-only native bridge test verifies typed track-state commands and
  v2 track-state readback across save/reopen. It compiles locally through
  Flutter analysis; native Flutter runtime execution remains CI-only. Local
  native/runtime verification is
  `NOT RUN — LOCAL NATIVE EXECUTION DISALLOWED BY POLICY`.

## Phase 8C video transform foundation coverage

- `or_core` verifies exact visual-setting updates, range rejection without
  project/history mutation, undo/redo, and schema-v7 encode/decode. `or_render`
  checks crop, opacity, translation, scale, rotation, and anchor behavior
  through wgpu readback when a headless adapter is available.
- Flutter widget coverage selects a video clip, edits transform values through
  the Inspector, and dispatches the visual-settings update through the project
  gateway. `flutter analyze` checks the generated bridge consumer and hosted
  integration-test adapter.
- The hosted native bridge lifecycle test reads defaults, updates every
  visual-setting field, verifies revision behavior, saves/reopens, and reads
  the same values. Native Flutter runtime execution remains
  `NOT RUN — LOCAL NATIVE EXECUTION DISALLOWED BY POLICY`.

## Phase 8D basic text and manual-caption coverage

- `or_render::TextRasterizer` tests load exactly the four bundled Inter faces,
  produce deterministic transparent RGBA for multiline text, preserve the
  configured color and alpha, and reject invalid dimensions or an oversized
  canvas. The font files and OFL notice are checksum-pinned in
  `docs/TECHNICAL_PLAN.md`.
- `or_app_bridge` DTO tests cover Media, Text, and Caption read models with
  optional media/source fields, formatting, stable IDs, and exact rational
  starts and durations. The hosted Rust bridge integration test inserts and
  edits a title, then inserts a manual caption and reads the persisted typed
  values through the generated Flutter Rust Bridge.
- Flutter widget tests add text and caption tracks, create a title at the
  preview playhead, edit its content and exact duration, and create a manual
  caption through the fake gateway. `flutter analyze` checks the generated
  binding consumer and integration-test adapter.
- Active preview overlays use the shared bundled-font rasterizer with bounded
  canvas memory; no automatic transcription, arbitrary font path, or font scan
  is introduced. Export reuses this text path when 8F implements the exporter.
- The widget and Rust unit suites may run locally. The Flutter integration
  test that launches the native app remains hosted-only;
  `LOCAL NATIVE VERIFICATION: NOT RUN — LOCAL NATIVE EXECUTION DISALLOWED BY POLICY`.

## Phase 8E basic audio and effects coverage

- `or_audio` tests deterministic gain, linear pan, fade envelopes, invalid
  ranges, exact device-clock conversion, underrun silence, bounded SPSC
  transfer, and the callback's no-allocation contract. The desktop stream is
  compiled against cpal 0.18.1 but no physical audio device is needed by CI.
- `or_render` tests byte-domain brightness and contrast, Rec.709 saturation,
  bounded blur, and the alpha, black-fade, and wipe transition semantics.
  `or_app_bridge` tests exact rational audio/effect/transition DTO conversion.
- Flutter widget tests exercise the visual-effect/transition Inspector and the
  audio Inspector's gain, pan, and exact fade edits. The hosted bridge
  lifecycle tests save and reopen typed visual effects, transitions, and audio
  clip settings.
- Hosted Linux builds install `libasound2-dev` for cpal's ALSA compile path;
  macOS and Windows hosted builds compile their native host backends. These
  checks do not open an output device. Local Rust tests, Flutter analysis, and
  widget tests are allowed; native Flutter runtime execution remains
  `NOT RUN — LOCAL NATIVE EXECUTION DISALLOWED BY POLICY`.

## Phase 8F export, autosave, and Desktop MVP hardening

- `or_core` tests cover dirty recovery-sidecar creation and replacement,
  canonical-file byte preservation, explicit-save promotion of the exact
  checkpoint, and protection against stale or conflicting recovery bases.
  Job tests cover the bounded export kind, monotonic progress snapshots,
  cancellation, and visible failure diagnostics.
- `or_media/tests/software_export.rs` writes and reopens the fixed Matroska +
  FFV1 + PCM S16LE profile, verifies both stream codecs, and checks incomplete
  export cleanup and private staging-directory permissions. The hosted
  platform workflow builds only the required muxer and encoders in the LGPL
  FFmpeg 8.1.3 configuration and runs this linked integration test.
- `or_ipc/tests/live_host.rs` verifies that export start, status, and cancel use
  the same read-only application handler through direct and IPC requests and do
  not change project revision. Flutter widget coverage exercises export
  progress/cancel and periodic recovery status; recovery tests cover save/reopen
  of typed project content and persistent markers.
- `or_app_bridge` uses the same timeline loader and render resources for
  preview and export, including bundled-font text, visual settings, effects,
  transitions, and audio mixing. Android preview uses software FFmpeg decode,
  offscreen wgpu composition, and a bounded pixel copy into a reusable bitmap
  presented through Flutter `SurfaceProducer`. Rust and Flutter unit/widget
  checks are local; native Flutter runtime execution remains
  `NOT RUN — LOCAL NATIVE EXECUTION DISALLOWED BY POLICY`.

## Phase 9A Android SAF and project/media I/O coverage

- `or_core` tests strict `FileUri` and `AndroidSafDocumentUri` identities,
  preserves the existing `local_file` serialization, rejects malformed and
  oversized SAF URIs, and verifies schema-v7 SAF sources round-trip without
  opening media or producing a filesystem path. Schema v1–v6 remain accepted;
  schema v6 rejects the new source kind.
- `or_media/tests/software_decode.rs` exercises seekable descriptor-backed
  FFmpeg input while retaining the descriptor for the decoder lifetime. The
  runtime capability carries no project data and does not materialize media.
- `apps/or_app/test/project_file_picker_test.dart` checks the SAF working-copy
  path and document URI remain separate, and that provider conflicts surface
  as safe application errors. A widget regression verifies that failed provider
  sync keeps the local session open and that clean close retries synchronization.
  Flutter widget and analysis checks cover the updated picker contract.
- Hosted Android CI builds the APK and runs the x86_64 native bridge smoke
  path. Local Android build/runtime verification is
  `NOT RUN — LOCAL NATIVE EXECUTION DISALLOWED BY POLICY`.

## Phase 9B Android media and render surface coverage

- `LiveProjectHost::in_process_with_export_handler` is covered headlessly: it
  dispatches commands through the shared Rust project session, keeps dirty/save
  behavior, and exposes no IPC descriptor. A Flutter widget regression
  keeps the Settings CLI details visible only when a descriptor is present.
- Android `or_app_bridge` links the existing `or_media` software FFmpeg
  decoder and shared `or_render` wgpu composition for all supported ABIs.
  SAF descriptors are duplicated into a bounded runtime-only registry (64
  sources); source URIs remain project identities and are never converted to
  filesystem paths. FFmpeg seek/decode cancellation and Rust generation
  invalidation remain active.
- The Android viewer uses Flutter `SurfaceProducer`, one coalesced presenter
  task, one reusable `ARGB_8888` bitmap, a 1920×1080 / 16 MiB transfer bound,
  and the existing three-lease viewer mailbox. The Rust frame lease is released
  after the copy and surface post. Concurrent frame requests share the pending
  presentation result; one follow-up is scheduled when an overlapping request
  arrives before a frame is available. The result remains false unless a frame
  is copied to and posted on the Android surface. Android export and native
  audio output remain unavailable in this checkpoint.
- Hosted API 36 x86_64 SwiftShader integration decodes the existing tiny FFV1
  Matroska fixture through the Android software FFmpeg path, presents its frame
  to a Flutter surface, checks concurrent presentation requests, and verifies
  the project revision stays unchanged. The Android Rust project host follows
  the shared in-process command/state path
  without starting the unsupported desktop IPC transport. The
  job records renderer path, queue budgets, cancellation and preview error
  telemetry, and `ANDROID_HARDWARE_MEDIA=UNVERIFIED`; the texture plugin's Java
  compile target matches the app's JVM 17 target. Physical MediaCodec and
  HardwareBuffer coverage is not claimed. CI verifies the guest Flutter logcat
  stream before each `flutter drive` by emitting and observing a readiness
  marker, then captures the ADB device snapshot on failure. If that pre-driver
  check fails while the configured emulator is offline, CI can safely restart
  because no test has been launched. After driver launch, the retry classifier
  accepts only the exact disposed VM service signature, no expected-test marker
  in either log, and configured `emulator-5554` offline. An unreadable guest
  logcat while the emulator is online, an online emulator, a missing offline
  snapshot, or an app-side Flutter marker showing the test started does not
  retry; synthetic guards cover both classifiers and the app-side marker. CI
  keeps the emulator/logcat/driver logs and permits at most two clean-AVD
  retries, each rerunning the FFmpeg probe and both integration tests. Exhausting
  that budget fails the job. Flutter 3.47.5's
  `flutter drive` uninstalls the app after a successful run by default, and each
  new drive invocation stops and installs the target APK. The bridge diagnostics
  run uses `--keep-app-running` until CI stages the preview fixture with
  `run-as` into the app's persistent `files/` directory; the preview test reads
  it from there and deletes it during teardown. The cache directory is not used
  for this cross-invocation handoff. Local Android build/runtime verification is
  `NOT RUN — LOCAL NATIVE EXECUTION DISALLOWED BY POLICY`.

## Future verification layers

### CONFORMANCE

Normal CI should verify control-plane and runtime contracts with deterministic,
small fixtures: exact `RationalTime`, snapshot revision behavior, semantic
commands/queries, frame lease ownership, software fallback, bounded queues,
cancellation, preview/export semantic parity, provider/task schemas, model
manifests, permissions, and migration/recovery. Platform jobs remain the
authority for native bridge and device/runtime behavior.

### PERFORMANCE / HARDWARE LAB

Dedicated known hardware or self-hosted runners should measure Phase 7 before
hardware paths are treated as improvements. Record decode throughput, seek
latency, A/V drift, audio underrun, dropped frames, copy count, RAM, GPU memory
and resources, queue depths, and software/hardware fallback rate, together with
device, driver, backend, codec, fixture, and configuration. Ordinary hosted CI
timings are not a hardware-performance authority.

Phase 7's performance gate must keep those measurements separate from product
correctness and must not justify a dependency, native path, or removed fallback
without repeatable evidence.

## Product acceptance evidence from 9B

The execution policy now distinguishes static, unit, integration, native,
packaged, user-journey, clean-environment, relaunch, performance, resource,
and cross-platform evidence. The exact required classes for a checkpoint are
in `docs/execution/PLAN.json`; named hosted job steps proving them are in
`docs/execution/EVIDENCE_POLICY.json`. The supervisor records successful
exact-SHA step proofs. A lower-level bridge or synthetic check is not a
substitute for the required Android SAF journey or 9B1 packaged desktop
journey.

The earlier 9B Android retries remain failure provenance. Before another
repair, reproduce or distinguish repeated Flutter-driver lifecycle effects,
application/native or JNI failure, VM-service failure, emulator instability,
and startup main-thread load. Skipped-frame warnings are measurements to
investigate. Do not classify a disconnect as infrastructure based only on ADB
being offline.
