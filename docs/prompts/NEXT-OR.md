# NEXT-OR — continue under the quality system

Authority for this work: [../AGENT-AUTHORITY.md](../AGENT-AUTHORITY.md). This
prompt is an owner instruction from 2026-10-17 and does not grant anything that
file forbids. It explicitly authorises the protected-gate changes listed below,
which is the only reason they are in scope.

## What changed before you start

A reorganization ran while your session was paused. Read these first:

- [../docs/QUALITY.md](../docs/QUALITY.md) — OR's definition of good, its
  attributes and its known gaps.
- `~/AEH/docs/quality-system.md` — the process this repository now follows:
  three gates, one evidence ledger, loadable skills, quality bar, deletion policy.

Two gates now exist and mean different things:

| Gate | Command | Proves |
|---|---|---|
| verify | `bash scripts/check-repo.sh` | hygiene, documents, claims, dependency boundary |
| acceptance | `bash scripts/acceptance.sh` | the above, plus the documented Rust lanes and the Flutter suite |

Run the acceptance gate at least once before your first commit and once at the
end. `hub.py verify or --gate acceptance` records it. A skipped or unavailable
check is `UNVERIFIED`, never PASS. The Flutter lane reports UNVERIFIED on this
host because the native-assets hook cannot download Rust components; do not turn
that into a false pass.

## Do these, in order

1. **Harden `scripts/check_claims.py`.** Today it validates only the SHA shape and
   field presence, so a fabricated receipt passes. Require the implementation SHA
   to exist in Git (`git cat-file -e <sha>^{commit}`) and to be an ancestor of
   HEAD, and require the hosted verification identity (workflow, job, run id).
   Add a fixture that fails on a fabricated SHA so the guard cannot regress.
2. **Stop the silent skips.** Wire `scripts/test_android_preview_runner.py` and
   `scripts/test_verify_android_saf_recovery.py` into the gate, or delete them
   with a recorded reason. A suite that no gate runs is dead weight.
3. **Complete the document gates.** Add `docs/adr/0008-*.md`,
   `docs/PAUSE_MIGRATION_HANDOFF.md`, `docs/prompts/*.md` and
   `docs/evidence/PRODUCT_ROADMAP_HISTORY.md` to `check-repo.sh` required files;
   extend `check_doc_freshness.sh` to every current-claim document (at minimum
   README, DESIGN, RELEASE, TOOLING, PROJECT_FORMAT, OPEN_SOURCE_CONVERGENCE,
   AGENT-AUTHORITY); normalise `Date:` to `Last reviewed:` so the check can read
   it; add the four missing documents to `docs/INDEX.md`.
4. **Make the preflight safe to interrupt.** `scripts/run_linux_ui_preflight.py`
   rewrites the tracked `crates/or_app_bridge/rust-toolchain.toml` and restores it
   only in `finally`; a SIGKILL leaves an unattributed dirty tree. Add a clean
   precondition (refuse to run on a dirty tree), a backup/restore that survives a
   kill, and a `--plan` dry run. Split `pub get` out of the 600s native budget.
5. **Close the secret-scan gap.** Either run gitleaks (pinned) in
   `repo-hygiene.yml` and in the acceptance gate when it is installed, or state in
   `docs/TOOLING.md` plainly that no automated secret scan backs the local gate.
   Do not leave the current ambiguity.
6. **Performance.** Either add one enforced budget (a frame-time or memory
   ceiling measured by a script that fails on regression) or record in
   `docs/QUALITY.md` that OR has no performance budget and why. A prose budget is
   not a budget.
7. **Reconcile the release story.** `docs/AGENT-AUTHORITY.md` cites version,
   artifact, health and rollback gates in `docs/RELEASE.md` that do not exist.
   Either add them or delete the reference.

## Rules

- Work on the current branch (`feat/semantics-journey-guards`) unless the owner
  says otherwise. Do not force-push, rewrite published history, merge protected
  branches, or infer publication authority from a green gate.
- One coherent change per commit, with the gate result in the message.
- Never weaken an assertion, scanner or environment boundary to obtain PASS.
- Record evidence in the project's existing location with the exact SHA; do not
  start a second journal.
- Report `IMPLEMENTED`, `VERIFIED_*`, `ACCEPTED` and `BLOCKED/UNVERIFIED`
  separately, and name what you did not run.

## Stop when

Every item above is either done with recorded evidence, or reported as blocked
with the exact missing capability. Then hand off: branch, HEAD, gate result,
unverified items, next action.
