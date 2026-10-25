# Development Tooling

Last reviewed: 2026-10-21.

## Current local Mac mission

The owner authorizes building and launching the real macOS application, exercising
its UI, and running focused local checks on this MacBook Air. Use skill
or-dev-loop and the current permissions in AGENT-AUTHORITY.md. Recheck installed
tool versions, free disk/memory and FFmpeg capabilities before a build or
measurement. Use an isolated project/profile for repeatable or destructive runs.

Real UI claims require the actual app and observable project/output state.
Use the available macOS automation or Computer Use surface when a CLI harness
cannot exercise the interaction. Flutter documents the desktop commands in
[desktop integration](https://docs.flutter.dev/platform-integration/desktop) and
[macOS building](https://docs.flutter.dev/platform-integration/macos/building). A synthetic test or screenshot alone does not
prove a complete journey. Local results describe only the tested source, build,
host and workflow.

The source gate is bash scripts/check-repo.sh. It selects an installed
Python 3.11+ interpreter when the system Python is older; its Linux UI-preflight
and Android shell-runner tests run only on Linux. Other platform-specific
procedures below remain available for future work.

Do not push, open pull requests, trigger remote CI, publish or deploy during this
mission. The Linux, Android, Windows and hosted procedures below are retained
for future work; they are not the default local workflow and do not authorize
their execution in this mission.

## The agent-facing UI harness

`python3 scripts/or_ui.py` is the instrument behind OR's interface claims. It
drives the real application by widget key, semantics label (exact or regex),
visible text, role or dialog-scoped control — never by screen coordinate — and
writes a step-by-step transcript with the action, selector, result, elapsed
milliseconds, screenshot path and semantics-snapshot path.

On this Mac the default target is `macos`: the lane builds and launches the real
desktop app through `flutter test -d macos integration_test/or_ui_scenario_test.dart`
and reads artifacts straight from the host filesystem. Prefer this over AppKit
coordinate clicking and over screenshots: it names what it acted on, and a whole
window at 1440x900 launches in about twenty seconds. Reach for a screenshot,
MCP or vision only when semantics cannot express the question (pure visual
hierarchy, colour, or a rendering artefact).

`python3 scripts/or_tree_outline.py <tree-*.json>` prints one line per node with
the global rect and its actions, which is the fast way to read a screen.

```sh
python3 scripts/or_ui.py screens                       # inventory the app's screens (macos by default here)
python3 scripts/or_ui.py tree --name editor           # semantics tree as JSON plus the dump
python3 scripts/or_ui.py shot --name editor           # screenshot at the scenario viewport
python3 scripts/or_ui.py act --label "Import Media" --role button
python3 scripts/or_ui.py run scripts/ui_scenarios/editor-journey.json
python3 scripts/or_ui.py --viewport 390x844 run scripts/ui_scenarios/compact-editor-journey.json
python3 scripts/or_ui.py --target android run scripts/ui_scenarios/editor-journey.json
python3 scripts/or_ui.py --target linux run scripts/ui_scenarios/editor-journey.json
```

On Linux it runs through `scripts/run_linux_ui_preflight.py --exec`, so it reuses
the same project-local caches, writable Flutter SDK mirror, host-scoped Rust
toolchain manifest and shared native lock as the bounded preflight. On Android it
needs an already-booted guest: it installs `ui.apk` from
`apps/or_app/build/local-lanes/android-case-apks/`, pushes the scenario into the
app-private files directory, drives it with `flutter drive`, and pulls the
artifacts back with `run-as`. It exits 2 for a bad invocation or scenario, 3 for
`BLOCKED` (no guest, no Xvfb, another lane holding the lock) with the command that
would settle it, and 1 when a step fails.

Artifacts land in `apps/or_app/build/ui-harness/<scenario>/`: `scenario.json`,
`run.log`, `report.json`, and `artifacts/` with the transcript, screenshots,
semantics JSON and the framework's semantics dump. Those paths are gitignored
runtime state, not evidence records.

A known host hazard: this workstation's shared `$CARGO_HOME/config.toml` pins
`FFMPEG_DIR` and `PKG_CONFIG_PATH` with `force = true` to the Linux desktop
prefix `apps/or_app/build/local-deb/ffmpeg-install`. `force` makes that file win
over the environment, so any other FFmpeg linkage — the Android probe included —
silently compiles against host libraries unless it overrides the value on the
command line. `scripts/stage_android_ffmpeg.sh` passes scoped `--config`
overrides for exactly this reason; the shared file itself is not modified, since
the Linux package lane depends on it.

The harness above exercises Linux and Android targets. It is retained
for future platform work; it is not the required Mac journey. For the current
mission, local native macOS app execution is authorized. Exercise the visible
controls and resulting project state in the actual app.

Classify local checks as PASS, FAIL, LOCAL ENVIRONMENT BLOCKED, UNVERIFIED or
NOT RUN. A missing tool is not a source failure. Do not use remote CI as a
substitute in this mission, because remote execution is explicitly out of scope.

## Linux development baseline — dated reference

Install only the tools needed for the part you are changing. The workspace MSRV
is **Rust 1.89** (`rust-version` in the root `Cargo.toml`); the pinned build
toolchain is **Rust 1.98.0**, declared in the root `rust-toolchain.toml` and
`crates/or_app_bridge/rust-toolchain.toml`.
Flutter CI uses Flutter 3.47.5 stable.

These are two different numbers for two different things: 1.89 is the oldest
compiler the crate metadata claims to support, 1.98.0 is what the bridge is
built with. A contributor must satisfy both.

The workspace-root `rust-toolchain.toml` selects the same 1.98.0 compiler as
the bridge and hosted builds. It requests only host tools; the bridge's nested
configuration retains cross-compilation targets for platform builders. Use the
rustup Cargo proxy on PATH: Debian's standalone `/usr/bin/cargo` does not apply
rustup toolchain selection.

### Historical Debian toolchain PATH

On the inspected Debian host (2026-10-17), the system PATH selects Debian Rust
1.85; Rust 1.98.0 and Flutter 3.47.7 are restored under the owner home and are
shared with workers through H07P policy.
Flutter's Rust native-assets hook also needs `rustup` on PATH. Without it,
`flutter test` can fail with:

```
RustBuilder: Failed to invoke "rustup [show, active-toolchain]"
RustProcessException: Failed to invoke rustup; is it installed?
```

That failure is an environment gap, not a source defect. To fix it for a local
run, export both the toolchain home and the binary path:

```sh
export CARGO_HOME=/home/huou07/.cargo
export RUSTUP_HOME=/home/huou07/.rustup
export PATH="$CARGO_HOME/bin:/home/huou07/.local/share/flutter/bin:$PATH"
rustup show active-toolchain   # expect stable-x86_64-unknown-linux-gnu (Rust 1.98.0)
```

Workers use the owner-home rustup proxy. Keep the toolchain read-only; Rustup
native-assets builds need write access to its temporary directory, so configure
that exact path through the Playground GUI for the selected worker and verify
the enforced revision before running the preflight. Keep Rustup, Flutter
settings and harness credential stores private.
The Linux preflight gives Flutter temporary XDG config/cache/data directories and
copies only readable public Cargo registry/Git and pub.dev caches into ignored
`apps/or_app/.dart_tool/aeh-cargo` and `aeh-pub-cache` when the shared caches
are not writable. Writability is decided by an actual create probe, not
`os.access`: the shared owner caches carry `rw` ACLs for workers, which
`os.access` reports as writable even where a sandboxed run denies the write.
Flutter additionally rewrites `bin/cache/engine.stamp` beside its own script on
every invocation, so when the shared SDK cache is not writable the preflight
mirrors the SDK into ignored `apps/or_app/.dart_tool/flutter-overlay` and puts
that `bin` first on `PATH`. The overlay keeps a real `bin` tree and symlinks the
large read-only `artifacts`/`dart-sdk` trees; it never writes the shared SDK. The
preflight excludes Cargo config and credentials, resolves dependencies
offline with the checked-in lockfile, and runs the Linux UI test with
`--no-pub`. The bounded runner uses one compiler worker, a shared lock, a
300-second dependency stage and a 600-second native limit, and stops below 2 GiB
available RAM. Do not copy another
machine's `target`, `build`, `.dart_tool` or Flutter ephemeral directories.
Preserve source, locks and user data.

The inspected Flutter SDK is 3.47.7; CI pins 3.47.5, so local Flutter output is
not exact-version CI evidence. Seventeen focused preflight tests pass. The Linux
runner temporarily narrows the bridge's nested `rust-toolchain.toml` to the
host target and restores its original bytes in `finally`; it avoids downloading
unused cross-platform Rust targets. See the 2026-10-17 Debian run in
[TESTING.md](TESTING.md#evidence-selection-and-bounded-linux-preflight).

Interrupting that rewrite is what made it dangerous, so the runner is now safe
to kill:

- It refuses to start on a dirty tree (`git status --porcelain` must be empty),
  because a toolchain rewrite is only attributable when nothing else is edited.
- Before narrowing the manifest it writes the original bytes to ignored
  `apps/or_app/build/ui-preflight/rust-toolchain.toml.original`, and the next run
  restores from that backup before its clean-tree check. A `SIGKILL` never runs
  `finally`, so startup recovery — not the handler — is what guarantees the tree
  is repaired.
- `python3 scripts/run_linux_ui_preflight.py --plan` reports the resolved source,
  dirtiness, headroom, missing tools, cache and overlay decisions, budgets and
  commands without changing anything, and exits 3 when a real run would be
  blocked.
- Dependency resolution no longer shares the native budget: `pub get` gets 300
  seconds of its own, then the native journey keeps the 600-second limit. The
  recorded `stage_timings` show each stage's cost.

With the PATH above, the Debian inspection passed:

- `cargo check --workspace --all-targets --locked --offline -j 2`, including
  media, audio, render and Flutter bridge source checks against local libraries.
- `cargo test -p or_core -p or_ipc -p or_cli --locked --offline`.
- `flutter pub get --offline` without tracked lockfile changes, followed by
  `flutter analyze --no-pub`.
- `bash scripts/check-repo.sh`.

This is local source evidence at baseline `21bfb3aa6891` plus the toolchain/doc
patch, not packaged/native runtime acceptance or a hosted claim record. The
full native app and platform packaging still use their dedicated CI gates.

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

## Hosted platform verification — future reference

GitHub Actions is the canonical place for native platform builds. Contributors do not need to own a Mac, Windows PC, or Linux machine, and do not need to install every platform SDK.

- `Repository hygiene`: repository checks from `scripts/check-repo.sh`.
- `Rust checks`: formatting, workspace Clippy, and workspace tests on Ubuntu.
- `Flutter static and widget checks`: dependency resolution, Dart formatting, analysis, and Flutter widget tests on Ubuntu.
- `macOS native build and bridge smoke`: project-storage, recovery, real Unix IPC and shared-host/attached-CLI parity tests, macOS app build, CLI bootstrap capture, and native Flutter bridge/project-lifecycle integration tests.
- `Linux native build`: Linux app build.
- `Windows native build`: project-storage and recovery tests, real named-pipe IPC and attached-CLI shared-host parity tests, owner-only endpoint ACL tests, and the Windows app build.
- `Android APK build`: FFmpeg 8.1.3 shared-library builds and Rust cross-links for `arm64-v8a`, `armeabi-v7a`, and `x86_64`; APK checks for FFmpeg, Rust bridge, and Android texture JNI libraries on all three ABIs; and API 36 x86_64 SwiftShader emulator coverage for FFmpeg loading, Flutter/Rust bridge loading, and software preview presentation through Flutter `SurfaceProducer`. Physical MediaCodec and HardwareBuffer coverage is reported as `ANDROID_HARDWARE_MEDIA=UNVERIFIED`.
- `Developer Preview`: scheduled nightly or manual `main` builds; publication requires successful Platform Verification for the exact source commit and includes five app packages (Linux tarball and Debian/Ubuntu amd64 package), three desktop CLI packages, checksums, and build information. The Android APK reuses that run's verified FFmpeg install, stages its shared libraries into `jniLibs`, and checks the bridge and FFmpeg libraries for all three packaged ABIs.

The macOS native project-lifecycle integration step is bounded to 35 minutes.
The same lifecycle group passed in 21 minutes on one hosted run but later
remained stuck after Flutter reported an app-foreground failure; the timeout
keeps an unresolved hosted launch from consuming the full macOS job window.

The desktop jobs also build the production FFmpeg 8.1.3 link probe and verify
runtime packaging and loading. Their pinned Opus 1.6.1 archive comes from the
OSU Open Source Lab Xiph mirror; its SHA-256 remains checked against the
[official release checksum](https://opus-codec.org/release/stable/2026/01/14/libopus-1_6_1.html),
and build provenance records both upstream and mirror URLs. The Windows probe
uses MSYS2 for `pkg-config`
path handling, so Visual Studio's `link.exe` must take precedence over
MSYS2's `/usr/bin/link.exe` when Cargo links the MSVC target. The Windows
build fetches zlib 1.3.2 from the upstream release asset and checks its pinned
SHA-256 before building the static helper dependency. Before FFmpeg configure,
the Windows job reconstructs and checks the pinned Opus/libvpx pkg-config path
in the consuming MSYS2 process, since validating it in the earlier codec-build
step does not prove that a later shell can resolve it. After configure, the
Windows libvpx archive uses the `vpxmt.lib` static CRT variant so its linker
probes match FFmpeg's `/MT` MSVC build. The job requires FFmpeg to enable its
VP9 encoder and prints the configure log immediately if that capability is
missing, including the libvpx symbol and linker probe context. The Windows
texture adapter also parenthesizes
`numeric_limits::max()` to avoid the
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

> **Archival (2026-10-14).** The supervisor that read these workflows as
> evidence gates belonged to the paused execution framework and no longer runs.
> Its behaviour is preserved here because it explains how the pre-pivot receipts
> in `docs/archive/execution/evidence/` were produced. It is not the current
> verification mechanism.

The execution supervisor treated these workflows as evidence gates, not merely
status badges. It used the GitHub REST API with Python's standard library to
match exact `head_sha`, `main` branch, push event, completed status, successful
conclusion, workflow file/name, and every policy-required job. `GH_TOKEN` was
preferred over `GITHUB_TOKEN`; public unauthenticated reads remained supported.
Authenticated polling was 15 seconds, unauthenticated polling 90 seconds, and
the default hosted wait 7200 seconds. Rate-limit exhaustion stopped the
supervisor. Tokens never entered logs, subprocess arguments, or evidence.

For checkpoints that required a preview, the supervisor verified the existing
Developer Preview workflow, exact `dev-<first-12-of-SHA>` tag target,
prerelease flag, successful publish job, the five app packages, three desktop CLI
packages, `SHA256SUMS.txt`, `BUILD-INFO.txt`, `FFMPEG-BUILD-INFO.txt`, and
`ffmpeg-8.1.3-source.tar.xz`. It could dispatch that existing workflow only with
an appropriate authenticated token; it never published a release directly.

Today, verify a requirement by reading its recorded claim in
[evidence/claims/](evidence/claims/README.md) and inspecting that run's jobs
directly. Nothing in the repository decides on its own that a requirement is met.

Pushes changing only `docs/archive/execution/STATE.json` and/or
`docs/archive/execution/evidence/**` skip Platform verification because the
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

## Playwright

`playwright-cli` (Microsoft's agent-facing [playwright-cli](https://github.com/microsoft/playwright-cli), installed as `@playwright/cli`) is an agent-local browser driver, not a repository dependency. It lets an agent read the DOM and accessibility tree of a rendered page instead of guessing, which is what the HTML prototype in `prototypes/` and the UX acceptance work in `docs/UX_ACCEPTANCE.md` need. It adds no package to this workspace and no test target to CI.

Like CodeGraph it is machine-local tooling, so it belongs on the developer's `PATH` rather than in `Cargo.toml`. It is used only when a task needs a rendered page; Rust and Flutter analysis stay in their own tooling, and hosted Actions remains canonical for native acceptance. It drives a Playwright-provided Chromium build by default and requires no branded Chrome and no root.

## Ponytail

Ponytail helps coding agents prefer small, necessary implementations while retaining correctness and safety checks. The official upstream is [DietrichGebert/ponytail](https://github.com/DietrichGebert/ponytail).

- Codex installation uses `codex plugin marketplace add DietrichGebert/ponytail` followed by `codex plugin add ponytail@ponytail`.
- OpenCode uses the plugin name `@dietrichgebert/ponytail` in the existing user configuration.
- Node must be on `PATH` for Ponytail's Codex lifecycle hooks. Review and trust its two hooks in Codex `/hooks`; restarting Codex or starting a new task may be needed for activation.

Review third-party agent tooling before installing it. Do not install additional agent frameworks or overlapping instruction suites by default. Add tooling only when a concrete project need justifies it.

## Repository / GitHub safeguards

- `scripts/check-repo.sh` checks required files, tracked whitespace, local/generated paths, file sizes, MIT license text, and obvious private-key file types without network access. It then runs four gates: `check_doc_links.py` (relative Markdown links resolve), `check_doc_freshness.sh` (every current-claim document carries a recent `Last reviewed` date), `check_claims.py` (claim records are well formed, and their `implementation_sha` is a commit in this repository that is an ancestor of `HEAD`), and `test_check_claims.py` (that reject behaviour cannot regress). It also runs the hermetic preflight-preparation and Android report suites.
- It also runs `check_or_core_deps.py`, which enforces that `or_core` stays free of GPU, media, audio, text-shaping, scripting and AI-runtime dependencies. It lints `scripts/check_doc_freshness.sh` and itself with `shellcheck -S warning` when `shellcheck` is available, and warns rather than silently skipping when it is not.
- Execution-plan graph validation is not reinstated. It guarded a frozen ledger and has no live counterpart; the validators are preserved under `scripts/archive/` for history. See [SECURITY_LICENSING.md](SECURITY_LICENSING.md).
- `.github/workflows/repo-hygiene.yml` runs on pull requests, pushes to `main`, and manual dispatch. The repository requires Actions references to use full commit SHAs; the workflow pins `actions/checkout` accordingly, checks out full history (`fetch-depth: 0`) so the claim-SHA and secret scans can resolve history, and runs gitleaks 8.30.1 from its pinned release after verifying the archive's SHA-256.
- GitHub Actions is enabled. The default `GITHUB_TOKEN` permission is read-only, and workflows cannot approve pull request reviews by default.
- Dependabot alerts and security updates are enabled. `.github/dependabot.yml` configures weekly updates for GitHub Actions only.
- The dependency graph is enabled for this public repository.
- Secret scanning and push protection are enabled on GitHub. That is a hosting-side control and is not what a local pass proves: until 2026-10-17 nothing in `scripts/check-repo.sh` or `scripts/acceptance.sh` scanned for secrets, so a green local gate said nothing about leaked credentials. `scripts/acceptance.sh` now runs `gitleaks detect` over the repository when `gitleaks` is installed and reports the lane `UNVERIFIED` with that reason when it is not; `scripts/check-repo.sh` still performs no secret scan of its own, and the pinned scan above is the CI authority.
- GitHub Private Vulnerability Reporting is enabled; see [SECURITY.md](../SECURITY.md).
- The active `main-safety` ruleset applies only to `main` and blocks branch deletion and non-fast-forward updates. It does not require pull requests, approvals, status checks, or signed commits.

## Future tooling

- CodeQL is not enabled yet. Evaluate coverage for Rust and GitHub Actions workflows; Flutter/Dart should continue to use its own static-analysis and test tooling.
- Add stable-release signing, notarization, and attestations when distribution requirements are scoped.
- Add Storage Access Framework checks when Android project-file integration is implemented.

## Approved agent tooling

- CodeGraph
- Ponytail
- Playwright (`playwright-cli`, agent-local; added 2026-10-15 for rendered-page inspection of the prototype and UX work)

### Android emulator on the previous Linux workstation

The local AVD is named **`OR-android-test`** (Android 36, google_apis, x86_64). It is OR's
own machine; `an3-api36` and `vce-api36` belong to the VCE project and must not be renamed,
recreated or wiped from here.

Start it with OR's script rather than the UI-automation plugin's boot path:

```sh
mkdir -p /tmp/or-android
RUNNER_TEMP=/tmp/or-android OR_AVD_NAME=OR-android-test \
  bash scripts/start-android-emulator.sh
```

Measured 2026-10-19: `android_boot OR-android-test` reported "no emulator serial appeared
within 180000 ms" twice (once for `an3-api36`, once for the fresh OR AVD) because the
plugin's default launch does not pass this host's GPU mode. The script starts the same AVD
as `emulator-5554` with `gpu swiftshader_indirect` and adb reports it `device` within a
minute. The plugin's own tools then work against that serial, which is the combination to
use: **the script boots, the plugin drives**.

## Android 9B preview repair and hosted proof

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
supervisor's gate; local native execution outside the two owner-granted lanes is
disallowed, and optional hardware media remains `ANDROID_HARDWARE_MEDIA=UNVERIFIED`.
