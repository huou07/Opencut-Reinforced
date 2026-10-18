---
name: or-ui-ux
description: Design, change or judge any OR user-visible surface — layout, navigation, interaction, typography, colour, feedback, accessibility, and the desktop-versus-compact parity of a journey — and measure the interaction performance that makes it acceptable. Use when adding or changing a screen, panel, dialog, sheet, control or shortcut, when reviewing whether OR's UI/UX is acceptable, or when a UI claim needs evidence. Do NOT use for backend-only or domain-only work (or-edit-feature), a failing build or gate (verify-debug), or as a substitute for the generic review method in the shared ui-ux-review skill, whose evidence-class contract this skill binds to OR.
---

# OR UI/UX

You make one OR surface good and prove it. A build pass, a widget test or a
screenshot each establish less than they look like: name the class you actually
have.

## Authorities

Read, in this order, and do not restate them here:

- [DESIGN.md](../../../DESIGN.md) — the Focused Monochrome visual language. It is
  the design source of truth, not a starting suggestion.
- [UX_ACCEPTANCE.md](../../../docs/UX_ACCEPTANCE.md) — the permanent UI guards and
  the reproduced-and-fixed record. A guard here is not negotiable.
- [PRODUCT.md](../../../docs/PRODUCT.md) — the required behaviour and its maturity
  label for the surface you are touching.
- [QUALITY.md](../../../docs/QUALITY.md) — the accessibility and performance bar
  and its known gaps.
- For the generic evidence-class method, journey checklist and review-versus-fix
  rule, use the shared `ui-ux-review` skill when the session exposes it. This
  skill adds only what is OR-specific.

## What OR's product requires

- **One product, one meaning.** macOS, Linux and Windows present the same
  concepts, terminology, layout intent and command semantics; input and layout
  adapt to platform and window size. Android is an adapted presentation of the
  same model, never a fork. Pixel-identical cross-platform output is not
  required and must not be claimed.
- **One command path.** Presentation changes may not create a second editable
  project state. A pointer gesture, keyboard shortcut, CLI call and agent action
  must resolve to the same validated command.
- **Existing shell slots only.** App Bar, Tool Rail, Left Tool Panel, Viewer,
  Inspector, Timeline Toolbar, Timeline, Task or Status Area, Command Palette,
  Dialog or Mobile Sheet. A new permanent navigation region needs a clear
  justification and an ADR where the workflow says so.
- **No design-system invention.** No marketing surface, hero banner, decorative
  background, glow, gradient clutter or invented metric. Tools, panels, controls
  and clear state.
- **Semantics are part of the interface.** A control an agent or an assistive
  technology must operate exposes a real label, role and action. Inspect the
  existing tree before wrapping anything; decorative paint needs no node.

## Enforceable thresholds

These are the numbers `UX_ACCEPTANCE.md` and `QUALITY.md` already carry. Do not
lower one to make a change pass; fix the surface.

| Requirement | Threshold |
|---|---|
| Text contrast | OR colour text tokens hold at least 4.5:1 against `background` |
| Type size | no font size below 11 px |
| Hit area | at least 24 px on desktop and 44 px in the compact layout |
| Keyboard | every pointer action has a documented keyboard alternative; focus is visible and restorable |
| Compact layout | the editor stays clean at 320x480 and 320x440; taller compact viewports such as 390x844 must not regress |
| Overflow | no clipping or overflow at the named viewports; a short viewport absorbs content instead of squeezing it |

## Evidence classes — never promote one into another

| Claim | Adequate observation | What it does not establish |
|---|---|---|
| Source | `flutter analyze`, `dart format`, widget tests | appearance, focus, a real journey |
| `VERIFIED_SEMANTIC` | the Flutter semantics tree: roles, names, states, actions, keyboard reachability | appearance, a completed journey |
| `VERIFIED_BEHAVIORAL` | real interaction through the app plus the resulting canonical project state | appearance alone |
| `VERIFIED_VISUAL` | screenshot or layout measurement at a named viewport, theme and state | a working journey |
| `VERIFIED_NATIVE` | the packaged or hosted target for the exact source | a debug app is not a package |
| Performance | a bounded workload measured on the named host | screenshots, hosted-runner timing, emulator CPU, or a software GL path can never become a device or GPU claim |

A green widget test is not UX acceptance. A screenshot is not a journey. Xvfb,
llvmpipe and SwiftShader runs are software-path evidence.

## Procedure

1. State the user problem, the target platforms, the input devices and the
   observable result.
2. Pick the cheapest tool that can answer the actual question:
   `scripts/or_ui.py` (when Phase 0 of the current prompt has built it) for
   `screens`, `tree`, `shot`, `act` and a scenario transcript;
   `bash scripts/run_local_lane.sh linux` for the bounded native journey under
   Xvfb; the Flutter widget suite for headless semantics and behaviour.
3. Cover the journey's states: loading, empty, partial, success, failure, and
   its undo, cancel or recovery path. Name the one you did not cover.
4. Implement domain behaviour first, then the desktop presentation, then the
   compact presentation with the same command and state model.
5. Assert the semantics you rely on, at the cheapest adequate layer. Add or
   extend the permanent regression guard when you fix a defect; a UI fix without
   a guard is incomplete.
6. Keep a small visual set for a visual claim: viewport or window, theme, state,
   and the screenshot or measurement. Inspect hierarchy, clipping, overflow,
   contrast and visible focus.
7. Measure the interaction, do not assert it. Report p50 and p95 for the
   command and project paths you touched and the frame time for the render path
   you touched, on the named host, against the budget in
   [QUALITY.md](../../../docs/QUALITY.md). If a row cannot be measured, report
   `UNVERIFIED` with the exact command that would settle it.
8. Update the owning document. A new guard or a changed invariant belongs in
   `UX_ACCEPTANCE.md`; a changed design rule belongs in `DESIGN.md`; a new
   capability's status belongs in `PRODUCT_ROADMAP.md`.

## Performance is an acceptance dimension

An editing surface that is correct but slow is not finished. Playback-critical
decode, audio and render work outranks opportunistic thumbnail, waveform, proxy
and analysis work, and a test must show that an opportunistic load cannot starve
or delay it. A dropped-frame or responsiveness claim needs a measurement, and a
host that cannot produce an honest number says so rather than inventing a
threshold.

## Stop conditions

- A journey you did not exercise is `UNVERIFIED`, not "looks fine".
- A missing semantic node is reported with the smallest remedy, never worked
  around with coordinates.
- Never add a wrapper, lower a threshold, or drop a guard to obtain a pass.
- `ACCEPTED` for a UI claim requires the project's authoritative hosted gate for
  the exact source; a local preflight, emulator or screenshot result never
  becomes it.

## Report

Surface and journey, target platforms and viewports, source identity, each
evidence class actually established, the commands and observed output, the
screenshots or measurements retained, the performance numbers with their limits,
the guard you added, and every state, viewport or platform left `UNVERIFIED` or
`BLOCKED`.
