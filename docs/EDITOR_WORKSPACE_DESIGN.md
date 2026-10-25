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
