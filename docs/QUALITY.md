# OR quality bar

Authority: this is OR's definition of "good" for the product. It does not replace
[AGENTS.md](../AGENTS.md), `docs/AGENT_POLICY.md`, `docs/UX_ACCEPTANCE.md` or the
project's gates; where they disagree, the source and the exact-SHA hosted runs
decide.

Last reviewed: 2026-10-17.

## Supported targets

| Target | Claimed support | Evidence class | Last exact-source run |
|---|---|---|---|
| Debian Linux (developer preview) | full for the documented journeys | VERIFIED_BEHAVIORAL via `scripts/run_linux_ui_preflight.py` | hosted `ui-preflight` + local preflight |
| macOS / Windows | built and packaged by CI | VERIFIED_NATIVE only when the hosted job passes | `platform-verification.yml` |
| Android | software preview with SAF media; no audio | hosted emulator lane | `platform-verification.yml` |

Not claimed: pixel-identical parity across platforms, Android audio, a packaged
`.deb`/installer built and installed locally.

## Attributes

| Attribute | Target | How it is measured | Gate that enforces it | Current status |
|---|---|---|---|---|
| Correctness | acceptance gate passes | `bash scripts/acceptance.sh` | acceptance | see recorded result |
| Correctness (source) | hygiene, docs, claims, core dependency boundary | `bash scripts/check-repo.sh` | verify | see recorded result |
| Performance | no enforced budget; see "Performance: no budget, and why" | not measured | none — UNVERIFIED | gap, recorded rather than hidden |
| Reliability | project save/reopen, undo/redo, autosave and recovery in the named journeys | widget + preflight journeys | acceptance / hosted | partially verified |
| Accessibility / UX | 4.5:1 contrast, >= 11 px type, 24/44 px hit areas, keyboard alternatives | headless guards in `docs/UX_ACCEPTANCE.md` | widget tests | verified for the covered screens |
| Security | no secrets in tracked files or history; dependency boundary enforced | `gitleaks detect` (acceptance, and pinned in `repo-hygiene.yml`), `check_or_core_deps.py` | acceptance / verify / hosted | local scan runs when gitleaks is installed; `check-repo.sh` alone still does not scan |
| Compatibility | ubuntu/macOS/Windows/Android hosted jobs | CI matrix | hosted CI | hosted authority |
| Maintainability | index complete, links resolve, every current-claim doc dated, claims exact-SHA and resolvable in Git | doc and claim gates | verify | verified |
| Cost | resource use of the bounded preflight | preflight headroom checks and per-stage budgets | acceptance (manual) | partially verified |

## Performance: no budget, and why

OR has **no enforced performance budget**. That is a recorded gap, not an
oversight, and the reasons are specific:

- No frame-time or latency budget can be measured honestly on the available
  hosts. The Linux preflight renders through software (`LIBGL_ALWAYS_SOFTWARE`)
  under Xvfb, and the Android lane runs a SwiftShader emulator; a frame time
  measured there describes the absence of a GPU, not the product. `TESTING.md`
  already disclaims the preflight as a performance benchmark.
- The only real measurements are hosted export wall times recorded in the
  roadmap history (Linux 47.299 s, macOS 57.394 s, Windows 182.114 s and
  169.702 s for the same 30-second VP9/Opus journey). They come from
  GitHub-hosted runners of unstated hardware class, and the Windows spread
  between those two runs is a runner-variance signal. Turning them into a
  threshold would encode CI runner variance as a product requirement.
- A budget needs a named representative device or runner class, a repeatable
  measurement command, and a recorded baseline with its variance. None of those
  exist yet, so any number written today would be prose.

What *is* enforced, and fails on regression, are resource ceilings rather than
performance budgets: the disposable artifact cache's 8 MiB per-entry and 256 MiB
total budgets with LRU eviction (`cache::tests`, `media_artifacts::tests`), the
bounded media page (50) and timeline clip page (100), the at-most-64
simultaneously active Android preview sources, and the preflight's own headroom
and per-stage time limits. Those bound resource use; they do not measure speed.

To add a real budget: pick one named runner or device class, add a benchmark
command that reports a single number with its variance, record a baseline, and
then enforce a threshold with margin in `acceptance` or hosted CI.

## Known gaps

1. **No performance budget** — see the section above for the reason and what a
   real budget requires.
2. **Local Flutter widget lane, 2026-10-17:** this workstation cannot run it.
   The `flutter` wrapper rewrites `bin/cache/engine.stamp` beside its own script
   and the shared SDK cache is read-only for this user, so the runner exits
   before any test starts. `scripts/acceptance.sh` reports that lane `UNVERIFIED`
   with the exact reason rather than a false PASS, and hosted CI owns it.
3. **Android, macOS, Windows, packaging and codec lanes** remain hosted-CI
   authority and are never claimed from this workstation.

## Evidence rules

- Source gate: `bash scripts/check-repo.sh` — hygiene, documents and claims only.
- Acceptance gate: `bash scripts/acceptance.sh` — source gate plus the documented
  Rust test lanes and the Flutter widget suite.
- Release gate: GitHub Actions (`ui-preflight`, `platform-verification`,
  `developer-preview`) — the only authority for native, packaged and
  cross-platform claims.
- A skipped, missing or unavailable check is `UNVERIFIED`; it is never PASS.
