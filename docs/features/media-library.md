# Feature — Media Library and Import

Last reviewed: 2026-11-01

Scope: bringing media into a project, showing it, replacing it, and telling the
user precisely what is wrong when it cannot be used.

## Import

- Desktop: multiple selected local files and bounded recursive folders.
- Android: persistable SAF tree/document grants, bounded by depth and entry
  count, with an app-private working copy.
- Each successful source enters canonical state only through `media.add`.
- Sources are probed independently: a failed item leaves project state unchanged
  while successful items remain imported, and the user gets a combined result.
- While a batch runs, the top bar reports `Importing 2 of 5 · name…` with an
  import glyph, so a slow file is visibly in flight instead of leaving the
  surface silent. A completed multi-file batch reports the count.
- Opening an existing project and importing into it is proven end to end by
  `scripts/ui_scenarios/existing-project-import.json`: the project is saved
  without a sequence frame rate (the shape a new project can be saved in), the
  real clip imports, and inserting it creates the missing track.
- Folder import accepts at most 64 supported files, does not follow desktop
  symlinks, and reports an empty or oversized selection before mutating anything.
- Unsupported media is reported as unsupported (container or codec), never as
  corruption.
- A failed packaged `ffprobe` does not block import: the bridge falls back to
  the libavformat it links for playback and applies the identical matrix. The
  `mov-import` journey imports a real QuickTime file in the running app with
  `OR_FFPROBE_PATH` pointed at a missing executable, so the fallback is the path
  under test.

Supported profile: Matroska and WebM with AV1, VP8/VP9, H.264, H.265, FFV1,
ProRes, MJPEG, MPEG-4 and similar video codecs plus AAC, Opus, Vorbis, FLAC, ALAC,
AC-3/E-AC-3 and PCM audio; MOV/MP4/M4V with H.264/H.265/AAC; WAV, MP3, Ogg, FLAC,
AIFF and AVI single-container files; PNG, JPEG and WebP stills. Rejected
containers stay named, and a codec the *running build* cannot decode is refused at
import with `UNSUPPORTED_CODEC` rather than admitted and failing at the first
preview frame; the release bundle links a full FFmpeg while a local debug build
links a minimal one. See
[architecture/media-pipeline.md](../architecture/media-pipeline.md).

## Library surface

- Media panel lists items with metadata, thumbnails and audio waveforms.
- Search filters the library without touching project state; a search with no
  matches names the result and offers **Clear search**.
- Removing an item is a command and is guarded when timeline clips reference it.
- Relink replaces one item's source under the same `MediaId` and preserves
  timeline references; it is undoable.
- Offline/missing sources are shown as such; loading a project never probes them.

## Generated previews

Thumbnails and waveforms are produced by bounded background jobs into disposable
cache storage with a persistent LRU index. They are regenerable and never part
of the project. Proxy generation exists as a cache foundation, not as an exposed
workflow.

A preview that finishes for media no timeline clip shows repaints only the media
pane, through a value listenable; media a clip shows still publishes through the
editor state so the clip filmstrip updates. The pane therefore fills in without
rebuilding the editor for every thumbnail, and the timeline keeps the only
snapshot of previews it paints.

A preview that fails leaves a retry control in its place rather than a muted
placeholder that never resolves; retrying re-queues that one item instead of
refreshing the whole library. The whole tile is the retry target, not the
sixteen-pixel glyph inside it.

A page of the library that fails says so in a footer with its own Retry, which
reloads the open project state rather than appending a second page; the footer
Retry is unavailable while a page is in flight so one press cannot queue a
duplicate request over a page that was just appended.

## Where

- `crates/or_core/src/` — media identity, probe, cache, jobs
- `crates/or_app_bridge/` — artifact service wiring
- `apps/or_app/lib/screens/editor_media_library.dart` — library pane and item rows
- `apps/or_app/lib/screens/editor_media_preview.dart` — one item's preview states
- `apps/or_app/lib/` — import, relink and artifact loading flows

## Known gaps

Large-library cache behaviour, a broad offline/bulk-relink workflow, representative
camera-media compatibility, and Android physical-device performance.

## Related

- [editing.md](editing.md)
- [architecture/media-pipeline.md](../architecture/media-pipeline.md)
