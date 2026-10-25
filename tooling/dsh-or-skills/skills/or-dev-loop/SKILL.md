---
name: or-dev-loop
description: Build and verify OR on the owner-authorized local macOS machine, including focused checks, real app journeys, semantic UI driving and performance measurements. Use for local verification. Do NOT use to design a feature (or-edit-feature), review a UI (or-ui-ux), record hosted claims (or-claim-evidence), or publish (or-release).
---

# OR local macOS loop

Use the cheapest check that answers the question, then exercise the real app for
user-visible claims. This procedure is scoped to the current local Mac mission;
other platform and hosted procedures remain in `docs/TOOLING.md` for future work.

## Observe and drive the interface by CLI first

`python3 scripts/or_ui.py` drives the real desktop app by widget key, semantics
label, visible text or role — never by screen coordinate — and writes a step
transcript plus screenshots and semantics snapshots. On this Mac its default
target is `macos`: it builds and launches the real app through
`flutter test -d macos integration_test/or_ui_scenario_test.dart`.

Use it before any coordinate clicking, MCP bridge or screenshot pass. It names
what it acted on, and a 1440x900 window answers in roughly twenty seconds.

```sh
python3 scripts/or_ui.py screens                          # which screens exist
python3 scripts/or_ui.py --viewport 1440x900 tree --name editor
python3 scripts/or_tree_outline.py <tree-*.json>           # one line per node
python3 scripts/or_ui.py --viewport 1440x900 shot --name editor
python3 scripts/or_ui.py act --label "New Project" --role button
python3 scripts/or_ui.py run scripts/ui_scenarios/editor-journey.json
```

Reach for a screenshot, MCP or vision only when semantics cannot express the
question — pure visual hierarchy, colour, or a rendering artefact — and read the
screenshot as the narrow question it answers. A single `act` command relaunches
the app, so a multi-step journey belongs in a scenario file under `run`.

## Procedure

1. Record branch, HEAD, dirty state and the source/build being checked. Preserve
   unrelated changes and use a separate test profile.
2. Recheck available Flutter, Rust, Xcode, disk/memory and FFmpeg tools before
   building. Do not reuse the old Debian host facts as Mac facts.
3. Run the affected Rust, Flutter or repository checks. Use the focused package
   or test first; run the full repository gate only for a coherent increment.
4. Build a macOS app, copy it to a fresh ignored path, stage the linked FFmpeg
   runtime, then launch the copy. Do not rebuild or stage a bundle that is
   currently running. From the repository root:

   ```sh
   flutter build macos --debug
   ffmpeg_prefix="$HOME/Library/Caches/OpencutReinforced/ffmpeg-8.1.3-macos-mp4/ffmpeg-install"
   app_source="apps/or_app/build/macos/Build/Products/Debug/or_app.app"
   app_copy="$(mktemp -d apps/or_app/build/local-macos-package.XXXXXX)/or_app.app"
   ditto "$app_source" "$app_copy"
   # Use a separate app container for repeatable acceptance runs.
   local_app_id="io.github.huou07.orApp.localTest$(date +%s)"
   /usr/libexec/PlistBuddy -c "Set :CFBundleIdentifier $local_app_id" "$app_copy/Contents/Info.plist"
   bash scripts/package_local_macos_app.sh "$app_copy" "$ffmpeg_prefix"
   open -n "$app_copy"
   ```

   The helper embeds the six FFmpeg libraries and media tools, rewrites their
   search paths to be bundle-relative, preserves the app sandbox while
   re-signing, and checks the Rust bridge, ffprobe, and VideoToolbox H.264/AAC
   encoders with DYLD variables unset. This is a local run bundle; release
   packaging must add its third-party notices. Use profile or release mode for
   performance measurements, not debug mode.
5. Perform the affected interaction in the visible app. Verify canonical project
   state, save/reopen and output as appropriate. For exported media, inspect
   streams with ffprobe, decode the file, and open or play it.
6. Record source revision, build mode, machine, fixture, codec/backend,
   measurements, output path and limits. Commit verified coherent work locally.

## Evidence

Classify source tests, semantics, real behavior, visual inspection, native build
and performance separately. A local result proves only the tested source and
workflow on this Mac. Do not claim hosted, cross-platform, release or untested
behavior. The harness runs a debug build through the Flutter test binding: its
screenshots are not claims about a packaged or release build, and its timings are
debug-mode software-path evidence.

Do not push, open PRs, trigger remote CI, publish or deploy during the current
mission. Do not install system toolchains or change machine security settings as
a build workaround.

## Session skills

This plugin supplies only the OR-specific contract. Pair it with the session
skills the host exposes: `executing-plans` when a written plan exists,
`systematic-debugging` when a check fails, `test-driven-development` for the
regression guard, `verification-before-completion` before any done claim, and
`ui-ux-pro-max` for visual and UX decisions. Do not re-create those procedures
here.
