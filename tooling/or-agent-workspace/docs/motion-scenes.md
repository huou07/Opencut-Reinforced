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

- **Flat colour only.** Pick three to five flat fills and reuse them; do not
  fake a gradient with stacked bands. The format has no gradient and reviewers
  read bands as a mistake.
- **Never a dead frame.** Keep two or three animations running across the whole
  duration — a slow drift, a pulse, a wipe — not only entrance fades. If the
  middle of the scene is static, it reads as a slide.
- **Type first.** Titles are at most six or seven words per line at 1080p, at
  least 28 px, with a 5 % margin. ASCII lines use Inter; a line with Vietnamese
  or any non-ASCII character uses Be Vietnam Pro, the bundled display face, so
  stacked marks keep their shape. Write NFC text, never strip diacritics to fit,
  and never pick a font name — both faces are bundled and the renderer chooses.
  Wrapping and shrinking to 60 % are safety nets, not a layout choice.
- **Cut on movement.** Overlap the end of one idea with the start of the next;
  avoid a fully empty frame between beats.
- **A camera move is the cheapest life.** A slow push-in (`camera.keyframes`
  with `zoom` 1 → 1.08, a small pan) keeps every frame moving; keep zoom ≥ 1 so
  a flat background never shows the canvas edge.
- Read the result with `tools/or motion layout --file ... --at MID --json`; the
  boxes are what a viewer sees, so check that text stays inside its canvas.

## Envelope

```json
{
  "format": "opencut-reinforced-motion-scene",
  "schema_version": 1,
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
  (byte-offset highlight runs).
- Rectangle and ellipse need `size`; optional `position`, `fill`, `stroke`,
  `stroke_width`, `rotation_degrees`, `opacity`.
- Image needs `asset_id`; group needs `children`.
- Colours are typed `{red, green, blue, alpha}` with 0–255 channels. There is no
  CSS string and no expression language.
- Animated properties are `translation_x`, `translation_y`, `scale`, `rotation`,
  `opacity`, `width`, `height`. Keyframes are ascending, inside `[0, duration]`,
  and each node animates a property once. Easings: `linear`, `ease_in`,
  `ease_out`, `ease_in_out`.

To change the title, a colour or a size, edit the node's fields and re-materialise.
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
