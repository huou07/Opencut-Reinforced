# Feature — Media Library and Import

Last reviewed: 2026-10-26

Scope: bringing media into a project, showing it, replacing it, and telling the
user precisely what is wrong when it cannot be used.

## Import

- Desktop: multiple selected local files and bounded recursive folders.
- Android: persistable SAF tree/document grants, bounded by depth and entry
  count, with an app-private working copy.
- Each successful source enters canonical state only through `media.add`.
- Sources are probed independently: a failed item leaves project state unchanged
  while successful items remain imported, and the user gets a combined result.
- Folder import accepts at most 64 supported files, does not follow desktop
  symlinks, and reports an empty or oversized selection before mutating anything.
- Unsupported media is reported as unsupported (container or codec), never as
  corruption.

Supported profile: Matroska (FFV1/PCM S16LE, VP9/Opus, H.264/AAC), WAV PCM
S16LE, MP3 audio and WebM VP9/Opus. MP4-family H.264/AAC import is a candidate
profile pending packaged acceptance and codec-patent review; H.265 and AV1 are
future. See [architecture/media-pipeline.md](../architecture/media-pipeline.md).

## Library surface

- Media panel lists items with metadata, thumbnails and audio waveforms.
- Search filters the library without touching project state.
- Removing an item is a command and is guarded when timeline clips reference it.
- Relink replaces one item's source under the same `MediaId` and preserves
  timeline references; it is undoable.
- Offline/missing sources are shown as such; loading a project never probes them.

## Generated previews

Thumbnails and waveforms are produced by bounded background jobs into disposable
cache storage with a persistent LRU index. They are regenerable and never part
of the project. Proxy generation exists as a cache foundation, not as an exposed
workflow.

## Where

- `crates/or_core/src/` — media identity, probe, cache, jobs
- `crates/or_app_bridge/` — artifact service wiring
- `apps/or_app/lib/` — library panel, import flow

## Known gaps

Large-library cache behaviour, a broad offline/bulk-relink workflow, representative
camera-media compatibility, and Android physical-device performance.

## Related

- [editing.md](editing.md)
- [architecture/media-pipeline.md](../architecture/media-pipeline.md)
