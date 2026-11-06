# Tooling Inventory

Last reviewed: 2026-11-05

Scope: the scripts and machine-local tools an agent uses, and what each is for.
Host-specific commands are in [operations/local-macos.md](../operations/local-macos.md).

## Repository scripts

| Script | Purpose |
|---|---|
| `scripts/check-repo.sh` | source gate: required files, whitespace, licence, link/freshness/claim checks, `or_core` dependency boundary, shellcheck |
| `scripts/acceptance.sh` | source gate plus documented Rust/Flutter lanes and the local performance budget |
| `scripts/check-locks.sh` | resolve every committed `Cargo.lock` with `cargo metadata --locked --offline`, so a workspace version bump that was not carried into `tools/ffmpeg-link-probe/Cargo.lock` fails here instead of in the hosted Rust lane |
| `scripts/verify_pinned_assets.py` | check every pinned runtime download against the release it names: it reads the URL, byte count and SHA-256 out of `apps/or_app/lib/updates/resource_center.dart` and streams each archive from GitHub, so a stale constant fails here instead of when a user clicks Install; `--only` narrows to one archive and `--check` prints the recorded values without touching the network |
| `scripts/or_ui.py` | drive the real app by semantics and record a transcript; `--target macos|linux|windows`, where the Windows run needs `flutter.bat` and the FFmpeg prefix on `PATH` and records `shot-*.png.unavailable.txt` in place of an image (no raster surface in that test binding) |
| `scripts/or_tree_outline.py` | one line per semantics node with rect and actions |
| `scripts/packaged_drive.py` | CoreGraphics mouse/keyboard driving of the packaged app |
| `scripts/bench_quality.py` | local performance budget rows on macOS, Linux and Windows; on Windows the FFmpeg prefix `bin` must be reachable (the tool prepends it) or the export lane dies with `STATUS_DLL_NOT_FOUND`, and `command.*` stays unverified unless a helper with lavfi is on `PATH` — rows that cannot be measured say so instead of reporting a number |
| `scripts/probe_media_matrix.py` | measure the media matrix: build one fixture per container/codec, probe and decode it with the shipped FFmpeg, apply the Rust import profile, and emit a table or JSON; codec truth is read out of `crates/or_core/src/media_probe.rs` so the probe cannot drift |
| `scripts/bench_export.py` | portable export/render benchmark: `decode.<profile>`, `render.<profile>` and `full.<profile>` rows for mp4/webm/mkv at one picture size, driven through the bridge test `export_media_benchmark`; resolves the FFmpeg prefix from `FFMPEG_DIR`, the cargo config, or the known prefixes, runs the test binary directly with the library path it needs, and exits 3 when a row cannot be measured |
| `scripts/build_macos_release.sh` | build/package/codesign the macOS bundle with bridge hash check; a packaged local bundle also carries the `or` CLI the assistant edits through |
| `scripts/stage_macos_ffmpeg.sh` | build the pinned LGPL FFmpeg 8.1.3 prefix (codec sources, packaged helpers, VideoToolbox) that macOS app packaging consumes |
| `scripts/check_doc_links.py`, `scripts/check_doc_freshness.sh`, `scripts/check_claims.py` | documentation and claim gates |
| `scripts/check_or_core_deps.py` | keeps `or_core` free of GPU/media/audio/text/AI dependencies |
| `scripts/run_local_lane.sh`, `scripts/run_android_lane.sh` | platform lanes for future/other-platform work |
| `scripts/stage_desktop_runtime.py` | stage the pinned Whisper engine, the speech adapter and the `or` CLI into any desktop bundle, then start the staged binaries when the target matches the host; the one copy rule for macOS, Linux and Windows |
| `scripts/test_stage_desktop_runtime.py` | checks that the stager produces the same payload for Linux and Windows bundles and that its smoke check reports a binary that cannot start |
| `scripts/stage_codec_licences.sh` / `scripts/test_stage_codec_licences.py` | stage the third-party licence tree into a package and check that the x264 text is present only when the Linux build linked it |
| `scripts/windows_installer.iss` | Inno Setup 6 definition for the per-user Windows installer: `{localappdata}\Programs\Opencut Reinforced`, no administrator prompt, close-and-restart support for in-place updates |
| `scripts/record-hosted-evidence.sh` | record a hosted run for the current commit |
| `tooling/or-agent-workspace/build.mjs` | pack the installed assistant workspace into its Flutter asset; `node --test` beside it guards drift |

## Machine-local tools

| Tool | Use |
|---|---|
| CodeGraph (`codegraph`) | indexed symbol/source/relationship lookup; index lives in `.codegraph/` and is never committed |
| `dsh-tool-router` plugin | keeps heavy tool schemas and skill catalogues out of every request; expose them via `tool_search`, `tool_call`, `skill_search` |
| Playwright (`playwright-cli`) | inspect a rendered page (the HTML prototype) instead of guessing |
| Ponytail | prefer small necessary implementations while keeping correctness checks |

Third-party agent tooling must be reviewed before installation; do not install
overlapping instruction suites by default.

## Hosted CI jobs

Repository hygiene; Rust checks; Flutter static and widget checks; macOS native
build and bridge smoke; Linux native build; Windows native build; Android APK
build with SAF preview journey; Developer Preview packaging. These are the
canonical native and packaged evidence.

## Related

- [testing.md](testing.md)
- [operations/local-macos.md](../operations/local-macos.md)
- [operations/ui-harness.md](../operations/ui-harness.md)
