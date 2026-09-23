# Local Agent Tooling

This tooling supports development and is not part of the Opencut Reinforced application.

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

## Future, not yet enabled

- Add Rust build/test CI when a Cargo workspace exists.
- Add Flutter analyze/test CI when a Flutter app exists.
- Enable CodeQL after source code is introduced. Scan Rust and GitHub Actions workflows where useful. Verify CodeQL coverage for each supported language; Flutter/Dart should also use its own static-analysis and test tooling.
- Add release workflows when real build artifacts exist.
- Add artifact signing and attestations when releases exist.

## Approved agent tooling

- CodeGraph
- Ponytail
