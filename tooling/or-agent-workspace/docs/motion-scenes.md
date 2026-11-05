# Motion scenes

Scope: turning code or a declarative `.ormotion.json` scene into an ordinary,
editable video clip through the app's deterministic MotionScene renderer. No
provider or network call happens here. For several scenes, splitting and
parallel renders, see [motion-production.md](motion-production.md).

## Two inputs

| Input | Use |
|---|---|
| A source file | `tools/or motion publish --attach project.json --source work/sample.rs --name "Parser" --json` turns the code into a highlighted illustration. |
| A scene file | `tools/or motion publish --attach project.json --scene scenes/title.ormotion.json --json` renders an authored scene. |

Pass exactly one of `--source` and `--scene`. One call validates the scene,
renders it through the app, imports the result, inserts an ordinary clip and saves
the scene as a reusable library asset (see below). It is slow because every frame
is encoded; do not start a second render while one is running and do not retry
after a timeout without checking the timeline.

`publish` accepts vector scenes only. A scene with image `assets` is refused: the
app's Motion panel publishes those, because it copies the images into the library.

Useful optional arguments: `--title` (the generated scene's title for a `--source`
run), `--name`, `--category`, `--at`, `--track`, `--width`, `--height`,
`--frame-rate` (`30/1`), `--output`, `--library`, `--no-library`. When `--at` is
omitted the clip is appended after the target track; when `--track` is omitted the
first unlocked video track is used, and one is created when the project has none.
`--output` must be absolute and defaults to `<project folder>/generated/`.
`materialize` remains available when only a file is wanted; it never imports or
inserts.

## The motion library folder

The app keeps its library at `<application support>/motion-library`, and the
workspace `project.json` records that path as `motion_library_path`. Every motion
verb attached through the pointer therefore finds the same library the app shows:
`tools/or motion asset list --attach project.json --json` needs no other flag, and
`publish` saves the scene as a reusable asset by default. `--library PATH`
overrides the folder for one call; pass `--no-library` when a library copy is
genuinely not wanted. Do not copy or edit library files yourself.

If a pointer written by an older build has no library entry, the CLI says so:
reconnect the assistant so the app rewrites `project.json`, or pass `--library`.

## Authoring a scene

Start from the scaffold, then edit the JSON:

```sh
node tools/motion-scaffold.mjs --name "Title" --out scenes/title.ormotion.json \
  [--width 1920] [--height 1080] [--seconds 4]
```

Validate before rendering:

```sh
tools/or motion validate --file scenes/title.ormotion.json --json
```

`tools/or motion inspect` summarises nodes and animation counts;
`tools/or motion layout --file ... --at 1/2 --json` lists every node's evaluated
box (position, size, rotation, opacity, text, animated flag) at one exact time —
this is how you read the composition, without a screenshot. `tools/or motion
evaluate --file ... --at 1/2` shows the resolved paint instructions. Neither
touches a project or the encoder. `tools/or motion render` rasterises one frame to
a PNG when a still is enough. `tools/or motion from-code` writes a scene from a
source file without materialising it, so the agent can adjust the generated scene
first.

`tools/or motion materialize --attach project.json --source PATH --output
<absolute .mkv>` renders to disk through the live app but does not import or
insert the result; use it only when a file is wanted, never as an insertion path.

## Design rules (learned from the slideshow failure)

- **Three to five fills, one of them a gradient.** Reuse a small palette. A
  two-stop gradient on a card or a bar reads as depth; stacked bands and
  per-node rainbow fills read as a mistake. A gradient is one node property,
  never twenty rectangles.
- **Never a dead frame.** Keep two or three animations running across the whole
  duration — a slow drift, a pulse, a wipe — not only entrance fades. If the
  middle of the scene is static, it reads as a slide.
- **Type first.** Titles are at most six or seven words per line at 1080p, at
  least 28 px, with a 5 % margin. Two OFL faces are bundled: `inter` (default,
  ASCII) and `be_vietnam_pro` (the display face for Vietnamese, so stacked marks
  keep their shape). Set `font` only when you mean to override the automatic
  ASCII/non-ASCII choice. Write NFC text and never strip diacritics to fit.
  Wrapping and shrinking to 60 % are safety nets, not a layout choice.
- **Build hierarchy with the type fields, not with size alone.** A title card
  reads as designed when the display line is `font_weight: "bold"` with
  `letter_spacing` around -1 to -2, and the supporting line is `"medium"` with
  `line_height` 1.4–1.5. Give a multi-line block an explicit `size` box so it
  wraps at the measure you chose and aligns; without a box, `text_align` has
  nothing to align inside and a long line is measured against the canvas edge.
- **Cut on movement.** Overlap the end of one idea with the start of the next;
  avoid a fully empty frame between beats.
- **A camera move is the cheapest life.** A slow push-in (`camera.keyframes`
  with `zoom` 1 → 1.08, a small pan) keeps every frame moving; keep zoom ≥ 1 so
  a flat background never shows the canvas edge.
- Read the result with `tools/or motion layout --file ... --at MID --json`; the
  boxes are what a viewer sees, so check that text stays inside its canvas.

## A card that does not look unfinished

The difference between a slide and a product shot is usually four properties,
not forty nodes:

- a rectangle with `corner_radius` between 8 and 24 at 1080p,
- a `gradient` from a dark tint to a slightly lighter one at the same angle for
  every card in the scene,
- a text box whose `size` is inside the card with 24–32 px of padding,
- one animation on the card (`scale` 0.98 → 1 or `opacity` 0 → 1) so it lands
  with the type.

Panels, chart backgrounds and lower thirds use the same recipe; only the
gradient angle changes between the light and dark halves of a scene.

## Envelope

```json
{
  "format": "opencut-reinforced-motion-scene",
  "schema_version": 3,
  "scene_id": "<UUIDv4>",
  "name": "...",
  "duration": {"numerator": 4, "denominator": 1},
  "canvas": {"width": 1920, "height": 1080},
  "assets": [],
  "nodes": [],
  "animations": [],
  "camera": {
    "keyframes": [
      {"at": {"numerator": 0, "denominator": 1}, "zoom": 1.0},
      {"at": {"numerator": 4, "denominator": 1}, "zoom": 1.08, "pan_x": -24}
    ],
    "easing": "ease_in_out"
  }
}
```

`camera` is optional; without it the identity camera is used, so existing scenes
render unchanged. Keys carry `pan_x`/`pan_y`, `zoom` (0.25–4) and
`rotation_degrees`; they are interpolated like node animations and applied to
every resolved instruction, so preview, layout and materialisation agree.

Node kinds: `group`, `text`, `rectangle`, `ellipse`, `arrow`, `image`.

- Text needs `text`; optional `position`, `font_size`, `fill`, `spans`
  (byte-offset highlight runs), and the typography set: `font` (`inter` |
  `be_vietnam_pro`), `font_weight` (`regular` | `medium` | `semibold` | `bold`),
  `text_align` (`start` | `center` | `end`), `line_height` (0.5–3.0, a multiple of
  the type size) and `letter_spacing` (-50–200 points). Omitted fields use the
  version 1 behaviour, so a version 1 document renders exactly as before.
- `size` on a text node is an explicit box: its width is the measure, its height
  bounds the block, and the text anchors at the box top. Omit it and the line is
  measured against the canvas and shrunk (to 60 %) before wrapping.
- Rectangle and ellipse need `size`; optional `position`, `fill`, `stroke`,
  `stroke_width`, `rotation_degrees`, `opacity`, and for shapes two style
  fields: `corner_radius` (canvas pixels, clamped to half the shorter side) and
  `gradient` (`{"from": colour, "to": colour, "angle_degrees": n}`, 0° pointing
  right and measuring counter-clockwise). A gradient replaces `fill` across the
  node's own box; omit it for a flat fill. Version 2 documents without either
  field keep their square, flat look.
- Image needs `asset_id`; group needs `children`.
- Colours are typed `{red, green, blue, alpha}` with 0–255 channels. There is no
  CSS string and no expression language.
- Animated properties are `translation_x`, `translation_y`, `scale`, `rotation`,
  `opacity`, `width`, `height`. Keyframes are ascending, inside `[0, duration]`,
  and each node animates a property once. Easings: `linear`, `ease_in`,
  `ease_out`, `ease_in_out`.

To change the title, a colour, a size or the typography, edit the node's fields and
re-materialise.
The materialised clip already on the timeline is an ordinary video clip; re-running
the scene never silently replaces it — insert the new render and delete the old
clip only if the user asks.

## Reusable assets

Reusable scenes, fragments and image cutouts carry typed parameters for text,
colour, size, opacity and rotation (ADR 0010). `project.json` records the library,
so every verb below takes `--attach project.json` and resolves the same folder the
app shows; `--library PATH` overrides it for one call. The asset verbs never open
a project except `insert`.

| Command | Use |
|---|---|
| `motion asset list --attach project.json --json` | Ids, names, categories, kinds and durations. |
| `motion asset validate --file FILE` | Validate one asset document. |
| `motion asset save --attach project.json --file FILE [--category C]` | Classify and store an authored asset. |
| `motion asset instantiate --attach project.json --id ID [--values JSON] --output OUT.ormotion.json [--scene-id] [--name]` | Write a scene file from an asset; no render, no project. |
| `motion asset import --attach project.json --source IMAGE.png [--name] [--category] [--tags a,b] [--width] [--height] [--seconds]` | Add a PNG cutout asset. |
| `motion asset remove --attach project.json --id ID` | Remove one asset. |
| `motion asset insert --attach project.json --asset-id ID [--values JSON] [--at] [--track] [--output] [--frame-rate] [--width] [--height]` | Instantiate, render through the app, import and insert one clip. |

Text and choice values are strings, numbers are numbers, and colours are
`{red,green,blue,alpha}`. Only `insert` needs the running app. A scene with
library-relative image assets is inserted from the app, not by `publish`.

## Bounds

A scene file is at most 4 MiB, 4096 nodes, groups nested 16 deep, 4096
animations, 4096 keyframes per animation, 64 KiB per text node. Asset paths must
be scene-local or project-relative; remote URLs are refused. Keep scenes short:
a ten-second 30 fps scene is 300 encoded frames, and one publish materialises at
most 200,000 frames.
