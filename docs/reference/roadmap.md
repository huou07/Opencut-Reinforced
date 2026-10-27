# Active Product Roadmap

Last reviewed: 2026-10-27

Scope: what the owner selected to build now, what acceptance means, and what is
deferred. Requirements live in [product.md](product.md); capability contracts in
[features/](../features/).

## Current focus (owner restatement, 2026-10-21)

One near-term delivery mission for the local macOS app, in this order:

1. **Finish ordinary editing.** Complete and dogfood the connected journey:
   project create, common media import, multi-track editing, visible pointer
   trim/move/split/reorder, image/audio, speed/volume, snapping, undo/redo,
   composition preview, save/reopen, playable export.
2. **Finish deterministic MotionScene.** Turn source code into a short
   declarative illustration, preview and render it through the existing
   architecture, insert it into the ordinary timeline, and export the combined
   edit.
3. **Assistant panel (owner request, 2026-10-26).** Let the user edit the open
   project and generate MotionScene illustrations by chatting with the local
   DeepSeek Harness. This is an external client over ACP and `or mcp`, not an OR
   provider integration; provider and inference work inside OR stays deferred.
4. **Agent workspace (owner request, 2026-10-27).** Open the assistant in an
   app-installed workspace whose `AGENTS.md` routes the agent to one guide,
   skill or tool per task, instead of the project folder. Delivered 2026-10-27
   ([agent-workspace.md](../features/agent-workspace.md)); dogfooding it against
   real edits is the next check.
5. **Motion asset library (owner request, 2026-10-27).** Turn motion output into
   reusable, parameterised assets — adjustable text, colour and size; automatic
   classification; a browsable library; drag and drop into the timeline or an
   open scene. P1 (asset document, typed parameters, classifier, library store
   and `or motion asset` CLI) landed 2026-10-27
   ([motion-assets.md](../features/motion-assets.md)); P2 is the Motion surface,
   parameter editing and drag to the timeline, then P3 composition and agent
   tools ([ADR 0010](../adr/0010-motion-assets-and-reusable-library.md)).

macOS on the owner's MacBook Air is the only required runtime target. Complete
supporting work only when it unblocks one of these two journeys.

## Deferred by the owner

Transcription, cloud or generative AI, community/templates catalog, advanced
effects, animation-heavy showcase surfaces, Android/Windows packaging work,
hosted CI claims, release publication, and cross-platform parity work.

## Acceptance target

A beginner can use the built macOS application to complete both journeys through
real pointer and keyboard interaction, with the resulting Rust-owned project
state, save/reopen behaviour, actual preview, and independently validated
playable exports. A widget test, hidden command, screenshot or successful build
alone does not establish a completed journey.

## Longer-term inventory (not the active queue)

| Stream | Capabilities | Gate |
|---|---|---|
| A. Install, open, protect a project | safe create/open, first-run guidance, recent projects, dirty-state protection, autosave/checkpoint, crash recovery, schema compatibility, install/upgrade/uninstall | Rust ownership, exact-base save, explicit migrations, packaged journeys |
| B. Bring media in reliably | multi-file/folder import, metadata and errors, thumbnails/waveforms, offline/relink, removal, SAF access, bounded cache/proxy | permissions before ingest; revoked/missing media and large libraries tested |
| C. Make and revise an edit | selection, tracks/clips, insert/move/trim/split/ripple, undo/redo, keyboard/pointer/touch, zoom/scroll, snapping, properties, markers, mobile adaptation | one exact-time command/history source; complex edits and persistence verified |
| D. Preview, audio, effects, text | seek/play/scrub/frame-step, audio sync, transforms, text/caption styling, effects/transitions, preview/export parity | bounded queues, immutable snapshots, deterministic fonts, measured baseline first |
| E. Export a useful result | presets and destinations, explicit capabilities, progress/cancel/error recovery, valid output, repeatable packaged export | independent probe/playback of the artifact |
| F. Focused AI assistance | transcripts, translation, analysis, generative assets, voice, provenance, review-before-apply | only after human paths work; provider-independent, no secrets to clients |
| G. Extensible and contributable | stable command/API contracts, captions/interchange, declarative templates/motion, reproducible builds | prefer formats and upstream contributions over a marketplace |
| H. Cross-platform release quality | consistent design, feature parity, accessibility, signed distribution, install/upgrade, performance | explicit platform matrix, unverified cells reported honestly |
| I. Code-to-illustration video | code ingestion, tokenisation/highlighting, `.ormotion.json`, materialisation, callouts, captions, templates | ADR 0007 contract, shared text path, zero provider calls |

Dependency order: safe project open/recovery and packaged runtime foundation →
import/media permissions → edit interactions → preview/audio/effects/text →
export; reviewable AI proposals only after the human paths work.

## Architecture direction (convergence decision, 2026-10-09)

Keep OR as an MIT, human-first, shared desktop/mobile product: keep the versioned
project format, exact-time editing model, Rust command/session/recovery authority,
semantic CLI, Flutter UI and wgpu preview/export direction. Do not adopt a
candidate on roadmap promises. OpenCut's rewrite is monitored until its promised
core/API ships and can be tested. MLT/GES are bounded media-engine candidates to
compare only against a concrete measured gap — media-engine replacement is not
approved.

## History

Dated run records, phase status and superseded plans are historical evidence:
`docs/archive/legacy/` and `docs/evidence/`. They are not current status on this
Mac and must not be used to prove a failure still exists or a success still works.
