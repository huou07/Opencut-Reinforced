# OR quality bar

Authority: this is OR's definition of "good" for the product. It does not replace
[AGENTS.md](../AGENTS.md), `docs/AGENT_POLICY.md`, `docs/UX_ACCEPTANCE.md` or the
project's gates; where they disagree, the source and the exact-SHA hosted runs
decide.

Last reviewed: 2026-10-18.

## Supported targets

| Target | Claimed support | Evidence class | Last exact-source run |
|---|---|---|---|
| Debian Linux (developer preview) | full for the documented journeys | VERIFIED_BEHAVIORAL via `scripts/run_linux_ui_preflight.py` | hosted `ui-preflight` + local preflight |
| macOS / Windows | built and packaged by CI | VERIFIED_NATIVE only when the hosted job passes | `platform-verification.yml` |
| Android | software preview with SAF media; no audio | hosted emulator lane | `platform-verification.yml` |

Not claimed: pixel-identical parity across platforms, Android audio, an
`apt`-mediated install/upgrade/purge of a locally built `.deb`, or `.deb` and
installer acceptance from this workstation. One local Linux `.deb` was built,
extracted into a disposable root and driven through the packaged journey at
`33b84eb`; that run is recorded in
[the 2026-10-18 local record](evidence/2026-10-18-linux-deb-local-acceptance.md),
and hosted `platform-verification` remains the packaging authority.

## Attributes

| Attribute | Target | How it is measured | Gate that enforces it | Current status |
|---|---|---|---|---|
| Correctness | acceptance gate passes | `bash scripts/acceptance.sh` | acceptance | see recorded result |
| Correctness (source) | hygiene, docs, claims, core dependency boundary | `bash scripts/check-repo.sh` | verify | see recorded result |
| Performance | one bounded local budget: CLI cold start <= 200 ms and the FFmpeg export lane <= 1500 ms, best of N, on the named Debian OptiPlex-class host | `python3 scripts/bench_quality.py`, run by `bash scripts/acceptance.sh` | acceptance | enforced locally; interactive frame-time/latency still UNVERIFIED |
| Reliability | project save/reopen, undo/redo, autosave and recovery in the named journeys | widget + preflight journeys | acceptance / hosted | partially verified |
| Accessibility / UX | 4.5:1 contrast, >= 11 px type, 24/44 px hit areas, keyboard alternatives | headless guards in `docs/UX_ACCEPTANCE.md` | widget tests | verified for the covered screens |
| Security | no secrets in tracked files or history; dependency boundary enforced | `gitleaks detect` (acceptance, and pinned in `repo-hygiene.yml`), `check_or_core_deps.py` | acceptance / verify / hosted | local scan runs when gitleaks is installed; `check-repo.sh` alone still does not scan |
| Compatibility | ubuntu/macOS/Windows/Android hosted jobs | CI matrix | hosted CI | hosted authority |
| Maintainability | index complete, links resolve, every current-claim doc dated, claims exact-SHA and resolvable in Git | doc and claim gates | verify | verified |
| Cost | resource use of the bounded preflight | preflight headroom checks and per-stage budgets | acceptance (manual) | partially verified |

## Performance: one bounded budget, and its limits

OR enforces one small performance budget, measured only on the named host class
below. It exists so an accidental catastrophic regression fails locally instead
of waiting for a user to notice.

| Row | What it times | Baseline (2026-10-18, idle) | Budget (best of N) |
|---|---|---|---|
| CLI cold start | `or version --json`, fresh process, N=15 | 1.1 ms best / 2.0 ms median | <= 200 ms |
| FFmpeg export lane | the ignored `software_export` WebM fixture (64x48, 10 frames, VP9/Opus), N=5 | 35.1 ms best / 42.1 ms median | <= 1500 ms |

`python3 scripts/bench_quality.py` measures both rows and
`bash scripts/acceptance.sh` runs it, so a breach fails the admission gate. The
budget is enforced on the **best of N** runs, not the mean, so a host busy with
sibling builds does not fail spuriously; a systematic regression still trips. A
host that cannot produce an honest number is reported `UNVERIFIED` with the
exact command that would settle it, never a fabricated threshold.

Named machine class: **Debian 13 (trixie) x86_64, OptiPlex-class workstation,
13th Gen Intel Core i5-13500T, 15 GiB RAM, NVMe SSD** — the host the baseline was
measured on. The budget is a property of this class. GitHub-hosted runners have
their own variance and keep their own thresholds; no number measured here is
claimed as a hosted-runner budget.

What is **not** covered, and stays a recorded gap:

- No frame-time or latency budget. The Linux preflight renders through software
  (`LIBGL_ALWAYS_SOFTWARE`) under Xvfb, and the Android lane runs a SwiftShader
  emulator; a frame time measured there describes the absence of a GPU, not the
  product. `TESTING.md` disclaims the preflight as a performance benchmark, and
  the roadmap's "responsive preview" promise still has no measurement.
- The only packaged export wall times are hosted values recorded in the roadmap
  history (Linux 47.299 s, macOS 57.394 s, Windows 182.114 s and 169.702 s for
  the same 30-second VP9/Opus journey) from GitHub-hosted runners of unstated
  hardware class. The Windows spread is runner variance; turning those numbers
  into a product requirement would encode CI noise as a budget.
- Physical-device Android responsiveness remains unmeasured.

Resource ceilings also stay enforced and fail on regression: the disposable
artifact cache's 8 MiB per-entry and 256 MiB total budgets with LRU eviction
(`cache::tests`, `media_artifacts::tests`), the bounded media page (50) and
timeline clip page (100), the at-most-64 simultaneously active Android preview
sources, and the preflight's own headroom and per-stage time limits. Those bound
resource use; they do not measure speed.

To extend the budget: add one row to `scripts/bench_quality.py` with the command
it times, a baseline measured on this named host and a stated margin, then let
`acceptance` enforce it.

## Known gaps

1. **No interactive performance budget** — the local CLI/export budget above is
   a coarse regression guard; frame time, seek latency and preview
   responsiveness remain unmeasured (see the section above).
2. **Local Flutter widget lane:** it runs on this host again. Corrected
   2026-10-18: `cd apps/or_app && flutter test --no-pub` passed 124 tests at
   `4f079e2` clean, and the acceptance gate no longer classifies the lane as a
   host gap. The earlier "read-only shared SDK cache / native-assets download"
   entry described a host state that no longer holds; a lane that fails now is a
   real `FAIL`, not a host gap.
3. **Android, macOS, Windows, packaging and codec lanes** remain hosted-CI
   authority for acceptance. Since 2026-10-18 the Android emulator lane can also
   run locally as developer evidence under
   [AGENT-AUTHORITY.md](AGENT-AUTHORITY.md); a local guest result never becomes
   `ACCEPTED`.
4. **No agent-facing UI harness and no screen inventory** — an agent can only
   observe the two screenshots and one clip semantics dump the bounded preflight
   happens to produce. It cannot list the app's screens and states, dump the
   semantics tree on demand, or act by semantics label on a target of its
   choosing. `docs/prompts/OR-BUILD.md` Phase 0 owns this gap; until
   then, UI/UX claims rest on widget tests plus two fixed screenshots.

## Evidence rules

- Source gate: `bash scripts/check-repo.sh` — hygiene, documents and claims only.
- Acceptance gate: `bash scripts/acceptance.sh` — source gate plus the documented
  Rust test lanes and the Flutter widget suite.
- Release gate: GitHub Actions (`ui-preflight`, `platform-verification`,
  `developer-preview`) — the only authority for native, packaged and
  cross-platform claims.
- The acceptance gate writes the machine-readable report `.hub/gate-report.json`
  declared by `.aeh/quality.json`: it names every lane the local run did not
  execute, the gates it actually covered, and the local runner class. The report
  is runtime state and is gitignored.
- A skipped, missing or unavailable check is `UNVERIFIED`; it is never PASS.
- Hosted evidence: after a successful hosted run for the current commit,
  `bash scripts/record-hosted-evidence.sh <workflow>` records it in `.hub/hosted-evidence.json`. The Hub
  merges it into a verification record only when it names that exact revision;
  without it `release-check` is `BLOCKED` by "no hosted-runner evidence", which
  is true, not a defect.
