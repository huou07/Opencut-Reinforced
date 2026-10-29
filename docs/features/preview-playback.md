# Feature — Preview and Playback

Last reviewed: 2026-10-29

Scope: the viewer and transport — seek, scrub, play/pause, frame step, and the
timing contract they obey.

## Timing contract

- `ProjectTimeline.sequence_frame_rate` is optional and explicit: one exact
  rational rate in frames per second. No project and no active source supplies an
  implicit default.
- Play and frame step stay unavailable until a sequence rate is set through the
  validated project setting command.
- The timeline content end is half-open and includes audio-only duration. An
  empty timeline has no playable frames. Playback does not loop.
- Seek and scrub preserve the requested exact rational time; display labels may
  convert to floating point for formatting only.
- Playback, seek, scrub, frame step, buffering and dropped frames never mutate the
  project or increment `ProjectRevision`.

## Transport surface

Fit / 25% / 50% / 100% / 200% zoom, exact seek, scrubbing on ruler and lanes,
play/pause, previous/next frame, timecode, and direct viewer manipulation
(transform) where the inspector exposes it.

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

## Known gaps

No hardware decode path is approved; 1080p software decode is the current path.
Presented-frame (vsync/compositor) budgeting is measured only through the app's
own pacing log, not a dedicated gate.

## Related

- [editing.md](editing.md)
- [architecture/realtime-plane.md](../architecture/realtime-plane.md)
- [operations/local-macos.md](../operations/local-macos.md) — how to measure
