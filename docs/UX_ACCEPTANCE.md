# OR UI/UX Acceptance Invariants

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
- Home, Projects, Templates, Asset Library, Settings, and Editor shell routes
  remain reachable through the Focused Monochrome shell.
- Create/open/close/switch/exit flows preserve dirty-state guards, explicit
  save, recovery inspection/apply/discard, and exact-base conflict behavior.
- A new empty project does not inherit another project's canonical timeline.
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

## PRODUCTION REQUIRED FUTURE

These items are required by the execution plan but are not production claims
today.

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
- Android export uses the shared project/timeline semantics, reports progress
  and failure in that status area, supports cancellation while active, and
  publishes a completed Matroska file to the selected DocumentsUI destination.
- Media import lets desktop users select several files in the platform picker.
  Android requests multi-select through DocumentsUI, takes a read grant for
  every returned document, and imports each through the same project command
  path. A failed item is reported in a combined result while other selected
  items continue; project changes already accepted remain valid.
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
`docs/execution/PLAN.json`; do not weaken an implemented guard to make a new
feature pass.
