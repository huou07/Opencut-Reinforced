# Archived skill: dont-giveup-soon

Status: **archival data, not operational authority.** This document is not an
agent skill. It is not discoverable, cannot be activated, and grants no tools or
permissions. Nothing below is current instruction; the live equivalent is the
shared skill at `/srv/agent-workbench/skills/sustained-repair/`, routed by
`/srv/agent-workbench/skills.md`. Host policy and this repository's `AGENTS.md`
govern execution.

Archived: 2026-10-14. Reason: the procedure was promoted to the shared library
after a duplication audit found six of its eleven clauses already present in
host policy, `verify-debug` and `project-steward`. Keeping an activatable copy
here would have produced a drifting parallel copy.

Provenance:

| Fact | Value |
| --- | --- |
| Original source | `/Users/meomeo/.agents/skills/dont-giveup-soon/SKILL.md` (owner macOS machine) |
| Imported into this repository | 2026-10-14, byte-for-byte, SHA-256 verified on both copies |
| Verbatim content SHA-256 | `b4a3676bda3543aa56ff879adf8995b4fc13dd337b66269e94366871857f1c07` |
| Activatable path while active | `.agents/skills/dont-giveup-soon/SKILL.md` |
| Commit that added it | `7ab59780a238b1880c8c2a2ab8e278d75991a8f7` (`docs(agents): add project dont-giveup-soon skill`) on `feat/semantics-journey-guards`, not pushed at the time of archiving |
| Shared snapshot | `/srv/agent-workbench/instruction-history/2026-10-14-sustained-repair-promotion/` |

Retrieval: the exact original bytes remain in that commit and in the shared
instruction-history snapshot. This repository's copy of the original file is
intentionally removed so no activatable duplicate survives the promotion.

---

## Verbatim archived content

```markdown
---
name: dont-giveup-soon
description: Persist through ordinary bugs, failed checks, regressions, and runtime problems by improving the diagnosis instead of abandoning the requested outcome or retrying blindly. Use for sustained implementation, repair, migration, and verification work; stop for genuine authority, safety, or external capability limits.
---

# dont-giveup-soon

Persistence means finishing the authorized outcome with better evidence and
methods. It does not mean infinite retries, widening scope, or ignoring a real
boundary.

## Work through failure

- Treat an ordinary test, build, packaging, CI, performance, or UX failure as
  work to diagnose and repair. Preserve the original requirement and acceptance
  threshold.
- Capture the exact failure, identify the affected boundary, and form a
  falsifiable cause. Reproduce it or add a test that distinguishes that cause
  from nearby alternatives before making another repair.
- After two speculative repairs to the same subsystem or gate, stop guessing.
  Gather better evidence, change the hypothesis or reproduction method, and
  make another change only when its causal effect can be checked.
- Re-run the checks that prove the changed claim. Do not treat a green lower
  layer as proof that a packaged or user-facing workflow works.
- Never weaken tests, documentation, security, accessibility, performance
  targets, product scope, or architecture to manufacture progress.

## Keep the outcome recoverable

- Keep the authorized objective and boundaries active across turns. Use the
  product requirements as traceability, not as mandatory implementation or
  prompt boundaries.
- Prefer the smallest coherent change that advances the actual product.
  Preserve real technical dependencies and parallelize only when the measured
  coordination benefit is clear.
- Commit verified progress atomically at safe handoff points. Before context or
  quota becomes unsafe, leave a precise durable handoff: objective, branch and
  exact HEAD, completed and current work, decisions and invariants, checks and
  evidence, known limitations, and the next action.
- On quota exhaustion or interruption, hand off cleanly rather than spending
  the last context on unfinished speculative work. Resume from the durable
  state and exact Git identity; do not rediscover completed work unnecessarily.

## Stop only at a real boundary

Stop or ask the operator only for a decision that exceeds the existing
authorization, a required credential or capability that cannot be safely
obtained or substituted, or a proven contradiction in authoritative
requirements. Do not stop merely because work is long, difficult, temporarily
failing, or requires another diagnosis.
```

## Audit outcome against the promoted shared body

| Archived clause | Disposition |
| --- | --- |
| Preserve requirement and acceptance threshold | Kept as shared preconditions |
| Falsifiable cause and discriminating reproduction | Already in host policy and `verify-debug`; shared skill points to them |
| Two speculative repairs, then change method | Already in host policy and `verify-debug`; shared skill restates only as a pointer |
| Green lower layer does not prove the packaged journey | Kept |
| Never weaken tests, security, accessibility or performance | Already in host policy and `verify-debug`; restated as a fail-closed authority clause |
| Requirements as traceability, not prompt boundaries | Project-specific; already covered by this repository's `AGENTS.md` |
| Parallelize only on measured coordination benefit | Kept |
| Atomic commits at safe handoff points | Already in host policy; kept as one line |
| Durable handoff before context or quota pressure | Kept; unique to the shared library |
| Resume without rediscovering completed work | Kept |
| Do not stop merely because work is long or temporarily failing | Kept; unique to the shared library |
