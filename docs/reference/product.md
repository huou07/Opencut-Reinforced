# Product Requirements Inventory

Last reviewed: 2026-11-05

Scope: what OR is meant to be, area by area, with maturity labels. This is the
requirement inventory, not implemented status — current work is selected by
[roadmap.md](roadmap.md), and each capability's real state is in its feature
document.

## Status and labels

OR is pre-MVP with a working project/timeline/runtime foundation. Labels describe
intended capability depth:

- **MVP FOUNDATION** — required for the initial usable editor.
- **PLANNED** — intended after or alongside MVP; no claim it exists.
- **ADVANCED** — later, deeper editing capability.
- **FUTURE** — deferred pending product, safety, licensing or platform decisions.

## Principles

- Human, CLI and agent parity: one set of domain commands.
- Local first: core editing needs no OR cloud backend; AI is optional and user-configured.
- Content first: Focused Monochrome, workspace not marketing surface.
- Deterministic core: structured, predictable, testable operations.
- Inspectable state: project data and automation interfaces stay machine-readable.
- Secure by default: no plaintext secrets to agents; all imported/generated content untrusted.
- Extensible without UI rewrites: stable shell slots and command contracts.
- Preserve working behaviour: [design/ux-invariants.md](../design/ux-invariants.md) is mandatory.
- Minimal implementation: no component, crate or dependency before a feature needs it.

## Project and workflow

**MVP:** create/open with initial options, periodic recovery checkpoints, explicit
crash-safe atomic save, clear saved/unsaved state, reliable save and reopen.
Autosave updates a recovery checkpoint and never overwrites the canonical file.

**PLANNED:** pin/archive, project settings, manual snapshots, version history,
backup, consolidation, offline-media detection and bulk relink, proxies, workspace
presets, recent-project history. Simple/Advanced modes ship as the guided-mode
preference described in [workspace-ui.md](../features/workspace-ui.md).

## Editor core

**MVP:** multitrack video/audio/text/caption tracks; selection; insert and move;
trim; split; ripple delete; delete; duplicate; snap; markers; track lock,
visibility, mute, solo; timeline zoom and scroll; playhead, scrubbing, frame
stepping, follow playhead; undo/redo; save and reopen.

**PLANNED:** grouping, compound clips, nested timelines, adjustment layers,
keyframes, graph editor. **ADVANCED:** speed ramps, time remapping, freeze,
reverse. **FUTURE:** multicam, audio/timecode sync. Linked-clip semantics are
deferred until typed semantics and migrations are approved.

## Viewer

**MVP:** fit / 25 / 50 / 100 / 200% zoom, exact seek and scrub, playback and frame
stepping on the explicit sequence frame lattice, no looping; composition guides
(title-safe, action-safe, thirds, centre) as a viewer overlay that never reaches
an exported frame; transform handles that commit one command per gesture.
**PLANNED:** fullscreen/cinema mode, rulers, bounding boxes.

## Media, text, audio, effects

See [media-library.md](../features/media-library.md), [text-captions.md](../features/text-captions.md),
[effects-audio.md](../features/effects-audio.md) and [export.md](../features/export.md)
for the contracts. Product-level expectations: useful metadata and errors,
thumbnails/waveforms, offline/relink, captions and subtitle interchange, typed
transform and effect parameters, coherent preview/export parity.

**Onboarding (delivered):** first run offers a guided first video, and
Settings → Resources measures what this installation can do instead of
promising: the machine's processors, memory and free space, the export formats
the linked encoder opens, the external tools that are optional, and the one
warning worth showing when the disk cannot stage an export.

**MVP FOUNDATION (delivered):** the import codec list and the three export
profiles are shared Rust rules, and whether a *given host* can decode a codec is
measured at import rather than assumed: the first video frame is decoded, and a
codec whose decoder exists but cannot run on this machine is refused with
`DECODE_UNAVAILABLE` and the reason. Measured rows per host, including the AV1
finding on Apple silicon, are in
[media-capability-matrix.md](media-capability-matrix.md). GIF and TIFF import,
extra export profiles (HEVC, AV1, MOV) and HDR stay PLANNED. Which of the three
delivered profiles a build can actually write is measured at runtime
(`describe_export_capability`): the Export dialog offers only those and defaults
to the engine's preferred one, and Settings → Resources lists them as Ready or
Unavailable.

## AI assistance

**MVP FOUNDATION (external harness):** an in-editor assistant panel that connects
to a locally installed DeepSeek Harness over ACP, selects workspace and model,
and edits the open project through `or mcp` and the shared command path. It can
also turn a code file into a deterministic MotionScene video and insert it as an
ordinary clip. The assistant opens by default in an app-installed agent
workspace whose `AGENTS.md` routes it to one task guide, skill or tool
([agent-workspace.md](../features/agent-workspace.md)). OR stores no provider
credentials; the harness keeps them.

**MVP FOUNDATION (delivered):** local multilingual transcription of one media
item (Vietnamese and English auto-detected) with a reviewed, editable caption
track, through the app's Generate Captions action and the host-owned
`or captions transcribe` runtime request
([text-captions.md](../features/text-captions.md)). The installed assistant
workspace also carries an offline production workflow: an `or-assistant-script`,
macOS `say` narration with exact caption timing, reviewed SRT import, and a
generated royalty-free music bed
([assistant.md](../features/assistant.md)). The Audio tool also generates
offline narration (macOS `say`, or a local keyless speech server with instant
voice cloning from a reference clip) and royalty-free music beds in-app, with
exact captions and optional −12 dB ducking under a voice track; the visible
voice and music surfaces are no longer assistant-only.

**PLANNED/FUTURE:** translation, dubbing, scene/silence analysis, selected
generative assets, hosted voices and music, and optional hosted providers
inside OR. Requirements: provider-independent tasks returning proposals/assets,
review before apply, credentials outside project/CLI, explicit consent and cost,
no bundled model weights by default, no network call in deterministic rendering.

## Motion assets

**MVP FOUNDATION (delivered):** reusable scene, fragment and image assets with
typed parameters for text content, colour, size, opacity and rotation; an
app-level JSON library with deterministic name/tag classification
(background, people, nature, animal, vehicle, ...) and a manual override; PNG
import and rendering; a Motion panel with search, previews, parameter editing
and a composition; drag to the timeline; `or mcp` and `or motion asset` tools.
A composed scene's text can be set with its own family, weight, alignment, line
height and tracking inside an explicit text box (schema version 2, ADR 0012), so
titles can carry hierarchy instead of one uniform weight. A project text clip
reaches the same two metrics from its own dialog (line height 0.80–3.00 in tenths,
tracking -5.0–20.0 points) and keeps the historical 1.32/0 when untouched. See
[motion-assets.md](../features/motion-assets.md) and
[motionscene.md](../features/motionscene.md).

**DEFERRED:** a live timeline MotionClip that stays editable in place; motion
presets applied to an existing node; a marketplace. Placement stays materialised
for now.

## Interchange and extensibility

SRT/WebVTT now; OTIO, Lottie/dotLottie, ASS, audio stems, EDL/XML as bounded
adapters later. Plugins, a community template marketplace, themes and community
distribution are future and default to no code execution. Bundled templates,
text looks, sound effects and motion scenes ship today and are covered by the
credits panel in Library → Guide.

## Accessibility, localisation, platforms

Accessibility MVP: keyboard navigation, visible focus, usable touch targets,
status not conveyed by colour alone; planned: shortcut editor, screen readers,
text scaling, reduced motion. English is the source language; Vietnamese is an
intended UI language. Release targets: macOS, Windows and Linux; Android is out of
scope for now and is not built, and iOS and web are not targets.

## Related

- [roadmap.md](roadmap.md) — what is actually being worked on
- [features/](../features/) — per-capability contracts
