# Release and Distribution

Last reviewed: 2026-11-01

Scope: what is published today, how the macOS release is built and verified, and
what remains for a broader platform matrix.

## Current status

Stable releases: **macOS 13+ on Apple Silicon only**, published as a `.dmg` from
`huou07/Opencut-Reinforced`. The image is ad-hoc signed, not notarised; the first
launch needs the standard right-click → Open step. The app is Apple Silicon only, and the release build pins `ARCHS = arm64` so the
Rust bridge is not also compiled for x86_64 against an arm64 FFmpeg. It is
self-contained: FFmpeg 8.1.3, the local speech runtime (`whisper-cli` and the
provider), the `or` CLI and the assistant workspace ship inside the bundle, and
the workspace is installed into Application Support on first launch. Speech
models and an assistant harness remain optional downloads. Later beta packages
for macOS, Windows, Linux and Android were retired because they were not usable;
the workflow and tags are kept out of the current release line.

Build locally with `scripts/stage_macos_ffmpeg.sh` followed by
`scripts/build_macos_dmg.sh`; the DMG script refuses an image built anywhere except
Apple Silicon and refuses a missing or non-LGPL FFmpeg prefix. The packaging gate
also rejects any Mach-O in the bundle whose deployment target is newer than
macOS 13, so a build made on a newer Mac cannot silently raise the public minimum.
`.github/workflows/macos-release.yml` runs the same build on a fresh `macos-15`
runner, runs the Flutter and Rust tests, publishes the DMG and `SHA256SUMS.txt`
for a `v*` tag or a manual dispatch with `publish` enabled, and writes install,
requirement and limitation notes.

## Updates and optional resources

The app is not self-updating in the background. **Settings → Resources &
updates** (also available from Home) reports whether the bundled runtime is
intact, opens the speech-model manager when a project is open, links to the
assistant harness installer, and checks the public GitHub `releases/latest`
endpoint when the user asks. A newer DMG is downloaded to Downloads, verified
against the release asset digest when GitHub supplies one, and opened for the
user to drag into Applications. The app never replaces `/Applications` while it
is running. The release page carries `SHA256SUMS.txt` for users who prefer a
manual check.

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
