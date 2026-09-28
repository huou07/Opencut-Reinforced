# ADR 0002 — wgpu render spine and native interop

- Status: Accepted
- Date: 2026-09-28

## Context

OR targets desktop and mobile GPUs with platform-specific resources, while the
editor needs one compositing/evaluation model and deterministic preview/export
behavior.

## Decision

Use wgpu as the shared cross-platform render-graph spine. Keep Metal, DX12,
Vulkan, CUDA, and other native interop behind runtime adapters. Platform handles
are never part of domain state, serialized project data, IPC, or cache identity.

## Alternatives rejected

- A separate rendering architecture for each platform.
- Sending platform handles through `or_core` or the project document.
- Choosing a backend before a measured capability and licensing gate.

## Consequences

The render core stays portable and testable with synthetic/offscreen scenes.
Adapters require platform-specific ownership and fallback tests. wgpu version,
MSRV, backend, and license evidence must precede a dependency pin.
