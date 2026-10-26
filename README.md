# Opencut Reinforced

Last reviewed: 2026-10-26

A free and open-source video editor built around one structured editing core
shared by the human interface, CLI automation and AI agents.

## Status

**Pre-MVP with an active editor implementation.** The application creates, opens,
edits, saves and recovers `.orproj` projects on macOS, Windows, Linux and
Android. A Rust-owned project session serves the Flutter UI and an attached CLI
through the same validated commands. Users can import local media or Android SAF
documents, assemble and revise timeline clips, preview, add titles and manual
captions, import/export subtitles, and export Matroska or WebM.

Stable releases: none. Debug Developer Preview prereleases are published from
[GitHub Releases](https://github.com/huou07/Opencut-Reinforced/releases) for
testing only; see [docs/reference/developer-preview-install.md](docs/reference/developer-preview-install.md).
Advanced editing, most codec profiles, AI workflows, hardware acceleration and
production signing remain open. Current work is selected by
[docs/reference/roadmap.md](docs/reference/roadmap.md).

## Architecture in one paragraph

Flutter is presentation. Rust owns canonical project state, history, recovery and
timeline semantics. The desktop app and an attached CLI share one live session —
the app through a typed opaque bridge, the CLI through local IPC — and both use
the same command/query dispatch, revisions and undo. Playback and rendering run
on a separate real-time plane that consumes immutable, versioned snapshots and
never mutates the project. See [docs/architecture/system-overview.md](docs/architecture/system-overview.md).

## Platforms

macOS, Windows, Linux and Android. iOS and web are not targets.

## Documentation

[AGENTS.md](AGENTS.md) is the routing index: one short description, one "when to
use it" trigger and one exact path per document, tool and skill. Design source of
truth: [docs/design/design-language.md](docs/design/design-language.md). Recorded
verification claims: [docs/evidence/claims/](docs/evidence/claims/). Notable
changes: [CHANGELOG.md](CHANGELOG.md). Superseded plans, delivery logs and dated
receipts are preserved under [docs/archive/](docs/archive/).

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for the contributor path, and
[AGENTS.md](AGENTS.md) for the document that owns whatever you are changing.

## License

MIT. See [LICENSE](LICENSE).
