# Finish OR on the local Linux host and the local Android emulator

Owner instruction, recorded 2026-10-18 in the AEH control session. Authority for
what an agent may do without asking is [AGENT-AUTHORITY.md](../AGENT-AUTHORITY.md);
this file selects work and defines completion. It is a prompt artifact, not a
second roadmap: the product backlog stays
[PRODUCT_ROADMAP.md](../PRODUCT_ROADMAP.md).

## Owner intent (2026-10-18)

1. Finish OR for **Linux on this host** and **Android on the local emulator** first.
2. The features named in the architecture must actually work on those two
   targets — not a shell, not a checklist.
3. Good performance, good threading/parallelism, and **low latency that is
   measured and benchmarked**, not asserted.
4. Good UI/UX, with tooling that lets an agent **read the interface and act on
   it**, on both targets.
5. Easy to understand and easy to use; optimise the real product, not only the
   code.
6. Tooling that lets an agent see the real product and run meaningful tests
   against it.
7. Commit after each feature cluster.

## Verified starting state (measured 2026-10-18, ~13:50–14:05 +07)

Re-verify before relying on it; every row is a claim with a command.

| Fact | Value |
|---|---|
| Repository | `/home/huou07/Projects/OR`, branch `feat/semantics-journey-guards`, HEAD `4f079e2492cb`, clean tree, no other worktree |
| Hygiene gate | `bash scripts/check-repo.sh` — recorded PASS at this HEAD |
| Acceptance gate | `bash scripts/acceptance.sh` — recorded PASS at this HEAD (`.hub/gate-report.json`, 13:30) |
| Performance budget | `python3 scripts/bench_quality.py` — PASS, run for this handover: CLI cold start 1.2 ms best / 1.4 ms median (budget 200 ms); FFmpeg export fixture 19.0 ms best / 19.4 ms median (budget 1500 ms) |
| Local Flutter suite | `cd apps/or_app && flutter test --no-pub` — **PASS, 124 tests** (run for this handover). This contradicts `QUALITY.md` "known gap 2"; the gap is stale |
| Linux runtime lane | `python3 scripts/run_linux_ui_preflight.py` — PASS at `4f079e2` clean, 112.05 s (`apps/or_app/build/ui-preflight/run.json`); Xvfb + `LIBGL_ALWAYS_SOFTWARE`, fixture picker, 2 PNGs + 1 clip semantics dump |
| Host | Dell OptiPlex Micro 7010, i5-13500T (20 threads), 15 GiB RAM, ~94 GB free on `/`, bare metal (`systemd-detect-virt` = none) |
| GPU | Intel UHD 770 (ADL-S GT1), Vulkan 1.4.305 / Mesa ANV 25.0.7; `/dev/dri/renderD128` readable+writable; llvmpipe also present |
| KVM | `/dev/kvm` readable+writable; `emulator -accel-check` → "KVM (version 12) is installed and usable" |
| Android SDK | `~/Android/Sdk` (platform-tools, emulator, ndk 26.3/27.2/28.2, platforms, `system-images;android-36;google_apis;x86_64`); AVD `an3-api36` (x86_64, `hw.ramSize=2048`, `hw.cpu.ncore=2`, `swiftshader_indirect`) |
| Existing local Android build | `apps/or_app/build/app/outputs/apk/debug/app-debug.apk`, 109,993,015 bytes, built 2026-10-18 13:05; contains a genuine NDK r28c Android x86_64 `libavcodec.so`; `adb -s emulator-5554 shell pm list packages` shows `io.github.huu07.or_app` installed. **No command, staging root or provenance for it is recorded in the repository**, and the staging path `android/app/src/main/jniLibs` is not in `.gitignore` |
| Emulator contention | During this inspection an emulator (`qemu-system-x86_64-headless -avd an3-api36 -gpu swiftshader_indirect -accel on`) was running while a second session built another project's APK. Treat the host and the `emulator-5554` serial as shared |
| Tools | `flutter` 3.47.7 (CI pins 3.47.5), Rust 1.98.0, `gitleaks` installed, `playwright-cli` installed (prototype only), system `ffmpeg` 7.1.5 |

## Stale claims that must not block you

- `docs/AGENT-AUTHORITY.md` previously said Android was **unavailable** on this
  host (no SDK/NDK/adb/AVD, UID outside `kvm`). It is false today; corrected in
  the commit that added this file.
- `docs/QUALITY.md` "known gap 2" no longer claims the local Flutter widget lane
  cannot run; it was corrected on 2026-10-18 and records the 124-test pass.
- `AGENT-AUTHORITY.md` also said a raw `flutter test` fails on macOS-target
  resolution. It does not today.
- `docs/AGENT_POLICY.md` and `docs/DEVELOPMENT_WORKFLOW.md` previously stated a
  flat ban on launching a local emulator. Both were reconciled on 2026-10-18 with
  the two owner-granted local lanes; if you find a surviving flat ban, it is a
  defect — the grant in `AGENT-AUTHORITY.md` wins.
- The dated handoffs `CONTINUE-OR.md` and `FINISH-OR.md` keep inspections that
  predate this state. They now live in `docs/archive/prompts/`; read them for
  method, not for current facts.

A stale capability claim is a real defect: it makes agents refuse work that is
possible and trust a boundary that does not exist. When you find one, correct it
in the same commit as the evidence that disproves it.

## Definition of done for this handover

1. `bash scripts/run_local_lane.sh linux` and `bash scripts/run_local_lane.sh android`
   are the two supported local entry points. Each runs the real product, writes
   one machine-readable report plus screenshots, and names its evidence class and
   its limits.
2. An agent can, from one command, list the app's screens/states, dump the
   semantics tree, take a screenshot, and act (tap/type/key/scroll) by semantics
   label — on Linux and on the Android emulator — and get JSON another agent can
   read.
3. `python3 scripts/bench_quality.py` reports measured p50/p95 latency for the
   interactive and media paths plus render/encode throughput on this named host
   (real Intel GPU), and `scripts/acceptance.sh` fails on a regression.
4. Playback-critical work (decode, audio, render) has priority over opportunistic
   work (thumbnails, waveforms, proxies, analysis); a test proves an
   opportunistic load cannot starve or delay it.
5. Every roadmap work-stream A–E journey runs on Linux **and** on the Android
   emulator, or is recorded `UNSUPPORTED` with the reason. No silent gaps.
6. Every cluster ends in its own commit(s) after `check-repo.sh`; the cluster
   boundary also runs `acceptance.sh`; the roadmap is updated with exact source
   and evidence class.

## Non-goals

AI beyond local English transcription, plugins/OpenFX/WebMotion, a community
registry, templates/themes/MotionScene materialisation, dubbing, signing and
notarisation, stable release, iOS/web, and media-engine replacement (MLT/GES)
without a measured OR gap. Do not start a second roadmap or revive
`docs/archive/execution/`.

Two owner decisions are still open and are **not** to be invented:
(a) whether "full architecture features" also means streams F/G (AI Studio,
plugins, templates, dubbing) on these two targets — this handover bounds the scope
to streams A–E plus the platform/tooling/perf/UX requirements above;
(b) whether local Android results may ever be promoted from developer evidence,
which today requires the hosted `platform-verification` run.

## Clusters

Run them in order. One writer per repository: do not run two of these in the same
working tree at the same time. If the owner explicitly wants two at once, use
`git worktree` with disjoint write scopes and merge through pull requests.

### C0 — truthful local capability record — LANDED 2026-10-18

Docs only: `AGENT-AUTHORITY.md` host facts and grants, `TOOLING.md` local-execution
rule, `QUALITY.md` stale gap, `INDEX.md` pointer. Acceptance: `check-repo.sh` PASS
and no row claims a lane that did not run. Treat C0 as done; do not re-litigate it.

### C1 — one honest local Linux lane

- **Outcome:** `bash scripts/run_local_lane.sh linux` runs, from a clean tree:
  the hygiene gate, the Flutter widget suite, the bounded Linux preflight, and the
  performance budget; it writes one report and exits non-zero on a real failure.
  `--packaged` adds the local `.deb` build plus `scripts/run_packaged_desktop_journey.py`
  and refuses to call that packaged acceptance.
- **Write scope:** `scripts/run_local_lane.sh` (new), `scripts/run_linux_ui_preflight.py`,
  `scripts/package_linux_deb.sh`, `docs/TESTING.md`, `docs/DEVELOPMENT_WORKFLOW.md`.
- **Acceptance:** from clean HEAD one command exits 0 and leaves
  `apps/or_app/build/local-lanes/linux.json` plus
  `docs/evidence/<date>-local-linux-lane.md` naming the exact commands, timings,
  artifact hashes and what the run does not establish. A deliberately broken
  source check makes it exit non-zero.
- **Stop:** if a lane cannot run for a host reason, record `BLOCKED` with the
  probe output; never substitute a weaker check or a PASS.
- **Commit:** `feat(tooling): one local Linux lane with a machine-readable report`.

### C2 — one reproducible local Android emulator lane

- **Outcome:** `bash scripts/run_local_lane.sh android` boots a dedicated AVD with
  KVM, builds the debug APK (with the x86_64 Android FFmpeg staged reproducibly),
  installs it, drives the bridge-diagnostics and preview-surface integration
  tests, optionally the SAF journey, then stops the guest and writes a report —
  with no dependency on `GITHUB_WORKSPACE`/`RUNNER_TEMP`.
- **Write scope:** `scripts/run_local_lane.sh`, `scripts/start-android-emulator.sh`,
  `scripts/run-android-preview-check.sh`, `scripts/stop-android-emulator.sh`,
  a documented local FFmpeg-for-Android staging step, `.gitignore`, `docs/TOOLING.md`,
  `docs/TESTING.md`.
- **Must reconstruct first:** the 2026-10-18 13:05 APK exists but its build
  commands and FFmpeg staging root are unrecorded. Reproduce them from the
  repository's own pinned recipe in `.github/workflows/platform-verification.yml`
  (`android` job, NDK `28.2.13676358`, API 26, x86_64 only locally) before adding
  anything new; record the one-time cross-build cost. Add `android/app/src/main/jniLibs/`
  to `.gitignore` with the reason.
- **Resource rules:** the guest takes ~3.3 GiB and Gradle/JVMs ~4.4 GiB on a
  15 GiB host. Build the APK **before** booting the guest; check `MemAvailable`
  and refuse below the existing floor; use a dedicated AVD/serial so you cannot
  collide with another project's emulator; always stop what you started.
- **Acceptance:** report names the AVD, serial, GPU mode, ADB/KVM probe output,
  APK hash, each test result, and the guest/host health samples. A deliberate
  emulator loss is classified `EMULATOR_INSTABILITY` (the existing classifier),
  never retried into a pass.
- **Stop:** a guest that never settles inside the bounded probes is `BLOCKED`
  with the measured reason.
- **Commit:** `feat(tooling): a reproducible local Android emulator lane`.

### C3 — agent UI sense-and-act harness

- **Outcome:** one harness that lets an agent observe and drive the real app:
  `python3 scripts/or_ui.py screens | tree | shot | act | run <scenario.json>`.
  It supports Linux (Xvfb + the real Rust host) and the Android emulator, and
  emits a step-by-step JSON transcript (action, selector, result, elapsed ms,
  screenshot path, semantics snapshot path).
- **Write scope:** `scripts/or_ui.py` (new), `apps/or_app/integration_test/ui_driver_test.dart`
  (new), `apps/or_app/lib/` only if a missing semantic node needs one,
  `scripts/ui_scenarios/` (new), `docs/UI_AGENT_TOOLING.md` (new), `docs/INDEX.md`,
  `docs/UX_ACCEPTANCE.md`.
- **Rules:** act by semantics label/role/key, never by screen coordinates; never
  inject state to force a pass; keep the DOM/semantics tree honest — if an
  interactive element has no semantic node, record it with the smallest remedy
  instead of wrapping everything.
- **Acceptance:** one scenario covers Home → Projects → open/create → import a
  synthetic fixture → edit → undo → seek → save → reopen → export setup → cancel,
  passes on both targets, and leaves JSON + PNGs per step. A deliberately wrong
  selector fails loudly. Keyboard reachability is asserted for the same journey.
- **Stop:** a target that cannot expose the needed node is reported with the
  missing node and the smallest fix, not worked around with clicks.
- **Commit:** `feat(tooling): an agent UI sense-and-act harness for Linux and Android`.

### C4 — measured latency, throughput and priority scheduling

- **Outcome:** `scripts/bench_quality.py` grows from two rows to a measured
  product budget on this named host, each row printing baseline, margin and
  `UNVERIFIED` when it cannot be measured honestly:
  `cli-cold-start` and `ffmpeg-export` (keep), `command-latency` (p50/p95 for
  create/import/add/trim/split/undo/seek through the CLI on a ~200-clip project),
  `project-open`/`project-save` (large project), `render-frame-time` (offscreen
  wgpu through `or_render` on the real Intel UHD 770: identity, transform, text),
  `export-throughput` (frames/s for the 30 s VP9 job), `queue-priority`
  (playback-critical p95 while opportunistic work saturates the pool).
  `scripts/acceptance.sh` enforces the budgets.
- **Threading:** implement the priority boundary `ARCHITECTURE.md` names as
  missing ("There is still no job progress API, job persistence, or priority
  system"): playback-critical decode/audio/render first, opportunistic
  thumbnail/waveform/proxy/analysis last, with backpressure instead of unbounded
  queues. Measure before/after on this host.
- **Write scope:** `scripts/bench_quality.py`, `scripts/acceptance.sh`,
  `crates/or_runtime/`, `crates/or_media/`, `crates/or_render/`,
  `docs/QUALITY.md`, `docs/TESTING.md`, `docs/ARCHITECTURE.md` (status line only).
- **Acceptance:** every row has a before/after number from this host; an injected
  regression (for example a deliberate sleep or a serialised encode) trips the
  gate; the priority test fails if the priority split is removed; the docs state
  exactly what the numbers do not establish (Xvfb software rendering,
  SwiftShader, emulator CPU, hosted-runner variance).
- **Stop:** an unmeasurable row is `UNVERIFIED` with the exact settling command —
  never a fabricated threshold, and never a GPU claim from a software path.
- **Commits:** one per row group, e.g. `perf(bench): measure command and project
  latency on the named host`, then `perf(runtime): prioritise playback-critical
  work over opportunistic jobs`.

### C5 — finish features and UX on both targets

Repeat one journey per iteration until the roadmap's A–E journeys hold on Linux
and Android, or are recorded `UNSUPPORTED` with the reason. Each iteration:

1. pick the highest-value incomplete journey from `PRODUCT_ROADMAP.md`;
2. reproduce it with C3's harness on both targets;
3. fix the first real gap; add the narrow regression;
4. re-run `run_local_lane.sh linux` and `... android`, then `acceptance.sh`;
5. commit; update the roadmap with exact source and evidence class.

Known backlog this loop must cover, in the order the roadmap names it:

- media: missing-source and bulk offline/relink UX, large-library cache
  behaviour, folder-import bounds feedback;
- preview: play/pause/seek/scrub/frame-step acceptance, dropped-frame honesty,
  audio-device fallback, Android audio decision (implement or record `UNSUPPORTED`);
- export: progress/cancel/failure on both targets, Android Matroska vs WebM
  decision at the packaged runtime, independent decode checks;
- editing: clip/track operations parity, zoom/scroll, touch and keyboard paths;
- mobile UX: compact layout, sheets, 320 px top bar, persistent navigation;
- first-run: understandability — empty states, error copy, recovery guidance,
  install/dogfood instructions.

Maintainability guard: `apps/or_app/lib/screens/editor_shell_preview_screen.dart`
is 7,536 lines. Before adding UI to it, extract the viewer/inspector/timeline
panels into their own files with their tests, and show the extraction did not
regress C4's frame-time row.

## Commit and evidence discipline

- Conventional commits, one cluster per commit where possible:
  `feat|fix|perf|test|docs|ci|refactor(scope): ...`.
- Before a commit: `bash scripts/check-repo.sh`. At a cluster boundary:
  `bash scripts/acceptance.sh`. Record both results.
- Never commit: `apps/or_app/build/**`, `target/**`, `.hub/**`, `.dart_tool/**`,
  `android/app/src/main/jniLibs/**`, emulator images or downloaded models. If a
  generated path is newly produced, add it to `.gitignore` with the reason.
- Leave the tree clean at every cluster boundary: the Linux preflight refuses a
  dirty tree and a dirty tree makes every later claim unattributable.
- Local runs are developer evidence. Hosted `platform-verification` remains the
  acceptance authority for native, packaged and cross-platform claims; a local
  pass never promotes to `ACCEPTED`.
- Record evidence under `docs/evidence/` with exact commands, timings, hashes,
  and the limits of the run; keep claim records under `docs/evidence/claims/`.

## Handing a cluster to an agent

Point the agent at this file and one cluster, for example:

> Work in `/home/huou07/Projects/OR`. Read `AGENTS.md`, then
> `docs/prompts/OR-LOCAL-FINISH.md` — execute **C2**. Follow its write scope,
> acceptance and stop conditions exactly, run `bash scripts/check-repo.sh` before
> each commit, and report what you verified, in which evidence class, and what
> remains `UNVERIFIED`/`BLOCKED`.
