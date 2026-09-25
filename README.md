# Opencut Reinforced

A free and open-source cross-platform video editor designed around one structured editing core shared by the human interface, CLI automation, and AI agents.

## Status

**Pre-MVP:** Phase 3, Phase 4A–4E2 foundations, and Phase 4UI-1's production-direction Flutter visual foundation are implemented. Phase 4E2 adds a Rust crash-recovery checkpoint core; recovery UI and autosave are not implemented. Timeline and media editing have not started.

The repository contains a bootstrap Rust core and CLI, project identity and revision, exact-time values, `.orproj` v1 storage, command/query contracts, transactions, session-local history, and a typed Flutter-to-Rust bridge. The native Flutter shell includes Home, Projects, Templates, Asset Library, Settings, and an explicitly non-functional Editor Shell Preview. It is not a usable video editor.

Stable application releases: none. Debug Developer Preview prereleases are available from [GitHub Releases](https://github.com/huou07/Opencut-Reinforced/releases) for visual shell and architecture evaluation only. The interactive HTML prototype remains a frozen product and UX reference, not the final application or its production architecture.

## Vision

OR aims to be a powerful but approachable editor that is desktop-first, Android-capable, local-first where practical, agent-native, and open to community-created content. Editing state should stay structured and inspectable, and editing behavior should be deterministic across the GUI, CLI, and agents.

## Architecture direction

The CLI and Flutter shell receive bootstrap data from the same Rust core. The Flutter UI now has a production-direction native shell aligned with the frozen prototype; the broader product architecture remains the intended direction:

    Flutter UI
        |
    shared Command API  <--- CLI
        |                 <--- Agents
    Rust Core
        |
    Timeline / Media / Render

Flutter is the presentation layer. The Rust core is intended to own editing and project truth so the GUI, CLI, and agents do not grow separate editing engines.

The project format has bounded filesystem load and atomic save, and `or_core` provides explicit crash-recovery checkpoint APIs. Recovery UI, autosave, CLI project-file commands, timeline editing, media processing, rendering, FFmpeg, wgpu, and AI are not implemented.

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
