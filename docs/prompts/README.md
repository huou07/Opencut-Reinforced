# Prompt artifacts

Purpose: how a work-selection prompt is written, routed and retired here.

Authority: navigation and convention only. What to do next comes from
[PRODUCT_ROADMAP.md](../PRODUCT_ROADMAP.md); what an agent may do without asking
comes from [AGENT-AUTHORITY.md](../AGENT-AUTHORITY.md); permanent rules are
[AGENT_POLICY.md](../AGENT_POLICY.md).

Last reviewed: 2026-10-18.

## One current plan at a time

Exactly one prompt file is the current work selection. It is listed in
[docs/INDEX.md](../INDEX.md) under "Current work selection". Today that is
[OR-BUILD.md](OR-BUILD.md) — one prompt for the whole program, with phases
inside it rather than one prompt per phase.

A prompt is not a second roadmap. It selects and bounds work that
`PRODUCT_ROADMAP.md` already names, and it must not contradict the roadmap, the
product requirements or a permanent invariant. When it does, the roadmap wins and
the prompt is corrected in the same commit as the evidence that exposed the
conflict.

## What a prompt must contain

- **Owner intent** — one dated statement of what the owner asked for.
- **Verified starting state** — branch, HEAD, clean or dirty tree, gate results
  and their freshness; every row a claim with the command that rechecks it.
- **Outcome** — one observable completion, not a topic.
- **Write scope** — the files or subsystems the work may touch.
- **Acceptance** — the command, artifact or observation that closes it.
- **Stop** — what makes a cluster `BLOCKED` or `UNVERIFIED` rather than endlessly
  attempted.
- **Non-goals** — what the work must not grow into.

Cluster ids inside a prompt (`C0`, `C1`, ...) are local to that prompt. They are
handoff labels, not roadmap rows, and they are not stable across prompts.

## Retirement

A prompt whose work has landed, or which a newer prompt supersedes, moves to
`docs/archive/prompts/` with its dated content intact. Do not rewrite an archived
prompt to match today: it records what was asked then. Archived prompts are read
for method, never for current facts.

Before archiving, move the still-durable facts into the authority that owns them:
landed clusters into [PRODUCT_ROADMAP.md](../PRODUCT_ROADMAP.md) with exact source
and evidence class, host and toolchain facts into
[AGENT-AUTHORITY.md](../AGENT-AUTHORITY.md), commands and limits into
[TESTING.md](../TESTING.md) or [TOOLING.md](../TOOLING.md).
