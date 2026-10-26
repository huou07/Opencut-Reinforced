# Feature — Export

Last reviewed: 2026-10-26

Scope: turning a project into a playable file — profiles, job behaviour, failure
semantics, and how an export is validated.

## Profiles

| Profile | Container | Video | Audio | Where |
|---|---|---|---|---|
| Lossless | Matroska | FFV1 | PCM S16LE, 48 kHz stereo | desktop, Android |
| Delivery | WebM | VP9 | Opus, 48 kHz stereo | desktop, Android (verified on emulator) |

MP4/MOV, H.264/H.265/AV1 delivery and hardware encoders are future work gated on
licence, platform and build-configuration review. The package build enables only
the selected profile's muxer and encoders.

## Behaviour

- Export captures one project revision and queues bounded background work with
  monotonic frame progress and cancellation.
- Preview and export share the same timeline evaluation, renderer, text
  rasteriser and transform/effect path; resolution, quality and scheduling may
  differ.
- Output is staged beside the destination and published only after streams and
  trailer complete. Cancellation, offline media or encode failure cleans staging,
  preserves an existing destination, and leaves a new destination absent.
- A directory destination is rejected before encoding starts.
- Export never mutates canonical project state or increments the revision.
- The export receipt names the produced file and offers Show in Finder.

## Validation

An export is proven by an independent probe/playback of the actual file
(container, streams, duration, dimensions, expected frames), not by the export
job reporting success. See [operations/verification.md](../operations/verification.md).

## Interchange

SRT/WebVTT subtitles are supported. OpenTimelineIO is a bounded Rust/serde JSON
adapter over a documented subset (never a second project format), with Lottie
1.0/dotLottie 2.0 as bounded interchange; EDL/XML are future. Importers do not
execute expressions or fetch remote assets.

## Related

- [architecture/media-pipeline.md](../architecture/media-pipeline.md)
- [text-captions.md](text-captions.md)
- [reference/security-licensing.md](../reference/security-licensing.md) — codec obligations
