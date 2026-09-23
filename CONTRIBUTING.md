# Contributing

Opencut Reinforced is in an early, pre-implementation phase. Keep contributions focused on the requested scope.

Before contributing:

- Read [AGENTS.md](AGENTS.md) for repository workflow and safety rules.
- Read [DESIGN.md](DESIGN.md) before UI or visual changes.
- Read [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) before architecture changes.
- Discuss large or difficult-to-reverse architecture changes before implementing them.

For changes:

- Make one logical change per commit; use a Conventional Commit-style subject where practical.
- Update relevant documentation with implementation changes and relevant tests when behavior changes.
- Ensure all relevant checks pass. Explain any checks that do not apply.
- Do not commit secrets, model weights, generated builds, or other local artifacts.
- Review dependency licenses for compatibility before adding dependencies.
- CodeGraph and Ponytail are optional maintainer/agent tools. They are not required for external contributors.
