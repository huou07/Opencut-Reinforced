# System Architecture Overview

Last reviewed: 2026-10-26

Scope: the shape of the running system — clients, planes, crates, and the data
flow between them. It states what exists and what is only planned. Subsystem
detail lives in the sibling architecture documents.

## One core, three clients

OR has exactly one editable project state, owned by Rust. The Flutter desktop and
Android UI, the `or` CLI, and agents are clients of the same validated command
and query API. None of them owns a second editing engine or a second project
document.

```
Flutter UI ──typed bridge──┐
CLI ─────────local IPC─────┼──> command/query dispatch ──> Rust domain core
Agents ──────local IPC─────┘         │
Assistant panel ──ACP──> dsh ──MCP──> or mcp ──local IPC──┘
                                     ├─> ProjectDocument + history + persistence
                                     └─> evaluated RenderSnapshot
                                              │
                                    decode / audio / render workers
                                              │
                                     preview texture or export encoder
```

Flutter keeps presentation state (navigation, selection, panel layout, input)
and bounded read-model caches. It never keeps a second editable project model,
and it never writes project state directly.

The in-editor assistant follows the same rule. DeepSeek Harness runs out of
process; its `or mcp` tools attach to the running app and use the typed command
path. Provider credentials stay with the harness (ADR 0009).

## Two planes

The architecture separates decisions from time-sensitive work. This split is
permanent (ADR 0001).

| Plane | Owns | Must never do |
|---|---|---|
| Control / project plane | commands, queries, undo/redo, `ProjectRevision`, persistence, timeline edit decisions | run per-frame playback or render work |
| Real-time media plane | decoded frames, audio buffers, playback clock, queues, GPU resources, dropped-frame decisions, transient playhead | create project transactions, increment `ProjectRevision`, mutate `ProjectDocument` |

Workers consume an immutable, versioned `RenderSnapshot` of a specific
`ProjectRevision`. Playback ticks, decoding, buffering, rendering, and dropping
an obsolete frame are runtime activity, not project edits.

## Crate map

| Crate | Responsibility |
|---|---|
| `or_core` | domain, `ProjectDocument`, command/query contracts, history, persistence, recovery, media probing, cache and job foundations |
| `or_runtime` | snapshot identity, frame descriptor/lease, bounded queues, budgets, capability selection |
| `or_media` | linked FFmpeg demux/decode/seek, software frame and audio output |
| `or_render` | wgpu render spine, layer evaluation, text rasterisation |
| `or_audio` | audio clock, bounded output buffer, gain/pan/fade |
| `or_ai` | task/provider/model boundary (deferred work) |
| `or_app_bridge` | Rust↔Flutter bridge (typed, opaque handle) |
| `or_ipc` | local IPC transport and shared live-host dispatch |
| `or_cli` | `or` semantic CLI |

`or_core` stays free of GPU, media, audio, text-shaping, scripting and AI-runtime
dependencies; `scripts/check_or_core_deps.py` enforces that boundary. Platform
interop (Metal, VideoToolbox, MediaCodec, DMABUF, CUDA) belongs behind narrow
runtime adapters, never in domain code.

Planned crates do not exist as empty scaffolds. A subsystem is added when a
concrete feature needs it.

## Data flow of one edit

1. A client submits a command with `ProjectId`, `ProjectInstanceId` and the
expected `ProjectRevision`.
2. The command path validates, applies the mutation, produces a `ChangeSet`,
   appends one history entry, and increments the revision once.
3. Persistence writes atomically on explicit save; the recovery sidecar is a
   separate envelope.
4. A new revision invalidates the current `RenderSnapshot`; the runtime builds a
   snapshot for the new revision and workers adopt it.
5. Queries and events tell clients what changed; stale events are ignored or
   requeried by revision.

## What is implemented, what is planned

The product surface is incomplete; treat every capability claim as belonging to
its feature document, and recheck source and runtime before relying on it.

- Implemented: project create/open/save/recovery, media import and relink,
  timeline tracks/clips with exact edits, markers and snapping, typed
  transforms, text and captions, selected audio controls, preview transport on
  desktop, Matroska/WebM export, local IPC and CLI parity, and a desktop
  assistant panel over ACP plus `or mcp`.
- Planned or partial: hardware decode/encode, proxies as a user workflow, AI
  providers, plugins, OTIO/Lottie interchange, cross-platform parity evidence.

## Architecture change rule

Do not introduce a second orchestrator, service, database, editing engine, or
project state. A significant or hard-to-reverse change (new subsystem, render or
media engine replacement, new persistent format) requires an ADR in `docs/adr/`
and measured evidence.

## Related

- [control-plane.md](control-plane.md) — commands, queries, history, persistence, IPC, CLI
- [realtime-plane.md](realtime-plane.md) — snapshots, frames, preview, audio clock
- [media-pipeline.md](media-pipeline.md) — FFmpeg, probing, cache, jobs, encoding
- [project-format.md](project-format.md) — `.orproj` contract
- [platforms.md](platforms.md) — targets, packaging, permissions
- [security-boundaries.md](security-boundaries.md) — trust boundaries
- ADRs: [docs/adr/README.md](../adr/README.md)
