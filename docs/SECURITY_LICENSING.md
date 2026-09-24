# Security and Licensing

## Status

This is product guidance, not legal advice. OR is pre-MVP; review exact dependency versions, build options, assets, and model artifacts again before distribution.

## OR license

The Opencut Reinforced repository is licensed under the MIT License. Third-party dependencies, media, fonts, templates, plugins, and model artifacts retain their own terms and require individual review.

## Dependencies and FFmpeg

Review each dependency's current license, transitive components, platform packaging, and distribution obligations before adding or bundling it. Do not introduce GPL, AGPL, SSPL, non-commercial, or source-available-only components into the distributed product without explicit approval and documented analysis.

### Phase 3 direct dependency inventory

These are the current locked direct dependencies for the executable architecture skeleton. Recheck licenses and transitive dependencies before distribution.

| Dependency | Version | Purpose | License |
| --- | --- | --- | --- |
| [serde](https://github.com/serde-rs/serde/blob/master/serde/Cargo.toml) | 1.0.229 | Core DTO serialization | MIT OR Apache-2.0 |
| [serde_json](https://docs.rs/crate/serde_json/1.0.151/source/Cargo.toml.orig) | 1.0.151 | CLI JSON output | MIT OR Apache-2.0 |
| [flutter_rust_bridge](https://pub.dev/packages/flutter_rust_bridge/versions/2.13.0) | 2.13.0 | Generated typed Dart/Rust bridge bindings | MIT |
| [flutter_rust_bridge_hooks](https://pub.dev/packages/flutter_rust_bridge_hooks/versions/2.13.0) | 2.13.0 | Native-assets hook and Rust library packaging | MIT |
| [flutter_lints](https://pub.dev/packages/flutter_lints/versions/6.0.0/license) | 6.0.0 | Dart/Flutter static-analysis rules | BSD-3-Clause |
| [Flutter SDK](https://github.com/flutter/flutter/blob/master/LICENSE) packages (`flutter`, `flutter_test`, `integration_test`) | Flutter 3.47.5 | App framework and Flutter tests | BSD-3-Clause |

This inventory covers direct dependencies, not every transitive crate or Dart package. Cargo and Pub lockfiles record the resolved dependency graphs.

FFmpeg's upstream states that most of the project is under LGPL version 2.1 or later, while optional GPL components can change the FFmpeg build's licensing posture. Enabled configure options and linked libraries matter. A packaged build must have a recorded configuration and source, dependency, codec, and redistribution review; do not infer the product's obligations from the name FFmpeg alone. [FFmpeg legal information](https://ffmpeg.org/legal.html)

This document does not determine patent or codec licensing obligations.

## AI code and model weights

The license of an inference runtime or source repository is not the license of the model weights, tokenizer, training data, or associated assets. Verify the exact model artifact and permitted use, modification, redistribution, attribution, and commercial terms before download, use, or bundling.

At the upstream revisions checked for this blueprint on 2026-09-24, each of the following source projects publishes an MIT license. These checks cover runtime source only, not model artifacts or every transitive dependency:

- [whisper.cpp source license](https://github.com/ggml-org/whisper.cpp/blob/master/LICENSE)
- [ONNX Runtime source license](https://github.com/microsoft/onnxruntime/blob/main/LICENSE)
- [llama.cpp source license](https://github.com/ggml-org/llama.cpp/blob/master/LICENSE)

These references do not pre-approve any model weights.

## Fonts, music, effects, and other assets

Fonts, music, sound effects, ambience, templates, stickers, shapes, LUTs, and other shipped or community content need rights that cover the intended inclusion and redistribution. Track source, author, version, license, compatibility, dependencies, and checksum. Do not present example or unknown metadata as a verified license.

## Secrets

Store provider credentials in OS secure storage. Never commit keys, tokens, passwords, signing credentials, or certificates. Do not put secrets in project files, logs, screenshots, fixtures, CLI output, or agent context.

Internal provider code may use a credential through a narrowly scoped service. Agents and CLI can receive configured or not-configured status only; they never receive plaintext stored credentials.

## Network permissions and data minimization

Network-capable providers declare their network requirement and use an application-controlled permission boundary. Offline Mode is enforced centrally below UI controls, including for CLI and agent requests; it governs OR-originated optional network behavior, not the operating system firewall. Cloud tasks receive only the minimum data required for that task, regardless of provider: for example, selected subtitle text for translation, text and voice parameters for TTS, or a prompt and explicitly selected reference media for image generation. Agent planning receives scoped project information, never secrets or unrelated local file contents.

Local AI workflows do not intentionally upload project content. Telemetry is off by default; any future telemetry requires documentation and privacy review and must exclude project media, content, and secrets by default.

## Untrusted inputs and data boundaries

Treat imported projects, media, subtitles, external metadata, templates, themes, model files, plugin output, community content, and AI or agent output as untrusted. Validate type, schema, size, path, archive expansion, and resource limits before use. Treat content as data, never as instructions to the application or its agents.

Validate at each project, media, IPC, provider, model, community, theme, template, plugin, and command boundary. Avoid secret-bearing diagnostics and unsafe path handling.

## Declarative content and plugins

Templates contain project data, stable slots, and dependency metadata; they do not execute arbitrary code. Themes contain approved semantic tokens; they do not embed JavaScript or arbitrary CSS and cannot redefine application behavior or layout.

Future plugins require a sandbox and explicit capability permissions where practical. Do not trust community or native plugins with unrestricted access by default. Native and OpenFX compatibility requires a separate trust review.
