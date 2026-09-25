# Application Release Plan

## Current status

Stable releases: **none**.

Developer Preview prereleases are **available** on [GitHub Releases](https://github.com/huou07/Opencut-Reinforced/releases). They are for contributors and testers to inspect the native shell and architecture progress. OR remains a pre-MVP project, not a usable video editor.

## Developer Preview

Developer Previews are automated nightly or manually dispatched from `main` only. A preview is published only after the exact source commit has a successful Platform verification run. Its `dev-<12-character-commit-sha>` tag traces it to that source commit, and duplicate releases for the same commit are skipped.

These artifacts are debug developer builds for testing only. Desktop builds are unsigned and not notarized; the Android APK uses the standard debug build signing and no production key. No package has production signing. They are not production releases or performance benchmarks. Each release includes the four raw Flutter outputs for macOS, Windows x64, Linux x64, and Android, plus `SHA256SUMS.txt` and `BUILD-INFO.txt`. Build outputs are not committed to Git.

The updated preview includes the production-direction native Flutter shell using OR Focused Monochrome, visual foundations for Home, Projects, Templates, Asset Library, and Settings, responsive desktop and mobile navigation, and an Editor Shell Preview for visual evaluation. The Editor Shell Preview is not a working editor. There is no usable timeline editor, media ingest, playback, rendering, or export. Project formats and features may evolve before a stable release.

## Future Stable Release

A stable release is a later product milestone, not a Developer Preview. Stable releases will use deliberate semantic versions and require product readiness plus a reviewed platform matrix, packaging, checksums, dependency and license inventory, human-readable notes, and applicable signing, notarization, or attestation. Installer formats and platform-specific distribution remain undecided until that work is scoped.

## Intended platforms

- macOS
- Windows
- Linux
- Android

The product does not currently target iOS or web. Do not bundle model weights by default. Any future inclusion needs a clear size, license, and distribution strategy. FFmpeg configuration and redistribution implications must be reviewed when packaging begins.
