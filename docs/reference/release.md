# Release and Distribution

Last reviewed: 2026-11-05

Scope: what is published today, how each desktop release is built and verified,
and what remains for a broader platform matrix.

## Current status
The published line is **0.1.19** (2026-11-06), cut from one pull request whose
`release-gate` went green and built by one tag run of `desktop-release.yml`. It
is the line where the Windows app is driven by the same semantics harness as
macOS and Linux (`scripts/or_ui.py --target windows`), where the "This machine"
panel stops spinning when a probe is slow, and where no checked-in scenario
names a fixture on its author's Mac. It also carries everything in 0.1.16–0.1.18:
the guided first project, scene typography in schema version 2, the shape style
with a corner radius and a two-stop gradient in schema version 3, measured export
capability, machine facts, removable optional components, library licence
credits, the preview/export parity lane, the Windows rooted-path fix, the
packaged Linux lane fix, and the Windows installer that updates and uninstalls
over a running copy. Its five assets —
`Opencut-Reinforced-0.1.19-arm64.dmg` (37,668,553 B),
`Opencut-Reinforced-0.1.19-linux-x64.deb` (26,096,196 B),
`Opencut-Reinforced-0.1.19-windows-x64-setup.exe` (28,637,395 B),
`Opencut-Reinforced-0.1.19-windows-x64.zip` (41,891,227 B) and
`SHA256SUMS.txt` — were downloaded again after publication and matched their
recorded SHA-256 values; the DMG mounted read-only ran `or version` (0.1.19), the
`.deb` installed with `dpkg -i` and ran `or version` from
`/opt/opencut-reinforced` (0.1.19), and the `.setup.exe` installed silently on
the local Windows host and reported 0.1.19 from `%LOCALAPPDATA%\Programs\Opencut
Reinforced\or.exe`, with `certutil -hashfile` matching the release's own
`SHA256SUMS.txt` row.


Stable releases ship three desktop packages from one source tree:
**macOS 13+ Apple Silicon** as a `.dmg`, **Linux x86_64** as a `.deb` (Debian 13
and Ubuntu 24.04, both amd64), and **Windows x86_64** as an Inno Setup
`.setup.exe` plus the portable `.zip`, all published from
`huou07/Opencut-Reinforced`. The Windows installer is per-user
(`{localappdata}\Programs\Opencut Reinforced`, `PrivilegesRequired=lowest`), so
it needs no administrator prompt and an in-place update never asks for one
either. An install over a running copy closes it first: `PrepareToInstall` stops
`or_app.exe`, `or.exe` and `or-whisper-provider.exe`, because the running app
keeps the bundled FFmpeg DLLs mapped and the silent installer would otherwise
retry `DeleteFile`, then abort the whole update with exit code 5 and no message
(`RestartApplications=no` keeps a silent update from relaunching the app).
Uninstalling over a running copy works the same way: Inno's two-phase
uninstaller never runs `PrepareToInstall`, so `InitializeUninstall` calls the
same close-app step. Ending the process is not enough on its own — Windows keeps
the image sections mapped for a moment afterwards — so the step retries a rename
of `or_app.exe` for up to five seconds and only continues once the image is
really released. Without that, a silent uninstall over a running copy exited 1
and removed nothing, and with only the process wait it exited 0 but left every
DLL behind for the next reboot. When the uninstaller itself cannot be deleted
(the last file in the folder) Inno defers that folder to the next reboot; the
program files and the uninstaller are gone, and the empty folder with the
`unins000.exe` stub is all that remains until then. The macOS image is ad-hoc signed, not notarised; the first
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
Windows packages from `scripts/run_windows_lane.ps1` (which drives
`scripts/stage_windows_bundle.ps1`, `scripts/test_desktop_archive_install.py`
and, for the installer, `scripts/windows_installer.iss` through Inno Setup 6).

The Linux codec build also links x264, so MP4 delivery works without a GPU. That
makes the Linux FFmpeg build GPL-2.0-or-later and ships `licenses/x264/COPYING`
beside the FFmpeg notice; macOS and Windows stay LGPL. See
[ADR 0011](../adr/0011-linux-delivery-encoder.md).
A real Windows host needs four things on PATH before the lane behaves like CI:
the MSVC tools that carry `dumpbin.exe` (VS 2022 Build Tools), a Python 3 that
`python3` does not shadow with the Store alias, `LIBCLANG_PATH` pointing at the
folder with `libclang.dll` (msys64 clang64), and `FFMPEG_INSTALL_PREFIX` with
`FFMPEG_DIR` pointing at the pinned FFmpeg install.

The lane builds `or_cli` and `or_whisper_provider` in release before it stages,
and staging refuses a staged `or.exe` whose `version` disagrees with the
workspace version in `Cargo.toml`. Both exist because a stale
`target/release/or` once produced a setup package whose name said 0.1.16 while
`or version` printed 0.1.15.

Run the lane with PowerShell 7 (`pwsh -File scripts/run_windows_lane.ps1 …`).
Windows PowerShell 5.1 cannot load `System.Runtime.InteropServices.NativeLibrary`,
so the staging step stops with a `TypeNotFound` error. The Windows release job
installs Python 3.12 explicitly (`actions/setup-python`) because the staging
script needs it; the stager also searches `RUNNER_TOOL_CACHE\Python` and the
usual per-machine install directories, and it proves a candidate by starting it
rather than by reading its banner, so a Store alias that opens the Microsoft
Store is skipped instead of trusted. When a machine keeps its interpreter
somewhere the search does not look, pass `OR_PYTHON=<real python.exe>`; the
failure message names every candidate it tried. `--require=` is passed
explicitly because the stager otherwise defaults to `whisper,provider` and
refuses a package that deliberately skips the optional engine. A real Linux host needs the
rustup toolchain and Flutter on PATH, and the UI preflight refuses to start
below 6 GiB of free memory and disk.

The CI desktop jobs call the same scripts, so a package verified on a real host
and one built by CI are produced by one procedure. All three platform scripts
stage the optional runtime by calling `scripts/stage_desktop_runtime.py`, which
carries the pinned Whisper archive table once; adding a helper is a change in
that file plus its call sites, not three separate packaging paths. The packaging gate
also rejects any Mach-O in the bundle whose deployment target is newer than
macOS 13, so a build made on a newer Mac cannot silently raise the public minimum.
`.github/workflows/desktop-release.yml` builds all three packages from the same
tag: a `macos-15` job produces the DMG, an `ubuntu-24.04` job produces the `.deb`,
and a `windows-latest` job produces the `.zip` and the `.setup.exe`, each through
the documented scripts. A `publish` job downloads those artifacts, normalizes their names,
writes one `SHA256SUMS.txt` covering every asset, and creates or updates the
release for a `v*` tag or a manual dispatch with `publish` enabled. A release is
therefore either complete on all three platforms or it does not publish; there is
no path that attaches one platform's package by hand.

That workflow runs only for a release. Packaging is proven on the owner's
machines first — this Mac, a Windows host and a Debian host — because a hosted
run costs about half an hour per platform and only answers questions a local run
answers sooner. Staging runs `whisper-cli --help` and `or version` against the
binaries it just copied when the target matches the host, so a package that
contains the engine but cannot start it fails locally, before a tag exists.

A release therefore makes two CI calls and no more: one pull request through
`platform-verification.yml` (the `release-gate` check the `main` ruleset
requires) and one tag run of this workflow. Do not push a commit to that pull
request to test something; run the lane locally, fix it there, and only then
push.

The pull request itself is short. `platform-verification.yml` skips the three
native desktop jobs on `pull_request` events, so a branch waits only on the
shared checks — repository hygiene, the Rust lane, the Flutter static and widget
lane and the descriptor boundary — while `release-gate` accepts their `skipped`
result for that event only. The full desktop matrix still runs on every push to
`main` and on a tag, which is where the platform evidence is recorded. Set the
repository variable `OR_VERIFY_DESKTOP_ON_PULL_REQUEST=1` when a change genuinely
needs the hosted desktops before merge.

## Version numbers

One release carries two versions and they must agree: `apps/or_app/pubspec.yaml`
(`version:`, with a build number after `+`) and `Cargo.toml`'s
`[workspace.package] version`, which is what `or version` prints and what the
packaged helpers report. `scripts/check-repo.sh` fails when the two differ, so a
release cannot publish a build whose command line names a different version from
the one on the download page. Bump both, then refresh the root `Cargo.lock`
with `cargo update --workspace` and the probe's own lock with
`cargo update --workspace --manifest-path tools/ffmpeg-link-probe/Cargo.toml`,
because `tools/ffmpeg-link-probe` is a separate package that depends on the
workspace crates.

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
- Four app packages come from three build jobs (Linux produces a `.deb` from the
  same staged bundle): macOS `.dmg`, Windows `.zip` and `.setup.exe`, Linux
  `.deb`. Count packages, not jobs.
- Three desktop CLI packages plus build/provenance files and the FFmpeg, Opus and
  libvpx source archives complete the asset set.
- macOS is ad-hoc debug-signed and not notarised. No package has production
  signing.
- A validation-only dispatch on a non-main branch has no release/tag permission;
  its artifacts are seven-day Actions artifacts.

## Stable release (future)

A notarised build needs an Apple Developer Program identity and a hardened-runtime
signing step; the local pipeline stops at ad-hoc signing because this mission has
no account. The Windows `.zip`, the Windows `.setup.exe` and the Linux `.deb` are unsigned
packages: they install by extraction, by the per-user installer and by `dpkg -i`
respectively, and none of them is notarised.
Android is out of the desktop matrix and iOS and web are not targets.

## Platforms

Desktop release targets: macOS 13+ Apple Silicon (`.dmg`), Linux x86_64 Debian 13
and Ubuntu 24.04 (`.deb`), Windows 11 x86_64 (`.setup.exe` and `.zip`). The release gate fails
unless the macOS, Linux and Windows native jobs all succeed on the same commit.
Android is not a release target and is no longer built in CI: the cost of a
hosted emulator that failed on unrelated commits outweighed its value, and the
source stays in the tree for the day it returns to scope.

## Related

- [developer-preview-install.md](developer-preview-install.md)
- [security-licensing.md](security-licensing.md)
- [operations/git.md](../operations/git.md) — publication rules
