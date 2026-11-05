# ADR 0012 — Scene typography, schema version 2 and explicit text boxes

- Status: Implemented — schema version 2 landed 2026-11-05
- Date: 2026-11-05

This record is architecture intent. The fields, the renderer path, the typed edit
operations and the inspector are implemented; the current behaviour and its gaps
are in [motionscene.md](../features/motionscene.md).

## Context

MotionScene version 1 could place text but not set it. A text node chose its own
font (Inter for ASCII, Be Vietnam Pro for anything else), always drew at
`TextWeight::Regular`, always aligned to the start, always used one line height,
never tracked, and never had a box: it measured against the canvas edge, shrank
until it fit one line, and only then wrapped. Every title in every scene
therefore looked the same weight and the same rhythm, which is the opposite of
what a professional title needs — a display line is set tight and heavy, a
subtitle is set light with air.

The project's own text clips shared the limitation: `TextFormatting` carried a
font, a size, a weight and an alignment, but not the line height or the tracking
that make a multi-line block readable. The renderer had one line-height constant
(`LINE_HEIGHT_FACTOR = 1.32`) and no tracking at all.

Three constraints shaped the fix:

- MotionScene is declarative, deterministic, seekable and `deny_unknown_fields`.
  A new field must be readable by a future build and must not change how an old
  document renders.
- The vector-to-pixel pass is a layer producer inside the existing render spine.
  It must not become a second text engine, and it must not learn to load fonts
  from the network, from the project, or from the OS.
- The Flutter side drives edits through the typed `or_motion::edit` operations.
  It may not mutate scene JSON itself.

## Decision

1. **Schema version 2 is additive.** A text node may carry five optional fields:
   `font` (`inter` | `be_vietnam_pro`), `font_weight` (`regular` | `medium` |
   `semibold` | `bold`), `text_align` (`start` | `center` | `end`),
   `line_height` (0.5–3.0, a multiple of the type size) and `letter_spacing`
   (-50–200 points). Absent means the version 1 behaviour, so a version 1
   document parses, validates and rasterises to the same pixels it did before.
   `MIN_MOTION_SCENE_SCHEMA_VERSION = 1` keeps the reader accepting old files;
   a future version is refused rather than guessed at, as before.

2. **`size` on a text node is now an explicit box.** Width caps the measured
   line, height bounds the block, and the block anchors at the top of the box.
   Without a box, the old rule is unchanged: one line shrinks to 60 % before it
   wraps. Alignment needs a box to mean anything, and a fixed box is what makes a
   two-line title resolve to the same pixels on every frame instead of reflowing
   as the box grows.

3. **The metrics live in `or_core::TextFormatting`, not in MotionScene alone.**
   `line_height_permille` (default 1320, 800–3000) and
   `letter_spacing_milli_points` (default 0, -5000–20000) are serialized with
   `skip_serializing_if` so a document written before the fields existed encodes
   byte-for-byte as before. One formatting contract means the projector text path
   and a scene cannot drift into two typography models — the same reason ADR 0007
   put scene text on the cosmic-text path in the first place.

4. **Edits stay typed and JSON-driven.** `set_font_family`, `set_font_weight`,
   `set_text_align`, `set_line_height` and `set_letter_spacing` extend
   `MotionSceneEdit`, reject non-text nodes with `MOTION_EDIT_NOT_TEXT`, refuse
   out-of-range values with `MOTION_EDIT_INVALID_VALUE`, and are re-validated by
   the same `validate_scene` as every other edit.

5. **The app reads typography from the draft document.** The generated bridge
   views predate these fields, and regenerating them is a separate, riskier
   change with its own toolchain requirements. The Motion inspector therefore
   reads the five fields from `draftSceneJson` — the same document the edits go
   back through — instead of widening the generated view. No second model is
   introduced; there is one document and two read paths over it.

## Alternatives rejected

- **A font manager with arbitrary font files.** It would break determinism on a
  machine without the font, add per-font licence evidence, and multiply the text
  cache. The two bundled OFL faces cover Latin and Vietnamese; a broader library
  is a separate product decision, not a scene field.
- **An HTML/CSS or browser text layout path.** A second text engine with its own
  measurement, and a runtime in the render path that ADR 0007 excludes.
- **Regenerating the Flutter bridge for six values.** The generated views are a
  mechanical mirror; forcing a regeneration in the same change as a schema bump
  makes review and rollback harder for no user-visible gain.
- **Gradients, rounded corners and cubic-bezier easing in the same version.**
  Each needs new rasteriser coverage and new determinism tests; typography is the
  quality gap users see first, so it goes first.

## Consequences

- A version 1 scene needs no migration, and a version 2 scene with no fields set
  is indistinguishable in output from version 1.
- Rasterising a boxed text node now depends on the box, not only on the canvas,
  so a scene must set `size` to get wrapping at a chosen measure.
- `TextFormatting` round-trips the two new fields, but the bridge view that the
  text dialog uses does not carry them yet, so the dialog writes the defaults.
  That limit is stated in [text-captions.md](../features/text-captions.md); it is
  not a silent loss, because nothing in the app sets them on a project text clip.
- Evidence: `crates/or_motion` (version 1 compatibility, version 2 resolution,
  unknown key refusal, range rejection, typed edit operations),
  `crates/or_render` (boxed wrapping, alignment, tracking, weight ink),
  `crates/or_core` (default-omitted serialization, historical decode, range
  rejection) and `apps/or_app/test/motion_library_test.dart` (the inspector edits
  the scene and the document changes).
