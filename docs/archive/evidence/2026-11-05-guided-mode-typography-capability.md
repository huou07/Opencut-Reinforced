# Guided first video, scene typography and measured capability — 2026-11-05

Status: **local macOS evidence** for branch `mission/pro-video-platform` on the
commits named below. It is not hosted CI evidence and not cross-platform evidence.
The limits at the end are part of the record.

## Commits

| Commit | Increment |
|---|---|
| `bf9e7bc` | guided mode, intent-first export dialog, composition guides |
| `d073c61` | MotionScene schema v2 typography |
| `8161fa5` | import proves a decode; capability matrix measured |
| `78ce0b0` | agent workspace routes capability and design guides |

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

## Limits, stated plainly

- **Cross-platform verification did not happen.** The Debian host has no
  `cargo`/`flutter`; the Windows host has neither and no FFmpeg. The mission's
  permission boundary forbids installing system tooling on those hosts. No
  Linux or Windows build, install or journey is claimed here.
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
