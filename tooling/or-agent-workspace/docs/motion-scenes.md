# Motion scenes

Scope: turning code or a declarative `.ormotion.json` scene into an ordinary,
editable video clip through the app's deterministic MotionScene renderer. No
provider or network call happens here.

## Two inputs

| Input | Use |
|---|---|
| A source file | `tools/or motion publish --attach project.json --source work/sample.rs --name "Parser" --json` turns the code into a highlighted illustration. |
| A scene file | `tools/or motion publish --attach project.json --scene scenes/title.ormotion.json --json` renders an authored scene. |

Pass exactly one of `--source` and `--scene`. One call validates the scene, saves
it to the app's motion library (with a preview), materialises the video through
the app, imports the result and inserts an ordinary clip. It is slow because
every frame is encoded; do not start a second render while one is running and do
not retry after a timeout without checking the timeline.

Useful optional arguments: `--name`, `--category`, `--at`, `--track`, `--width`,
`--height`, `--frame-rate` (`30/1`), `--output`, `--library`, `--no-library`.
When `--at` is omitted the clip is appended after the target track; when `--track`
is omitted the first unlocked video track is used, and one is created when the
project has none. `--output` must be absolute. `materialize` remains available
when only a file is wanted; it never imports or inserts.

## Authoring a scene

Start from the scaffold, then edit the JSON:

```sh
node tools/motion-scaffold.mjs --name "Title" --out scenes/title.ormotion.json
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
touches a project or the encoder. `tools/or motion
render` rasterises one frame to a PNG when a still is enough. `tools/or motion
from-code` writes a scene from a source file without materialising it, so the
agent can adjust the generated scene first.

`tools/or motion materialize --attach project.json --source PATH --output
<absolute .mkv>` renders to disk through the live app but does not import or
insert the result; use it only when a file is wanted, never as an insertion path.

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
  "animations": []
}
```

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

The app has an asset library (ADR 0010): reusable scenes, fragments and image
cutouts with typed parameters for text, colour, size, opacity and rotation. The
app records the library path in `project.json`, so the CLI needs no extra flag:

- `tools/or motion asset list --attach project.json --json` — ids, names,
  categories, kinds and durations.
- `tools/or motion asset insert --attach project.json --asset-id ID --values
  '{"title":"New"}' --json` — instantiate one asset with values keyed by
  parameter id, render it through the app and insert an ordinary clip. Text and
  choice values are strings, numbers are numbers, and colours are
  `{red,green,blue,alpha}`.
- `tools/or motion publish --scene scenes/title.ormotion.json --json` — saves an
  authored scene as a reusable, classified library asset with a preview, then
  inserts it.

`--library PATH` overrides the recorded library for one call. Do not copy or edit
library files yourself.

## Bounds

A scene file is at most 4 MiB, 4096 nodes, groups nested 16 deep, 4096
animations, 4096 keyframes per animation, 64 KiB per text node. Asset paths must
be scene-local or project-relative; remote URLs are refused. Keep scenes short:
a ten-second 30 fps scene is 300 encoded frames.
