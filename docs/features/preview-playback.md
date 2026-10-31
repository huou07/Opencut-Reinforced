# Feature — Preview and Playback

Last reviewed: 2026-11-01

Scope: the viewer and transport — seek, scrub, play/pause, frame step, and the
timing contract they obey.

## Timing contract

- `ProjectTimeline.sequence_frame_rate` is optional and explicit: one exact
  rational rate in frames per second. No project and no active source supplies an
  implicit default.
- Play and frame step stay unavailable until a sequence rate is set through the
  validated project setting command.
- The timeline content end is half-open and includes audio-only duration. An
  empty timeline has no playable frames. Playback does not loop; pressing play
  with the playhead already at the content end replays from zero, the way a
  familiar player behaves.
- An optional project canvas (`width` × `height`, both even) fixes the
  composition shape and is what preview and export draw. Without one, the first
  video clip decides the shape. Setting or clearing the canvas is an undoable
  command (`timeline.sequence.set_canvas`), never a view-only preference.
- Playback speed is an exact rational applied to elapsed time: 0.25×, 0.5×,
  0.75×, 1×, 1.5× or 2×. Audio is mixed at 1×, so a speed other than 1× plays
  silently and says so in the menu instead of pitching or desyncing the sound.
  The choice is transport state and does not mutate the project.
- Seek and scrub preserve the requested exact rational time; display labels may
  convert to floating point for formatting only.
- Playback, seek, scrub, frame step, buffering and dropped frames never mutate the
  project or increment `ProjectRevision`.

## Transport surface

Fit / 25% / 50% / 100% / 200% zoom, exact seek, scrubbing on ruler and lanes,
play/pause, previous/next frame, ±5 s jump (`[` and `]`), 0.25×–2× speed,
timecode, and direct viewer manipulation (transform) where the inspector exposes
it. The viewer header carries the canvas menu, and full screen carries the same
transport plus a scrub bar, elapsed/total time and the canvas menu; its controls
fade while watching and return on pointer movement or any transport key.
`Space` plays or pauses, `←`/`→` seek ±1 s, `Shift+←`/`Shift+→` step a frame and
`Esc` exits full screen.

## Runtime behaviour

Decode and presentation are bounded and cancel-aware: a newer request replaces an
obsolete one, a backward request seeks, and an explicit seek starts a fresh
generation. Frames from a stale revision or generation are rejected. See
[architecture/realtime-plane.md](../architecture/realtime-plane.md).

## Evidence

The desktop path is exercised by the semantic UI harness (`scripts/or_ui.py`) and
by packaged-build journeys; release-mode pacing is recorded from the app itself
(`OR_PACING_LOG`) against the 16.68 ms budget. Debug-lane pacing is a measurement,
not a performance claim.

Measured 2026-10-29 (`playback-pacing`, Debug lane, 720p30 on the local Mac):
153 presented frames over 5.1 s — 29.9 fps, engine build p95 0.43 ms, frame span
p95 0.56 ms, gap p95 33.33 ms (vsync spacing) with a 50 ms worst gap. The build
and span stay two orders of magnitude under the budget, so pacing there is owned
by presentation, not by Flutter work.

Measured 2026-10-29 (`editor-frames`, Debug lane, 30 fps sequence): editor-open,
audio-panel and text-panel each presented 90-91 frames at 30.0 fps with UI
build p95 0.21-0.25 ms, frame span p95 0.32-0.37 ms and vsync gap p95 33.33 ms,
so the editor's own widget work stays under half a millisecond per frame.

## Known gaps

- Audio is silent at any speed other than 1× until the mixer can resample at
  arbitrary rates. The menu states this rather than shipping pitched audio.

No hardware decode path is approved; 1080p software decode is the current path.
Presented-frame (vsync/compositor) budgeting is measured only through the app's
own pacing log, not a dedicated gate.

## Related

- [editing.md](editing.md)
- [architecture/realtime-plane.md](../architecture/realtime-plane.md)
- [operations/local-macos.md](../operations/local-macos.md) — how to measure
