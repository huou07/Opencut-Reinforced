# Technology Review — November 2026

Last reviewed: 2026-11-09

Scope: the adoption review of twenty named repositories plus the alternatives
that were checked against them. It records what each one is, its licence, its
verdict for OR and why. Adoption rules live in
[security-licensing.md](security-licensing.md); this page keeps the dated
evidence behind "we looked, and here is what we decided".

Versions, release dates and licence texts were read from upstream sources on
2026-11-05: GitHub repository `LICENSE` files through the GitHub API, published
release metadata, npm registry metadata (`license`, unpacked size) and the
vendors' own licence pages. Sizes are npm unpacked size unless marked otherwise.

## Verdict vocabulary

| Verdict | Meaning |
|---|---|
| Shared dependency | Runs in-process today, shipping in every build. |
| Optional runtime | Could ship behind a capability check or an install, not adopted now. |
| Isolated sidecar | Only acceptable as a separate optional process with a materialisation boundary. |
| Agent skill | Usable as agent instructions, not as product code. |
| Format conversion | A build-time or import-time converter, never a runtime. |
| Do not use | Excluded for licence, duplication, or no path to OR's native stacks. |

## Outcome

No new runtime dependency entered the product from this list. OR keeps its
Rust-owned media and render path: FFmpeg for decode/encode, wgpu for compositing,
cosmic-text for shaping and rasterisation, and its own declarative MotionScene
for motion. Three.js is the single candidate that survives as an optional
sidecar; it is not built in this review and needs its own ADR and boundary
before any code lands.

## The twenty repositories

### Agent skills

| Repository | Version | Licence | Verdict | Reason |
|---|---|---|---|---|
| `remotion-dev/skills` | none | Remotion License (source-available, not OSI) | Do not use | The skill text is part of Remotion's licensed material, which forbids copying or sublicensing derivatives; nothing MIT-compatible to adopt. |
| `motiondivision/ai-kit` | npm `motion-ai@14.2.0` | Repository has **no** licence file; only the npm package declares MIT; Motion+ material is paid | Agent skill | Spring-easing guidance is the only reusable part; the hosted MCP and audits are a paid service and stay out. |
| `greensock/gsap-skills` | none | MIT (skill text only; GSAP itself is separately licensed) | Agent skill | MIT text can inform an OR skill; the GSAP library it describes is not MIT and is not adopted. |

### Design

| Repository | Version | Licence | Verdict | Reason |
|---|---|---|---|---|
| `svg/svgo` | 4.1.0 (2026-08-24) | MIT | Do not use | Node-only optimiser; OR has no JavaScript runtime and no SVG pipeline to feed. |
| `vercel/satori` | 0.63.2 | MPL-2.0 | Do not use | Node-only, 26 MB with the HarfBuzz WASM build, and it emits SVG which OR cannot rasterise. |

### Animation

| Repository | Version | Licence | Verdict | Reason |
|---|---|---|---|---|
| `greensock/GSAP` | 3.15.1 | Custom "no charge" GSAP licence (not OSI) | Do not use | Forbids use in tools that let users build visual animations without code and ends no-charge use when end users are charged; incompatible with an MIT editor. |
| `juliangarnier/anime` | 4.5.0 (2026-06-22) | MIT | Do not use | Needs a JavaScript runtime; MotionScene already expresses the same keyframe model declaratively. |
| `motiondivision/motion` | 14.9.1 | MIT | Do not use | Browser animation library; no native path, and OR's animation is deterministic and seekable by design. |
| `rive-app/rive-wasm` | 2.49.3 | MIT runtime, proprietary editor | Do not use | Browser WASM; native playback would need the C++ runtime and a second asset pipeline for no determined gain. |
| `LottieFiles/dotlottie-web` | 0.82.1 | MIT | Do not use | Browser/Node WASM player. The adoptable piece is the MIT Rust `dotlottie-rs` core if Lottie import ever becomes scope. |

### Video

| Repository | Version | Licence | Verdict | Reason |
|---|---|---|---|---|
| `remotion-dev/remotion` | 4.0.540 (2026-11-02) | Remotion License (source-available) | Do not use | Free only for individuals, non-profits and organisations of three people or fewer; a video editor is named as an automation product needing a paid company licence. It also needs Node, a per-OS Rust compositor (18–30 MB) and a Chromium shell (~100 MB+). |
| `motion-canvas/motion-canvas` | 3.17.2 (2024-12-14) | MIT | Do not use | No release in roughly two years, no official headless API, and it duplicates MotionScene with weaker determinism; its FFmpeg exporter ships prebuilt GPL-build binaries. |

### 3D

| Repository | Version | Licence | Verdict | Reason |
|---|---|---|---|---|
| `mrdoob/three.js` | r187 / 0.187.0 (2026-10-30) | MIT | Isolated sidecar (candidate) | MIT and healthy, but frames need a live WebGL2/WebGPU context, so only an optional Chromium sidecar is defensible. GPU and driver variance rule it out of the deterministic MotionScene path. |
| `pmndrs/react-three-fiber` | 9.8.2 | MIT | Do not use | React 19 plus a DOM only to wrap three.js; buys nothing without a JavaScript runtime. |
| `pmndrs/drei` | 10.7.9 | MIT wrapper; Apache-2.0 `hls.js` and `@mediapipe/tasks-vision` inside | Do not use | Inherits the browser constraint and fetches remote HDRIs and fonts at runtime, which breaks offline determinism and adds notices. |
| `pmndrs/react-three-rapier` | 2.2.0 (2025-11-03) | MIT repo; Apache-2.0 WASM core | Do not use | Physics belongs in the Rust core (`rapier3d`); the React wrapper needs r3f and a DOM. |
| `pmndrs/react-postprocessing` | 3.2.2 | MIT wrapper over Zlib `postprocessing` | Do not use | Effect stacks belong in OR's wgpu compositor; a browser effect composer adds a second pipeline. |

### Audio and export

| Repository | Version | Licence | Verdict | Reason |
|---|---|---|---|---|
| `goldfire/howler.js` | 2.2.4 (2023-09-19) | MIT | Do not use | Maintenance-only Web Audio playback wrapper; OR already decodes and plays in Rust (`cpal` plus FFmpeg). |
| `Vanilagy/mediabunny` | 1.65.2 (2026-11-04) | MPL-2.0 (file-level copyleft; bundle-safe in an MIT app) | Do not use | Browser-first WebCodecs muxer; outside a browser it needs `@mediabunny/server` and a Node host. No Rust or Dart path. |
| `FFmpeg/FFmpeg` | 8.1.3 pinned (2026-09-21); 9.0.2 is current major | LGPL-2.1-or-later on macOS and Windows; GPL-2.0-or-later on Linux, which links x264 (ADR 0011) | Shared dependency (already adopted) | The media runtime for every platform. Only the build profile is a decision, and it is now one script per platform. |

## Alternatives checked

| Candidate | Licence | Verdict | Reason |
|---|---|---|---|
| `resvg` 0.49.0 | Apache-2.0 OR MIT | Optional runtime | The licence-clean SVG rasteriser if vector import becomes scope; it adds a second rasteriser beside wgpu. |
| `harfrust` 0.15.0 | MIT | Not needed | Its predecessor `rustybuzz` is archived and redirects here; cosmic-text already shapes text in-process. |
| `rav1e` 0.8.1, `ravif` 0.13.0 | BSD-2-Clause, BSD-3-Clause | Optional runtime | Licence-clean AV1 encoders, but FFmpeg already wraps rav1e; owning an encoder separately is not justified yet. |
| libaom 3.15.3, SVT-AV1 4.3.0, libvpx 1.17.0, Opus 1.6.1 | BSD-3-Clause (plus the royalty-free AOM patent grant) | Shared dependency | Already the licence-clean codecs inside the FFmpeg build; no new decision needed. |
| x265 4.2 | GPL-2.0, or commercial from MulticoreWare | Do not use | GPL would relicense the whole FFmpeg build; commercial terms are a recorded-decision purchase, not a default. |
| Kvazaar 2.3.2 | BSD-3-Clause | Optional runtime | The only permissive software HEVC encoder and absent from FFmpeg's GPL list: the GPL-free route to HEVC delivery if that becomes scope. |
| Rubber Band 4.0.0 | GPL-2.0-or-later, or commercial from Breakfast Quay | Do not use | It sits in FFmpeg's `EXTERNAL_LIBRARY_GPL_LIST`, so enabling it relicenses every build; the LGPL `atempo` filter covers tempo today. |
| Symphonia 0.6.1 | MPL-2.0 | Do not use | Redundant with FFmpeg and has no Opus decoder and no encoders. |
| `image-rs` 0.25.10, `zune-jpeg` 0.5.15 | MIT OR Apache-2.0 (Zlib for zune) | Optional runtime | Stills only; FFmpeg plus the existing `png` crate already cover OR's image needs. |
| FreeType 2.15.0 | FTL or GPL-2.0 | Do not use | cosmic-text and swash already rasterise the bundled fonts; FreeType would add a C toolchain. |
| `png` 0.18.1 | MIT OR Apache-2.0 | Shared dependency (upgrade candidate) | Already a direct dependency at 0.17.16; the upgrade is routine, not an adoption. |

## Decisions recorded here

1. No animation, design, 3D or video-generation library is added to the Rust or
   Flutter process. The product's expressiveness comes from OR's own declarative
   scene format, not from a second renderer.
2. GSAP, Remotion and Remotion's skills are excluded on licence, not on quality.
   A no-charge GSAP licence that ends when users are charged, and a Remotion
   licence that charges per render for editor-class products, cannot be the base
   of an MIT editor.
3. Three.js is not adopted now. If it is ever adopted it is an optional sidecar
   with a bounded resource envelope, a pinned Chromium build, offline assets and
   a materialisation step into MotionScene or a video clip; it may not become a
   third project state.
4. HEVC export, if it becomes scope, uses Kvazaar through FFmpeg's `libkvazaar`
   (BSD-3-Clause) rather than x265, so no platform has to relicense its FFmpeg
   build. That is a future ADR, not an implementation.
5. FFmpeg stays pinned at 8.1.3 while 9.0.2 is current. 8.1 is still maintained,
   and moving majors needs its own compatibility run. The static-link LGPL §6
   obligation is the live licensing exposure, not the GPL question.

## Re-check, 2026-11-06

Re-read on the same day the render and audio work landed. No version in the
twenty changed a verdict, so nothing was added as a runtime dependency. Three
things did change in the product, and they are in-house rather than from this
list:

- Text-over-video compositing no longer leaves the GPU path. The NV12 layer
  pipeline, the pooled compositor and the keyed text upload are OR code
  (`crates/or_render`), because no library in this review offers compositing on
  OR's wgpu stack.
- Reduce Noise is a peak-keyed gate in `or_audio`, not a port of a JavaScript
  audio library. Howler.js remains a browser playback wrapper; Web Audio in a
  Flutter desktop app would add a second audio clock.
- Linux MP4 export now has one build recipe (`scripts/build_linux_ffmpeg.sh`)
  instead of an inline CI body. That is the licence-bearing change in this
  review: the Linux binary is GPL because of x264, while macOS and Windows stay
  LGPL with their OS encoders. Remotion, GSAP and Remotion's skills stay
  excluded on licence grounds; Three.js stays an unbuilt sidecar candidate;
  Mediabunny, Satori, SVGO, Anime.js, Motion, Rive, dotLottie, Motion Canvas,
  R3F, Drei, Rapier and postprocessing stay out for the reasons in the tables
  above — no JavaScript runtime, or a second renderer beside wgpu, or a licence
  that an MIT editor cannot carry.

Video-embedded audio is no longer a gap: a video clip whose media has an audio
stream now contributes that stream to the mixer and carries the same audio
settings (gain, pan, fades, noise reduction) as an audio clip, so "reduce noise
on the audio of a video clip" is answered by an in-house path rather than a
dependency. Per-stream selection for a file with several audio streams stays
deferred.

## What the twenty produced, 2026-11-08

The review asked for a verdict per repository; this asks what each verdict
actually changed in the product. A "do not use" that never touches a feature is
a decision, not an outcome, so every row below names code that exists or the
concrete route a deferred idea would take.

| Idea taken from | What shipped | Where |
|---|---|---|
| GSAP, Anime.js and Motion agree that a curve vocabulary is most of what makes motion feel authored, not the library runtime | MotionScene can now ask for `ease_out_back` and `ease_out_bounce`, the two overshoot curves a clip keyframe already had, so the same motion on a clip and in a composition finally matches. Both are closed-form, so a scene stays deterministic and seekable. | `crates/or_motion/src/lib.rs` (`MotionEasing`, `ease`), commit `9135beba` |
| Remotion and Motion Canvas both treat a cut as data built from a composition, never as a baked file | An edit style turns the project's own media into a reviewed plan and then into real clips: one timeline insert per shot plus a title, each its own undo step, with the plan shown — shot order, lengths and why — before anything is written. Nothing is rendered and source media is never touched. | `apps/or_app/lib/library/edit_styles.dart`, `apps/or_app/lib/screens/edit_style_plan_dialog.dart`, `apps/or_app/lib/shell/app_shell.dart`, commit `562e74c7` |
| Satori's core insight is that a style is structured data (type, colour, layout, captions) rather than an image | Creative directions carry exactly those axes as separate fields and print them as a brief an agent can consume; the renderer stays cosmic-text and wgpu instead of a second rasteriser. A kept copy can be edited section by section. | `apps/or_app/lib/library/creative_directions.dart`, `apps/or_app/lib/screens/creative_direction_editor_dialog.dart`, commit `886af845` |
| The agent-skill repositories show that the reusable artifact is instructions, not code | OR's own skills live in the assistant workspace (`tooling/dsh-or-skills`), and the assistant reads the same creative directions the Library shows. GSAP's MIT skill text remains the only text from that group an OR skill could build on; ai-kit is unusable because its repository carries no licence file, and Remotion's skill text is licensed material. | `tooling/dsh-or-skills/skills`, `AGENTS.md` routing |
| Three.js, Rive and dotLottie describe animated scenes richer than a keyframe graph | Lottie and glTF are now usable without moving a runtime into the core: both render through a pinned browser sidecar (`scripts/lottie-render` with puppeteer + lottie-web, `scripts/gltf-render` with three.js 0.137), write a video, and record their provenance so the library shows `From Lottie renderer` / `From glTF renderer`. The alpha-preserving python-lottie route ships beside them. Rive is still deferred: the C++ runtime would have to be owned. No renderer entered `or_core`. | `scripts/lottie-render/render.js`, `scripts/gltf-render/render.js`, `crates/or_core/src/external_tools.rs`, journeys `companion-lottie.json`, `companion-gltf.json` |
| SVGO and resvg describe vector work OR cannot do yet | Deferred with the licence-clean route named: `resvg` (Apache-2.0 OR MIT) if SVG import lands, with `svgtypes`-style normalisation owned in Rust rather than a Node optimiser. | "Alternatives checked" above |
| Mediabunny and Howler.js describe muxing and playback outside Rust | Not adopted; FFmpeg muxes and `cpal` plays. Nothing in the media path moved. | `crates/or_media`, `crates/or_audio` |

What this does **not** claim: no third-party animation, 3D, vector or Lottie
runtime entered `or_core`; the Lottie and glTF routes are sidecars that shell out
to Node and a browser, exactly as decision 2 allows, and Rive, resvg and
Mediabunny remain deferred without a prototype. The renderer rows above are the
product changes; the remaining rows are the honest state of the deferrals, with
the evidence for each still in the tables above.

The sidecar route was chosen over embedding for three reasons that the prototype
tested: the licence is MIT on both sides and no new Rust dependency entered the
tree, the maintenance cost is two scripts pinned to a Node version instead of a
copied-in renderer, and it keeps the editability promise — the source project
(the `.json` animation or the `.gltf` model) stays beside the rendered clip and
is named in the provenance record, so a user can reopen it in the tool that made
it. What it costs is honest: a Chromium download (about 300 MB with the Lottie
renderer) and a Node dependency, both behind an explicit install step that
Settings → External tools prints.
