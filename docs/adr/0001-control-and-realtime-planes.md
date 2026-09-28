# ADR 0001 — Control and realtime planes

- Status: Accepted
- Date: 2026-09-28

## Context

OR needs human UI, CLI, and agent operations to share one project model while
preview, decode, render, audio, and export work at realtime or background
cadences. Mixing these lifecycles would make project revision/history and frame
delivery unpredictable.

## Decision

Use two explicit planes. The control plane is
`Flutter/CLI/Agent -> typed commands and queries -> or_core -> ProjectDocument`.
The runtime plane consumes an immutable, versioned `RenderSnapshot` and
produces frames, audio, preview, or export results. Runtime work never executes
project commands or increments `ProjectRevision`.

## Alternatives rejected

- Letting Flutter or a worker own a second editable document.
- Routing every playback tick or frame through the command/history path.
- Treating CLI or agent operations as UI-click automation.

## Consequences

Commands, revision preconditions, history, and persistence remain auditable.
Runtime cancellation, backpressure, and stale-result handling must be designed
explicitly. Read-model and frame transport contracts are separate concerns.
