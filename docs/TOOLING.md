# Development Tooling

This tooling supports contributors and maintainers; it is not part of the Opencut Reinforced application.

## Verification model

Local machines are for editing and headless source verification. GitHub Actions is the canonical verification environment for platform, linker, native builds, and runtime checks. A local platform toolchain is not required for ordinary core or domain development when Actions provides the equivalent check.

By default, agents must not launch the native OR application, a platform emulator or simulator, or an attached physical device locally for verification. Do not run native Flutter integration tests that launch OR, manually smoke-test the local GUI, or open a built/downloaded Developer Preview. This restriction is separate from the toolchain-installation rule. Local repository checks, formatting, static analysis, and headless Rust/CLI/unit/integration tests remain allowed; Flutter format, analysis, and widget tests are allowed when they do not launch a native application. If local runtime interaction is genuinely required and Actions cannot provide the evidence, explain why and ask the user before launching anything.

Report each check as one of:

- `PASS` — it ran and succeeded.
- `FAIL` — it ran and exposed a source, test, or configuration defect.
- `LOCAL ENVIRONMENT BLOCKED` — it could not run because a missing or intentionally unconfigured local platform tool prevented it. This status is neither pass nor fail.
- `NOT RUN` — it was intentionally skipped; for a native/runtime check skipped by policy, use `NOT RUN — LOCAL NATIVE EXECUTION DISALLOWED BY POLICY`.

When a required local check is environment-blocked and an equivalent Actions job exists, continue and inspect that remote job. The task may complete for that check only after the remote job passes. A blocked local check with no remote result is still unverified. A local native/runtime check intentionally skipped by policy is `NOT RUN — LOCAL NATIVE EXECUTION DISALLOWED BY POLICY`, not `LOCAL ENVIRONMENT BLOCKED`.

## Local development baseline

Install only the tools needed for the part you are changing. The repository pins Rust 1.98.0; Flutter CI uses Flutter 3.47.5 stable.

Rust checks run from the repository root:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

For focused Phase 4F/4UI-2 checks, run `cargo test -p or_cli --test semantic_cli` for real executable and same-live-host CLI parity contracts, `cargo test -p or_ipc --test local_transport` for the host platform's real local transport, and `cargo test -p or_ipc --test live_host` for direct/IPC shared-session behavior. On macOS, run the two named native tests separately: the bridge diagnostics test takes the CLI snapshot through `--dart-define=OR_CLI_BOOTSTRAP_JSON=...`, and the project-lifecycle test exercises the real Flutter/Rust host. On Windows, `cargo test -p or_ipc --lib` inspects the actual runtime-directory, descriptor, and named-pipe ACLs. The Linux workspace suite includes the CLI integration tests and Unix IPC tests.

Run lightweight checks first. For core-only work, prefer affected-package checks such as `cargo check -p or_core --all-targets`, and use package-scoped Clippy/tests where they work in the current environment. Attempt stronger workspace checks when useful. A native linker failure caused only by unavailable local platform tooling is `LOCAL ENVIRONMENT BLOCKED`; a Rust source or test failure is `FAIL` and must be fixed.

Flutter checks run from `apps/or_app`:

```sh
flutter pub get
dart format --output=none --set-exit-if-changed .
flutter analyze
flutter test
```

Flutter's Rust native-assets hook builds the bridge for the host target during Flutter builds and tests. Flutter checks that need unavailable host tooling may be environment-blocked locally; use the matching Actions job. Do not change Xcode selection, set `DEVELOPER_DIR`, accept licenses, install/repair platform components, or use `sudo` for platform setup as a normal agent workaround. Those are optional manual choices only for a developer who explicitly wants local platform builds and chooses to configure that machine.

## Hosted platform verification

GitHub Actions is the canonical place for native platform builds. Contributors do not need to own a Mac, Windows PC, or Linux machine, and do not need to install every platform SDK.

- `Repository hygiene`: repository checks from `scripts/check-repo.sh`.
- `Rust checks`: formatting, workspace Clippy, and workspace tests on Ubuntu.
- `Flutter static and widget checks`: dependency resolution, Dart formatting, analysis, and Flutter widget tests on Ubuntu.
- `macOS native build and bridge smoke`: project-storage, recovery, real Unix IPC and shared-host/attached-CLI parity tests, macOS app build, CLI bootstrap capture, and native Flutter bridge/project-lifecycle integration tests.
- `Linux native build`: Linux app build.
- `Windows native build`: project-storage and recovery tests, real named-pipe IPC and attached-CLI shared-host parity tests, owner-only endpoint ACL tests, and the Windows app build.
- `Android APK build`: FFmpeg 8.1.3 shared-library builds and Rust cross-links for `arm64-v8a`, `armeabi-v7a`, and `x86_64`; APK checks for FFmpeg, Rust bridge, and Android texture JNI libraries on all three ABIs; and API 36 x86_64 SwiftShader emulator coverage for FFmpeg loading, Flutter/Rust bridge loading, and software preview presentation through Flutter `SurfaceProducer`. Physical MediaCodec and HardwareBuffer coverage is reported as `ANDROID_HARDWARE_MEDIA=UNVERIFIED`.
- `Developer Preview`: scheduled nightly or manual `main` builds; publication requires successful Platform Verification for the exact source commit and includes four app packages, three desktop CLI packages, checksums, and build information. The Android APK reuses that run's verified FFmpeg install and checks the bridge and FFmpeg libraries for all three packaged ABIs.

The desktop jobs also build the production FFmpeg 8.1.3 link probe and verify
runtime packaging and loading. The Windows probe uses MSYS2 for `pkg-config`
path handling, so Visual Studio's `link.exe` must take precedence over
MSYS2's `/usr/bin/link.exe` when Cargo links the MSVC target. The Windows
texture adapter also parenthesizes `numeric_limits::max()` to avoid the
function-like `max` macro from Windows headers. Its callback follows Flutter
3.47.5's C++ `(width, height)` signature rather than the C callback's extra
`user_data` parameter. Plugin registration creates its `unique_ptr` inside the
class method so the private constructor remains encapsulated.

Flutter Native Assets hooks filter the parent process environment. Android
bridge builds therefore receive the per-ABI FFmpeg install root through
`hooks.user_defines.or_app_bridge.android_ffmpeg_install_root` in
`apps/or_app/pubspec.yaml`; hosted verification links that path to the staged
runner install before building the APK. The hook forwards `FFMPEG_DIR`,
`PKG_CONFIG_PATH`, and `PKG_CONFIG_ALLOW_CROSS` to Cargo for each Android ABI.

The execution supervisor treats these workflows as evidence gates, not merely
status badges. It uses the GitHub REST API with Python's standard library to
match exact `head_sha`, `main` branch, push event, completed status, successful
conclusion, workflow file/name, and every policy-required job. `GH_TOKEN` is
preferred over `GITHUB_TOKEN`; public unauthenticated reads remain supported.
Authenticated polling is 15 seconds, unauthenticated polling is 90 seconds,
and the default hosted wait is 7200 seconds. Rate-limit exhaustion stops the
supervisor. Tokens never enter logs, subprocess arguments, or evidence.

For checkpoints that require a preview, the supervisor verifies the existing
Developer Preview workflow, exact `dev-<first-12-of-SHA>` tag target,
prerelease flag, successful publish job, all seven package archives,
`SHA256SUMS.txt`, `BUILD-INFO.txt`, `FFMPEG-BUILD-INFO.txt`, and
`ffmpeg-8.1.3-source.tar.xz` under the required eleven-asset contract. It may dispatch that existing workflow
only with an appropriate authenticated token; it never publishes a release
directly.

Pushes changing only `docs/execution/STATE.json` and/or
`docs/execution/evidence/**` skip Platform verification because the
implementation SHA has already passed it. Repository hygiene still runs for
that state/evidence commit. Any product, source, configuration, or workflow
change continues to trigger Platform verification.

Full Xcode, CocoaPods, Android SDK/JDK, Windows SDK, and Linux platform packages are optional for general OR development. Install or configure them only when explicitly choosing local platform development; GitHub-hosted jobs provide canonical coverage for the configured targets.

## CodeGraph

CodeGraph indexes a project for symbol search, source exploration, and code relationships. The official upstream is [colbymchenry/codegraph](https://github.com/colbymchenry/codegraph).

CodeGraph is machine-local development tooling. Its project index lives in `.codegraph/`, which is ignored and must not be committed. It is currently wired for Codex and OpenCode.

From the repository root, verify the installation and index with:

```sh
codegraph --version
codegraph status .
```

Initialize an index with `codegraph init` when needed.

## Ponytail

Ponytail helps coding agents prefer small, necessary implementations while retaining correctness and safety checks. The official upstream is [DietrichGebert/ponytail](https://github.com/DietrichGebert/ponytail).

- Codex installation uses `codex plugin marketplace add DietrichGebert/ponytail` followed by `codex plugin add ponytail@ponytail`.
- OpenCode uses the plugin name `@dietrichgebert/ponytail` in the existing user configuration.
- Node must be on `PATH` for Ponytail's Codex lifecycle hooks. Review and trust its two hooks in Codex `/hooks`; restarting Codex or starting a new task may be needed for activation.

Review third-party agent tooling before installing it. Do not install additional agent frameworks or overlapping instruction suites by default. Add tooling only when a concrete project need justifies it.

## Repository / GitHub safeguards

- `scripts/check-repo.sh` checks required files, tracked whitespace, local/generated paths, file sizes, MIT license text, and obvious private-key file types without network access.
- `.github/workflows/repo-hygiene.yml` runs on pull requests, pushes to `main`, and manual dispatch. The repository requires Actions references to use full commit SHAs; the workflow pins `actions/checkout` accordingly.
- GitHub Actions is enabled. The default `GITHUB_TOKEN` permission is read-only, and workflows cannot approve pull request reviews by default.
- Dependabot alerts and security updates are enabled. `.github/dependabot.yml` configures weekly updates for GitHub Actions only.
- The dependency graph is enabled for this public repository.
- Secret scanning and push protection are enabled.
- GitHub Private Vulnerability Reporting is enabled; see [SECURITY.md](../SECURITY.md).
- The active `main-safety` ruleset applies only to `main` and blocks branch deletion and non-fast-forward updates. It does not require pull requests, approvals, status checks, or signed commits.

## Future tooling

- CodeQL is not enabled yet. Evaluate coverage for Rust and GitHub Actions workflows; Flutter/Dart should continue to use its own static-analysis and test tooling.
- Add stable-release signing, notarization, and attestations when distribution requirements are scoped.
- Add Storage Access Framework checks when Android project-file integration is implemented.

## Approved agent tooling

- CodeGraph
- Ponytail

### Android 9B preview repair and hosted proof

The Android software path retains `content://` media identities. A preview
request first evaluates the shared Rust program at its exact time, returns the
sorted/deduplicated active video sources (at most 64 simultaneously active
sources), binds checked provider descriptors, and completes only that request.
An unused library entry consumes no active-source budget. Cached bindings
revalidate Android read permission without reopening the provider every frame.
A partial open/register failure clears the entire set; recovery can retry the
same sources. Project close cancels and drains decoding before clearing the
platform's duplicated descriptors. Android audio continues to return its
explicit unavailable result; this repair does not claim an audio output path.

Initial `SurfaceProducer` creation is usable immediately. The pinned
[Flutter 3.47.5 engine source](https://github.com/flutter/flutter/blob/3.47.5/engine/src/flutter/shell/platform/android/io/flutter/embedding/engine/renderer/FlutterRenderer.java)
(revision `6a19cca56475dbfba1478ee68d7bd0c2ef891da1`) creates the producer at
lines 225–240, initializes `notifiedDestroy=false` around line 491, and only
assigns its callback at lines 819–821. `getSurface` obtains the active reader
lazily at lines 868–874. Restoration at lines 128–141 invokes availability only
for a producer previously notified of destruction. OR's previous false initial
availability flag prevented reaching `getSurface`. The unchanged product
control [run 37100447806](https://github.com/huou07/Opencut-Reinforced/actions/runs/37100447806)
reproduced `[false, false]` presentation after successful decode, without a
VM/ADB disconnect. The initial-surface regression guard requires presentation
with zero restoration callbacks. The official
[plugin migration contract](https://github.com/flutter/website/blob/main/sites/docs/src/content/release/breaking-changes/android-surface-plugins.md)
requires cleanup before invalidation, so copying runs on the worker and surface
access/drawing/cleanup run in main-thread order. One pending slot retains bitmap
ownership until drawing completes; epoch and generation checks reject stale
copies and queued draws. Neither thread waits for the other in a latch cycle.

The presenter exposes actual lease/descriptor counts, bitmap bytes, queue
peaks, draw microseconds, frames, stale drops and surface events. Bounds are one
copied pending frame, eight pending presentation callers, eight queued worker
operations (one slot reserved for descriptor clear), three Rust viewer leases,
and an at-most 1920×1080/16 MiB leased frame. These are boundedness checks;
SwiftShader/debug startup and draw measurements do not establish target-device
frame rate or a speedup.

`Android APK build` retains the named runtime/bridge proof step and adds
`Verify Android SAF preview user journey and resource bounds`. It builds a
separate `saf_fixture` APK only with `-PorSafFixture=true`, then uses native
DocumentsUI and real editor controls against its separate-UID, permission-
enforced provider. The helper provider is absent from OR's APK. The debug-only
fixture control bridge prepares/revokes fixture grants; it cannot return a
picker result or render a frame. Composed screenshots, the asserted report,
actual `/proc/self/fd` provider links, native leases, and collector logs are
preserved in the Android FFmpeg artifact. Every captured texture pixel is
machine-checked against the fixture frame, including the texture issued after a
full release and recreate: that surface is cold, so the journey keeps asking the
real plugin for real frames and lets this guest composite before the capture,
bounded so a genuinely blank surface still fails. The composed PNGs are
published through the binding's own screenshot accumulation, and the journey
asserts that accumulation before it reports, because replacing that map made
the screenshots exist only while the test failed. The proof retains distinct
driver lifecycles: diagnostic bootstrap, existing local-file surface
assertions, and the real SAF user journey. Generic clean-AVD retries and the old
offline classifier are removed because the diagnostic did not prove their
cause.

Every Android drive case now ends with a deterministic disconnect
classification instead of a bare exit status. `scripts/run-android-preview-check.sh`
invokes `scripts/classify_android_disconnect.py` on the preserved driver, guest,
health and state logs, writes `android-disconnect-<case>.json` into the Android
artifact, and leaves the case exit status untouched. A separate always-run step
appends the classifications to the job summary, so a red Android run states its
cause without downloading artifacts. The health sampler also records
`/proc/pressure/memory` alongside `/proc/meminfo`. The classifier explains a
disconnect; it is never product acceptance evidence.

The corrected diagnostic [run 37101265973](https://github.com/huou07/Opencut-Reinforced/actions/runs/37101265973)
failed before tests in its first app lifecycle, while a fresh original-surface
control reached the independent presentation defect. Native/JNI crash,
main-thread pressure, VM service failure and emulator instability remain
unresolved explanations for the VM/ADB loss. The first host sampler had only
one earlier sample (possible pipeline/ADB timeout termination), so an alive
emulator and no captured fatal/ANR cannot classify that disconnect. The final
collector preserves failed samples, consumes complete process listings and
captures all logcat buffers. Diagnostic collection success is not product
acceptance. Hosted verification for the final implementation SHA remains the
supervisor's gate; local native execution is disallowed, and optional hardware
media remains `ANDROID_HARDWARE_MEDIA=UNVERIFIED`.
