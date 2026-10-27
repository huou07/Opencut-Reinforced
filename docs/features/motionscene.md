# Feature — MotionScene (Deterministic Code-to-Video)

Last reviewed: 2026-10-27

Scope: turning source code into a short, deterministic illustration that can be
previewed, materialised, inserted into an ordinary timeline and exported.

## Contract (ADR 0007)

MotionScene is a versioned, **declarative, non-executable** data format
(`.ormotion.json`). It has no script, shader, HTTP, environment, glob, shell,
provider or hidden font lookup, and evaluation makes **zero** model calls.

V1 envelope: `format`, `schema_version`, UUIDv4 `scene_id`, `name`, exact
`duration`, `canvas`, typed `assets`, `nodes`, `animations`.

- Time is nonnegative finite `RationalTime` on a half-open `[0, duration)` local
  timeline with exact-time random access.
- Primitives: `Group`, `Text`, `Rectangle`, `Ellipse`, `Line`/`Arrow`,
  renderer-gated `Polyline`, `Image`.
- Animation: typed translation, scale, rotation, opacity, bounded geometry, and
  supported fill/stroke values with a small locked easing set.
- Charts and diagrams start as typed templates over those primitives.
- Any randomness must be explicit and seeded from scene/source identity; V1 is
  otherwise deterministic.
- Text uses the shared `cosmic-text` bundled-font path — not a second text engine.
- Image nodes draw a decoded PNG asset resolved relative to the scene file, with a
  16 MiB encoded / 64 megapixel bound; a missing image keeps the visible
  placeholder.

## Canonical path

```
MotionScene → validate → evaluate exact time → RenderSnapshot
            → or_render/wgpu → preview or materialisation/export
```

Preview and materialisation share the evaluator. The materialisation-first rule
means: render persistent generated media, preserve source/asset provenance,
register it through normal media commands, and use ordinary timeline clips. A
re-render never silently replaces an existing canonical `MediaId`, and a
timeline-referenced materialised asset is not cache-only.

## Producers and surfaces

- Authored in the app's Motion panel (render facts, insert, render again).
- External agents and templates are first-class producers. `or mcp` exposes
  `motion_insert_scene` to the assistant; it materialises through the app
  runtime, then imports and inserts through ordinary commands. CLI verbs are
  `or motion validate`, `inspect`, `render` and `materialize`; only
  `materialize` needs a live app because the CLI ships no FFmpeg libraries.
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
