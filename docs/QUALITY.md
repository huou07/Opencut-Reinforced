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
| Performance | no benchmark budget exists yet | not measured | none — UNVERIFIED | gap |
| Reliability | project save/reopen, undo/redo, autosave and recovery in the named journeys | widget + preflight journeys | acceptance / hosted | partially verified |
| Accessibility / UX | 4.5:1 contrast, >= 11 px type, 24/44 px hit areas, keyboard alternatives | headless guards in `docs/UX_ACCEPTANCE.md` | widget tests | verified for the covered screens |
| Security | no secrets in tracked files; dependency boundary enforced | `check-repo.sh`, `check_or_core_deps.py`, gitleaks in publication flow | verify / hosted | local secret scan is NOT wired into the gate — gap |
| Compatibility | ubuntu/macOS/Windows/Android hosted jobs | CI matrix | hosted CI | hosted authority |
| Maintainability | index complete, links resolve, claims exact-SHA | doc gates | verify | verified |
| Cost | resource use of the bounded preflight | preflight headroom checks | acceptance (manual) | partially verified |

## Known gaps, in priority order

1. `scripts/check_claims.py` validates only the SHA shape and field presence, so a
   fabricated receipt passes; it must also require the SHA to exist in Git and be
   an ancestor of HEAD, and require the hosted run identity.
2. Local secret scanning is not part of the gate (`check-repo.sh` deliberately
   does not run gitleaks), so a leaked secret would not block a local pass.
3. No performance budget is enforced anywhere; `TESTING.md` explicitly disclaims
   the preflight as a benchmark.
4. Two Python suites (`scripts/test_android_preview_runner.py`,
   `scripts/test_verify_android_saf_recovery.py`) are referenced by nothing.
5. `docs/INDEX.md` omits `AGENT_POLICY.md`, `PAUSE_MIGRATION_HANDOFF.md` and
   `docs/skills.md`.
6. **Host gap, 2026-10-17:** the Flutter widget lane cannot run on this
   workstation. The native-assets hook tries to download Rust components
   (`rust-std-aarch64-apple-darwin` and others) and fails with
   `component download failed`; `scripts/acceptance.sh` therefore reports that
   lane as UNVERIFIED with the reason rather than a false PASS. Hosted CI owns it.

## Evidence rules

- Source gate: `bash scripts/check-repo.sh` — hygiene, documents and claims only.
- Acceptance gate: `bash scripts/acceptance.sh` — source gate plus the documented
  Rust test lanes and the Flutter widget suite.
- Release gate: GitHub Actions (`ui-preflight`, `platform-verification`,
  `developer-preview`) — the only authority for native, packaged and
  cross-platform claims.
- A skipped, missing or unavailable check is `UNVERIFIED`; it is never PASS.
