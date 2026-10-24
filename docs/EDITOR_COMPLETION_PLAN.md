# OR Editor Completion Plan (local macOS mission)

Created: 2026-10-25. Owner delegation: finish every basic non-AI editing
capability and the deterministic MotionScene journey on this MacBook Air, with
an architecture that stays Rust-owned, a CapCut-class workflow, and evidence
from the real application.

## Objective and stop condition

The mission is complete when a person can, in the packaged local macOS app:

1. create a project, import video/image/audio, arrange a multi-track timeline
   with pointer trim/move/split/reorder, speed and volume, snapping, undo/redo,
   composition preview, save/reopen and a playable export;
2. author, preview, materialise and insert a deterministic MotionScene into the
   ordinary timeline and export the combined edit;
3. do both through an interface that follows `DESIGN.md` (Focused Monochrome),
   keeps keyboard parity, and has no known clipping, invisible feedback or
   dead-end controls at the supported window sizes.

"Rust owns canonical state" is unchanged: every edit still travels through the
validated project command path. Flutter remains presentation and bounded
read-model state. No new orchestrator, second project store or editing engine.

## Out of scope (deferred by the owner)

AI/transcription, cloud providers, community/templates catalog, animation-heavy
showcase surfaces, Android/Windows packaging, hosted CI claims, release
publication.

## Verified starting point (2026-10-25, branch `codex/local-macos-dev`)

- `flutter test`: 140 passing (including new snackbar-contrast and 800 px
  workspace-header guards).
- Real app on this machine: Screen Recording and Accessibility automation are
  now available; window capture works (`screencapture -l`), native menu and
  window frames can be read.
- Rust bridge exposes the full basic-editing command surface already
  (`crates/or_app_bridge/src/api/project.rs`, `ProjectGateway`); the gap is
  presentation, discoverability, some workflow wiring, verification and
  performance, not missing domain commands.
- Confirmed defects at the start (all reproduced from real pixels or the real
  semantics tree): snackbar text invisible (1:1 contrast), 800×600 first-run
  window with clipped workspace actions, action cluster not right-aligned,
  toolbar overflow at 1024/1440 with a permanently disabled zoom control,
  inspector internal-unit labels and buried Apply, modal clip dialog with raw
  IDs and eleven equal-weight actions, dead tool rail entries, page content
  vertically centred, white system tooltips, no File menu, Settings… has no
  action.

## Design direction

Keep the existing semantic tokens. Adopt the information architecture of a
modern desktop NLE (CapCut/Premiere class):

- one project header with save state and primary actions, right-aligned;
- left tool rail that opens real panels (Media, Text, Audio, Effects,
  Transitions, Motion), no placeholder entries;
- centre viewer with transport, timecode and transform/crop affordances;
- right inspector with human units (%, °, px, dB, s), presets and a sticky
  action row;
- bottom timeline with thumbnail/waveform clips, visible trim handles, a
  context menu, markers, snapping and pointer + keyboard parity.

Density stays high (8/10), motion stays functional (<200 ms, reduced-motion
aware). No pure-black surfaces, no new accent colours; selection cyan
`#3FC7FF` and the existing type scale remain authoritative.

## Work units

Each unit is one coherent, independently testable change with its own commit.

### U1 — Feedback, first-run window and shell anchoring (in progress)

- Theme: declare `snackBarTheme` (content/close/action roles) and
  `tooltipTheme`; guard both with contrast tests.
- Native window: first-run 1440×900, `contentMinSize` 1024×680, frame autosave.
- Shell: stretch the page row so every page title shares one top edge; add focus
  rings to the brand and desktop navigation.
- Workspace header: right-aligned actions; icon-only + overflow below 720 px;
  no horizontal scroll for primary actions; verified at 800 px by test.
- Empty states scale their padding instead of overflowing short panels.
- Create dialog asks for frame rate (default 30) and applies it through
  `set_timeline_sequence_frame_rate` before the workspace loads, so Play and
  frame stepping work immediately.
- Transport readout renders `HH:MM:SS:FF` with exact rationals in the tooltip;
  the rate control says "Set frame rate" and is emphasised only when missing.

Evidence: widget tests, `editor-journey` on macOS, real-window screenshots at
800/1024/1280/1440.

### U2 — Timeline as the editing surface

- Clip visuals: thumbnail strip for visual media, waveform for audio, cached
  per (mediaId, artifact kind) and reused across redraws.
- Selection: click selects without a modal; double-click focuses the inspector;
  right-click/`…` opens a context menu (Split at playhead, Duplicate, Delete,
  Ripple delete, Speed, Volume, Transition, Remove).
- Visible trim handles, pointer cursors, snapping guides, playhead scrub.
- Toolbar: core tools (Split, Delete, Duplicate, Speed, Volume, Text,
  Transitions, Adjust) plus Add-track and Captions menus; no dead controls;
  fits at 1024 without horizontal scroll.
- Keyboard: Space, ⌘B split, ⌘D duplicate, ⌫ delete, ⌘Z/⇧⌘Z, arrows/shift
  arrows, ⌘+/⌘−, Home/End, Escape; shortcut hints use ⌘ on macOS.

Evidence: widget tests for reachability at 1024/1280, journey extension,
screenshots with a selected clip and a context menu.

### U3 — Inspector and direct manipulation

- Human units and presets for transform, crop, opacity, effects, speed,
  volume/pan/fades, transition kind/duration, text formatting.
- Sticky Apply/Reset; section grouping; reset-per-section; disable when the
  track is locked.
- Viewer overlay for the selected visual clip: drag to move, corner handles to
  scale, rotation handle, crop mode, centre/edge snapping, double-click reset.
  Gestures preview locally and commit one command on release.

Evidence: widget tests for value conversion and one-command-per-gesture,
screenshots of the overlay and inspector.

### U4 — Media, Text, Audio, Effects and Transitions panels

- Media: import, thumbnails, drag-to-timeline, add-at-playhead, relink/remove.
- Text: add title/caption at playhead, live formatting, caption import/export.
- Audio: gain/pan/fade presets, mute/solo.
- Effects: typed presets (brightness, contrast, saturation, blur) applied to the
  selection with visible badges.
- Transitions: presets applied to clip in/out with duration control and a
  visible badge on the clip.

Evidence: each panel exercised on the real app; journey covers add → edit →
reopen.

### U5 — Export and project lifecycle

- Export sheet: profile cards (MP4 H.264/AAC, WebM VP9/Opus, Matroska FFV1),
  destination, progress with cancel, success with Reveal/Play.
- Project settings (frame rate, canvas summary) and a real File menu (New, Open,
  Save, Save As, Export, Close) plus Settings ⌘, routed to the shell.
- Autosave/recovery copy and recent-project behaviour remain unchanged.

Evidence: exported files validated by decoding them; screenshot of the sheet;
native-menu walk with Accessibility automation.

### U6 — MotionScene as a first-class tool

- Motion panel: source picker, scene parameters, deterministic preview,
  materialise, insert at playhead, re-edit from the clip.
- Keep `.ormotion.json` declarative, no provider/network, bundled fonts only.
- Tests for determinism and for the insert/export round trip.

Evidence: MotionScene journey on the real app with a saved project and a
combined export.

### U7 — Performance on M5

- Cache thumbnails/waveforms, bound decode concurrency, avoid full rebuilds on
  playhead ticks, keep the preview texture path.
- Confirm hardware encode (VideoToolbox) on export; measure playback CPU/frame
  pacing and export wall time; record numbers in the plan evidence section.

Evidence: measured timings recorded with the exact build; no unmeasured claims.

## Evidence policy

- `UNIT_VERIFIED`: focused widget/unit tests and analyzer on the exact commit.
- `INTEGRATION_VERIFIED` (local): real macOS app launched from a packaged local
  build, driven through the real UI, screenshots retained.
- No hosted runs, no publication. Native-menu verification is local only.

## Risks

- The editor screen file is large; new panels live in dedicated widget files and
  the existing command wiring is reused rather than duplicated.
- Live inspector edits must not spam undo: values commit on release/Apply and
  previews stay presentation-only.
- Thumbnail/waveform decode pressure can stall the UI; concurrency is bounded
  and cached with eviction.
- macOS menu routing needs one method channel; failure must fall back to the
  in-app controls rather than disabling the app.
