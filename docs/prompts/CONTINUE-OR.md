# Continue OR product development

Deliver a reliable first-project editing journey in OR, fixing the first reproducible user-facing gap and continuing through the active product roadmap with exact-source acceptance evidence.

## Start from the repository

Work in /home/huou07/Projects/OR. Read AGENTS.md, then only the relevant sections
of docs/AGENT_POLICY.md, docs/PRODUCT_ROADMAP.md, docs/TESTING.md and
docs/UX_ACCEPTANCE.md. Load matching skills through the current harness's actual
loader. Establish current branch, HEAD, worktrees and dirty files before editing.

Dated inspection, 2026-10-15: branch feat/semantics-journey-guards, HEAD
f409cb90d312b3f1cdc52cfc8205f46a9c94baff. docs/TOOLING.md and
rust-toolchain.toml contain pre-existing work. The native preflight cache helper
was subsequently repaired during handoff preparation. Recheck the actual tree;
do not discard, blindly stage or publish unrelated changes. The public Actions
API returned no runs for that inspected HEAD; older roadmap runs do not validate
this tree.

Dated inspection, 2026-10-17 (paused mid-session): branch
feat/semantics-journey-guards, HEAD 16b74e1f56b00868ad1d968700f0301feb0054e6, in
sync with origin apart from the local handoff commit that records this note.
`git status --short` was `?? docs/prompts/FINISH-OR.md`, the session prompt
written by the harness and not this worker's file to commit; nothing else was
dirty and no background job was left running. Landed this session: 726f194 runs
the bounded Linux preflight from a writable project-local Flutter SDK overlay
because the shared SDK cache is read-only under a workspace-restricted sandbox,
and decides cache reuse with a real write probe because `os.access` reports
those ACLs as writable; 5255a0d keeps already-rendered media previews when a
later 50-item library page loads; 6232528 records that fix's evidence and
corrects a stale UX-acceptance sentence; 16b74e1 restores real overflow
detection on the 320x640 create-project guard, whose committed probe replaced
`FlutterError.onError` and so could never fail. The last completed command was
`python3 scripts/run_linux_ui_preflight.py` (deferred behind a headroom watcher
while an unrelated `h07p-lab` VM held ~2.4 GB): the Linux debug journey passed in
8 s after the overlay build, exit 0, recorded in
apps/or_app/build/ui-preflight/run.json. Hosted CI for 16b74e1 was still running
when the owner paused: Repository hygiene and Linux UI preflight had passed, and
Platform verification run 39018753306 was in progress, so its macOS, Windows and
Android jobs are unconfirmed for that SHA. PR #15 remains open and unmerged. Next
action: read `gh run view 39018753306`, repair a genuine failure if one appears,
otherwise continue to the next roadmap journey — the broader offline-media and
bulk-relink workflow is still unimplemented, and real-user feedback, Android
transcription and physical-device responsiveness remain unverified.

Dated inspection, 2026-10-17 (second session, owner-paused under
`docs/prompts/NEXT-OR.md`). **This entry supersedes the "Next action" above**:
that instruction to read `gh run view 39018753306` is obsolete, because the
reorganization added commits and the NEXT-OR task replaced that queue. Branch
feat/semantics-journey-guards, HEAD
1ae03db2552af9cb9a63b0d9de5ed29927ec1840, `git status --short` empty, tree clean,
11 commits ahead of `origin` and **not pushed** (the owner forbade pushing in
both pauses). The NEXT-OR scope is complete except for one verification run; all
seven of its items are committed:

- 14175b4 reports truthful gate results: it repairs the broken link that had left
  HEAD's verify gate red, and teaches `scripts/acceptance.sh` to classify "the
  Flutter lane could not start" as UNVERIFIED with the exact reason instead of
  FAIL, while still failing any suite that actually starts and fails.
- 7517e6a hardens `scripts/check_claims.py`: `implementation_sha` must exist in
  Git and be an ancestor of HEAD, `verification` must name workflow/jobs/run_id,
  and a shallow or absent repository is reported as an unavailable check.
  `scripts/test_check_claims.py` feeds it a fabricated SHA, a real commit outside
  HEAD history, missing hosted identities, a short SHA and a non-repository, and
  `repo-hygiene.yml` now checks out full history (`fetch-depth: 0`).
- aa7dabf runs the two orphan Android suites (`test_android_preview_runner.py`,
  `test_verify_android_saf_recovery.py` — the latter as `python3 -m`, which is
  why it never ran) from `check-repo.sh`.
- 2dbd8d2 extends required files and doc freshness, normalises `Date:` to
  `Last reviewed:`, dates the documents that lacked it and indexes five missing
  documents. Host note: the repository root carries the sticky bit and is owned
  by `huou07`, so root-level files (README.md, DESIGN.md) can only be written in
  place from this worker UID, never replaced by rename.
- f2d364f makes the preflight interrupt-safe: clean-tree precondition,
  startup recovery from `apps/or_app/build/ui-preflight/rust-toolchain.toml.original`,
  a `--plan` dry run, and a separate 300 s `pub get` stage before the 600 s
  native budget.
- b3bddc3 closes the secret-scan gap: `acceptance.sh` runs `gitleaks detect` when
  installed and reports the lane UNVERIFIED when not, and `repo-hygiene.yml`
  downloads gitleaks 8.30.1 and verifies its SHA-256 before running it.
- adc2e13 records in `docs/QUALITY.md` why OR has no performance budget and what
  would be needed, and 1ae03db removes the release gates `AGENT-AUTHORITY.md`
  claimed but `RELEASE.md` never defined.

Last completed command: `bash scripts/acceptance.sh` ->
`PROJECT_ACCEPTANCE=PASS with 1 UNVERIFIED lane (Flutter widget tests: shared
Flutter SDK cache not writable by this user)`, with the repository, both Rust
lanes and the new gitleaks lane PASS. Unfinished: the real
`python3 scripts/run_linux_ui_preflight.py` run under the interrupt-safe code.
It never started because the host stayed below the preflight's own 6 GiB
MemAvailable guard (an unrelated `h07p-lab` qemu VM holds ~2.4 GB);
`--plan` correctly reported `BLOCKED: insufficient RAM or disk headroom` and
changed nothing. Its logic is covered by 16 unit tests, but the end-to-end run
is UNVERIFIED. Next action: once MemAvailable is at least 6 GiB and the tree is
still clean, run `python3 scripts/run_linux_ui_preflight.py --plan` and then the
real preflight, then push the branch and let hosted CI decide.

## Implement the next useful outcome

The active roadmap identifies ordinary-user feedback as the highest-value gap.
Use the existing create/import/edit/preview/save/reopen/export flow and lawful
small fixtures. Reproduce installation/first-project friction, unsupported-media
feedback, permission/relink recovery or responsiveness before choosing a fix.
Use actual owner feedback when available; never invent dogfood observations.
If no defect is reproduced, report the observed journey and remaining human
feedback instead of inventing a feature.

Make the smallest coherent improvement through existing application commands.
Preserve canonical Rust project state shared by UI, CLI and agents, Focused
Monochrome, accessible keyboard alternatives, recovery and declarative motion.
Keep domain logic out of Flutter presentation. Do not restart the paused legacy
checkpoint framework, create another roadmap or introduce an engine migration
without measured need. Android transcription and physical-device responsiveness
are not established by an emulator pass.

## Execute and verify within the installed lanes

The owner authorizes cohesive commits, scoped pushes and stable publication to
registered destinations after applicable exact-source CI, version, artifact,
health and rollback gates. This standing authorization supersedes older blanket
session prohibitions; it grants no credentials, branch-protection bypass, host
sudo or permission-management capability.

On this host, use only enabled owner-installed test profiles through
/usr/local/bin/aehctl. Inspect status first; run or-linux serially with other
heavy work. Its first installed attempt failed on read-only Flutter cache and
Git ownership before the native journey. The reviewed repair requires owner
installation from the Hub's ops/product-verification/UPGRADE.md. Verify actual
status/result after that update; do not execute source installers, edit protected
configuration, or run the journey outside the profile to bypass a failure.
Disabled Android profiles need provisioned inputs and owner registration.

Keep fresh memory/disk headroom, timeouts and complete process-group cleanup.
Use isolated synthetic data; never inspect owner credentials or private media.
Request only the smallest concrete owner action if an installed capability is
missing, and continue independent product work.

Run bash scripts/check-repo.sh for repository hygiene. Use existing Rust,
Flutter, semantic/widget and native journey gates appropriate to the change,
then required hosted platform/package workflows for the exact final source.
A Linux debug app with a fixture picker is not packaged-install, OS-picker,
cross-platform or physical-device acceptance. Verify current package/artifact
availability rather than relying on old expiry dates.

## Finish each slice and continue

A slice closes when the reproduced defect is repaired, its real journey and
applicable required CI jobs pass for the exact source, and regressions are
covered without weakening assertions. Commit coherent reviewed changes, push
through the existing scoped route, inspect actual CI results, and update only
the active roadmap and relevant claim records with source/run/artifact pointers
and limitations. After two speculative repairs, state a falsifiable hypothesis
and run a discriminating check.

Report implemented, verified evidence classes, accepted, blocked and unverified
separately. Continue to the next justified roadmap outcome; stop only at an
actual decision/capability boundary and provide the precise evidence and next
action. Stable publication additionally requires the registered release route;
a green local gate or an older successful workflow is insufficient.
