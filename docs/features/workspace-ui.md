# Feature — Workspace and UI Shell

Last reviewed: 2026-11-05

Scope: the application shell, where controls live, and the interaction rules the
UI must keep. Visual language is in [design/design-language.md](../design/design-language.md);
non-regression invariants are in [design/ux-invariants.md](../design/ux-invariants.md).

## Shell slots

Stable regions: App Bar, Editor Tool Rail, Left Tool Panel, Viewer, Inspector,
Timeline Toolbar, Timeline, Task/Status Area, Command Palette, Dialog or Mobile
Sheet. New work integrates into an existing slot; a permanent new region needs
justification and review.

## Workspace destinations

Four project-independent destinations, in one rail on every desktop: **Home**,
**Projects**, **Library** and **Settings**; the editor replaces the workspace
once a project is open. Home is the first-run surface, Projects lists recent
work, Library is the creative catalogue, Settings owns machine configuration.

## First run

Home carries an explanation panel until the person either starts a project or
dismisses it: media stays where it is, a project is one `.orproj` file saved
atomically, and export runs locally with the hardware encoder when the machine
has one. The panel is three sentences and three actions (New Project, Open
Library, Read Guide) — no tour, no overlay, no modal. Dismissal is stored in
`welcome.json` beside the other preferences, and a preference file that cannot
be read shows the panel rather than hiding the only explanation a beginner gets.

## Library

`AppDestination.library` is the project-independent catalogue, so a template or
a sound can be browsed before a project exists. Four tabs:

- **Templates** — canvas and frame rate per template, with starter text overlays
  placed through the ordinary `timeline.insertText` command after the create
  action releases the shell's busy flag. A template never asks for a frame rate;
  its own rate is applied and the create dialog's rate row disappears.
- **Text** — seven text looks shown with their real font, size, weight,
  alignment and anchor, so the sample is the result rather than a promise.
- **Sound** — generated sound effects (tick, click, pop, whoosh, riser, impact,
  chime) written as real 16-bit PCM WAV into the app support folder, then
  imported through the ordinary media command; the music generator stays one
  action away.
- **Guide** — the in-app manual: start, import, edit, text, audio, export,
  shortcuts, files and troubleshooting. Shortcut rows render `Cmd` on macOS and
  `Ctrl` elsewhere from the platform seam, and the list matches the shell's real
  shortcut map.

An action that needs a project is visibly unavailable with its reason, never a
silent no-op: adding a text look or a sound without an open project states what
is missing.

## Editor layout

- One bar per zone: project name and save state, undo/redo, project menu, export,
  command palette — primary actions right-aligned.
- Tabbed library (Media / Text / Audio / Motion) instead of a placeholder icon
  rail. No dead entries: every control opens a real panel.
- Centre viewer with transport, timecode, a canvas menu (landscape, portrait and
  square presets, or match the first clip) and transform/crop affordances. Full
  screen repeats the transport with pause, ±5 s, scrub bar, elapsed/total time and
  speed. The export dialog offers profile, quality and resolution in one place.
- Right rail with the **Assistant** and an optional advanced **Inspector** tab.
  The assistant is the default surface; the inspector keeps human units, presets
  and a sticky action row, and is hidden until **Settings → General → Show
  inspector panel** or the palette's **Show Inspector Panel** reveals it. Both
  reuse the same slot over one project state instead of adding a region.
- Bottom timeline with thumbnail/waveform clips, visible trim handles, context
  menu, markers, snapping, and one control row (zoom, Fit, Snap, create menu).
  Selecting a media clip adds its speed control to that strip, and an audio clip
  adds a volume (gain) control; both write the ordinary typed commands and keep
  their values in the advanced inspector.
- The Audio tool exposes **Record voice…**, **Generate voiceover…**,
  **Generate music…** and **Speech model…** next to import/add-track; the speech
  picker states when the build has no local runtime, disables **Use model** then,
  and offers a **Download** action on each not-installed row when it does. Each opens one dialog that owns the offline synthesis,
  import, placement and (for music) optional ducking, then refreshes through the
  normal project reads. The music dialog leads with a **Describe the music**
  field: one sentence in Vietnamese or English is read for mood, bpm, length and
  drum intent, the interpreted values replace the fine-tune controls and a
  readback line shows exactly what will be generated; one-tap examples start a
  common request, and **Variation** reseeds the bed without changing the read.
  **Record voice…** will not close while a take runs — Cancel, Escape and the
  barrier all say why — and tearing the dialog down stops the capture instead of
  leaving the microphone open.
  Generation is a bounded batch of ordinary commands, not
  a second project model. The voice a narration run uses is chosen in
  **Settings → Narration**, where it can be searched and previewed at the
  configured rate, with the sample spoken in the voice's own language. The Generate
  voiceover dialog also picks the source: installed voices, a cloud provider, or
  a **Local server** with voice cloning; a local server needs no key and can
  enrol a recorded or chosen 3–8 s reference clip as a cloned voice after a
  rights acknowledgement. **Settings → Narration** keeps governing the
  installed-voice fallback and says so when the active source is a local server
  or a cloud provider. The dialog previews the chosen voice with one sample line (the first script line,
  or a short Vietnamese default) before a full run, for installed and local
  voices alike. Switching
  the source keeps the last voice chosen for each source, and a choice made in
  Settings becomes the next run's default. The same
  Settings panel holds the cloud provider cards (key save/test/remove, Azure
  region, ElevenLabs personal-voice creation from a recorded WAV sample); keys
  live in the macOS keychain, not in this app's files.
- **Settings → Resources & updates** (also a Home quick action) is the
  clean-machine surface: it checks the packaged FFmpeg, speech runtime and `or`
  CLI, links to the assistant harness, opens the speech-model manager when a
  project is open, and checks the public GitHub release. A newer DMG downloads to
  Downloads and opens for the user to drag into Applications; the app never
  replaces itself silently. Optional model and harness downloads stay explicit.
- Status/snackbar feedback goes through the shared snackbar so shell and editor
  cannot diverge.

## Guided mode

Guided mode is the beginner surface and it is a preference, not a second
editor. It is stored in `editor-settings.json` beside the inspector preference
and changes only what is shown, never what a command does or what a project
file holds. A version-1 settings file opens in guided mode.

- **Next-step row.** Above the timeline, one row names the next useful action
  and offers it as a single button: import media, add the first clip, add a
  title, add sound, then export. The row is derived from the open project's
  read model (media count, clip count, whether a text or audio clip exists, and
  whether this session has exported), so it disappears on its own as the work
  is done instead of tracking a tutorial. It can be dismissed on its own; that
  is a separate preference from guided mode, and Settings owns the switch that
  brings it back.
- **Export by intent.** The same export command and the same three output
  profiles back a dialog that leads with what the file is for — share or
  upload, smaller file, highest quality, keep every pixel — and states the real
  container and limitation under each choice. A choice maps onto the existing
  profile, quality and resolution fields; **Advanced settings** reveals those
  fields unchanged. Nothing about the job, staging or validation path differs.
- **Composition guides.** Title-safe (10%), action-safe (5%), thirds and centre
  draw over the preview only, for the program frame and for fullscreen. They
  are painted in Flutter after presentation, never handed to a renderer, and
  never appear in an exported frame. `G` toggles them when no text field has
  focus and the viewer guide button toggles the same preference.
- **Settings** carries the three switches under Appearance: guided mode,
  next-step tips, composition guides. Each writes the shared store, so the
  shell, the editor and the next launch agree.

## Interaction rules

- Keyboard and pointer have parity: every transport, timeline and edit verb is
  reachable by key, menu, palette and pointer where applicable. `Cmd/Ctrl+Shift+A`
  opens the assistant rail, and the palette exposes both rail tabs.
- Shortcuts resolve through the single shell `Shortcuts`/`Actions` map and do not
  fire while a text field has focus.
- One gesture dispatches one command; no optimistic edit geometry in Flutter.
- Simple and Advanced modes are visibility preferences over the same state and
  commands. Hiding a control never forks or removes project data.
- Mobile/compact layout exposes the same semantics as the desktop rail, with
  selection carried by fill and border, not colour alone.
- The menu bar renders from one description of the verbs: macOS draws the
  native bar, Windows and Linux draw an in-window bar in the same shell. Both
  disable the same entries under the same conditions, so a verb added once
  appears on every platform.

## Rules

- Flutter keeps presentation state only: navigation, panel layout, selection,
  input, bounded read-model caches. Never a second editable project model.
- New features use a lightweight static descriptor (id, label, icon, group,
  availability, command IDs, panel, inspector sections, shortcut metadata); do
  not build a plugin framework for it.
- Every visible action must work or be visibly unavailable — no silent dead ends.
- A voiceover batch is several validated commands (one per imported line and one
  caption import), so it is several undo steps; the notice says what was added.
  Music ducking is one `timeline.clip.duck` command and therefore one undo step.

## Where

- `apps/or_app/lib/` — shell, panels, inspector, timeline widgets, palette, menus
- `apps/or_app/test/` — widget guards for the invariants above

## Where — library data

- `apps/or_app/lib/library/` — `project_templates.dart`, `text_presets.dart`,
  `sound_library.dart` (deterministic WAV synthesis), `usage_guide.dart`
- `apps/or_app/lib/screens/library_screen.dart` — the four tabs and their
  unavailable states
- `apps/or_app/lib/production/welcome_store.dart` — the first-run preference

## Related

- [editing.md](editing.md)
- [design/design-language.md](../design/design-language.md)
- [operations/ui-harness.md](../operations/ui-harness.md)
