# Feature — Motion Assets and Library

Last reviewed: 2026-10-28

Scope: the reusable MotionAsset document, its deterministic classifier, the
app-level library, the Motion surface and the timeline/agent insertion paths
([ADR 0010](../adr/0010-motion-assets-and-reusable-library.md)). Placement is
materialise-first: a placed asset is an ordinary rendered video clip, not a live
scene clip.

## Surfaces

| Surface | What it does |
|---|---|
| Motion tool panel | Browse/search/filter the library, import a PNG cutout, adjust parameters, set the category, delete, add to the composition, add to the timeline, save the composition to the library |
| Composition canvas | Select (click, shift-click, marquee), drag, resize, rotate, retitle, recolour, outline, resize text, nudge, add text, add a library object, delete, and undo/redo the draft — inline or in the expanded editor |
| Timeline | Accepts a dragged asset card and appends the materialised clip at the end of the first unlocked video track |
| `or mcp` | `motion_asset_list`, `motion_asset_insert` for the assistant — the fallback the CLI cannot replace |
| `or motion asset` | `validate`, `save`, `list`, `instantiate`, `insert`, `remove`, `import`; every verb takes `--library`, and `insert` renders and places a library asset in the open project |
| `or motion publish` | save a scene as an asset, render it, import it and insert it in one call |

## Asset document

`.ormotionasset.json`, format `opencut-reinforced-motion-asset`, schema version 1.
Data, not a program: `deny_unknown_fields`, no expression language, no network
reference, no provider call.

V1 envelope: `format`, `schema_version`, UUIDv4 `asset_id`, `name`, `category`,
`category_source`, `tags`, `kind`, `canvas`, exact `duration`, typed
`parameters`, and one kind-specific payload.

| Kind | Payload | Reuse |
|---|---|---|
| `scene` | `scene` body: assets, nodes, animations | instantiate as a whole scene |
| `fragment` | `scene` body | instantiate with ids remapped by the asset id |
| `image` | `image` reference plus a `scene` body with an image node | instantiate as a full-canvas cutout with opacity/width/height/rotation parameters |
| `motion` | `preset`: property, keyframes, easing | catalogued and validated; applying it to a node is not implemented yet |

The `scene` body is validated by the same node, asset and animation rules as a
MotionScene. An image asset's scene body must carry an asset whose id equals the
document id and whose path equals `image.file`, and at least one image node must
reference it. Diagnostics use `MOTION_ASSET_*` codes.

## Parameters

- `id`, `label`, `type` (`text`, `color`, `number`, `choice`), `target`
  (`node_id` + one field), `default`, optional `min`/`max`/`choices`.
- Fields: `text`, `font_size`, `fill`, `size_width`, `size_height`, `opacity`,
  `rotation_degrees`, `asset_id`. A field must fit the node kind; two parameters
  cannot write the same node field; the default must equal the targeted field.
- Instantiation substitutes values into a copy of the payload and validates the
  result as an ordinary MotionScene. Text stays literal.
- Imported images get four parameters: `opacity`, `width`, `height` and
  `rotation`.

## Classification

Closed set: `background`, `people`, `nature`, `animal`, `vehicle`, `object`,
`text`, `shape`, `icon`, `chart`, `effect`, `motion`.

Order: explicit → tag token → name token → kind default (`motion` for a motion
preset, otherwise `object`). The winning rule is stored in `category_source`.
Keywords include Vietnamese and English synonyms; matching is lowercase token
comparison plus whole-phrase containment for multi-word synonyms. A manual
choice always wins and is stored as `explicit`.

## Library

```
<root>/
  catalog.json
  assets/<asset_id>.ormotionasset.json
  previews/<asset_id>.png
  images/<asset_id>.png
```

- The root is `<application support>/motion-library`; the Dart layer passes it
  to the Rust bridge, and the assistant session pointer records it so `or mcp`
  finds the same library.
- Saves are atomic (temporary file plus rename); the catalog is bounded to
  4 MiB and 4096 entries; asset ids are UUIDv4 so they cannot name a path.
- `import_image` copies a PNG (16 MiB bound), writes a preview copy, and stores
  an image document. Non-PNG files are refused.
- A composed scene's image assets stay library-relative. When a scene is
  previewed or materialised, `prepare_scene_assets` copies those images beside a
  scratch scene and rewrites the paths, so the ordinary materialiser resolves
  them and no absolute library path enters a project.
- `write_preview` stores a generated PNG preview; the panel shows it as the card
  thumbnail.

## Editing a composed scene

The composition draft is editable in place, through one closed operation set that
is applied in Rust and re-validated before the UI sees a frame:

| Operation | Effect |
|---|---|
| `move` | puts one node where the picture shows it, at a canvas position |
| `move_many` | puts a whole selection at its target positions in one step |
| `resize` | sets a shape's evaluated box (top-left corner and size) |
| `rotate` | sets a node's evaluated rotation in degrees |
| `set_text` | replaces a text node's literal content and drops stale highlight spans |
| `set_fill` / `set_fill_many` | sets one node's, or a selection's, fill colour |
| `set_stroke` | sets or clears a shape's outline (`color: null` removes it) |
| `set_font_size` | sets a text node's type size |
| `delete` / `delete_many` | removes nodes, their children and the animations that targeted them |
| `add_text` | appends a new text node with a caller-chosen unique id |

`motion_scene_edit` takes the scene JSON and one edit document, applies it,
validates the result as an ordinary MotionScene and returns the edited scene, a
still of its first frame and the node boxes that still shows — one round trip per
gesture. `MOTION_EDIT_*` codes name every refusal (unknown node, wrong kind,
duplicate id, unusable value, invalid result, animated size, oversized selection).

### Opening a scene that was authored elsewhere

The Motion panel's **open a scene file** action takes an `.ormotion.json` from
disk through `motion_scene_read`: the file is bounded (8 MiB), parsed with the
same reader as every other scene, validated, and its referenced PNGs are copied
into the app motion library (`images/<id>.png`) before the canvas sees it. The
rewritten scene therefore keeps its images after the first edit, and
materialisation never depends on the folder the file was opened from. A missing
image, an unsafe relative path or an oversized image is a named refusal, not a
placeholder frame that silently drops the file's content.

A scene saved with **Save to library** keeps its whole payload; its row's edit
action reopens it (`motion_scene_from_asset`) on the same canvas, so editing is
not limited to the session that composed it.

### Coordinates are evaluated, not raw

A node under a translation track sits at its anchor **plus** the track's value at
time zero. Every geometry operation therefore takes the position where the picture
shows the node and subtracts that track offset before writing the anchor: dropping
an object where the pointer is does not make it jump by the track's first keyframe.
`resize` divides out a scale track the same way, and refuses with
`MOTION_EDIT_ANIMATED_SIZE` when a width or height track owns the size — a static
value cannot win against a keyframed one, and pretending otherwise would be a
silent no-op.

### Selection, handles and history

- A click selects, shift-click toggles, and a drag on empty canvas draws a marquee
  that selects every box it touches. A group draws nothing, so it is reachable from
  the layer list rather than the canvas.
- Dragging any selected handle moves the whole selection in one edit; the canvas
  shows the pending offset before the commit.
- One selected shape gets four corner grips and a rotate grip. The grips ride the
  rotated box, and a corner keeps the opposite corner still by shifting the centre
  the renderer rotates around.
- The draft keeps its own 32-step undo/redo stack. It is **not** project history:
  undoing a scene gesture never touches the project revision, and the project's
  undo never rewinds a draft gesture. One gesture is one step, even when it is a
  bulk colour or a set of outlines that needs several bridge calls.
- The motion library's list has its own multi-selection: tick rows, then add them
  all to the composition or remove them together.

The node boxes are pointer targets, not glyph measurements: a text node's box is
an estimate of its advance at `MOTION_TEXT_HIT_ADVANCE` per character, while the
renderer still shapes real text with `cosmic-text`. They are read from the
evaluated first frame, so a track's translation and rotation are reflected where
the picture puts a node, while the edit still writes the static anchor they apply
to.

A drag is committed once, on release; a text edit is debounced. Neither can write
a scene the validator refuses, because the op set is typed and every result is
validated before the bridge returns it.

## Rendering and composition

- `or_render` decodes PNG assets to straight-alpha RGBA (16 MiB encoded,
  64 megapixel decoded) and draws them with bilinear sampling, opacity and
  rotation. A missing image still draws the visible placeholder rather than a
  silent hole.
- The Motion surface keeps a draft scene. "Add to scene" appends one asset with a
  unique id prefix and a canvas offset; two placements of the same asset cannot
  collide. The preview is a real still rendered by the same evaluator and
  rasteriser.
- "Add to timeline" materialises the draft (or the selected asset) at
  1920×1080/30 fps into `<project>/generated/`, then imports and inserts it with
  the ordinary media and timeline commands.
- Re-rendering never silently replaces a clip: inserting a new render is an
  explicit action. The panel keeps the last insert as an **On timeline ·
  W×H · N frames** row with **Render again**, which re-materialises the same
  source and appends a second clip. The row belongs to the panel's session
  (`apps/or_app/lib/motion/motion_insert_log.dart`) and is cleared when a project
  is attached or closed; it is not project state and does not survive a restart.
- A successful materialisation writes `<output>.provenance.json` beside the
  video: kind (`motion-scene` or `motion-asset`), scene id and name, asset ids,
  parameter values, canvas, frame rate and a scene digest. It is a sidecar, not
  project state; a later editor can find the library assets without a schema
  change.

## Agents

`or motion asset list --attach project.json` pages the catalogue; `or motion asset
insert --attach project.json --asset-id ... [--values JSON]` instantiates,
prepares the image files, materialises through the app, imports and inserts; and
`or motion asset save`, `instantiate`, `import`, `remove` and `validate` are pure
reads and writes of the library directory, with no project and no render. `or
motion publish --attach project.json` saves an authored scene as a scene asset
with a preview, then renders and inserts it; `--no-library` skips the save.

The library folder is `<application support>/motion-library`, and both pointers
record it as `motion_library_path`: the workspace `project.json` the CLI attaches
to, and the assistant session pointer (`<application support>/assistant/
session-pointer.json`) that `or mcp` reads. Every asset verb therefore resolves the
same folder from `--attach`, and `--library PATH` overrides it for one call.

The MCP tools (`motion_asset_list`, `motion_asset_insert`, `motion_asset_save`,
`motion_insert_scene`) remain as the fallback when a CLI verb is unavailable.
The workspace guide and persona route to the CLI first.

## Bounds

4 MiB per asset, 64 parameters, 16 tags of 64 bytes, 128-byte name, 4096
nodes/animations, 16-deep groups, 64 KiB per text value, 32 choices per
parameter, 4096 entries, 16 MiB per image, 64 megapixels decoded, 256 KiB per
edit document, 1024 nodes per selection edit, 32 draft undo steps, 200,000
materialised frames.

## Gaps

- No live timeline MotionClip; an inserted asset is video plus a provenance
  sidecar, not an editable scene object, and the app does not surface the sidecar
  in its inspector yet.
- `publish` accepts vector scenes; a scene with image assets is still saved from
  the app, because publishing would have to copy those images into the library.
- A dragged card appends at the track end; it does not place at the drop time or
  the playhead.
- Motion presets (`kind: motion`) are validated and catalogued but cannot be
  applied to a node yet.
- Image import accepts PNG only; no JPEG/WebP and no alpha trimming.
- The canvas edits fill, outline, size, rotation and text; it has no per-corner
  radius, no gradient, no layer reordering and no grouping inside the draft.
- Resizing a shape whose width or height is animated is refused rather than
  rewritten track-by-track; a rotation track is compensated instead.
- The draft history is per session and per draft: closing the panel or clearing
  the scene does not persist an undo stack, and two windows editing the same draft
  are not merged.
- No per-project library, no download/marketplace, no tag editor.
- Previews render one frame at time 0; animation timing is not previewed.

## Where

- `crates/or_motion/src/asset.rs` — document, validation, classifier, instantiation, composition
- `crates/or_motion/src/edit.rs` — the closed scene-edit operation set and node boxes
- `crates/or_motion/src/library.rs` — catalog, image import, previews, scene asset preparation
- `crates/or_render/src/motion_image.rs` — PNG decode and image drawing
- `crates/or_app_bridge/src/api/motion_library.rs` — bridge surface
- `crates/or_cli/src/commands.rs`, `crates/or_cli/src/mcp.rs` — CLI and agent tools
- `apps/or_app/lib/motion/` — controller, panel, canvas editor and drag model
- `apps/or_app/lib/screens/editor_shell_preview_screen.dart` — panel and timeline drop
- `scripts/ui_scenarios/motion-library.json`, `scripts/ui_scenarios/motion-scene-edit.json` — real-app journeys

## Related

- [motionscene.md](motionscene.md)
- [assistant.md](assistant.md)
- [ADR 0010](../adr/0010-motion-assets-and-reusable-library.md)
