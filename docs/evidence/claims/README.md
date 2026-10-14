# Claim evidence

Purpose: machine-readable proof that a product requirement was actually
verified, replacing the paused checkpoint ledger.

Authority: this directory is evidence, not a work selector. It records what was
verified and when. What to do next lives in [../../PRODUCT_ROADMAP.md](../../PRODUCT_ROADMAP.md).

Last reviewed: 2026-10-14

## Why this exists

Before the 2026-10-09 convergence pivot, `scripts/archive/agent_supervisor.py`
bound hosted CI runs to requirement SHAs and wrote one evidence file per
checkpoint. Those receipts are real and are preserved under
[../../archive/execution/evidence/](../../archive/execution/evidence/). The
supervisor is paused: `MISSION.json` records `legacy_execution: PAUSED`, and
`STATE.json` is frozen at `last_updated: 2026-10-09`.

That left a gap. Product work continued, and `PRODUCT_ROADMAP.md` accumulated
exact-SHA verification claims — but with no machine-checkable backing, because
the ledger no longer advanced. These claim records close that gap with a much
smaller mechanism.

## What a claim record is

One JSON file per verified requirement, named
`YYYY-MM-DD-<short-requirement-id>.json`. Fields:

| Field | Meaning |
|---|---|
| `requirement` | Stable requirement id, as used in `PRODUCT_ROADMAP.md` |
| `claim` | One sentence stating what is now true for a user |
| `implementation_sha` | Full 40-char SHA of the commit implementing it |
| `verification` | What proved it: hosted run id, workflow, job, and the exact command |
| `verified_at_utc` | ISO-8601 UTC timestamp of the verifying run |
| `evidence_class` | `hosted` / `packaged` / `local-source` / `emulator` / `unverified` |
| `limitations` | What this record does NOT establish |

A record without `implementation_sha` and `verified_at_utc` is invalid and the
validator rejects it.

## Rules

1. **A model statement is not evidence.** A claim record exists only when a real
   check ran.
2. **The SHA must be the implementation, not the documentation.** A commit that
   only updates docs does not verify a capability.
3. **`evidence_class` is honest.** `emulator` is not `packaged`; `local-source`
   is not platform acceptance. Record what the run actually was.
4. **`limitations` is mandatory in substance.** "Emulator evidence does not
   establish physical-device performance" is a limitation worth recording;
   an empty list means you should ask why.
5. **Do not edit a record for a requirement you did not implement.** A
   correction is a new dated record that supersedes the old one.
6. **Failures are recorded, not hidden.** A failed run that was then repaired
   belongs in `limitations` with its run id, or as its own dated record.

## Validating

```sh
python3 scripts/check_claims.py
```

The validator checks schema, SHA shape, timestamp format, evidence-class
vocabulary, and that every `requirement` appears in `PRODUCT_ROADMAP.md`. It is
part of `scripts/check-repo.sh`, so a malformed record fails repository
hygiene.

## What this does not do

It does not replace the [archived execution
ledger](../../archive/execution/README.md), it does not run CI, and it does not
decide whether a claim is *important* — only whether it is recorded honestly.
A record proves a check ran, not that the product is ready to ship.
