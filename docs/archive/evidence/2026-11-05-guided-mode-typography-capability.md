# Guided first video, scene typography and measured capability — 2026-11-05

Status: **local macOS and Linux evidence** for branch
`mission/pro-video-platform` on the commits named below. Windows stays
unverified. The limits at the end are part of the record.

## Commits

| Commit | Increment |
|---|---|
| `bf9e7bc` | guided mode, intent-first export dialog, composition guides |
| `d073c61` | MotionScene schema v2 typography |
| `8161fa5` | import proves a decode; capability matrix measured |
| `78ce0b0` | agent workspace routes capability and design guides |
| `0a7475c` | local gates green; measured macOS budget recorded |
| `63a4bd6`, `6a1fbb3` | the next step moves out of its own band so no pane changes height |
| `343e089` | the import decode gate sizes its own lease budget |

## What was proven, and how

| Claim | Evidence |
|---|---|
| A beginner can create a project, import, place a first clip, add a title, reach the audio step, toggle guides and open the export dialog | `python3 scripts/or_ui.py run scripts/ui_scenarios/guided-beginner.json` → `RESULT: OR_UI=PASS`, 39 steps, artifacts in `apps/or_app/build/ui-harness/artifacts/` |
| Scene typography is editable and reaches the scene | `flutter test test/motion_library_test.dart` → 23 pass, including "the inspector edits scene typography without a bridge regeneration"; `motion-scene-edit` journey → PASS, 63 steps |
| A scene with an explicit box wraps instead of shrinking, and tracking/weight change the ink | `cargo test -p or_render --lib` → 43 pass, four typography tests |
| Schema v2 is additive and refuses unknown keys and out-of-range values | `cargo test -p or_motion --lib` → 95 pass; `schema_version_tests` |
| Historical text JSON decodes to the same metrics | `cargo test -p or_core --lib` → 375 pass; `advanced_typography_serializes_only_when_it_differs_from_the_default` |
| An undecodable-on-this-host codec is refused with the reason and no media row | `scripts/ui_scenarios/media-decode-refused.json` → PASS, 10 steps, tree artifact holds the refusal text |
| The refusal message keeps its advice inside the import-summary budget | `cargo test -p or_core --lib media_import` → 5 pass |
| The guided export dialog really encodes, and the file plays | `scripts/ui_scenarios/export-encode.json` → PASS; its inner measure step waits for **Export complete** (p50 3066 ms) and the produced `export.mp4` probes as `h264` 1440×1080 at 30/1 with `aac` 48 kHz stereo, 15 frames, then decodes fully: video to ffv1 and audio to WAV |
| Flutter surface stays green | `flutter analyze --no-pub` → no issues; `flutter test` → 467 pass |
| Repository gates | `scripts/check-repo.sh` → pass (858 tracked files), `check_doc_links.py` → pass (155 files), `check_doc_freshness.sh` → pass |

Measured media rows, per host, are in
[media-capability-matrix.md](../../reference/media-capability-matrix.md); the
adoption verdicts for the twenty candidate repositories are in
[technology-review.md](../../reference/technology-review.md).

## Correction, same day

The guided first-video scenario caught a real defect the narrower refusal
scenario could not: `video_frame_decodes` asked for a single in-flight lease and
so refused its own session, which made every import report `DECODE_UNAVAILABLE`.
`apps/or_app/build/ui-harness/artifacts` and the `OR_IMPORT_TRACE_FILE` trace
showed `import-done ok=false code=DECODE_UNAVAILABLE msg=… runtime in-flight
budget exceeded (limit 1)`. Fixed by sizing the gate at four leases and 64 MiB,
and pinned by `crates/or_media/tests/software_decode.rs::the_import_decode_gate_admits_a_frame_it_can_decode`
plus the guided journey itself, which now asserts the media row and the first
clip appear. The refusal scenario still passes, and its trace shows the AV1
reason rather than a budget reason.

## Linux verification, same branch

The Debian 13 host was brought up in user space only — no `sudo`, no system
packages, no security-setting change: Flutter 3.47.5 at `~/or-linux/flutter`
(framework `6a19cca564`) and rustup `1.98.0` under `$HOME`. The clone tracked
the same branch and commit as the macOS tree.

| Claim | Evidence |
|---|---|
| The Flutter surface analyzes and tests on Linux | `flutter analyze --no-pub` → no issues; `flutter test` → `+462 ~5` pass |
| The Rust bridge and IPC layers test on Linux | `cargo test -p or_app_bridge -p or_ipc -p or_runtime --locked` → 53, 7, 10, 4 and 22 pass |
| The CLI drives a real project on Linux | `or project create`, `media add` (decode gate admitted the fixture), `timeline add-track` video and text, `insert-clip`, `insert-text` |
| The editor preferences are reachable under their own name | `scripts/ui_scenarios/resources-setup.json` → PASS: Settings → Resources reports each bundled component and the external-tools controls, and the guided/tips/guides/inspector switches sit in an **Editor** panel rather than trailing the narration panel |
| The semantic editor journey passes on Linux | `python3 scripts/run_linux_ui_preflight.py` under bounded Xvfb → `All tests passed!` at `6a1fbb3` |

Export through the CLI is **not** claimed on Linux: `or session serve` installs
no export handler (`EXPORT_RUNTIME_UNAVAILABLE`, "the project host has no export
runtime"), which the desktop application supplies. That is a boundary of the
CLI host, and the same boundary exists on macOS.

The Linux run is also what diagnosed the six failing widget tests and the
failing `editor` workflow: the guided band took 46 px from the panes, which on a
1280×720 window pushed the first clip under a transient snackbar and pushed
toolbar controls past the visible surface. At `6a1fbb3` the step is a toolbar
item, so nothing moves, and both the widget suite and the Xvfb journey pass.

## Limits, stated plainly

- **Windows verification did not happen.** The Windows 11 host has no
  toolchain and no FFmpeg, and the mission's permission boundary forbids
  installing system tooling there. No Windows build, install or journey is
  claimed. macOS and Linux are both verified above, from one branch and commit.
- **No packaged install was exercised on Linux.** The journey ran from a
  developer build under Xvfb, not from a Linux bundle installed through the
  package manager.
- **The project text dialog still cannot set line height or tracking.** The
  bridge view is not widened; the two values default to the historical 1320 and
  0, so existing projects render identically. This is recorded in
  [text-captions.md](../../features/text-captions.md), not implied complete.
- **AV1 does not decode on this Apple silicon host** with the pinned build, and
  the app now says so at import instead of failing at the first frame. AV1 is not
  claimed as playable there.
- **No optional component was installed through the resource manager** in this
  session, so that GUI journey is unverified.
- **Screenshots were not visually inspected** in this session; interaction claims
  rest on widget keys and the semantics tree, which is why the journeys above use
  keys and text rather than appearance.
