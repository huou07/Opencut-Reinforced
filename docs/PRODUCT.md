# Opencut Reinforced Product Vision

## Status and maturity labels

This is the complete planned product scope, not a list of implemented features. The repository is in pre-MVP planning; the HTML prototype simulates selected workflows. Labels describe intended roadmap placement:

- **MVP FOUNDATION** — required for the initial usable editor or a product-wide foundation.
- **PLANNED** — intended after or alongside MVP, with no claim that it exists.
- **ADVANCED** — a later, deeper editing capability.
- **FUTURE** — intentionally deferred pending product, safety, licensing, or platform decisions.

Where a product area spans stages, each group of capabilities is labeled separately.

The machine-readable implementation order is [docs/execution/README.md](execution/README.md). Product labels describe scope; `PLAN.json` and `STATE.json` decide which checkpoint may execute next.

## Product principles

- **Human, CLI, and agent parity:** editing operations converge on the same domain commands.
- **Local first:** core editing does not require an OR cloud backend. AI can run locally or use an optional user-configured provider.
- **Content first:** follow the [OR Focused Monochrome design](../DESIGN.md); the workspace is a tool, not a marketing surface.
- **Deterministic core:** project and timeline operations are structured, predictable, and testable.
- **Inspectable state:** project data and automation interfaces remain structured and machine-readable.
- **Secure by default:** agents cannot retrieve plaintext secrets; project, media, community, model, plugin, and generated content is untrusted.
- **Extensible without UI rewrites:** features use stable workspace slots and command contracts.
- **Preserve working behavior:** [UX_ACCEPTANCE.md](UX_ACCEPTANCE.md) and the repository regression rules are mandatory.
- **Minimal implementation:** do not create components, crates, or dependencies before a concrete feature needs them.

## Project and workflow

**MVP FOUNDATION:** create and open projects with initial project options; recent project access; periodic recovery checkpoints; explicit crash-safe atomic Save and recovery; clear saved or unsaved state; reliable save and reopen. Autosave updates a recovery checkpoint and never silently overwrites the canonical project file.

**PLANNED:** pinned and archived project organization; full project settings; manual snapshots; before-agent and before-template snapshots; version history; backup; project migrations; collect project and consolidate media; relink and replace media; offline-media state; proxy and optimized media; disposable cache; workspace presets; Simple and Advanced workspace modes.

**Workflow references:** Home and Projects support quick create, open, recent work, pinning, archive state, search, filters, sort, and grid or list presentation. Project-safety workflows include snapshot preview, restore, and duplicate. Import media and folders, template entry, and return-to-project navigation are represented as product workflows.

## Editor core

**MVP FOUNDATION:** multitrack video, audio, text, and caption tracks; selection; insert and move; trim; split; ripple delete; delete; duplicate; snap; markers; track lock, visibility, mute, and solo; timeline zoom and horizontal scroll; playhead, scrubbing, frame stepping, and follow playhead; undo and redo; save and reopen.

Complex linked-clip and group semantics are **ADVANCED** editing scope (Phase
13), not a Desktop MVP requirement. Basic track enabled/locked/solo behavior is
MVP; linked/unlinked relationships are intentionally deferred until their typed
semantics and migration rules are approved.

**PLANNED:** grouping and ungrouping; compound clips; nested timelines; adjustment layers; keyframes; graph editor.

**ADVANCED:** constant speed, speed ramp, time remapping, freeze frame, and reverse.

**FUTURE:** multicamera editing; synchronization by audio, timecode, or manual alignment may be evaluated later.

## Viewer

**MVP FOUNDATION:** fit-to-view, 25%, 50%, 100%, and 200% zoom; exact seek and scrubbing plus playback and frame stepping on the explicit sequence frame lattice. Play and frame step require an explicitly configured project rate. Playback does not loop.

**PLANNED:** fullscreen preview; cinema or viewer-focus mode; safe areas, grid, guides, rulers, bounding boxes, and transform handles.

## Desktop MVP implementation map

The optimized Desktop MVP milestone is represented by the execution graph from
6E2B through 8F:

- **Timeline foundation (6E2B):** complete persistent marker read models and
  bounded bridge paging, the marker ruler, Add/Move/Rename/Delete UI, and
  marker-aware pointer Snap V2 with save/reopen/recovery UI hardening.
- **Runtime foundation (7A–7H):** capability policy, `RenderSnapshot`,
  `FrameDescriptor`/`FrameLease`, wgpu/render, software and hardware media
  paths, audio clock, viewer transport, playback, scrubbing, frame step, and
  performance/hardening gates. 7F0 locks the explicit sequence rate, exact
  frame lattice, and shared viewer presentation contract without adding viewer
  UI or playback controls. 7F1 proves desktop FFmpeg runtime packaging and the
  native Flutter pixel-buffer texture adapter on macOS, Linux, and Windows.
  7F connects the product viewer and exact-time transport through the Rust
  runtime; hosted verification remains required before checkpoint advancement.
- **Project/timeline gate (8A):** one schema-v6 typed timeline model gate for
  Video, Audio, Text, and Caption content, typed track state, and the project
  settings consumed by 8B–8E.
- **Timeline usability (8B):** selection, duplicate, track enabled/locked/solo,
  timeline zoom, direct media-to-timeline insertion, and bounded viewport input.
- **Video (8C):** typed transform, crop, and opacity, edited through bounded
  Inspector controls and applied by the shared preview render path.
- **Text (8D):** basic text and manual captions through cosmic-text 0.19.0
  using the bundled deterministic Inter 4.1 font identity. Automatic captions
  are Phase 10, not Desktop MVP.
- **Audio/effects (8E):** basic gain, pan, fades, transitions, and closed typed
  effects; desktop output uses cpal 0.18.1 inside `or_audio`.
- **Release behavior (8F):** Matroska + FFV1 + PCM S16LE software export,
  recovery-checkpoint autosave, explicit Save/reopen, and preview/export
  hardening.

Linked clips, grouping, nested timelines, multicamera, and other complex
relationship semantics remain advanced Phase 13 scope unless re-promoted by a
separate plan amendment.

## Video tools

**MVP FOUNDATION:** transform position, scale, rotation, anchor, and opacity; crop.

**PLANNED:** blend modes, masks, chroma key, stabilization, motion tracking, object tracking, subject detection and segmentation, background removal, and automatic reframing.

These controls act on project objects through domain commands. AI-assisted operations use the same command, permission, and validation path.

## Text

**MVP FOUNDATION:** add and edit basic text and title clips on the timeline with a minimal set of formatting controls. Export geometry uses the bundled Inter 4.1 baseline through the shared cosmic-text 0.19.0 shaping/render path, independent of host-installed fonts.

**PLANNED:** body text, lower thirds, credits, callouts; font, weight, size, alignment, letter spacing, line height, fill, stroke, shadow, and background controls; text templates, saved styles, font library, and in/out/loop animation.

**ADVANCED:** kinetic typography.

## Captions and transcript

**PLANNED:** automatic captions; local transcription; word timestamps; caption segmentation; inline editing; transcript editor; text-based timeline editing; filler-word and pause removal; caption templates; speaker styles; karaoke and word highlighting; translation and bilingual captions; SRT, VTT, and ASS import or export.

The transcript and caption workflow includes searching, replacing, jumping from a transcript segment to its timeline position, and reviewing translated text. Recognition and translation remain editable outputs, not authoritative project instructions.

## Dubbing and voice

**PLANNED:** AI voiceover; subtitle-to-speech; translate and dub; source and target languages; speaker mapping and multiple voices; pronunciation dictionary; timing fit; speed, pitch, and loudness controls; regenerate an individual segment; keep, lower, or mute original dialogue; voice isolation; automatic music ducking.

**FUTURE:** voice cloning, only with explicit consent requirements and a separately approved safety design; lip sync.

## Audio

**MVP FOUNDATION:** basic project audio editing and playback, gain, pan, mute, solo, and fades. Desktop device output uses cpal 0.18.1 through `or_audio`; its callback consumes bounded prepared buffers and does not mutate projects or take project locks.

**PLANNED:** mixer; normalization and loudness controls; EQ; compressor; limiter; noise reduction; voice enhancement and isolation; automatic ducking; beat detection; music, sound-effect, and ambience libraries; audio stems.

## Effects and transitions

**MVP FOUNDATION:** a small useful set of basic transitions and effects, evaluated by the shared preview and export pipeline.

**PLANNED:** effect categories for color, blur, sharpen, distortion, light, stylize, utility, and audio; cut, cross dissolve, fade, dip, slide, push, zoom, wipe, mask, light, and stylized transitions; save effect presets; favorites; apply a transition to an edit point.

## Filters and color

**PLANNED:** quick filters; exposure, contrast, highlights, shadows, whites, blacks, temperature, tint, saturation, and vibrance; curves; HSL; LUTs; waveform, vectorscope, and histogram; color management.

**FUTURE:** HDR workflow, after output, display, and test-fixture requirements are defined.

## Image tools

**PLANNED:** crop, resize, rotate, flip, exposure, brightness, contrast, temperature, saturation, blur, sharpen, masks, background removal, AI subject selection, and send to timeline.

## Creative asset library

**PLANNED:** text styles, fonts, music, sound effects, ambience, stickers, shapes, effects, transitions, filters, LUTs, templates, and themes from built-in, project, downloaded, and community sources.

Each asset records source, author, version, license, compatibility, dependencies, and checksum. Users can preview, inspect details and license metadata, favorite, download, remove, and add eligible assets to a project.

## Templates

**PLANNED:** browse, community, downloaded, and personal templates; categories; preview and details; use a template; and a creator workflow for info, editable slots, dependencies, preview, validation, save, export, and publish.

Slot types are replaceable media, editable text, optional audio, editable color, locked, and optional. Templates are declarative project data with stable slot IDs and dependency metadata. They contain no arbitrary executable code. Future templates may instantiate concrete, validated MotionScene structures with typed and bounded parameters; they do not execute code at render time.

**FUTURE:** simple declarative MotionScene source assets for explainers,
diagrams, charts, kinetic text, and other bounded motion graphics. A
MotionScene uses exact timing, typed primitives, deterministic seekable
evaluation, asset/font provenance, and the shared OR render/export path. The
first timeline workflow is materialization-first: validate and preview the
source, render persistent generated media, register it through normal media
commands, and add the resulting ordinary media to the timeline. Re-rendering
does not silently replace an existing canonical media identity.

MotionScene authoring may come from a human, template, external agent, or
optional AI provider. Validation, inspection, rendering, and materialization do
not require an AI provider. The initial UX is a source item with validation
state, duration, canvas, missing-asset diagnostics, Preview, Render/Rerender,
and Add rendered media; a full node graph, curve editor, browser IDE, and code
editor are not 11F requirements.

## Themes

**PLANNED:** Focused Monochrome as the default; built-in, downloaded, community, and personal themes; a token-based format; live preview; create, duplicate, import, and export.

A theme may set approved semantic tokens. It cannot execute code or redefine application behavior or layout architecture. Theme switching must not change navigation or editor structure.

## AI assist

**PLANNED:** automatic captions, translation, dubbing, silence and filler removal, scene detection, auto reframe, subject detection and tracking, background removal, highlight extraction, transcript editing, and optional declarative MotionScene generation.

Phase 10 defines the provider-independent task boundary: `Transcribe`,
`Translate`, `TextToSpeech`, `Segment`, `DetectScene`, `PlanEdit`,
`GenerateImage`, `GenerateVideo`, `GenerateMotionScene`, and `GenerateAudio`.
`GenerateVideo` returns an opaque raster/video asset; `GenerateMotionScene`
returns an editable declarative scene proposal. Providers return
reviewable proposals, analyses, or assets with model manifests and provenance;
only normal validated commands apply accepted results. Stored credentials never
enter project, CLI, or agent output.

Phase 10A owns one typed provider manager and a bounded local sidecar protocol
for later AI work. Its model manifest foundation is separate from
`ProjectDocument`; provider or model absence is a normal `Unavailable`
capability, and no roadmap checkpoint requires cloud credentials or bundled
model weights.

**FUTURE:** semantic media search.

AI output is untrusted data. The user can inspect and edit it, and any proposed project mutation goes through domain validation and permission checks.

## AI generation

**FUTURE:** image, video, music, sound-effect, voice, and declarative
MotionScene generation with prompt, reference media, model, provider, seed,
quality, and output parameters; queue and history; review results and add them
to the timeline. MotionScene rendering itself makes no LLM/provider calls and
does not require a cloud service.

Generation is not required for MVP. Local and cloud provider use stays optional and must respect user-selected settings and content rights.

## Agent-native workflows

**MVP FOUNDATION:** the shared command model, semantic CLI, JSON output where appropriate, stable exit codes and IDs, command and capability discovery, project and timeline inspection, dry-run support, and semantic/domain operation parity with the GUI.

**PLANNED:** strict EditPlan schema; permission review; validation; dry run; diff preview; apply as one undoable transaction; undo agent edit; task queue; declarative automation recipes.

Agents do not control normal editing by visual UI clicks, do not receive plaintext API keys, and cannot bypass command, permission, or validation layers.

CLI parity means semantic/domain operation parity: if an operation changes or meaningfully inspects project state and can be represented semantically, it should normally be available to CLI and agents. Examples include creating, opening, and inspecting projects; importing media; listing timelines; splitting, moving, and trimming clips; setting opacity or text properties; adding text or effects; generating or translating captions; exporting; creating snapshots; and inspecting capabilities. Parity does not require CLI exposure for presentation-only controls such as collapsing an inspector, viewer zoom, cinema mode, hover tooltips, mobile sheets, sidebar resizing, or moving the application window.

## Privacy and network control

**MVP FOUNDATION:** core project editing works locally without an OR cloud service.

**PLANNED:** an application-level Offline Mode denies OR-originated optional network provider calls, community browsing and downloads, and cloud AI while leaving local editing available. This governs OR's own network behavior; it is not an operating-system firewall.

**PLANNED:** local AI workflows do not intentionally upload project content. Cloud providers are optional. Before sending project-derived data, the provider/task boundary identifies its kind—such as prompt text, subtitles, audio, images, video or reference media, or project metadata—and the UI communicates meaningful network and data-use status. Agents cannot bypass the same provider and network permissions.

**PLANNED:** telemetry is off by default. Any future telemetry requires documentation and privacy review and must not include project media, content, or secrets by default.

## Model management

**PLANNED:** installed and available models, updates, and storage use; model metadata for ID, version, task, source, hash, size, runtime, hardware needs, language, license, and install state; download, pause, resume, verify, update, and remove.

10A defines the checksum-verified artifact manifest and managed storage
boundary; 10H owns the user-facing manager. Weights are stored outside the Git
repository and are not bundled by default. A missing model is reported as
typed `Unavailable`.

## Local and cloud providers

**PLANNED:** local providers and optional user-configured cloud providers, including OpenAI-compatible-style, Gemini-compatible-style, and custom adapters. Provider capabilities are described by task rather than embedded as vendor-specific editing logic.

Secrets use OS secure storage. The application may reveal configured or not configured status; agents and CLI never receive stored plaintext credentials.

## Community

**FUTURE:** template, asset, and theme browsing; publishing and versioning; dependencies; license metadata; compatibility; and updates.

The initial architecture can use a GitHub-first static registry with versioned manifests and release assets, automated validation, and pull-request-based publishing. An OR-hosted backend is not required in early phases. Do not display fake popularity statistics.

## Plugins

**FUTURE:** effect, transition, importer, exporter, AI-provider, automation,
and optional procedural WebMotion extensions. Begin with a sandbox and explicit
capability permissions where feasible. Native and OpenFX compatibility is a
later, higher-trust evaluation, not an unrestricted default. WebMotion is not
canonical MotionScene and may run only in an explicit isolated, bounded
sidecar; it is never the normal renderer or project-open path. Phase 16B uses
wasmi 1.1.0 in the extension runtime boundary without default WASI; its fuel,
store, and outer host resource limits are explicit. Native extensions remain
separately gated.

## Export

**MVP FOUNDATION:** usable software video export through a background job with progress and cancellation. The mandatory correctness profile is Matroska + FFV1 video + PCM S16LE audio through linked software FFmpeg. Other delivery and hardware profiles are optional and require separate evidence.

**PLANNED (optional delivery profiles):** MP4, MOV, and WebM containers; codecs H.264, H.265, AV1, and VP9 only after legal, platform, and build-configuration review; hardware encoding where supported. Desktop MVP correctness uses the required software Matroska + FFV1 + PCM S16LE profile.

## Interchange

**PLANNED:** SRT, VTT, ASS, audio stems, and OpenTimelineIO import or export as appropriate.

OpenTimelineIO is a bounded Rust/serde JSON interchange adapter over a documented subset, not the native OR project format or a media container. Unsupported content receives explicit diagnostics.

**FUTURE:** EDL and XML workflows.

## Accessibility

**MVP FOUNDATION:** keyboard navigation, visible focus, usable touch targets, and status that is not communicated by color alone.

**PLANNED:** shortcut editor, screen-reader support, text scaling, and reduced-motion preference. Accessibility work applies across product areas; minimal visual design must not reduce access.

## Localization

**PLANNED:** English may be the initial source language, and Vietnamese is an intended UI language. The architecture should support additional translations without promising completeness or delivery dates. Visible application strings and accessibility labels should be localizable; date, number, and time formatting should be locale-aware where appropriate; layouts must tolerate translated text length. Community translations may be considered later.

## Platforms

**MVP FOUNDATION:** desktop release targets are macOS, Windows, and Linux.

**PLANNED:** Android with the same project and Rust core, touch-native Flutter presentation, and resource-aware editing. 9A0 proves FFmpeg native packaging and bridge loading for `arm64-v8a`, `armeabi-v7a`, and `x86_64`; 9A uses typed SAF source references and an app-private canonical working copy. Keep preview, transport, and timeline accessible while editor tools use mobile sheets or panels.

iOS and web are not current release targets.

## Developer tools

**PLANNED / ADVANCED:** CLI, Agent Hub, Command Inspector, Capability Explorer, diagnostics, plugin management, and automation recipes. Developer features remain discoverable in Settings under Advanced or Developer and do not crowd the normal editing workspace.

## Prototype coverage audit

The frozen prototype is a UX and product reference only. A full source inspection confirmed the expected inventory:

- **25 registered views:** Home, Projects, Templates, Asset Library, Editor, Media, Captions, Audio, Effects, Transitions, Color, Image Tools, AI Studio, Agents & CLI, Models, Export, Settings, Plugins, Feature Map, Snapshots, Downloads, Diagnostics, Command Inspector, Capability Explorer, and Automation Recipes.
- **11 editor tool panels:** Media, Templates, Text, Captions, Stickers / Shapes, Audio, Effects, Transitions, Filters / Color, AI, and More.
- **18 Feature Map groups:** Project & Workflow, Editor Core, Video, Text, Captions, Dubbing, Audio, Color, Creative Assets, AI Assist, AI Generation, Agent-Native, Community, Infrastructure, Export / Interchange, Accessibility, Platforms, and Developer.

The product areas and workflows above account for those routes, panels, and groups, including project and media management, editor controls, safety and recovery, asset and template creation, themes, AI and model management, agent plans and permissions, export jobs, and developer discovery. The prototype's JavaScript and simulated state do not establish production architecture or implementation status.
