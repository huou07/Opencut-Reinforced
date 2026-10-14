# Execution evidence

These JSON files are immutable historical attestations created by the
execution supervisor. There is one file per checkpoint from the enforcement
boundary in `EVIDENCE_POLICY.json`; historical checkpoints before that
boundary remain grandfathered and do not receive fabricated evidence.

An evidence record binds a checkpoint to:

- the exact implementation commit and subject;
- successful push-triggered GitHub Actions workflows for that SHA;
- every policy-required job in those workflows; and
- Developer Preview release evidence when the checkpoint in `PLAN.json`
  requires it.

Feature checkpoint runners must not create or edit these files. Only the
supervisor writes an evidence record after independently verifying hosted
evidence, and the record is committed together with the supervisor-owned
state transition. Records must not contain tokens, credentials, machine-local
paths, or runner prose.

## Quality evidence from 9B

From 9B onward, schema-2 records include `evidence_classes`. Every
`required_evidence_classes` entry in `PLAN.json` must have the exact hosted
workflow job and successful named step specified by `EVIDENCE_POLICY.json`.
Each proof records its class, gate/run, job ID, step name/number, and success.
The supervisor fetches those job steps for the exact implementation SHA.
Historic schema-1 records are retained as issued and do not claim these new
classes retroactively.
