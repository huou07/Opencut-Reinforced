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
| Performance | one bounded local budget of sixteen measured rows: process, command, project-file, render-frame and queue-priority (see the table below), on the named Debian OptiPlex-class host | `python3 scripts/bench_quality.py`, run by `bash scripts/acceptance.sh` | acceptance | enforced locally on this host class; a release-build, physical-device or hosted-runner number is **not** claimed |
| Reliability | project save/reopen, undo/redo, autosave and recovery in the named journeys | widget + preflight journeys | acceptance / hosted | partially verified |
| Accessibility / UX | 4.5:1 contrast, >= 11 px type, 24/44 px hit areas, keyboard alternatives | headless guards in `docs/UX_ACCEPTANCE.md` | widget tests | verified for the covered screens |
| Security | no secrets in tracked files or history; dependency boundary enforced | `gitleaks detect` (acceptance, and pinned in `repo-hygiene.yml`), `check_or_core_deps.py` | acceptance / verify / hosted | local scan runs when gitleaks is installed; `check-repo.sh` alone still does not scan |
| Compatibility | ubuntu/macOS/Windows/Android hosted jobs | CI matrix | hosted CI | hosted authority |
| Maintainability | index complete, links resolve, every current-claim doc dated, claims exact-SHA and resolvable in Git | doc and claim gates | verify | verified |
| Cost | resource use of the bounded preflight | preflight headroom checks and per-stage budgets | acceptance (manual) | partially verified |

## Performance: one bounded budget, and its limits

OR enforces one local performance budget, measured only on the named host class
below. It exists so a catastrophic regression fails locally instead of waiting
for a user to notice, and so "fast" is a number with a provenance rather than an
assertion. Every row below was measured on 2026-10-18 on an idle host; the
margin column says what the threshold was set from.

| Row | What it times | Baseline (2026-10-18, idle) | Budget |
|---|---|---|---|
| `cli-cold-start` | `or version --json`, fresh process, N=15 | 1.1 ms best / 2.0 ms median | best <= 200 ms |
| `ffmpeg-export` | the ignored `software_export` WebM fixture (64x48, 10 frames, VP9/Opus), N=5 | 35.1 ms best / 42.1 ms median | best <= 1500 ms |
| `export-throughput` | frames per second through that same lane | 285 frames/s | >= 60 frames/s |
| `command.create` | project create on a fresh path, 40 iterations | 0.052 ms p50 / 0.089 ms p95 | p95 <= 5 ms |
| `command.import` | `media.add` after a real `ffprobe` of a 1 s clip | 14.3 ms p50 / 20.3 ms p95 | p95 <= 60 ms |
| `command.add_track` | `timeline.track.add` on a 200-clip project | 0.013 / 0.016 ms | p95 <= 2 ms |
| `command.add_clip` | `timeline.clip.insert`, 200 of them | 0.027 / 0.038 ms | p95 <= 2 ms |
| `command.trim` | `timeline.clip.trim` | 0.139 / 0.195 ms | p95 <= 5 ms |
| `command.split` | `timeline.clip.split` | 0.175 / 0.203 ms | p95 <= 5 ms |
| `command.undo` | `history.undo` | 0.150 / 0.165 ms | p95 <= 5 ms |
| `command.seek_summary` | `project.summary`, the read a seek performs | 0.000 / 0.000 ms | p95 <= 1 ms |
| `command.seek_clip_page` | `timeline.clips` page of 100 on a 200-clip track | 0.006 / 0.007 ms | p95 <= 2 ms |
| `project-open` | loading the 200-clip project file | 8.8 / 9.0 ms | p95 <= 60 ms |
| `project-save` | atomic publish of that project file | 15.0 / 15.2 ms | p95 <= 80 ms |
| `render-frame-time` | a 640x360 offscreen `or_render` frame **plus readback**, on the host's real GPU | 1.96 ms p50 / 2.18 ms p95 over 60 frames | p95 <= 16.7 ms |
| `queue-priority` | that same frame while 40 CPU-bound opportunistic jobs run on 20 workers | idle p95 2.77 ms -> loaded p95 6.30 ms, ratio 2.27 | loaded p95 <= 33.3 ms and ratio <= 6.0 |

`python3 scripts/bench_quality.py` measures all of them and
`bash scripts/acceptance.sh` runs it, so a breach fails the admission gate. The
process rows are enforced on the **best of N** so a host busy with sibling builds
does not fail spuriously; the in-process rows are enforced on the **p95**, which
is what a user feels. `OR_BENCH_ONLY=<rows>` selects rows. A host that cannot
produce an honest number is reported `UNVERIFIED` with the exact command that
would settle it, never a fabricated threshold.

The render rows print the adapter, driver, backend and adapter class. A software
rasteriser (llvmpipe, SwiftShader, Lavapipe) is reported `UNVERIFIED` as
software-path evidence and can never become a GPU claim; the number above is the
Intel UHD 770 through Vulkan on this workstation. The render time **includes the
offscreen readback**, so it is a deliberately heavier upper bound than
presentation, and it is a debug build: it guards OR's own code, it is not a claim
about a release build's speed.

### Measured interaction latency (software path)

The budget above measures the canonical command path and the offscreen render
path. The Flutter presentation layer is measured separately by
`python3 scripts/or_ui.py run scripts/ui_scenarios/interaction-latency.json`,
which times acting steps with their fixed settle disabled and lets the following
wait step observe the visible result, so the number is latency to the visible
state rather than the harness's settling budget (at most one 16 ms poll interval
of uncertainty).

Measured 2026-10-18 on this host, 1280x800, software GL under Xvfb, ten runs each:

Motion materialisation is measured but **not yet a budget row**: 252 frames at 640x360
took 20.1 s in a debug build on this host — about 12.5 frames per second, or 80 ms per
frame including the FFV1 encode — for a 9.2 MB file (2026-10-20), measured through a CLI
path that has since been reverted because the CLI must not link FFmpeg. No release
claim follows from a debug number, and the row is owed once the application-layer client
exists: it will be measured by the materialisation command that replaces the reverted
one, at a named frame rate and canvas. The rasterisation row below covers the CPU half
of that cost separately.

Motion rasterisation has a budget row: `motion-rasterize-frame` evaluates and
rasterises a forty-node 1920x1080 MotionScene frame and encodes it as a PNG — p50
59.5 / p95 62.9 ms against a 250 ms p95 budget, encode p50 72.2 / p95 75.2 ms beside
it (2026-10-20). It is pure CPU work in a debug build with no GPU adapter involved,
so it is not comparable with the `render-frame-time` row, which covers the wgpu path.

Caption import has a budget row: `command.caption_import` parses a two-hundred-cue
SubRip file and applies it in one revision-checked command — the path a reviewed
transcription proposal applies through — at p50 5.9 / p95 6.0 ms against a 60 ms p95
budget (2026-10-20). The row undoes each import outside the timed region so every
iteration starts from the same project.

Noise filtering has a budget row: `command.audio_cleanup` measures the product's own
path — derive the floor from the material, then run the filter — over a three-second
fixture, so it includes two helper process starts. Measured 2026-10-20 on this host:
p50 135.5 ms / p95 142.0 ms against a 1000 ms p95 budget. The threshold is a
regression guard with roughly seven times headroom rather than a target: the cost is
dominated by starting `ffmpeg`, not by OR's code, and a filter chain that decoded at
real time would blow past it. The lane reports the row `UNVERIFIED` on a host with no
ffmpeg helper rather than inventing a number.

Noise filtering's app path is now exercised end to end: the `audio-cleanup` journey
drives the media action through the real bridge on a noisy fixture and asserts the
confirmation, and the transcript shows the library gaining the cleaned item. It also
found the defect that the harness's silent fixtures had hidden: a silent source
measured `-inf` in every window and reported a generation failure. Silent material is
now `CLEANUP_SILENT_SOURCE` with its own message.

Noise filtering's app path is guarded but not yet measured: four widget tests cover
the desktop and compact actions, the import of the result, the failure path and the
no-audio case, and the clean-up itself is measured through the CLI and the helper.
What is missing is a journey that runs the action end to end against a real helper
and a performance row for the command — both recorded as owed in
[PRODUCT_ROADMAP.md](PRODUCT_ROADMAP.md).

Noise filtering is measured, not asserted. `scripts/measure_denoise.py` builds a
fixture whose two windows are known — one second of white noise, then two seconds of
a 1 kHz tone buried in the same noise — runs the packaged helper's `afftdn` chain
with the noise floor **derived from the fixture's own measurement**, and reports both
windows. The product also derives the floor itself when it is not given one: it measures
per-window RMS levels, takes the quietest tenth and adds two dB. On the fixture that
derivation produced -21 dBFS, and the resulting clean-up measured identically to the
run with the figure passed in, so the derivation is not a separate code path with
its own numbers. The same measurement drives the product path when given `--cli`: on 2026-10-20 the
`or media clean-audio` command and a direct helper invocation produced identical
numbers on the same fixture (11.50 dB floor drop, 4.90 dB signal change, 6.60 dB of
discrimination), which is what shows the command uses the chain it claims to.
Measured 2026-10-20 with FFmpeg 7.1.5 on this host:
`afftdn=nf=-21:nr=12` moved the noise floor from -23.20 dB to -34.70 dB (**11.50 dB
drop**) while the tone fell 4.90 dB, i.e. **6.60 dB of discrimination** — the filter
removed more noise than signal, which is what denoising means. The check requires at
least a 6 dB floor drop and at least 3 dB of discrimination; a filter that merely
turned everything down would fail the second condition. The tone is the hardest case
for an FFT denoiser (a stationary narrowband signal looks like noise to it), so the
check deliberately bounds the *ratio* rather than the absolute signal loss, and the
script says so where the thresholds are defined.

Track reordering has a row: `command.track_move` on a two-track project is p50
0.002 ms / p95 0.003 ms against a 2.0 ms p95 budget (2026-10-20). It costs the
track count rather than the clip count — a removal and an insertion in a short
vector — which is why it is two orders of magnitude below the clip-level rows.

The three clip-level moves that leave a clip where it sits all have rows, measured
on the tiled 200-clip project (2026-10-20): `command.slip` (move one clip's source
window) p50 0.131 / p95 0.136 ms, `command.roll` (move the boundary between two
adjacent clips) p50 0.229 / p95 0.238 ms, and `command.slide` (move a clip and let
both neighbours absorb it) p50 0.271 / p95 0.291 ms. Slide is the most expensive of
the three because it rewrites three clips and both neighbours' source ranges, and
all three are far inside their budgets.

Ripple trim has its own row too: `command.ripple_trim` shortens the first clip of
the 200-clip project and pulls the other 199 with it, measured p50 0.295 ms /
p95 0.312 ms against a 6.0 ms p95 budget (2026-10-19). Shifting almost two hundred
clips costs about twice a single-clip trim, because it is one linear pass over the
track's clip list.

The overwrite edit mode has its own row: `command.overwrite_insert` replaces a
covered clip in one transaction, measured p50 0.260 ms / p95 0.286 ms against a
6.0 ms p95 budget on the 200-clip project (2026-10-19). It pays for the delete and
the insert together, so it is the cost a user sees when an edit replaces a range
instead of being refused.

| Interaction | p50 | p95 |
|---|---|---|
| Switch Projects <-> Home | 100.0 ms | 149.8 ms |
| Step the preview one frame forward and back | 4.7 ms | 16.8 ms |
| Open and cancel the export dialog | 181.6 ms | 229.2 ms |

These are **software-path developer numbers on this host**, not a device, GPU or
release-build claim, and no ceiling is asserted for them yet: they are recorded so
a later regression has a baseline, and so "the surface is responsive" is a number
instead of an assertion. A presented-frame budget remains the open gap below.

### The budget must be able to fail

Two documented seams make the enforcement path itself testable, and both are used
by `scripts/test_bench_quality.py`:

- a real injected slowdown:
  `OR_BENCH_INJECT_SLEEP_MS=500 OR_BENCH_ONLY=cli-cold-start python3 scripts/bench_quality.py`
  must print `row cli-cold-start : FAIL — best 506.1 ms …` and exit 1 (observed
  2026-10-18);
- `OR_BENCH_THRESHOLD_SCALE=0.001` tightens every ceiling and raises every floor,
  so one run trips the enforcement path of every row kind without shipping a
  regression. A scale above 1.0 is ignored: the seam can only make a budget
  stricter.

`scripts/test_bench_quality.py` runs inside `bash scripts/check-repo.sh` and
checks that every row declares a dated baseline and a threshold kind, that a
breach fails while an in-budget number passes, that an unmeasurable row is
`UNVERIFIED` and never `PASS`, and that an empty row selection is refused.

Named machine class: **Debian 13 (trixie) x86_64, OptiPlex-class workstation,
13th Gen Intel Core i5-13500T, 15 GiB RAM, NVMe SSD, Intel UHD 770/Vulkan** — the
host every baseline above was measured on. The budget is a property of this
class. GitHub-hosted runners have their own variance and keep their own
thresholds; no number measured here is claimed as a hosted-runner budget.

What is **not** covered, and stays a recorded gap:

- **Presentation frame time is still unmeasured.** `render-frame-time` measures a
  headless offscreen render plus readback, not a presented viewer frame on a
  real window: there is no vsync, no compositor and no swapchain in that path.
  The Linux preflight renders through software (`LIBGL_ALWAYS_SOFTWARE`) under
  Xvfb and the Android lane runs a SwiftShader emulator, so neither can supply a
  presented-frame number. The command that would settle it is a real
  presentation on this host's Intel GPU with the viewer texture path active, and
  it does not exist yet.
- **Release-build speed.** Every in-process row is a debug build.
- The only packaged export wall times are hosted values recorded in the roadmap
  history (Linux 47.299 s, macOS 57.394 s, Windows 182.114 s and 169.702 s for
  the same 30-second VP9/Opus journey) from GitHub-hosted runners of unstated
  hardware class. The Windows spread is runner variance; turning those numbers
  into a product requirement would encode CI noise as a budget.
- Physical-device Android responsiveness remains unmeasured.
- A breached row measured beside a contended host is a measurement to repeat, not
  automatically a product regression. `bench_quality.py` prints `MemAvailable`
  and every foreign Gradle/Kotlin daemon it finds for exactly that reason.

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

1. **No presented-frame performance budget** — command, project-file, offscreen
   render, queue-contention and Flutter-layer interaction rows are now measured,
   but a *presented* viewer frame (vsync, compositor, swapchain) is still
   unmeasured and the interaction numbers above carry no ceiling; the command path
   that would measure a presented frame does not exist yet.
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
4. **The agent-facing UI harness exists; its Android target has not yet produced
   a guest transcript.** `scripts/or_ui.py` lists the running app's screens,
   dumps the semantics tree, takes a screenshot at a named viewport, performs one
   semantic action, or runs a scenario end to end, on Linux under Xvfb and on an
   Android guest. On 2026-10-18 the Linux target passed the full editor journey
   (50 steps), the screen inventory at 1280x800 and 320x440, and the negative
   control. The Android target needs a booted guest and a built `ui.apk`; whether
   it has produced a guest transcript is recorded in
   [PRODUCT_ROADMAP.md](PRODUCT_ROADMAP.md), not assumed here.

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
- Acceptance has two dimensions a build or unit-test pass cannot establish, so
  they are part of the bar rather than optional extras: the **user-visible
  interface** (see [UX_ACCEPTANCE.md](UX_ACCEPTANCE.md) and skill `or-ui-ux`) and
  **measured interaction performance** (see the budget section above). A
  capability whose surface has no semantic, behavioural or visual evidence, or
  whose interaction latency stays `UNVERIFIED`, is not accepted.
- A skipped, missing or unavailable check is `UNVERIFIED`; it is never PASS.
- Hosted evidence: after a successful hosted run for the current commit,
  `bash scripts/record-hosted-evidence.sh <workflow>` records it in `.hub/hosted-evidence.json`. The Hub
  merges it into a verification record only when it names that exact revision;
  without it `release-check` is `BLOCKED` by "no hosted-runner evidence", which
  is true, not a defect.
