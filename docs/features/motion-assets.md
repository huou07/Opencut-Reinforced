# Feature — Motion Assets and Library

Last reviewed: 2026-10-27

Scope: the reusable, parameterised MotionAsset document, its deterministic
classifier, the app-level library store and the CLI that proves them (P1 of
[ADR 0010](../adr/0010-motion-assets-and-reusable-library.md)). The Motion
surface, drag-and-drop and MCP tools are P2/P3 and are **not** implemented.

## Contract

A motion asset is a versioned declarative document, `.ormotionasset.json`,
format `opencut-reinforced-motion-asset`, schema version 1. It is data, not a
program: `deny_unknown_fields` everywhere, no expression language, no script, no
network reference, no provider call.

V1 envelope: `format`, `schema_version`, UUIDv4 `asset_id`, `name`, `category`,
`category_source`, `tags`, `kind`, `canvas`, exact `duration`, typed
`parameters`, and one kind-specific payload:

| Kind | Payload | Reuse |
|---|---|---|
| `scene` | `scene` body: `assets`, `nodes`, `animations` | instantiate into a whole scene |
| `fragment` | `scene` body | instantiate with node ids remapped by the asset id, so a later merge cannot collide |
| `motion` | `preset`: property, keyframes, easing | applied to a selected node (not instantiable yet) |
| `image` | `image`: relative file + optional SHA-256 | still cutout (image pipeline not implemented yet) |

The `scene` body is validated by the same node, asset and animation rules as a
MotionScene. Diagnostics use `MOTION_ASSET_*` codes and point into `/scene/...`
for body problems.

## Parameters

`parameters` is a bounded list of typed slots; it is how an asset is adjustable
without an editor:

- `id`, `label`, `type` (`text`, `color`, `number`, `choice`), `target`
  (`node_id` + one field), `default`, optional `min`/`max`/`choices`.
- Addressable fields: `text`, `font_size`, `fill`, `size_width`, `size_height`,
  `opacity`, `rotation_degrees`, `asset_id`. A field must fit the node kind
  (text/font size on text, size on rectangle/ellipse, asset id on image, and so
  on); two parameters cannot write the same node field.
- The default must equal the targeted node field. That keeps the payload and the
  control in the library UI from drifting apart.
- Instantiation substitutes values into a copy of the payload and then validates
  the result as an ordinary MotionScene. Text stays literal bytes; a value that
  looks like a template is never evaluated.

## Classification

The category set is closed: `background`, `people`, `nature`, `animal`,
`vehicle`, `object`, `text`, `shape`, `icon`, `chart`, `effect`, `motion`.

`classify_motion_asset` runs in a fixed order and records the winning rule in
`category_source`:

1. an explicit category (author's or user's manual choice) — `explicit`;
2. a tag token matching a category keyword — `tags`;
3. a name token matching a category keyword — `filename`;
4. a kind default (`motion` for a motion preset, otherwise `object`) — `default`.

Keywords include Vietnamese and English synonyms (`nền`, `người`, `động vật`,
`xe`, `biểu đồ`, ...). Matching is a lowercase token comparison, so it is
deterministic and needs no model or network. A manual choice always wins and is
stored as `explicit`; the library recomputes the others on save.

## Library

The library is a plain directory, not a database and not project state:

```
<root>/
  catalog.json                      # bounded index, sorted by name then id
  assets/<asset_id>.ormotionasset.json
  previews/                         # reserved for P2
  images/                           # reserved for image assets
```

- `MotionLibrary::save` validates the document, resolves the category, writes
  the asset file atomically (temporary file plus rename), then rewrites the
  catalog atomically. At most 4096 entries; the catalog is bounded to 4 MiB.
- `list` reads the catalog; `load` reads and re-validates one asset and refuses a
  mismatch between file name and `asset_id`; `remove` deletes the asset, its
  preview and the catalog row.
- Asset ids must be UUIDv4, so an id can never name a path outside `assets/`.
  Asset-internal paths (`preview`, `image.file`) must be relative and cannot
  contain `..`, a drive prefix, a scheme or a platform separator.
- The library location is the caller's: the CLI takes `--library PATH`. P2 wires
  it to the application support directory and the app bridge.

## CLI

```
or motion asset validate --file PATH [--json]
or motion asset save --library PATH --file PATH [--category CATEGORY] [--json]
or motion asset list --library PATH [--json]
or motion asset instantiate --library PATH --id ASSET_ID [--values JSON] --output PATH [--scene-id UUID] [--name NAME] [--json]
or motion asset remove --library PATH --id ASSET_ID [--json]
```

`--values` is a JSON object keyed by parameter id; text and choice are strings,
number is a number, colour is `{red, green, blue, alpha}`. An unknown id or a
value of the wrong type is refused with `MOTION_ASSET_UNKNOWN_PARAMETER` or
`MOTION_ASSET_INVALID_VALUE`. `instantiate` writes a normal `.ormotion.json`
that the existing validate/render/materialise verbs accept.

None of these verbs opens a project, renders a frame or contacts a provider.
`motion materialize` still needs the live app, and `image`/`motion` assets are
catalogued but not instantiated in P1.

## Bounds

4 MiB per asset, 64 parameters, 16 tags of 64 bytes, 128-byte name, 4096
nodes / animations, 16-deep groups, 64 KiB per text value, 32 choices per
parameter, 4096 library entries. Every bound is enforced before a value reaches
a renderer.

## Gaps

- No Motion surface, library browsing, parameter editor or drag-and-drop (P2).
- No bridge command or MCP tool yet; an agent cannot list or insert a library
  asset through `or mcp` (P3).
- No generated previews and no image pipeline, so `image` assets and the
  `preview` field are references only.
- No live timeline Clip; placement remains materialise-first per ADR 0010.
- No per-project library and no import from the existing Asset Library screen.

## Where

- `crates/or_motion/src/asset.rs` — document, validation, classifier, instantiation
- `crates/or_motion/src/library.rs` — catalog, atomic store, entries
- `crates/or_cli/src/commands.rs` — `or motion asset` verbs
- `crates/or_cli/tests/semantic_cli.rs` — round-trip and refusal journeys

## Related

- [motionscene.md](motionscene.md)
- [ADR 0010](../adr/0010-motion-assets-and-reusable-library.md)
- [assistant.md](assistant.md)
