# OR Agent Workspace

This is the operating workspace the Opencut Reinforced assistant uses by default.
It is not the OR source checkout. The open project is recorded in `project.json`,
and the `or` CLI attached through that pointer is the default way to read or
change it. The `mcp__or__*` tools are a fallback for motion insertion, the motion
library, and a CLI failure whose error does not explain itself.

## Rule

Read the smallest entry that answers the current task. Do not preload the guides:
each line below names the situation that makes it the right thing to read.

## Routing

### Guides

- **Timeline editing** — tracks and their state, clips, markers, slip/roll/slide and sequence commands with exact times.
  When: the user asks to add, move, trim, slip, roll, slide, split, delete, reorder, re-time or mark something on the timeline.
  Path: `docs/timeline-editing.md`
- **Media and import** — media ids, probing, importing, paging, source relinking and removal.
  When: the task needs a file in the library before it can be placed, or needs an existing media id.
  Path: `docs/media-and-import.md`
- **Motion scenes** — code-to-video scenes, `.ormotion.json`, camera, the scene scaffold, validation, materialisation and the reusable asset library.
  When: the user asks for an illustration, motion graphic, animated title or scene from code, or a reusable scene asset.
  Path: `docs/motion-scenes.md`
- **Project safety** — revisions, save, undo/redo, rename, refused commands and the verbs that are not the agent's to call.
  When: deciding whether to save, undoing a change, or recovering from a failed command.
  Path: `docs/project-safety.md`
- **Script, narration and music** — an `or-assistant-script`, offline macOS
  voiceover, exact caption clips, generated background music, and the rule that
  cloud narration happens in the app.
  When: the user asks for a script, voiceover/dubbing, or background music.
  Path: `docs/script-and-voice.md`
- **Audio prep** — recording a take in the app, cleaning noise from a file, choosing a speech model and transcribing existing audio.
  When: the audio itself is the problem — too noisy, needs recording, or needs subtitles from what was said.
  Path: `docs/audio-prep.md`
- **Troubleshooting** — missing tools, no open project, refused scenes, missing motion library, failed transcription, slow or failed renders.
  When: a command fails and the error is not self-explanatory.
  Path: `docs/troubleshooting.md`

### Skills

The session catalog carries these. Load one only when its task matches.

- `or-timeline-edit` — orient, act, verify and save one timeline change, including
  track state, slip/roll/slide and markers.
- `or-motion-scene` — author, validate and materialise a deterministic scene, and
  place a reusable asset.
- `or-project-safety` — protect revisions, respect locked tracks, and report edits
  honestly.
- `or-script-production` — plan a script, narrate it offline, derive exact captions
  and add a generated music bed.

### Tools

- `tools/or` — the OR CLI bound to this app build and the default project surface.
  Run it from this workspace: `tools/or <verb> --attach project.json --json`.
  `project.json` is the app-written session pointer; the CLI follows its
  `descriptor_path`, so the same command line keeps working when another project
  opens. `tools/or commands` and `tools/or queries` print the exact verb surface.
- `tools/or motion publish` — validate, render through the app, import and insert a
  scene in one call. `project.json` records the app's motion library, so the call
  also saves the scene as a reusable asset; `--no-library` skips that on purpose.
  `tools/or motion asset validate|list|save|instantiate|insert|import|remove`
  reads and writes the same library: pass `--attach project.json` and the CLI
  resolves the folder from the pointer, while `--library PATH` overrides it for one
  call. The `mcp__or__motion_*` tools are a fallback for a CLI failure whose error
  does not explain itself; report the failure before switching.
- `tools/or media clean-audio --source IN --out OUT --json` — offline noise
  reduction that writes a new file; import it to use it. The app's media menu does
  the same in one step.
- `tools/or captions transcribe` / `tools/or timeline import-captions` — local
  multilingual transcription of existing audio, and applying reviewed cues as a
  caption track. Transcription needs the running app and the model the app has
  installed. This is the "subtitles from audio" path.
- `tools/motion-scaffold.mjs` — writes a valid V1 `.ormotion.json` skeleton with a
  camera move:
  `node tools/motion-scaffold.mjs --name "Title" --out scenes/title.ormotion.json`.
- `tools/voiceover.mjs` — validates an `or-assistant-script`, speaks it with the
  macOS voice, writes a timing manifest and SRT, and can import/place/caption the
  result with `--apply`:
  `node tools/voiceover.mjs --script work/scripts/demo.json --plan --json`.
- `tools/music.mjs` — synthesises a quiet, royalty-free background bed offline.
  A prompt is read the same way as the app's Generate music dialog; flags are the
  fallback and override it:
  `node tools/music.mjs --out work/music/bed.wav --prompt "nhạc nền nhẹ nhàng 30 giây, không trống" --json`.

## Working files

- `scenes/` — scene sources you author. `tools/or motion publish --scene` reads them.
- `work/` — scratch files, notes and copies. Nothing here is project state.
- `project.json`, `or-cli.json` and `tools/or` are written by the app; leave them
  alone. `project.json` names the live descriptor the CLI attaches through, the
  `motion_library_path` the motion verbs resolve and the `app_support_path` other
  app preferences are read from.

## Bounds

- Never edit a `.orproj` file, the project directory, or any path listed in `project.json`.
- Project mutations go through `tools/or ... --attach project.json`; the app applies
  them and owns history. The motion MCP tools are a fallback write path, not the
  default.
- Never read, ask for or store a cloud provider key. Narration keys live in the
  macOS keychain and belong to the app's Settings → Narration panel.
- Playback speed, clip volume, title and caption styling, export, microphone
  recording, cloud voices and the speech-model picker are app-only: say which
  surface owns the request instead of inventing a command, and never fake one
  with a trim, a split or a duplicate clip.
- Make one requested change at a time, confirm it with a timeline read, then save
  unless the user asked to review first.
