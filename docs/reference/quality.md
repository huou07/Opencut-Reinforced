# Quality Bar

Last reviewed: 2026-10-27

Scope: the attributes OR holds itself to, how each is measured, and which gaps
are known. The current runtime target is the local macOS app on the owner's M5
MacBook Air; older Linux/hosted numbers are historical, not a current baseline.

## Attributes

| Attribute | Target | Measured by | Gate |
|---|---|---|---|
| Correctness | acceptance journeys pass | `bash scripts/acceptance.sh` | acceptance |
| Correctness (source) | hygiene, docs, claims, core dependency boundary | `bash scripts/check-repo.sh` | verify |
| Performance | bounded local budget rows (process, command, project file, render frame, queue priority) on the named host class | `python3 scripts/bench_quality.py` | acceptance |
| Reliability | save/reopen, undo/redo, autosave and recovery in named journeys | widget + UI journeys | acceptance / hosted |
| Accessibility / UX | 4.5:1 contrast, ≥11 px type, 24/44 px hit areas, keyboard alternatives | headless guards in [design/ux-invariants.md](../design/ux-invariants.md) | widget tests |
| Security | no secrets tracked; dependency boundary enforced | `gitleaks detect`, `check_or_core_deps.py` | acceptance / hosted |
| Compatibility | macOS/Windows/Linux/Android hosted jobs | CI matrix | hosted |
| Maintainability | links resolve, current docs dated, claims resolve to a commit | doc and claim gates | verify |

## Performance discipline

A performance claim needs the workload, source/build, codec/backend, machine and
measured result. The recorded budget rows were measured on 2026-10-18 on an idle
Debian 13 (trixie) OptiPlex-class x86_64 host; do not reuse them as M5 numbers. Add a current Mac measurement only after recording
those fields.

### MotionScene materialisation on the local host (2026-10-27)

- Workload: the ignored `measures_materialisation_throughput_at_a_named_canvas`
  scene — a 24-line Rust sample, 640×360 at 10 fps, 88 frames.
- Source/build: `or_app_bridge` the commit that parallelised the motion sweep,
  release profile.
- Codec/backend: FFV1 in Matroska, software encode, CPU rasteriser.
- Machine: MacBook Air M5 (Mac17,3), 10 CPU cores (4 performance, 6 efficiency),
  8 GPU cores, 16 GB unified memory, idle host.
- Result: 351 frames/s end to end (0.251 s) against 78 frames/s before the
  change (1.124 s) — 4.5× — with an identical 2,104,841-byte output. The
  raster-only sweep at 1920×1080 went from 33 to 166 frames/s in the same run.

### Export through the render-ahead pipeline (2026-10-27)

- Workload: the ignored `a_render_ahead_export_writes_a_probeable_file` — a blank
  1920×1080, 30 fps, 3 s timeline (90 frames), no source media, so the row measures
  the export loop itself: GPU composite, readback, colour conversion, FFV1 encode,
  mux.
- Source/build: `or_app_bridge` the commit that pipelined the export loop, debug
  test profile.
- Codec/backend: FFV1 in Matroska, software encode, wgpu Metal for the composite.
- Machine: MacBook Air M5 (Mac17,3), 10 CPU cores, 8 GPU cores, 16 GB unified
  memory, idle host.
- Result: 21.6 frames/s (4.166 s) against 13.7 frames/s (6.569 s) for the
  frame-at-a-time loop — 1.58× — with both runs writing 947,529 bytes. The file
  probes as Matroska, `ffv1` 1920×1080 at 30 fps plus `pcm_s16le` 48 kHz stereo,
  duration 3.000 s on both streams.

This row has no source decode in it: a project with camera media will show a
smaller multiple, because decode adds work to the render side that pipelining
cannot overlap with itself.

These are measurements of one workload, not a budget row: they are not enforced
by `scripts/bench_quality.py` and must be re-measured before being quoted again.

## Known gaps

1. No presented-frame (vsync/compositor) budget; pacing is read from the app's own
   log, not a dedicated gate.
2. Hardware decode/encode, shared GPU surfaces and Android audio are not
   implemented.
3. Native/packaged claims for Windows, Linux and Android remain hosted-CI
   authority; a local result never becomes acceptance for them.
4. Representative camera-media compatibility, large libraries and
   physical-device performance are unproven.
5. Clean-machine install/upgrade/uninstall is unverified.

## Evidence rules

- `scripts/check-repo.sh` is the source gate; `scripts/acceptance.sh` adds the
  documented Rust and Flutter lanes.
- A skipped, missing or unavailable check is `UNVERIFIED`, never PASS.
- Hosted CI is the only authority for native, packaged and cross-platform claims.
- Interface behaviour and interaction performance are part of the bar, not extras.

## Related

- [testing.md](testing.md)
- [operations/verification.md](../operations/verification.md)
