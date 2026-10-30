# Media Pipeline — Probe, Decode, Cache, Jobs, Encode

Last reviewed: 2026-10-28

Scope: how media enters the project, how frames and audio are produced, what is
cached, and how bounded background work is scheduled.

## Runtime

OR links FFmpeg 8.1.3 built from the official source archive under an LGPL-only
configuration. Supported app bundles carry the linked runtime plus
packaged-first helper executables; a packaged user journey requires no system
`ffmpeg`/`ffprobe` and no developer `PATH` override. Unpackaged development may
use explicit overrides.

- Probing: `or_core::probe_media_file(&Path)` runs the packaged-first `ffprobe`
  helper directly (no shell), requests a bounded JSON field set, and enforces a
  timeout, output limits, process cleanup and metadata validation.
- Helper fallback: when the helper cannot start or exits without reading the
  file (`PROBE_FAILED`, `PROBE_BACKEND_UNAVAILABLE`, `INVALID_PROBE_OUTPUT`), the
  bridge probes the same path through the libavformat it already links for
  playback and export, then applies the identical import matrix. A file the
  helper rejected on its matrix is not retried; the helper's diagnostic wins.
- Decoding: `or_media` uses `ffmpeg-the-third` for demux, software decode and
  seek, with bounded queues and exact timestamps.
- Android probes granted SAF descriptors through the in-process FFmpeg boundary;
  a `content://` URI is never converted into a filesystem path.

Media bytes never enter the project document. Sources stay external and are
referenced by typed identity.

## Import profile

Accepted containers and streams are bounded; anything else is rejected before
or after probing with an accurate machine-readable reason (unsupported
container, unsupported codec — not "corrupt").

| Container | Supported streams |
|---|---|
| Matroska (`.mkv`) | FFV1/PCM S16LE, VP9/Opus, H.264/AAC |
| RIFF/RF64 WAV (`.wav`) | audio-only PCM S16LE |
| MP3 (`.mp3`) | audio-only MP3 |
| WebM (`.webm`) | VP9/Opus |
| ISO BMFF / MP4 family | H.264 video, AAC audio — candidate profile; packaged acceptance and codec-patent review are pending |

H.265/AV1 import and MP4-family export remain future work with their own
licence and distribution review. Details: [features/media-library.md](../features/media-library.md).

## Cache and generated artefacts

Thumbnails, waveforms, proxies, render intermediates and indexes are disposable
cache data, never canonical project state, and never required for project
correctness.

- Source fingerprints (v1) key the cache; fixed generation profiles keep output
  reproducible.
- A separate disposable SQLite index (schema v1) stores only kind, opaque cache
  key, byte size and access sequence — no source path, no project identity.
- Eviction is sequence-based LRU applied only when needed to satisfy the global
  artifact budget; index metadata does not count against it.
- Startup reconciliation repairs stale or drifted rows, because SQLite metadata
  and filesystem changes are not one atomic transaction.

## Background jobs

`JobManager` runs a fixed, caller-configured number of worker threads (never one
thread per job) with bounded queue and bounded tracked records. Submission is
non-blocking and reports backpressure as a structured error; cancellation is
cooperative for queued and running work; a panicking task cannot kill the pool;
shutdown stops submissions, skips queued work, signals running work and joins
workers. Terminal records are reclaimed oldest-first.

The media artifact service uses `JobManagerConfig(2, 32, 128)` and
`CacheStoreConfig(8 MiB, 256 MiB)` in the desktop bridge. It returns `Ready`,
`Queued`, `Running`, `NotApplicable` or `Failed`, shares identical in-flight work
by cache key, and carries an independent monotonic sequence on terminal events.
Closing cancels and reaps child processes before workers join.

This is not a realtime scheduler: playback-critical decode, audio and render work
must eventually outrank opportunistic thumbnail, waveform, proxy and AI work.
There is no job progress API, job persistence or priority system yet.

## Encoding

Export writes through the shared render path into a bounded FFmpeg encode/mux
job; see [features/export.md](../features/export.md) for profiles and failure
semantics.

## Related

- [realtime-plane.md](realtime-plane.md)
- [features/media-library.md](../features/media-library.md)
- [reference/security-licensing.md](../reference/security-licensing.md)
