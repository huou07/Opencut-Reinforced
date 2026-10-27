# UI/UX Acceptance Invariants

Last reviewed: 2026-10-26

Scope: behaviour that must not regress, and what counts as evidence for a UI
change. Do not add an item here without a check that can fail.

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
- Transport and timeline keys resolve through the single shell
  `Shortcuts`/`Actions` map, dispatch the same gateway calls as the pointer
  controls, and do not fire while a text field holds focus.
- The mobile tool dock exposes the same semantics as the desktop rail; selection
  is carried by fill and border, not colour alone.
- Every visible action works or is visibly unavailable — no dead-end controls, no
  clipped actions at supported window sizes.
- Simple mode is a visibility setting over one state and command model.
- The editor's right rail exposes Inspector and Assistant as two tabs over one
  project state; switching tabs is presentation only, and the assistant never
  writes project state outside the shared command path.
- A shell shortcut never consumes a key a focused text field needs: the key map
  is split by focus, and native menu items carry no modifier-less equivalents.

## Proven interaction surfaces

Project create/open/save/reopen and cross-track move; import and library search;
pointer trim/move/split/reorder, marquee select and group move with one undo;
blade tool; speed badge and inspector speed; copy/paste; reverse and freeze; text
and captions; effect presets; MotionScene render-and-insert; in-app scene
editing (drag an object, retitle, recolour, resize text, add text, add a library
object, delete) inline and in the expanded editor; assistant chat edits and
assistant-driven MotionScene insertion through the real harness; native menus;
export receipt; pacing recorded from the app.

Each of these is backed by a named scenario or widget test in
`scripts/ui_scenarios/` or `apps/or_app/test/`. Re-run the affected one when you
change that area.

## Known gaps

Packaged-build driving cannot type into native AppKit panels on this host; those
journeys click rows instead. Hardware decode and Android audio remain absent.

## Related

- [design-language.md](design-language.md)
- [operations/ui-harness.md](../operations/ui-harness.md)
