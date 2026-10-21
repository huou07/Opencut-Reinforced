---
name: or-claim-evidence
description: Record or audit an OR hosted completion claim binding exact source to a run/job, or reconcile an existing claim. Exclude ordinary test reports and implementation, test execution, general status, release readiness and publication.
---

# OR durable hosted claims

[evidence/claims/README.md](../../../docs/evidence/claims/README.md) owns schema,
supersession and evidence classes. Read its relevant section; do not duplicate
the schema or convert every local test into a claim.

1. Confirm exact implementation SHA exists in Git and resolve the actual hosted
   run and job for that source. Another SHA is STALE.
2. Inspect assertions, actions, skipped lanes and artifacts against the named
   outcome. Green CI alone is not usability, native picker, playback or release
   approval. Classify missing required evidence as BLOCKED.
3. Preserve semantic/behavioral/visual/native scope and state explicit limits.
   Never invent run IDs, promote local debug evidence or edit unrelated claims.
4. Record through the existing schema/path. record-hosted-evidence.sh writes
   ignored runtime state; committed claim records are the durable artifacts.
   Supersede using the documented links, preserving historical observations.
5. Validate with check_claims.py. Syntax and ancestry do not authenticate what
   the test actually did: inspect the run/artifacts separately.

Stop once the named claim is recorded/audited; do not run more product work or
publish. Report claim, exact source, run/job, observations, verdict and gaps.
