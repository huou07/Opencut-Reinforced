# Feature — Text and Captions

Last reviewed: 2026-10-26

Scope: title/text clips, manual captions, subtitle interchange, and the font and
rendering rules they share with MotionScene.

## Text clips

Typed text clip content with formatting (family, size, weight, colour, alignment)
rendered by the shared `or_render` text path on top of `cosmic-text` with bundled
fonts. There is exactly one text rasteriser; MotionScene and captions reuse it
rather than adding a second text engine.

## Captions

- Manual caption clips on Caption tracks, with typed content and exact timing.
- SRT and WebVTT import/export through a bounded parser/serialiser
  (`subtitler`, SRT+VTT features only). Unsupported or malformed input produces
  explicit diagnostics; cue/style loss must be understood before relying on a
  round trip.
- Imported captions become normal editable clips, not a hidden overlay.

## Rules

- Text and captions never fake a media ID or source range.
- Font identity is deterministic and bundled; no hidden system-font lookup, no
  network font fetch.
- Caption text is untrusted input and is bounded on import.

## Deferred

ASS, styled subtitle round-trip guarantees, transcript-driven auto-captions as a
product workflow (local transcription is deferred by the owner), translation, and
dubbing.

## Where

- `crates/or_core/src/` — caption codec, text/caption clip content
- `crates/or_render/` — text shaping and rasterisation
- `apps/or_app/lib/` — text/caption panels and inspector

## Related

- [editing.md](editing.md)
- [motionscene.md](motionscene.md) — shares the text path
