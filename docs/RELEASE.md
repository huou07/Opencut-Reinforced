# Application Release Plan

## Current status

Stable releases: **none**.

Developer Preview prereleases are **available** on [GitHub Releases](https://github.com/huou07/Opencut-Reinforced). They are unsigned or ad-hoc-signed testing builds, not production releases. The latest published product supports project save/reopen/recovery, media import, timeline editing, preview, manual titles/captions, subtitle interchange, and desktop Matroska/WebM export. Android WebM is claimed only after exact-SHA all-ABI and SAF output verification; Android physical-device performance remains unmeasured. Folder import passed exact-SHA packaged Platform Verification, but is not yet in the latest published preview. Local English automatic captions have now passed the packaged desktop journey on a validation-only PR candidate; they are not yet in the latest public preview. Other AI workflows, advanced editing, and production signing remain open.

See the [Developer Preview install and dogfood guide](DEVELOPER_PREVIEW_INSTALL.md) for the current package formats, security limitations, user journey, feedback route, update behavior, and uninstall behavior.

The legacy checkpoint/evidence process is preserved in `docs/archive/execution/` for provenance. Current product work and capability evidence follow the outcome-based roadmap; a passing product check is still separate from publishing a stable release.

## Developer Preview

Developer Preview **releases** are automated nightly or manually dispatched from `main` only. A release is published only after the exact source commit has a successful Platform Verification push run. Its `dev-<12-character-commit-sha>` tag traces it to that source commit, and duplicate releases for the same commit are skipped.

For earlier dogfooding, maintainers may manually dispatch the workflow on a
non-main branch. That validation-only mode requires a successful Platform
Verification run for the exact PR head SHA, builds the five app packages
described below, including a Debian 13 / Ubuntu 24.04 amd64 `.deb`, and runs the
packaged desktop transcription acceptance. It has no
release/tag write permission; its app packages are seven-day Actions artifacts,
not public releases. A failed candidate may upload no desktop package. Main remains the only publication path.

**What "five app packages" means.** The Developer Preview workflow has four
build jobs, but the Linux job produces two artifacts, so a complete run yields
five packages: macOS, Windows, Linux tarball, Debian/Ubuntu amd64 `.deb`, and
Android APK. A candidate that fails a platform legitimately uploads fewer. Count
packages, not jobs — the two numbers will otherwise be compared by mistake.

Publication requires successful Platform Verification for the exact source
commit. The tag resolves to that commit, the release is a prerelease, and each
of the fourteen required assets is non-empty and checksummed (except the
checksum file itself): five app packages, three desktop CLI packages, two
build/provenance files, and the FFmpeg, Opus, and libvpx source archives.

The packages are debug developer builds for testing, not production releases or performance benchmarks. macOS uses an ad-hoc debug signature and is not notarized; Android uses the standard debug signing key. No package has production signing. Desktop app archives carry their matching FFmpeg runtime libraries and license notices. Build information records codec source identities, configuration, toolchains, and license posture. Pinned libvpx license and patent notices are extracted from the exact Git commit blobs, so Windows checkout line-ending conversion cannot alter the distributed provenance. Build outputs are not committed to Git.

The Android preview APK reuses the FFmpeg install from the successful
Platform Verification run for the same source commit. Android WebM is included
only when that exact-SHA run builds all three ABIs and passes the SAF output
journey. Emulator results do not establish physical-device performance.

The preview is a cross-platform Flutter editor using OR Focused Monochrome. Desktop and Android share project concepts and Rust-owned edit commands, adapted to platform file permissions and screen size. Users can create/open projects, import media, revise timeline clips, preview, add titles and manual captions, save/reopen, recover, relink supported sources, and export. Desktop CLI clients can inspect and edit the same live session. The build remains incomplete and is not production-ready.

The current media library supports multiple-file import on desktop and multiple-document import through Android SAF. Packaged desktop apps carry FFmpeg helpers for media inspection and generated previews; Android uses the shared FFmpeg libraries and SAF descriptors. Sources remain external to project documents. Preview thumbnails and waveforms use bounded background work and disposable cache storage with a persistent LRU index; proxy generation remains a core foundation and is not exposed as a user workflow.

**Phase 6D — Trim, split, and ripple editing**

- The project workspace shows canonical Video/Audio track order and visible clip blocks with derived V#/A# labels, a time ruler, and exact rational details.
- Users can add Video and Audio tracks, remove empty tracks, insert media clips through explicit exact-time dialogs, move clips between same-kind tracks, explicitly delete, trim, split, and ripple-delete clips.
- Trim uses an absolute timeline edge, split uses an exact interior point with Rust-generated right-clip IDs, and ripple delete shifts only later clips on the selected track. Each operation uses the existing Rust command and session-history system; attached CLI edits update the open Flutter view through shared host events.
- Flutter loads at most 100 clips per track page and offers explicit Load more for additional clips. Exact timeline values remain Rust-owned; the UI uses doubles only for display layout and never applies optimistic edit geometry.

The timeline supports canonical track and clip editing, pointer move/trim, marker-aware snapping, typed transforms, basic text/captions, selected audio controls, and a closed set of effects/transitions. Desktop playback uses exact seek, scrubbing, play/pause, frame stepping, software decode, and the shared render path; Android has a software preview path with SAF media. Matroska with FFV1/PCM is available; Android WebM with VP9/Opus is published only after the exact-SHA all-ABI and SAF output checks pass. Playback and frame stepping require an explicit sequence rate. The codec profile, editing depth, accessibility/usability evidence, and production packaging remain below a general-purpose release bar.

The attached CLI archives contain `or`/`or.exe` plus safe usage examples, but no descriptor or project-session credential. The macOS CLI is a universal Apple silicon and Intel binary. `SHA256SUMS.txt` covers the other twelve release assets. The current debug package and hosted journey do not establish production signing, broad user-perceived performance, or a complete codec profile.

## Future Stable Release

A stable release is a later product milestone, not a Developer Preview. Stable releases will use deliberate semantic versions and require product readiness plus a reviewed platform matrix, packaging, checksums, dependency and license inventory, human-readable notes, and applicable signing, notarization, or attestation. Installer formats and platform-specific distribution remain undecided until that work is scoped.

## Intended platforms

- macOS
- Windows
- Linux
- Android

The product does not currently target iOS or web. Do not bundle model weights by default. Any future inclusion needs a clear size, license, and distribution strategy. FFmpeg configuration and redistribution implications are recorded per target in the Developer Preview provenance asset and must be rechecked for any codec or packaging change.
