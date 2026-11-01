# Feature — Text and Captions

Last reviewed: 2026-11-02

Scope: title/text clips, manual captions, subtitle interchange, and the font and
rendering rules they share with MotionScene.

## Text clips

Typed text clip content with formatting (family, size, weight, colour, alignment)
rendered by the shared `or_render` text path on top of `cosmic-text` with bundled
fonts. There is exactly one text rasteriser; MotionScene and captions reuse it
rather than adding a second text engine.

Formatting also carries placement and legibility fields, all optional and
defaulted so every project written before them renders unchanged:

- `font` — Inter (the default, so old projects stay identical) or Be Vietnam Pro,
  a display face designed for Vietnamese. Both are bundled OFL 1.1 assets.
- `anchor` — top, centre or bottom of the frame; new caption clips start at the
  bottom, text clips at the centre.
- `offset_x_basis_points` / `offset_y_basis_points` — a nudge from the anchor in
  basis points of the canvas, so a caption can sit anywhere without a new schema.
- `background` — an optional filled box behind the shaped glyph bounds, with a
  colour (including alpha) and padding in milli-points.
- `opacity_basis_points` — whole-block opacity, independent of the clip's visual
  opacity.

The Edit Title/Caption dialog writes all of these through the existing
`timeline.clip.update` command; the renderer measures the shaped text, so the
box follows real advances rather than a character-count estimate. The dialog
offers a full colour picker for text and for the background box (alpha included;
a cleared background is transparent), and the viewer draws a selected title or
caption as a direct-manipulation box: drag the interior to move it, drag the
corner handle to resize the font, and release to commit one ordinary clip
update. The same fields are available without the GUI through
`or timeline set-text` (`--color`, `--background`, `--background-padding`,
`--size-pt`, `--font`, `--anchor`, `--offset-x`, `--offset-y`, `--opacity`).

## Captions

- Manual caption clips on Caption tracks, with typed content and exact timing.
- SRT and WebVTT import/export through a bounded parser/serialiser
  (`subtitler`, SRT+VTT features only). Unsupported or malformed input produces
  explicit diagnostics; cue/style loss must be understood before relying on a
  round trip.
- Imported captions become normal editable clips, not a hidden overlay.
- **Caption style** is per clip: position, size, weight, alignment, colour, text
  opacity and an optional background box travel with the clip, so a reviewed
  transcription can be restyled without recreating it.
- **Local transcription** turns one media item's audio into a bounded transcript
  proposal: the app decodes a managed 16 kHz mono WAV, runs the pinned
  whisper.cpp runtime with the downloaded multilingual model, and detects
  Vietnamese or English automatically. The Flutter surface is **Generate
  Captions** on a media item; the same host service is exposed to the CLI and
  agents as `or captions transcribe`. Both paths require review before apply and
  apply through the ordinary `timeline.captions.import` command, so an accepted
  proposal is an editable caption track, never a hidden layer.
- **Subtitle delivery** exports the caption clips themselves: `or captions
  export --out FILE.srt|FILE.vtt` walks every caption track in start order and
  writes SRT or WebVTT with the shared encoder. Nothing is burned into the
  picture, and an empty project reports `CAPTION_TRACK_EMPTY`.
- Model weights are not bundled. Five multilingual checkpoints are selectable —
  Base (141 MiB), Small (465 MiB), Medium (1.43 GiB), Large v3 Turbo q5
  (547 MiB) and Large v3 Turbo (1.51 GiB). One is downloaded at a time only
  after explicit consent, verified by size and SHA-256, and stored in the app's
  private model folder; the licence and source are shown before download. A
  refused source, a truncated transfer or an unreachable network states that
  reason instead of a generic failure. The
  chosen model is persisted app-side and is the one both the app and
  `or captions transcribe` use. A picker changes it; only the installed models
  need disk space. Listing a model checks its presence and size, and the
  transcription job still verifies the full digest before running, so a large
  model does not have to be hashed on every dialog open. The picker shows the
  total installed size and can remove one downloaded model to free space; the
  service falls back to the catalog default when the active model is removed.
- A plain local debug build does not ship the whisper runtime. `scripts/
  install_local_transcription_helpers.sh` copies a pinned `whisper-cli` and the
  rebuilt provider into the app bundle, re-signs it ad hoc, and can install a
  verified small model; packaged macOS builds include both binaries, and the
  packaging script refuses an image that asks for them without them.
- The picker says when a build has no local speech runtime and disables **Use
  model** instead of offering a download that cannot run. With the runtime
  present, every not-installed row carries a **Download** action; the dialog
  still states the size, licence and source before transferring anything.

## Rules

- Text and captions never fake a media ID or source range.
- Font identity is deterministic and bundled; no hidden system-font lookup, no
  network font fetch.
- Caption text is untrusted input and is bounded on import.

## Deferred

ASS, styled subtitle round-trip guarantees, translation, dubbing, speaker
diarisation, and word-level karaoke timing.

## Where

- `crates/or_core/src/` — caption codec, text/caption clip content
- `crates/or_render/` — text shaping and rasterisation
- `crates/or_ai/` — model manifest, validation and transcript proposals
- `crates/or_whisper_provider/` — the out-of-process local whisper adapter
- `crates/or_app_bridge/src/transcription.rs` — decode, job and review service
- `apps/or_app/lib/` — text/caption panels, inspector and the Generate Captions action

## Related

- [editing.md](editing.md)
- [motionscene.md](motionscene.md) — shares the text path
