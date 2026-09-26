# Opencut Reinforced

A free and open-source cross-platform video editor designed around one structured editing core shared by the human interface, CLI automation, and AI agents.

## Status

**Pre-MVP:** Phase 3, Phase 4A–4F, Phase 4UI-1/4UI-2, and Phase 5A–5B are implemented; Phase 5 remains in progress, with Phase 5C next. The desktop app creates and opens real projects, hosts one Rust-owned live session shared by the Flutter bridge and authenticated local IPC, and supports rename, undo/redo, explicit save, close, recovery decisions, and dirty-state protection. Phase 5B adds a persistent project media library in `.orproj` schema v2, with v1 loading/migration, validated local-file references, `media.add`/`media.remove`, undo/redo, and paginated `media.list`. Desktop users can import, list, and remove media through the Rust project host; import requires a system-provided `ffprobe`. Project loading never opens or probes referenced sources, so a source may be offline or moved. Android builds, but project New/Open remain unavailable until Storage Access Framework support is implemented. There is no autosave, thumbnail or waveform generation, proxy/cache system, job scheduler, timeline editing, decode, playback, rendering, or export.

The repository contains a Rust core and CLI, project identity and revision, exact-time values, `.orproj` v2 storage with v1 migration, crash-recovery checkpoints, shared command/query/transaction contracts, session-local history, local IPC, a typed Flutter-to-Rust bridge, and the Phase 5A media probe foundation. The CLI provides read-only `or media probe` inspection plus headless and attached media list/add/remove operations. Media import probes one selected canonical local file with system `ffprobe`, then persists only its file URI and validated metadata, never the media bytes; `media probe` remains read-only and does not persist a `MediaId`. Headless project operations use the core file session; attached CLI operations can connect to either the Flutter app or developer-run `or session serve` host through an explicit descriptor. The shell includes Home, Projects, Templates, Asset Library, Settings, and an explicitly non-functional Editor Shell Preview. OR is not a usable video editor.

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

The project format has bounded filesystem load, race-safe no-clobber creation, and atomic save. Schema v1 projects load with an empty in-memory media library and are written as v2 only on the next explicit save; schema conversion alone does not change project revision. `or_core` provides explicit crash-recovery checkpoint APIs and the desktop UI offers explicit recovery choices. Autosave, Android Storage Access Framework integration, thumbnails, waveforms, proxies, cache storage, a job scheduler, timeline editing, media decode/encode processing, FFmpeg library integration, rendering, wgpu, and AI are not implemented. `ffprobe` remains an external system executable and is not bundled.

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
