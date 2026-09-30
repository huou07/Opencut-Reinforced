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

The current execution state remains governed by `docs/execution/STATE.json`.
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
