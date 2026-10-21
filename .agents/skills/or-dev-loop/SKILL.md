---
name: or-dev-loop
description: Operate OR's existing bounded Linux or Android development lane, diagnose its host preconditions, or select a scoped native UI run. Exclude ordinary unit/source checks that already have a known command, product implementation, generic performance investigation, planning, claims and release publication.
---

# OR bounded runtime tools

[TOOLING.md](../../../docs/TOOLING.md) owns commands;
[TESTING.md](../../../docs/TESTING.md) owns check selection;
[AGENT-AUTHORITY.md](../../../docs/AGENT-AUTHORITY.md) owns standing host grants.
Read the needed lane sections only. An already explicit runtime request suffices;
a skill never grants execution on its own.

1. Establish source/build, tree changes and the smallest check that answers the
   request. Do not run the full Linux/Android ladder for an unrelated prose edit.
2. Probe tools, memory/disk, shared native lock and required media prefix. Keep
   existing floors/bounds; unavailable requirements are BLOCKED with probe output.
3. Reuse run_linux_ui_preflight.py --exec for prepared Linux runs and or_ui.py
   for semantic scenarios. run_local_lane.sh selects full linux/android lanes;
   --packaged adds local Debian extraction/journey evidence, never apt acceptance.
4. Build Android case APKs before starting a dedicated guest. Retain actual
   SAF/provider tests for native picker claims. Stop only owned jobs/guests;
   leave shared caches, unrelated Gradle/Kotlin processes and services alone.
5. Collect logs, report, hashes and cleanup. Existing local lanes classify dirty
   runs as DIRTY; an attributable working-diff diagnostic may expose a failure,
   but never becomes exact-commit product acceptance.

Stop at the selected result. Do not repeatedly rerun passing or unchanged failed
lanes. Use verify-debug's cause-based repair method only if repair is authorized.
Report PASS/FAIL/BLOCKED/UNVERIFIED, source/diff/build, platform/backend/mode,
executed actions, evidence and missing lanes. Local evidence is not ACCEPTED;
hosted platform-verification remains the acceptance authority.
