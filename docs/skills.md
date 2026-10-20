# OR skill routing

Authority: navigation and instruction-contract expectations. Execution authority
stays with the request, AGENT_POLICY and host/session permissions.
Last reviewed: 2026-10-20.

Project-local procedures live in [.agents/skills/](../.agents/skills/).
They add OR-specific traps and references; the development loop is maintained
once in DEVELOPMENT_WORKFLOW. Read task-relevant sections only. A readable file
is not proof of discovery in a particular session.

| Primary request | Project procedure | Exclusions |
|---|---|---|
| Implement canonical Rust editing, history, lifecycle or CLI parity | or-edit-feature | UI-only changes, failing checks, codecs/providers, planning |
| Implement/review a visible OR interaction or accessibility journey | or-ui-ux | Backend-only work, generic performance diagnosis, build failure, prose maintenance |
| Operate an existing bounded native Linux/Android lane or resolve its prerequisites | or-dev-loop | Ordinary unit checks with a known command, product design, claims, publication |
| Verify/implement import/export compatibility, media/DSP, providers or MotionScene | or-media-ai | Plain timeline editing, UI-only work, host lane mechanics |
| Record/audit an exact-source hosted completion claim | or-claim-evidence | Routine test reports, implementation, execution, general readiness |
| Explicit release verification or scoped packaging/release workflow changes | or-release | Local package diagnostics, general status, source-only checks; no implied publication |

Select one primary procedure. Add another only for a distinct task boundary;
do not recursively load all referenced skills. Shared procedures supplied by the
current session own generic diagnosis (verify-debug), investigation
(code-investigate), security review (security-review), instruction maintenance
(skill-maintainer), planning (task-planning), status (project-steward), prompt
artifacts (prompt-assist) and sustained work (sustained-repair). Generic UI review
methods may inform OR review, but loading or-ui-ux does not automatically require
loading another skill. A typo needs no project skill or product gate.

Scope remains discriminating: review/analysis/plan produces its artifact and
stops. Implement only a requested change; a skill is not authority to repair a
defect noticed during review. Stop at the scoped observations or verified change;
report source, actual outcome, evidence and unrun targets. No routine checklist
requires repeated planning, full-platform verification or unrelated changes.

[Routing cases](../scripts/skill_routing_cases.json) cover UI, Rust editing,
media compatibility, measured performance failure, release verification and
neighboring no-skill/read-only cases. Run:

```sh
python3 scripts/test_skill_contracts.py
```

This validates reference existence, narrow descriptions, stopping/output
contracts, size bounds and the reviewed expectation corpus. It does not execute
a model classifier or prove live discovery. A live trial must record the actual
session's catalog, selected skills, response and scope obedience; do not spawn
agents merely to label a structural check as behavioral evidence.

Never copy AEH's shared procedures here or expose OR skills host-wide.
The retired dont-giveup-soon procedure remains historical data in
[evidence/dont-giveup-soon-archived-skill.md](evidence/dont-giveup-soon-archived-skill.md).

OR remains registered as `or` in AEH. Resolve it through `hub.py where or`;
read-only status never executes project code. AEH indexes evidence; product
state, architecture, priorities and acceptance stay in this repository.
