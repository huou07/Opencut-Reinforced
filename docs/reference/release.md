# Release and Distribution

Last reviewed: 2026-10-26

Scope: what is published today, how the macOS release is built and verified, and
what remains for a broader platform matrix.

## Current status

Stable releases: **macOS Apple Silicon only**, published as a `.dmg` from
`huou07/Opencut-Reinforced`. The image is ad-hoc signed, not notarised; the first
launch needs the standard right-click → Open step. The app is self-contained:
FFmpeg, the `or` CLI and the assistant workspace ship inside the bundle, and the
workspace is installed into Application Support on first launch. Later beta
packages for macOS, Windows, Linux and Android were retired because they were not
usable; the workflow and tags are kept out of the current release line.

Build locally with `scripts/build_macos_dmg.sh`; it refuses to publish an image
built anywhere except Apple Silicon. `.github/workflows/macos-release.yml` runs
the same build on a fresh `macos-14` runner, runs the Flutter and Rust tests, and
publishes the DMG for a `v*` tag or a manual dispatch with `publish` enabled.

## Developer Preview pipeline

- Nightly or manually dispatched from `main` only.
- Publication requires a successful Platform Verification push run for the exact
  source commit; the `dev-<12-char-sha>` tag resolves to that commit, the release
  is a prerelease, and every required asset is non-empty and checksummed.
- Five app packages come from four build jobs (Linux produces two): macOS,
  Windows, Linux tarball, Debian/Ubuntu amd64 `.deb`, Android APK. Count packages,
  not jobs.
- Three desktop CLI packages plus build/provenance files and the FFmpeg, Opus and
  libvpx source archives complete the asset set.
- macOS is ad-hoc debug-signed and not notarised; Android uses the debug signing
  key. No package has production signing.
- A validation-only dispatch on a non-main branch has no release/tag permission;
  its artifacts are seven-day Actions artifacts.

## Stable release (future)

A notarised build needs an Apple Developer Program identity and a hardened-runtime
signing step; the local pipeline stops at ad-hoc signing because this mission has
no account. Windows, Linux and Android packages are out of the current matrix.
Dependency and licence inventory, notarisation and a broader platform matrix
remain future work and must not be claimed now.

## Platforms

macOS Apple Silicon is the only current release target. Windows, Linux and
Android are not built for release; iOS and web are not targets.

## Related

- [developer-preview-install.md](developer-preview-install.md)
- [security-licensing.md](security-licensing.md)
- [operations/git.md](../operations/git.md) — publication rules
