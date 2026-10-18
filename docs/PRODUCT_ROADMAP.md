# Active Product Roadmap

Last reviewed: 2026-10-18

## Authority and status

This outcome-based roadmap supersedes the old checkpoint sequence for active
work as of the convergence pivot recorded in
[OPEN_SOURCE_CONVERGENCE.md](OPEN_SOURCE_CONVERGENCE.md). It groups real user
capabilities and technical dependencies; its rows are not separate operator
prompts or mandatory commits. `docs/archive/execution/PLAN.json`, `STATE.json`
and `phases/` remain immutable historical traceability for the work
already performed. Their future `NEXT` cursor does not authorize a return to
the old implementation sequence.

Product requirements in [PRODUCT.md](PRODUCT.md), UX truth in
[DESIGN.md](../DESIGN.md) and [UX_ACCEPTANCE.md](UX_ACCEPTANCE.md), and
architecture/security/persistence invariants remain in force. A roadmap row
can be narrowed only when the source requirements or measured user value
justify it; record the exact rationale and evidence here. Do not label a
capability done from plans, source inspection, or model claims alone.

## Current product focus (owner restatement, 2026-10-18)

The owner restated the product direction on 2026-10-18. These four capability
groups are the active focus, in dependency order. Dogfood validation of the
first-project journey continues in parallel rather than as the single trumping
focus.

1. **Complete basic editing** — the editing journey must be complete, fast and
   understandable, with the same concepts, terminology and command semantics on
   macOS, Linux and Windows, and an adapted but equivalent Android presentation.
2. **Code-to-illustration video** — turn a code artifact into a declarative
   MotionScene and render it deterministically. Track A is primary; Track B, an
   external generative-video provider, is a later optional addition.
3. **Automatic subtitles** — implemented for desktop debug packages; Android
   support, word timings and translation remain open.
4. **Noise filtering and audio cleanup** — belongs in the audio layer.

Build and test run fast locally on the Debian workstation for Linux desktop and
the Android emulator; every release still passes hosted CI before publication.

Recent focused work: Candidate `3bf2e6e8980c093072b217ba33c6c91a67e1aa82`
removes the visual-only editor shell from Home and makes Add to Timeline
default to the current content end, so consecutive clips can be assembled
without manually calculating a start time. Exact-SHA Platform Verification
[38366017857](https://github.com/huou07/Opencut-Reinforced/actions/runs/38366017857)
passed all seven jobs. Validation-only Developer Preview
[38369979125](https://github.com/huou07/Opencut-Reinforced/actions/runs/38369979125)
built Android, Linux, macOS, and Windows packages and passed packaged desktop
transcription/caption acceptance. That is four build jobs; the Linux job emits
both a tarball and a `.deb`, so a complete run produces five app packages (see
RELEASE.md). The five temporary artifacts expire October
20, 2026; the run summary includes the dogfood journey and feedback route.
These are debug packages, not stable installers or a public release.
Clean-machine installation/upgrade and physical-device responsiveness remain
unverified; Android transcription is unsupported. Detailed historical
evidence remains in the [history archive](evidence/PRODUCT_ROADMAP_HISTORY.md).

Developer Preview [dev-656cd656199c](https://github.com/huou07/Opencut-Reinforced/releases/tag/dev-656cd656199c)
targets exact product SHA `656cd656199ca6a236921c83ba9bdb5705f61d57`;
Platform Verification [38192698936](https://github.com/huou07/Opencut-Reinforced/actions/runs/38192698936)
passed all seven jobs, and Developer Preview
[38194980068](https://github.com/huou07/Opencut-Reinforced/actions/runs/38194980068)
published and downloaded all 13 assets to re-verify `SHA256SUMS.txt`. The
release is an unsigned or ad-hoc-signed debug preview, not a stable release;
Android verification covers emulator correctness and resource bounds only,
not physical-device performance. The earlier transcription WAV handoff, pinned
Whisper proof, and proposal contract are recorded in the
[transcription integration note](evidence/local-transcription-poc.md).

The 2026-10-15 quality pass adds a bounded Linux debug journey and executable
semantics actions for custom timeline clips/trim handles. Local Rust-backed
create/import/recovery/edit/undo/seek/save-reopen and selected screenshots ran;
[testing evidence](TESTING.md#evidence-selection-and-bounded-linux-preflight)
records scope and rendering limitations. Exact source `66164227f3e39aa32567890eb88850d54ff9aa37`
passed [hosted Linux UI preflight 38679257316](https://github.com/huou07/Opencut-Reinforced/actions/runs/38679257316).
This accepts the scoped Linux debug journey; it does not advance packaged,
preview rendering, media playback or cross-platform acceptance claims.

## Product acceptance target

A non-developer can install a supported package, understand the workspace,
create or open a project, import local media through the platform's real
permission model, assemble and revise a useful edit, preview with responsive
audio/video behavior, use available assisted features as reviewable proposals,
save/reopen/recover safely, and export a valid result using shipped
dependencies. The same project/editing concepts, terms, layout intent, and
command semantics hold across macOS, Windows, Linux, and Android; input and
layout adapt to platform and screen size. iOS remains an intentional future
parity target, not an acceptance claim until supported tooling and packages
exist.

Acceptance covers packaged user journeys, usability, performance, and
resources on real hosted platform paths. Green unit tests alone are
insufficient. Track crashes, corruption, import/playback/export failures,
accessibility and task-completion issues as product defects.

## Work streams

The order below preserves actual dependencies while allowing independent work
to proceed together. Exact evidence is attached to requirement IDs and
implementation/workflow SHAs in the durable execution evidence ledger; this
roadmap supplies the human mapping.

| Work stream | Capabilities and mapped legacy requirements | Dependencies and evidence required |
|---|---|---|
| **A. Install, open, and protect a project** | Finish safe project creation/opening, clear first-run guidance, recent projects where justified, dirty-state protection, autosave/checkpoint policy, crash recovery, schema compatibility, and package install/upgrade/uninstall. Legacy: 9E; relevant 4E1/4E2/4F and release requirements. | Preserve Rust ownership, exact-base and explicit migrations. Verify clean packaged installs and create/edit/save/close/reopen/recovery across desktop; verify Android SAF grants and process death through hosted device acceptance. |
| **B. Bring media into the project reliably** | Media import for multiple files/folders where platform APIs permit, useful metadata/errors, thumbnails/waveforms, offline/relink workflow, media removal, Android SAF/document provider access, and bounded cache/proxy controls. Legacy: 9A/9B/9D/9E and 5A–5F. | Permissions and source references precede ingest. Test revoked/missing media, large libraries, >64 assets, SAF providers and packaged ffprobe/FFmpeg. Existing evidence covers desktop local files and Android SAF preview/export/recovery. Single- and multi-file SAF import plus desktop multi-file selection are implemented at `8dcadd6`; clearer missing-source feedback and the corrected acceptance assertion are at `8648be7`. Undoable desktop local-file relink preserving MediaId and timeline references is implemented at `d313a13`, with the packaged desktop journey at `e40c1ef`. Android real-picker relink acceptance targets the existing item at index 64 and checks MediaId/timeline preservation through save, reopen, and process recovery; recent and selected-source SAF grants are validated through the real picker. Folder import passed exact-SHA packaged verification at `a7f4bf4`; remaining gaps are large-library cache behavior and a broader offline-media/bulk-relink workflow. Historical failure receipts are in the [history archive](evidence/PRODUCT_ROADMAP_HISTORY.md). |
| **C. Make and revise an edit** | Clear timeline selection, tracks/clips, insert/move/trim/split/ripple, undo/redo, keyboard/pointer/touch operations, zoom/scroll, snapping, clip properties, markers, and mobile adaptations. Legacy: 6A–6E, 8A–8F, 9C, 13A/13B/13G/13H. | Keep exact Rust time and one command/history source. Group interaction and command work by user journey. Verify representative complex edits, persistence, CLI parity, and touch/keyboard accessibility. Defer advanced nested/multicam semantics until normal edits are smooth. |
| **D. Preview, audio, effects, and text coherently** | Reliable seek/play/scrub/frame-step, audio sync and device controls, typed transforms, text/caption styling, effects/transitions, and preview/export semantic parity. Legacy: 7A–7H, 8A–8F, 10C–10D, 13C–13F. | Preserve bounded queues, immutable snapshots, deterministic fonts, fallback behavior, and frame/audio resource budgets. Measure the packaged OR baseline first. If it exposes a material media compatibility, reliability, or performance gap, run the bounded MLT/GES comparison in the convergence audit before considering a runtime replacement. |
| **E. Export a useful result** | Easy output presets and destinations, explicit format/codec capabilities, progress/cancel/error recovery, valid audio/video output, and repeatable packaged export. Legacy: 8F, 9D, 13 and release requirements. | Same typed project/render semantics as preview; FFmpeg provenance and redistribution review per target. Verify files in independent media probes/players and compare output behavior across desktop/mobile. |
| **F. Add focused AI assistance** | Transcript/caption proposals, translation, scene/silence analysis, selected generative assets, voice workflows, provenance, and review-before-apply. Legacy: 10A–10H, 12A–12F, 14A–14G. | Implement only after corresponding human edit/import/export paths work. Provider-independent tasks; credentials stay outside project/CLI; explicit consent, costs, network/data flow and model licenses. Outputs remain editable proposals/assets applied through normal commands. Use existing providers/services where useful; do not bundle models by default. |
| **G. Make the product extensible and contributable** | Stable documented command/API contracts, useful import/export interoperability (prioritize SRT/WebVTT captions against the `subtitler` integration proof), declarative templates/motion/assets, accessible developer docs, reproducible builds, community contribution path, and only then a justified plugin or asset distribution model. Legacy: 11A–11F, 15A–15D, 16A–16G and 4F. | Favor formats and upstream contributions over a marketplace/backend. Preserve no-code-execution defaults and explicit permissions. Do not build executable plugins, OpenFX, WebMotion, accounts, or hosted registries without a demonstrated user need and security/license acceptance. The parser's SRT/VTT tests and exact-time boundary proof pass; upstream PR [#8](https://github.com/subtitle-rs/subtitler/pull/8) proposes removing 22 active package nodes from that library-only feature set while preserving default CLI behavior. OR still needs to validate cue/style loss, undo behavior, and shipped-package size before depending on it. |
| **H. Cross-platform release quality** | Consistent design/terminology and material feature parity on macOS, Windows, Linux, Android; accessibility; signed/notarized production distribution; install/upgrade/uninstall; performance and reliability. Legacy: 7H–9E and release requirements. | Platform-specific file pickers, permissions, packaging, window integration and hardware APIs may differ. Editing semantics and recovery must not. Maintain an explicit platform matrix and report unverified cells honestly. iOS is future until supported. |
| **I. Code-to-illustration video** | Turn a code artifact into an illustrated explainer: code ingestion and region selection, tokenisation and syntax highlighting, a versioned declarative `.ormotion.json` MotionScene, deterministic materialisation to persistent media, callouts and arrows, captions and narration, and reusable templates. Track B — an external generative-video provider — is a later, costed, licence-reviewed extension. | Depends on the [ADR 0007](adr/0007-declarative-motion-scenes-and-procedural-isolation.md) contract, the `or_render` text and layer path, export, and captions. Reuse the deterministic bundled-font rasteriser and the existing Text/Caption clip content; do not add a second text engine. Materialisation registers output through normal media/timeline commands, and validation, preview, render and materialisation make zero provider calls. Code and its highlight theme are untrusted input, so bound the parser and never execute the sample. |

The current Android app floor is API 26 (Android 8.0), matching CPAL's AAudio
backend. This intentionally excludes older Android releases; the Android
package and native linker both target API 26.

## Dependency graph

```text
safe project open/recovery ──┐
                             ├─> import/media permissions ─> edit interactions
packaged runtime foundation ─┘                                 │
                                                              ├─> preview/audio/effects/text
                                                              │                  │
                                                              └──────────────────┴─> export
human edit paths ─> reviewable AI proposals ─> apply through shared commands
all user paths ─> platform parity, accessibility, performance, release quality
```

The MLT/GES comparison is a bounded, baseline-triggered architecture
experiment, not a prerequisite for all product work. Do not let it block
project usability, Android import, or finishing the existing preview/export
journey unless results establish a real engine migration dependency.

## Current product status and highest-value gaps

- **Verified baseline:** exact-SHA run [38087624707](https://github.com/huou07/Opencut-Reinforced/actions/runs/38087624707) passed the required Rust, Flutter, descriptor, and packaged platform checks on `1dade2093a739322a894764f453700be0f9009fb`. Prior packaged desktop journeys exercised representative 1080p H.264/AAC import, preview, save/reopen, relink, and export; exact run reports and limitations are in the [history archive](evidence/PRODUCT_ROADMAP_HISTORY.md).
- **Packaged edit/export:** exact-SHA run [38162539040](https://github.com/huou07/Opencut-Reinforced/actions/runs/38162539040) on `c3bcd95030d7fbcd7905a5a28d947e375f46497c` passed Linux, macOS, and Windows real-media journeys. Exact repeat on [38165543230](https://github.com/huou07/Opencut-Reinforced/actions/runs/38165543230) on `81491556f6035a309cee129f25b7e762bd1dd11c` passed Windows again. Both runs imported 1080p H.264/AAC, saved/reopened/relinked, exported 30 seconds of VP9/Opus, and independently decoded 720 video plus 1,501 audio frames. Hosted export times were Linux 47.299 s, macOS 57.394 s, and Windows 182.114 s then 169.702 s; monitor the Windows result through dogfood before changing quality/speed settings. Run 38165543230 also had a macOS lifecycle-suite timeout with no assertion failure (the identical suite passed on 38162539040) and a transient Linux TLS failure downloading Opus from `downloads.xiph.org`. Its Android loop exposed a repository-relative script path that fails after FFmpeg changes directories; the absolute-path repair was validated by the Android job in exact-SHA run [38172368290](https://github.com/huou07/Opencut-Reinforced/actions/runs/38172368290). Details are in the [history archive](evidence/PRODUCT_ROADMAP_HISTORY.md).
- **Exact-SHA platform run:** run [38172368290](https://github.com/huou07/Opencut-Reinforced/actions/runs/38172368290) on `33f7f355a5bb1c848551babe9c4c5c11c2a65316` passed Rust, Flutter, repository hygiene, descriptor-boundary, Linux, macOS, Windows, and Android gates. The desktop packaged journeys imported, edited, saved/reopened/relinked and independently decoded 30-second VP9/Opus exports: Linux 46.883 s, macOS 54.526 s, Windows 151.311 s, each with 720 video and 1,501 audio frames. Android built all three ABIs and its real DocumentsUI SAF journey wrote valid WebM with independently decoded video/audio; its small half-second fixture proves correctness only, not representative mobile performance. Full measurements and limits are in the [history archive](evidence/PRODUCT_ROADMAP_HISTORY.md).
- **Android multi-source import:** the initial exact-SHA run [38196732759](https://github.com/huou07/Opencut-Reinforced/actions/runs/38196732759) exposed a real SAF multi-import regression; root cause, fix, and final evidence are in the [history archive](evidence/PRODUCT_ROADMAP_HISTORY.md). Platform Verification [38199486640](https://github.com/huou07/Opencut-Reinforced/actions/runs/38199486640) on `39afed57adf3a04faba842f78ac73a9d68a1bbed` passed all seven jobs. Its Android DocumentsUI journey imported H.264/AAC MP4 and audio-only MP3 in one selection, then passed captions, relink, export, process recovery, and bounded-resource checks; desktop Linux, macOS, and Windows packaged journeys also passed. The fixture establishes emulator correctness, not representative media performance. The emulator logged skipped frames and a 121 ms maximum main-draw measurement under this acceptance workload; physical-device responsiveness remains unverified, so further emulator tuning is not the next task.
- **Current product candidate:** `c329d8e26c994fb24af5f53d3f4c50e31087d1cd` stages the pinned whisper.cpp CLI and OR provider in desktop preview bundles and adds a real packaged transcription, caption review/edit/apply, save/reopen, and SRT export gate. Exact-SHA PR Platform Verification [38234509928](https://github.com/huou07/Opencut-Reinforced/actions/runs/38234509928) passed all seven required jobs, including macOS, Windows, Linux packaged journeys and Android SAF/resource acceptance. Those checks verify existing cross-platform behavior. The validation-only workflow passed its exact-SHA prepare gate on `4033f90a16c9042008e404c1d597db7eaed6c493`, built Android, and skipped all release-only jobs. Its macOS, Windows, and Linux packaged transcription gates failed; root causes and follow-up evidence are recorded in the history archive. Android transcription is unsupported and physical-device performance remains unverified.
- **Folder import — exact-SHA verified:** `a7f4bf4010c3d3283e788387839edbc6f83a0f3d` passed Platform Verification [38275213412](https://github.com/huou07/Opencut-Reinforced/actions/runs/38275213412), Repository Hygiene, and all Linux, macOS, Windows, Android, Rust, Flutter, and descriptor jobs. The clean desktop journeys and Android DocumentsUI flow passed; Android imported three selected files plus two nested files under the selected `Footage` grant, then passed preview, captions, relink, export, recovery, and descriptor/resource checks. A prior Android run timed out before the import gateway, but this run passed; its cause is still unlocalized. Emulator evidence does not establish physical-device performance. Detailed timing and scope evidence are in the [history archive](evidence/PRODUCT_ROADMAP_HISTORY.md).
- **Recent project return — exact-SHA verified:** `a3db4f63319029ea683b6d4cc749df7fdba27a11` passed Platform Verification [38328757217, attempt 2](https://github.com/huou07/Opencut-Reinforced/actions/runs/38328757217), with Repository Hygiene [38328757237](https://github.com/huou07/Opencut-Reinforced/actions/runs/38328757237). Linux, macOS, and Windows packaged journeys reopened a recent project after application restart. Android verified real project selection, the persisted recent URI, a fresh-picker SAF rebind, read-grant/path validation, and provider synchronization; its larger journey also passed multi-media import, captions, relink, process recovery, preview resource bounds, and independently decoded VP9/Opus WebM. Attempt 1's Android provider freeze and macOS launch timeout, and their exact-SHA passing rerun evidence, are detailed in the [history archive](evidence/PRODUCT_ROADMAP_HISTORY.md).
- **First-project handoff — exact-SHA verified:** `72fe14c` adds an empty-timeline Import Media action that uses the existing platform picker and permission path on desktop and compact layouts. Its first Platform Verification run exposed a close-control readiness race in the packaged test on Linux and Windows; a fresh rerun passed Windows but reproduced Linux. Commit `f5cf1c26353feaebf49e9e5ca058414ad9798809` now waits for the actual enabled, hit-testable Close control and reports its state if closing stalls. Platform Verification [38353664539](https://github.com/huou07/Opencut-Reinforced/actions/runs/38353664539) passed all seven jobs, including the Linux packaged journey. The UI change preserves project and import semantics.
- **Most recent interaction work — exact-SHA verified:** at `4d4c6fb091b5930cb1cbed0769ba83acd1693df9`, Home no longer presents the visual-only editor shell as a usable project. At `3bf2e6e8980c093072b217ba33c6c91a67e1aa82`, Add to Timeline starts at the occupied timeline's content end. Platform Verification [38366017857](https://github.com/huou07/Opencut-Reinforced/actions/runs/38366017857) passed seven jobs for the latter SHA; its packaged desktop and Android journeys cover create/import/edit/preview/persistence/export paths.
- **Compact short-viewport editor — hosted Linux UI preflight verified:** the compact editor no longer overflows at 320x480. The reproduced 31 px overflow came from the viewer's empty placeholder, not the transport: the compact split left the surface 55.7 px tall while the icon-and-text column needed more. A 320x440 viewport additionally squeezed the fixed 34 px timeline view controls into an 8.7 px strip. Both now absorb a short viewport (the placeholder keeps its icon and exposes its message through semantics below 56 px high or 120 px wide; the timeline view controls are omitted below 60 px high). A headless regression mounts the editor at 320x480 and 320x440. Hosted Linux UI preflight [39013698516](https://github.com/huou07/Opencut-Reinforced/actions/runs/39013698516) passed the analyzer, the focused semantics tests including that regression, and the real Linux journey at exact source `c90f026bc7e0c816e0c10ecb2eb09b38d12f2493`; its machine-readable record [the claim](evidence/claims/2026-10-17-C-compact-short-viewport.json) is retained but superseded: it claimed the only later change was a dart-format-only commit, while 21 commits actually followed. Exact-HEAD Platform verification [39043596474](https://github.com/huou07/Opencut-Reinforced/actions/runs/39043596474) later passed all seven required platform jobs plus the aggregate `release-gate` at `c8b627be9e931f2b332ac1c8f2de459b4c30b22d`, and the superseding machine-readable record is [the claim](evidence/claims/2026-10-17-C-compact-short-viewport-c8b627b.json). Compact-viewport rendering on a physical mobile device remains unverified.
- **Large-library media previews — fixed, hosted Flutter verification:** a media library larger than one 50-item page dropped the thumbnail or waveform already generated for every item on an earlier page as soon as the next page loaded: `_loadMoreMedia` passed only the newly fetched items to `_queueMediaPreviews`, which treats its argument as the complete loaded set, prunes every preview outside it and never re-requests the pruned sources. The load-more path now passes the accumulated page. Canonical project state and `ProjectRevision` are untouched; only disposable Flutter presentation state changed and no command is dispatched. Platform Verification [39017187079](https://github.com/huou07/Opencut-Reinforced/actions/runs/39017187079) job *Flutter static and widget checks* ran `dart format`, `flutter analyze` and the full `flutter test` suite on the canonical Flutter 3.47.5 pin at exact source `5255a0dbd123d289c49a93e82a8d84b36a9a9c7d`, including the 55-item regression; Linux UI preflight [39017187058](https://github.com/huou07/Opencut-Reinforced/actions/runs/39017187058) also passed at that SHA. The machine-readable record is [the claim](evidence/claims/2026-10-17-B-large-library-media-previews.json). The broader offline-media and bulk-relink workflow named by work stream B remains unimplemented and unverified.
- **Current dogfood build available:** validation-only Developer Preview [38369979125](https://github.com/huou07/Opencut-Reinforced/actions/runs/38369979125) passed the exact-source gate for SHA `3bf2e6e8980c093072b217ba33c6c91a67e1aa82`, built all five app packages (four build jobs; Linux emits a tarball and a `.deb`), and passed Linux, macOS, and Windows packaged transcription, caption review/edit/apply, save/reopen, and SRT export. Artifacts expire October 20, 2026; publication and CLI packages were skipped. The packages are debug builds, not production installers.
- **Clean desktop install, upgrade, and removal — exact-SHA verified:** commit `53de6d51c6d45c7ec7ae0a2989455972118406c1` passed Platform Verification [38570870756](https://github.com/huou07/Opencut-Reinforced/actions/runs/38570870756) and Repository Hygiene [38570870938](https://github.com/huou07/Opencut-Reinforced/actions/runs/38570870938). The Linux job installed, upgraded, launched, and purged the `.deb` in Ubuntu 24.04 and Debian 13 containers while preserving a user-data sentinel. The macOS and Windows jobs extracted, installed, launched, replaced, and removed their ZIP packages from per-user application directories while preserving user data. Limits and exact job evidence are recorded in the [Linux claim](evidence/claims/2026-10-14-9E-linux-debian-install-upgrade-uninstall.json), [macOS claim](evidence/claims/2026-10-14-9E-macos-zip-install-replacement-removal.json), and [Windows claim](evidence/claims/2026-10-14-9E-windows-zip-install-replacement-removal.json).
- **Local Linux `.deb` packaged evidence:** at exact source `33b84ebe2df37280ca0a27759030c9efc6fb3dca` this workstation built the pinned libvpx 1.17.0 / libopus 1.6.1 / FFmpeg 8.1.3 runtime, built the Flutter Linux debug bundle against it, and packaged `opencut-reinforced-linux-x64-debug.deb` (31,599,552 bytes, SHA-256 `7253f9e13589da1051391595fe389fc2143d4133dbde4163feec8f5dab4b72d2`). The package was extracted into a disposable root, the installed `/opt/opencut-reinforced/or_app` ran for the full 20-second smoke window, and `scripts/run_packaged_desktop_journey.py` then used the installed package's `ffmpeg`, `ffprobe`, Rust bridge and media libraries: all four app processes passed and the export independently decoded to 720 VP9 video and 1,501 Opus audio frames at 30.008 s. This is local packaged evidence on this host (Flutter 3.47.7, not the pinned 3.47.5), not an `apt` install/upgrade/purge and not hosted acceptance; the commands, payload hashes and limits are in [the record](evidence/2026-10-18-linux-deb-local-acceptance.md).
- **One local Linux lane — implemented, native step blocked by host memory:** `bash scripts/run_local_lane.sh linux` is now the single local entry point for the hygiene gate, the Flutter widget suite, the bounded preflight and the performance budget, and it leaves one machine-readable report (`apps/or_app/build/local-lanes/linux.json`). At exact source `b4a84956883422ed51ef1fb799d6cae5c9ff18b4` the lane reported `BLOCKED` (exit 3) with `hygiene`, `flutter` (124 tests) and `bench` PASS and the preflight BLOCKED at its own 6 GiB `MemAvailable` floor; a deliberately broken source check made the same command exit 1, and the packaged steps' blocked branches were exercised with a deliberately absent pinned prefix. `bash scripts/acceptance.sh` passed at that source with 8 named UNVERIFIED lanes. Evidence class: local developer evidence only, no claim record and no `ACCEPTED`; the native journey at this source is unverified and `--packaged` was not executed, and the commands, timings, hashes and limits are in [the record](evidence/2026-10-18-local-linux-lane.md).
- **Basic editing — implemented baseline with enumerated gaps:** 23 commands and
  8 queries dispatch through the shared application path
  (`crates/or_core/src/application.rs:64`), covered by 109 core unit tests and 92
  Flutter widget tests across tracks, insert, move, trim, split, ripple delete,
  clip properties, markers, captions and undo/redo. Not implemented: overwrite
  edit mode; ripple trim; slip, slide and roll; track reordering; clip
  speed/rate change; keyframes; clip linking or grouping. Zoom, scroll and
  snap-guide rendering are presentation state, and snapping itself is the
  read-only `timeline.snap` query.
- **Automatic subtitles — desktop implemented, Android unsupported by design:**
  whisper.cpp `tiny.en` is pinned by hash and downloaded only on confirmation
  (`crates/or_ai/src/model.rs:38`), the provider runs as a sidecar
  (`crates/or_whisper_provider`), transcription yields a provenance-bound
  proposal that a user reviews, edits and applies through a revision-checked
  command, and SRT/WebVTT round-trips through `crates/or_core/src/subtitle_io.rs`.
  Packaged desktop journeys pass on macOS, Windows and Linux.
  `crates/or_app_bridge/src/transcription.rs:798-826` has no Android model-store
  arm, so `TranscriptionService::new()` returns `None` and every call reports
  `ProviderUnavailable`; Android transcription stays unsupported until that path
  and its acceptance exist.
- **Noise filtering and audio cleanup — absent, and not reachable through the
  shipped FFmpeg:** the only audio processing is clip gain, pan and linear fades
  applied in both preview and export (`crates/or_audio/src/lib.rs:428-436`,
  reached from `crates/or_app_bridge/src/preview.rs:2612`). No denoise, EQ,
  compressor, limiter or filter graph exists in code, export profiles or the
  command catalog. The pinned shared library build disables avfilter
  (`FFMPEG_COMMON_CONFIGURE_ARGS`, `.github/workflows/platform-verification.yml:33`);
  only the desktop helper executables re-enable it with six filters
  (`:51,58-63`), so the library the editor links cannot denoise as shipped.
  **Owner decision, 2026-10-18:** the route is a deliberate FFmpeg
  build-configuration change that enables the filters this capability needs,
  with the licence, packaging and per-platform consequences re-verified before
  it is enabled anywhere. Bounded Rust DSP in `or_audio`/`or_media` is not the
  chosen route. The change is phase 4 of
  [prompts/OR-BUILD.md](prompts/OR-BUILD.md) and no part of it is implemented
  yet.
- **Code-to-illustration video — absent; only enabling primitives exist:** there
  is no code ingestion, tokenisation, syntax highlighting, MotionScene schema or
  parser, scene evaluator or materialisation path. `crates/or_ai/src/protocol.rs:26`
  declares an `AiTask::GenerateMotionScene` variant with no producer or consumer.
  What exists to build on: deterministic bundled-font text rasterisation
  (`crates/or_render/src/text.rs:18-39`), first-class text and caption clip
  content (`crates/or_core/src/timeline.rs:181`), and the shared wgpu layer render
  path (`crates/or_render/src/lib.rs:602`). ADR 0007 is architecture intent, not
  implementation.
- **One local Android lane — entry point landed, guest drive unexecuted:**
  `bash scripts/run_local_lane.sh android` selects it and delegates to
  `scripts/run_android_lane.sh`, which probes `MemAvailable` against a 7 GiB
  floor, the AVD, `/dev/kvm`, `adb`, `flutter` and `OR_ANDROID_FFMPEG_PREFIX`
  before booting anything, drives the repository's guest-lifecycle and
  SAF/preview scripts, always stops the guest it started, and writes
  `apps/or_app/build/local-lanes/android.json` in the Linux report's shape.
  `scripts/test_android_lane.py` checks its usage, blocked-precondition,
  dirty-tree and no-acceptance rules without a device, inside `check-repo.sh`.
  On 2026-10-18 an earlier revision of the lane reported `BLOCKED` with its own
  measurements: `MemAvailable` 5.5 GiB, the ~2.25 GiB `GradleDaemon 8.14.3` that
  belongs to the sibling project VCE, and the unset pinned FFmpeg prefix. That
  revision could not have run even with a prefix: it never created the
  `android/.ffmpeg-install` link the bridge build hook requires, and it invoked
  the preview script with no arguments against a script that takes four. Both
  are repaired, its guest RAM default now matches the emulator script's measured
  size, and reproducible staging is `scripts/stage_android_ffmpeg.sh` (which also
  builds the guest FFmpeg link probe) with `scripts/build_android_case_apks.sh`.
  The staged prefix and probe are now built on this host; whether the guest
  drive itself passes is **unverified until a run records a real result**. Both are phase 0 of
  [the current prompt](prompts/OR-BUILD.md).
- **Next product gap — complete the editing journey on desktop and Android:**
  close the enumerated basic-edit gaps in the order product value needs them,
  using the local Linux and Android lanes for fast iteration and hosted CI for
  acceptance. Code-to-illustration video follows once the text, materialisation
  and export paths it depends on are complete. Real-user dogfood feedback
  continues in parallel and still decides which fix matters first.

Detailed historical investigations remain in the archive for provenance. Keep new active-roadmap updates to user-visible outcomes, current limitations, exact acceptance evidence, and the next product gap.

## Traceability from the preserved roadmap

| Legacy scope | Active mapping | Treatment |
|---|---|---|
| Phases 0–4: repository, design, architecture, project/command foundation | A, C, G, H | Mostly implemented foundations; preserve their invariants and tests. |
| Phase 5: media foundation | B | Keep bounded job/cache/proxy and metadata work; prove user-facing behavior. |
| Phase 6: timeline MVP | C | Keep exact semantics; judge completeness by useful interaction, not checkpoint labels. |
| Phase 7: preview/playback | D | Keep bounded runtime and measure real packaged behavior. |
| Phase 8: desktop editing/export | A–E, H | Finish coherent user workflows and release packaging. |
| Phase 9: Android | A–E, H | Preserve 9B/9B1/9C/9D verified work; identify unsupported SAF/import/recovery gaps rather than assuming parity. |
| Phase 10: captions/transcript/AI assist | D, F | Human captions/text first; selective task-oriented assistance. |
| Phase 11: templates/assets/themes/motion | G | Keep declarative and provenance-safe; prioritize only user-valued workflows. |
| Phase 12: dubbing/voice | F | Optional, consent/privacy/licensing-gated workflows. |
| Phase 13: advanced editing/color/audio | C, D, E | Add capabilities as user needs and engine measurements justify. |
| Phase 14: AI generation | F | Integrate outputs as reviewable media/proposals; avoid model/service duplication. |
| Phase 15: community ecosystem | G | Start with contribution and static interchange; defer hosted systems. |
| Phase 16: plugins/interchange/OpenFX/WebMotion | G | Keep security contracts; build only bounded formats/extensions supported by demonstrated demand. |

## Replanning and completion

After each coherent capability, update this roadmap with status, exact product
commit, hosted runs/artifacts, platform coverage, performance measurements,
and any unmet requirement IDs. A requirement may be marked unnecessary only
with evidence grounded in the authoritative product requirements and an
explicit explanation of the replacement or removed user need. Preserve old
execution evidence; add convergence evidence rather than rewriting history.

The roadmap is complete only when all required user journeys and supported
platform gates pass, remaining limitations are explicit, and the product
provides a clear reason for its OR-specific layer to exist alongside its
upstreams.
