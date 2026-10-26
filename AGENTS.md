# Opencut Reinforced — Agent Guide

Last reviewed: 2026-10-26

OR is an MIT-licensed, local-first video editor with one Rust-owned project core
shared by the Flutter UI, the `or` CLI and agents.

## Rule

Use this routing table to pick what to read for the current task, and do not read
material the task does not need. When editing docs or this file, follow the
routing template in `docs/operations/docs-maintenance.md` exactly and do not add
any other rule to this file.

## Routing

### Architecture

- **System overview** — clients, control and real-time planes, crate map, data flow.
  When: you need the shape of the system before touching a subsystem.
  Path: `docs/architecture/system-overview.md`
- **Control plane** — commands, queries, revisions, history, persistence, IPC, CLI parity.
  When: changing project state, save/recovery, the session, the bridge or the CLI.
  Path: `docs/architecture/control-plane.md`
- **Real-time plane** — snapshots, frames and leases, preview pipeline, audio clock, budgets.
  When: changing playback, rendering, presentation or frame lifetime.
  Path: `docs/architecture/realtime-plane.md`
- **Media pipeline** — FFmpeg runtime, probing, import profile, cache, jobs, encoding.
  When: changing media probing, decoding, cache, background jobs or codec support.
  Path: `docs/architecture/media-pipeline.md`
- **Project format** — `.orproj` envelope, versions, invariants, safe writing.
  When: touching the schema, migrations, codec or anything that reads project files.
  Path: `docs/architecture/project-format.md`
- **Platforms** — targets, storage and permissions per platform, packaging, capability matrix.
  When: work depends on platform behaviour, SAF, packaging or native interop.
  Path: `docs/architecture/platforms.md`
- **Security boundaries** — untrusted input, secrets, IPC limits, agent surface.
  When: a change crosses a trust boundary or touches credentials or permissions.
  Path: `docs/architecture/security-boundaries.md`

### Features

- **Project lifecycle** — create, open, save, reopen, recover, relink, recent projects.
  When: changing project opening, saving, recovery, conflicts or source relinking.
  Path: `docs/features/project-lifecycle.md`
- **Editing** — tracks, clips, exact-time commands, selection, snapping, undo, gestures.
  When: changing any timeline edit, command, history entry or editing interaction.
  Path: `docs/features/editing.md`
- **Media library** — import, supported profile, library surface, thumbnails, waveforms.
  When: changing import, library listing, removal, relink or generated previews.
  Path: `docs/features/media-library.md`
- **Preview and playback** — timing contract, transport, runtime behaviour, pacing evidence.
  When: changing the viewer, transport, seek/scrub, frame step or sequence rate.
  Path: `docs/features/preview-playback.md`
- **Text and captions** — text clips, captions, SRT/WebVTT, shared text path.
  When: changing titles, caption clips, subtitle interchange or text rendering.
  Path: `docs/features/text-captions.md`
- **Effects, colour and audio controls** — transforms, closed effect set, colour, gain/pan.
  When: changing clip visual settings, effects, transitions or audio parameters.
  Path: `docs/features/effects-audio.md`
- **Export** — profiles, job behaviour, staging and failure semantics, validation, interchange.
  When: changing export, an output profile, progress/cancel or an interchange format.
  Path: `docs/features/export.md`
- **MotionScene** — declarative code-to-video contract, evaluator, materialisation.
  When: working on deterministic scenes, the Motion panel or motion CLI verbs.
  Path: `docs/features/motionscene.md`
- **Workspace and UI shell** — shell slots, editor layout, interaction rules.
  When: adding or moving any control, panel, menu, shortcut or compact layout.
  Path: `docs/features/workspace-ui.md`
- **Assistant panel** — ACP harness connection, MCP editing tools, motion insertion, panel states.
  When: changing the assistant panel, its harness overlay, or the `or mcp` tools.
  Path: `docs/features/assistant.md`

### Design

- **Design language** — Focused Monochrome tokens, hard visual rules, motion, copy.
  When: any visual, layout, colour or copywriting decision in the app.
  Path: `docs/design/design-language.md`
- **UX invariants** — behaviour that must not regress, and what counts as evidence.
  When: changing an implemented interaction, or checking a UI claim.
  Path: `docs/design/ux-invariants.md`

### Operations

- **Change workflow** — the work loop from request to verified local increment.
  When: starting any non-trivial change.
  Path: `docs/operations/workflow.md`
- **Verification** — evidence classes, check ladder, how to report results.
  When: deciding what check answers your question or reporting a result.
  Path: `docs/operations/verification.md`
- **Local macOS** — build, run, check, measure commands and host traps.
  When: building or running the app on this machine, or measuring performance.
  Path: `docs/operations/local-macos.md`
- **UI harness** — driving the real app by semantics and reading artifacts.
  When: proving a user-visible behaviour or debugging an interaction.
  Path: `docs/operations/ui-harness.md`
- **Git** — commit and history rules, and actions that are out of bounds.
  When: committing, or unsure whether an external action is allowed.
  Path: `docs/operations/git.md`
- **Permissions** — what this mission authorizes and what it forbids.
  When: any build, launch, install, network or repository action needs a boundary check.
  Path: `docs/operations/permissions.md`
- **Docs maintenance** — documentation shape and the routing template.
  When: adding, moving, renaming or reviewing documentation.
  Path: `docs/operations/docs-maintenance.md`
- **OR skills** — how OR skills are packaged, installed and edited.
  When: editing a skill or its routing.
  Path: `docs/operations/skills.md`

### Reference

- **Product requirements** — the requirement inventory and maturity labels.
  When: you need to know what the product is meant to do, not what is implemented.
  Path: `docs/reference/product.md`
- **Roadmap** — current owner-selected focus, acceptance target, deferred work.
  When: choosing what to work on next or checking whether work is in scope.
  Path: `docs/reference/roadmap.md`
- **Quality bar** — attributes, measurements, known gaps.
  When: making or checking a quality, performance or reliability claim.
  Path: `docs/reference/quality.md`
- **Testing** — test layers, determinism rules, what to run.
  When: adding or choosing a regression check.
  Path: `docs/reference/testing.md`
- **Tooling** — repository scripts, machine-local tools, hosted jobs.
  When: you need an existing script or tool instead of writing a new one.
  Path: `docs/reference/tooling.md`
- **Security and licensing** — adoption rules, dependency posture, FFmpeg, secrets.
  When: adding a dependency, asset, codec, font or model, or checking licence risk.
  Path: `docs/reference/security-licensing.md`
- **Release** — Developer Preview pipeline and stable-release requirements.
  When: packaging or distribution work is explicitly in scope.
  Path: `docs/reference/release.md`
- **Developer Preview install** — installing and dogfooding a preview build.
  When: a tester needs to install or exercise a preview build.
  Path: `docs/reference/developer-preview-install.md`
- **Claim records** — schema and rules for the committed verification claim JSON files.
  When: recording, auditing or superseding a hosted verification claim.
  Path: `docs/evidence/claims/README.md`

### Decisions

- **ADR index** — recorded architecture decisions and their status.
  When: changing a subsystem boundary, or checking why an architecture choice was made.
  Path: `docs/adr/README.md`

### History

- **Archive index** — what the archived plans, delivery logs and dated receipts are.
  When: you need historical detail or a dated record, not current truth.
  Path: `docs/archive/README.md`

### Tools

- **Repository gate** — required files, whitespace, links, freshness, claims, `or_core` dependencies.
  When: before committing, or when a documentation or dependency rule may be violated.
  Path: `scripts/check-repo.sh`
- **Acceptance gate** — source gate plus Rust/Flutter lanes and the local performance budget.
  When: you need the broadest local check before declaring an increment done.
  Path: `scripts/acceptance.sh`
- **UI harness** — drive the real app by widget key, semantics label, text or role.
  When: proving a user-visible behaviour in the running app.
  Path: `scripts/or_ui.py`
- **Semantics outline** — one line per node with its rect and actions.
  When: reading a screen dump quickly.
  Path: `scripts/or_tree_outline.py`
- **Packaged driver** — CoreGraphics mouse and keyboard events for the built bundle.
  When: the semantic harness cannot reach a packaged-app surface.
  Path: `scripts/packaged_drive.py`
- **Release bundle** — build, package and codesign the macOS bundle with a bridge hash check.
  When: producing or re-verifying a packaged local build.
  Path: `scripts/build_macos_release.sh`
- **Performance budget** — measure the local budget rows.
  When: checking for a performance regression or recording a measurement.
  Path: `scripts/bench_quality.py`

### Skills

- **or-edit-feature** — canonical editing: timeline, media/project commands, clips, undo, save/reopen, recovery.
  When: changing editing or project behaviour.
  Path: `tooling/dsh-or-skills/skills/or-edit-feature/SKILL.md`
- **or-ui-ux** — layout, information architecture, pointer/keyboard interaction, accessibility, visual review.
  When: changing a user-visible surface.
  Path: `tooling/dsh-or-skills/skills/or-ui-ux/SKILL.md`
- **or-media-render** — import, decode/playback, audio, compositing, FFmpeg, export, media performance.
  When: changing the media or render pipeline.
  Path: `tooling/dsh-or-skills/skills/or-media-render/SKILL.md`
- **or-motionscene** — deterministic code-to-video scenes: authoring, preview, render, insertion.
  When: working on MotionScene.
  Path: `tooling/dsh-or-skills/skills/or-motionscene/SKILL.md`
- **or-dev-loop** — local builds, focused checks, real macOS runs, semantic UI driving, evidence classes.
  When: running or verifying anything on this machine.
  Path: `tooling/dsh-or-skills/skills/or-dev-loop/SKILL.md`
- **or-claim-evidence** — exact-SHA hosted claim records.
  When: recording or auditing a hosted claim, only when explicitly in scope.
  Path: `tooling/dsh-or-skills/skills/or-claim-evidence/SKILL.md`
- **or-release** — packaging, publishing and distribution.
  When: release work is explicitly authorized.
  Path: `tooling/dsh-or-skills/skills/or-release/SKILL.md`
