# Opencut Reinforced

Last reviewed: 2026-11-09

A free and open-source video editor built around one structured editing core
shared by the human interface, CLI automation and AI agents.

## Status

**Pre-MVP with an active editor implementation, published as 0.1.28.** The
application creates, opens, edits, saves and recovers `.orproj` projects on
macOS, Windows, Linux and Android. A Rust-owned project session serves the
Flutter UI and an attached CLI through the same validated commands. Users can
import local media or Android SAF documents, choose a landscape, portrait or
square canvas, assemble and revise timeline clips, preview with a full transport
(speed, ±5 s, full screen), add titles and manual captions, import/export
subtitles, animate clips and scenes, clean up audio, and export Matroska, WebM or
MP4 at a chosen resolution.

Beyond editing, three surfaces share the same core:

- **Companion tools.** Settings → External tools probes the programs OR can use
  (FFmpeg, Node, SVGO, Inkscape, Blender, Audacity, Remotion, Motion Canvas, a
  Lottie renderer and a glTF renderer), prints the install command for the
  platform when one is missing, runs a ready tool, and records what produced each
  result. A companion result carries a provenance record into the Media Library
  and the timeline, so the source project, manifest, command and version are
  still readable after the media is on the timeline.
- **AI edit plans.** An agent writes an `or.edit-plan/1` document through MCP;
  the app reviews it and writes approved shots as ordinary clips. Edit styles and
  creative directions (bundled, and ones you keep) feed that plan.
- **Performance.** Settings → Performance reports what the machine actually uses
  for rendering and encoding, and the machine-fit mode is the only mode.

Public builds are published from
[GitHub Releases](https://github.com/huou07/Opencut-Reinforced/releases): a
macOS 13+ Apple Silicon DMG, a Debian/Ubuntu amd64 `.deb`, a Flatpak, a Windows
x64 setup and a portable ZIP. Install steps, permissions and uninstall steps live
in [docs/reference/install.md](docs/reference/install.md).

The project is still pre-MVP: no API, project-format or plugin stability is
promised. The macOS build is ad-hoc signed and **not** notarized. The Flatpak
builds and installs, but its sandbox runtime has not been proven on a machine
that can run `bwrap`, so treat it as untested rather than supported. Hardware
encode/decode paths are verified on the three hosts in
[docs/reference/hardware-profiles.md](docs/reference/hardware-profiles.md);
NVIDIA and AMD GPUs are untested here, and a machine with no usable GPU falls
back to the software backend. **Settings → Resources & updates** reports bundled
resources and checks for newer public builds. Current work is selected by
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

[docs/reference/install.md](docs/reference/install.md) covers installing,
permissions and uninstalling on each platform.
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
