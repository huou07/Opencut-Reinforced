# Active Product Roadmap

## Authority and status

This outcome-based roadmap supersedes the old checkpoint sequence for active
work as of the convergence pivot recorded in
[OPEN_SOURCE_CONVERGENCE.md](OPEN_SOURCE_CONVERGENCE.md). It groups real user
capabilities and technical dependencies; its rows are not separate operator
prompts or mandatory commits. `PLAN.json`, `STATE.json`, and
`execution/phases/` remain immutable historical traceability for the work
already performed. Their future `NEXT` cursor does not authorize a return to
the old implementation sequence.

Product requirements in [PRODUCT.md](PRODUCT.md), UX truth in
[DESIGN.md](../DESIGN.md) and [UX_ACCEPTANCE.md](UX_ACCEPTANCE.md), and
architecture/security/persistence invariants remain in force. A roadmap row
can be narrowed only when the source requirements or measured user value
justify it; record the exact rationale and evidence here. Do not label a
capability done from plans, source inspection, or model claims alone.

Current product focus: finish the cross-platform ordinary-MP4 editing journey
and make a usable Developer Preview available for dogfooding. Exact-SHA run
[38124449401](https://github.com/huou07/Opencut-Reinforced/actions/runs/38124449401)
on `af73be02d5907b38851bb45b88132b2b3a136f4d` passed Linux and macOS packaged
H.264/AAC MP4-to-WebM VP9/Opus journeys, Android all-ABI packaging and SAF
preview/resource acceptance, and Rust, Flutter, and descriptor checks. Windows
passed codec-source setup but failed FFmpeg configure while linking its MSVC
`stdbit.h` probe. The pinned compatibility header compiles; the failed probe
alone previously added FFmpeg's `-debug` link flag. Exact-SHA run
`38126190565` on `2c43fccad8cb80c850c52f5291927e57843a6e6c` disproved disabling
debug symbols as a fix. Run
[38133514739](https://github.com/huou07/Opencut-Reinforced/actions/runs/38133514739)
on `ff0f0fd5f276c7bae2c30780b37aaeee170c4b90` passed Linux, macOS and Android
packaged journeys and all Rust, Flutter, and descriptor checks, but Windows
again failed at the probe. The failure-only diagnostic runs in a fresh shell,
so its `/usr/bin/link.exe` result does not prove which executable configure
used. Run
[38135332968](https://github.com/huou07/Opencut-Reinforced/actions/runs/38135332968)
on `8de5b346e529ad142b405018fe8b83df370548dd` again passed every non-Windows
job, but the probe still failed despite a build-shell assertion that MSVC's
`link.exe` resolved first. The cause remains open. The next run traces
`compat/windows/mslink` inside configure and captures that trace on failure.
Profile longer media only when real evidence shows a quality, responsiveness,
cancellation, or resource problem. Android remains Matroska-only; emulator
acceptance does not establish physical-device performance.

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
| **B. Bring media into the project reliably** | Media import for multiple files/folders where platform APIs permit, useful metadata/errors, thumbnails/waveforms, offline/relink workflow, media removal, Android SAF/document provider access, and bounded cache/proxy controls. Legacy: 9A/9B/9D/9E and 5A–5F. | Permissions and source references precede ingest. Test revoked/missing media, large libraries, >64 assets, SAF providers and packaged ffprobe/FFmpeg. Existing evidence covers desktop local files and Android SAF preview/export/recovery. Single- and multi-file SAF import plus desktop multi-file selection are implemented at `8dcadd6`; clearer missing-source feedback and the corrected acceptance assertion are at `8648be7`. Undoable desktop local-file relink preserving MediaId and timeline references is implemented at `d313a13`, with the packaged desktop journey at `e40c1ef`. Run `37861883543` on `3678792` passed Linux/macOS/Windows plus Rust/Flutter/descriptor checks but failed Android because the fixture project was read-only on the save-and-close step. The fixture now exercises writable SAF project synchronization and verifies the provider document digest changes on save; hosted rerun still required. Android real-picker relink acceptance now targets the existing item at index 64 and checks MediaId/timeline preservation through save, reopen, and process recovery; the selected source's persisted DocumentsUI grant is checked without synthesizing it. This acceptance extension is unverified pending the exact-SHA hosted run. Folder import and large-library cache behavior remain open. |
| **C. Make and revise an edit** | Clear timeline selection, tracks/clips, insert/move/trim/split/ripple, undo/redo, keyboard/pointer/touch operations, zoom/scroll, snapping, clip properties, markers, and mobile adaptations. Legacy: 6A–6E, 8A–8F, 9C, 13A/13B/13G/13H. | Keep exact Rust time and one command/history source. Group interaction and command work by user journey. Verify representative complex edits, persistence, CLI parity, and touch/keyboard accessibility. Defer advanced nested/multicam semantics until normal edits are smooth. |
| **D. Preview, audio, effects, and text coherently** | Reliable seek/play/scrub/frame-step, audio sync and device controls, typed transforms, text/caption styling, effects/transitions, and preview/export semantic parity. Legacy: 7A–7H, 8A–8F, 10C–10D, 13C–13F. | Preserve bounded queues, immutable snapshots, deterministic fonts, fallback behavior, and frame/audio resource budgets. Measure the packaged OR baseline first. If it exposes a material media compatibility, reliability, or performance gap, run the bounded MLT/GES comparison in the convergence audit before considering a runtime replacement. |
| **E. Export a useful result** | Easy output presets and destinations, explicit format/codec capabilities, progress/cancel/error recovery, valid audio/video output, and repeatable packaged export. Legacy: 8F, 9D, 13 and release requirements. | Same typed project/render semantics as preview; FFmpeg provenance and redistribution review per target. Verify files in independent media probes/players and compare output behavior across desktop/mobile. |
| **F. Add focused AI assistance** | Transcript/caption proposals, translation, scene/silence analysis, selected generative assets, voice workflows, provenance, and review-before-apply. Legacy: 10A–10H, 12A–12F, 14A–14G. | Implement only after corresponding human edit/import/export paths work. Provider-independent tasks; credentials stay outside project/CLI; explicit consent, costs, network/data flow and model licenses. Outputs remain editable proposals/assets applied through normal commands. Use existing providers/services where useful; do not bundle models by default. |
| **G. Make the product extensible and contributable** | Stable documented command/API contracts, useful import/export interoperability (prioritize SRT/WebVTT captions against the `subtitler` integration proof), declarative templates/motion/assets, accessible developer docs, reproducible builds, community contribution path, and only then a justified plugin or asset distribution model. Legacy: 11A–11F, 15A–15D, 16A–16G and 4F. | Favor formats and upstream contributions over a marketplace/backend. Preserve no-code-execution defaults and explicit permissions. Do not build executable plugins, OpenFX, WebMotion, accounts, or hosted registries without a demonstrated user need and security/license acceptance. The parser's SRT/VTT tests and exact-time boundary proof pass; upstream PR [#8](https://github.com/subtitle-rs/subtitler/pull/8) proposes removing 22 active package nodes from that library-only feature set while preserving default CLI behavior. OR still needs to validate cue/style loss, undo behavior, and shipped-package size before depending on it. |
| **H. Cross-platform release quality** | Consistent design/terminology and material feature parity on macOS, Windows, Linux, Android; accessibility; signed/notarized production distribution; install/upgrade/uninstall; performance and reliability. Legacy: 7H–9E and release requirements. | Platform-specific file pickers, permissions, packaging, window integration and hardware APIs may differ. Editing semantics and recovery must not. Maintain an explicit platform matrix and report unverified cells honestly. iOS is future until supported. |

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
- **Export and ordinary MP4 editing:** exact SHA `346d2402cf0a519cb840380512b5fc6b6c7bcb2d`, run [38116245081](https://github.com/huou07/Opencut-Reinforced/actions/runs/38116245081), passed Linux's clean packaged journey with the pinned real 1080p H.264/six-channel AAC MP4: create/edit/title/save/reopen/relink and independent WebM VP9/Opus probe all passed. Linux exported 30.008 seconds at 12,763,556 bytes in 61,361 ms (56.2% of source size); all four app phases exited 0. Exact SHA `a7183b34734c4cfeec3066dfe944d7cbdae4d0f8`, run [38121804385](https://github.com/huou07/Opencut-Reinforced/actions/runs/38121804385), passed the same clean packaged journey on Linux and macOS; each output was 30.008 seconds, independently decoded as 720 VP9 video frames and 1501 Opus audio frames. The macOS output was 12,768,681 bytes (56.2% of the 22,718,509-byte MP4). Android's packaged SAF preview/resource journey and all non-Windows checks passed. Windows passed corrected libvpx CRT configuration, then failed FFmpeg configure at `stdbit.h`; a local compatibility-header path fix awaits verification. Android remains Matroska-only.
- **Known platform evidence gaps:** the overall Platform Verification run [38111281741](https://github.com/huou07/Opencut-Reinforced/actions/runs/38111281741) failed only the Windows VP9 configure guard; Linux, macOS, Android, Rust, Flutter, and descriptor jobs passed. Android SAF preview/resource acceptance passed in that run, but API 36 emulator observations do not prove physical-device performance or acoustic output.
- **Next work:** verify the Windows FFmpeg compatibility-header fix and packaged MP4 journey, then publish the exact verified Developer Preview for dogfooding. Android preview correctness remains emulator-only; physical hardware is still unverified. Add a real MOV journey only if user evidence or compatibility failures warrant it. Assess longer-media quality, cancellation, and resources only where real user journeys expose a material issue.

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
