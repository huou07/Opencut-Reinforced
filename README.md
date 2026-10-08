# Opencut Reinforced

A free and open-source cross-platform video editor designed around one structured editing core shared by the human interface, CLI automation, and AI agents.

## Status

**Pre-MVP, with an active cross-platform editor implementation.** The current
application creates, opens, edits, saves, and recovers `.orproj` projects on
macOS, Windows, Linux, and Android. A Rust-owned project session serves the
Flutter UI and attached CLI through shared validated commands. Users can import
multiple local media files on desktop and multiple Android SAF documents,
preview media, assemble and revise timeline clips, and export the supported
Matroska profile. Android projects use SAF documents and an app-private working
copy with explicit synchronization. Dirty work periodically updates a recovery
checkpoint without overwriting the canonical project file; Save remains
explicit.

The product is not yet a complete general-purpose editor: the current export
profile and editing tools are limited, and captions, richer audio/effects,
relinking, and focused AI workflows remain incomplete. Stable application
releases: none. Debug Developer Preview prereleases are available from
[GitHub Releases](https://github.com/huou07/Opencut-Reinforced/releases); they
are for testing, not production distribution. See the active outcome-based
[product roadmap](docs/PRODUCT_ROADMAP.md) and
[release limitations](docs/RELEASE.md).

The former checkpoint plan and state remain preserved for traceability in
[docs/execution/README.md](docs/execution/README.md); they do not select active
product work. The [convergence audit](docs/OPEN_SOURCE_CONVERGENCE.md) records
which components OR should keep, reuse, evaluate, or defer based on inspected
upstream evidence.

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

The project format has bounded filesystem load, race-safe no-clobber creation, explicit migrations, atomic save, and recovery checkpoints. Android project storage uses SAF documents with a bounded app-private working copy and explicit synchronization. Desktop and Android can import media through their platform permission models; generated previews use bounded background jobs and disposable indexed cache. The shared Rust session supports timeline tracks, clip insert/move/trim/split/ripple delete, markers, snapping, history, desktop preview, and a limited Matroska export profile. Proxies remain a cache foundation rather than an exposed workflow. Supported codec/container choices, advanced editing, audio tools, effects, captions, offline relinking, and AI assistance remain limited or incomplete. Packaged FFmpeg components and provenance are documented in the release plan.

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

The application design source of truth is [DESIGN.md](DESIGN.md). The full documentation map is [docs/INDEX.md](docs/INDEX.md), including the [product vision](docs/PRODUCT.md), [architecture](docs/ARCHITECTURE.md), [roadmap](docs/ROADMAP.md), and [execution control plane](docs/execution/README.md).

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for the contributor path and repository expectations, and [docs/TOOLING.md](docs/TOOLING.md) for local checks and hosted platform verification.

## License

OR is licensed under the MIT License. See [LICENSE](LICENSE).
