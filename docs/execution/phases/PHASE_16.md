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
`INV-SEC-001`, `INV-DEP-001`.

## 16B — Sandbox and declarative-first model

Implement declarative effects/templates and a sandbox boundary before any native
extension. Define IPC/resource limits, cancellation, versioning, failure
containment, and cache disposal. No arbitrary native load is allowed by this
checkpoint.

Affected invariants: `INV-RT-001`, `INV-RT-002`, `INV-RENDER-002`,
`INV-RENDER-003`, `INV-JOB-001`, `INV-CACHE-001`, `INV-SEC-001`, `INV-DEP-001`.

## 16C — Permissions, resources, and network

Make filesystem, project, media, network, compute, and credential permissions
explicit. Enforce quotas, timeouts, cancellation, auditability, and offline
behavior. Network access is opt-in and never implicit in rendering or editing.

Affected invariants: `INV-JOB-001`, `INV-CACHE-001`, `INV-SEC-001`,
`INV-DEP-001`.

## 16D — Versioned APIs

Define stable typed plugin/task/render APIs with capability discovery, semantic
versioning, migration/deprecation policy, and conformance fixtures. Native
handles are exchanged only through runtime-owned opaque interfaces and are
never serialized.

Affected invariants: `INV-RT-002`, `INV-RENDER-001`, `INV-RENDER-002`,
`INV-RENDER-003`, `INV-IPC-001`, `INV-DEP-001`.

## 16E — OTIO, EDL, XML, and bounded motion interchange evaluation

Evaluate OpenTimelineIO, EDL, XML, Lottie, and dotLottie import/export as
bounded, explicit interchange adapters. Preserve exact-time conversion
policies, unsupported-feature diagnostics, rights, and no-surprise project
mutation. Imports are reviewable before apply; exports are deterministic where
the format permits. Lottie and dotLottie are interchange candidates only, not
the canonical MotionScene format or a required runtime dependency.

Affected invariants: `INV-STATE-001`, `INV-STATE-002`, `INV-STATE-003`,
`INV-TIME-001`, `INV-PERSIST-001`, `INV-IPC-001`, `INV-DEP-001`.

## 16F — OpenFX/native high-trust evaluation

Only after the prior security and API gates, evaluate OpenFX or other native
high-trust extensions as optional, explicitly enabled adapters. Require signed
or reviewable manifests, isolation where possible, platform/license evidence,
crash containment, fallback behavior, and clear incompatibility reporting.
Arbitrary native loading remains prohibited by default; this checkpoint is an
evaluation gate, not blanket permission to load untrusted code.

Affected invariants: `INV-RT-001`, `INV-RT-002`, `INV-RENDER-001`,
`INV-RENDER-002`, `INV-RENDER-003`, `INV-HW-001`, `INV-HW-002`, `INV-JOB-001`,
`INV-SEC-001`, `INV-DEP-001`.

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
GPU workaround. If safe acceleration is unavailable, use a safer fallback or
report the capability unavailable. A future pinned, capability-managed
Chromium/Chrome version enters provenance but is not sufficient isolation by
itself. Prefer bounded frame/media streaming into the existing OR encoder over
thousands of PNGs or a second encoder.

Future tests cover network/credential denial, bounded inputs and outputs,
timeouts, cancellation, crash containment, filesystem and project-write
denial, process isolation, no execution on project open, materialization-only
application, unsafe-browser rejection, and normal media validation.

Affected invariants: `INV-STATE-002`, `INV-STATE-003`, `INV-RENDER-001`,
`INV-RENDER-002`, `INV-MOTION-003`, `INV-JOB-001`, `INV-SEC-001`,
`INV-DEP-001`.

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
