# Testing Strategy

## Status

Phase 3 has executable tests for the bootstrap core, CLI, Flutter shell, and native bridge. This is still a pre-MVP skeleton, not a production editor. The test layers below distinguish the checks that exist from the planned product coverage.

## Current Phase 3 checks

- Rust unit tests cover core bootstrap values and bridge DTO mapping; CLI contract tests execute the real binary and verify human output, JSON, help, and invalid input.
- Flutter widget tests cover desktop and compact navigation and diagnostics rendering with a fake gateway.
- A macOS integration test initializes the native Rust library, calls app info, health, and capability discovery through the typed bridge, and compares the results with the CLI snapshot.
- GitHub Actions runs Rust format, Clippy, and tests; Flutter dependency, format, analysis, and widget checks; and native builds for macOS, Linux, Windows, and Android. The macOS job includes the real bridge smoke test.

These checks prove the bootstrap architecture only. They do not demonstrate editing, media, or release behavior.

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
- media probing, decode, encode, and FFmpeg configuration checks
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

## Fixtures and results

Use tiny, self-created or legally safe media fixtures. Keep fixture provenance and rights clear. Avoid shipping downloaded models or copyrighted media as test data.

A check that did not run must never be reported as passing. Report its exact status and reason. Keep a failing or unavailable check visible rather than silently omitting it. Prototype simulation tests are not production application tests.
