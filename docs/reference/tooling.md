# Tooling Inventory

Last reviewed: 2026-11-03

Scope: the scripts and machine-local tools an agent uses, and what each is for.
Host-specific commands are in [operations/local-macos.md](../operations/local-macos.md).

## Repository scripts

| Script | Purpose |
|---|---|
| `scripts/check-repo.sh` | source gate: required files, whitespace, licence, link/freshness/claim checks, `or_core` dependency boundary, shellcheck |
| `scripts/acceptance.sh` | source gate plus documented Rust/Flutter lanes and the local performance budget |
| `scripts/or_ui.py` | drive the real app by semantics and record a transcript |
| `scripts/or_tree_outline.py` | one line per semantics node with rect and actions |
| `scripts/packaged_drive.py` | CoreGraphics mouse/keyboard driving of the packaged app |
| `scripts/bench_quality.py` | local performance budget rows |
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
