# Application Release Plan

## Current status

Stable releases: **none**.

Developer Preview prereleases are **available** on [GitHub Releases](https://github.com/huou07/Opencut-Reinforced/releases). They are for contributors and testers to inspect the native shell and architecture progress. OR remains a pre-MVP editor; timeline editing and desktop video preview are available, while several editing tools and export remain planned.

The current Phase 7 hardening checkpoint updates hosted verification and Developer Preview packaging. It does not publish a release; the supervisor owns hosted evidence and checkpoint advancement.

Checkpoint completion is separate from release publication. From the evidence
policy boundary onward, a checkpoint is not repository-authoritative `DONE`
until the supervisor records exact-SHA hosted CI evidence, verifies all
required jobs, verifies a required preview when `PLAN.json` demands it, and
performs the protected state transition. A runner cannot create release or
completion evidence. This checkpoint prepares the existing preview workflow but does not publish a Developer Preview itself.

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

The current preview includes the production-direction native Flutter shell using OR Focused Monochrome, Home, Projects, Templates, Asset Library, and Settings, responsive desktop and mobile navigation, and an Editor Shell Preview for visual evaluation. On macOS, Windows, and Linux, Flutter creates/opens real `.orproj` projects and hosts one Rust-owned live session shared with attached CLI clients. Users can rename, undo/redo, explicitly save, close, inspect/apply/discard recovery, and see dirty state. CLI clients use the visible descriptor path under Settings → Advanced / Developer; the token and descriptor contents are never displayed. Unix endpoints use owner-only permissions; Windows endpoints use protected owner-only DACLs and reject remote clients. Android builds, but project New/Open are unavailable pending Storage Access Framework integration. There is no periodic recovery checkpoint scheduler or autosave. The non-project Editor Shell Preview remains a layout preview.

Phase 5B adds a persistent `.orproj` schema v2 media library with v1 loading/migration, paginated CLI list, and headless/attached CLI add/remove operations. Desktop users can import and remove media from the Media panel; import requires a system-provided `ffprobe`. Phase 5D adds generated cached PNG thumbnails for video media and a combined waveform preview for audio-only media, displayed read-only in the desktop Media panel. Preview generation uses bounded background jobs and disposable cache storage; it requires a system-provided `ffmpeg`. Sources may be offline after import because loading does not open or probe them, and media bytes and preview artifacts are not stored in the project. Library thumbnail/waveform generation still uses system-provided `ffmpeg` and `ffprobe`; desktop viewer playback uses the separately linked FFmpeg runtime packaged with the application. Phase 5E adds persistent disposable cache indexing and bounded LRU eviction of old media-preview artifacts when the global cache budget needs space; Phase 5F adds a core-only file-backed proxy-generation foundation that is not exposed by the UI, CLI, or IPC.

**Phase 6D — Trim, split, and ripple editing**

- The project workspace shows canonical Video/Audio track order and visible clip blocks with derived V#/A# labels, a time ruler, and exact rational details.
- Users can add Video and Audio tracks, remove empty tracks, insert media clips through explicit exact-time dialogs, move clips between same-kind tracks, explicitly delete, trim, split, and ripple-delete clips.
- Trim uses an absolute timeline edge, split uses an exact interior point with Rust-generated right-clip IDs, and ripple delete shifts only later clips on the selected track. Each operation uses the existing Rust command and session-history system; attached CLI edits update the open Flutter view through shared host events.
- Flutter loads at most 100 clips per track page and offers explicit Load more for additional clips. Exact timeline values remain Rust-owned; the UI uses doubles only for display layout and never applies optimistic edit geometry.

Phase 6E adds pointer move/trim, marker-aware snapping, and persistent timeline markers. Phase 7 adds desktop video preview with exact seek, play/pause, scrubbing, and frame stepping; play and frame step require an explicit sequence rate. Desktop decoding is software-only on macOS, Windows, and Linux. Android playback is not enabled. Transforms, crop, opacity, text/captions, effects, transitions, export, and audio device output remain future work. OR remains a pre-MVP project, not a full video editor.

The attached CLI archives contain `or`/`or.exe` plus safe usage examples, but no descriptor or project-session credential. The macOS CLI is a universal Apple silicon and Intel binary. The SHA-256 manifest covers all ten other release assets.

## Future Stable Release

A stable release is a later product milestone, not a Developer Preview. Stable releases will use deliberate semantic versions and require product readiness plus a reviewed platform matrix, packaging, checksums, dependency and license inventory, human-readable notes, and applicable signing, notarization, or attestation. Installer formats and platform-specific distribution remain undecided until that work is scoped.

## Intended platforms

- macOS
- Windows
- Linux
- Android

The product does not currently target iOS or web. Do not bundle model weights by default. Any future inclusion needs a clear size, license, and distribution strategy. FFmpeg configuration and redistribution implications are recorded per target in the Developer Preview provenance asset and must be rechecked for any codec or packaging change.
