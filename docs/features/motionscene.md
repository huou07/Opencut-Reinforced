# Feature — MotionScene (Deterministic Code-to-Video)

Last reviewed: 2026-10-28

Scope: turning source code into a short, deterministic illustration that can be
previewed, materialised, inserted into an ordinary timeline and exported.

## Contract (ADR 0007)

MotionScene is a versioned, **declarative, non-executable** data format
(`.ormotion.json`). It has no script, shader, HTTP, environment, glob, shell,
provider or hidden font lookup, and evaluation makes **zero** model calls.

V1 envelope: `format`, `schema_version`, UUIDv4 `scene_id`, `name`, exact
`duration`, `canvas`, typed `assets`, `nodes`, `animations`, and an optional
`camera`.

- Time is nonnegative finite `RationalTime` on a half-open `[0, duration)` local
  timeline with exact-time random access.
- Primitives: `Group`, `Text`, `Rectangle`, `Ellipse`, `Line`/`Arrow`,
  renderer-gated `Polyline`, `Image`.
- Animation: typed translation, scale, rotation, opacity, bounded geometry, and
  supported fill/stroke values with a small locked easing set.
- Camera: optional `camera.keyframes` with `pan_x`, `pan_y`, `zoom` (0.25–4) and
  `rotation_degrees`, interpolated with the same easing set and applied to every
  resolved instruction. Absent camera means identity, so old scenes render
  unchanged; assets may carry the camera body too.
- Charts and diagrams start as typed templates over those primitives.
- Any randomness must be explicit and seeded from scene/source identity; V1 is
  otherwise deterministic.
- Text uses the shared `cosmic-text` bundled-font path — not a second text engine.
  ASCII lines use Inter; a line containing non-ASCII (Vietnamese) uses Be Vietnam
  Pro, so stacked marks get the display face. Each line is measured against the
  canvas and shrunk (down to 60 % of the
  requested size) or wrapped before it is rasterised, so a long Vietnamese title
  cannot run off the frame.
- Image nodes draw a decoded PNG asset resolved relative to the scene file, with a
  16 MiB encoded / 64 megapixel bound; a missing image keeps the visible
  placeholder.

## Canonical path

```
MotionScene → parse → validate → evaluate exact time → RGBA layer
            (CPU rasteriser) → PNG still (panel preview)
                             → shared encode path (materialisation/export)
```

Rasterisation is the `or_render::motion` CPU pass: the scene's vector
instructions are painted into one RGBA layer, which then takes an ordinary path —
a PNG still for the Motion panel, or the shared encoder for a materialised file.
ADR 0007's `or_render/wgpu` step is the compositor that carries decoded media and
timeline overlays; a motion scene reaches it only after materialisation, as an
ordinary video clip. Rasterising scenes on the GPU is a separate, measured change,
not something the current path claims.

A frame sequence is produced by a bounded worker pool rather than one thread per
sequence: one core is left to the visiting writer, the pool is capped at eight
threads, and the finished canvases waiting ahead of the writer are bounded at
192 MiB, so a long scene is limited by the encoder rather than by memory. Frames
still reach the writer in index order and are byte-identical to a one-thread
sweep; parallelising only changes wall time.

Preview and materialisation share the evaluator. The materialisation-first rule
means: render persistent generated media, preserve source/asset provenance,
register it through normal media commands, and use ordinary timeline clips. A
re-render never silently replaces an existing canonical `MediaId`, and a
timeline-referenced materialised asset is not cache-only.

## Producers and surfaces

- Authored in the app's Motion panel (render facts, insert, render again). A
  composed draft is editable on the canvas — move, text, colour, size, add and
  delete — through the typed operation set in `or_motion::edit`; see
  [motion-assets.md](motion-assets.md#editing-a-composed-scene).
- External agents and templates are first-class producers. CLI verbs are
  `or motion validate`, `inspect`, `evaluate`, `layout`, `render`, `from-code`,
  `materialize` and `publish`; `materialize` and `publish` need a live app
  because the CLI ships no FFmpeg libraries. `publish` also saves the scene to
  the motion library, imports the render and inserts the clip, so the agent's
  default path is the CLI. `or mcp` still exposes `motion_insert_scene` as the
  fallback.
- Materialised output appears in the library and on the timeline like any media.

## Bounds

Code and its highlight theme are untrusted input: bound the parser, never execute
the sample, and bound asset size, node count, duration and evaluation work.

## Deferred

Live `MotionClip` in the timeline without a separate schema/architecture gate,
WebMotion (isolated browser sidecar for HTML/CSS/Canvas/WebGL), Remotion/Motion
Canvas as runtimes, and generative motion providers.

A reusable, parameterised motion asset format and app-level library exist as
**P1** of [ADR 0010](../adr/0010-motion-assets-and-reusable-library.md): the
document, typed parameters, classifier, store and `or motion asset` verbs are
implemented in [motion-assets.md](motion-assets.md). There is no Motion surface,
drag and drop, bridge command or MCP tool yet, and the placed clip remains
materialised video.

## Where

- `crates/or_app_bridge/src/api/motion.rs` — bridge surface
- `crates/or_render/` — evaluation into the render spine
- `apps/or_app/lib/` — Motion panel and insertion flow
- ADR: [docs/adr/0007-declarative-motion-scenes-and-procedural-isolation.md](../adr/0007-declarative-motion-scenes-and-procedural-isolation.md)

## Related

- [motion-assets.md](motion-assets.md) — the reusable asset document, classifier and library
- [text-captions.md](text-captions.md) — shared text path
- [export.md](export.md) — exporting the combined edit
