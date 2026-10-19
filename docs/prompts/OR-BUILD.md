# OR-BUILD — build Opencut Reinforced to the owner's product

Last reviewed: 2026-10-18

This is the single execution prompt for OR. It selects and bounds the whole
program: complete basic editing, code-to-illustration video, automatic
subtitles, noise filtering, the UI/UX and measured-performance bar every one of
them must clear, and the local-plus-hosted toolchain that carries them to a
release. Work it end to end in one sustained effort. Do not ask for a new prompt
per phase; the phases below are the order of work, not separate handoffs.

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
5. every one of the above must be **good to use** on desktop and Android — the
   same concepts, terminology, layout intent and command semantics on macOS,
   Linux and Windows, with Android an adapted presentation of the same model,
   never a fork;
6. **interaction performance must be measured and gated, not asserted.** "Fast"
   is an acceptance criterion with a number attached, on a named host, with the
   limit of that measurement stated;
7. the Debian workstation builds and tests the Linux desktop and the Android
   emulator build quickly, and every release still goes through GitHub CI.

## Verified starting state

Re-verify each row before relying on it; every row names the command that
rechecks it.

| Fact | Value |
|---|---|
| Repository | `/home/huou07/Projects/OR`, branch `feat/semantics-journey-guards`, HEAD `1ff53bb` |
| Gates | `bash scripts/check-repo.sh` PASS at HEAD; `bash scripts/acceptance.sh` adds the Rust and Flutter lanes, `gitleaks` and the performance budget. The Hub recorded `verify` and `acceptance` PASS-CURRENT and `release-check` VERIFIED at `a51b615`; re-record after each push |
| Local lanes | `bash scripts/run_local_lane.sh linux` with `--packaged`, `--steps`, `--report`. Exit codes: 0 pass, 1 fail, 2 bad invocation, 3 host-BLOCKED, 4 UNVERIFIED or dirty tree |
| Android lane | **Implemented; the guest drive has never produced a result.** `run_local_lane.sh android` delegates to `scripts/run_android_lane.sh` with steps `apk,emulator,preview,saf`. Inputs: `scripts/stage_android_ffmpeg.sh` (pinned x86_64 prefix plus the guest link probe) and `scripts/build_android_case_apks.sh` (the `android/.ffmpeg-install` link the build hook requires, one APK per case). `scripts/test_android_lane.py` covers its rules hermetically |
| Android lane blocker | `preflight` now **PASSes** with the Flutter SDK grants below. The `apk` step then reaches Gradle 9.3.1 `assembleDebug` and **hangs**: measured 2026-10-18, no file written for over five minutes, 297 of 302 daemon threads parked in `futex_do_wait`, idle CPU, zero swap traffic, and Google/Cloudflare sockets in `CLOSE-WAIT`. A stalled dependency fetch is the leading, **unconfirmed** hypothesis |
| Flutter SDK grants | Owner-authorized 2026-10-18: write access for the worker UID on exactly two paths, `packages/flutter_tools/gradle` and `packages/integration_test/android`. Commands, scope and reversal are in [AGENT-AUTHORITY.md](../AGENT-AUTHORITY.md) |
| UI harness | **Not implemented.** No `scripts/or_ui.py`, no `scripts/ui_scenarios/` |
| Performance budget | `scripts/bench_quality.py` has two rows only: `cli-cold-start` and `ffmpeg-export`. Interactive frame time, seek latency and command latency are UNVERIFIED |
| UI/UX authorities | [DESIGN.md](../../DESIGN.md), [UX_ACCEPTANCE.md](../UX_ACCEPTANCE.md) and the accessibility and performance rows of [QUALITY.md](../QUALITY.md); skill `or-ui-ux` binds them |
| Android SDK | `~/Android/Sdk` with platform-tools, emulator, NDK 26.3/27.2/28.2, `system-images;android-36;google_apis;x86_64`; AVD `an3-api36`; `/dev/kvm` usable |
| FFmpeg 8 | `~/.local/ffmpeg8/{ffmpeg,ffprobe}` present, so the media lanes in `acceptance.sh` run |
| Host | Dell OptiPlex Micro 7010, i5-13500T (20 threads), 15.3 GiB RAM, bare metal |
| Host headroom | **Time-varying; measure it.** Observed range 5.4–7.3 GiB `MemAvailable`. A Gradle 8.14.3 daemon holding ~2.25 GiB can belong to the sibling project VCE, and the Android lane's own Gradle 9.3.1 daemon (`-Xmx8G`) stays resident after its `apk` step until it is reaped. Attribute every java daemon before touching it |
| Desktop packages | Built and published by `.github/workflows/developer-preview.yml`; acceptance authority is `.github/workflows/platform-verification.yml` |

Measured capability state at `1ff53bb`:

- **Basic editing is implemented**: 23 commands and 8 queries dispatch through
  the shared application path (`crates/or_core/src/application.rs:64`), with 109
  core unit tests and 92 Flutter widget tests. Absent: overwrite edit mode;
  ripple trim; slip, slide and roll; track reordering; clip speed/rate change;
  keyframes; clip linking and grouping. Zoom, scroll and snap guides are
  presentation state; snapping is the read-only `timeline.snap` query.
- **The UI/UX evidence base is thin**: 92 widget tests cover semantics and
  behaviour on the screens they touch, and the compact 320x480 and 320x440
  regressions are permanent guards, but there is no screen inventory, no
  on-demand semantics dump, no scenario runner, and no measured interaction
  latency. UI claims therefore rest on widget tests plus two fixed preflight
  screenshots.
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
  fades (`crates/or_audio/src/lib.rs:428-436`). The pinned shared library build
  disables avfilter (`.github/workflows/platform-verification.yml:33`,
  `FFMPEG_COMMON_CONFIGURE_ARGS`); only the desktop helper executables re-enable
  it with six filters (`:51,58-63`), so the library the editor links cannot
  denoise as shipped. **Owner decision, 2026-10-18: the route is a deliberate
  FFmpeg build-configuration change that enables the filters this needs, with the
  licence, packaging and per-platform consequences re-verified before it is
  enabled anywhere.** Bounded Rust DSP in `or_audio`/`or_media` is not the chosen
  route.

## Acceptance has four dimensions, not one

Every capability is accepted on behaviour **and** interface **and** performance
**and** evidence. A passing build, a green widget test or a screenshot each
answer a different question, and none of them is the whole answer.

| Dimension | What must hold |
|---|---|
| Behaviour | the real journey works through the shared command path on desktop and Android, with its loading, empty, partial, success, failure, undo, cancel and recovery states covered or explicitly named as not covered |
| Interface | the thresholds below hold, the semantics tree is honest, and the compact and desktop presentations share one model |
| Performance | the interaction and render paths the change touches are measured against the budget in [QUALITY.md](../QUALITY.md), with p50/p95 and the measurement's limits stated |
| Evidence | each claim carries the class it actually established — source, semantic, behavioural, visual, native — and never promotes one into another |

The enforceable interface thresholds, from
[UX_ACCEPTANCE.md](../UX_ACCEPTANCE.md) and [QUALITY.md](../QUALITY.md):

| Requirement | Threshold |
|---|---|
| Text contrast | OR colour text tokens hold at least 4.5:1 against `background` |
| Type size | no font size below 11 px |
| Hit area | at least 24 px on desktop, 44 px in the compact layout |
| Keyboard | every pointer action has a documented keyboard alternative; focus is visible and restorable |
| Compact layout | 320x480 and 320x440 stay clean; taller compact viewports such as 390x844 must not regress |
| Overflow | no clipping or overflow at the named viewports |

Parity rule: macOS, Linux and Windows present one product. Pixel-identical
output is not required and must not be claimed; identical concepts, terminology
and command semantics are required. Android adapts the presentation — sheets,
touch targets, layout — and never the model.

Performance rule: playback-critical decode, audio and render work outranks
opportunistic thumbnail, waveform, proxy and analysis work, and a test must show
an opportunistic load cannot starve or delay it. Xvfb, llvmpipe, SwiftShader and
emulator numbers are software-path evidence and can never become a device or GPU
performance claim.

Method, evidence classes, thresholds and stop conditions are in skill
`or-ui-ux`. Load it for any UI work.

## What to build

### Phase 0 — make the local loop real

The phases below cannot be iterated or accepted without these. Land them first.

1. **Finish the Android lane.** The entry point, the preconditions, the FFmpeg
   stager, the case-APK builder and the hermetic checks have landed
   (`scripts/run_android_lane.sh`, `scripts/stage_android_ffmpeg.sh`,
   `scripts/build_android_case_apks.sh`, `scripts/run_android_ffmpeg_probe.sh`,
   `scripts/test_android_lane.py`). What remains:
   - find out why `assembleDebug` hangs and fix or bound it; start from
     `flutter build apk --debug --target-platform android-x64 -v` on one case and
     check whether the dependency fetch is the cause. Record the finding either
     way.
   - derive the lane's memory floor from the guest size plus a margin instead of
     a flat 7 GiB, and **stop the Gradle/Kotlin daemons the lane itself started**
     before the `emulator` step. Today the 6144 MB guest default and the 7 GiB
     floor cannot both hold on this host.
   - then drive the guest: boot a dedicated AVD with KVM, install the APK, run
     the bridge-diagnostics, preview-surface and SAF journeys, stop what you
     started, and write `apps/or_app/build/local-lanes/android.json` naming the
     AVD, serial, GPU mode, probes, APK hash, every test result and the
     guest/host health samples.
2. **`scripts/or_ui.py`** — the harness that makes UI claims observable:
   `screens`, `tree`, `shot`, `act`, `run <scenario.json>` on Linux (Xvfb plus
   the real Rust host) and on the Android emulator, emitting a step-by-step JSON
   transcript with the action, selector, result, elapsed ms, screenshot path and
   semantics snapshot path. Act by semantics label, role or key; never by screen
   coordinates; never inject state to force a pass. It is also the cheapest
   instrument for the interface dimension below.
3. **Grow `scripts/bench_quality.py`** into a measured budget on this named host:
   keep `cli-cold-start` and `ffmpeg-export`, and add `command-latency` (p50/p95
   for create/import/add/trim/split/undo/seek through the CLI on a ~200-clip
   project), `project-open`/`project-save`, `render-frame-time` (offscreen wgpu
   through `or_render` on the real Intel UHD 770), `export-throughput`, and
   `queue-priority` (playback-critical p95 while opportunistic work saturates the
   pool). `acceptance.sh` enforces the budgets, and a deliberately injected
   regression must trip them.
4. **Extend the UI guards** as the phases add surfaces: `UX_ACCEPTANCE.md` gains
   the permanent regression for each UI defect fixed, and the compact-viewport
   and accessibility guards keep passing.

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
existing shell slots. Keep the semantics tree honest so the Phase 0 harness can
drive every control by label. Then clear the interface and performance dimensions
for that surface: thresholds hold, semantics asserted, the journey exercised on
both targets, and the latency of the new command measured and added to the
budget.

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
6. Cover the caption review surface as a UI journey: its loading, running,
   failed and empty states, progressive status feedback on a long transcription,
   cancel behavior, and the review/edit/apply path driven by semantics. Measure
   the transcription start and apply latency you add.

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

This is a visual feature, so the visual and render-performance dimensions carry
real weight: keep a small representative screenshot set at a named viewport and
theme, inspect hierarchy, clipping, contrast and code legibility, and add a
`render-frame-time` row for the scene so "reproducible" also means "measured".

A new ADR is required before the canonical MotionScene contract becomes
load-bearing: see the ADR triggers in
[DEVELOPMENT_WORKFLOW.md](../DEVELOPMENT_WORKFLOW.md).

### Phase 4 — noise filtering and audio cleanup

1. Route decided by the owner on 2026-10-18: a deliberate FFmpeg
   build-configuration change enabling the filters this capability needs, with
   the licence, packaging and per-platform consequences re-verified before it is
   enabled anywhere. Bounded Rust DSP in `or_audio`/`or_media` is not the route.
   Do not assume the shipped build can do it: the pinned shared library disables
   avfilter today.
2. Implement denoise as a typed, bounded, per-clip audio setting that runs in
   both preview and export through the same code path, never on the realtime
   callback with unbounded work.
3. Add loudness normalization and a limiter, then measure.
4. Give the setting a real control: an explicit off state, a bounded parameter
   range, the cost stated before it is applied, and an audible before/after the
   user can judge. Add its latency and CPU cost to the budget.

### Phase 5 — release

Hosted CI releases, never a local package: `platform-verification` must pass for
the exact commit, then `developer-preview` publishes. Follow
[RELEASE.md](../RELEASE.md) and skill `or-release`. The release candidate must
carry the four acceptance dimensions for the capabilities it claims: behaviour,
interface, performance and evidence. A stable release remains an owner-owned
milestone with signing, notarisation and a reviewed platform matrix.

## Working method

One writer per repository. Check headroom before starting a lane, and serialise
against any sibling project that holds a java daemon:

```sh
awk '/MemAvailable/{printf "%.2f GiB\n", $2/1048576}' /proc/meminfo
ps -eo pid,etimes,rss,args | grep -E '[G]radleDaemon|[K]otlinCompileDaemon'
readlink /proc/<pid>/cwd        # attribute it before touching it; VCE's is not yours
bash scripts/check-repo.sh      # before every commit
bash scripts/acceptance.sh      # at each phase boundary
bash scripts/run_local_lane.sh linux
bash scripts/run_local_lane.sh android
python3 scripts/or_ui.py run scripts/ui_scenarios/<journey>.json
python3 scripts/bench_quality.py --json
```

If `MemAvailable` is below the lane's floor because another project holds the
RAM, the honest result is `BLOCKED` with the probe output — never a lowered
floor, a weaker check, or a fabricated pass. Do the next phase that does not
need the runtime lane, and return to the blocked one when the host is free.

Per coherent change: implement, add the regression, add or extend the UI guard,
measure the interaction you touched, update the owning source-of-truth document,
commit with a Conventional Commit subject, and report what you verified and what
remains unverified. `IMPLEMENTED` and `ACCEPTED` are different claims; only
hosted evidence makes the second one.

## Boundaries

- Only the shared command/application path mutates canonical project state. No
  second editable project in Dart, CLI code, a worker or an agent session.
- Presentation state stays presentation-local: navigation, panel, selection and
  temporary input. No new design system, no marketing surface, no new permanent
  navigation region without justification.
- Never weaken or delete a UX guard, an accessibility threshold or a security
  boundary to obtain a pass. A regression fix is incomplete until a guard covers
  it.
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
  history, never force-push.
- Models are not bundled by default; a download requires confirmation, progress
  and cancellation, and its licence is checked independently.

## Non-goals

iOS and web. Plugin or OpenFX distribution, a community registry, executable
WebMotion, accounts or hosted services. Signing and notarisation for a stable
release. A media-engine replacement such as MLT/GES unless a measured packaged
gap justifies it. Generative video (Track B) inside this program. A second
roadmap or a revival of `docs/archive/execution/`. A blanket screenshot suite, a
cross-platform pixel-identity target, or a new design system.

## Authority boundary

Nothing here authorizes: host `sudo` beyond the two dated Flutter SDK ACL grants
already recorded in [AGENT-AUTHORITY.md](../AGENT-AUTHORITY.md), host account or
group changes, reading or copying owner credentials, publishing a stable release,
force-pushing `main`, rewriting tags, or running two writers in this tree. A local
preflight, emulator or screenshot result is developer evidence and never
`ACCEPTED`. Hosted `platform-verification` decides native, packaged and
cross-platform acceptance.

## Completion criteria

Each phase is complete only when its behaviour, interface, performance and
evidence dimensions all hold, or the gap is recorded as `BLOCKED`/`UNVERIFIED`
with the measurement that shows why.

- **Phase 0:** one command runs each local lane and writes one report each; the
  Linux lane reports its real per-step verdict; the Android lane either produces
  a guest result naming the AVD, serial, GPU mode, probes, APK hash, each test
  result and the guest/host health samples, or reports `BLOCKED` with the measured
  reason and the hang is diagnosed either way; `scripts/or_ui.py` runs one
  scenario end to end (Home → Projects → open/create → import a synthetic fixture
  → edit → undo → seek → save → reopen → export setup → cancel) on both targets,
  with a deliberately wrong selector failing loudly and a keyboard path asserted;
  every benchmark row carries a measured number, states its limits, and trips on
  an injected regression.
- **Phase 1:** each enumerated gap is a command with a regression and a desktop
  and compact presentation that clears the threshold table and exposes honest
  semantics, or is recorded not wanted with the reason. Each new command has a
  measured p50/p95 in the budget.
- **Phase 2:** transcription either runs on Android with evidence, or is recorded
  `UNSUPPORTED` with the measured reason; word timings, translation and CLI
  parity have regressions; the review surface covers its loading, running, failed
  and empty states with asserted semantics and a measured apply latency.
- **Phase 3:** a code artifact produces a materialised illustrated video whose
  frames are reproducible from the same scene, with no provider call during
  render, a representative screenshot set, a measured render frame time, and an
  ADR for the scene contract.
- **Phase 4:** denoise is a typed per-clip setting applied identically in preview
  and export, with a real control, a bounded-cost test, and a measured before and
  after.
- **Phase 5:** `platform-verification` passes for the exact commit and
  `developer-preview` publishes, or the release gap is reported with the exact
  unmet precondition. The published notes state the interface and performance
  evidence for the capabilities claimed.

## Durable state and recovery

Record progress in [PRODUCT_ROADMAP.md](../PRODUCT_ROADMAP.md) with exact source
and evidence class; complete claims in
[evidence/claims/README.md](../evidence/claims/README.md); host and toolchain
facts in [AGENT-AUTHORITY.md](../AGENT-AUTHORITY.md); commands, lanes and limits
in [TESTING.md](../TESTING.md) and [TOOLING.md](../TOOLING.md); permanent UI
guards in [UX_ACCEPTANCE.md](../UX_ACCEPTANCE.md); design rules in
[DESIGN.md](../../DESIGN.md); the acceptance bar and its gaps in
[QUALITY.md](../QUALITY.md). A new session must be able to continue from the
repository alone: keep the tree clean at every phase boundary, and leave the next
action unambiguous in the roadmap row you touched.

## Report

Per phase: what changed, files, exact commands and results, each acceptance
dimension with the evidence class it actually established, the performance
numbers with their limits, the UI guards added, the commit hash and subject,
every lane not run, and what remains `BLOCKED`/`UNVERIFIED`. Never report a check
as passing unless it ran and passed.
