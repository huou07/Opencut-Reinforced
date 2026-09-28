# ADR 0003 — Frame memory domain and ownership

- Status: Accepted
- Date: 2026-09-28

## Context

Full-rate decoded frames are too large and frequent for copied Dart byte
arrays. Software buffers and native GPU/video surfaces also have different
lifetime and release rules.

## Decision

Represent runtime output with a `FrameDescriptor` plus an owned `FrameLease`.
The descriptor carries format, dimensions, exact timestamp, and access mode;
the lease carries lifetime/release semantics for software or native memory.
Flutter receives an external/native surface or viewer handle, not a copied
frame stream. Native handles are runtime-only.

## Alternatives rejected

- Copying decoded frames into Dart for every presentation.
- Serializing native resource handles in projects or cache keys.
- Giving workers a mutable project lock for the frame lifetime.

## Consequences

Ownership, backpressure, stale frame dropping, and software fallback need
explicit tests. Viewer integration is platform-aware at the presentation edge,
while domain evaluation remains platform-neutral.
