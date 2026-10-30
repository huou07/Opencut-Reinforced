# Test Inventory and Strategy

Last reviewed: 2026-10-26

Scope: which layer proves which contract, and where the existing suites live.
Phase-by-phase historical coverage records are in `docs/archive/legacy/TESTING.md`.

## Layers

| Layer | Proves | Lives in |
|---|---|---|
| Rust domain/application unit tests | commands, queries, timeline rules, undo, revision semantics | `crates/or_core/`, `crates/*/tests/` |
| Serialization and command contracts | project round trip, migrations, malformed input, command/query schemas, atomic save, recovery | `crates/or_core/` |
| CLI and agent contracts | discovery, exit codes, JSON shape, headless/attached parity, no secrets in output | `crates/or_cli/tests/` |
| IPC and shared host | protocol, ACLs, one session across bridge and CLI, event order | `crates/or_ipc/tests/` |
| Flutter widget tests | panels, focus, state rendering, accessibility guards | `apps/or_app/test/` |
| Real-app semantic journeys | user journeys through the running app | `scripts/ui_scenarios/` via `scripts/or_ui.py` |
| Render/audio/media integration | exact timing, decode/seek, queue bounds, export output | `crates/or_media/`, `crates/or_render/`, `crates/or_audio/` |
| Packaged and native journeys | app bundle behaviour, FFmpeg packaging, platform integration | GitHub Actions jobs |

## What to run

- Focused first: `cargo test -p <crate>`; escalate to the workspace only when the
  change crosses crates.
- Flutter: `cd apps/or_app && flutter analyze --no-pub && flutter test --no-pub`.
- UI behaviour: run the affected scenario with `scripts/or_ui.py`; add a scenario
  for a new journey.
- Export: inspect the artifact independently (`ffprobe`, decode), never the job
  status.

## Determinism rules

- Domain and CPU reference paths are deterministic; assert exact behaviour there.
- GPU/image comparisons must not assume bit-identical pixels across backends:
  use per-channel tolerance, max error, differing-pixel percentage or perceptual
  thresholds as appropriate, and fail clearly on semantic regressions.
- Preview and export golden cases verify the same edit semantics while allowing
  different resolution, quality, scheduling and encoders.
- No wall-clock dependence and no network in deterministic tests.

## Fixtures

Use generated or lawfully reusable small fixtures. Real-media fixtures are pinned
by exact source and licence. Do not commit generated media or model weights.

## Related

- [quality.md](quality.md)
- [operations/verification.md](../operations/verification.md)
- [operations/ui-harness.md](../operations/ui-harness.md)
