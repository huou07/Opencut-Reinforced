# Architecture Decision Records

These records capture decisions that are difficult to reverse. They supplement
the machine execution plan; they do not replace `docs/execution/PLAN.json`,
`STATE.json`, or the permanent invariants.

## Records

- [0001 — Control and realtime planes](0001-control-and-realtime-planes.md)
- [0002 — wgpu render spine and native interop](0002-wgpu-render-spine-and-native-interop.md)
- [0003 — Frame memory domain and ownership](0003-frame-memory-domain-and-ownership.md)
- [0004 — Runtime capabilities and provider selection](0004-runtime-capabilities-and-provider-selection.md)
- [0005 — AI provider boundary](0005-ai-provider-boundary.md)
- [0006 — Autonomous agent execution contract](0006-autonomous-agent-execution-contract.md)
- [0007 — Declarative MotionScene and procedural isolation](0007-declarative-motion-scenes-and-procedural-isolation.md)
- [0008 — Product acceptance and execution quality](0008-product-acceptance-and-execution-quality.md)
- [0009 — Model orchestrator V2 (frozen candidate)](0009-model-orchestrator-v2.md)

All records are architecture intent. A later implementation checkpoint must
provide its own dependency, platform, licensing, test, and migration evidence.
