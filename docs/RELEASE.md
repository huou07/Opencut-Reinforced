# Application Release Plan

## Current status

Stable releases: **none**.

Developer Preview prereleases are **available** on [GitHub Releases](https://github.com/huou07/Opencut-Reinforced/releases). They are unsigned or ad-hoc-signed testing builds, not production releases. OR remains pre-MVP: projects, media import, timeline editing, preview, and a limited Matroska export path are implemented, but supported formats, editing tools, audio/effects, text, and AI workflows remain incomplete.

The legacy checkpoint/evidence process is preserved in `docs/execution/` for provenance. Current product work and capability evidence follow the outcome-based roadmap; a passing product check is still separate from publishing a stable release.

## Developer Preview

Developer Previews are automated nightly or manually dispatched from `main` only. A preview is published only after the exact source commit has a successful Platform verification run. Its `dev-<12-character-commit-sha>` tag traces it to that source commit, and duplicate releases for the same commit are skipped.

The supervisor verifies the workflow and release rather than publishing
directly: the tag must resolve to the exact source commit, the release must be
a prerelease, the publish workflow must succeed, and every policy-required
asset must be non-empty. The enforced preview contract requires eleven assets:
the existing seven application/CLI packages, `SHA256SUMS.txt`,
`BUILD-INFO.txt`, `FFMPEG-BUILD-INFO.txt`, and
`ffmpeg-8.1.3-source.tar.xz`. If a required preview is absent and no suitable
token can dispatch the existing workflow, verification stops with execution
state unchanged.

These artifacts are debug developer builds for testing only. macOS carries only an ad-hoc debug signature and is not notarized; the Android APK uses the standard debug signing key. No package has production signing. They are not production releases or performance benchmarks. The enforced contract contains four application packages (macOS, Windows x64, Linux x64, and Android), three desktop CLI packages (universal macOS, Windows x64, and Linux x64), `SHA256SUMS.txt`, `BUILD-INFO.txt`, `FFMPEG-BUILD-INFO.txt`, and `ffmpeg-8.1.3-source.tar.xz`—eleven non-empty assets total. The three desktop application archives carry their matching FFmpeg runtime libraries and LGPL notices. The build-info asset records, per desktop target, FFmpeg version, source identity, exact configure arguments, enabled libraries, decoders, encoders, muxers, demuxers and protocols, compiler/toolchain identity, patch status, license posture, source URL, and runtime library names. `SHA256SUMS.txt` covers each other release asset. Build outputs are not committed to Git.

The Android preview APK reuses the FFmpeg install artifact from the successful
Platform Verification run for the same source commit. Before publication, the
preview job stages that install for the bridge build, copies its shared
libraries into Android's `jniLibs` directories, and checks that the APK
contains the Rust bridge and FFmpeg shared libraries for `arm64-v8a`,
`armeabi-v7a`, and `x86_64`.

The current preview includes the native Flutter application using OR Focused Monochrome, Home, Projects, Templates, Asset Library, Settings, responsive desktop/mobile navigation, and a non-project Editor Shell Preview for layout evaluation. On macOS, Windows, and Linux, users create/open `.orproj` projects, import multiple media files, edit a shared Rust-owned session, preview video, and export the currently supported Matroska profile. Attached CLI clients use the same live session and command path. Android uses SAF to create/open project documents, media import, preview, and export through an app-private working copy and explicit document synchronization. Project changes can be saved explicitly; dirty projects also update a recovery sidecar periodically without overwriting the canonical file. Users can inspect, apply, or discard recovery state. CLI descriptor credentials are not displayed; Unix endpoints use owner-only permissions and Windows endpoints use protected owner-only DACLs and reject remote clients. The editor has materially incomplete features and formats; these packages are not production-ready.

The current media library supports multiple-file import on desktop and multiple-document import through Android SAF. Packaged desktop apps carry FFmpeg helpers for media inspection and generated previews; Android uses the shared FFmpeg libraries and SAF descriptors. Sources remain external to project documents. Preview thumbnails and waveforms use bounded background work and disposable cache storage with a persistent LRU index; proxy generation remains a core foundation and is not exposed as a user workflow.

**Phase 6D — Trim, split, and ripple editing**

- The project workspace shows canonical Video/Audio track order and visible clip blocks with derived V#/A# labels, a time ruler, and exact rational details.
- Users can add Video and Audio tracks, remove empty tracks, insert media clips through explicit exact-time dialogs, move clips between same-kind tracks, explicitly delete, trim, split, and ripple-delete clips.
- Trim uses an absolute timeline edge, split uses an exact interior point with Rust-generated right-clip IDs, and ripple delete shifts only later clips on the selected track. Each operation uses the existing Rust command and session-history system; attached CLI edits update the open Flutter view through shared host events.
- Flutter loads at most 100 clips per track page and offers explicit Load more for additional clips. Exact timeline values remain Rust-owned; the UI uses doubles only for display layout and never applies optimistic edit geometry.

The timeline supports canonical track and clip editing, pointer move/trim, marker-aware snapping, typed transforms, basic text/captions, selected audio controls, and a closed set of effects/transitions. Desktop playback uses exact seek, scrubbing, play/pause, frame stepping, software decode, and the shared render path; Android has a software preview path with SAF media. Export currently writes Matroska with FFV1 video and PCM S16LE audio. Playback and frame stepping require an explicit sequence rate. The codec profile, editing depth, accessibility/usability evidence, and production packaging remain below a general-purpose release bar.

The attached CLI archives contain `or`/`or.exe` plus safe usage examples, but no descriptor or project-session credential. The macOS CLI is a universal Apple silicon and Intel binary. The SHA-256 manifest covers all ten other release assets. The current debug package and hosted journey do not establish production signing, broad user-perceived performance, or a complete codec profile.

## Future Stable Release

A stable release is a later product milestone, not a Developer Preview. Stable releases will use deliberate semantic versions and require product readiness plus a reviewed platform matrix, packaging, checksums, dependency and license inventory, human-readable notes, and applicable signing, notarization, or attestation. Installer formats and platform-specific distribution remain undecided until that work is scoped.

## Intended platforms

- macOS
- Windows
- Linux
- Android

The product does not currently target iOS or web. Do not bundle model weights by default. Any future inclusion needs a clear size, license, and distribution strategy. FFmpeg configuration and redistribution implications are recorded per target in the Developer Preview provenance asset and must be rechecked for any codec or packaging change.
