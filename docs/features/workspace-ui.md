# Feature — Workspace and UI Shell

Last reviewed: 2026-10-29

Scope: the application shell, where controls live, and the interaction rules the
UI must keep. Visual language is in [design/design-language.md](../design/design-language.md);
non-regression invariants are in [design/ux-invariants.md](../design/ux-invariants.md).

## Shell slots

Stable regions: App Bar, Editor Tool Rail, Left Tool Panel, Viewer, Inspector,
Timeline Toolbar, Timeline, Task/Status Area, Command Palette, Dialog or Mobile
Sheet. New work integrates into an existing slot; a permanent new region needs
justification and review.

## Editor layout

- One bar per zone: project name and save state, undo/redo, project menu, export,
  command palette — primary actions right-aligned.
- Tabbed library (Media / Text / Audio / Motion) instead of a placeholder icon
  rail. No dead entries: every control opens a real panel.
- Centre viewer with transport, timecode and transform/crop affordances.
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
  **Generate music…** and **Speech model…** next to import/add-track. Each opens one dialog that owns the offline synthesis,
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
  enrol a recorded or chosen 3–8 s reference clip as a cloned voice. The
  dialog previews the chosen voice with one sample line (the first script line,
  or a short Vietnamese default) before a full run, for installed and local
  voices alike. Switching
  the source keeps the last voice chosen for each source, and a choice made in
  Settings becomes the next run's default. The same
  Settings panel holds the cloud provider cards (key save/test/remove, Azure
  region, ElevenLabs personal-voice creation from a recorded WAV sample); keys
  live in the macOS keychain, not in this app's files.
- Status/snackbar feedback goes through the shared snackbar so shell and editor
  cannot diverge.

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
- Native macOS menu bar mirrors the same verbs (File/Edit/View/OR).

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

## Related

- [editing.md](editing.md)
- [design/design-language.md](../design/design-language.md)
- [operations/ui-harness.md](../operations/ui-harness.md)
