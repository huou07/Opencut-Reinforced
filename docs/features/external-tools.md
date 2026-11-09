# External Tools and Companion Programs

Last reviewed: 2026-11-09

Scope: how OR detects, explains, launches and records the external programs a
project can use, and what a result has to satisfy before it counts as part of a
project. Install policy and licence posture are in
[security-licensing.md](../reference/security-licensing.md); packaging is in
[platforms.md](../architecture/platforms.md).

## Principle

An external tool stays outside OR. OR calls its CLI, opens its companion window
or exchanges a file with it; it does not copy the tool's source, runtime or
renderer into the core. The value OR adds is the integration: one place that
answers what the tool is for, whether this machine can run it, what it costs in
space and licence, how to install it, how to remove it, and how its output
becomes an editable part of a project.

## Catalog

`or_core::external_tools` is the single catalog. It is data plus pure policy:
no process is spawned there, so the app, the CLI and an agent answer with the
same facts. Each entry carries:

- identity: `id`, `name`, `category`, `summary`, `benefit`;
- cost: `license`, `license_note`, `size_hint`, `runtime`;
- install: `homepage`, `install_page`, one `install_command` per platform with
  a note, and `update_hint`;
- invocation: `probe` (a version flag), optional `gui_program`, `outputs`;
- scope: `project_scoped`, `platforms`, `removable`, `remove_hint`.

The catalog is curated, not a filesystem scan. Opening Settings runs one short
version probe per supported tool; nothing else is touched.

## States

| State | Meaning |
|---|---|
| `ready` | The probe ran, exited zero and printed a version. OR may launch it. |
| `missing` | The probe could not start the program. |
| `failed` | The program started but did not answer a version. |
| `unsupported` | The catalog knows it, but not on this platform. |
| `project_scoped` | It installs into a project; a machine-wide probe cannot answer. |

A launch requires `ready`. A broken install reports why instead of flashing a
window and dying.

## Surfaces

- **Settings → Resources → Companion tools** lists every entry with its state,
  version, licence, size, runtime, outputs, the exact install command, the
  official page and the update/removal steps. Expanding a row is read-only;
  nothing downloads or installs from the screen. `Check all` re-probes.
- **`or tools [list|check <id>|launch <id>|run <id> [-- args...]] [--json]`**
  prints the same states, opens a companion window, or runs a ready tool's CLI
  with the arguments given. `or tools check <id>` exits non-zero for an unknown
  id, not for a missing tool; `or tools run` refuses an id outside the catalog
  and a tool that is not ready, so a typo never spawns something else. Arguments
  are passed to the process, never through a shell.
- **`or mcp`** exposes `tools_list` and `tools_check`, so an agent can ask which
  companion tools are installed before promising a workflow.
- **`or_tools`** is the only crate that spawns a tool. Probes are bounded
  (`PROBE_TIMEOUT`, 4 s); a run is bounded by `RUN_TIMEOUT` (600 s) and a tool
  that overruns is killed and reported.

## What counts as a result

A result is usable when it can be brought into the project without pretending
it is more than it is:

- video, image or audio → an ordinary media item, inserted as a clip;
- subtitle → an ordinary caption import;
- MotionScene → the native scene path;
- an image sequence or a scene that only a companion can render → the rendered
  media plus the source project, manifest and the command that produced it.
  `or tools provenance write --tool ID --output PATH [--source-project P]
  [--manifest M] -- CMD...` writes `<PATH>.orprovenance.json` beside the media;
  `or tools provenance read --file SIDECAR` prints it again. Both verbs answer
  `--json`; `write` returns `{"file": SIDECAR, "record": …}` so a caller learns
  what it recorded without re-reading the sidecar. The same two operations are
  MCP tools (`tools_provenance_write`, `tools_provenance_read`)
  so an agent records the origin as part of the job it just ran. The record carries
  the catalog id and name, the version, the exact argument vector, the source
  project and manifest, and the output path. Re-rendering the tool to the same
  output path replaces the bytes the timeline already points at, so clips,
  undo history and project references stay valid; a new path is a new media
  item, not a silent swap. The media library reads the sidecar when it lists an
  item and shows `From <tool> <version>` in the row, so a rendered result is
  recognisable without opening anything else; `Where This Came From…` in the
  item's menu opens the whole record — source project, manifest, result file,
  command and the sidecar's path — and `Copy command` puts the argument vector
  on the clipboard so the render can be repeated.

### The two rendered routes that exist today

Both are sidecars: OR runs a tool, takes the media, and keeps the source file
beside it. Neither adds a renderer or a runtime to the core.

- **Lottie** — catalog id `lottie`, MIT (`puppeteer-lottie`, `lottie-web`).
  `scripts/lottie-render` installs the npm package and draws the animation in a
  browser, and `scripts/make_lottie_fixture.sh` renders
  `scripts/lottie-render/or-badge.json` through it and records the result. The
  animation file is the source: it is what the record's `source_manifest`
  points at, so it can be reopened in the app that authored it and rendered
  again to the same output path. `scripts/ui_scenarios/companion-lottie.json`
  proves the round trip in the real app — import, `From Lottie`, the origin
  dialog with the animation and the command, insert into the timeline, and the
  origin still there afterwards.
- **glTF** — catalog id `gltf`, MIT (`three.js`). `scripts/gltf-render` renders
  a `.gltf` or `.glb` to a PNG sequence in a browser, `or tools run ffmpeg`
  turns the frames into a clip, and `scripts/make_gltf_fixture.sh` records both
  the model and the render. `scripts/ui_scenarios/companion-gltf.json` runs the
  same journey as the Lottie one.

The layer and keyframe structure is not imported in either route: what arrives
is a render plus the source, which is the agreement the catalog makes for every
companion. An image sequence is the alternative when a caller wants frames
rather than a clip, and `scripts/lottie_frames.py` is a python-lottie variant
that keeps the frames' alpha: `python3 scripts/lottie_frames.py
scripts/lottie-render/or-badge.json OUT --frames 90 --fps 30` writes PNGs with
`python-lottie` 0.7.2 and cairo (macOS needs
`DYLD_FALLBACK_LIBRARY_PATH=/opt/homebrew/lib`), and `or tools run ffmpeg`
turns them into a clip. Neither route imports layers or keyframes.

A flat render is never described as an editable composition. The source stays
beside the output so it can be reopened in the companion and re-rendered; the
project keeps the reference and the provenance.

## Boundaries

- No automatic download, install, media upload or permission grant. The screen
  shows the command and the official page; the person runs it.
- A companion that lives outside a Flatpak sandbox is named and documented; OR
  does not assume a host tool can be executed inside the sandbox.
- Project-scoped tools are checked in the project that contains them, never
  globally.

## The twenty candidates, judged as tools

The 2026-11-06 review asked whether each repository should become an
in-process dependency and answered no for almost all of them. That is not the
same question as "can a person use it from OR". This table asks the second
question: what job it does, how OR can reach it without owning its runtime, and
what the result can become. Licence and version facts stay in
[technology-review.md](../reference/technology-review.md); nothing below is
bundled with OR.

| Candidate | The job it does for a person | How OR can reach it | Result and editability | Verdict |
|---|---|---|---|---|
| Remotion | React-authored video templates, titles and data-driven scenes | `npx remotion render` inside the person's own project, then import the file | MP4/WebM/PNG sequence; the source project is kept beside the clip as provenance | Companion candidate (project-scoped); licence excludes bundling |
| Motion Canvas | Animated explainers from TypeScript scenes | `npx motion-canvas render` in the person's project, then import | Video or image sequence plus the source scene | Companion candidate (project-scoped, MIT) |
| GSAP | Timeline animation with easing and stagger | Its licence allows code in the person's project, not a bundled editor feature | Rendered through Remotion or another project; no direct asset format | Excluded from OR; licence forbids an editor-class product |
| GSAP skills | Easing and stagger craft as instructions | Agent skill text (MIT) | Informs a plan or MotionScene by hand | Agent skill |
| Anime.js | Keyframe and stagger animation in a browser | Only through an external project | Rendered video | Reference only; MotionScene already expresses the model |
| Motion (Framer) | Spring and gesture animation in React | Only through an external project | Rendered video | Reference only |
| Satori | JSX-to-SVG title and card layout | Node CLI/script in the person's project | SVG or PNG; OR can import the PNG as a still, not edit its layers | Companion candidate only after an SVG path; deferred |
| SVGO | Smaller, cleaner SVG files | `svgo` CLI on a chosen file | Optimised SVG; OR has no SVG importer today | Deferred on the same importer gap |
| Rive | Interactive vector motion with a state machine | Native runtime needed; editor is proprietary | `.riv` playback, not an editable OR object | Excluded |
| dotLottie | Lottie-format vector animation | `dotlottie-rs` could parse, but OR has no Lottie renderer | Would need a new asset pipeline | Deferred |
| Three.js | 3D scenes, camera moves, motion graphics | Headless Chromium sidecar renders frames; OR imports them | PNG frames or a rendered clip; the scene file is provenance | Companion candidate; own ADR and resource envelope required |
| React Three Fiber | React binding over Three.js | Only inside a browser project | Same as Three.js | Not standalone |
| Drei | Helpers and assets for R3F | Only inside a browser project; fetches remote assets | Same as Three.js, plus offline-determinism risk | Not standalone |
| react-three-rapier | Physics for R3F scenes | Rust `rapier3d` is the native path | — | Not adopted |
| react-postprocessing | Effect stacks for R3F | OR's wgpu compositor owns effects | — | Not adopted |
| Howler | Web Audio playback wrapper | OR already plays audio in Rust | — | Not adopted |
| Mediabunny | Browser muxing and demuxing | Needs a Node/browser host | — | Not adopted |
| FFmpeg | Decode, encode, mux, probe | Bundled runtime and `ffmpeg`/`ffprobe` beside the app | The media foundation of every clip and export | Adopted |

Two consequences follow. First, a companion that only renders video is still
useful: OR inserts it as an ordinary media clip and keeps the source project
path, so the person can reopen the companion, change it, render again and
replace the clip. Second, the importer is the gate: SVG, Lottie and glTF
results have no editable path until OR grows one, which is why those rows say
deferred rather than done.

## Gaps

- The provenance record for a rendered result and its "update from source"
  action are not implemented yet.
- Lottie/dotLottie and glTF have no importer; both need a renderer before their
  output can enter a timeline, and that decision is recorded in
  [technology-review.md](../reference/technology-review.md).
- There is no per-tool job log or cancellation yet; a launch reports its
  outcome and nothing more.

### Setting a tool up from the app

The panel states each tool's own command, and that same string is what the
**Set up here** button runs (`apps/or_app/lib/updates/companion_setup.dart`):
`/bin/sh -lc` on macOS and Linux so a Homebrew or npm install on `PATH` is
found, `cmd /c` on Windows because `npm` is a `.cmd` shim. The command's output
streams into the row, the exit code decides the message, and a successful run
re-checks the tool — the state shown afterwards is what the probe found, never
an assumption that the install worked.

Removal is offered only where it can be derived (`npm install -g <pkg>` →
`npm uninstall -g <pkg>`); every other tool keeps its written removal steps,
because a guessed uninstall is worse than an honest instruction. The same two
operations are on the command line as `or tools setup <id>` and
`or tools remove <id>` (`crates/or_tools/src/lib.rs` `run_setup`).
