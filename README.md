# Opencut Reinforced

A free and open-source cross-platform video editor designed around one structured editing core shared by the human interface, CLI automation, and AI agents.

## Execution and implementation evidence

Current checkpoint status is maintained only in
[STATE.json](docs/execution/STATE.json); [PLAN.json](docs/execution/PLAN.json)
selects the execution order. Inspect it with
`python3 scripts/execution_plan.py status`. Product direction below does not establish implementation or completion.

For an exact implemented checkpoint, consult its
[locked specification](docs/execution/phases) and
[supervisor evidence](docs/execution/evidence/README.md), then the source at
that evidence's commit. [Task documentation routing](docs/DOCUMENTATION.md)
selects the relevant contract and feature sections without requiring the full
architecture/testing history. The HTML prototype remains a frozen product and
UX reference, not production implementation.

## Vision

OR aims to be a powerful but approachable editor that is desktop-first, Android-capable, local-first where practical, agent-native, and open to community-created content. Editing state should stay structured and inspectable, and editing behavior should be deterministic across the GUI, CLI, and agents.

## Architecture direction

The intended architecture joins presentation, automation, and agents through one shared command boundary:

    Flutter UI
        |
    shared Command API  <--- CLI
        |                 <--- Agents
    Rust Core
        |
    Timeline / Media / Render

Flutter is the presentation layer. The Rust core is intended to own editing and project truth so the GUI, CLI, and agents do not grow separate editing engines.

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
