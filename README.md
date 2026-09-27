# Opencut Reinforced

A free and open-source cross-platform video editor designed around one structured editing core shared by the human interface, CLI automation, and AI agents.

## Status

**Pre-MVP:** Phase 3, Phase 4A–4F, Phase 4UI-1/4UI-2, and Phase 5A–5F are implemented; Phase 5 is DONE / FOUNDATION COMPLETE. Phase 6 is IN PROGRESS: 6A, 6B, 6C, and 6D are DONE; 6E is PLANNED. The desktop app creates and opens real projects, hosts one Rust-owned live session shared by the Flutter bridge and authenticated local IPC, and supports rename, undo/redo, explicit save, close, recovery decisions, and dirty-state protection. Phase 5B added a persistent project media library with validated local-file references, media commands, undo/redo, and paginated queries. Phase 6A added canonical `.orproj` v3 timeline persistence; Phase 6B added core/CLI track and clip commands, bounded timeline queries, and session-local history. Phase 6C adds a real project timeline with Video/Audio tracks, visible clips, exact-time insertion, dialog-based move/delete, bounded clip pages, and Rust-command-driven edits. Phase 6D adds exact trim, split, and track-local ripple-delete commands, compact undo/redo recipes, headless and attached CLI parity, typed Rust bridge methods, and explicit Flutter action dialogs. Drag/drop and playback remain out of scope. Desktop users can import, list, and remove media through the Rust project host; import requires a system-provided `ffprobe`. Phase 5D adds generated PNG video thumbnails and audio-only waveform previews to the existing Media panel through bounded background jobs and a disposable cache; preview generation requires a system-provided `ffmpeg`. Phase 5E adds a persistent disposable SQLite cache index and sequence-based LRU eviction when the global artifact budget requires space. Phase 5F adds a core-only, file-backed disposable video proxy profile to the same cache. Cache metadata and artifacts remain outside `.orproj` and do not affect `ProjectRevision`; the proxy is not exposed by the current UI, CLI, or IPC and is not a playback source. Project loading never opens or probes referenced sources, so a source may be offline or moved. Android builds, but project New/Open remain unavailable until Storage Access Framework support is implemented. There is no autosave, playback, rendering, or export.

The repository contains a Rust core and CLI, project identity and revision, exact-time values, `.orproj` v3 storage with v1/v2 migrations and canonical timeline state, crash-recovery checkpoints, shared command/query/transaction contracts, session-local history, local IPC, a typed Flutter-to-Rust bridge, the Phase 5A media probe foundation, and the Phase 5C–5F bounded job, cache-index, preview, and core proxy-generation path. The CLI provides read-only `or media probe` inspection, headless and attached media operations, and semantic timeline track/clip commands with exact rational times, including trim, split, and ripple delete. Media import probes one selected canonical local file with system `ffprobe`, then persists only its file URI and validated metadata, never the media bytes; `media probe` remains read-only and does not persist a `MediaId`. Headless project operations use the core file session; attached CLI operations connect to the same Flutter or developer-run `or session serve` host through an explicit descriptor. The shell includes Home, Projects, Templates, Asset Library, Settings, and an Editor Shell Preview for layout evaluation; real project workspaces also show the Phase 6D timeline editing actions. OR is not a usable video editor.

Stable application releases: none. Debug Developer Preview prereleases are available from [GitHub Releases](https://github.com/huou07/Opencut-Reinforced/releases) for native shell, project lifecycle, CLI, and architecture evaluation. The interactive HTML prototype remains a frozen product and UX reference, not the final application or its production architecture.

## Vision

OR aims to be a powerful but approachable editor that is desktop-first, Android-capable, local-first where practical, agent-native, and open to community-created content. Editing state should stay structured and inspectable, and editing behavior should be deterministic across the GUI, CLI, and agents.

## Architecture direction

The Flutter app and attached CLI share one `LiveProjectHost` and one `ProjectFileSession`; Flutter calls it through a typed opaque Rust handle, while the CLI reaches it through local IPC. Both use the same command/query dispatch and runtime identity, revision, history, and dirty state. The Flutter UI has a production-direction native shell aligned with the frozen prototype; the broader product architecture remains the intended direction:

    Flutter UI
        |
    shared Command API  <--- CLI
        |                 <--- Agents
    Rust Core
        |
    Timeline / Media / Render

Flutter is the presentation layer. The Rust core is intended to own editing and project truth so the GUI, CLI, and agents do not grow separate editing engines.

The project format has bounded filesystem load, race-safe no-clobber creation, and atomic save. Schema v1 projects migrate to empty media and timeline state; schema v2 projects retain media and migrate to an empty timeline. Older files are not rewritten on open; the next explicit save writes v3 without a conversion-only revision change. `or_core` provides explicit crash-recovery checkpoint APIs and the desktop UI offers explicit recovery choices. The bounded background Job Manager and disposable CacheStore generate read-only thumbnails and waveforms through system `ffmpeg`; Phase 5E adds a rebuildable SQLite index and automatic LRU eviction, and Phase 5F adds a file-backed Matroska proxy generation foundation under the same cache. Artifacts, index data, and source fingerprints are not project state. Autosave, Android Storage Access Framework integration, drag/drop timeline editing, snapping, markers, playback, media decode/encode processing, FFmpeg library integration, rendering, wgpu, and AI are not implemented. `ffprobe` and `ffmpeg` remain external system executables and are not bundled.

## Product direction

The planned product covers project and timeline editing, text and captions, audio, color and effects, export and interchange, creative assets, templates and themes, local and optional cloud AI, dubbing, safe agent and CLI workflows, and a future community ecosystem.

## Platforms

Current intended release targets:

- macOS
- Windows
- Linux
- Android

iOS and web are not current release targets.

## Design and documentation

The application design source of truth is [DESIGN.md](DESIGN.md). The full documentation map is [docs/INDEX.md](docs/INDEX.md), including the [product vision](docs/PRODUCT.md), [architecture](docs/ARCHITECTURE.md), and [roadmap](docs/ROADMAP.md).

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for the contributor path and repository expectations, and [docs/TOOLING.md](docs/TOOLING.md) for local checks and hosted platform verification.

## License

OR is licensed under the MIT License. See [LICENSE](LICENSE).
