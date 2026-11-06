# UI/UX Acceptance Invariants

Last reviewed: 2026-11-04

Scope: behaviour that must not regress, and what counts as evidence for a UI
change. Do not add an item here without a check that can fail.

- A destructive confirmation names its consequence, not just its question:
  what is closed, what is not written, which recovery copy is deleted.
  Rounds 85-86 fixed two prompts that asked "continue?" while the branch also
  discarded the recovery checkpoint and aborted a running export.

## Evidence requirement

A user-visible capability is accepted only with real interaction through the app
plus the resulting canonical state, or a deterministic headless guard where the
app cannot express the question. A screenshot alone proves a visual state, not a
journey. See [operations/verification.md](../operations/verification.md).

## Must hold

- New/open/close/switch/exit preserve dirty-state guards, explicit save, recovery
  inspect/apply/discard, and exact-base conflict behaviour.
- Playback, seek, scrub and frame step never mutate the project or bump the
  revision; frame stepping uses the exact project frame policy.
- Text tokens hold ≥4.5:1 against background, raised and surface; focus ≥3:1 with
  translucent indicators composited before measuring.
- No UI text style below 11 px; the theme carries the design type scale.
- Trim handle hit area ≥24 px desktop / ≥44 px compact, with a restrained painted
  bar.
- A divider is either backed by a working resize handler or renders only the
  structural rule; a drag-looking grip without a handler is a defect.
- Every transient message goes through the shared snackbar.
- Enrolling or cloning a voice asks for the same explicit acknowledgement on
  every provider (installed, cloud or local server) before the clip is used.
- User-facing copy never states an internal identifier (clip id, media id,
  job id, artifact key, checkpoint path); it names the consequence instead.
- Copy names the device or the installation, never one operating system: no
  "this Mac" in a Windows or Linux build, no hardcoded "Starting DeepSeek
  Harness", and shortcut hints come from `shortcutLabel`/`deleteShortcutLabel`.
  Guarded by `apps/or_app/test/platform_copy_test.dart`.
- Transport and timeline keys resolve through the single shell
  `Shortcuts`/`Actions` map, dispatch the same gateway calls as the pointer
  controls, and do not fire while a text field holds focus.
- The mobile tool dock exposes the same semantics as the desktop rail; selection
  is carried by fill and border, not colour alone.
- Every visible action works or is visibly unavailable — no dead-end controls, no
  clipped actions at supported window sizes. A narrow viewer drops the optional
  skip/speed controls instead of clipping the time readout; full screen keeps
  them.
- The canvas is a real, undoable project control, not a badge: the viewer header
  menu sets or clears it, and the export dialog offers a resolution that keeps
  that shape. A control that only displays the texture size is a defect.
- Full-screen preview carries the same transport as the docked viewer — play or
  pause, ±5 s, a scrub bar, elapsed and total time, speed and exit — and its
  keyboard path is `Space`, arrows, `Shift`+arrows and `Esc`.
- Pressing play at the content end replays from zero. A speed other than 1× turns
  preview audio off and says so; it never plays pitched or drifting audio.
- Simple mode is a visibility setting over one state and command model. Its
  next step names one action derived from the open project, never from a
  tutorial counter, and disappears once the work exists; dismissing it is a
  separate preference that Settings can restore. Guidance never changes
  layout: the step is a toolbar item, so opening it cannot resize a pane, move
  an existing control, or shift a clip into a transient snackbar's hit area. Guided mode changes the export
  dialog's order, not the export fields: an intent maps onto the same profile,
  quality and resolution triple, and **Advanced settings** reveals those fields.
- Composition guides are a preview overlay. They draw after presentation, are
  never handed to the renderer, and `G` toggles them only when no text field has
  focus. An exported frame with a guide burned in is a defect.
- The editor's right rail exposes the Assistant by default and the advanced
  Inspector as an optional second tab over one project state. The Inspector is
  hidden unless the user enables **Show inspector panel** in Settings or opens
  it from the palette. Switching tabs is presentation only, and the assistant
  never writes project state outside the shared command path.
- A shell shortcut never consumes a key a focused text field needs: the key map
  is split by focus, and native menu items carry no modifier-less equivalents.

- **A field that refuses a keystroke names its accepted range.** Silent clamping
  reads as a broken field. Where the field cannot yet reject with a message, it
  shows the range as helper text (round 175, inspector audio fields).
  The hint and the formatter must read the same computed bound; a parallel
  table drifted by 10x once (rounds 177-180).


## Proven interaction surfaces

Project create/open/save/reopen and cross-track move; import and library search;
pointer trim/move/split/reorder, marquee select and group move with one undo;
blade tool; speed badge and selection-strip speed/volume (the same values remain
in the advanced panel); copy/paste; reverse and freeze; text
and captions; effect presets; MotionScene render-and-insert; in-app scene
editing (drag an object, retitle, recolour, resize text, add text, add a library
object, delete) inline and in the expanded editor; assistant chat edits and
assistant-driven MotionScene insertion through the real harness; native menus;
export receipt; pacing recorded from the app; the guided next-step step and the
intent-first export dialog (`guided_steps_test.dart`, `widget_test.dart`
"guided export dialog"/"codec choice"/"G toggles composition guides"), with the
macOS run recorded in [ui-harness.md](../operations/ui-harness.md); setup through
Settings (`resources-setup` scenario, which also proves the editor preferences
sit in their own panel); the compact window reaching the media panel through the
tool dock sheet (`widget_test.dart` "compact window reaches the tools"); and a
guided export that really encodes, probed and decoded independently.

Each of these is backed by a named scenario or widget test in
`scripts/ui_scenarios/` or `apps/or_app/test/`. Re-run the affected one when you
change that area.

## Known gaps

Packaged-build driving cannot type into native AppKit panels on this host; those
journeys click rows instead. Hardware decode and Android audio remain absent.
- **A bound key is a listed key.** The shell's `Shortcuts` map and the sheet in
  `apps/or_app/lib/shell/shortcut_sheet.dart` are checked against each other by
  `test/shortcut_sheet_test.dart`; adding a binding without a row, or listing a
  key that is not bound, fails the suite. Evidence: `?` opens the sheet in the
  real app (`scripts/ui_scenarios/shortcut-sheet.json`, `OR_UI=PASS`).


## Related

- [design-language.md](design-language.md)
- [operations/ui-harness.md](../operations/ui-harness.md)
