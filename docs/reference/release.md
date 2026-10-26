# Release and Distribution

Last reviewed: 2026-10-26

Scope: what is published today, how the Developer Preview pipeline works, and what
a stable release would require. Release work is out of scope for the current
local mission.

## Current status

Stable releases: **none**. Developer Preview prereleases are available on GitHub
Releases: unsigned or ad-hoc-signed testing builds, not production releases.

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

Requires product readiness, a reviewed platform matrix, packaging and checksums,
dependency and licence inventory, human-readable notes, and applicable signing,
notarisation or attestation. Installer formats are undecided. No version, health
or rollback gate exists today, and none may be claimed.

## Platforms

macOS, Windows, Linux, Android. iOS and web are not targets.

## Related

- [developer-preview-install.md](developer-preview-install.md)
- [security-licensing.md](security-licensing.md)
- [operations/git.md](../operations/git.md) — publication is not authorized locally
