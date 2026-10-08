# Open-Source Convergence Audit

## Decision date and scope

This is the evidence snapshot that changes the active product direction. It
compares the repository at product SHA
`4261f43cd03b78e445c3f278ebb53dec15843cb3` (9D implementation; supervisor
evidence is pending) with upstream source, releases, tests, platform claims,
and license terms checked on 2026-10-09. The upstream refs below make this
snapshot reproducible; this document does not claim that future upstream work
has already shipped.

The old roadmap remains intact as historical requirements and traceability.
This audit changes implementation choices only where evidence supports a
different route. Product invariants, accepted UX requirements, security
boundaries, data compatibility, and licensing obligations remain authoritative.

## Executive architecture decision

Keep OR as an MIT, human-first, shared desktop/mobile product while it proves
that its editor-specific layer adds value. Keep its versioned project format,
exact-time editing model, Rust command/session/recovery authority, semantic
CLI, Flutter product UI, and wgpu preview/export direction. These are working
OR code with a coherent desktop/mobile and human/automation contract; none of
the inspected candidates is currently a demonstrated drop-in replacement for
that whole contract.

Do not adopt a candidate on roadmap promises. OpenCut's rewrite is the closest
strategic overlap but its checked Rust workspace has no editor core or Editor
API yet, its desktop README describes a window shell, and its tracker labels
those capabilities as future/input work. OpenTake has more concrete Rust
editing/runtime structure, but is GPL-3.0, beta, desktop-first, and lacks
verified Android parity. MLT is the strongest bounded media-engine candidate:
an active LGPL framework used by mature editors, but adopting it would add a
C++/module/package boundary and has not yet demonstrated OR's mobile and
runtime needs. Run a small MLT adapter comparison before any engine migration.

The first product priority is to finish and verify the ordinary editor journey
on the existing architecture. Defer a plugin marketplace, community backend,
local model catalog, and speculative procedural runtime until users can create,
import, edit, preview, save/recover, and export a useful project. Optional AI
must arrive as explicit, reviewable tasks that produce proposals/assets and
apply through the same validated project commands.

Decision vocabulary outcome: **KEEP** the working OR state/command/UI and
media-runtime contracts; **REUSE** FFmpeg and optional provider capabilities;
**BUILD** only missing user-facing workflows on those boundaries; **DEFER or
REMOVE** speculative plugin/marketplace/model-manager scope. No code is
approved to **PORT** or **REPLACE** an OR subsystem today because no candidate
has shown a product-level win under OR's platform, license, and runtime
requirements. **UPSTREAM** a generic fix when a reproducible OR issue is found
in a healthy project and that upstream accepts contributions; no speculative
patch is justified by this audit alone.

## Subsystem decisions

| OR subsystem | Decision | Evidence and rationale | Next action / boundary |
|---|---|---|---|
| Project/document model | **KEEP** | OR has strict versioned `.orproj` schemas, explicit migrations, exact rational time, bounded atomic save, recovery ancestry checks, and Android SAF identities. Candidate editors do not demonstrate equal OR compatibility or a shared human/CLI/agent mutation contract. | Preserve schema and recovery tests. Add interoperability only through previewable, bounded import/export adapters. |
| Timeline/editing core | **KEEP** | Rust owns typed tracks/clips, revision-checked commands, undo/redo, and deterministic CLI parity. MLT, Shotcut, Kdenlive, and OpenShot have mature editing engines but their project semantics and UI are not directly compatible with OR/mobile. | Finish ordinary editing interactions; later measure an OTIO subset for exchange, not as canonical state. |
| Media ingest/probing | **REUSE** | FFmpeg/ffprobe are established format tools and already underlie OR's packaged runtime/probe path. OpenShot/libopenshot and MLT add alternate media stacks rather than removing the need for codecs. | Keep FFmpeg as the media compatibility layer. Keep source permission/SAF behavior platform-owned and tested. |
| Decode/playback | **BUILD + COMPARE** | OR already has a bounded FFmpeg software decode path, immutable frame snapshots, cancellation/queue limits, and wgpu preview. MLT provides a mature LGPL alternative, but adapter cost, Android, latency, seek behavior, and runtime packaging are unmeasured. | Build a narrow MLT evaluation adapter against representative OR fixtures and compare seek, playback, memory, packaging, and Android feasibility. Do not replace until it wins materially. |
| Renderer/compositor | **KEEP** | The OR wgpu render spine is shared and platform-neutral; MLT/Shotcut/Kdenlive/libopenshot render stacks are mature but C++-centric and do not establish the required Flutter/mobile parity. | Keep one OR evaluation/render contract. Permit native surfaces only behind explicit adapters and measured fallbacks. |
| Audio playback/mixing | **KEEP + COMPARE** | OR has bounded prepared audio buffers and a native cpal callback boundary. MLT/Shotcut/Kdenlive have mature audio routing/effects that may be reusable at a process/runtime boundary, but no current parity or package test exists. | Measure sync, device behavior, resource limits, and Android. Reuse MLT only if adapter and distribution evidence are favorable. |
| Export | **BUILD on existing FFmpeg path** | OR packages a pinned LGPL-only FFmpeg runtime with recorded build provenance. Mature editors export well, but adopting their GPL applications is not a compatible code shortcut. | Finish packaged export workflows, codec/output choices, cancellation/progress, and preview/export parity. Track each shipped FFmpeg component/license. |
| Proxies/cache | **KEEP** | OR already has bounded background jobs, disposable indexed cache, LRU eviction, and proxy-generation primitives; cache does not enter canonical project state. No inspected project justifies replacing this with an unmeasured service. | Expose only user-valued proxy controls after real media measurements and cleanup/recovery checks. |
| Thumbnails/waveforms | **KEEP** | OR generates bounded FFmpeg artifacts through its existing job/cache path. This is small, compatible work with no proven superior drop-in. | Preserve limits and packaged dependency tests; improve only with observed UX/performance gaps. |
| Effects/transitions | **KEEP semantics; REUSE selectively** | OR has typed brightness/contrast/saturation/blur and transition semantics shared by preview/export. MLT/Frei0r contain mature effect implementations but have module-by-module license and behavior differences. | Compare selected filters through MLT/FFmpeg/wgpu proof fixtures. Preserve OR parameter semantics and preview/export parity; no wholesale effect ABI yet. |
| Captions/text | **KEEP** | OR's typed caption/text clips and deterministic bundled-font rendering align with project and export invariants. OpenMontage/MoneyPrinterTurbo generate subtitle workflows, not a directly compatible editor text model. | Prioritize normal caption creation/edit/import/export and optional transcription proposals after the core edit journey is solid. |
| UI/editor interaction | **KEEP; improve** | OR's Flutter workspace and Focused Monochrome design are the only checked candidate UI intended to share desktop and touch concepts. Shotcut/Kdenlive/LosslessCut are mature desktop references, not reusable UI code; Palmier is closed for contribution and Mac-only. | Make visible journeys complete and understandable. Use mature editors as interaction references, not as a reason to replace the shared product shell. |
| Desktop/mobile shells | **KEEP Flutter; retain iOS contract only** | OR currently packages macOS, Windows, Linux, and Android through a shared Flutter product surface. OpenCut's rewrite multi-platform direction is not implemented; OpenTake and inspected mature NLEs do not prove this parity. iOS is not currently a supported target. | Verify shared semantics on all current targets. Keep iOS as an architectural intent, not an acceptance claim, until toolchain, packaging, and native journey exist. |
| CLI/application API | **KEEP** | OR's typed Rust application requests and local IPC let GUI and CLI use one project host with revision checks and no TCP fallback. OpenCut's Editor API is a planned capability in the checked rewrite, not shipped evidence. | Keep commands semantic, discoverable, stable, and secret-safe. Use end-to-end parity tests. |
| Agent/MCP integration | **BUILD on the command API; defer broad MCP surface** | OR has safe CLI/control primitives; OpenCut MCP/headless support is future tracker work. OpenTake has an MCP package but is GPL-3.0 and its compatibility with OR is unproven. | Add only task-oriented proposal/apply operations after human workflows exist. Never let an agent bypass revision checks or mutate a second document. |
| Plugins | **DEFER / REMOVE from near-term scope** | OR's planned sandbox/API is substantial security/runtime work. OpenCut plugins are future work; no inspected compatible mature plugin host satisfies OR's platform, permission, resource, and licensing contract. | Keep declarative templates/assets and typed extension boundaries. Revisit executable plugins only for demonstrated user demand and a reviewed threat model. |
| Generative AI workflows | **REUSE provider capabilities; BUILD thin OR workflows** | Open Generative AI is MIT and useful as a provider/UI catalog reference, but has a separate Electron/web stack, external service/local-model setup, and incomplete video wiring. MoneyPrinterTurbo is a real Python short-video pipeline, not a timeline editor; its bundled sample music and voice-data behavior need rights/privacy review. OpenMontage is an AI-agent production pipeline, not an editor. | Provide opt-in providers and task adapters for selected high-value features. Make outputs editable proposals/assets with provenance; do not embed other apps, default model weights, or opaque content libraries. |
| Packaging | **KEEP and complete** | OR already builds platform packages and records FFmpeg source/configuration provenance, but Developer Preview artifacts are debug builds, not stable releases. Shotcut/Kdenlive/LosslessCut show mature packaging expectations; their packages are not OR's license/product. | Finish clean-machine install/update/uninstall and signed-release paths per target. Use hosted native/runtime verification. |
| Persistence/recovery | **KEEP** | Exact-base atomic storage, schema migrations, dirty guards, and recovery sidecars are OR's demonstrated data-safety advantage. None of the candidates proves drop-in recovery compatibility with `.orproj`. | Preserve migrations and adversarial recovery tests; add Android SAF/media import as an explicit real product gap where unsupported. |
| Collaboration/community extensibility | **BUILD small, defer services** | OR is MIT and can accept contributions, but no backend/community service is required for local editing. OpenCut currently says it is not ready for outside code contributions while its rewrite architecture is designed; OpenMontage is AGPL. | Prioritize documented formats, contribution guide, reproducible builds, optional static registries, and upstream contributions where a healthy upstream accepts them. No account/backend/marketplace until a real use case justifies it. |
| Generic upstream fixes | **UPSTREAM when evidence identifies one** | MLT, FFmpeg, and established NLE projects are maintained upstreams. The audit found no independently reproduced defect where an OR change would be more appropriate than a focused upstream fix. OpenCut's current rewrite is not accepting outside code contributions. | For a reproducible upstream defect, check contribution policy and send a minimal tested patch; avoid maintaining an unnecessary fork. |

## Candidate evidence and license boundaries

### OpenCut rewrite — closest strategic peer, not a current foundation

Checked repository: [OpenCut](https://github.com/OpenCut-app/OpenCut),
MIT-licensed at the checked revision. Its own
[rewrite tracker](https://github.com/OpenCut-app/OpenCut/issues/811) describes
Rust core, Editor API, plugins, desktop/mobile/browser, MCP, headless, and
scripting as future work. The checked workspace includes a desktop GPUI window
and a `media` area, but no Rust editor/core crate in the workspace; the desktop
README calls the current state a window shell and its panels are placeholders.
The README also says the classic product is the usable editor and asks the
community to wait while rewrite architecture is designed. This is evidence to
monitor and learn from, not evidence that OR can replace its working project,
timeline, or package layers today.

**Decision: KEEP OR while monitoring OpenCut.** Re-evaluate by shipped API,
tests, packages, and a runnable editor. If that evidence becomes strong, first
prototype a narrow adapter or upstream a generic improvement; do not fork or
migrate solely because product descriptions overlap.

### OpenTake and Palmier Pro

[OpenTake](https://github.com/appergb/OpenTake) is a community fork of Palmier
Pro at GPL-3.0. Its checked source has a multi-crate Rust/Tauri architecture
covering project, operations, media, render, agent, and generative workflows,
plus CI and tests. The checked release/product evidence is beta and desktop
oriented; Linux/Windows runtime and installer evidence is less complete than
its macOS path, and it does not demonstrate OR's Android/shared Flutter model.
This makes it a valuable architecture and interaction benchmark, not a
license-compatible code base for MIT OR.

[Palmier Pro](https://github.com/palmier-io/palmier-pro) now distributes a
proprietary macOS/Apple-Silicon product and does not accept community code
contributions. The public source history ends at an older GPLv3 release; later
binary functionality is not evidence of reusable source. **Decision: do not
copy Palmier code. Benchmark OpenTake behavior and monitor upstream only if a
license-compatible contribution path and platform evidence appear.**

### MLT, Shotcut, Kdenlive, OpenShot, and LosslessCut

[MLT](https://github.com/mltframework/mlt) is an actively maintained LGPL-2.1
media framework with module-based producers, filters, transitions, and
consumers; [Shotcut](https://github.com/mltframework/shotcut) and
[Kdenlive](https://invent.kde.org/multimedia/kdenlive) demonstrate it in mature
GPL desktop editors. Their release notes and test/packaging systems show real
maintenance, but their application frontends are not an MIT-compatible code
shortcut. MLT itself is the candidate to test: its C++ ABI/module packaging,
selected module licenses, per-platform runtime footprint, exact seek, audio,
and Android path must be demonstrated before adoption.

[libopenshot](https://github.com/OpenShot/libopenshot) is LGPL-3.0 and
actively built/tested, while the OpenShot application is GPL-3.0. Its C++,
OpenCV, FFmpeg, audio, and language binding stack overlaps the OR media layer
but does not demonstrate a lower-cost shared mobile integration. **Decision:
MLT gets a bounded comparison POC; libopenshot remains a secondary candidate
only if it solves a measured gap better.**

[LosslessCut](https://github.com/mifi/lossless-cut) is GPL-2.0, Electron-based,
and focused on stream-copy trimming/remuxing. It has valuable UX and media
handling lessons, but its narrower editing semantics do not replace OR's
compositor/timeline goal and its code is not copied into MIT OR.

### Motion, generation, and production pipelines

[Motion Canvas](https://github.com/motion-canvas/motion-canvas) is MIT and a
useful TypeScript/vector-animation authoring and rendering reference. It is
not a general NLE. Keep OR's declarative, exact-time MotionScene contract;
consider bounded asset interchange only after the core product is useful.

[Remotion](https://github.com/remotion-dev/remotion) is a capable React/browser
render system with a custom commercial license and limits on redistribution
and commercial use. **Do not embed it in OR** absent explicit license approval
and a measured product benefit. Treat it as a workflow/reference project.

[MoneyPrinterTurbo](https://github.com/harry0703/MoneyPrinterTurbo) is MIT,
maintained Python software with tests and a CLI/Streamlit workflow that creates
short videos from scripts, stock media, voice, subtitles, and music. It does
not provide normal timeline editing in its web UI. Its source explicitly flags
rights for bundled music and data flow for voice providers. Reuse the idea of
optional generated assets or an import/export bridge; do not ship its Python
stack or default assets inside OR.

[Open Generative AI](https://github.com/Anil-matcha/Open-Generative-AI) is MIT
at its top-level UI and has a useful catalog of image/audio/video provider
integrations. It combines Electron/web subsystems, external APIs, and local
models; some video paths depend on separately hosted GPU services, and the
checked README leaves parts of video wiring as planned. Inspect submodule and
model licenses individually. Reuse provider concepts only through a narrow,
tested OR task adapter.

The authentic [calesthio/OpenMontage](https://github.com/calesthio/OpenMontage)
is AGPL-3.0 Python/FFmpeg/Remotion-oriented production automation with tests;
it is not a normal interactive NLE. Use it as an AI-assisted production
workflow reference; do not incorporate its code into the MIT product.

Provenance warning: a separate
[Open-Montage-app/OpenMontage](https://github.com/Open-Montage-app/OpenMontage)
lookalike advertises installer assets despite having only two commits and no
relationship established to the authentic upstream. An open issue on the
authentic project's [issue #626](https://github.com/calesthio/OpenMontage/issues/626)
reports that the similarly named repository is suspicious;
that report is not a maintainer-confirmed malware finding. Do not download or
execute those binaries, and do not attribute them to the authentic project.

## License/provenance policy for convergence

- OR remains MIT unless the operator explicitly approves a project-wide
  license change after legal review.
- Do not copy GPL/AGPL source or link those components into OR without an
  explicit compatibility analysis and product-license decision. A separate
  process is not automatically an obligation-free boundary; distribution and
  derivative-work terms still require review.
- LGPL components such as MLT/libopenshot require the selected module's own
  license, dynamic/static linking, relinking, notices, source, and packaging
  obligations to be reviewed per target. FFmpeg remains governed by the exact
  configure flags and libraries recorded in build provenance.
- MIT at a repository root does not license every submodule, model, dataset,
  binary, media asset, or remote provider. Preserve upstream notices and
  identify the exact revision and component before reuse.
- Keep a source URL, exact revision, license evidence, modifications, and
  distribution obligations for every adopted or ported component. Prefer an
  upstream contribution when the change is generic and the upstream accepts
  it.

## Evidence gaps and explicit experiments

This audit is not a runtime benchmark. No project was installed or run locally
as part of this evidence snapshot. Before replacing any working media
subsystem, run one bounded hosted comparison using the same generated and
redistributable fixtures:

1. OR FFmpeg/wgpu baseline versus an MLT-backed decode/render/export adapter.
2. Exact seek and preview/export pixel/audio agreement on representative
   H.264/AAC, intra-frame, variable-frame-rate, text, alpha, and transition
   cases that OR claims to support.
3. CPU/GPU time, peak RSS, queue depth, frame latency, seek latency, and
   packaged runtime size on macOS, Windows, Linux, and Android where the
   candidate can actually build.
4. Clean package install and end-to-end import/edit/save/reopen/export journey
   with only shipped dependencies.
5. License/module/package inspection for the exact MLT build and FFmpeg
   configuration.

OpenTake can be added as a desktop behavior benchmark, but GPL code remains
outside OR unless the license decision changes. OpenCut should be re-audited
when it ships an editor core/API, externally usable contribution path, and
verifiable packages. These gates prevent both premature rewrites and
indefinite discussion: the next product work is the useful user journey, while
the engine experiment runs only when it can resolve a real architecture choice.

## Snapshot ledger

These are the inspected source snapshots or release markers used above. A
snapshot is not an endorsement and must be refreshed before integrating code.

| Project | Inspected snapshot | Code/test/release observations |
|---|---|---|
| OpenCut rewrite | `OpenCut-app/OpenCut@e668010778568641babef2cc40be4703ae6916d6` (2026-09-24) | MIT; workspace contains `apps/desktop`, GPUI, and media setup; no active Rust editor-core/API crate; placeholder desktop panels; README says rewrite is early and outside code contributions are not yet open. |
| OpenTake | `appergb/OpenTake@282278e9f89a6b2ed3a74f14c4ecd150deea0fb1` (2026-09-07) | GPL-3.0; Rust/Tauri multi-crate core; CI/tests and dated QA artifacts; beta source product; macOS strongest evidence, Windows/Linux and mobile less demonstrated. |
| Palmier Pro | `palmier-io/palmier-pro@c948160b8fc304a3b2a86bf4492329cfedf0230f` (2026-10-05) | Proprietary v0.11.0 binary for macOS 26+/Apple Silicon; public GPL source stops at v0.7.6; small visible test set and no contribution path. |
| Open Generative AI | `Anil-matcha/Open-Generative-AI@00132ece7c38b6f2e428793973ae99c6953d899e` (2026-10-07) | MIT top-level app; v2.0.0 release; Electron/web plus provider/submodule integrations; selected tests; external hosted GPU and multi-GB local-model paths. |
| MoneyPrinterTurbo | `harry0703/MoneyPrinterTurbo@9887b459e754ee073c056d988499b91136af9662` (2026-10-08) | MIT; v1.3.8; active Python/FastAPI/Streamlit/CLI project, CI and tests; local-project workflow is CLI-only rather than a web timeline editor. |
| Authentic OpenMontage | `calesthio/OpenMontage@9327439db69021ab4b0e2776729bf3b58fdb5a87` (2026-10-03) | AGPL-3.0; 110 test files in checked tree; Python/FFmpeg/Remotion-oriented agent pipeline; no GitHub releases. |
| Motion Canvas | `motion-canvas/motion-canvas@7b91435c301d530351dcf5ebb91dd139c002e405` (2025-07-02) | MIT; stable v3.17.2 with a newer alpha line; tests/E2E and CI; animation-authoring focus rather than NLE. |
| Remotion | `remotion-dev/remotion@90e3db961921dadf99e8fd61eb18a6cd09265b4b` (2026-10-08) | v4.0.534; custom two-tier commercial license; React/browser rendering rather than native editor core. |
| MLT | `mltframework/mlt@77ae5f8f8cb4e2f502c5dc868d6c7cf7b45bfc54` (2026-10-06) | LGPL-2.1 framework; v7.42.0; component/ctest coverage and Linux/macOS/Windows CI. |
| Shotcut | `mltframework/shotcut@895ee392b6e0381353fbdc48b6889da9d3ad5c27` (2026-10-06) | GPL-3.0 desktop app; v26.9.27; active cross-platform releases and concrete fixes; built on MLT/FFmpeg. |
| Kdenlive | v26.08.2 (2026-09 release) | GPL-3.0-or-KDE accepted GPL; MLT/KDE desktop app; tests, fuzzing, UI automation, and multi-platform packaging. |
| libopenshot | `OpenShot/libopenshot@983c847ce7d0b7bfa4014d90bf86aa961a825f86` (2026-10-06) | LGPL-3.0 library; v1.0.1; C++/OpenCV/FFmpeg/audio dependencies and Linux/macOS/Windows CI; OpenShot application is GPL-3.0. |
| LosslessCut | `mifi/lossless-cut@70f2663a7a7c995903701acd2f616d057f4fdc2f` (2026-09-30) | GPL-2.0; v3.69.0; Electron; unit and desktop E2E coverage; stream-copy/remux trim focus. |
| Lookalike OpenMontage | `Open-Montage-app/OpenMontage` (2 commits in checked repo) | Identity is not the authentic AGPL project above; README advertises installers but the source/release provenance was not trusted or executed. |

The checked test and release evidence is based on repository workflows, test
trees, tagged releases, release metadata, and project documentation. It was not
treated as equivalent to an independent clean-machine application test; that
distinction is why the MLT replacement decision remains an experiment rather
than a conclusion.
