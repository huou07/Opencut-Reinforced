---
name: or-dev-loop
description: Build, test and verify OR quickly on this Debian workstation using the local Linux lane and the local Android emulator lane, and decide when the evidence must instead come from hosted GitHub Actions. Use when running local build, test, benchmark or preflight work, when a lane reports BLOCKED, or when deciding whether a local result is admissible. Do NOT use for publishing a release (or-release), for writing or auditing claim records (or-claim-evidence), or for adding an editing capability (or-edit-feature).
---

# OR local development loop

You run the cheapest check that can answer the question, on the right lane, and
you report the evidence class honestly. A local run is developer evidence; it
never becomes accepted product evidence.

## Preconditions

- Work on a clean tree. The Linux preflight refuses a dirty tree, and a run on a
  dirty tree can only report `DIRTY`, never `PASS`.
- Establish branch, HEAD and the current gate results first. Do not trust a
  remembered state.
- Read [TOOLING.md](../../../docs/TOOLING.md), [TESTING.md](../../../docs/TESTING.md) and
  [AGENT-AUTHORITY.md](../../../docs/AGENT-AUTHORITY.md) for the authoritative rules.

## The ladder

1. `bash scripts/check-repo.sh` — hygiene, tracked whitespace, generated paths,
   doc links, doc freshness, claim records and source-identity, the `or_core`
   dependency boundary, and the hermetic report suites. No network.
2. `bash scripts/acceptance.sh` — the source gate plus the documented Rust lanes,
   the Flutter widget suite, `gitleaks` when installed, and the performance
   budget. It writes `.hub/gate-report.json` and names every lane it did not run.
3. `bash scripts/run_local_lane.sh linux` — one entry point that runs hygiene,
   the Flutter widget suite, the bounded Linux preflight and the performance
   budget, and writes `apps/or_app/build/local-lanes/linux.json`.
   `--packaged` adds a local `.deb` build and the packaged journey; that is local
   developer evidence, never packaged acceptance.
4. `bash scripts/run_local_lane.sh android` — the local Android emulator lane.
   It is being consolidated by cluster C2 of
   [docs/prompts/OR-LOCAL-FINISH.md](../../../docs/prompts/OR-LOCAL-FINISH.md); until it
   lands, `scripts/start-android-emulator.sh`,
   `scripts/run-android-preview-check.sh` and `scripts/stop-android-emulator.sh`
   are the mechanism.
5. Push through normal Git and inspect the required GitHub Actions jobs for that
   exact commit. Hosted CI is canonical for native, packaged and cross-platform
   behavior.

## Lane exit codes

`0` every selected step passed on a clean tree · `1` a step failed · `2` wrong
invocation · `3` a step was `BLOCKED` by a host precondition · `4` a step could
not be measured (`UNVERIFIED`) or the tree was dirty.

## Host constraints you must respect

- The host is a 15 GiB workstation. The preflight requires 6 GiB available RAM
  and disk; the Android lane needs roughly 3.3 GiB for the guest plus about
  4.4 GiB for Gradle and JVMs. Check headroom before starting, not after.
- Build the Android APK **before** booting the guest, use a dedicated AVD and
  serial, and always stop what you started.
- The two lanes and the emulator are shared host resources. Serialise them; never
  run two writers in this working tree.
- Stopping a Gradle or Kotlin daemon to free memory is only acceptable when you
  can show it is idle and that you started it. Report what you freed and why it
  was safe.
- `~/.cargo` is not writable: a new Rust dependency needs a writable `CARGO_HOME`
  and is a dependency decision, not a workaround.
- Flutter locally is newer than the CI pin, so a local Flutter pass is not
  exact-CI evidence.

## Evidence classes

`VERIFIED_SOURCE`, `VERIFIED_BEHAVIORAL`, `VERIFIED_SEMANTIC`, `VERIFIED_NATIVE`
are distinct and never promote one another. A local preflight or emulator result
is developer evidence. Hosted `platform-verification` decides native, packaged
and cross-platform acceptance. `LOCAL ENVIRONMENT BLOCKED` and `NOT RUN` are
neither pass nor fail.

## Stop conditions

- A step the host cannot run is `BLOCKED` with the probe output that proves it.
  Never lower a headroom floor, substitute a weaker check, or report `PASS` from
  a dirty tree to make a lane green.
- A guest that never settles inside the bounded probes is `BLOCKED` with the
  measured reason. A deliberate emulator loss is classified
  `EMULATOR_INSTABILITY` and is never retried into a pass.
- After two speculative repairs for the same failing check, stop and diagnose
  with a falsifiable hypothesis and a discriminating observation before editing
  again.

## Report

Lane, exact source SHA, per-step classification, timings, artifact paths and
hashes, the evidence class, every lane not run, and exactly what the run does not
establish.
