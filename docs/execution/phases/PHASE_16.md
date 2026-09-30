# Phase 16 — Plugins, interchange, OpenFX, and procedural WebMotion evaluation

## Status

Phase 16 is planned. Extensibility remains capability-scoped, versioned,
reviewable, and disabled by default where trust cannot be established. Lottie,
dotLottie, and an optional sandboxed procedural WebMotion adapter are future
evaluation scopes; neither replaces canonical MotionScene or the OR render
spine.

## 16A — Plugin security and capabilities

Define a capability manifest, permission prompt, trust level, resource budget,
and revocation model. The default plugin path is declarative and sandboxable.
Plugins cannot read stored credentials or mutate canonical state outside typed
commands.

Affected invariants: `INV-STATE-002`, `INV-STATE-003`, `INV-JOB-001`,
`INV-SEC-001`, `INV-DEP-001`, `INV-EXT-001`.

## 16B — Sandbox and declarative-first model

Implement declarative effects/templates and a sandbox boundary before any native
extension. Use `wasmi` 1.1.0 as the first WASM sandbox runtime, added only when
16B executes, in a dedicated extension/plugin runtime boundary. It must never
be a direct `or_core` dependency. Do not enable general WASI by default; a
plugin receives only explicitly defined OR host functions/capabilities, with
no default filesystem, network, environment enumeration, stored credentials,
process launch, wall-clock dependency, or arbitrary project mutation. All
project changes become ordinary validated OR commands.

Enable wasmi fuel metering and explicit `StoreLimits`. Also enforce outer host
limits for module bytes, linear memory, instances/tables, input/output size,
host-side allocations, execution time, and job cancellation; runtime memory
limits do not account for every host allocation. Define IPC/resource limits,
failure containment, and cache disposal. No arbitrary native load or native
plugin ABI is allowed by this checkpoint; 16D owns the stable versioned
extension ABI/API.

Affected invariants: `INV-RT-001`, `INV-RT-002`, `INV-RENDER-002`,
`INV-RENDER-003`, `INV-JOB-001`, `INV-CACHE-001`, `INV-SEC-001`,
`INV-DEP-001`, `INV-EXT-001`.

## 16C — Permissions, resources, and network

Make filesystem, project, media, network, compute, and credential permissions
explicit. Enforce quotas, timeouts, cancellation, auditability, and offline
behavior. Network access is opt-in and never implicit in rendering or editing.

Affected invariants: `INV-JOB-001`, `INV-CACHE-001`, `INV-SEC-001`,
`INV-DEP-001`, `INV-EXT-001`.

## 16D — Versioned APIs

Define stable typed plugin/task/render APIs with capability discovery, semantic
versioning, migration/deprecation policy, and conformance fixtures. Native
handles are exchanged only through runtime-owned opaque interfaces and are
never serialized.

These are runtime contract APIs; 16D does not migrate persistent
`ProjectDocument` schema. Future persistent plugin or project metadata requires
an explicit model gate. Do not create a native-plugin ABI by accident in 16B.

Affected invariants: `INV-RT-002`, `INV-RENDER-001`, `INV-RENDER-002`,
`INV-RENDER-003`, `INV-IPC-001`, `INV-DEP-001`, `INV-EXT-001`.

## 16E — Bounded interchange adapters

Implement a bounded OTIO JSON adapter directly in Rust/serde against a
documented supported subset and conformance fixtures. Do not add OpenTimelineIO
C++ or Python runtime dependencies. Unsupported schemas/features produce
explicit diagnostics; imports are reviewable before apply, with exact-time
conversion and no-surprise mutation. Keep EDL and XML adapters bounded and
explicit.

Use Lottie JSON specification 1.0 as the V1 baseline. Unsupported/newer fields
produce explicit compatibility diagnostics. Do not execute expressions or
fetch network assets during import/render. Use dotLottie specification 2.0 as
the initial container baseline and treat the archive as untrusted ZIP input:
enforce entry-count, total uncompressed-byte, and per-entry limits; reject path
traversal and duplicate/colliding paths; bound manifest/JSON parsing; and never
fetch remote assets implicitly. Lottie and dotLottie are interchange formats,
not canonical MotionScene and not a second renderer.

Affected invariants: `INV-STATE-001`, `INV-STATE-002`, `INV-STATE-003`,
`INV-TIME-001`, `INV-PERSIST-001`, `INV-IPC-001`, `INV-DEP-001`.

## 16F — OpenFX/native high-trust evaluation

Only after the prior security and API gates, evaluate OpenFX or other native
high-trust extensions as optional, explicitly enabled adapters. Require signed
or reviewable manifests, isolation where possible, platform/license evidence,
crash containment, fallback behavior, and clear incompatibility reporting.
Arbitrary native loading remains prohibited by default; this checkpoint is an
evaluation gate, not blanket permission to load untrusted code.
`OPENFX_NATIVE_TIER = NOT_APPROVED` is a valid DONE outcome when isolation,
crash containment, package/signing, platform, or security evidence is
insufficient. Lack of OpenFX approval does not block 16G.

Affected invariants: `INV-RT-001`, `INV-RT-002`, `INV-RENDER-001`,
`INV-RENDER-002`, `INV-RENDER-003`, `INV-HW-001`, `INV-HW-002`,
`INV-JOB-001`, `INV-SEC-001`, `INV-DEP-001`, `INV-EXT-001`.

## 16G — Sandboxed procedural WebMotion adapter

Evaluate an optional advanced `WebMotionBundle` for cases that cannot
reasonably fit declarative MotionScene. Candidate inputs include custom
HTML/CSS, SVG or Canvas runtimes, Three.js-style scenes, WebGL/WebGPU, and
bounded simulations. The path is:

```text
WebMotionBundle -> explicit execution request -> isolated sidecar/browser
                 -> bounded frames/media -> materialized ordinary OR asset
```

It is never the canonical renderer, never used during ordinary project open,
and never runs inside `or_core` or the Flutter process. It has no default
network, credential, arbitrary project-file, or direct `ProjectDocument` write
access. The outer OR-owned process boundary enforces least privilege with
read-only bounded approved inputs, dedicated bounded scratch/output, time,
memory/resource, and output-size limits, cancellation, crash containment, and
structured diagnostics. Browser sandbox/CSP is defense in depth, not the sole
trust boundary.

Production must reject a configuration that disables the browser sandbox as a
GPU workaround. Never launch Chromium/Chrome with `--no-sandbox`. If a
trustworthy outer process isolation boundary cannot be demonstrated, record
`WEBMOTION = UNAVAILABLE`; this is a valid checkpoint outcome. If safe
acceleration is unavailable, use a safer fallback or report the capability
unavailable. A future pinned, capability-managed
Chromium/Chrome version enters provenance but is not sufficient isolation by
itself. Prefer bounded frame/media streaming into the existing OR encoder over
thousands of PNGs or a second encoder.

Future tests cover network/credential denial, bounded inputs and outputs,
timeouts, cancellation, crash containment, filesystem and project-write
denial, process isolation, no execution on project open, materialization-only
application, unsafe-browser rejection, and normal media validation.

Affected invariants: `INV-STATE-002`, `INV-STATE-003`, `INV-RENDER-001`,
`INV-RENDER-002`, `INV-MOTION-003`, `INV-JOB-001`, `INV-SEC-001`,
`INV-DEP-001`, `INV-EXT-001`.

## Stop conditions

Stop for arbitrary native loading, unbounded capabilities, secret access,
unversioned APIs, missing crash/fallback behavior, unsafe interchange mutation,
unresolved OpenFX license/platform risk, a procedural browser without an outer
least-privilege boundary, default network, `--no-sandbox`, direct project
mutation, or a second encoding authority.

## Handoff

Report capability/security model, sandbox/resource evidence, API fixtures,
interchange diagnostics, native-extension and WebMotion evaluation decisions,
procedural security tests, docs, commit, and hosted platform status.
