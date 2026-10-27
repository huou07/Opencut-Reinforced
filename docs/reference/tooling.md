# Tooling Inventory

Last reviewed: 2026-10-27

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
| `scripts/build_macos_release.sh` | build/package/codesign the macOS bundle with bridge hash check |
| `scripts/check_doc_links.py`, `scripts/check_doc_freshness.sh`, `scripts/check_claims.py` | documentation and claim gates |
| `scripts/check_or_core_deps.py` | keeps `or_core` free of GPU/media/audio/text/AI dependencies |
| `scripts/run_local_lane.sh`, `scripts/run_android_lane.sh` | platform lanes for future/other-platform work |
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
