# Contributing

Last reviewed: 2026-10-26

Opencut Reinforced is pre-MVP. Keep contributions focused on an agreed scope and
preserve existing behaviour.

## Contributor path

1. Read [README.md](README.md) and the [routing index](AGENTS.md).
2. Open only the document that owns your area; the routing table tells you which.
3. For architecture-changing work, discuss the decision before implementing it and
   record it as an ADR under [docs/adr/](docs/adr/README.md).
4. Put editing behaviour in the domain and command layer before adding UI.
5. Reuse existing shell slots and follow [the design language](docs/design/design-language.md).
6. Add the relevant regression check and update the owning document.
7. Run the applicable checks (`bash scripts/check-repo.sh`, focused `cargo test`,
   `flutter analyze`/`flutter test`) and report anything that did not run.
8. Open one focused pull request describing what changed, why, architecture
   impact, checks, docs and risks.

## Expectations

- Rust owns canonical project state; every edit travels through the validated
  command path. Flutter is presentation and bounded read models.
- Preserve user data, project compatibility, existing behaviour and unrelated
  changes.
- Do not add a dependency, asset, model or codec without checking its licence and
  maintenance (see [security and licensing](docs/reference/security-licensing.md)).
- Keep secrets, model weights, generated media and build output out of Git.

External contributors do not need CodeGraph, Ponytail or any private tooling. You
do not need every platform SDK: a check blocked by missing local tooling is
`LOCAL ENVIRONMENT BLOCKED`, and the equivalent GitHub Actions job is the
canonical evidence for that platform.
