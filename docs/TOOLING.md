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
- `Android APK build`: debug APK build; no emulator runtime test is currently configured.
- `Developer Preview`: scheduled nightly or manual `main` builds; publication requires successful Platform Verification for the exact source commit and includes four app packages, three desktop CLI packages, checksums, and build information.

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
- Add Android runtime and Storage Access Framework checks when Android project-file integration is implemented.

## Approved agent tooling

- CodeGraph
- Ponytail
