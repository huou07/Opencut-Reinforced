---
name: or-dev-loop
description: Build and verify OR on the owner-authorized local macOS machine, including focused checks, real app journeys and performance measurements. Use for local verification. Do NOT use to design a feature (or-edit-feature), review a UI (or-ui-ux), record hosted claims (or-claim-evidence), or publish (or-release).
---

# OR local macOS loop

Use the cheapest check that answers the question, then exercise the real app for
user-visible claims. This procedure is scoped to the current local Mac mission;
other platform and hosted procedures remain in TOOLING.md for future work.

## Procedure

1. Record branch, HEAD, dirty state and the source/build being checked. Preserve
   unrelated changes and use a separate test profile.
2. Recheck available Flutter, Rust, Xcode, disk/memory and FFmpeg tools before
   building. Do not reuse the old Debian host facts as Mac facts.
3. Run the affected Rust, Flutter or repository checks. Use the focused package
   or test first; run the full repository gate only for a coherent increment.
4. From apps/or_app, use Flutter's macOS target to build or launch the app:
   flutter build macos --debug
   flutter run -d macos
   Use profile or release mode for performance measurements, not debug mode.
5. Perform the affected interaction in the visible app. Verify canonical project
   state, save/reopen and output as appropriate. For exported media, inspect
   streams with ffprobe, decode the file, and open or play it.
6. Record source revision, build mode, machine, fixture, codec/backend,
   measurements, output path and limits. Commit verified coherent work locally.

## Evidence

Classify source tests, semantics, real behavior, visual inspection, native build
and performance separately. A local result proves only the tested source and
workflow on this Mac. Do not claim hosted, cross-platform, release or untested
behavior.

Do not push, open PRs, trigger remote CI, publish or deploy during the current
mission. Do not install system toolchains or change machine security settings as
a build workaround.
