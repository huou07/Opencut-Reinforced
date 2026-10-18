# OR-BUILD — build Opencut Reinforced to the owner's product

This is the single execution prompt for OR. It selects and bounds the whole
program: complete basic editing, code-to-illustration video, automatic
subtitles, noise filtering, and the local-plus-hosted toolchain that carries
them to a release. Work it end to end in one sustained effort. Do not ask for a
new prompt per phase; the phases below are the order of work, not separate
handoffs.

Owner intent recorded 2026-10-18. Authority for what an agent may do without
asking is [AGENT-AUTHORITY.md](../AGENT-AUTHORITY.md); the permanent rules are
[AGENT_POLICY.md](../AGENT_POLICY.md). This file selects work; the product
backlog stays [PRODUCT_ROADMAP.md](../PRODUCT_ROADMAP.md).

## Owner intent

OR must be a cross-platform video editor that is fast, small in surface and easy
to use, with a good and consistent UI/UX across the desktop platforms that share
a machine class — macOS, Linux and Windows — and an adapted equivalent on
Android phones. In order:

1. the basic editing capabilities must all be present and pleasant to use;
2. AI illustration video from code ("turn code into video") must exist;
3. automatic subtitles must work;
4. noise filtering must work;
5. the Debian workstation builds and tests the Linux desktop and the Android
   emulator build quickly, and every release still goes through GitHub CI.

## Verified starting state

Re-verify each row before relying on it; every row names the command that
rechecks it.

| Fact | Value |
|---|---|
| Repository | `/home/huou07/Projects/OR`, branch `feat/semantics-journey-guards` |
| Gates | `bash scripts/check-repo.sh` PASS; `bash scripts/acceptance.sh` adds the Rust and Flutter lanes, `gitleaks` and the performance budget |
| Local lanes | `bash scripts/run_local_lane.sh linux` exists with `--packaged`, `--steps`, `--report`. Exit codes: 0 pass, 1 fail, 2 bad invocation, 3 host-BLOCKED, 4 UNVERIFIED or dirty tree |
| Android lane | **Not implemented.** `run_local_lane.sh` prints "linux — the only lane implemented today". `scripts/start-android-emulator.sh`, `run-android-preview-check.sh`, `stop-android-emulator.sh` exist and are CI-proven |
| UI harness | **Not implemented.** No `scripts/or_ui.py`, no `scripts/ui_scenarios/` |
| Performance budget | `scripts/bench_quality.py` has two rows only: `cli-cold-start` and `ffmpeg-export`. Interactive frame time and seek latency are UNVERIFIED |
| Android SDK | `~/Android/Sdk` with platform-tools, emulator, NDK 26.3/27.2/28.2, `system-images;android-36;google_apis;x86_64`; AVD `an3-api36`; `/dev/kvm` usable |
| FFmpeg 8 | `~/.local/ffmpeg8/{ffmpeg,ffprobe}` present, so the media lanes in `acceptance.sh` run |
| Host | Dell OptiPlex Micro 7010, i5-13500T (20 threads), 15.3 GiB RAM, bare metal |
| Host headroom | **Contended.** `MemAvailable` measured 5.4 GiB. The Gradle 8.14.3 daemon holding ~2.25 GiB belongs to the sibling project VCE (`~/Projects/VCE/native-offline/src-tauri/gen/android`), not to OR. The Linux preflight refuses below 6 GiB and the Android lane needs ~7.7 GiB |
| Desktop packages | Built and published by `.github/workflows/developer-preview.yml`; acceptance authority is `.github/workflows/platform-verification.yml` |

Measured capability state, read from source at `e9b21a8` plus the two
organization commits:

- **Basic editing is implemented**: 23 commands and 8 queries dispatch through
  the shared application path (`crates/or_core/src/application.rs:64`), with 109
  core unit tests and 92 Flutter widget tests. Absent: overwrite edit mode;
  ripple trim; slip, slide and roll; track reordering; clip speed/rate change;
  keyframes; clip linking and grouping. Zoom, scroll and snap guides are
  presentation state; snapping is the read-only `timeline.snap` query.
- **Code-to-illustration video is absent**, not half-built. Only primitives
  exist: deterministic bundled-font rasterisation
  (`crates/or_render/src/text.rs:18-39`), first-class text and caption clips
  (`crates/or_core/src/timeline.rs:181`), and the shared wgpu layer path
  (`crates/or_render/src/lib.rs:602`). `crates/or_ai/src/protocol.rs:26`
  declares `AiTask::GenerateMotionScene` with no producer or consumer. ADR 0007
  is intent, not implementation.
- **Automatic subtitles work on desktop** (whisper.cpp sidecar, hashed model,
  provenance-bound proposal, review/edit/apply, SRT/WebVTT) and **Android is
  unsupported** because `model_store_root()` has no Android arm
  (`crates/or_app_bridge/src/transcription.rs:798-826`), so
  `TranscriptionService::new()` returns `None`. There is no transcription CLI
  subcommand.
- **Noise filtering is absent.** Audio processing is only gain, pan and linear
  fades (`crates/or_audio/src/lib.rs:428-436`). The pinned FFmpeg build is
  `--disable-avfilter` with only `scale,setpts,format,aformat,aresample,
  showwavespic` enabled
  (`.github/workflows/platform-verification.yml:33,58-63`), so denoise cannot be
  delegated to it without a deliberate codec-build and licensing change.

## What to build

### Phase 0 — make the local loop real

The program below cannot be iterated quickly without these. Land them first.

1. `bash scripts/run_local_lane.sh android`: boot a dedicated AVD with KVM,
   build the debug APK with the x86_64 Android FFmpeg staged reproducibly,
   install it, drive the bridge-diagnostics, preview-surface and SAF journeys,
   stop the guest, and write one machine-readable report beside
   `apps/or_app/build/local-lanes/linux.json`. It must not depend on
   `GITHUB_WORKSPACE` or `RUNNER_TEMP`, must build the APK before booting, must
   check host headroom and refuse below its floor, must use a dedicated
   AVD/serial, and must stop what it started. Reconstruct the APK staging
   recipe from the `android` job in `.github/workflows/platform-verification.yml`
   (NDK `28.2.13676358`, API 26, x86_64 locally) and add
   `android/app/src/main/jniLibs/` to `.gitignore` with the reason.
2. `scripts/or_ui.py`: one harness that lets an agent observe and drive the real
   app — `screens`, `tree`, `shot`, `act`, `run <scenario.json>` — on Linux
   (Xvfb plus the real Rust host) and on the Android emulator, emitting a
   step-by-step JSON transcript with the action, selector, result, elapsed ms,
   screenshot path and semantics snapshot path. Act by semantics label, role or
   key; never by screen coordinates; never inject state to force a pass.
3. Grow `scripts/bench_quality.py` into a measured budget on this named host:
   keep `cli-cold-start` and `ffmpeg-export`, and add `command-latency`
   (p50/p95 for create/import/add/trim/split/undo/seek through the CLI on a
   ~200-clip project), `project-open`/`project-save`, `render-frame-time`
   (offscreen wgpu through `or_render` on the real Intel UHD 770), and
   `export-throughput`. `acceptance.sh` enforces the budgets.

### Phase 1 — complete basic editing

Close the enumerated gaps, highest user value first, with desktop and Android
sharing one command and state model:

- overwrite edit mode as a command with an explicit mode, never a UI-only gesture;
- ripple trim, then slip, slide and roll;
- track reordering;
- clip speed/rate change with exact rational time preserved;
- clip linking and grouping, and their interaction with move, trim and delete;
- keyframes only after the above are stable.

For each: put behavior in the Rust domain first, add the narrow regression, then
expose it through the desktop UI and the compact/Android presentation using
existing shell slots. Keep the semantics tree honest so Phase 0's harness can
drive every control by label.

### Phase 2 — automatic subtitles on every target

1. Give `model_store_root()` a real Android arm (`crates/or_app_bridge` or the
   app-private files directory) and prove the sidecar runs there, or record
   Android transcription `UNSUPPORTED` with the measured reason instead of a
   silent `ProviderUnavailable`.
2. Add word-level timings and caption segmentation.
3. Add translation as a reviewable proposal, never an authoritative rewrite.
4. Add CLI parity: generate, inspect and export captions through the semantic
   CLI, since agents and the CLI are first-class clients.
5. Keep every result editable and applied through a revision-checked command.

### Phase 3 — code-to-illustration video

Track A is the primary path and needs no model download to render:

1. Define the versioned, non-executable `.ormotion.json` MotionScene subset
   needed for a code explainer — code block, highlight ranges, theme, callouts,
   arrows, captions, timing.
2. Ingest a code artifact, tokenise it and produce highlight runs. Code and theme
   are untrusted input: bound the parser, never execute the sample, and record
   provenance.
3. Add a scene evaluator and materialisation that registers its output through
   normal media/timeline commands, reusing the existing text rasteriser and the
   shared RenderSnapshot/wgpu path. Do not add a second text engine.
4. Render deterministically with zero provider calls. Materialisation is a
   command with a revision precondition, and its output is ordinary project
   media.
5. Only then wire an optional AI provider that generates the scene from code,
   under the ADR 0005 boundary, with the result as a reviewable proposal.
6. Track B — an external generative-video provider — is a later, costed,
   licence-reviewed addition. Do not start it in this program.

A new ADR is required before the canonical MotionScene contract becomes
load-bearing: see the ADR triggers in
[DEVELOPMENT_WORKFLOW.md](../DEVELOPMENT_WORKFLOW.md).

### Phase 4 — noise filtering and audio cleanup

1. Decide and record the route once: bounded Rust DSP in `or_audio`/`or_media`,
   or a deliberate FFmpeg configuration change enabling the needed filters, with
   the licence and packaging consequences re-verified. Do not assume the shipped
   build can do it.
2. Implement denoise as a typed, bounded, per-clip audio setting that runs in
   both preview and export through the same code path, never on the realtime
   callback with unbounded work.
3. Add loudness normalization and a limiter, then measure.

### Phase 5 — release

Hosted CI releases, never a local package: `platform-verification` must pass for
the exact commit, then `developer-preview` publishes. Follow
[RELEASE.md](../RELEASE.md) and skill `or-release`. A stable release remains an
owner-owned milestone with signing, notarisation and a reviewed platform matrix.

## Working method

One writer per repository. Check headroom before starting a lane, and serialise
against the sibling project that currently holds the Gradle daemon:

```sh
awk '/MemAvailable/{printf "%.2f GiB\n", $2/1048576}' /proc/meminfo
pgrep -af GradleDaemon            # attribute it before touching it; VCE's is not yours
bash scripts/check-repo.sh        # before every commit
bash scripts/acceptance.sh        # at each phase boundary
bash scripts/run_local_lane.sh linux
bash scripts/run_local_lane.sh android
```

If `MemAvailable` is below the lane's floor because another project holds the
RAM, the honest result is `BLOCKED` with the probe output — never a lowered
floor, a weaker check, or a fabricated pass. Do the next phase that does not
need the runtime lane, and return to the blocked one when the host is free.

Per coherent change: implement, add the regression, update the owning
source-of-truth document, commit with a Conventional Commit subject, and report
what you verified and what remains unverified. `IMPLEMENTED` and `ACCEPTED` are
different claims; only hosted evidence makes the second one.

## Boundaries

- Only the shared command/application path mutates canonical project state. No
  second editable project in Dart, CLI code, a worker or an agent session.
- Canonical motion is declarative and non-executable. Rendering makes no
  provider call. Arbitrary web code belongs only to the isolated WebMotion
  sidecar, which is out of scope here.
- AI output is a reviewable proposal applied through a revision-checked command.
  Credentials stay in OS secure storage and never reach the CLI, agents, logs,
  fixtures or screenshots.
- `scripts/check_or_core_deps.py` enforces the canonical-state dependency
  boundary; a failure needs a decision, never a widened pattern list.
- Memory, render, audio and export work must stay bounded, cancellable and
  honest about dropped work. Do not route per-frame work through the project
  mutation path.
- Do not regress a fixed behavior. Preserve unrelated work, never amend shared
  history, never force-push, never weaken a guard to get a green result.
- Models are not bundled by default; a download requires confirmation, progress
  and cancellation, and its licence is checked independently.

## Non-goals

iOS and web. Plugin or OpenFX distribution, a community registry, executable
WebMotion, accounts or hosted services. Signing and notarisation for a stable
release. A media-engine replacement such as MLT/GES unless a measured packaged
gap justifies it. Generative video (Track B) inside this program. A second
roadmap or a revival of `docs/archive/execution/`.

## Authority boundary

Nothing here authorizes: host `sudo` or account changes, reading or copying
owner credentials, publishing a stable release, force-pushing `main`, rewriting
tags, or running two writers in this tree. A local preflight or emulator result
is developer evidence and never `ACCEPTED`. Hosted `platform-verification`
decides native, packaged and cross-platform acceptance.

## Completion criteria

- Phase 0: one command runs each local lane and writes one report each; the
  Linux lane reports its real per-step verdict; the Android lane names the AVD,
  serial, GPU mode, probes, APK hash, each test result and the guest/host health
  samples, or reports `BLOCKED` with the measured reason; the UI harness runs one
  scenario end to end (Home → Projects → open/create → import a synthetic
  fixture → edit → undo → seek → save → reopen → export setup → cancel) on both
  targets, with a deliberately wrong selector failing loudly; every benchmark row
  carries a measured number and trips on an injected regression.
- Phase 1: each enumerated gap is a command with a regression and a desktop and
  compact presentation, or is recorded not wanted with the reason.
- Phase 2: transcription either runs on Android with evidence, or is recorded
  `UNSUPPORTED` with the measured reason; word timings, translation and CLI
  parity have regressions.
- Phase 3: a code artifact produces a materialised illustrated video whose
  frames are reproducible from the same scene, with no provider call during
  render, and the scene contract has its ADR.
- Phase 4: denoise is a typed per-clip setting applied identically in preview and
  export, with a bounded-cost test and a measured before/after.
- Phase 5: `platform-verification` passes for the exact commit and
  `developer-preview` publishes, or the release gap is reported with the exact
  unmet precondition.

## Durable state and recovery

Record progress in [PRODUCT_ROADMAP.md](../PRODUCT_ROADMAP.md) with exact source
and evidence class, complete claims in
[evidence/claims/README.md](../evidence/claims/README.md), host and toolchain
facts in [AGENT-AUTHORITY.md](../AGENT-AUTHORITY.md), commands and limits in
[TESTING.md](../TESTING.md) and [TOOLING.md](../TOOLING.md). A new session must
be able to continue from the repository alone: keep the tree clean at every phase
boundary, and leave the next action unambiguous in the roadmap row you touched.

## Report

Per phase: what changed, files, exact commands and results, the evidence class,
the commit hash and subject, every lane not run, and what remains
`BLOCKED`/`UNVERIFIED`. Never report a check as passing unless it ran and passed.
