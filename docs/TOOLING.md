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
- `Developer Preview`: scheduled nightly or manual `main` builds; publication requires successful Platform Verification for the exact source commit and includes four app packages, three desktop CLI packages, checksums, and build information.

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

## Model orchestrator V2 — disabled M1 primitives

M1 adds `scripts/model_orchestrator/store.py` and `sandbox.py` under the frozen
V2 contract. These are controller primitives, not a product runner. They provide
no M2 authorization, operational adoption, promotion, or full-auto eligibility.
The product plan/state and schema versions remain unchanged.

An operator explicitly initializes a separate trusted runtime using an M1-scoped
`ValidatedReleaseAuthority` loaded from the M0-R3 independent Git pin loader.
The pristine authority materialization remains separate from the mutable worker
candidate. Read-only inspection creates nothing and refuses corrupt, missing,
symlink-controlled, unsupported-version or drifted authority. Initial storage is
local Linux/macOS POSIX storage with an observed supported mount type and a real
child-process flock/atomic-replace/fsync probe; network and native Windows
profiles refuse. This is not a claim of resilience to hardware that lies about
fsync, or a substitute for worker isolation.

`state.json` is the sole mutable authoritative snapshot. A transaction checks
sequence/epoch under the state lock, exclusive-creates and fsyncs content-addressed
objects, fsyncs their directory, writes/fsyncs a complete temporary snapshot,
atomically replaces state, then fsyncs its parent directory. Unreferenced objects
remain diagnostic only. Separate task and integration OS locks obey task →
integration → state order. A stage claim re-reads state after acquiring the task
lease, reserves one attempt/epoch/nonce, and persists identity before launch.
Pause records a durable generation and prevents new claims; it does not cancel
an already claimed stage. Controller death, PID reuse, boot mismatch or unknown
container liveness never releases a recorded writer. Recovery preserves dirty
work and worker commits; it never resets, cleans or silently re-freezes a task.

Run the focused checks from the repository root:

```sh
python3 scripts/model_orchestrator/tests/test_isolation_and_state.py -v
python3 scripts/model_orchestrator/tests/test_contracts.py -v
python3 -m unittest scripts.test_execution_infra
python3 scripts/check_execution_plan.py
python3 scripts/check_architecture_policy.py
bash scripts/check-repo.sh
python3 -m compileall -q scripts/model_orchestrator
git diff --check
```

The M1 suite labels actual process/filesystem/Git observations separately from
injected transaction crashes and Docker fixtures. Fixtures validate refusal and
inspection logic; they cannot certify container isolation. CP06 and container
portions of CP07, CP08 and CP10 require an actual verified runtime/image/profile.
CP09/CP11 cover real independent Git topology and interruption preservation;
CP27/CP28/CP29/CP34 cover claims, pause, transaction crash boundaries and supported
storage. A missing Docker daemon yields
`M1_ISOLATION_ENVIRONMENT_UNAVAILABLE`, zero launches and incomplete M1
certification. Do not install/start Docker as an agent workaround or substitute
a host-process sandbox. A daemon alone is insufficient: image, effective
configuration, namespaces/mounts, user, resource bounds and persistent candidate
disk bounds must also be verified. OpenCode remains unavailable unless its exact
version and complete discoverable configuration surface can be pinned and
observed; project permissions alone provide no OS isolation.

### Separately authorized Linux host preparation and live checks

Host administration requires a separate explicit operator task. It is not an
automatic workaround available to the orchestrator. The 2026-10-04 host task
used the installed `dockerd-rootless-setuptool.sh check --force` and
`install --force` on Debian 13.6. The normal user's enabled systemd service and
per-user linger retain the rootless engine across logouts. Rootful services
remain intact, and the default Docker context remains rootful.

For the rootless execution host, construct `DockerCLI` with the pinned CLI
bytes and `endpoint="unix:///run/user/<controller-uid>/docker.sock"`. The
endpoint accepts only the current user's standard rootless Unix socket;
rootful, TCP and another user's endpoints refuse. This selects a connection,
not authority: live daemon, rootless security options, cgroup v2, image and
effective stage inspection remain mandatory. No Docker configuration or
credential environment is passed into the CLI subprocess.

The operator-authorized storage preparation reserved a 48 GiB backing file,
created ext4 on it, and mounted it through a loop block device at a dedicated
candidate root. Its observed filesystem capacity is 50,407,821,312 bytes.
The existing `_bounded_candidate_filesystem()` accepts this genuine block
backing without a contract change. Standard `/etc/fstab` persistence uses
`loop,nodev,nosuid,nofail,x-systemd.mount-timeout=30s`; if the mount is absent,
the ordinary root directory fails the bounded-storage probe. The backing file
is root-owned mode 0600, and its parent is mode 0700. The candidate parent is
controller-owned mode 0700; the disposable fixture tree uses the subordinate
UID mapped to container user 10001. Do not repartition the root disk or treat
an unbounded directory as an equivalent profile. To reverse preparation, first
reconcile all stages, unmount the dedicated volume, remove only its fstab entry,
and detach its loop device. Preserve candidate work before removing any backing
file. The per-user Docker service and linger can be disabled independently of
rootful Docker.

Real Linux/rootless observations used Docker 29.7.2, cgroup v2/systemd, and
`debian@sha256:3783cc01769c7b2b1b83a5c5ad96c815348e28ed7da68e2e3687004faa906251`
on amd64. Actual containers verified CP06 host-path/record isolation, CP08
effective settings and kernel resource limits, CP27 one launch after real
process contention, CP10 restart reconciliation after controller exit, and
CP28 durable pause with bounded-stage settlement. These were inert shell
fixtures on a real container boundary, not product/model execution or fake
Docker observations. Changing live CPU, memory or PID settings refused before
start. Exact commands, container identities and host cleanup manifests remain
in the operator's separate host-preparation evidence directory.

**CP07 remains failed and M1 remains NOT CERTIFIED.** A live filesystem probe
read the inert hostile candidate `opencode.json`/`.opencode` contents: generated
external role files are not mounted into `prepare_fixture_stage()`'s launch
view. This correction only pins the rootless connection. Do not certify the
unmounted files as an effective configuration boundary, or close the gap by
editing candidate configuration. A subsequent M1 correction must mask all
frozen discoverable surfaces, including absent names, in an external launch
view and preserve candidate contents. OpenCode execution still refuses; pinned
adapter/protocol/provider conformance belongs to M3/M5. No M2 authority follows
from these observations.

Available-boundary observations for this M1 candidate (2026-10-03/04):

- OS: macOS 27.0.1, Darwin 27, arm64 (`sw_vers`, `uname -a`). Storage:
  APFS (observed `/sbin/mount`; verified by real flock, replace and fsync probes).
- `docker version`: client 29.7.2, API 1.55, context `desktop-linux`; server
  unavailable at the configured local socket. No image or live container profile
  was observed and zero workers were launched. This is
  `M1_ISOLATION_ENVIRONMENT_UNAVAILABLE`, not a passing container result.
- `python3 scripts/model_orchestrator/tests/test_isolation_and_state.py -v`:
  CP09/CP11 use actual independent Git repositories, preserved dirty files and
  commits, sanitized import, and a fresh controller process. CP27 uses two real
  contending controller processes. CP28 observes durable pause/claim state.
  CP29 uses eight explicit fault points in real child processes plus a parent
  fsync failure. CP34 observes actual local APFS and process-lock semantics;
  unsupported-profile refusal is a deterministic fixture.
- CP06 has no live acceptance result. CP07 role/config and CP08 effective-setting
  negatives are fixtures; their positive container boundaries remain blocked.
  CP10 persistence/unknown-liveness refusal passes deterministic/process checks,
  while whole-container kill/reconciliation is a fixture and remains uncertified.
  Live settlement of CP28's already claimed container stage is also blocked.
- The initial implementation observes bounded dedicated ext4/xfs candidate
  storage on Linux. Ordinary directories and macOS VM quota/profile observers
  remain unavailable. OpenCode configuration artifacts are prepared separately,
  but effective installed-CLI overlays have not been certified. These limitations
  remain explicit refusals; later integration cannot treat this candidate as
  certified merely because Docker becomes reachable.
