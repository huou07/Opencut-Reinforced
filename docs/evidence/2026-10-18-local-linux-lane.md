# One local Linux lane — 2026-10-18

Status: **local developer evidence** for exact source
`b4a84956883422ed51ef1fb799d6cae5c9ff18b4`, the final source of this cluster,
recorded on the named Debian workstation. It is not hosted CI evidence: Flutter
here is 3.47.7 while the hosted jobs pin 3.47.5, so this run proves this tree's
behaviour on this host, not CI-parity acceptance. Hosted
`platform-verification` remains the authority for native, packaged and
cross-platform claims.

This record covers `scripts/run_local_lane.sh linux`, the single entry point the
cluster adds. **Three of its four steps passed and the native step is `BLOCKED`
by host memory pressure, not by the product.** That is the honest result of this
session and it is recorded as such: the lane's own report says `BLOCKED` and the
lane exited 3.

An earlier revision of this file recorded the run at `064edf2b28a3…`. That
commit is still an ancestor of the recorded source; the last packaged-path fix
`b4a8495` came after it, so the runs below were repeated at the final source
rather than cited from the older one.

## Result

| Item | Value |
|---|---|
| Source HEAD | `b4a84956883422ed51ef1fb799d6cae5c9ff18b4` |
| Command | `bash scripts/run_local_lane.sh linux` |
| Lane result | `BLOCKED` (exit 3): `hygiene` PASS, `flutter` PASS, `preflight` BLOCKED, `bench` PASS |
| Lane elapsed | 55.6 s (07:38:00Z → 07:38:55Z), clean tree, `dirty_at_start: false` |
| Report | `apps/or_app/build/local-lanes/linux.json` (gitignored), 4 steps, 4 records, SHA-256 `66c08aafa37b9155391880197f72ec495775d491ed719c6e7f6baef5f24f54b4` |
| Host | Debian GNU/Linux 13 (trixie) x86_64, kernel `7.2.6+deb13-amd64`, 16,441,069,568 bytes RAM, ~97 GB free disk |
| Flutter | 3.47.7 (framework revision `abaf9c5237`); hosted jobs pin 3.47.5 |
| Rust | 1.98.0 (the version `crates/or_app_bridge/rust-toolchain.toml` declares) |

### Steps

| Step | Command the lane ran | Result | Exit | Elapsed | Log |
|---|---|---|---|---|---|
| `hygiene` | `bash scripts/check-repo.sh` | PASS | 0 | 34.04 s | `apps/or_app/build/local-lanes/linux-hygiene.log` |
| `flutter` | `cd apps/or_app && flutter --no-version-check test --no-pub` | PASS | 0 | 21.19 s | `apps/or_app/build/local-lanes/linux-flutter.log` |
| `preflight` | `python3 scripts/run_linux_ui_preflight.py` | BLOCKED | 3 | 0.05 s | `apps/or_app/build/local-lanes/linux-preflight.log` |
| `bench` | `python3 scripts/bench_quality.py --json` | PASS | 0 | 0.18 s | `apps/or_app/build/local-lanes/linux-bench.log` |

The Flutter step ran the full widget suite: `00:19 +124: All tests passed!`
(124 tests). The benchmark rows were `cli-cold-start` best 1.1 ms / median
1.4 ms (budget 200 ms) and `ffmpeg-export` best 16.6 ms / median 19.4 ms
(budget 1500 ms), kept as the machine-readable artifact
`apps/or_app/build/local-lanes/bench.json` (401 bytes, SHA-256
`8d25bc5887358a4a92ef99af8b5e4dc83b02313c0ebc26127dd80a50935932a4`).

## Why the native step is BLOCKED

`apps/or_app/build/local-lanes/linux-preflight.log` contains exactly:

```
BLOCKED: require 6 GiB available RAM and disk before starting
```

`MemAvailable` on this host sat between 3.90 GiB and 3.98 GiB for the whole
session (07:10Z–07:39Z). The cause is another worker's build on the same host,
outside this repository: three `java` processes owned by a different worker UID
(Gradle/Kotlin) held ~3.48 GB + 0.86 GB + 0.53 GB RSS throughout. The bounded
preflight's floor is a deliberate invariant — it is what keeps a native run from
starving the host — so it was **not** lowered, and no cheaper check was
substituted for it. The lane kept running the steps that could still be honest
and stopped only at a real failure, which is why `bench` is present.

The last recorded *local* preflight pass is not this source: the still-present
`apps/or_app/build/ui-preflight/run.json` is from 13:30 at source
`4f079e2492cb451c18ed925b7c65fda4e44d2111`, exit 0, 112.05 s, minimum
7,343,734,784 bytes `MemAvailable`, Flutter phase 1.0 s and the native journey
111.04 s. It does not stand in for a run at `b4a8495`, and this record does not
claim it does. The lane makes the same distinction mechanically: the report
refused to record that file as this run's artifact because it predates the
preflight step, and says so in its `warnings` list.

## Negative check: a deliberately broken source check

The cluster requires that a broken source check makes the lane exit non-zero.
Performed at `b4a8495` on a deliberately dirty tree:

| Step | Detail |
|---|---|
| Injected break | appended one trailing-whitespace line to `README.md` |
| Command | `bash scripts/run_local_lane.sh linux --report apps/or_app/build/local-lanes/linux-negative-check.json` |
| Result | `FAIL`, exit 1: `hygiene` FAIL (exit 1, 30.48 s), `flutter`/`preflight`/`bench` NOT RUN |
| Report | `apps/or_app/build/local-lanes/linux-negative-check.json` |
| Restore | `README.md` rewritten in place from `git show HEAD:README.md` and byte-compared equal; `git status --porcelain` empty afterwards |

The lane stops at a real `FAIL`, so the broken gate is what fails the run rather
than a later symptom. `git checkout` cannot restore a tracked file owned by
another UID inside this sticky directory (`error: unable to unlink old
'README.md': Operation not permitted`), so the restore was done by writing the
exact content back; the byte comparison is the evidence that it worked.

## What reading the first report found, and the commits that followed

The cluster's first full lane run at `3c5d18d` was itself the instrument: reading
its report instead of trusting it exposed two artifacts that looked like evidence
and were not, and one behaviour that contradicted the documented rule.

| Commit | What it changed |
|---|---|
| `3c5d18d` | `feat(tooling)`: the lane, its report, and the `--exec` mode the packaged steps need |
| `f89e254` | `fix(tooling)`: a host-`BLOCKED` preflight stopped the lane and hid the performance budget; the lane now stops only at a real `FAIL` and continues after `BLOCKED`/`UNVERIFIED` |
| `064edf2` | `fix(tooling)`: the report credited the `BLOCKED` preflight step with an hour-old `run.json`, and kept an empty `bench.json` because the budget prints its rows only under `--json`; artifacts must now be non-empty and modified during their own step, and skipped ones are named in `warnings` |
| `b4a8495` | `fix(tooling)`: the packaged steps' `BLOCKED` branches bypassed the shared post-step rule, so a missing pinned prefix could hide a later step's evidence |

## Cluster gate

`bash scripts/acceptance.sh` at `b4a8495` on a clean tree:
`PROJECT_ACCEPTANCE=PASS with 8 UNVERIFIED lane(s)`, exit 0. Covered gates:
`source-integrity`, `product-build`.

| Lane | Result |
|---|---|
| Repository hygiene, document and claim gates | PASS |
| Rust core and CLI tests | PASS |
| Rust bridge, IPC and runtime tests | PASS |
| Media probe integration tests (real FFmpeg 8) | PASS |
| Media artifact integration tests (real FFmpeg 8) | PASS |
| Media crate tests | PASS |
| Media crate codec-enabled export writer | PASS |
| Performance budget | PASS |
| Flutter widget tests (fake gateway) | PASS |
| Secret scan (gitleaks 8.30.1) | PASS |
| Bounded Linux UI preflight | UNVERIFIED (host precondition, as above) |
| `or_render`, `or_audio`, `or_ai`, `or_whisper_provider` | UNVERIFIED (not selected by this bounded local gate) |
| `platform-verification`, `release-gate`, `packaging` | UNVERIFIED (hosted-CI authority) |

`bash scripts/check-repo.sh` also passed on its own before every commit in this
cluster (415 tracked files at the last run).

## The `--packaged` path: implemented, only its blocked branches exercised

`bash scripts/run_local_lane.sh linux --packaged` adds a `deb` step (build the
Linux bundle, stage the media runtime from the pinned local FFmpeg 8 prefix
`apps/or_app/build/local-deb/ffmpeg-install`, package through
`scripts/package_linux_deb.sh`) and a `journey` step (extract that `.deb` into a
disposable root and run `scripts/run_packaged_desktop_journey.py` against the
installed payload). Both run inside
`python3 scripts/run_linux_ui_preflight.py --exec`, which is how one command gets
the prepared developer environment: the project-local Cargo/pub caches, the
writable Flutter SDK mirror when the shared cache is read-only, the host-scoped
Rust toolchain manifest, and a Cargo config pinning the local FFmpeg 8 prefix
that is written for the build and removed afterwards.

**Neither packaged step was executed in this session.** The same host memory
floor gates them: `--exec` refuses to start below 6 GiB available RAM, and the
host never had that headroom. They are therefore `UNVERIFIED`, not claimed. What
*was* exercised is their precondition path, with a deliberately absent pinned
prefix (`OR_LANE_FFMPEG_PREFIX=/nonexistent/pinned-ffmpeg`): `deb` reported
`BLOCKED` naming every missing input and the recipe that would produce it
(`scripts/build_desktop_export_codecs.sh` plus the pinned FFmpeg 8 configure in
`.github/workflows/platform-verification.yml`), `journey` reported `BLOCKED`
naming that no `.deb` was produced, the lane reported `BLOCKED` and exited 3, and
neither step was silently reported as NOT RUN.

The settling command is `bash scripts/run_local_lane.sh linux --packaged` on a
host with the headroom. The earlier local `.deb` build and packaged journey at a
different source are recorded separately in
[2026-10-18-linux-deb-local-acceptance.md](2026-10-18-linux-deb-local-acceptance.md);
they were driven by scratch scaffolding under `apps/or_app/build/local-deb/`,
not by this lane, and that scaffolding is gitignored build state rather than a
reproducible entry point — which is what this cluster replaces.

## What this does not establish

- **Not hosted or CI-parity evidence.** Flutter 3.47.7 locally versus 3.47.5 in
  CI. No claim record is created from this run and no `ACCEPTED` status follows
  from it.
- **No native journey at this source.** The bounded preflight did not run, so
  this record claims no `VERIFIED_NATIVE` Linux UI behaviour at `b4a8495`; the
  `BLOCKED` probe output above is the whole evidence for that step.
- **No packaged, installed-package or OS-picker claim**, and no `--packaged`
  execution at all.
- **One host, one architecture, one session.** amd64 Debian 13 only; no Android,
  macOS, Windows, physical-device or cross-platform claim.
- **No new performance claim.** The two benchmark rows are a coarse best-of-N
  regression guard on this named host class, not frame-time or latency budgets.
  The preflight, when it does run, renders with software GL under Xvfb; the
  Android emulator and this lane were never run together.
- **The report is developer evidence, not an acceptance input.** It is written to
  a gitignored path and says `"acceptance": false` and
  `"evidence_class": "local-developer"` in its own payload.

## Where the evidence lives

- Lane report: `apps/or_app/build/local-lanes/linux.json` (gitignored).
- Negative check: `apps/or_app/build/local-lanes/linux-negative-check.json`.
- Packaged blocked-branch check:
  `apps/or_app/build/local-lanes/mechanics-packaged-blocked.json`.
- Local gate report: `.hub/gate-report.json` (gitignored).
- Step contract and limits: [TESTING.md](../TESTING.md#one-local-lane-entry-point).
- Host grants: [AGENT-AUTHORITY.md](../AGENT-AUTHORITY.md).

## Later run — all four steps PASS, 2026-10-18

The run above was `BLOCKED` only by host memory pressure. Once the sibling
project's Gradle daemon exited, the same lane on the same host passed every step
at source `1c731776130d0fe4a248a54137e8253cefa5f1c9`:

| Step | Result | Exit | Elapsed |
|---|---|---|---|
| `hygiene` | PASS | 0 | 36.24 s |
| `flutter` | PASS | 0 | 21.01 s |
| `preflight` | PASS | 0 | 20.09 s |
| `bench` | PASS | 0 | 0.17 s |

| Item | Value |
|---|---|
| Lane result | `PASS` (exit 0), `dirty_at_start: false`, 77.69 s |
| Report | `apps/or_app/build/local-lanes/linux.json`, SHA-256 `3acbe80fe26deeea...` |
| Host headroom at start | 7.54 GiB `MemAvailable`, no Gradle or Kotlin daemon resident |
| Preflight | `flutter test -d linux integration_test/linux_editor_preflight_test.dart` under Xvfb against the real Rust host; built `build/linux/x64/debug/bundle/or_app` and reported `All tests passed!` in 20 s |
| Flutter step | 124 tests, `All tests passed!` |
| Benchmark | `cli-cold-start` best 1.1 ms, `ffmpeg-export` best 14.8 ms, both inside budget |

This is the first run of this lane in which the native step was not `BLOCKED`.
It is still local developer evidence: Flutter 3.47.7 here against 3.47.5 in CI,
software GL under Xvfb, and no packaged or cross-platform claim.
