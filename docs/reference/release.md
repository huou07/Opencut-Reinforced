# Release and Distribution

Last reviewed: 2026-11-08

Scope: what is published today, how each desktop release is built and verified,
and what remains for a broader platform matrix.

## Current status

The published line is **0.1.27** (2026-11-09), cut from `main`. It is the first
public build since 0.1.21: 0.1.24–0.1.26 were local iterations that were never
tagged. What it carries is in `CHANGELOG.md`; the install steps are in
[install.md](install.md).

Five assets are published per release:

| Asset | Built by |
|---|---|
| `Opencut-Reinforced-<version>-arm64.dmg` | `desktop-release.yml` (macos-15) |
| `Opencut-Reinforced-<version>-linux-x64.deb` | `desktop-release.yml` (ubuntu-24.04) |
| `Opencut-Reinforced-<version>-windows-x64-setup.exe` | `desktop-release.yml` (windows-latest) |
| `Opencut-Reinforced-<version>-windows-x64.zip` | `desktop-release.yml` (windows-latest) |
| `opencut-reinforced-<version>-x86_64.flatpak` | uploaded by hand — CI does not build it |

The tag run publishes the four CI artifacts and writes `SHA256SUMS.txt`; the
Flatpak is built locally with `scripts/package_linux_flatpak.sh` and attached to
the same release, and its entry is appended to `SHA256SUMS.txt`. A release with
a missing or mismatched Flatpak is incomplete, not "four out of five".

All five assets for 0.1.27 were built and installed on the three hosts before
publication, from `main` at `dc829d7c`, and uploaded after the tag run so the
published bytes are the tested ones:

| Asset | Bytes | SHA-256 |
| --- | --- | --- |
| `Opencut-Reinforced-0.1.27-arm64.dmg` | 37,528,006 | `22171092ad11232ca192a60495d28ab48e198837082c7723390b1142878f8b8f` |
| `Opencut-Reinforced-0.1.27-linux-x64.deb` | 23,439,512 | `a087d88d285a387bb5154f4c1bbd572cada68ff843c58b0aaaee77f6db1192f8` |
| `Opencut-Reinforced-0.1.27-x86_64.flatpak` | 22,592,440 | `3d36aafc1ffd068ddcfc39eb6f081d82672faac842ed19277d558939c1f0a25c` |
| `Opencut-Reinforced-0.1.27-windows-x64-setup.exe` | 25,420,752 | `37d323fd2c259b4a26688683d504cc72550d95db358266ef2fce419eaaa99a66` |
| `Opencut-Reinforced-0.1.27-windows-x64.zip` | 35,073,186 | `b4b8ee9b82966689f912afd40453b31215d7d877ad1d544b0af01f7b80db272e` |

The Flatpak **installs and declares its permissions** (`pulseaudio`, `dri`,
`wayland`, `fallback-x11`, `xdg-documents|xdg-videos|xdg-download`) but its
sandboxed runtime is **BLOCKED** on the build host: `bwrap: Can't mkdir parents
for /proc: Permission denied` for both the app and a stock `org.gnome.Sdk`
control, with `CapEff: 0000000000000000`. It is published with that stated, not
as a tested build. See `docs/evidence/2026-11-09/public-release.md`.

The published **0.1.21** build remains the reference for the previous line, and
the **0.1.23** artifacts under `dist/release-0.1.23/` are local-only history: they
were built, installed and exercised in this checkout and never published. The
0.1.20 notes below are kept because their install receipts still describe how the
Windows installer and the `.deb` upgrade behave.

Historical receipt — 0.1.20 (2026-11-06): five assets
`Opencut-Reinforced-0.1.20-arm64.dmg` (37,340,205 B),
`Opencut-Reinforced-0.1.20-linux-x64.deb` (26,100,464 B),
`Opencut-Reinforced-0.1.20-windows-x64-setup.exe` (28,647,456 B),
`Opencut-Reinforced-0.1.20-windows-x64.zip` (41,902,754 B) and
`SHA256SUMS.txt`, each downloaded again after publication and matched against
its recorded SHA-256. The DMG mounted read-only ran `or version` (0.1.20) with an
ad-hoc signature (`Identifier=io.github.huou07.orApp`); the `.deb` upgraded an
installed 0.1.19 with `dpkg -i` (`Setting up opencut-reinforced (0.1.20+e5e49c8b7b02)`)
and ran `/opt/opencut-reinforced/or version`; the `.setup.exe` upgraded an
installed 0.1.19 in place on the Windows host with a running app closed by the
installer (`or: stopping a running copy before touching files`), exited 0 and
reported 0.1.20.

Stable releases ship four desktop packages from one source tree:
**macOS 13+ Apple Silicon** as a `.dmg`, **Linux x86_64** as a `.deb` and a
`.flatpak` (Debian 13 and Ubuntu 24.04, both amd64), and **Windows x86_64** as
an Inno Setup `.setup.exe` plus the portable `.zip`, all published from
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
- Five app packages come from three build jobs (Linux produces a `.deb` and a
  `.flatpak` from the same staged bundle): macOS `.dmg`, Windows `.zip` and
  `.setup.exe`, Linux `.deb` and `.flatpak`. Count packages, not jobs.
- Three desktop CLI packages plus build/provenance files and the FFmpeg, Opus and
  libvpx source archives complete the asset set.
- macOS is ad-hoc debug-signed and not notarised. No package has production
  signing.
- A validation-only dispatch on a non-main branch has no release/tag permission;
  its artifacts are seven-day Actions artifacts.

## Stable release (future)

A notarised build needs an Apple Developer Program identity and a hardened-runtime
signing step; the local pipeline stops at ad-hoc signing because this mission has
no account. The Windows `.zip`, the Windows `.setup.exe`, the Linux `.deb` and the Linux
`.flatpak` are unsigned packages: they install by extraction, by the per-user
installer, by `dpkg -i` and by `flatpak install` respectively, and none of them
is notarised. The Flatpak manifest, AppStream metadata and packaging script live
in `flatpak/` and `scripts/package_linux_flatpak.sh`; the sandbox grants only
documents, videos, downloads, the display, the GPU device and network (for the
configured assistant provider and loopback sidecars). Companion tools installed
on the host are outside the sandbox by design and are not silently reachable
from it.
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
