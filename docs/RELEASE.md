# Application Release Plan

## Current status

The repository does not yet release Opencut Reinforced application binaries. No production Flutter or Rust application has been started.

## Future GitHub-first workflow

A future application release should follow this sequence:

    reviewed commit
    -> CI
    -> version tag
    -> full test matrix
    -> platform builds
    -> packaging
    -> checksums
    -> dependency and license inventory
    -> SBOM where practical
    -> signing and attestation where available
    -> GitHub Release

Use a release only after all required checks pass and artifacts can be traced to the tagged source and build configuration.

## Intended platforms

- macOS
- Windows
- Linux
- Android

The product does not currently target iOS or web.

## Release artifacts and metadata

- Do not commit application build artifacts into Git source.
- Publish checksums and human-readable release notes with each future artifact.
- Include dependency and license inventory for shipped components.
- Produce an SBOM where practical.
- Sign artifacts and provide attestations where the platform and release infrastructure support them.
- Record the exact FFmpeg configuration and review its redistribution implications.
- Do not bundle model weights by default. Any exception needs a clear size, license, and distribution strategy.

Exact installers, archive types, mobile delivery channels, signing providers, and package formats are not selected. Decide them when the packaging phase begins.
