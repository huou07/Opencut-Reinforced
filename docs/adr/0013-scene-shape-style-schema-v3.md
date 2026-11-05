# ADR 0013 — Scene shape style, schema version 3 (rounded corners and gradients)

- Status: Implemented — schema version 3 landed 2026-11-05
- Date: 2026-11-05

This record is architecture intent. The fields, the edit operation, the renderer
path and the inspector are implemented; current behaviour and gaps are in
[motionscene.md](../features/motionscene.md).

## Context

Version 2 gave scene text a real typographic range but left every shape a hard
rectangle with one flat fill. Cards, chart panels, lower thirds and title plates
therefore all shared the same silhouette: a sharp-edged block. Reviewing a
rendered explainer next to a comparable product clip, the difference was not
type — the type was already good — but the shape language: everything in the
other clip had a corner radius and a soft two-stop gradient, and everything in
ours had a 90° corner and one colour.

The format is deliberately small and closed (`deny_unknown_fields`, no
expression language, no CSS), and that must stay true. The question was where
the missing expressiveness belongs: a new node kind, a paint model, or two
properties on the shape that already exists.

## Decision

1. **Two optional properties on shape nodes, nothing new to compose.**
   `corner_radius: Option<f64>` (canvas pixels) and
   `gradient: Option<{from, to, angle_degrees}>`. Both are absent in every
   version 1 and 2 document, and absent means exactly the old rendering, so the
   schema version moves to 3 without a migration and old scenes stay
   byte-identical.

2. **The renderer clamps, the validator refuses nonsense.** A radius above half
   the shorter side is clamped when drawn so a resize cannot produce an invalid
   scene, while a negative or non-finite radius and a non-finite gradient angle
   are validation errors (`MOTION_INVALID_CORNER_RADIUS`,
   `MOTION_INVALID_GRADIENT`) reported with a path.

3. **A gradient belongs to the node's own box.** The angle is measured
   counter-clockwise from +x like rotation, and the stop positions are the
   box's projection onto that direction, so a gradient rotates with the node and
   preserves its look when the node is resized. There are exactly two stops and
   no stop positions: a third stop would be a paint model, and the format's job
   is to stay smaller than one.

4. **One edit operation, `set_corner_radius`.** Gradients are authored in the
   scene file rather than stepped in the inspector, because choosing two colours
   and an angle is a picker, not a gesture; the inspector exposes the radius
   because it is the value a designer nudges while looking at the frame.
   Shape-only operations use a `shape_node()` helper that returns
   `MOTION_EDIT_NOT_SHAPE` for text, group, arrow and image nodes.

## Rejected alternatives

- **A new `card` node kind.** Would duplicate rectangle plus radius, and every
  future node kind would need its own radius, gradient and animation wiring.
- **A general paint model** (multiple stops, radial gradients, per-stop offsets,
  images as fills). That is a vector editor's model; it enlarges the validation,
  the renderer and the agent contract for a benefit no shipped template needs.
- **Faking corners with an alpha mask layer.** Not seekable, not deterministic,
  and it doubles the layer count of every card.
- **Cubic-bezier easing in the same version.** It needs named control points or a
  list of four numbers in a field that is currently a string enum; that is a
  schema-shape change with its own determinism tests, so it stays deferred rather
  than smuggled in with shapes.

## Consequences

- A scene can express a rounded, gradient-filled card in two fields, and the
  agent guide teaches the four-property recipe instead of forbidding gradients.
- The renderer pays one signed-distance test per pixel only for rectangles that
  set a radius, and one lerp only for pixels that are not stroke.
- Version 3 stays readable by this build; a version 4 document is still refused
  rather than guessed at.
- Gradients are authored in the file/agent path, not in the inspector yet. That
  is a real limitation and is recorded as such in
  [motionscene.md](../features/motionscene.md).
