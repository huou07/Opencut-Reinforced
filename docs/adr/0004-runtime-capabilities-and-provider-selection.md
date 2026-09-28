# ADR 0004 — Runtime capabilities and provider selection

- Status: Accepted
- Date: 2026-09-28

## Context

Hardware, codecs, GPU backends, local model runtimes, and cloud services vary
by machine, platform, permissions, and license. Scattered capability checks
would make behavior nondeterministic and make fallbacks difficult to verify.

## Decision

Centralize capability discovery, policy, provider/model selection, budgets, and
fallback decisions. Runtime adapters advertise typed capabilities; domain
evaluation consumes platform-neutral policies. Selection is explicit and
observable, and every hardware path retains a correctness-preserving software
or CPU fallback where possible.

## Alternatives rejected

- Platform checks scattered through project/domain code.
- Assuming a particular GPU, codec, model runtime, or cloud provider.
- Treating acceleration as required for project correctness.

## Consequences

Capability manifests and evidence become part of runtime acceptance. Performance
optimization waits for measurements. Provider/model licenses and credentials
must be reviewed independently from library licenses.
