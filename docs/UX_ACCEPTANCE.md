# OR UI/UX Acceptance Invariants

Last reviewed: 2026-10-20

## Purpose

This checklist separates the frozen HTML prototype reference from behavior
verified in the production-direction Flutter application and behavior required
by future execution checkpoints. Do not move an item into **PRODUCTION
IMPLEMENTED** without an automated or hosted acceptance check. Before changing
a related area, inspect the applicable section and re-run affected checks.

## PROTOTYPE REFERENCE

The frozen prototype is a UX/product reference only. Its simulated state is
not production architecture or implementation evidence.

- Home recent project can open Editor.
- Projects screen can open Editor.
- New Project → Create enters Editor.
- Template → Use Template enters Editor.
- Import Media can enter the intended project/editor context.
- Exiting Editor returns to the intended destination.
- The prototype shows the Viewer as the dominant upper workspace region.
- Prototype left panel and Inspector are collapsible/resizable.
- Prototype timeline is vertically resizable.
- Prototype Play/Pause occupies a stable fixed position.
- Rapid prototype Play/Pause clicks do not move the hit target.
- Prototype timeline ruler seeking works.
- Prototype timeline playhead dragging works.
- Prototype Previous/Next Frame uses the displayed project FPS.
- Prototype common editing toolbar commands show consistent icons.
- Prototype More opens a visible advanced-command menu.
- Prototype More menu commands remain reachable and clickable.
- Prototype menus are not visually clipped by their toolbar/container.
- Prototype mobile flow shows a preview, transport, timeline, horizontal scroll,
  touch scrubbing, and tool sheets.
- Prototype theme switching does not alter navigation or editor structure.
- Prototype custom theme import rejects unsafe/arbitrary CSS or script input.
- Prototype self-test passes.
- Prototype browser console has no unexpected errors or warnings caused by the
  prototype.

These items are retained as reference guards and are not claims that playback,
frame stepping, or mobile playback-like behavior is implemented in production.

## PRODUCTION IMPLEMENTED

The following behavior is implemented and covered by the current Rust/Flutter
headless or widget-level checks:

### Project and shell

- New/open project workflows create or open a real Rust-owned project.
- New Project derives a safe `.orproj` filename suggestion from the entered
  title; the native save dialog lets the user choose another location or name.
- Home, Projects, Templates, Asset Library, Settings, and Editor shell routes
  remain reachable through the Focused Monochrome shell.
- Create/open/close/switch/exit flows preserve dirty-state guards, explicit
  save, recovery inspection/apply/discard, and exact-base conflict behavior.
- Projects exposes create/open, recent-project return, and active-project
  controls; unimplemented search, filter, sort, and view controls are not shown.
- Recent project history is stored in the app-support directory and shown from
  Home and Projects after a successful project open. Linux, macOS, and Windows
  packaged journeys reopened from that list after app restart. Android's real
  DocumentsUI journey validated the persisted SAF URI, retained read grant,
  URI-derived working-copy path, fresh-picker rebind, and provider
  synchronization. Saving a renamed project updates its recent display name
  without changing its last-opened time. Exact-SHA evidence: Platform
  Verification [38328757217](https://github.com/huou07/Opencut-Reinforced/actions/runs/38328757217),
  attempt 2. Clean-machine install/upgrade/uninstall is verified separately at
  `53de6d51c6d45c7ec7ae0a2989455972118406c1`: Platform Verification
  [38570870756](https://github.com/huou07/Opencut-Reinforced/actions/runs/38570870756)
  and Repository Hygiene
  [38570870938](https://github.com/huou07/Opencut-Reinforced/actions/runs/38570870938)
  installed, upgraded, launched and removed the Linux `.deb` and the macOS and
  Windows ZIP packages while preserving user data. Those are debug packages, not
  stable installers.
- A new empty project does not inherit another project's canonical timeline.
- Home focuses on creating, opening, and returning to projects. The visual-only
  Editor Shell Preview is reachable from Settings → Advanced and is not offered
  as a project action.
- A new project with no imported media offers an Import Media action in the
  empty timeline alongside the existing track choices. It uses the same
  platform picker and permission path as the Media panel on desktop and compact
  layouts.
- Add to Timeline pre-fills the exact timeline start at the current project
  content end, so another full media item can be appended without entering
  timing values. The timing field remains editable for deliberate placement.
- Viewer, left-panel, Inspector, timeline, status, dialog, and command-palette
  slots follow the Focused Monochrome workspace layout; no marketing banners,
  scenic backgrounds, glow-heavy decoration, or fake metrics are accepted.
- Registered product screens and registered Editor panels remain reachable;
  developer features remain discoverable under Settings → Advanced / Developer.

### Timeline and project state

- The workspace displays canonical Video/Audio/Text/Caption track order and
  bounded track read models with derived V#/A#/T#/C# labels.
- Video/Audio tracks can be added and empty tracks removed; clips can be
  inserted at exact times, moved between same-kind tracks, explicitly deleted,
  duplicated, trimmed, split, and ripple-deleted through existing Rust
  commands. Duplicate inserts after the selected clip while retaining its
  exact source range.
- Timeline action menus expose exactly Move, Duplicate, Trim, Split, Delete,
  and Ripple Delete. Dialogs show current timing, exact fields, and
  Rust-generated split IDs where applicable.
- A pointer or keyboard action selects at most one clip. Ctrl+D (or Cmd+D on
  macOS) duplicates it through the existing insert command; Escape clears the
  selection. Delete/Backspace asks for confirmation before deleting.
- Track visibility, audio mute, lock, and solo controls send the typed Rust
  track-state command and refresh from canonical state. Locked tracks reject
  clip edits and media drops; visible/muted and solo flags remain independent.
- Supported media can be dragged from its dedicated library handle onto a
  compatible unlocked timeline lane. Rust validates the insertion, and the
  stream-specific exact duration is preserved. The accessible Add to Timeline
  dialog remains available.
- Timeline zoom and horizontal viewport are presentation-only. Zoom stays
  between 12.5% and 800%, Fit returns to the content view, and changing view
  state does not change `ProjectRevision`.
- Ripple Delete confirms that only later clips on the selected track shift left;
  other tracks and global markers do not move.
- Timeline dialogs refresh from Rust after a command and never apply optimistic
  clip geometry.
- The ruler displays bounded persistent marker read models; Add, Move, Rename,
  and Delete dispatch the existing Rust marker commands and refresh the view
  from the canonical project state.
- Marker-bearing workspaces refresh after attached `project_changed` events,
  save/reopen, and recovery handling without weakening exact-base conflict
  guards.
- Flutter stores only disposable bounded read pages, gesture-local ghosts,
  dialog state, and presentation state; canonical state remains in Rust.
- Attached CLI edits refresh the open Flutter workspace through ordered
  `project_changed` events.
- Clip pages are bounded and loaded on demand; exact rational values remain
  exact across the bridge and are converted to doubles only for layout.

### Phase 6E1 pointer editing

- The project timeline exposes a visible default-on Snap toggle with an
  accessible label and active state; changing it does not persist a project
  setting or change `ProjectRevision`.
- Dragging a clip body creates only a temporary presentation ghost, targets
  same-kind lanes, retains the source lane outside valid lanes, and rejects an
  opposite-kind lane without dispatching a mutation.
- Small left and right clip-edge handles have pointer priority and expose
  accessible start/end trim labels. Pointer deltas are quantized once to the
  nearest 1 ms from the original exact time; repeated updates do not accumulate
  rounded values.
- Release performs at most one drop-time Snap V1 query and then uses the
  existing Rust move or trim command. A successful snap may show temporary cyan
  feedback before the command completes.
- Snap resolution uses the fixed `1/8`-second threshold and canonical timeline
  zero plus all other clip boundaries, including clips outside loaded Flutter
  pages. Revision, project-instance, project-switch, disposal, and attached
  CLI invalidation guards reject stale results without retry.
- Exact Move and Trim dialogs remain available for arbitrary exact `NUM/DEN`
  values.

### Phase 6E2B marker UI and Snap V2

- Typed marker read models and bounded marker paging cross the existing bridge;
  Flutter does not maintain a second marker document.
- The marker ruler exposes accessible marker handles and focused monochrome
  Add, Move, Rename, and Delete actions using the existing Rust commands.
- Flutter pointer editing uses the canonical marker-aware Snap V2 query with
  revision, project-instance, project-switch, disposal, and attached CLI stale
  guards; marker snaps show visible ruler feedback before the existing move or
  trim command is dispatched.
- Widget and bridge mapping checks cover bounded pages, exact rational values,
  marker commands, and marker-aware snap presentation without launching the
  native application locally.

### Preservation and accessibility

- Keyboard navigation, visible focus, usable touch targets, and status that is
  not communicated by color alone remain required for implemented UI.
- Simple Mode, when present, is only a visibility setting over the same state
  and command model and does not remove core workflows.
- The presentation contract is enforced by deterministic headless widget tests
  rather than by review, so the following cannot silently regress:
  - `OrColors` text tokens hold at least 4.5:1 against `background`,
    `backgroundRaised` and `surface`, and the focus indicator holds at least
    3:1 against the same surfaces, with translucent indicators composited
    before measuring.
  - `ThemeData` carries the DESIGN.md type scale, and no UI `TextStyle` uses a
    font size below 11 px.
  - The editor trim handle carries a hit area of at least 24 px on desktop and
    44 px in the compact layout, while the painted bar stays restrained.
  - A divider is either backed by a working resize handler or renders only the
    structural rule; a drag-looking grip without a handler is a defect.
  - Every transient message goes through the shared `OrSnackBar`, so the shell
    and the editor cannot diverge.
  - Editor transport and timeline keys resolve through the shell's single
    `Shortcuts`/`Actions` map and dispatch the same `ProjectGateway` preview
    calls as the pointer transport controls; a command does not fire while a
    text field holds focus.
  - The mobile tool dock exposes the same semantics as the desktop tool rail,
    and selection in both is carried by a fill and a border rather than by
    colour alone.

### Reproduced and fixed (2026-10-18, compact layout)

Four compact-layout defects reproduced through the running app at 320x440 with
`scripts/or_ui.py`, all four fixed with permanent widget guards, plus a fifth
found by the same guard once the fourth no longer masked it.

- **The workspace header overflowed by 44 px.** `_ProjectWorkspaceHeader` held
  the project name, the revision text, the state badge and the action row in one
  fixed Row. Short of 560 px the narrow presentation now keeps the name and the
  actions and drops the duplicated revision/badge metadata instead of clipping
  it; Save and the shell's own dirty indicator still carry that state.
- **The insert dialog's track dropdown overflowed by 46 px.**
  `DropdownButtonFormField` laid out its selected item at its natural width
  inside a 320 px dialog. Every dropdown in the editor now sets `isExpanded:
  true` and ellipsizes its labels.
- **A transient message covered the mobile tool dock and stole its taps.** The
  floating message sat over the 58 px dock, so the tap that opens the compact
  media library never arrived and the workflow stalled until the message
  expired. `OrSnackBar.show` takes a `bottomInset`, the shell and the editor pass
  the dock height in the compact layout, and a message is capped at two lines so a
  long one cannot push the surface past a short viewport.
- **The compact viewer overflowed by 17 px once a clip gave the preview a
  range.** The trigger is `hasRange`: the viewer's scrub strip (ruler, clock
  labels, slider) only appears once the preview has a content end, so the defect
  was invisible until an insert succeeded. Two causes, both fixed: `AspectRatio`
  inside the tight-height `Expanded` surface sized itself from the width and
  overflowed the height (the surface is now fitted to both dimensions and
  letterboxes), and the strip did not fit a 149 px panel (a short panel now keeps
  the surface, the scrub control and the transport and drops the decorative ruler
  and its clock labels, which the live readout and the timeline ruler carry). The
  compact journey runs end to end and the 320x480/320x440 viewports are clean.

**Measured at 320x440, and not a defect but a limit:** the compact editor gives
the viewer about 114 px once the toolbar, timeline and dock are laid out, so the
preview surface is one pixel tall. It is clean — no clipping, no overflow, every
control present — and it is not *usable* as a preview either. That allocation is
recorded here as a known limitation with its number; changing the compact flex
allocation is a design decision, not a bug fix.

Two harness defects were found by the same work and fixed: a tap delivered in the
same frame as `ensureVisible` was hit-tested against the pre-scroll position (the
Add Audio tap reported success and added nothing), and readiness was being
inferred from an enabled flag that a disabled button can also present. Scenarios
now wait for a user-visible state (the transient message clearing) before acting.

### Reproduced and fixed (2026-10-20, packaged desktop preview)

The 2026-10-19 Developer Preview desktop packages opened to a black window.
The cause was packaging order, not rendering: each desktop job built the app,
ran `flutter test integration_test/core_bridge_test.dart -d <platform>`, and
then packaged the same output directory. The test run rebuilds that directory
with the integration-test listener as its Dart entrypoint, so the shipped
bundle never called the application `main()` and the window never drew a frame.
The released Linux bundle's kernel carried `flutter_test_listener`,
`package:integration_test`, and `integration_test/core_bridge_test.dart`.

- macOS, Windows and Linux stage the application bundle before the test run,
  package the staged copy, and run the guard again on the packaged application.
- `scripts/check_app_bundle.py` inspects every `kernel_blob.bin` in a bundle and
  refuses the test listener and integration-test markers;
  `scripts/test_check_app_bundle.py` pins that classification inside
  `check-repo.sh`. The guard rejects the 2026-10-19 packages.
- A 1280x800 audit of the running app also found and fixed four visible
  defects: inspector text fields collapsed until the floating label overlapped
  its value, the tool rail clipped `Templates`/`Captions`/`Transitions`, the
  timeline toolbar hid `Export Captions` and `Add Caption` without a scroll
  affordance, and the top bar printed `Ctrl/⌘ K` on Linux and Windows. The
  empty inspector also asked to "unlock the selected track" when nothing was
  selected.

Local evidence: the Linux debug app under Xvfb, the `scripts/or_ui.py` editor
journey at 1280x800 and compact journey at 320x440, and a clean `flutter
analyze`. That is developer evidence; hosted platform verification and native
acceptance remain separate.

### Reproduced and fixed

- The compact editor overflowed its column by 31 px at a 320x480 viewport.
  This reproduced headlessly rather than only on a device. The proximate
  cause was the viewer's "No media loaded" placeholder, not the transport:
  the compact 4:2 viewer/timeline split plus the fixed transport left the
  viewer surface only 55.7 px tall, and the icon-and-text column needed more.
  A 320x440 viewport additionally squeezed the fixed 34 px timeline view
  controls into an 8.7 px strip. Both now absorb a short viewport: the viewer
  placeholder keeps its icon and exposes the message through semantics when
  the surface is shorter than 56 px or narrower than 120 px (matching the
  existing texture-unavailable path), and the timeline view controls are
  omitted when their region is shorter than 60 px. A headless regression
  mounts the editor at 320x480 and 320x440; taller compact viewports such as
  390x844 remain clean. This is source implementation with local developer
  evidence (Flutter 3.47.7, a clean analyzer, and the widget suite — run
  `flutter test` for the current count rather than a number written here);
  exact-SHA hosted verification is recorded in
  [PRODUCT_ROADMAP.md](PRODUCT_ROADMAP.md).
- The create-project journey at the emulator-sized 320x640 viewport, which the
  Android SAF acceptance run uses, had no working overflow guard: the committed
  test installed its own `FlutterError.onError`, and `takeException()` reads only
  the framework's recorded error, so its assertion could not fail. The test now
  uses the shared create-project helper and the framework error handler; it
  mounts the empty workspace at 320x640 and passes with no overflow. This
  corrects a vacuous guard rather than reporting a reproduced defect.

### Permanent harness guards (added 2026-10-18)

Widget tests answer a narrow question about a screen they mount. These guards
answer the journey question on the running app, on this host, through the real
command path. They are developer evidence: `ACCEPTED` still requires the
authoritative hosted gate for the exact source.

- **Editor journey** —
  `python3 scripts/or_ui.py run scripts/ui_scenarios/editor-journey.json`
  drives Home -> Projects -> create -> a rejected import (the failure state) ->
  import a synthetic fixture -> add a track -> insert a clip -> open and close the
  clip dialog -> keyboard duplicate (`Ctrl+D`) -> keyboard undo (`Ctrl+Z`) ->
  keyboard frame step (`.`) -> keyboard save (`Ctrl+S`) -> leave the editor ->
  reopen the saved project -> open the export dialog -> cancel it. Every action
  names a widget key, a semantics label (exact or regex), visible text, a role, or
  a dialog-scoped control; **no step names a screen coordinate**, and the harness
  has no option that injects canonical project state. A control reached only by
  pointer in an earlier revision now has to have a semantic identity to be
  driven at all.
- **Render-exception guard** — the runner fails a step if the framework recorded
  an exception while it ran, so a `RenderFlex overflowed` or a failed assertion
  cannot pass as green. This is what makes the journey a layout guard and not
  only a behaviour guard.
- **Compact viewports** —
  `python3 scripts/or_ui.py run scripts/ui_scenarios/compact-editor-journey.json`
  runs the same commands at 320x440 through the compact presentation (the media
  library is a bottom sheet opened from the mobile tool dock there), and the same
  file with `--viewport 390x844` proves the taller compact viewport has not
  regressed. The compact journey asserts the same semantics and the same absence
  of render exceptions.
- **Loud failure** —
  `python3 scripts/or_ui.py run scripts/ui_scenarios/wrong-selector.json` names a
  control the app does not have and must report `FAIL` with status `not_found`
  and exit 1. `scripts/test_or_ui.py` (run by `bash scripts/check-repo.sh`)
  asserts that behaviour, the semantic-selector-only rule for every shipped
  scenario, and that a malformed scenario is a usage error rather than a pass.
- **Observation on demand** — `python3 scripts/or_ui.py screens` lists the screens
  the running app exposes and dumps each one's semantics and screenshot;
  `tree`, `shot` and `act` answer the same questions for one point in time. On
  2026-10-18 `screens` found five destinations (Home, Projects, Templates, Asset
  Library, Settings) at both 1280x800 and 320x440.
- **Semantic state assertions** — a scenario can assert `expect_state` on a named
  control (`enabled`, `disabled`, `selected`, `checked`, `focused`, `toggled`),
  read from the semantics tree rather than from a widget field.
- **Positional assertions** — a scenario proves where an edit *left* a clip, not
  only that the app accepted the command: the timeline clip's semantics label
  carries the exact span (`starts 1/4, duration 1/4, spans 1/4 to 1/2`) and, for a
  media clip, its source window (`source 1/8`), so a screen reader announces the
  edit precisely and `scripts/or_ui.py` can assert it. Fixing a defect or adding a
  move without a positional assertion is incomplete: the four editing journeys
  (`ripple-trim.json`, `slip-roll-slide.json`, `editor-journey.json`,
  `compact-editor-journey.json`) all assert spans rather than acceptance.
- **Measured interaction** — a scenario can wrap sub-steps in
  `{"op": "measure"}` and the transcript records p50/p95/min/max with the
  statement that it is wall clock on this host through the software path. A timed
  step can set `"settle": false` so the following wait observes the visible
  result; without that the measurement includes the harness's own settling budget
  and measures the instrument rather than the app. The current numbers are in
  [QUALITY.md](QUALITY.md#measured-interaction-latency-software-path), and they are
  developer evidence, never a device or GPU claim.

## PRODUCTION IMPLEMENTED — workspace rebuild (local, 2026-10-25)

Delivered against docs/EDITOR_WORKSPACE_DESIGN.md §9–19 on the owner's macOS
machine. Evidence class is stated per row; none of it is a release claim.

| Surface | Evidence |
| --- | --- |
| One bar per zone: project name, save dot, undo/redo, project menu, export, palette | widget suite + `workspace-chrome.json` PASS at 1440x900 |
| Tabbed library (Media/Text/Audio/Motion) instead of an icon rail | `workspace-layout.json` PASS at 1440x900, 1280x800, 1024x768 |
| One timeline control row (zoom, Fit, Snap, `+` create menu) | widget suite + `blade-cut.json` |
| Contextual inspector: project facts when nothing is selected | `workspace-chrome.json`, inspector keys `inspector-project` |
| Viewer direct manipulation, one command per gesture | `viewer-transform.json` PASS (X 12.2 %, scale 124.4 %, rotate 4.622°, Alt+arrow nudge) |
| Library search without touching the project | widget test + `media-search`/`media-count` keys |
| Blade tool (C/V) cutting at the pointer | `blade-cut.json` PASS (0/1→1/4, 1/4→1/2 halves at 2x) |
| Clip speed badge | `blade-cut.json`, inspector `Playback speed 2×` |
| Copy/paste clip via keys, Edit menu and palette | widget test + `clipboard.json` PASS (0/1 then 1/2) |
| Effect/transition presets filling fields before Apply | widget test, `+event-preset-*` chips |
| MotionScene panel: render facts, insert, render again | `illustration-video.json` PASS (40 frames at 320x180, `On timeline`) |
| Native macOS menus over the same verbs | widget test (structure only, not AppKit rendering) |
| Export receipt naming the file, with Show in Finder | build + widget suite; reveal not driven end-to-end |
| Playback pacing measured from engine timings | `playback-pacing.json`: 29.9 fps, build p95 0.76 ms, gap p95 33.33 ms (debug lane) |

Known gaps that stay out of "done": release-mode pacing (the semantic harness
cannot run a profile build), VideoToolbox preview decode, reverse/freeze frame,
multi-select marquee.

## PRODUCTION REQUIRED FUTURE

These items are required by the execution plan but are not production claims
today.

> **Status correction, 2026-10-17.** Part of Phase 7 has shipped since this
> section was written. `docs/RELEASE.md` claims desktop play/pause, exact seek,
> scrubbing and frame stepping, and a texture-backed viewer surface exists in
> `apps/or_app/lib/screens/editor_shell_preview_screen.dart`. Read the Phase 7
> bullets below as the remaining acceptance conditions for that work, not as
> evidence that the feature is absent. The earlier contradiction between this
> section and RELEASE.md was a documentation defect; the source and the hosted
> evidence decide, and anything not covered by them stays `UNVERIFIED`.

### Phase 7 playback and preview

- A real viewer consumes an approved native/external texture or equivalent
  zero-copy surface contract; full-rate frames do not travel as copied Dart
  byte arrays.
- Play/Pause, transport, seek, scrubbing, playhead, ruler, and Previous/Next
  Frame use runtime work and do not mutate `ProjectRevision`.
- Frame stepping uses the exact project frame/time policy, and stale frames may
  be dropped without corrupting canonical state.
- Exact seek and scrub preserve the requested rational timeline time; display
  labels may use floating-point conversions only.
- Play and frame-step controls stay unavailable until an explicit sequence
  rate is set through the existing validated project setting command.
- Timeline content end is half-open and includes audio-only duration; an empty
  timeline has no playable frames, and playback never loops.
- Preview behavior remains consistent with export semantics.

### Phase 8 Desktop MVP

- Selection, duplicate, track enabled/locked/solo, timeline zoom, direct
  media-to-timeline insertion, transform/crop/opacity, basic text/manual
  captions, basic gain/pan/fades, basic transitions/effects, export, autosave,
  recovery, save/reopen, and preview/playback are accepted at their locked
  checkpoints.
- For 8C, selecting an unlocked Video clip exposes numeric position, scale,
  rotation, anchor, crop, and opacity controls in the Inspector. Values stay
  within the typed project ranges; crop edges cannot remove the full width or
  height. Apply and Reset use the canonical project command, preserve clip
  content and exact timing, and update the preview. Undo/redo and save/reopen
  retain the same visual settings.
- For 8D, the timeline offers Text and Caption tracks. Add Title and Add
  Caption create typed clips at the preview playhead on an unlocked matching
  track; adding captions is manual and does not invoke transcription. Users can
  edit text, exact duration, bundled Inter size, weight, alignment, and color.
  Move, duplicate, and delete continue through the canonical timeline command
  path. Preview text is rasterized from bundled Inter rather than Flutter text
  widgets or host-installed fonts.
- SRT/WebVTT caption import previews each cue's exact millisecond range and
  editable plain text, along with any styling/placement loss. Cancel leaves the
  project unchanged; Import applies the reviewed cues to one new Caption track
  through one revision-checked canonical command. Caption export offers SRT or
  WebVTT and saves through the platform file boundary. Packaged macOS, Windows,
  Linux, and Android journeys must verify these controls before this capability
  is marked accepted.
- The Media panel offers `Import Media` and `Import Folder`. Folder selection
  imports supported descendant files through the ordinary media import flow,
  preserves per-file success/failure feedback, and explains empty or oversized
  folders above the shared 64-file batch bound. Desktop uses the native folder
  picker; Android uses the SAF folder picker and retains its read grant. Mark
  this journey accepted only after packaged desktop and Android folder-import
  checks pass.
- For 8E, selecting an unlocked Audio clip exposes gain in dB, pan in percent,
  and exact rational fade-in/fade-out times in the Inspector. Apply and Reset
  use the canonical clip-update command and preserve exact clip timing. Desktop
  playback consumes bounded prepared stereo buffers; missing output hardware
  does not prevent video-only preview.
- For 8E, selecting an unlocked Video, Text, or Caption clip exposes the
  closed brightness, contrast, saturation, and blur controls plus transition-in
  and transition-out choices for Cross Dissolve, Fade Through Black, and Wipe.
  Transition durations are exact rational times bounded by clip duration.
  Preview applies these typed settings to the selected layer through the shared
  render path.
- Automatic captions are not a Desktop MVP acceptance item; they belong to
  Phase 10.
- Complex linked clips, grouping, nested timelines, and multicamera are
  advanced Phase 13 behavior unless re-promoted by a plan amendment.

### Mobile and later product behavior

- Android SAF project/media access, MediaCodec/native-buffer/Vulkan/wgpu paths,
  resource-aware fallback, and mobile-native editing UX are accepted only in
  Phase 9 hosted/device checks.
- Production mobile preview, transport, timeline visibility, horizontal scroll,
  touch scrubbing, and tool sheets require the real Android product journey;
  prototype behavior is not evidence. Playback pauses when the app leaves the
  foreground and remains paused when it resumes.
- At 320 px width, the mobile top bar keeps project, export, cancel, and command
  controls usable without overflow. Save/export status remains available through
  an accessible tooltip when its full text does not fit.
- The compact viewer's loading and unavailable states fit their allocated space
  during recovery and surface initialization, including very small transient
  viewports; status stays available to screen readers without a clipped message.
- Desktop export offers a smaller WebM VP9/Opus delivery profile and a
  lossless Matroska FFV1/PCM profile. The format choice is clear before the
  destination picker; progress, cancellation, failure, and completed output
  use the same export status path.
- Android export uses the shared project/timeline semantics, reports progress
  and failure in that status area, supports cancellation while active, and
  publishes a completed Matroska file to the selected DocumentsUI destination.
  It must not offer WebM until the packaged Android runtime includes and
  verifies the VP9/Opus encoders.
- Media import lets desktop users select several files in the platform picker.
  Android requests multi-select through DocumentsUI, takes a read grant for
  every returned document, and imports each through the same project command
  path. A failed item is reported in a combined result while other selected
  items continue; project changes already accepted remain valid.
- If an imported source has disappeared, the user sees that the selected file
  could not be found and that its source needs to exist; existing project media
  stays untouched.
- Each media row offers Relink Source. Relink accepts exactly one selected
  source, re-probes it, preserves the existing MediaId and timeline references,
  and updates the row after the command succeeds. A missing target, duplicate
  source, incompatible media, or failed probe leaves the project unchanged;
  Undo and Redo restore the previous and replacement sources respectively.
- Android media import uses seekable SAF descriptors. The typed `content://`
  source remains in project data; probing uses the packaged FFmpeg runtime and
  the same import-matrix validation as desktop. Hosted acceptance selects two
  real provider documents and verifies both imported media records and the
  measured command calls. The Android descriptor registry remains bounded at
  64 sources per preview surface.
- Android recovery acceptance writes a valid unsaved edit to the app-private
  project working copy, closes the live session, force-stops the app process,
  and reopens the same persisted SAF document through DocumentsUI. The actual
  recovery dialog must apply the checkpoint, remove its sidecar, and restore a
  preview frame before the journey passes.
- Phase 10–16 requirements follow their locked phase documents for captions,
  AI, templates, dubbing, advanced editing, generation, community packaging,
  plugins, and interchange.

Future regression guards must name their checkpoint and remain aligned with
`docs/archive/execution/PLAN.json`; do not weaken an implemented guard to make a new
feature pass.
