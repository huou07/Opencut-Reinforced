# One local Linux lane — 2026-10-18

Status: **local developer evidence** for exact source
`064edf2b28a3e6567ae9c86390b70d30b7f389db`, recorded on the named Debian
workstation. It is not hosted CI evidence: Flutter here is 3.47.7 while the
hosted jobs pin 3.47.5, so this run proves this tree's behaviour on this host,
not CI-parity acceptance. Hosted `platform-verification` remains the authority
for native, packaged and cross-platform claims.

This record covers `scripts/run_local_lane.sh linux`, the single entry point the
cluster adds. **Three of its four steps passed and the native step is `BLOCKED`
by host memory pressure, not by the product.** That is the honest result of this
session and it is recorded as such: the lane's own report says `BLOCKED` and the
lane exited 3.

## Result

| Item | Value |
|---|---|
| Source HEAD | `064edf2b28a3e6567ae9c86390b70d30b7f389db` |
| Command | `bash scripts/run_local_lane.sh linux` |
| Lane result | `BLOCKED` (exit 3): `hygiene` PASS, `flutter` PASS, `preflight` BLOCKED, `bench` PASS |
| Lane elapsed | 55.48 s (07:30:41Z → 07:31:36Z), clean tree |
| Report | `apps/or_app/build/local-lanes/linux.json` (gitignored), 4 steps, SHA-256 `be29133dd01e3b9f725ac5685e11c9139022380fc2afdf144ec0e6b563f8810b` |
| Host | Debian GNU/Linux 13 (trixie) x86_64, kernel `7.2.6+deb13-amd64`, 16,441,069,568 bytes RAM, 97,293,062,144 bytes free disk |
| Flutter | 3.47.7 (framework revision `abaf9c5237`); hosted jobs pin 3.47.5 |
| Rust | 1.98.0 (the version `crates/or_app_bridge/rust-toolchain.toml` declares) |

### Steps

| Step | Command the lane ran | Result | Exit | Elapsed | Log |
|---|---|---|---|---|---|
| `hygiene` | `bash scripts/check-repo.sh` | PASS | 0 | 33.93 s | `apps/or_app/build/local-lanes/linux-hygiene.log` |
| `flutter` | `cd apps/or_app && flutter --no-version-check test --no-pub` | PASS | 0 | 21.20 s | `apps/or_app/build/local-lanes/linux-flutter.log` |
| `preflight` | `python3 scripts/run_linux_ui_preflight.py` | BLOCKED | 3 | 0.04 s | `apps/or_app/build/local-lanes/linux-preflight.log` |
| `bench` | `python3 scripts/bench_quality.py --json` | PASS | 0 | 0.16 s | `apps/or_app/build/local-lanes/linux-bench.log` |

The Flutter step ran the full widget suite: `00:19 +124: All tests passed!`
(124 tests). The benchmark rows were `cli-cold-start` best 1.1 ms / median
1.3 ms (budget 200 ms) and `ffmpeg-export` best 16.1 ms / median 21.3 ms
(budget 1500 ms), kept as the machine-readable artifact
`apps/or_app/build/local-lanes/bench.json` (401 bytes, SHA-256
`5e5d022ac2becbb80c6b7ffbac7c7a8756b67929d024256d51eea5fb5a94bebb`).

## Why the native step is BLOCKED

`apps/or_app/build/local-lanes/linux-preflight.log` contains exactly:

```
BLOCKED: require 6 GiB available RAM and disk before starting
```

`MemAvailable` on this host sat between 3.90 GiB and 3.98 GiB for the whole
session (07:10Z–07:31Z). The cause is another worker's build on the same host,
outside this repository: three `java` processes owned by a different worker UID
(Gradle/Kotlin) held 3.48 GB + 0.86 GB + 0.53 GB RSS throughout. The bounded
preflight's floor is a deliberate invariant — it is what keeps a native run from
starving the host — so it was **not** lowered, and no cheaper check was
substituted for it. The lane kept running the steps that could still be honest
and stopped only at a real failure, which is why `bench` is present.

The last recorded *local* preflight pass is not this source: the still-present
`apps/or_app/build/ui-preflight/run.json` is from 13:30 at source
`4f079e2492cb451c18ed925b7c65fda4e44d2111`, exit 0, 112.05 s, minimum
7,343,734,784 bytes `MemAvailable`, Flutter phase 1.0 s and the native journey
111.04 s. It does not stand in for a run at `064edf2`, and this record does not
claim it does.

## Negative check: a deliberately broken source check

The cluster requires that a broken source check makes the lane exit non-zero.
Performed at `064edf2` on a deliberately dirty tree:

| Step | Detail |
|---|---|
| Injected break | appended one trailing-whitespace line to `README.md` |
| Command | `bash scripts/run_local_lane.sh linux --report apps/or_app/build/local-lanes/linux-negative-check.json` |
| Result | `FAIL`, exit 1: `hygiene` FAIL (exit 1, 34.06 s), `flutter`/`preflight`/`bench` NOT RUN |
| Report | `apps/or_app/build/local-lanes/linux-negative-check.json` |
| Restore | `README.md` rewritten in place from `git show HEAD:README.md` and byte-compared equal; `git status --porcelain` empty afterwards |

The lane stops at a real `FAIL`, so the broken gate is what fails the run rather
than a later symptom. `git checkout` cannot restore a tracked file owned by
another UID inside this sticky directory (`error: unable to unlink old
'README.md': Operation not permitted`), so the restore was done by writing the
exact content back; the byte comparison is the evidence that it worked.

## Cluster gate

`bash scripts/acceptance.sh` at `064edf2` on a clean tree:
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

`bash scripts/check-repo.sh` also passed on its own before each commit in this
cluster (414 tracked files at the last run).

## The `--packaged` path: implemented, not run here

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

**Neither packaged step was executed in this session.** It is the same host
memory floor that blocks them: `--exec` refuses to start below 6 GiB available
RAM, and the host never had that headroom. They are therefore `UNVERIFIED`, not
claimed. The settling command is
`bash scripts/run_local_lane.sh linux --packaged` on a host with the headroom,
and a missing pinned prefix is reported `BLOCKED` with the recipe that would
produce it rather than silently building against Debian's system FFmpeg 7.

## What this does not establish

- **Not hosted or CI-parity evidence.** Flutter 3.47.7 locally versus 3.47.5 in
  CI. No claim record is created from this run and no `ACCEPTED` status follows
  from it.
- **No native journey at this source.** The bounded preflight did not run, so
  this record claims no `VERIFIED_NATIVE` Linux UI behaviour at `064edf2`; the
  `BLOCKED` probe output above is the whole evidence for that step.
- **No packaged, installed-package or OS-picker claim**, and no `--packaged`
  execution at all.
- **One host, one architecture, one session.** amd64 Debian 13 only; no
  Android, macOS, Windows, physical-device or cross-platform claim.
- **No new performance claim.** The two benchmark rows are a coarse best-of-N
  regression guard on this named host class, not frame-time or latency budgets.
  The preflight, when it does run, renders with software GL under Xvfb.
- **The report is developer evidence, not an acceptance input.** It is written
  to a gitignored path and says `"acceptance": false` and
  `"evidence_class": "local-developer"` in its own payload.

## Where the evidence lives

- Lane report: `apps/or_app/build/local-lanes/linux.json` (gitignored).
- Negative check: `apps/or_app/build/local-lanes/linux-negative-check.json`.
- Local gate report: `.hub/gate-report.json` (gitignored).
- Step contract and limits: [TESTING.md](../TESTING.md#one-local-lane-entry-point).
- Host grants: [AGENT-AUTHORITY.md](../AGENT-AUTHORITY.md).
- Earlier local `.deb` build and packaged journey (different source):
  [2026-10-18-linux-deb-local-acceptance.md](2026-10-18-linux-deb-local-acceptance.md).
