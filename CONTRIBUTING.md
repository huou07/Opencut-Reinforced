# Contributing

Opencut Reinforced is in a pre-MVP product and architecture phase. Keep contributions focused on an agreed scope and preserve existing behavior.

## Contributor path

1. Read [README.md](README.md).
2. Read this guide and [docs/INDEX.md](docs/INDEX.md).
3. Choose a focused issue or feature and identify its product and architecture references. Discuss difficult-to-reverse architecture changes before implementation.
4. Follow [docs/DEVELOPMENT_WORKFLOW.md](docs/DEVELOPMENT_WORKFLOW.md).
5. Put editing behavior in the domain and command layer before adding UI.
6. Reuse existing UI shell slots and follow [DESIGN.md](DESIGN.md) for visual work.
7. Add relevant tests and update the existing source-of-truth documentation.
8. Run the applicable verification and report anything that did not run.
9. Open a focused pull request describing what changed, why, architecture impact, checks, docs, and risks.

External contributors do not need CodeGraph, Ponytail, proprietary software, or private tooling. Keep secrets, model weights, build outputs, and generated media out of Git. Review dependency, model, and asset licenses before distribution.

Contributors are not expected to install every target platform SDK or own a Mac, Windows PC, or Linux machine. See [docs/TOOLING.md](docs/TOOLING.md) for the local development baseline and GitHub-hosted platform checks.
