# ADR 0010 — Motion assets, parameterised scenes and a reusable library

- Status: Proposed (owner request, 2026-10-27)
- Date: 2026-10-27

This record is architecture intent. Nothing described here is implemented yet;
each phase must land with its own tests and measured evidence before a later
document states it as current truth.

## Context

MotionScene V1 can turn a code file or one authored `.ormotion.json` into a
video, and a person or an agent can insert the result as an ordinary clip
(ADR 0007, [motionscene.md](../features/motionscene.md)). It cannot yet:

- keep a scene, a reusable group of objects, or a motion as a named asset and
  find it later;
- change text content, text/object colour or size of an existing asset without
  editing raw JSON from scratch;
- classify assets into a browsable set such as background, people, object,
  nature, animal or vehicle;
- drag an asset into the timeline or into an open scene;
- expose any of that to the assistant as a tool.

The owner asked for all five, plus drag-and-drop editing inside the app. The
constraints do not change: one Rust-owned project truth, one render and encoding
authority, a declarative non-executable scene format, deterministic rendering
with no provider or network call, bounded untrusted input, and materialisation
through ordinary media and timeline commands.

A live `MotionClip` that stays editable in the timeline is explicitly deferred
by ADR 0007 "without a separate schema/architecture gate". This record does not
open that gate; it proposes the asset model and the editing surface that can be
built on the existing materialise-first path first.

## Decision

1. **A motion asset is a versioned declarative document, not a new runtime.**
   A library entry is `.ormotionasset.json` with `format`
   `opencut-reinforced-motion-asset`, `schema_version: 1`, UUIDv4 `asset_id`,
   `name`, `category`, `tags`, `kind`, `canvas`, `duration`, optional
   `parameters`, the payload under `scene` (nodes and animations), and a
   `preview` reference. `or_motion` parses and validates it with the same
   `deny_unknown_fields`, bounds and no-expression rules as a scene. It has no
   script, shader, URL, provider, glob or hidden font lookup.

2. **Four payload kinds cover the requested objects.**
   - `scene` — a complete scene template (a title card, an explainer);
   - `fragment` — a subtree of nodes plus animations, merged into a scene with
     remapped node ids;
   - `motion` — one animation preset (property, keyframes, easing) applied to a
     selected node;
   - `image` — a still PNG/WebP with alpha (background, person, animal, vehicle
     cutouts) referenced by digest, placed as an image node or a scene
     background.

3. **Adjustability is declared, typed parameters — never an expression.**
   `parameters` is a bounded list of `{id, label, type, target, default, min,
   max}`. `type` is `text`, `color`, `number` or `choice`; `target` names a node
   id and a supported field (`text`, `font_size`, `fill`, `size`, `opacity`,
   rotation, asset id). Instantiation substitutes values into a copy of the
   payload and then validates the result as an ordinary MotionScene. Text values
   stay literal bytes; a value that looks like a template is never evaluated.
   Every binding is proven against the payload at save time, so an asset cannot
   ship a parameter that targets nothing.

4. **The library is an app-level store, not project state and not a database.**
   It lives under application support as `motion-library/`: one bounded
   `catalog.json` index, one file per asset, one preview image per asset. The
   Rust application bridge owns reads and atomic writes; the CLI and `or mcp`
   use the same core validation. A project stores only an `asset_id` and a
   content digest as provenance for a materialised clip; absolute asset paths
   never enter a project file, so moving a project cannot break an asset
   reference. Images are copied into the library with a size bound and a digest.

5. **Classification is deterministic, explainable and overridable.**
   The category is a closed set: `background`, `people`, `nature`, `animal`,
   `vehicle`, `object`, `text`, `shape`, `icon`, `chart`, `effect`, `motion`.
   Classification runs in a fixed order — explicit category, then explicit
   tags, then filename/path tokens, then `object` — and records which rule
   fired. A manual choice in the library UI always wins and is stored. No model
   call and no network: an agent may propose a category, but it is shown as a
   suggestion the user accepts.

6. **Editing happens in a Motion surface, and placement stays materialised in
   the first delivery.** The Motion component gains a library column
   (search, category filter, preview), a parameter editor for the selected
   asset, and a scene composer canvas. Dragging an asset onto the timeline
   instantiates and materialises it through the existing `MotionMaterialize`
   request, then imports and inserts an ordinary clip; dragging it into an open
   scene merges its nodes and animations. The placed clip records its asset id,
   parameter values and scene digest, so "Edit asset" reopens the same
   parameters and re-renders an **explicit** replacement. A re-render never
   silently swaps a live `MediaId`. Saving a selection, a whole scene or a single
   animation captures it into the library with a preview and a category.

7. **Agents get the library through the existing MCP boundary.** `or mcp` adds
   `motion_asset_list`, `motion_asset_save` and `motion_asset_insert`, and
   `motion_insert_scene` accepts `asset_id` plus parameter values. Every call
   still attaches to the running app, still validates in `or_motion`, and still
   mutates only through ordinary commands. The agent workspace documents the
   library in its Motion guide.

## Delivery phases

| Phase | Deliverable | Gate |
|---|---|---|
| P1 | `or_motion` asset document, validation, parameter binding and instantiation; classifier with recorded rule; library store in the bridge; `or motion asset list/validate/save/instantiate`; tests | schema and bounds proven without UI; no project file change |
| P2 | Library browse/search/preview and parameter editing in the Motion surface; drag to timeline through materialisation with provenance; save-to-library | real macOS journey: save an asset, reopen the app, find it, adjust text and colour, insert it |
| P3 | Scene composer canvas, drag from library into an open scene, fragment and motion kinds, category override UI, MCP tools | composition served through the evaluator; export unchanged |

A shipped phase updates [motionscene.md](../features/motionscene.md) and the
roadmap; only then does any document describe the asset library as current.

## Alternatives rejected

- **A second database (SQLite) for the library.** Duplicates the project's
  storage concerns, adds migration and backup work, and buys nothing over a
  bounded JSON index with atomic writes.
- **Animate HTML/SVG/Canvas or execute user code.** Reintroduces the procedural
  isolation risk ADR 0007 forbids and creates a second renderer.
- **An expression or template language for parameters.** A declarative typed
  substitution covers text, colour and size without an evaluator to secure.
- **Absolute asset paths in the project.** Moving a project or a library would
  break references; digests plus a library-owned copy are stable.
- **A live MotionClip now.** It changes the project schema, preview pipeline,
  history and export before the asset model is proven; the materialise-first
  path already satisfies drag, adjust and re-render with an explicit replace.
- **Model-based classification in the first delivery.** It adds a provider,
  latency and nondeterminism to a local-first feature; deterministic rules plus
  a manual override are honest and reversible.

## Consequences

- New declarative format, bounds and diagnostics in `or_motion`; new bridge
  commands; a new application-support directory with atomic writes.
- A new Motion surface region and drag-and-drop interaction, which must follow
  the design language and UX invariants.
- Projects gain provenance for materialised assets, but no schema change is
  required for the first delivery. A future live MotionClip still needs its own
  ADR, schema version and migration.
- Agents gain library tools; they remain unable to write project files, reach
  arbitrary paths or run a renderer of their own.
