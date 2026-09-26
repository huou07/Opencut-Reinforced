# Security and Licensing

## Status

This is product guidance, not legal advice. OR is pre-MVP; review exact dependency versions, build options, assets, and model artifacts again before distribution.

## OR license

The Opencut Reinforced repository is licensed under the MIT License. Third-party dependencies, media, fonts, templates, plugins, and model artifacts retain their own terms and require individual review.

## Dependencies and FFmpeg

Review each dependency's current license, transitive components, platform packaging, and distribution obligations before adding or bundling it. Do not introduce GPL, AGPL, SSPL, non-commercial, or source-available-only components into the distributed product without explicit approval and documented analysis.

### Current direct dependency inventory

These are the current direct dependencies for the executable architecture and foundational core. Cargo and Pub lockfiles record resolved dependency graphs. Recheck licenses and transitive dependencies before distribution.

| Dependency | Version | Purpose | License |
| --- | --- | --- | --- |
| [serde](https://github.com/serde-rs/serde/blob/master/serde/Cargo.toml) | 1.0.229 | Core DTO serialization | MIT OR Apache-2.0 |
| [serde_json](https://docs.rs/crate/serde_json/1.0.151/source/Cargo.toml.orig) | 1.0.151 | CLI JSON output and strict `.orproj` codec | MIT OR Apache-2.0 |
| [uuid](https://github.com/uuid-rs/uuid) | 1.26.1 | Typed UUIDv4 project/runtime IDs and unique storage temp-file suffixes | Apache-2.0 OR MIT |
| [url](https://crates.io/crates/url/2.5.8) | 2.5.8 | Standards-based local file-URI validation and cross-platform native path conversion (`default-features = false`, `std` only) | MIT OR Apache-2.0 |
| [windows-sys](https://docs.rs/crate/windows-sys/0.61.2) | 0.61.2 | Windows-only atomic project-file replacement, named pipes, and owner-only endpoint ACLs (`cfg(windows)` target dependency) | MIT OR Apache-2.0 |
| [flutter_rust_bridge](https://pub.dev/packages/flutter_rust_bridge/versions/2.13.0) | 2.13.0 | Generated typed Dart/Rust bridge bindings | MIT |
| [flutter_rust_bridge_hooks](https://pub.dev/packages/flutter_rust_bridge_hooks/versions/2.13.0) | 2.13.0 | Native-assets hook and Rust library packaging | MIT |
| [file_selector](https://pub.dev/packages/file_selector/versions/1.1.0/license) | 1.1.0 | Flutter-ecosystem native open/save location selection for desktop projects | BSD-3-Clause |
| [flutter_lints](https://pub.dev/packages/flutter_lints/versions/6.0.0/license) | 6.0.0 | Dart/Flutter static-analysis rules | BSD-3-Clause |
| [Flutter SDK](https://github.com/flutter/flutter/blob/master/LICENSE) packages (`flutter`, `flutter_test`, `integration_test`) | Flutter 3.47.5 | App framework and Flutter tests | BSD-3-Clause |

This inventory covers direct dependencies, not every transitive crate or Dart package. Cargo and Pub lockfiles record the resolved dependency graphs.

`or_ipc` is an internal workspace crate and adds no new external production dependency. It reuses the existing `or_core`, `serde`, `serde_json`, and `uuid` dependencies; Windows API access uses the listed `windows-sys` dependency.

FFmpeg's upstream states that most of the project is under LGPL version 2.1 or later, while optional GPL components can change the FFmpeg build's licensing posture. Enabled configure options and linked libraries matter. A packaged build must have a recorded configuration and source, dependency, codec, and redistribution review; do not infer the product's obligations from the name FFmpeg alone. [FFmpeg legal information](https://ffmpeg.org/legal.html)

Phase 5A adds no FFmpeg production dependency: it neither links FFmpeg libraries nor bundles `ffmpeg`/`ffprobe`. Phase 5B persists only validated control-plane metadata and local `file:` source URIs in `.orproj` schema v2; project loading validates URI syntax but never opens or probes the referenced files, so a source can be offline or moved. The unchanged `.orproj` file ceiling is 64 MiB. V2 bounds each URI to 8,192 bytes; format names to 32 entries of 256 bytes each; streams to 4,096; codec names to 256 bytes; codec types and pixel formats to 128 bytes; and channel layouts to 256 bytes. Oversized metadata is rejected without truncation. The `url` crate is used only for standards-based URI parsing and native file-path conversion. The developer CLI and desktop import invoke a system-provided `ffprobe`; `OR_FFPROBE_PATH` is an optional local tooling override and is not stored in a project or accepted through IPC. The path is passed as a direct process argument with no shell. Probe execution is limited to 15 seconds, stdout to 1 MiB, and stderr to 64 KiB; timeout, overflow, and read failures terminate and reap the child. External JSON is parsed as untrusted input, unknown fields are ignored, required OR values and persisted metadata bounds are validated, and arbitrary tags are omitted from `MediaMetadata`. CI installs FFmpeg only on its hosted Linux Rust runner to generate and inspect a tiny synthetic test file; this CI tool is not included in application or CLI release packages. Future FFmpeg linking or packaging still requires the license review above.

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

The Phase 4E1 project-file boundary limits `.orproj` input to 64 MiB, reads at most one byte beyond that limit, rejects invalid UTF-8, and reuses the strict v1 codec. Saves encode and size-check before creating a same-directory temp file with exclusive creation; the old destination is never deleted first. Replacement is atomic on supported local filesystems, with file sync plus Unix directory sync or Windows write-through. A post-replace durability failure is reported as uncertain because replacement may already have happened. The current assumption is one owning application/session per save target; there is no file-locking framework or autosave scheduler.

Phase 4E2 recovery sidecars are untrusted input. Reads are bounded to 136 MiB, require strict UTF-8 and a strict v1 envelope, and validate each nested project through the `.orproj` codec and its 64 MiB limit. Checkpoint creation and candidate application require the exact saved base and matching project identity; inspection does not mutate state, and project load never applies a checkpoint automatically. Conflicts require explicit handling, and a failed apply preserves the checkpoint. Recovery snapshots contain `ProjectDocument` data only; credentials and secrets must remain outside project state and recovery files. The Flutter desktop UI now presents explicit candidate, stale, conflict, and invalid states. There is no periodic checkpoint scheduler or autosave.

Phase 4F local IPC is limited to control-plane requests. Protocol v1 uses a four-byte big-endian length prefix, a 1 MiB maximum frame, strict versioned JSON, UUIDv4 request IDs, and a fresh random authentication token per server. Authentication occurs before project details are returned or application requests are dispatched. The token is stored in the explicit endpoint descriptor, is never printed or logged, and is not included in `Describe` responses. On Unix, the server-owned runtime directory is mode 0700 and the socket and descriptor are mode 0600. On Windows, the default runtime directory, descriptor file, and named pipe receive protected owner-only DACLs; this also applies when a caller supplies an explicit descriptor path. Windows pipes reject remote clients. The CLI must be given the descriptor path; it does not scan for servers.

The IPC server exposes only `Describe`, shared application requests, explicit `Save`, and guarded `Shutdown`. Application requests carry the existing command/query/transaction envelopes. The server does not accept caller-selected filesystem paths, shell commands, user credentials, or arbitrary file reads/writes. There is no TCP, HTTP, WebSocket, or LAN listener or fallback. Other OS users cannot access the Unix endpoints or Windows objects through their ACLs. A malicious process running as the same OS user may be able to read the descriptor and authenticate, so this is not a same-user isolation boundary. Both `or session serve` and the Flutter desktop app can host a session. The app's bridge handle and IPC worker share one `ProjectFileSession`. Its descriptor path is shown only in Settings → Advanced / Developer; neither the token nor descriptor contents are displayed or included in CLI packages.

The sandboxed macOS app creates its short-lived IPC runtime directory under the app-provided temporary directory so the Unix socket fits the platform path limit and remains accessible to same-user CLI clients. The release app carries Apple's local server entitlement because it binds a local IPC endpoint; the implementation has no TCP or other network listener. The app also uses user-selected file read/write access for paths chosen by the native file selector. Android New/Open remain unavailable until Storage Access Framework support exists; content URIs are not treated as filesystem paths.

## Declarative content and plugins

Templates contain project data, stable slots, and dependency metadata; they do not execute arbitrary code. Themes contain approved semantic tokens; they do not embed JavaScript or arbitrary CSS and cannot redefine application behavior or layout.

Future plugins require a sandbox and explicit capability permissions where practical. Do not trust community or native plugins with unrestricted access by default. Native and OpenFX compatibility requires a separate trust review.
