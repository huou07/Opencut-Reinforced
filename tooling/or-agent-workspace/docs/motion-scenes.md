# Motion scenes

Scope: turning code or a declarative `.ormotion.json` scene into an ordinary,
editable video clip through the app's deterministic MotionScene renderer. No
provider or network call happens here.

## Two inputs

| Input | Use |
|---|---|
| A source file | `mcp__or__motion_insert_scene {source_path: "work/sample.rs"}` turns the code into a highlighted illustration. |
| A scene file | `mcp__or__motion_insert_scene {scene_path: "scenes/title.ormotion.json"}` renders an authored scene. |

Pass exactly one of `source_path` and `scene_path`. The call materialises video
frames through the app, imports the result and inserts an ordinary clip. It is
slow because every frame is encoded; do not start a second render while one is
running and do not retry after a timeout without checking the timeline.

Useful optional arguments: `title`, `at`, `track_id`, `width`, `height`,
`frame_rate` (`30/1`), `output_path`.

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
`tools/or motion render` rasterises one frame to a PNG when a still is enough.
`tools/or motion from-code` writes a scene from a source file without
materialising it, so the agent can adjust the generated scene first.

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

A motion asset (`.ormotionasset.json`, ADR 0010) wraps a scene template or a
fragment with typed parameters for text, colour, size, opacity and rotation.
The app has no asset MCP tool yet; when the user names a library folder, use the
CLI:

```sh
tools/or motion asset list --library PATH --json
tools/or motion asset save --library PATH --file scenes/title.ormotionasset.json --json
tools/or motion asset instantiate --library PATH --id ASSET_ID \
  --values '{"title":"Hello"}' --output scenes/instantiated.ormotion.json
```

`--values` is a JSON object keyed by parameter id. Saving recomputes the category
from tags and the name unless it is explicit; pass `--category` to override it.
An instantiated scene is an ordinary `.ormotion.json` you can validate and then
pass to `mcp__or__motion_insert_scene`.

## Bounds

A scene file is at most 4 MiB, 4096 nodes, groups nested 16 deep, 4096
animations, 4096 keyframes per animation, 64 KiB per text node. Asset paths must
be scene-local or project-relative; remote URLs are refused. Keep scenes short:
a ten-second 30 fps scene is 300 encoded frames.
