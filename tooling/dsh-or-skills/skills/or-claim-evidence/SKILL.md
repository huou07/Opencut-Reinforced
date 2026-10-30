---
name: or-claim-evidence
description: Record or audit an OR completion claim — a machine-readable record binding an exact implementation SHA to the hosted run and job that verified it, plus what the record does not establish. Use when a capability is claimed done, when recording a hosted run, when reconciling an existing claim, or when deciding what evidence class a result carries. Do NOT use for running local lanes (or-dev-loop), for publishing a release (or-release), or for ordinary implementation work (or-edit-feature).
---

# OR claim evidence

Turn a verified result into a record a later reader can check, or audit one that
exists. Never promote local or model-reported results into product acceptance.

## Preconditions

- Read `docs/evidence/claims/README.md` (schema and field meanings),
  `docs/reference/testing.md` and `docs/reference/quality.md`.
- Confirm the exact implementation SHA and the hosted run you will cite. A green
  run at another SHA is `STALE`, not evidence.

## Rules

- One JSON file per verified requirement, named
  `YYYY-MM-DD-<short-requirement-id>.json` under `docs/evidence/claims/`.
- `implementation_sha` is a full 40-character commit in this repository and an
  ancestor of `HEAD`; `scripts/check_claims.py` resolves it in Git and fails the
  gate otherwise.
- The verification block names the hosted run id, workflow, job, exact command
  and a UTC timestamp.
- A claim keeps its class (`VERIFIED_SOURCE`, `VERIFIED_BEHAVIORAL`,
  `VERIFIED_SEMANTIC`, `VERIFIED_NATIVE`); no class promotes another. Missing or
  skipped checks are `UNVERIFIED`, never `PASS`.
- Never edit a claim you did not implement, and never edit a claim to fit a newer
  result: add a record with `supersedes` and set `superseded_by` on the old one.
- Every claim states what it does **not** establish — unsupported platform,
  emulator-versus-device, debug-package-versus-release.

## Recording hosted evidence

```sh
bash scripts/record-hosted-evidence.sh <workflow>
```

That writes gitignored `.hub/hosted-evidence.json`; the committed claim record is
the durable artifact.

## Auditing

Check: SHA exists and is an ancestor of `HEAD`; the run resolves for that SHA;
the named job exists in that run; the claim does not overstate journey or
platform; supersession is correct; the roadmap row still says the same thing.
