# OR-BUILD — deliver a usable local macOS editor

Last reviewed: 2026-10-21

This brief summarizes the owner's direct mission request from 2026-10-21. The
request is the full scope authority; this file is a short routing and recovery
aid. Begin by inspecting the actual local checkout. Older GitHub, Linux and
Android reports may be stale and do not establish current usability.

## Outcome and order

Build and dogfood the existing macOS application until local evidence shows the
essential editing journey works. Then complete the deterministic MotionScene
journey. Cross-platform development remains a future objective.

The editing journey is: create a project; import common video, image and audio;
arrange multiple tracks; move, trim by a visible edge handle, split at the
playhead and reorder clips; add image and audio; change speed and volume; use
snapping and undo/redo; preview the actual composition with advancing video and
synchronized audio where supported; save/reopen; export a playable video.

The MotionScene journey is: use a real source-code fixture; create a short scene
with syntax highlighting, typing, zoom, callouts/arrows and simple transitions;
preview and render through OR's existing architecture and FFmpeg; insert the
result into an ordinary timeline; export it with ordinary media.

Prioritize the connected editing path and repair broken controls before adding
unrelated features. Defer transcription, cloud/generative AI, community features,
advanced effects and unrelated platform work. MotionScene rendering is
declarative, deterministic and provider-free.

## Boundaries

- Keep Flutter as presentation and Rust as canonical project/editing-state owner.
  Use existing validated commands, project format, revision checks, undo and
  recovery; do not create a second editor or mutable project model.
- Preserve user projects, app data, branch history, worktrees and unrelated
  changes. Use isolated test profiles and lawful, reusable small fixtures.
- Benchmark the actual Apple Silicon path before making acceleration claims.
  Verify practical export codecs with the built FFmpeg and independently inspect
  and decode each output.
- Keep work sequential when it shares build state. Avoid unnecessary agents,
  duplicate toolchains, expensive unrelated test matrices and new frameworks.
- This mission authorizes local macOS builds, launches, real UI dogfooding,
  focused checks and local commits. Do not push, open PRs, trigger remote CI,
  publish, deploy or contact third parties.

## Working and reporting

Recheck branch, HEAD, worktrees, dirty state and installed tools. Read only the
task-relevant authorities listed by [docs/INDEX.md](../INDEX.md). Use CodeGraph
before inspecting indexed code. Make one coherent verified change at a time,
then build, launch and exercise the affected path in the real app.

Use local CLI/tests for diagnosis and guards; they do not replace the visible
macOS journey. Record the app build/source, fixture, project state, output
validation and measurements. Classify each result as IMPLEMENTED,
VERIFIED_SOURCE, VERIFIED_BEHAVIORAL, VERIFIED_VISUAL, VERIFIED_NATIVE,
FAILED, BLOCKED, UNVERIFIED or NOT RUN. Never claim an unexercised interaction
or invalidated export works. Commit coherent verified progress locally.

The mission is complete only when the available local evidence demonstrates
both journeys in the real macOS application with acceptable responsiveness and
independently verified exports. Report remaining gaps plainly.
