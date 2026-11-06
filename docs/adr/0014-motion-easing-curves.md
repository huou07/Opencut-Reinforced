# 0014 — Custom easing curves in the scene schema

Status: Implemented (schema version 4, 2026-11-06)

## Context

Schema versions 1–3 offered four named easings — `linear`, `ease_in`,
`ease_out`, `ease_in_out` — implemented as quadratics in `or_motion::ease`.
That is enough to keep a move from looking mechanical, but not enough for the
motion a creator expects from a modern editor: a slow lead-in that snaps, a
settle that overshoots slightly and returns, the family of curves designers pick
from a graph. Every scene therefore looked "computer generated" in the same way,
and the agent workspace could not express the difference between them.

The set is documented as "small and locked, per ADR 0007", so changing it is a
schema decision, not a tuning change.

## Decision

- `schema_version` becomes **4**. Versions 1–3 keep parsing and render exactly
  as they did, because the new field is optional and its absence means the
  named easing.
- An animation gains `easing_curve: [p1x, p1y, p2x, p2y]` — a CSS-style cubic
  Bézier. When present it replaces `easing`; the named field stays so a scene
  that names one keeps working.
- The camera accepts the same field, because a move and its camera should be
  authorable with the same vocabulary.
- Validation refuses a curve whose x components are outside `0..=1` or whose
  numbers are not finite, with `MOTION_INVALID_EASING_CURVE` located at
  `/animations/{i}/easing_curve` or `/camera/easing_curve`. The y components
  are free: that is what allows an overshoot.
- `or_motion::ease_curve` solves the curve by bisection on `t` for a fixed 60
  iterations. Bisection is chosen over Newton's method deliberately: the
  iteration count cannot depend on the data, so two hosts stepping the same
  frame get bit-identical values, which is the same determinism rule the
  preview/export parity lane relies on.

## Rejected alternatives

- **A named catalogue** (`ease_out_back`, `ease_in_expo`, …) — cheaper to type
  but still a closed list, and the next request would be a schema change again.
- **A formula string** — an expression language inside the Rust core, which the
  security boundary forbids and which no reviewer can bound.
- **Storing sampled points** — loses the curve, bloats the file, and makes an
  edit ("a little more snap") impossible without re-sampling.
- **Replacing `easing` outright** — would break every scene already written.

## Consequences

- A scene can express spring-like settles and snappy leads. The inspector does
  not draw a curve editor yet; the field is authored in the file or by the
  agent, and the guide carries two worked examples.
- Two places must agree on the field's meaning: `or_motion` (sampling and
  validation) and the agent guide. Both are updated in the same commit, and
  `crates/or_motion/src/lib.rs` tests cover the sampling, the refusal and the
  determinism.
- A future curve editor is additive: it writes the same four numbers.
