---
name: or-claim-evidence
description: Record or audit an OR completion claim — a machine-readable record binding an exact implementation SHA to the hosted run and job that verified it, plus what the record does not establish. Use when a capability is claimed done, when recording a hosted run, when reconciling an existing claim, or when deciding what evidence class a result carries. Do NOT use for running local lanes (or-dev-loop), for publishing a release (or-release), or for ordinary implementation work (or-edit-feature).
---

# OR claim evidence

You turn a verified result into a record a later reader can check, or you audit
one that already exists. You never promote local or model-reported results into
product acceptance.

## Preconditions

- Read [evidence/claims/README.md](docs/evidence/claims/README.md) — it owns the
  schema and the field meanings — plus [TESTING.md](docs/TESTING.md) and
  [QUALITY.md](docs/QUALITY.md) for evidence classes and the quality bar.
- Confirm the exact implementation SHA and the hosted run you are about to cite.
  Do not cite a run at a different revision: CI green at another SHA is `STALE`,
  not evidence.

## Rules

- One JSON file per verified requirement, named
  `YYYY-MM-DD-<short-requirement-id>.json`, in `docs/evidence/claims/`.
- `implementation_sha` must be a full 40-character commit in this repository and
  an ancestor of `HEAD`. `scripts/check_claims.py` resolves it in Git and fails
  the gate otherwise, so a fabricated receipt cannot pass.
- The verification block names the hosted run id, workflow and job, and the exact
  command, with a UTC timestamp.
- A claim keeps its evidence class: `VERIFIED_SOURCE`, `VERIFIED_BEHAVIORAL`,
  `VERIFIED_SEMANTIC`, `VERIFIED_NATIVE`. No class promotes another.
- A missing, skipped or unavailable check is `UNVERIFIED`; it is never `PASS`. A
  model statement is not evidence. A green local run is not acceptance.
- Never edit a claim record for a requirement you did not implement, and never
  edit a claim to make a newer result fit. Supersede it: add a new record with a
  `supersedes` field and set `superseded_by` on the old one. Both stay in the
  directory.
- Every claim states what it does **not** establish — the unsupported platform,
  the emulator-versus-device limit, the debug-package-versus-release limit.

## Recording hosted evidence

After a successful hosted run for the current commit, the repository already
provides the recording path:

```sh
bash scripts/record-hosted-evidence.sh <workflow>
```

That writes `.hub/hosted-evidence.json`, which is runtime state and gitignored.
The control workspace merges it into a verification record only when it names
that exact revision; without it a release check is `BLOCKED` by "no
hosted-runner evidence", which is true rather than a defect. A claim record under
`docs/evidence/claims/` is the durable, committed artifact.

## Auditing an existing claim

Check, in order: does the SHA exist and is it an ancestor of `HEAD`; does the run
id resolve to a real run for that SHA; does the named job exist in that run; does
the claim overstate the journey or the platform; is it superseded; does the
`docs/PRODUCT_ROADMAP.md` row that references it still say the same thing.

## Report

Claim path, requirement id, implementation SHA, run and job, evidence class,
whether it supersedes or is superseded, and the exact limits the record states.

## Session skills

This plugin supplies only the OR-specific contract. Pair it with the session
skills the host exposes: `brainstorming` before changing an interaction model,
`writing-plans` when a change spans several commits, `test-driven-development`
for the regression guard, `systematic-debugging` when behavior is wrong, and
`verification-before-completion` before any done claim. `ui-ux-pro-max`
(`design_recommend` / `design_review` / `design_search`) owns visual and UX
decision advice. Do not re-create those procedures here.
