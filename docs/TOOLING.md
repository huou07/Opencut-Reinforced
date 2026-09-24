# Development Tooling

This tooling supports contributors and maintainers; it is not part of the Opencut Reinforced application.

## Local development baseline

Install only the tools needed for the part you are changing. The repository pins Rust 1.98.0; Flutter CI uses Flutter 3.47.5 stable.

Rust checks run from the repository root:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Flutter checks run from `apps/or_app`:

```sh
flutter pub get
dart format --output=none --set-exit-if-changed .
flutter analyze
flutter test
```

Flutter's Rust native-assets hook builds the bridge for the host target during Flutter builds and tests. On macOS, the Command Line Tools are sufficient for general Rust work and widget tests. If an unaccepted full Xcode selection blocks Flutter's native-asset packaging, run the Flutter command with `DEVELOPER_DIR=/Library/Developer/CommandLineTools` when those tools are installed.

## Hosted platform verification

GitHub Actions is the canonical place for native platform builds. Contributors do not need to own a Mac, Windows PC, or Linux machine, and do not need to install every platform SDK.

- macOS: native app build and real Rust bridge smoke test
- Linux: native app build
- Windows: native app build
- Android: debug APK build; no emulator runtime test is currently configured

Full Xcode is optional for general OR development. It is required only for contributors who want to build or debug the macOS app locally; GitHub-hosted macOS runners provide canonical verification. Android SDK, JDK, and emulator setup are optional unless actively developing or debugging Android-specific behavior.

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
- Add release workflows when real build artifacts exist.
- Add artifact signing and attestations when releases exist.

## Approved agent tooling

- CodeGraph
- Ponytail
