# ADR 0007 — Declarative MotionScene and procedural isolation

- Status: Accepted
- Date: 2026-09-28

## Context

OR needs a future path for simple motion graphics, diagrams, explainers, and
other authored visual sequences that can be produced by a person, a template,
an external agent, or an optional built-in AI provider. The path must remain
cross-platform, seekable, inspectable, and usable without an AI service. It
must not turn arbitrary web code into canonical project state or create a
second render and encoding authority.

The current execution state remains governed by `docs/archive/execution/STATE.json`.
This ADR records future architecture only; it does not implement MotionScene,
rendering, a live MotionClip, a browser runtime, or any dependency.

## Decision

Canonical simple motion graphics will use an OR-owned, versioned,
non-executable declarative `MotionScene` source format. A future `.ormotion.json`
file is reserved for this source. The conceptual V1 envelope is:

```json
{
  "format": "opencut-reinforced-motion-scene",
  "schema_version": 1,
  "scene_id": "<UUIDv4>",
  "name": "...",
  "duration": {"numerator": 1, "denominator": 1},
  "canvas": {"width": 1920, "height": 1080},
  "assets": [],
  "nodes": [],
  "animations": []
}
```

The exact serialized field names, bounds, and codec are deferred to Phase 11E,
after the Phase 7/8 renderer capabilities exist. No codec structs or
`ProjectDocument` fields are introduced by this ADR.

**Implemented 2026-10-20 (`crates/or_motion`) — the decisions this made.** The
envelope above is realized as written, with these concrete choices:
`deny_unknown_fields` on every struct, so a field this version does not implement is
refused rather than ignored; one **flat** node struct carrying every kind's optional
fields plus a `kind` discriminant, because serde does not support
`deny_unknown_fields` with an internally tagged enum and a flat shape also lets the
validator say exactly which field a kind requires; `Polyline` deliberately absent
rather than accepted and ignored; a locked `MotionProperty` set and a locked
`MotionEasing` set with no expression language; exact `or_core::RationalTime` for
durations and keyframe times; typed `MotionColor` rather than a CSS string; asset
references are identities with an optional SHA-256 and a scene-local or
project-relative path, and a remote URL is refused because rendering never fetches.
Bounds: 4 MiB per file, 4096 nodes, group nesting 16 deep, 4096 animations, 4096
keyframes each, 64 KiB per text node. Diagnostics carry stable codes and JSON paths
(`MOTION_INVALID_DURATION`, `MOTION_REMOTE_ASSET_REFUSED`, …) in the `{ok, errors}`
shape below. Text is literal: a string that looks like a template or an expression is
preserved byte for byte and never evaluated. Validation and inspection are pure reads
— no project, no renderer, no provider — and the agent surface is
`or motion validate|inspect|evaluate --file PATH [--json]`.

**Evaluator, 2026-10-20.** `evaluate_scene(scene, exact_time)` returns a `MotionFrame`
of flat, typed `MotionInstruction`s in document order, with every animated property
already resolved: a renderer never sees a keyframe, an easing curve or a property
name, so the render path cannot depend on scene structure. Evaluation refuses an
invalid scene rather than partially drawing it, and refuses a time outside the
half-open `[0, duration)` rather than clamping it, because a silent clamp would hide
a caller's arithmetic error behind a plausible picture. Animation sampling is total
over the scene (the first value holds before the first keyframe, the last after the
last), the time ratio is computed through the exact rationals rather than
floating-point seconds, and the easing set is the locked four. Evaluation reads
nothing: no asset, font, file, network or provider — a test evaluates a scene whose
asset path exists nowhere.

**Rasteriser, 2026-10-20.** `or_render::rasterize_motion_frame` turns an evaluated
frame into one RGBA layer that the existing compositor consumes like any decoded
frame: a layer *producer*, not a second compositor, so blending, effects and encoding
stay in one place. It draws rectangles, ellipses, arrows, image placeholders and text
through the existing bundled-font rasteriser — not a second text engine. Coordinates
are canvas pixels with the origin top left, x right and y down, matching the RGBA
buffer; rotation is clockwise in that space; document order is paint order. An image
node draws a visible placeholder rather than decoding an asset, because evaluation
resolves identities and the asset pipeline is a later step: drawing nothing would look
like a working scene with a missing picture. `or_render::encode_png` is a
dependency-free *still preview* encoder — materialised video still goes through the
normal media and export boundary — and its output is deterministic (stored deflate
blocks, no timestamps or metadata).

**Highlighted spans and code ingestion, 2026-10-20.** A text node may carry typed
`spans` — byte ranges with a colour — so one line can be drawn in more than one
colour; spans are values, never markup, and validation requires them ordered,
non-overlapping and inside the text. They render through the same shaping and layout
as ordinary text, with each glyph's own colour passed to the rasteriser, because a
glyph image is a mask and its colour comes only from that base. `or_motion::code`
builds a scene from a source file with a bounded lexical tokeniser and a locked theme
of named colour roles: a sample is untrusted input, so the scan has hard limits and
never executes anything, and it deliberately has no grammar, no macro expansion and no
type information. Only Rust is tokenised; anything else is plain text.

MotionScene source is data, not a program. It uses exact `RationalTime`, is
random-access and seekable, is provider-independent, and does not mutate
`ProjectDocument` during evaluation. Human, template, CLI/agent, and AI
producers all target the same validator and evaluator.

## V1 semantic contract

- Scene duration is finite, nonnegative at the value level, and evaluated over
  a half-open local interval `[0, duration)`.
- Evaluation is `Evaluate(scene, exact_time)` and has no frame-history or
  replay-from-zero requirement.
- Local asset references are typed, stable, and checksum/provenance-aware
  through the Phase 11 asset system. Rendering never requires a remote URL.
- Evaluation performs no HTTP fetch, environment lookup, filesystem glob,
  shell command, provider call, or script execution.
- V1 intentionally targets simple explainers with typed `Group`, `Text`,
  `Rectangle`, `Ellipse`, `Line`/`Arrow`, optional renderer-gated `Polyline`,
  and `Image` primitives.
- Typed animation properties cover translation, scale, rotation, opacity, and
  bounded geometry/size plus supported fill/stroke values.
- Easing is a small locked typed set; there is no expression language.
  Charts and diagrams start as templates composed from primitives rather than
  an open-ended chart-node language.
- Font references are typed and reproducible through an asset checksum and
  provenance. The deterministic baseline is the Phase 8 Inter 4.1 font identity
  rendered through cosmic-text 0.19.0; missing fonts produce diagnostics and
  rendering never silently downloads fonts or depends on host font discovery.
- V1 should avoid randomness. Any later randomness must use an explicit seed
  derived deterministically from scene/source identity and never wall-clock or
  system-random state.

The validator returns machine-actionable diagnostics with stable codes, JSON
paths, and human-readable messages, for example:

```json
{
  "ok": false,
  "errors": [
    {
      "code": "MOTION_INVALID_DURATION",
      "path": "/duration",
      "message": "duration must be greater than zero"
    }
  ]
}
```

## Canonical render and materialization path

The future canonical path is:

```text
MotionScene
  -> validate
  -> evaluate at exact time
  -> typed evaluated instructions / RenderSnapshot
  -> existing or_render/wgpu spine
  -> preview or normal OR materialization/export encoder
```

Preview and materialization share the same semantic evaluator. The design does
not create a second compositor, browser-based default renderer, or WebCodecs
default encoder. Materialized video uses the normal OR media/export encoding
boundary. Its mandatory software correctness profile, owned by Phase 8F, is
Matroska + FFV1 + PCM S16LE through linked FFmpeg; other delivery or hardware
profiles are optional and require their own evidence.

Phase 11 is materialization-first. A scene is reviewed and rendered into a
persistent generated media asset, registered through normal media commands, and
then placed on the ordinary timeline. The output referenced by a project is
not merely disposable cache. Editing a scene and rendering again never silently
replaces bytes behind an existing canonical `MediaId`; a new content identity
is created unless an explicit validated project operation or `EditPlan` asks to
replace generated content everywhere.

A first-class live `MotionClip` is not authorized. It can be reconsidered only
through a later explicit architecture and schema gate if measured workflows
show that materialization is insufficient.

## Agent and AI contract

The future vendor-neutral semantic surface may include:

```text
or motion validate <scene> [--json]
or motion inspect <scene> [--json]
or motion render <scene> --output <path> [...]
```

Phase 11E/11F may freeze exact names. Validation and inspection never mutate a
project. Rendering is non-mutating unless a separately explicit materialize or
project-application command is invoked. Codex, Claude Code, OpenCode, other
harnesses, human authors, and templates can therefore produce the same scene
contract.

Rendering a MotionScene requires zero LLM or provider calls. A harness may
generate, validate, and render locally without invoking another model. There
is no per-frame AI, mandatory planner/critic loop, or mandatory cloud service.
Phase 10 reserves `GenerateMotionScene` for a future provider task that returns
an editable declarative scene proposal. It is distinct from `GenerateVideo`,
which returns an opaque raster/video asset. Neither task is implemented here.

Templates instantiate concrete validated MotionScene structures with typed,
bounded parameters; they do not execute code at render time. TTS, music, and
SFX remain ordinary OR audio/media/timeline concerns. An agent or `EditPlan`
may coordinate their timestamps with a scene without placing provider or audio
engine semantics inside MotionScene.

## Optional procedural boundary

Some future work may require custom HTML/CSS, SVG or Canvas runtimes,
Three.js-style scenes, WebGL/WebGPU, or simulations. This is a separate
optional `WebMotionBundle` capability, not canonical MotionScene. It may run
only after an explicit request in an isolated sidecar/browser boundary and must
produce bounded frames or media that are materialized as an ordinary OR asset.

It must not be the canonical renderer, run during normal project open, run in
`or_core` or the Flutter process, access stored credentials, use network by
default, write arbitrary project files, or mutate `ProjectDocument` directly.
The future boundary requires browser process isolation plus an OR-owned
least-privilege outer process boundary, network off by default, read-only
bounded approved inputs, dedicated bounded scratch/output, no secret or
environment credential exposure, time/memory/resource/output limits,
cancellation, crash containment, and structured diagnostics. CSP and the
browser sandbox are defense in depth, not the sole trust boundary.

Production must never disable the browser sandbox as a GPU workaround. If a
safe accelerated configuration is unavailable, use a safer fallback or report
the capability unavailable. If Chromium/Chrome is later selected, its explicit
capability-managed version and renderer provenance must be recorded; a pinned
browser alone is not sufficient isolation. Prefer bounded frame/media streaming
to the existing OR encoder instead of thousands of PNG files or a second codec
authority.

## Technology position

- Remotion is a useful frame-addressed programmatic-video reference, not the
  canonical runtime or an OR dependency.
- Motion Canvas is a useful permissive code-animation reference, not the
  canonical runtime or an OR dependency.
- Lottie JSON 1.0 and dotLottie 2.0 are future bounded interchange formats, not
  the canonical MotionScene format.
- Chromium/headless browsers are optional future procedural runtimes only, not
  the normal preview/export path.
- WebCodecs is not OR's canonical encoding authority.
- The approved linked FFmpeg path remains the normal materialization and export
  path after its Phase 7/8 platform and licensing gates.

MotionScene semantics are independent of Metal, CUDA, MLX, Core ML, NVIDIA,
Android acceleration, and cloud AI. Hardware may accelerate rendering,
encoding, or scene generation, but it cannot change the schema or canonical
semantics. Existing wgpu, native-adapter isolation, and centralized
capability/provider selection remain the architecture.

## Provenance and cache

Future derived-preview/render cache identity includes the MotionScene source
hash, referenced asset hashes, evaluator/render version, output dimensions,
frame rate, and color/output settings. The cache is disposable. A persistent
materialized media asset referenced by a project is not cache-only.

Future provenance records source MotionScene hash/version, referenced asset
hashes, template identity/version where applicable, AI provider/model only when
AI actually produced the scene, renderer version, and output settings.
Human- and agent-authored scenes do not pretend that an AI provider was used.

## Consequences and gates

Phase 11E freezes the bounded schema, validation, exact-time evaluator,
semantic identity, and lowering into existing render primitives. Phase 11F
adds the non-AI CLI/materialization workflow and minimal source-asset UX, not a
full After Effects-style editor, node graph, keyframe curve editor, browser
IDE, or code editor. Phase 14G may add AI scene proposals only through the
existing provider boundary and review/apply path. Phase 16G may add the
explicitly sandboxed procedural adapter only after its security and capability
gates.

Future agents must stop rather than add JavaScript to canonical MotionScene,
browser or provider dependencies to `or_core`, network rendering, executable
templates, cache-only timeline output, a live MotionClip without a new gate,
disabled sandboxing, browser replacement of wgpu, WebCodecs replacement of the
normal OR encoder, or renderer/provider mutation of `ProjectDocument`.

This amendment preserves project schema v4, recovery schema v1, IPC v1, the
existing `or_core` dependency restrictions, and the current execution state.
