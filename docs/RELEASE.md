# Application Release Plan

## Current status

Stable releases: **none**.

Developer Preview prereleases are **available** on [GitHub Releases](https://github.com/huou07/Opencut-Reinforced/releases). They are for contributors and testers to inspect the native shell and architecture progress. OR remains a pre-MVP project, not a usable video editor.

## Developer Preview

Developer Previews are automated nightly or manually dispatched from `main` only. A preview is published only after the exact source commit has a successful Platform verification run. Its `dev-<12-character-commit-sha>` tag traces it to that source commit, and duplicate releases for the same commit are skipped.

These artifacts are debug developer builds for testing only. Desktop builds are unsigned and not notarized; the Android APK uses the standard debug build signing and no production key. No package has production signing. They are not production releases or performance benchmarks. Each release includes four application packages (macOS, Windows x64, Linux x64, and Android), three desktop CLI packages (universal macOS, Windows x64, and Linux x64), `SHA256SUMS.txt`, and `BUILD-INFO.txt`—nine non-empty assets total. Build outputs are not committed to Git.

The current preview includes the production-direction native Flutter shell using OR Focused Monochrome, Home, Projects, Templates, Asset Library, and Settings, responsive desktop and mobile navigation, and an Editor Shell Preview for visual evaluation. On macOS, Windows, and Linux, Flutter creates/opens real `.orproj` projects and hosts one Rust-owned live session shared with attached CLI clients. Users can rename, undo/redo, explicitly save, close, inspect/apply/discard recovery, and see dirty state. CLI clients use the visible descriptor path under Settings → Advanced / Developer; the token and descriptor contents are never displayed. Unix endpoints use owner-only permissions; Windows endpoints use protected owner-only DACLs and reject remote clients. Android builds, but project New/Open are unavailable pending Storage Access Framework integration. There is no periodic recovery checkpoint scheduler or autosave. The Editor Shell Preview is not a working editor. Phase 5B adds a persistent `.orproj` schema v2 media library with v1 loading/migration, paginated CLI list, and headless/attached CLI add/remove operations. Desktop users can import and remove media from the Media panel; Import Media requires a system-provided `ffprobe`. A source may be offline or moved after import because loading never opens or probes stored references. Media bytes are not copied into the project. Phase 5C adds a bounded background Job Manager and a disposable thumbnail/waveform cache foundation (deterministic cache keys, bounded atomic storage, cooperative cancellation). It is infrastructure only: no visible thumbnails, no waveform generation, and no cache is wired into the UI, IPC, or CLI. There is still no timeline, playback, decode/render pipeline, proxy generation, cache database, or bundled FFmpeg. Project formats and features may evolve before a stable release.

The attached CLI archives contain `or`/`or.exe` plus safe usage examples, but no descriptor or project-session credential. The macOS CLI is a universal Apple silicon and Intel binary. Releases include `SHA256SUMS.txt` for all seven platform packages, and the publish workflow verifies downloaded assets against those checksums.

## Future Stable Release

A stable release is a later product milestone, not a Developer Preview. Stable releases will use deliberate semantic versions and require product readiness plus a reviewed platform matrix, packaging, checksums, dependency and license inventory, human-readable notes, and applicable signing, notarization, or attestation. Installer formats and platform-specific distribution remain undecided until that work is scoped.

## Intended platforms

- macOS
- Windows
- Linux
- Android

The product does not currently target iOS or web. Do not bundle model weights by default. Any future inclusion needs a clear size, license, and distribution strategy. FFmpeg configuration and redistribution implications must be reviewed when packaging begins.
