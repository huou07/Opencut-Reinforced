# Release and Distribution

Last reviewed: 2026-11-03

Scope: what is published today, how each desktop release is built and verified,
and what remains for a broader platform matrix.

## Current status

Stable releases ship three desktop packages from one source tree:
**macOS 13+ Apple Silicon** as a `.dmg`, **Linux x86_64** as a `.deb` (Debian 13
and Ubuntu 24.04, both amd64), and **Windows x86_64** as a `.zip`, all published
from `huou07/Opencut-Reinforced`. The macOS image is ad-hoc signed, not notarised; the first
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
Apple Silicon and refuses a missing or non-LGPL FFmpeg prefix. The Linux package
comes from `scripts/run_local_lane.sh linux --packaged` (which drives
`scripts/package_linux_deb.sh` and `scripts/test_linux_deb_install.sh`), and the
Windows package from `scripts/run_windows_lane.ps1` (which drives
`scripts/stage_windows_bundle.ps1` and `scripts/test_desktop_archive_install.py`).
The CI desktop jobs call the same scripts, so a package verified on a real host
and one built by CI are produced by one procedure. All three platform scripts
stage the optional runtime by calling `scripts/stage_desktop_runtime.py`, which
carries the pinned Whisper archive table once; adding a helper is a change in
that file plus its call sites, not three separate packaging paths. The packaging gate
also rejects any Mach-O in the bundle whose deployment target is newer than
macOS 13, so a build made on a newer Mac cannot silently raise the public minimum.
`.github/workflows/desktop-release.yml` builds all three packages from the same
tag: a `macos-15` job produces the DMG, an `ubuntu-24.04` job produces the `.deb`,
and a `windows-latest` job produces the `.zip`, each through the documented
scripts. A `publish` job downloads those artifacts, normalizes their names,
writes one `SHA256SUMS.txt` covering every asset, and creates or updates the
release for a `v*` tag or a manual dispatch with `publish` enabled. A release is
therefore either complete on all three platforms or it does not publish; there is
no path that attaches one platform's package by hand.

The same file runs on every pull request that can change a package, minus
publishing: the rehearsal builds the release artifacts with the release mode,
the release staging script and its smoke checks. A green rehearsal means the tag
run has nothing left to discover, so a release failure is a defect caught before
the merge rather than a second full pipeline after it. Staging runs
`whisper-cli --help` and `or version` against the binaries it just copied when
the target matches the host, so a package that contains the engine but cannot
start it fails the build instead of the download page.

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
no account. The Windows `.zip` and the Linux `.deb` are unsigned packages: they
install by extraction and by `dpkg -i` respectively, and neither is notarised.
Android is out of the desktop matrix and iOS and web are not targets.

## Platforms

Desktop release targets: macOS 13+ Apple Silicon (`.dmg`), Linux x86_64 Debian 13
and Ubuntu 24.04 (`.deb`), Windows 11 x86_64 (`.zip`). The release gate fails
unless the macOS, Linux and Windows native jobs all succeed on the same commit.
Android is a separate diagnostic lane and is not part of the desktop gate.

## Related

- [developer-preview-install.md](developer-preview-install.md)
- [security-licensing.md](security-licensing.md)
- [operations/git.md](../operations/git.md) — publication rules
