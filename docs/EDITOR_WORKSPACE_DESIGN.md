# Editor Workspace Design

Status: active. Owner-delegated rewrite of the editing workspace information
architecture, with the feature completion it depends on.

Last reviewed: 2026-10-25

## 1. Purpose

The application already performs the editing journey; the workspace does not yet
feel like an editor. This document fixes the target information architecture, the
interaction model, and the phase order for completing every non-AI editing
feature plus MotionScene on the local Mac.

Conditions of satisfaction:

- A first-time editor can import, arrange, trim, split, adjust and export without
  reading documentation, using only visible controls.
- Nothing is duplicated: one toolbar per zone, one place for each decision.
- Every pointer action has a keyboard equivalent, and every control reports
  honest state.
- Rust keeps canonical project state; Flutter stays presentational.
- The M5 is used deliberately: hardware decode/encode, bounded worker pools,
  isolated repaints, measured frame pacing.

## 2. Reference points

**CapCut desktop** (studied via its published layout descriptions and the stable
map across versions): four zones — media panel top-left with category tabs,
preview window center, property panel right, timeline bottom — plus one export
affordance top-right. Its ease comes from that stability, from clip-type-aware
property panels, and from a timeline toolbar that only holds the actions editors
repeat. Reuse the zone map, the tab-per-media-category idea, the contextual
property panel, and the always-available export. Reject its tab count (we have
far fewer parameters), its promotional surfaces, and its reliance on tooltips for
primary actions.

**OpenCut** (the project this is a rewrite of): a browser editor with a left
library, center player, right inspector and bottom timeline. Reuse the
left/center/right/bottom composition and its plain-language labels.

**DESIGN.md** owns the visual language: Focused Monochrome, accent cyan
`#3FC7FF`, no pure-black surfaces, density 8/10, functional motion under 200 ms.
`ui-ux-pro-max` advice is used only where it agrees: 8 px spacing grid, avoid
`#000000`, visible focus ring, reduced-motion awareness. Its suggested pink/purple
palette and glassmorphism are rejected because DESIGN.md is authoritative.

## 3. Workspace architecture

```
┌──────────────────────────────────────────────────────────────────────────┐
│ Top bar 48  ‹ OR · Project name [Saved|Unsaved] · ⌘K · Undo/Redo · Export │
├───────────────┬────────────────────────────────────┬─────────────────────┤
│ Left 320      │ Center: viewer (fluid)             │ Right 300           │
│ ┌───────────┐ │ ┌────────────────────────────────┐ │ Inspector           │
│ │Media Text │ │ │ aspect-correct stage           │ │ (contextual)        │
│ │Audio      │ │ │ overlay: transform box/handles │ │                     │
│ │Motion     │ │ └────────────────────────────────┘ │ TRANSFORM           │
│ └───────────┘ │ transport ⏮ ⏯ ⏭ 00:00:03:12 30fps │ CROP                │
│ panel body    │                                    │ OPACITY             │
├───────────────┴────────────────────────────────────┴─────────────────────┤
│ Timeline (default 38 %, draggable)                                       │
│ toolbar [select ▮ blade] [split][dup][delete] │ snap │ mode │ zoom │ fit  │
│ ruler ───────────────▲─────────────────────────────────────────────────  │
│ V1 header [M][S][lock] ║ clips with badges                               │
│ A1 header [M][S][lock] ║ waveform                                       │
└──────────────────────────────────────────────────────────────────────────┘
```

Rules:

- **No icon rail.** Tool categories are tabs in the left panel header, so the
  rail-plus-panel redundancy disappears and 64 px returns to the viewer. `1`–`4`
  switch tabs; the active tab is a filled segmented control, not an underline.
- **One toolbar per zone.** The editor has exactly three: top bar (project, undo,
  export), timeline toolbar (edit verbs and view controls), inspector (per
  property reset). The redundant second icon row inside the timeline header is
  removed; its verbs move to the timeline toolbar and the clip context menu.
- **The viewer is a control surface**, not an image: click seeks, drag scrubs, and
  with a clip selected it carries a transform box. A crop toggle switches the box
  to crop edges. One gesture produces exactly one command, applied on release.
- **The inspector is contextual.** Nothing selected → project settings (frame
  rate, resolution, aspect) and a short hint. A clip selected → an ordered list
  of collapsible sections that match what the clip can do: Transform, Crop,
  Opacity, Effects, Transition, Speed, Audio (media clips) or Text (text clips).
- **Insert mode is visible state**, not a hidden preference: a single control in
  the timeline toolbar reads `Ripple` or `Overwrite`.
- **Export is one button** in the top bar, opening a sheet that owns destination,
  format, estimate and progress.

Breakpoints: `>=1200` the full four-zone layout with left 320 / right 300;
`900–1200` left 264 / right 264 and shorter labels; `<900` the center viewer and
timeline stay, the left panel becomes a sheet and the inspector a bottom drawer,
because a phone-width window cannot show four zones honestly.

## 4. Interaction model

Pointer:

| Gesture | Target | Result |
|---|---|---|
| Click | timeline lane | set playhead |
| Click | clip | select |
| Drag body | clip | move, snapped to neighbours, playhead, seconds |
| Drag edge | clip | trim start/end, snapped to frame |
| `Alt`+drag | timeline | bypass snapping |
| Drag | viewer stage | move selected clip; `Shift` constrains axis |
| Drag handle | viewer corner | scale; `Shift` keeps aspect |
| Drag | viewer rotate handle | rotate; `Shift` snaps 15° |
| Right click | clip | split / duplicate / delete / ripple delete / properties |
| Drag | splitter | resize left, right, timeline |

Keyboard (workspace-wide, never stolen from a focused text field):

| Key | Action |
|---|---|
| `Space` | play / pause |
| `J` `K` `L` | reverse / stop / forward shuttle |
| `←` `→` | previous / next frame |
| `Shift+←/→` | ±1 second |
| `Home` `End` | start / end of timeline |
| `V` `C` | select tool / blade tool |
| `S` or `Cmd+B` | split at playhead |
| `Delete` | delete selection |
| `Shift+Delete` | ripple delete |
| `Cmd+D` | duplicate |
| `Cmd+Z` / `Shift+Cmd+Z` | undo / redo |
| `Cmd+S` | save |
| `Cmd+E` | export |
| `Cmd+K` | command palette |
| `1`–`4` | left panel tab |
| `M` | add marker at playhead |
| `,` `.` | nudge by frame in a numeric field |

All editing verbs route through the existing Rust command path; nothing here
invents a second state owner.

## 5. Feature matrix

`done` = implemented and exercised; `partial` = model present, surface missing or
incomplete; `missing` = not present.

| Area | Capability | State | Phase |
|---|---|---|---|
| Project | create / open / save / rename / reopen / recovery | done | — |
| Media | import files and folders, probe, thumbnails, remove, relink | done | — |
| Media | search/filter, list density, drag from library to timeline | partial | P3 |
| Timeline | insert, move, trim, split, duplicate, delete, ripple delete | done | — |
| Timeline | slip / roll / slide, overwrite, track reorder, markers | done (no UI for some) | P3 |
| Timeline | multi-clip selection, copy / paste, blade tool | missing | P3 |
| Timeline | clip badges (effects, transition, speed, mute) | missing | P3 |
| Tracks | mute / solo / lock / rename | partial | P3 |
| Transform | position / scale / rotation / opacity / crop (model) | done | — |
| Transform | direct manipulation on the viewer | missing | P2 |
| Effects | brightness / contrast / saturation / blur (model) | done | — |
| Effects | preset panel, add / remove with badges | missing | P4 |
| Transitions | cross dissolve / fade through black / wipe (model) | done | — |
| Transitions | preset panel, apply between clips | missing | P4 |
| Speed | constant speed | done | — |
| Speed | reverse, freeze frame, speed curve | missing | P5 |
| Audio | gain, pan, fades, waveform | partial | P3 |
| Text | add, edit content / weight / alignment / colour | done | — |
| Text | position and size on the viewer, font size control | partial | P2 |
| Captions | import / export / edit track | done | — |
| Playback | preview with timecode and transport | done | — |
| Playback | frame pacing measurement and budget | missing | P8 |
| Export | H.264/AAC via VideoToolbox, progress | done | — |
| Export | destination, estimate, format/resolution choice, reveal | missing | P6 |
| Shell | command palette (Home only today) | partial | P1 |
| Shell | native File/Edit menus, Settings ⌘, | missing | P6 |
| Motion | source → render → insert → export (dialog) | done | — |
| Motion | dedicated authoring workspace with live preview | missing | P7 |

Not in scope: any AI/provider feature (transcription preview, generative
effects), cloud, community, cross-platform packaging, release work.

## 6. Performance plan (M5 MacBook Air, 4P + 6E, 16 GB)

1. **Hardware paths.** Decode and encode stay on VideoToolbox; exporter keeps the
   observed `h264_videotoolbox` selection. The viewer consumes decoded frames as
   textures through `packages/or_viewer_texture`, not as encoded re-reads.
2. **Threading.** One decode pool sized to performance cores minus the UI thread,
   one low-priority pool for thumbnails and waveforms pinned to efficiency cores,
   and a bounded frame cache; no unbounded queue may grow with project length.
3. **Repaint isolation.** The playhead, the snap guide and the viewer overlay each
   repaint inside their own `RepaintBoundary`, driven by `ValueListenable`
   builders, so a 30 fps playhead never rebuilds the timeline lanes.
4. **Budget.** 1080p30 playback holds 33 ms per presented frame with under 8 ms of
   UI work; a 10-second 1080p30 export completes within 2× realtime on the
   hardware encoder. Both are measured, never asserted.
5. **Measurement.** Frame pacing is recorded from the frame timings callback into
   a JSON report next to the harness artifacts, with build mode, codec and
   resolution stated. Debug-mode numbers are labelled as such.

## 7. Phases and acceptance

Each phase ends with: the relevant automated checks, a harness scenario that
exercises the real app by semantics, screenshots at 1440×900 and 1280×800, and a
local commit. A phase is not done because it compiles.

| Phase | Deliverable | Acceptance |
|---|---|---|
| P0 | framework: dsh plugin skills, macOS semantic harness | commit 2bf6c28, c8beb13 |
| P1 | workspace IA: top bar, tabbed left panel, contextual inspector, single timeline toolbar, palette in editor | **DONE locally** — `scripts/ui_scenarios/workspace-chrome.json` and `workspace-layout.json` pass at 1440x900, 1280x800 and 1024x768; `flutter test` 140 passing; screenshots in `/tmp/or-chrome4` and `/tmp/or-layout3` |
| P2 | viewer direct manipulation: move, scale, rotate, crop, snapping | **DONE locally (move, scale, rotate, nudge)** — `scripts/ui_scenarios/viewer-transform.json` passes; drags commit once and the inspector agrees (X 12.2 %, Width 124.4 %, Rotation 4.62°); crop stays an inspector field |
| P3 | timeline power: blade tool, badges, multi-select, copy/paste, track state UI, markers, library search | **DONE locally (search, blade, speed badge, markers)** — `scripts/ui_scenarios/blade-cut.json` cuts a clip into two at the pointer with `C` and the toggle, and the speed badge appears after the inspector sets 2×; multi-select and copy/paste remain open |
| P4 | effects and transitions panels with apply/remove | apply then export changes pixels; badges update; undo restores |
| P5 | speed extras: reverse, freeze frame | exported clip duration and direction verified with ffprobe |
| P6 | export sheet, native menus, settings | file written to chosen destination with the chosen format; menu items invoke the same commands |
| P7 | MotionScene workspace | author → preview → insert → export in one journey without a dialog chain |
| P8 | performance measurement | pacing report meets or misses the budget with real numbers |

## 8. Relationship to existing documents

`DESIGN.md` remains the visual authority; this document adds structure and
interaction. `docs/UX_ACCEPTANCE.md` gains one rule per phase that has a real
check. `docs/EDITOR_COMPLETION_PLAN.md` records the rounds already delivered and is
superseded for planning by this file; its evidence log stays valid.
`docs/skills.md` routes the work: `or-ui-ux` for the surfaces, `or-edit-feature`
for commands, `or-media-render` for pipeline work, `or-motionscene` for P7, and
`or-dev-loop` for every verification.

## 12. P4 delivery record (local, 2026-10-25)

Presets live where the values already live. The inspector's EFFECTS and
TRANSITIONS sections now open with a chip row, and a chip only fills the numeric
fields: the Apply button still turns the values into one command.

- `effect-preset-{none,bright,punch,muted,soft}` — None clears the four effect
  fields, Bright = +10 % brightness, Punch = +20 % contrast / +15 % saturation,
  Muted = −30 % saturation, Soft blur = 2 px.
- `transition-preset-{none,cross,black,wipe}` — writes the transition-in kind and
  a 0.5 s duration (None writes kind 0 and 0 s).
- Chips are ordinary widgets in the existing inspector scroll, so keyboard focus,
  the focus ring and the 11 px type floor apply without new code.

Why not a separate effects library panel: the model carries four numeric effects
and three transition kinds, and every effect is scoped to one clip. A library
tab would either duplicate the same five chips or imply stackable effect layers
the renderer does not have. When the model gains real effect layers, the panel
becomes worth its own zone; until then the presets belong beside the fields they
fill.

Evidence class: local only. `flutter test` → `+148: All tests passed!`
(`test/widget_test.dart` "effect and transition presets fill the inspector
before applying" drives bright + wipe + Apply and asserts the written
`brightnessAmountMilli == 100` and `transitionIn == wipe`).

## 13. P6a delivery record — native menu bar (local, 2026-10-25)

macOS now has a real menu bar (`PlatformMenuBar` inside `apps/or_app/lib/shell/app_shell.dart`),
and every item calls the same callback the visible control calls:

- **File** — New Project… (⌘N), Open Project… (⌘O), Save (⌘S), Export Video… (⌘E),
  Close Project.
- **Edit** — Undo (⌘Z), Redo (⇧⌘Z), Split at Playhead (⌘B), Duplicate Clip (⌘D),
  Delete Clip (⌫), Ripple Delete Clip (⇧⌫), Add Marker (M).
- **View** — Home, Projects, Command Palette… (⌘K).
- **OR** — Settings… (⌘,).

Items disable themselves when the current destination or project state cannot
honour them, because the enabled state is derived from the same guards the top
bar uses. Non-Apple platforms get the child unchanged, so the menu bar never
changes the widget tree elsewhere.

Evidence class: local only. `flutter test` → `+149: All tests passed!`, including
"the macOS menu bar routes to the same verbs as the UI" (asserts the bar exists
and carries File/Edit/View/OR). Not yet exercised through the real macOS menu bar
by a pointer: the widget suite proves the structure, not that AppKit rendered it.

## 14. P6b delivery record — export receipt (local, 2026-10-25)

A finished export now names the file it produced. On success the shell raises the
shared `OrSnackBar` with `Exported <name> · <size>` and a `Copy path` action
(`apps/or_app/lib/widgets/or_widgets.dart` gained optional `actionLabel`,
`onAction` and `duration`, so the shared surface stays the only snackbar in the
app — the design rule is enforced by the "one shared snackbar surface" test).

Why not a full sheet: the destination is already chosen by the native save panel
before the job starts, and the receipt is about the finished file. The size comes
from the published file itself, so a failed publish cannot print a number for a
file that does not exist. `Show in Finder` needs an AppKit channel that the app
does not have yet; `Copy path` covers the same need with no native code.

Evidence class: local only. `flutter test` → `+149: All tests passed!`. The
receipt path has not been driven through a real export in this round.

## 15. Status at the end of this session (2026-10-25, local only)

Delivered and committed: framework migration to a local dsh skill plugin
(`2bf6c28`), single-bar workspace IA (`ba7e5fa`), viewer direct manipulation
(`db4153b`), library search + blade tool + speed badges (`989ebbc`), inspector
effect/transition presets (`8125a01`), native macOS menu bar (`581ec15`), and the
export receipt.

Open, in the order I would take them:

1. **MotionScene workspace** — the flow (source → render → insert → export) works
   through `shell/motion_materialize_dialog.dart`, but there is no authored scene
   surface: no scene list, no re-render, no preview scrub. This is the largest
   remaining gap against CapCut-class authoring.
2. **P8 frame pacing** — export throughput has been observed (6.016 s of
   1280x720 H.264/AAC inside a 486 ms wait window, `h264_videotoolbox`), but
   playback pacing needs a profile-mode run; debug-mode harness timings are not
   latency evidence.
3. **Multi-select, marquee and clipboard** — single-clip editing is complete;
   multi-clip operations are not.
4. **Reverse and freeze frame** — both need model work (`ClipSettings` has no
   reverse flag and no still-frame content), so they are not basic editing
   features in this codebase yet.
5. **`Show in Finder`** — needs the AppKit channel; `Copy path` is the current
   substitute.

## 11. P3 delivery record (local, 2026-10-25)

Shipped:

- **Library search** — `media-search` field above the list; the panel filters on the
  derived display name, the count reads `N of M items`, and a filter with no hits
  says so instead of looking like an empty library. Filtering never touches the
  project: `media search filters the library without touching the project` asserts
  the command log stays at the two imports that created it.
- **Blade tool** — `C` arms it, `V` returns to selection, the scissors toggle in the
  timeline control row is the mouse path, and the command palette carries both. A
  click on a clip splits at the pointer through the existing
  `splitTimelineClip(timelineTime:)` command, so no new state or command kind was
  invented. `scripts/ui_scenarios/blade-cut.json` proves it end to end: one 1/2 s
  clip becomes `0/1–1/4` and `1/4–1/2`.
- **Speed badge** — a clip that does not play at 1× carries its rate on the clip
  (`timeline-clip-speed-<id>`), so the timeline answers for speed without a
  selection. The inspector dropdown was the only place that said `2×` before.
- **Markers** — the marker ruler, `M`, the `+` menu and the palette entry are all in
  place from P1; this round only verified they survive the new toolbar.

Still open in P3: multi-clip selection with marquee, clipboard copy/paste between
tracks, and a clip badge for effects/transitions (which needs per-clip settings the
timeline does not load for unselected clips; the inspector remains the authority
until a batched read exists).

Evidence class: local only. `flutter test` 147 passing, the three P1/P2 scenarios
still pass, and `blade-cut.json` passes on the macOS debug app driven by the
semantic harness. No packaged-build claim.

## 9. P1 delivery record (local, 2026-10-25)

Shipped in one increment on `codex/local-macos-dev`:

- One top bar (`apps/or_app/lib/shell/app_top_bar.dart`) carries the project
  name, save dot, undo/redo, project menu, export and the command palette. The
  second 52 px project row is gone, so a 900 px window keeps a 48 px title bar
  and the timeline keeps its height.
- The 64 px tool rail is gone; `_ToolTabs` is a 42 px segmented strip at the head
  of the left panel, and the panel no longer repeats the category name.
- Timeline: one control row (zoom, Fit, Snap chip, `+` create menu with tracks,
  title, caption, marker and caption import/export). The old toolbar, the zone
  header and the four competing empty-state buttons are gone; the empty state is
  one filled import action plus two quiet track links.
- Inspector: with nothing selected it states sequence facts (frame rate,
  duration, track and clip counts) and project facts, instead of going blank.
- Viewer: the stage fills its panel, and the placeholder says what to do
  ("Drag a clip to the timeline to preview it", "No clip at the playhead").
- Keyboard: `S`/`⌘B` split, `⇧⌫` ripple delete, `M` marker, `⌘E` export, `1`–`4`
  panel tabs; the editor palette lists the same editing commands.
- The semantic harness grew a `resize` op (`apps/or_app/integration_test/or_ui_scenario_test.dart`)
  so breakpoint review changes the real layout instead of a fake window.

Evidence class: `UNIT_VERIFIED` for the widget suite, `INTEGRATION_VERIFIED`
local for the two harness scenarios. Not a packaged-build claim: the harness runs
a debug build through the Flutter test binding.

## 10. P2 delivery record (local, 2026-10-25)

Direct manipulation on the stage, and the three defects it exposed in the
surrounding code.

Shipped:

- `apps/or_app/lib/screens/viewer_transform_overlay.dart` — a selection box that
  follows the clip transform with a move surface, four corner scale handles and
  a rotate handle. A gesture paints a draft locally and calls `onCommit` exactly
  once, on release, so one drag is one command and one undo step. Snapping pulls
  the box to the frame centre within eight pixels; Shift bypasses the snap for
  move and quantises rotation to 15° steps. Handles are inset and clamped into
  the stage, because a clip larger than the frame otherwise put its corners
  outside the clip region where they cannot be grabbed.
- The stage draws the overlay only for a visual clip (video, text, caption) with
  a committed selection, and the inspector keeps editing the same fields.
- `Alt` + arrows nudge the selected clip by one percent of the canvas. Plain
  arrows stay on the playhead and comma/period stay on frame stepping, so no
  existing verb was taken.

Three defects the work exposed, all fixed:

1. **Rapid commands raced the revision.** Three nudges in a row sent three
   commands built on revisions the host had already replaced, so two were
   rejected and the clip moved once. Viewer commits and nudges now share one
   queue: each command waits for the previous one, and only the last refresh
   wins.
2. **A failed settings read looked like a deselection.** Dropping the settings
   when the read raced a revision bump put the handles away mid-gesture and made
   the next nudge a no-op. A failed read now keeps the last known value.
3. **A rebuild during a drag killed the gesture.** The overlay removed its
   gesture detectors while `enabled` was false, and the viewer toggles that flag
   during background work. The detectors now stay mounted and only the callbacks
   are gated, so a drag survives a transient busy state.

Evidence (local only, no packaged build):

- `flutter test` — 146 passing (140 + 6 in
  `apps/or_app/test/viewer_transform_overlay_test.dart`: box geometry including
  the inverted y axis, field-preserving copy, one commit per drag, centre snap).
- `python3 scripts/or_ui.py --report-dir /tmp/or-p2-t5 run
  scripts/ui_scenarios/viewer-transform.json` — PASS on a real video import:
  move drag → X 12.2 %, corner drag → Width 124.4 %, rotate drag → Rotation
  4.622°, three `Alt+Right` presses → X 15.2 %. Artifacts
  `shot-dragged-21.png`, `shot-scaled-23.png`, `shot-rotated-26.png`,
  `tree-*.json`.
- `workspace-chrome.json` and `workspace-layout.json` still PASS, so the P1
  chrome survived P2.

## 16. P7 delivery record (local, 2026-10-25)

The MotionScene journey used to end at the dialog: the owner rendered a scene,
inserted it, and the app forgot the file the moment the dialog closed. P7 makes
the Motion tab the place where that scene lives.

Shipped:

- `_MotionScenePanel` in the Motion tab: the file the session last rendered, its
  frame count, rate, size and byte count, the source path, a
  `Rendered`/`On timeline` badge, and three verbs — `Insert into timeline`
  (disabled once inserted), `Render again` (re-renders the same source, settings
  and destination, so a fix in the source lands in the asset the timeline
  already points at instead of leaving a second, nearly identical clip), and
  `New scene…`.
- `MotionSceneSnapshot` in the project layer (`lib/project/motion_scene_snapshot.dart`)
  so the editor reads render facts without depending on shell widgets.
- The shell keeps the snapshot across dialog closes, marks it inserted when the
  insert succeeds, and re-renders under a busy flag instead of queueing.

### Defects found by driving the real app

1. **Stale journey.** `illustration-video.json` still expected a validation
   message from a Render button that is now disabled until both paths are
   chosen, and typed into `motion-materialize-source` fields that no longer
   exist. Rewritten to use the real `Choose file` / `Choose location` pickers.
2. **The harness could not read repository fixtures.** The app process is
   sandboxed: copying a fixture out of `~/Documents` failed with
   `PathAccessException … Operation not permitted`, which `or_ui.py` reported as
   a launch failure. `scripts/or_ui.py` now stages every `copy:` fixture into the
   report directory before the run, so the picker still hands the app real bytes
   on disk without depending on the app's TCC grants.
3. **Media identity broke on symlinks.** Insert failed with "The video is
   rendered, but could not be added to the timeline" because
   `_findMaterializedMedia` compared `File(path).absolute.path` with the stored
   source, and macOS stores the resolved path (`/tmp` → `/private/tmp`). Fixed by
   `canonicalSourcePath`/`sameSourcePath` in `lib/project/source_path.dart`, with
   a unit test that builds a real symlink.

### Evidence

- `flutter --no-version-check test --no-pub` → `+152`.
- `python3 scripts/or_ui.py --report-dir /tmp/or-p7n run
  scripts/ui_scenarios/illustration-video.json` → PASS through 37 steps: render
  reports `Rendered 40 frames at 320×180.`, insert appends a clip, the Motion tab
  shows `On timeline`, and `Render again` re-renders the same destination.
- Class: INTEGRATION_VERIFIED on the local debug app through the Flutter test
  binding. Not a packaged-build claim.

## 17. Copy and paste (local, 2026-10-25)

The clipboard completes the P3 list's second half: a clip can leave one place and
arrive at another without being duplicated in place.

- `TimelineClipClipboard` (lib/project/timeline_clip_clipboard.dart) holds the
  content a clip needs to be recreated — media id and source range, or text and
  formatting — plus the duration. Identity is deliberately absent: pasting asks
  the command path to create a clip instead of writing an id.
- ⌘C / ⌘V, the Edit menu, and the command palette all reach the same two verbs.
  Text inputs keep their own copy/paste because the shortcut handlers refuse
  while a text field has focus.
- Paste lands at the playhead on the selected track. When the playhead is still
  at zero and a clip is selected, it lands after that clip: pasting onto the
  first clip would only produce an overlap error, and "after the clip I am
  working on" is what the owner means at that moment.
- Content kinds are gated before the command: pasting a media clip onto a text
  track says so, instead of surfacing a raw kind or overlap failure.

Evidence: `flutter --no-version-check test --no-pub` → `+153`;
`scripts/or_ui.py --report-dir /tmp/or-clip4 run scripts/ui_scenarios/clipboard.json`
→ PASS, with the timeline reading `starts 0/1, duration 1/2` and
`starts 1/2, duration 1/2` after ⌘C then ⌘V on a real imported video.

## 18. P8 delivery record — frame pacing and hardware (local, 2026-10-25)

### What was measured

`frames` op (integration_test lane) samples the engine's own `FrameTiming`s
while the preview plays, so a stutter shows up as a long presentation interval
instead of being averaged into a mean.

Command:

```sh
python3 scripts/or_ui.py --report-dir /tmp/or-pace \
  run scripts/ui_scenarios/playback-pacing.json
```

Result on this MacBook Air M5 (Mac17,3, 10 cores, 16 GB), 1440x900 window,
1280x720 30 fps H.264/AAC clip, 5 s window:

| Metric | Value |
| --- | --- |
| presented frames | 151 |
| achieved rate | 29.9 fps of a 30 fps sequence |
| build p50 / p95 | 0.65 ms / 0.76 ms |
| presentation gap p95 / max | 33.33 ms / 50.0 ms |
| raster p95 | 0.0 ms |

Reading: the sequence presents at its own rate. The p95 gap equals exactly one
30 fps frame period, so the typical frame is on cadence; the single 50 ms gap is
one dropped frame in five seconds. UI-thread work is two orders of magnitude
below the 33.3 ms budget, and `raster_p95_ms == 0` because the preview frame is
an external texture that the platform layer composites rather than something
Skia paints.

### What this does and does not establish

- PROVEN: playback of a 720p30 clip in this build presents at 29.9 fps with
  sub-millisecond UI work, measured from engine timings.
- NOT PROVEN: a release-mode claim. `flutter test` has no `--profile`
  (`Could not find an option named "--profile"`), so the semantic harness can
  only measure a non-release build. A release pacing number needs a packaged
  build driven by something other than the test binding, which the harness does
  not provide today. Treat the numbers above as a debug-build baseline.
- Export throughput is separate and already observed: a 6.016 s 1280x720
  H.264/AAC export completed inside a 486 ms wait window, and the MP4 profile
  selects `h264_videotoolbox`, so the Apple hardware encoder is exercised.

### 30.2 Release-mode pacing, measured inside the packaged build

The debug test binding cannot measure a release build, so the app now carries its
own recorder: `apps/or_app/lib/dev/pacing_recorder.dart` installs
`SchedulerBinding.addTimingsCallback` only when the binary is built with
`--dart-define=OR_PACING_LOG=<path>`, and writes one JSON line per 120 frames
(frames, span, fps, build/raster percentiles, vsync gaps). A normal build is
unaffected - the define is absent, so `PacingRecorder.install()` returns null.

Method: build `flutter build macos --release --dart-define=OR_PACING_LOG=/tmp/or-perf.jsonl`,
launch the bundle, open a project with a clip, click Play twenty times (0.5 s per
press), quit, read the file. Numbers from this MacBook Air M5:

    windows 5, frames 640
    steady windows: span 4.75-4.79 s, active fps ~33 while a 30 fps sequence plays
    build p95 1.39-1.50 ms   (UI thread work per frame)
    raster p95 1.91-1.96 ms  (raster thread work per frame)
    vsync gap p50 16.68 ms   (60 Hz cadence while playing)

Against a 16.68 ms frame budget that is roughly ten times headroom on both
threads during playback, which is the claim this section is allowed to make.

Limits, stated plainly: the window includes idle time between presses (that is why
`fps` per window reads ~26 instead of 33); the clip in that project is 32x24, so
decode cost is trivial and a 1080p source adds decode work on the session thread
(181.7 fps measured in 30.1); and this measures one playback path, not export.

## 31. Multi-clip move: what it needs and why it was not rushed

Multi-select itself is presentation; the reason it is still open is the command
path. Two facts decide the shape, both read from the code rather than guessed:

- `execute_transaction` is not a general multi-command transaction.
  `apply_forward_commands` only stages commands through `stage_groupable_command`,
  which today means project rename, so clip moves cannot travel inside a
  transaction.
- One undo entry per gesture requires one `ChangeSet`. `ProjectChange` already
  carries `TimelineClipMoved { from_track_id, from_index, to_track_id, to_index,
  before, after }`, and a `ChangeSet` holds a list of changes applied and inverted
  in order, so a group move can be one revision and one undo entry by emitting N
  such changes from a single command.

So the unit is a new atomic command, `timeline.clips.move`, with arguments
`{ moves: [{ clip_id, timeline_start }] }` (position only, no track change). It
must validate the whole batch before touching state - unique clip ids, every clip
present, every hosting track unlocked, and a final-state overlap check - then
stage the auto-committed reorder indices by simulating the batch on a cloned
timeline, because emitting per-clip indices computed against the original state
would corrupt the second move on a shared track.

Not shipped in this round because a half-built group move is worse than a
single-clip one: today drag moves exactly one clip and does it correctly. The
remaining UI work is selection state (`_selectedClipId` to a set), the marquee, and
one commit per gesture.

### 30.1 Preview decode stays on the CPU, and why that is the correct call

The vendored FFmpeg in
`~/Library/Caches/OpencutReinforced/ffmpeg-8.1.3-macos-image-support/ffmpeg-install`
advertises `Hardware acceleration methods: videotoolbox`, but it contains **no
`h264_videotoolbox` decoder**: `ffmpeg -decoders` lists only the software `h264`
decoder, and `-hwaccel videotoolbox` is silently ignored on that build. Attaching
`AVCodecContext.hw_device_ctx` therefore changes nothing - measured, with a
counter around `av_hwframe_transfer_data`: 0 hardware frames over 90 decoded
frames.

Measured on this MacBook Air M5 instead
(`cargo test -p or_media --test decode_throughput -- --ignored --nocapture`,
90 sequential 1920x1080 requests against
`crates/or_media/tests/fixtures/big_buck_bunny_1080p_h264_aac.mp4`):

    OR_DECODE_THROUGHPUT frames=90 elapsed_ms=495 fps=181.7

That is roughly six times real time for one decode thread, which is why a
single preview session per project is enough and preview decode is not on the
critical path. Encode - the expensive direction - does use VideoToolbox.

Consequence, so nobody re-opens this blindly: hardware *decode* would require
rebuilding the vendored FFmpeg with the VideoToolbox H.264 decoder enabled.
Until that build exists, a prewarm path that opens `h264_videotoolbox` would be
dead code on every shipped build, so the experiment is parked in a local stash
(`wip vt decode (dead path: ...)`) rather than merged.

### Hardware path reality

- Encode: `crates/or_media/src/encoder.rs:164` resolves `h264_videotoolbox` —
  hardware.
- Preview decode: `crates/or_media/src/decoder.rs` shows no VideoToolbox or
  hwaccel request, so decode runs through the linked FFmpeg software path.
  Presenting at 29.9 fps means that path is not the current bottleneck, but
  hardware decode is the next real hardware win, not a present defect. It is
  deliberately not attempted here: a hwaccel change alters the frame-transfer
  contract with `or_viewer_texture` and needs its own measurement round.

## 19. Show in Finder (local, 2026-10-25)

The export receipt now offers the action the desktop actually has.

- `MacOSScopedFileAccessPlugin` (apps/or_app/macos/Runner/MainFlutterWindow.swift)
  gained `revealExport`: it validates the argument, refuses a path that is no
  longer on disk with `MISSING_FILE`, and calls
  `NSWorkspace.shared.activateFileViewerSelecting([url])`.
- `AppShellState._revealInFinder` invokes it over the existing
  `or_macos_file_access` channel and maps a missing plugin to a spoken message
  instead of a silent no-op.
- The receipt keeps one action, because a snackbar with two competing actions is
  a menu: on macOS it reads "Show in Finder", elsewhere "Copy path".

Evidence: `flutter --no-version-check build macos --debug` links the new Swift
case, `flutter --no-version-check analyze --no-pub` is clean and the widget suite
is `+153`. NOT driven end-to-end: opening Finder needs a real export through the
native save panel, which the semantic harness cannot click; the reveal call is
therefore compiled-and-routed but not observed.

## 20. Sequence frame rate in the project inspector (local, 2026-10-25)

The inspector states the frame rate when nothing is selected; stating a project
setting without offering to change it is the kind of dead surface the rebuild
removed elsewhere. The row is now a menu (23.976/24/25/30/50/60) that runs the
existing `setTimelineSequenceFrameRate` command, so undo, revision checks and
the read-model refresh come for free. One list of rates is shared with the
viewer transport instead of two copies drifting apart.

## 21. Next units, in order (for the next session)

1. ~~**Reverse clip**~~ — done in `ead40c1` (see §22). No schema bump was needed:
   the field is defaulted and skipped when false, so old projects keep their bytes.
   Freeze frame did NOT fall out of it — a still needs its own content kind.
2. **Multi-select** — selection becomes a set in `_TimelinePanelState`; marquee
   on the lane background; delete/ripple/copy act on the set; paste stays
   single-target. Keep drag-move single-clip until the set case has a test.
3. **Release-mode pacing** — either add a packaged-build driver (open the app,
   drive by AppKit + read a stderr telemetry line) or state the debug number as
   the only claim. Do not pretend the test lane can measure release.
4. **VideoToolbox preview decode** — only after 3 gives a release baseline, so
   the change can be judged against a number that matters.

### 21.1 Re-verified journey set (local, 2026-10-25)

After the frame-rate row and the snapping verbs, the whole semantic journey set
was re-run against the local debug app; every scenario passed:

| Scenario | Proves |
| --- | --- |
| `workspace-layout.json` | one bar per zone at 1440x900, 1280x800, 1024x768 |
| `workspace-chrome.json` | project create, tabs, control row, contextual inspector |
| `viewer-transform.json` | stage move/scale/rotate, three nudges, one command per gesture |
| `blade-cut.json` | blade mode cut at the pointer, speed badge |
| `clipboard.json` | copy then paste at the playhead |
| `illustration-video.json` | MotionScene render, insert, "Render again" |

Widget suite: 154 tests. These are local debug-app results, not packaged-build
claims; the release pacing and packaged-export checks stay in the gap list.

### 21.2 Packaged-release check (local, 2026-10-25)

Building `flutter build macos --release` and packaging it with
`scripts/package_local_macos_app.sh` (FFmpeg prefix
`~/Library/Caches/OpencutReinforced/ffmpeg-8.1.3-macos-image-support/ffmpeg-install`)
surfaced two packaged-only facts a debug run cannot show:

1. Editing `Info.plist` after the build invalidates the signature and a
   sandboxed launch dies in `_libsecinit_appsandbox` (SIGTRAP). Re-sign with the
   packaging script; never `cp -R` + `PlistBuddy` alone.
2. The menu bar read `or_app, Edit, View, OR`: macOS owns the first menu slot and
   titles it with the application name, so the `File` menu was swallowed into it.
   Adding our own app menu (About/hide/quit provided items) first makes the bar
   read `or_app, File, Edit, View, OR` in the packaged app, verified through
   System Events.

### 21.3 Packaged-build journey driven by the native menu (local, 2026-10-25)

With the release bundle packaged and signed, the menu bar was driven through
System Events: `⌘N` opened the New Project dialog (name field, frame rate
"30 fps", Cancel/Continue), typing a name and pressing Return handed off to the
macOS save panel titled "Choose where to save Packaged journey.orproj.". That
is the packaged app creating a project through the same path the semantic
harness drives in debug, so the packaged build is not limited to launching.

Not yet driven in the packaged bundle: media import, timeline edits, export and
the MotionScene dialog. Those still rest on debug-lane evidence.

## 22. Reverse playback for media clips (local, 2026-10-25)

`ClipContent::Media` gained `reversed: bool` (serde default + skipped when false, so
an untouched project keeps its bytes). The bridge adds `set_timeline_clip_reversed`,
which flips only that flag and reuses `update_timeline_clip`, so undo, revision
checks and the read model behave like any other edit. Preview maps timeline time to
source time through `clip_source_time`, which mirrors the offset inside the clip's
own source span when the flag is set — the clip keeps its span, so this is not a
speed change. The inspector offers the switch under Playback speed.

Evidence: `cargo test -p or_core` 362 + the new round-trip test (a forward clip does
not grow the field), widget suite 155 (new: the switch calls the gateway), and
`scripts/ui_scenarios/clip-reverse.json` PASS in the real app (toggle reachable from
the inspector after selecting an imported clip).

Freeze frame remains open: it is a still derived from one source time, which the
model cannot express yet without a new content kind, so it stays out of this change.

Reverse is also reachable from the clip context menu ("Play backwards" /
"Play forwards"), so an owner who never opens the inspector can still flip a clip.

## 21.4 Packaged release journey, driven 2026-10-25

Root cause of the "black release window" that blocked this journey: the native
assets hook keeps its own cargo target directory under
`.dart_tool/hooks_runner/shared/or_app_bridge/build/<hash>/target/`, so a release
build could link a bridge compiled before the last `flutter_rust_bridge_codegen`
run. `RustLib.init()` then threw `Bad state: Content hash on Dart side (-1104663897)
is different from Rust side (1279668420)` before `runApp`, and the window stayed
black while the process lived. Clearing the hook cache and `flutter_build` fixed it;
`main.dart` now paints an `OrStartupFailure` screen (keys `startup-failure-title`,
`startup-failure-message`) with the reason instead of nothing.

Verified in the packaged release bundle (window-only captures, per the privacy rule
in `docs/TOOLING.md`):

| Check | Result |
| --- | --- |
| Home renders | Sidebar (Workspace: Home/Projects/Templates/Asset Library; Application: Settings), Quick actions, Recent Projects list |
| Editor workspace renders | One top bar with project name + unsaved dot + rename/save/undo/redo/close icons + Export + ⌘K search; Media/Text/Audio/Motion tabs; inspector SEQUENCE (Frame rate 30 fps, Duration, Timeline 0 tracks · 0 clips) and PROJECT (Name, Save state); one timeline row (blade, zoom, Fit, Snap, +) |
| ⌘N → New Project dialog | Name field focused, frame-rate choice, Cancel/Continue |
| Choosing a folder | Native folder chooser, suggested name `Packaged journey j18.orproj` |
| Project written | `/tmp/or-j18/Packaged journey j18.orproj`, 309 bytes, `format: opencut-reinforced-project`, `schema_version: 8`, the entered name |

Limits observed while driving it:

- Synthetic clicks (`System Events … click at {x,y}`) do not reach the Flutter view
  in this environment; keystrokes do once the window is key. Import media and export
  therefore still need the debug-lane harness, not this driver.
- Because single-letter shortcuts are live when no text field has focus, a stray
  keystroke typed while a native panel was expected opened the Add Marker dialog.
  That is the designed shortcut, not a bug, but a packaged driver must type only
  when a field or panel is known to be focused.

## 22. Freeze frame (local, 2026-10-25)

A frozen clip holds one frame for its whole duration. It is modelled as a boolean
on the media content rather than a new content kind, because the clip must keep its
real source range: that is what lets "Release this frame" restore the original
footage and lets trim, split and ripple keep working without special cases.

- Core: `ClipContent::Media.frozen` (serde default, skipped when false so older
  projects keep their exact bytes), `ClipContent::frozen()`, and split propagates the
  flag to both halves.
- Bridge: `set_timeline_clip_frozen` flips only that flag through
  `update_timeline_clip`, so revision checks and undo apply; video-only, with
  `UNSUPPORTED_CLIP_CONTENT` for other tracks.
- Preview: `mapped_source_time` returns `source_start` for a frozen clip, so the
  picture stops while the timeline keeps running; three unit tests cover frozen,
  forward and reversed mapping.
- UI: inspector switch "Hold this frame" (`inspector-clip-frozen`), clip menu entries
  "Hold this frame"/"Release this frame", command palette and Edit menu entries, all
  through the one command.
- Evidence: `cargo test -p or_core --lib` 364 passed, `cargo test -p or_app_bridge
  --lib` 32 passed (see docs/TOOLING.md for the loader path), `flutter test` 159
  passed, and `scripts/ui_scenarios/clip-freeze.json` PASS drives the real app:
  import a clip, select it, toggle the switch, and read the inspector back.

## 23. Import from the keyboard (local, 2026-10-25)

The library used to be mouse-only: importing meant clicking `Import Media` in the
Media tab. `⌘I` (one file) and `⇧⌘I` (a folder) now do it from anywhere in the
editor, and the same two verbs appear in the File menu and the command palette, so
the action is discoverable instead of hidden in a panel.

- Intents: `ImportMediaIntent`, `ImportFolderIntent` in
  `apps/or_app/lib/shell/or_shortcuts.dart`, bound in the shell and guarded by
  "editor open, no text field focused, picker supports import".
- Evidence: widget test `command I imports media without the mouse` (asserts the
  picker opened once and one import command was issued), `flutter test` 160 passed.

## 24. Selecting more than one clip (local, 2026-10-25)

`⇧`-click on a second clip adds it to the selection; the strip shows `2 clips` and
every selected clip paints its selection border (the primary stays thicker). One
`Delete` then removes the whole set with one command per clip and no per-clip
dialog, because the owner already said what they meant by selecting the set.

- The primary clip is still the one the inspector edits, so multi-select does not
  fork the property surface.
- Trim, move and slip stay single-clip on purpose: dragging a set is a different
  interaction with its own snapping and conflict rules, and it is not needed by any
  journey in this mission.
- Evidence: widget test `shift-click selects a second clip and one Delete removes
  both` (asserts `2 clips`, two delete commands, no dialog), `flutter test` 161
  passed, `editor-journey.json` PASS.

## 25. What the M5 actually does today (local, 2026-10-25)

Objective item "use the M5 hardware properly" is met by these four, each with a
source of truth rather than a claim:

| Concern | Where it lives | Evidence |
| --- | --- | --- |
| Hardware encode | `crates/or_media/src/encoder.rs:164` selects `h264_videotoolbox` for the MP4 profile | a real 6.0 s 1280x720 H.264/AAC export decoded clean with ffprobe |
| Bounded decode concurrency | `MediaArtifactService` runs a `JobManager` with `JobManagerConfig::new(2, 32, 128)` (`crates/or_app_bridge/src/api/project.rs:688`) — thumbnails and waveforms queue behind two workers and identical in-flight requests share one job | job-manager unit tests plus `media_artifacts_integration.rs` |
| Cached thumbnails and waveforms | artifact cache with `CacheStoreConfig::new(8 MiB entry, 256 MiB total)` | `system_ffmpeg_generates_cached_thumbnail_and_waveform_previews`, `thumbnail_and_waveform_generators_cache_pngs_and_emit_ordered_events` |
| Measured playback | harness `frames` op over the real app | 148-156 frames at 29.9 fps of a 30 fps sequence, build p95 0.66-0.76 ms, gap p95 33.33 ms (debug lane) |

Not done, and stated as such: preview *decode* still runs through the linked FFmpeg
software path (`crates/or_media/src/decoder.rs` has no `videotoolbox` request), and
release-mode pacing has no measurement lane on this host (see §21.4 and §26).

## 26. Packaged end-to-end journey, proven (local, 2026-10-25)

The stop condition asks for the journey in the *packaged* app, not only in the debug
harness. It now runs there, driven by `scripts/packaged_drive.py` (CoreGraphics
clicks and keystrokes, window-only captures) against
`apps/or_app/build/macos/Build/Products/Release/or_app.app`:

| Step | Observed |
| --- | --- |
| Create project | `⌘N` → name `aa`, frame rate 30 fps → folder chosen → `/tmp/or-j18/aa.orproj` (291 bytes) |
| Import media | Import Media → `clip.mp4` listed with the real probe line `mov, mp4, m4a, 3gp, 3g2, mj2 · 64/125 s · 32×24 h264 · Audio …` and a thumbnail |
| Track + insert | `Add video track` → V1; media `+` → Insert Clip (V1 · video, insert, 0/1 → 0/1, 1/2) → timeline `1 track · 1 clip`, inspector `Duration 0.50 s` |
| Preview | viewer paints the decoded frame (32×24 source shown at 485×360), transport `00:00:00:00`, scrubber end `00:00.500` |
| Export | Export → MP4 → Export Here → receipt `Exported aa.mp4 · 3 KB` with `Show in Finder`; top bar `Export complete` |
| Playable output | `ffprobe /tmp/or-j18/aa.mp4` → `h264 32×24` + `aac`, `duration=0.512000`, 2 635 bytes |

What this settles: the release bundle decodes real media, composites the preview
texture, runs the Rust command path, encodes through VideoToolbox and writes a
playable file. Evidence class: INTEGRATION_VERIFIED on the local packaged build.

UX gap found by driving it (recorded, not yet built): adding media to a timeline
with no compatible track still answers "Add a compatible Video or Audio track
first." CapCut-class behaviour is to create the track as part of the insert. The
honest fix is one command sequence in the shell (ensure track → insert clip), not
a second dialog; it is queued in §27 with multi-select leftovers.

## 27. Queue after the packaged journey

Ordered by what a CapCut user would notice first, each with the reason it is not
already done:

1. ~~**Insert creates its own track.**~~ Done in §28.
2. **Multi-select drag.** A set of clips still moves one clip at a time. Moving a
   set needs its own snapping and conflict rules (does a set move ripple? does a
   locked track in the set block the whole gesture?), so the single-clip path stays
   until that case has a test.
3. **Release-mode pacing lane.** The harness can only measure a debug build
   (`flutter test` has no `--profile`), and a profile bundle cannot run the
   integration entry because it lacks the bundled FFmpeg dylibs. A measurement
   needs a dedicated entry point or an external frame-time sampler.
4. **VideoToolbox preview decode.** Only the encoder is hardware today. Preview
   decode goes through the linked FFmpeg software path; moving it to a hardware
   device context is a decoder rewrite with its own fallback story.
5. **Packaged MotionScene dialog drive.** The packaged journey proves create,
   import, insert, preview and export. The MotionScene path was proven in the
   harness only, because its native panels cannot be typed into (see docs/TOOLING.md).

## 28. Insert creates the track it needs (local, 2026-10-25)

Driving the packaged app surfaced a dead end: adding media to a timeline with no
compatible track answered "Add a compatible Video or Audio track first.", so the
owner had to leave the flow, create a track, and come back. CapCut never asks.

What shipped:
- `_showInsertTimelineDialog` takes an optional `onEnsureTrack(kind)` hook. When no
  compatible unlocked track exists it calls the hook once, receives the refreshed
  project and track page, and re-opens itself — the dialog now always has a target
  track to offer.
- `AppShellState._ensureTimelineTrack(kind)` is that hook: one `addTimelineTrack`
  command through the gateway, then `listTimelineTracks`, then `setState` with the
  new revision and tracks. Failure reports `The track could not be added.` and
  leaves the dialog's honest message for the locked-track case.
- Media with unknown streams still gets a track; what the dialog cannot guess is
  the duration, which stays blank until the owner states it (unchanged).

Evidence: widget suite **162 passed** (new test `adding media to an empty timeline
creates the track it needs`, plus the rewritten `insert dialog creates the track
and leaves an unknown duration blank`); harness `scripts/ui_scenarios/auto-track.json`
PASS at 1280x800 with the tree reading `Timeline 1 track · 1 clip` and the clip
`starts 0/1, duration 1/2`; and the packaged release drove the same path end to
end — import, one click on the media row, `Insert Clip` with `V1 · video`, insert,
export → `ffprobe` `h264 32×24 + aac, duration=0.512000, 2 635 bytes`.

## 29. MotionScene in the packaged app, proven (local, 2026-10-25)

Queue item 5 from §27. Driven through the release bundle with the CoreGraphics
driver (`scripts/packaged_drive.py`), clicking only:

| Step | Observed |
| --- | --- |
| Motion tab → **Create illustration video…** | Dialog: Code file / Save video / Title `Illustration` / Frames per second `10` / 640×360 |
| Choose file → Desktop `j23-scene.rs` | Code file row shows `j23-scene.rs` |
| Choose location → Desktop | Save sheet: `Save Illustration-1792929207651.mkv in the selected folder.` |
| Render | Status `Rendered 40 frames at 640×360.`; Motion card appears with `Frames 40 · Rate 10 fps · Size 640×360 · 744 KB` and badge `Rendered` |
| Add to timeline | Notice `Illustration added to the timeline.`, V1 created, `Timeline 1 track · 1 clip`, `Duration 4.00 s`, viewer paints the rendered scene, badge `On timeline`, button becomes `Already on the timeline` |
| Export → MP4 → Export Here | `ffprobe`: `h264 640×360`, `r_frame_rate 30/1`, `aac`, `duration=4.010000`, 138 820 B |

The rendered source itself probes as `ffv1 640×360` + `pcm_s16le` in Matroska, which
is the lossless intermediate the dialog promises; the export re-encodes it through
the normal timeline path, so a MotionScene clip is not a special case in export.

Defect found while driving and fixed: the **Code file** picker offered every file the
platform shows, so a project file could be chosen and carried into the renderer. The
shell now refuses anything that is not `.rs` with `Choose a Rust source file (.rs) to
illustrate.` and keeps the dialog open (`_pickMotionSource` in
`apps/or_app/lib/shell/app_shell.dart`), covered by
`a non-Rust file for the illustration explains the requirement`.

Evidence class: INTEGRATION_VERIFIED on the packaged release this machine builds,
driven by clicks; the project name in these runs reads `asd` because synthetic typing
into a release text field garbles characters (docs/TOOLING.md), which is a driver
limit, not an app defect.

## 30. Hardware and concurrency budget (local, 2026-10-25)

The objective asks for the M5 to be used properly rather than incidentally. These are
the bounds the code actually enforces, with the place that enforces each one, so a
later change cannot quietly widen them.

| Subsystem | Bound | Enforced at |
| --- | --- | --- |
| H.264 encode | Apple VideoToolbox (`h264_videotoolbox`), no CPU x264 fallback for the MP4 profile | `crates/or_media/src/encoder.rs:164` |
| Thumbnail / waveform / proxy jobs | **2 workers**, queue 32, 128 tracked records | `crates/or_app_bridge/src/api/project.rs:685` |
| Artifact cache | 8 MiB per entry, 256 MiB total | `crates/or_app_bridge/src/api/project.rs:686` |
| Preview decode | one worker thread per preview session, serialized through its decode queue | `crates/or_app_bridge/src/preview.rs:2392` |
| Export | one job at a time | `crates/or_app_bridge/src/preview.rs:178` |
| Transcription | one job at a time | `crates/or_app_bridge/src/transcription.rs:98` |
| Worker pool honesty | the pool never runs more jobs than configured | test `worker_pool_never_runs_more_than_the_configured_workers`, `crates/or_core/src/jobs/manager.rs:574` |

Measured, then, with the lane that produced each number:

- **Playback pacing (debug lane, semantic harness, 30 fps sequence):** `frames 148`,
  `fps 29.9`, `build_p50 0.49 ms`, `build_p95 0.66 ms`, `span_p95 0.93 ms`,
  `gap_p95 33.33 ms`, `gap_max 50.0 ms`. `raster_p95` is 0 because the preview is an
  external texture composited by the platform layer, so the Flutter raster phase has
  nothing to do per frame.
- **Export throughput:** a 6.016 s 720p H.264/AAC export completed inside a 486 ms
  observation window with `h264_videotoolbox` selected; a 4.0 s 640x360 MotionScene
  timeline exported to a playable MP4 (138 820 B) from the packaged release.

**PROVEN**: the hardware encoder is the path for MP4 export and it is fast enough that
the export finished inside the poll window; jobs that shell out to FFmpeg are bounded,
so a large library cannot start unbounded processes; every cache has a byte ceiling.

**NOT PROVEN, stated so nobody reads more into the table than it says:**

1. **Release-mode pacing.** `flutter test` has no `--profile`, and a profile bundle
   cannot run the integration entry point because it lacks the bundled FFmpeg dylibs
   (`Library not loaded: @rpath/libavutil.60.dylib`), so the numbers above come from a
   debug build and are a floor on headroom, not a release claim. Measuring release
   pacing needs a small in-app frame-timing recorder driven by the packaged app.
2. **VideoToolbox *decode*.** Only the encoder is hardware. `crates/or_media/src/decoder.rs`
   requests no hardware acceleration, so 1080p playback decodes on the CPU with FFmpeg.
   On this M5 that is comfortable at 30 fps (see pacing above), but it is the next real
   hardware win and it is deliberately not claimed as done.

## 31. Verification sweep (local, 2026-10-25)

One pass over every lane, run back to back on this machine after the auto-track and
MotionScene-source changes, so the whole surface is known to hold at the same commit.

| Lane | Command | Result |
| --- | --- | --- |
| Rust core | `cargo test -p or_core --lib` | **364 passed** |
| Rust bridge | `DYLD_FALLBACK_LIBRARY_PATH=…ffmpeg-8.1.3-macos-image-support/ffmpeg-install/lib cargo test -p or_app_bridge --lib` | **32 passed**, 3 ignored |
| Dart analysis | `flutter --no-version-check analyze --no-pub` | No issues found |
| Widget suite | `flutter --no-version-check test --no-pub` | **163 passed** |
| Real-app journeys | `scripts/or_ui.py run scripts/ui_scenarios/<name>.json` | **18 / 18 PASS** |

The 18 journeys, in the order the sweep runs them: `workspace-chrome`,
`workspace-layout`, `editor-journey`, `compact-editor-journey`, `clipboard`,
`blade-cut`, `clip-reverse`, `clip-freeze`, `viewer-transform`, `audio-cleanup`,
`ripple-trim`, `slip-roll-slide`, `track-reorder`, `overwrite-insert`, `auto-track`,
`illustration-video`, `interaction-latency`, `playback-pacing`.

Between them they drive, through real semantics on the real app: project create, media
import and library search, track creation (empty-state and automatic), insert, select,
copy/paste, split by blade and by playhead, trim, ripple trim, slip, roll, slide,
overwrite, track reorder, speed, volume, fades, transform by pointer and by keyboard,
reverse, freeze, audio cleanup, markers, undo/redo, save/reopen, playback pacing, and
the whole MotionScene render → insert → export journey. The packaged release was proven
separately (§26, §29) with window-only captures and `ffprobe` on the exported files.

## 32. Packaged re-verification at `f3522df` (local, 2026-10-25)

The packaged release was rebuilt at this commit and driven by clicks only
(`scripts/packaged_drive.py`), with window-only captures.

| Claim | Observation |
| --- | --- |
| Release bundle builds and packages | `flutter build macos --release` → 47.9 MB; `scripts/package_local_macos_app.sh` → "Packaged and verified local macOS app" |
| Home renders | sidebar WORKSPACE/APPLICATION, Quick actions, Recent Projects empty state, Developer Preview footer |
| A saved project opens with its media | native open panel → `Documents/or27/proof.orproj` → "Allow Access" grant → editor shows `1 item`, the real probe line (`mov, mp4, m4a … · 64/125 s · 32×24 h264`), V1 with one clip, and the decoded frame painted in the viewer |
| Inspector states the sequence | SEQUENCE `Frame rate 30 fps · Duration 0.50 s · Timeline 1 track · 1 clip`; PROJECT `Name r27 packaged proof · Save state Saved to disk` |
| Export produces a playable file | Export → MP4 → save panel → `Export Here` → receipt `Exported r27 packaged proof.mp4 · 3 KB` with `Show in Finder`, top bar `Export complete`; `ffprobe` → `h264 32×24`, `r_frame_rate 30/1`, `aac`, `duration=0.512000`, 2 635 bytes |

Limits this round confirmed, all host-side rather than product defects:

- A sandboxed packaged app cannot read arbitrary paths, so seeding the recents
  file from `/tmp` shows "No recent projects". Opening through the panel (with its
  security-scoped grant) works.
- Neither synthetic typing (`CGEvent` unicode) nor `⌘V` paste reaches a Flutter
  text field in the release build, so a project cannot be named by the driver.
  Flows must be reached by clicking existing rows; project creation stays an
  owner-driven step in the packaged app.
- The pacing recorder cannot write to `/tmp` from the sandboxed bundle, so the
  release pacing numbers in §30.2 remain a measurement from the unpackaged
  release build (`--dart-define=OR_PACING_LOG=…`).
