# Security and Licensing

## Status

This is product guidance, not legal advice. OR is pre-MVP; review exact dependency versions, build options, assets, and model artifacts again before distribution.

The architecture execution lock is [docs/execution/README.md](execution/README.md).
Its policy checker keeps `or_core` free of runtime/platform dependencies and
requires an explicit gate before adding wgpu, FFmpeg bindings, native interop,
AI runtimes, model artifacts, or provider integrations. Checkpoint 7C0 approves
the FFmpeg binding and package strategy for the future media layer; it does not
add the binding to the production workspace. Runtime capability selection,
software fallback, model manifests, and secret boundaries are architecture
requirements, not optional cleanup.

The future AI boundary is task-oriented and provider-independent. Tasks return
proposals, analyses, or assets; normal validated commands apply accepted
results. Provider credentials remain in secure storage and are never returned to
agents or CLI callers as plaintext.

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
| [sha2](https://github.com/RustCrypto/hashes) | 0.11.0 | Deterministic SHA-256 cache-key derivation for the disposable media cache (`default-features = false`, `alloc` only) | MIT OR Apache-2.0 |
| [rusqlite](https://github.com/rusqlite/rusqlite) | 0.40.1 | Private disposable cache index only (`default-features = false`, `bundled` feature) | MIT |
| [windows-sys](https://docs.rs/crate/windows-sys/0.61.2) | 0.61.2 | Windows-only atomic project-file replacement, named pipes, and owner-only endpoint ACLs (`cfg(windows)` target dependency) | MIT OR Apache-2.0 |
| [flutter_rust_bridge](https://pub.dev/packages/flutter_rust_bridge/versions/2.13.0) | 2.13.0 | Generated typed Dart/Rust bridge bindings | MIT |
| [flutter_rust_bridge_hooks](https://pub.dev/packages/flutter_rust_bridge_hooks/versions/2.13.0) | 2.13.0 | Native-assets hook and Rust library packaging | MIT |
| [file_selector](https://pub.dev/packages/file_selector/versions/1.1.0/license) | 1.1.0 | Flutter-ecosystem native open/save location selection for desktop projects | BSD-3-Clause |
| [flutter_lints](https://pub.dev/packages/flutter_lints/versions/6.0.0/license) | 6.0.0 | Dart/Flutter static-analysis rules | BSD-3-Clause |
| [Flutter SDK](https://github.com/flutter/flutter/blob/master/LICENSE) packages (`flutter`, `flutter_test`, `integration_test`) | Flutter 3.47.5 | App framework and Flutter tests | BSD-3-Clause |

This inventory covers direct dependencies, not every transitive crate or Dart package. Cargo and Pub lockfiles record the resolved dependency graphs.

`or_ipc` is an internal workspace crate and adds no new external production dependency. It reuses the existing `or_core`, `serde`, `serde_json`, and `uuid` dependencies; Windows API access uses the listed `windows-sys` dependency.

Phase 7A adds the internal `or_runtime` workspace crate with only the existing
`or_core` path dependency and Rust standard-library synchronization primitives.
It inherits MSRV Rust 1.85 and the repository MIT license, adds no third-party
runtime or platform library, and carries no native handles, credentials, media
paths, or provider secrets. Checkpoint 7C0 separately selects the FFmpeg
binding/version/configuration below; other future runtime dependencies still
require an upstream version, MSRV, build/license, platform, and hosted-evidence
review before pinning.

### Checkpoint 7C0 FFmpeg binding and package decision

The selected Rust integration is the high-level [`ffmpeg-the-third` 6.0.0
release](https://crates.io/crates/ffmpeg-the-third/6.0.0), package version
`6.0.0+ffmpeg-9.0`, with its paired `ffmpeg-sys-the-third` 6.0.0 layer. The
maintained upstream fork declares support for FFmpeg 5.1–9.0 and its changelog
confirms continued 5.1–8.1 support in 6.0.0. The selected FFmpeg line is 8.1.x
with 8.1.3 as the pinned CI probe baseline. Both Rust packages declare WTFPL.
The binding has suitable wrappers
for demux, codec/decode, frames, software resampling, and software scaling.
Its build uses bindgen 0.72 plus the runtime `clang` crate, `pkg-config`, and a
C compiler; MSVC also uses the vcpkg path. The project MSRV remains Rust 1.85.
Linux/macOS source builds use FFmpeg's `configure`/`make`; CMake is not a direct
`ffmpeg-sys` link requirement. A Windows vcpkg port may have separate CMake
requirements, which belong in that platform's later build gate.

Dynamic shared-library linking is approved for macOS, Linux, and Windows.
Development environments need the matching FFmpeg 8.1.x headers, shared
libraries, and discovery metadata; runtime packages will carry the five core
shared libraries (`avcodec`, `avformat`, `avutil`, `swresample`, `swscale`) and
their transitive runtime libraries. Linux uses `.so` libraries and app-relative
loader paths, macOS uses `.dylib` libraries and app-relative loader paths in
the signed bundle, and Windows uses MSVC-compatible `.dll` libraries plus
`.lib` import libraries at build time and app-side DLLs at runtime. Android is
deferred to Phase 9 for a separate NDK/ABI and packaging decision. The hosted
probe currently verifies Linux compile/link/load only; macOS and Windows
package/link jobs remain future platform evidence.

The baseline is built from the official [FFmpeg 8.1.3
source](https://ffmpeg.org/releases/ffmpeg-8.1.3.tar.xz) as shared libraries.
The CI probe and release baseline disable autodetection and leave
`--enable-gpl`, `--enable-nonfree`, and `--enable-version3` unset; do not enable
the binding's corresponding `build-license-gpl`, `build-license-nonfree`, or
`build-license-version3` features. Do not link GPL or nonfree external codec
libraries such as libx264. FFmpeg's normal code is LGPL-2.1-or-later, but its
optional GPL components change FFmpeg's license to GPL-2-or-later, and its
nonfree configuration is not redistributable. The CI probe checks the linked
`libavutil` license string is exactly `LGPL version 2.1 or later`.
The probe alone uses `--disable-everything` to keep this gate limited to API,
ABI, and shared-link verification; it does not select the product codec set.

For each release, distribute the exact corresponding FFmpeg source and
configuration, local patch diff, license notices, and a source download
location; retain FFmpeg's library names and allow replacement of the dynamic
libraries. Keep the FFmpeg shared libraries separate from the MIT application
binary. Recheck the exact transitive native libraries and release configuration
before distribution. The FFmpeg license does not resolve patents or codec
royalties; review those when the shipped codec set is selected. This strategy
does not add the binding to the production workspace or create `or_media`.

FFmpeg's upstream states that most of the project is under LGPL version 2.1 or later, while optional GPL components can change the FFmpeg build's licensing posture. Enabled configure options and linked libraries matter. A packaged build must have a recorded configuration and source, dependency, codec, and redistribution review; do not infer the product's obligations from the name FFmpeg alone. [FFmpeg legal information](https://ffmpeg.org/legal.html)

Phase 5A/5B use a system-provided `ffprobe`; Phase 5D uses a system-provided `ffmpeg` for desktop media-library previews, and Phase 5F uses that system executable for core-only disposable Proxy V1 generation. OR neither links FFmpeg libraries nor bundles either executable, so these operations require the user to provide the relevant executable at runtime. Proxy V1 uses FFmpeg's native `mpeg4` encoder; it adds no external codec library or dependency and has no codec fallback. This does not settle patent or codec licensing obligations. Phase 5B persists only validated control-plane metadata and local `file:` source URIs in `.orproj` schema v2; project loading validates URI syntax but never opens or probes referenced files, so a source can be offline or moved. The next explicit save of a loaded v1 project writes v2 without a revision increment solely for schema conversion. The unchanged `.orproj` file ceiling is 64 MiB. V2 bounds each URI to 8,192 bytes; format names to 32 entries of 256 bytes each; streams to 4,096; codec names to 256 bytes; codec types and pixel formats to 128 bytes; and channel layouts to 256 bytes. Oversized metadata is rejected without truncation. The `url` crate is used only for standards-based URI parsing and native file-path conversion. Prepared desktop/CLI import canonicalizes and probes only the selected file, then sends a validated media item through `media.add`; the `media probe` command itself remains read-only. The CLI and desktop import invoke a system-provided `ffprobe`; `OR_FFPROBE_PATH` is an optional local tooling override and is not stored in a project or accepted through IPC. Probe paths are passed as direct process arguments with no shell. Probe execution is limited to 15 seconds, stdout to 1 MiB, and stderr to 64 KiB; timeout, overflow, and read failures terminate and reap the child. External JSON is parsed as untrusted input, unknown fields are ignored, required OR values and persisted metadata bounds are validated, and arbitrary tags are omitted from `MediaMetadata`. CI installs FFmpeg only on its hosted Linux Rust runner to run generated-media probe and artifact tests; this CI tool is not included in application or CLI release packages. Checkpoint 7C0 records the approved future shared-library strategy above; production linking has not started, and release-specific FFmpeg configuration, native dependency, and codec licensing review remains required.

This document does not determine patent or codec licensing obligations.

Phase 5C adds the `sha2` crate (MIT OR Apache-2.0, `default-features = false`, `alloc` only) solely for stable cache-key derivation, plus small local lowercase-hex formatting (no `hex` crate). Phase 5D adds no Rust or Flutter dependency; Phase 5E adds `rusqlite` 0.40.1 with default features disabled and only the `bundled` feature enabled; Phase 5F adds no dependency. The `JobManager` executes only Rust-internal closures; it never accepts a command, executable path, shell string, IPC request, CLI argument, or project-file value, and it has no access to mutable `ProjectDocument`. The `CacheStore` treats cache contents as untrusted disposable local data: it accepts only internally generated typed `CacheKey` values, derives final paths solely from those keys under a caller-supplied root, rejects byte entries larger than the configured per-entry limit before touching the filesystem, bounds byte reads to `max_entry_bytes + 1`, uses indexed artifact-byte accounting with persistent sequence-based LRU eviction only when the global budget requires space, reconciles disposable metadata, avoids following symlinks for accounting and namespace clearing, and exposes only explicit `remove`/`clear_namespace`/`clear_all` deletion beneath its own root. Proxy remains in the same index and global budget but uses an opaque same-directory hidden staging file reserved with `create_new`, a derived `.mkv` destination, and a typed file-backed API; it is never read wholly into a `Vec`. The proxy file bound is `min(4 GiB, max_total_bytes)`, output growth is monitored during generation, stdout is null and stderr is capped at 64 KiB, and cancellation, timeout, or oversize kills and reaps FFmpeg. Normal success, failure, timeout, cancellation, and oversize clean up staging. An abrupt process or OS crash can leave a hidden staging file; reconciliation ignores it, and no cross-process cleanup is attempted because the file may belong to an active generator. No cache artifact or job state stores credentials, tokens, or project data, and cache presence is never required for project correctness. Phase 5D's source-fingerprint v1 hashes file size, available modification time, and bounded sample windows solely for cache invalidation; it is not a full-file hash, MediaId, integrity proof, or project state. The generator takes the validated native local source path as a direct argument to `std::process::Command`, never shell text. Thumbnail/waveform timeouts are 20/30 seconds; stdout/stderr are capped at 8 MiB/64 KiB, and timeout, cancellation, overflow, or read failure kills and reaps the child. FFmpeg is neither linked nor bundled; CI-only FFmpeg tooling is not in application or CLI packages.

Phase 5E uses SQLite only for `<cache-root>/cache-index.sqlite3`, a rebuildable index for disposable thumbnail, waveform, and Proxy artifacts. The unchanged schema v1 stores only artifact kind, opaque lowercase cache key, byte size, and access sequence; it stores no media source paths, `MediaId`, project IDs, or artifact bytes. Proxy uses the same kind/key row shape and global LRU budget, and its `.mkv` is kept outside the byte API. `libsqlite3-sys` 0.38.2 is transitive and MIT-licensed; its bundled SQLite source is public domain under the [SQLite copyright terms](https://www.sqlite.org/copyright.html). The database is local-only and has no network capability. Database and journal storage are excluded from the artifact-byte budget. Filesystem artifact changes and SQLite metadata do not form one atomic transaction; the index is advisory and startup reconciliation or targeted repair restores consistency after a mismatch.

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
