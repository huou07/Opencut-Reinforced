# Media capability matrix

Last reviewed: 2026-11-05

Scope: what media this product actually imports and decodes, per host, with the
measurement that produced each row. A format is listed as supported only when a
file of it was generated, accepted by the import matrix, probed and decoded end
to end on that build. Anything not measured here is not claimed.

## How a row is earned

`scripts/probe_media_matrix.py` builds one tiny fixture per container and codec,
probes it with the same `ffprobe` the app ships, decodes every stream, and checks
the result against the import profile in `crates/or_core/src/media_probe.rs`
(video and audio codec lists, container headers, still-image rules). The script
reads those lists out of the Rust source, so the document cannot drift into a
friendlier opinion than the product enforces.

```bash
python3 scripts/probe_media_matrix.py --markdown --out /tmp/matrix.json
```

The shipped build is decode-oriented and has no test-source filters, no encoders
for most codecs and no `null` muxer, so the script separates two roles:
`OR_FFMPEG_PATH`/`OR_FFPROBE_PATH` name the build under test (default: the pinned
macOS prefix), and `OR_FFMPEG_GENERATOR` (default: `ffmpeg` on `PATH`) only
writes the fixtures. A decode is proven by piping the decoded stream into a sink
the build really has (`-f matroska -c:v ffv1` for video, `-f wav` for audio).

## Import matrix

These rows are identical on every host because they come from the Rust import
profile, not from the local build: Matroska/WebM, the ISO BMFF family
(`mov`, `mp4`, `m4a`, `3gp`, `3g2`, `mj2`), WAV, MP3, OGG, FLAC, AIFF, AVI, and
single PNG/JPEG/WebP stills. Anything else is refused at the header, before a
decoder is opened, with `UNSUPPORTED_CONTAINER` or `UNSUPPORTED_CODEC`.

GIF and TIFF are refused on purpose. They are listed in some video-editor
marketing tables, but this product has no still-image path for them and the
pinned macOS build has no TIFF decoder, so admitting them would create a clip
that cannot play.

## macOS 15 (Apple silicon), pinned FFmpeg 8.1.3

Measured 2026-11-05 on this development MacBook with
`ffmpeg version 8.1.3` from `~/Library/Caches/OpencutReinforced/ffmpeg-8.1.3-macos-image-support/ffmpeg-install`.

| Container | Video | Audio | Import matrix | Decode here | Encoder in this build |
|---|---|---|---|---|---|
| MP4 | H.264 | AAC | accepted | ok | `h264_videotoolbox` |
| MOV | H.264 | AAC | accepted | ok | `h264_videotoolbox` |
| MKV | H.264 | Opus | accepted | ok | `h264_videotoolbox`, `libopus` |
| WebM | VP9 | Opus | accepted | ok | `libvpx-vp9`, `libopus` |
| MKV | FFV1 | FLAC | accepted | ok | `ffv1` |
| MKV | AV1 | Opus | accepted | **fails** | `libopus` (no AV1 encoder) |
| MOV | ProRes | — | accepted | ok | — |
| MP4 | HEVC | AAC | accepted | ok | — |
| WAV | — | PCM s16le | accepted | ok | `pcm_s16le` |
| MP3 | — | MP3 | accepted | ok | — |
| FLAC | — | FLAC | accepted | ok | `flac` |
| M4A | — | AAC | accepted | ok | — |
| OGG | — | Opus | accepted | ok | `libopus` |
| PNG | — | — | accepted | ok | — |
| JPEG | — | — | accepted | ok | — |
| WebP | — | — | not measured | not measured | — |
| GIF | — | — | refused | — | — |
| TIFF | — | — | refused | — | — |

Findings that changed the product:

- **AV1 decode fails on this host.** The build carries the native `av1`
  decoder, but it delegates to VideoToolbox and this machine reports no AV1
  hardware decode: `Your platform doesn't support hardware accelerated AV1
  decoding` then `Function not implemented`. The import path therefore proves a
  real decode of the first video frame and refuses the file with
  `DECODE_UNAVAILABLE` and the codec plus the reason, instead of admitting a
  clip that would fail at its first preview frame. AV1 in a build without that
  dependency (Debian's FFmpeg below) decodes normally.
- **The shipped macOS build has no HEVC, ProRes, AV1 or MP3 encoder.** Export
  profiles stay limited to the three recorded in
  [export.md](../features/export.md); adding an HEVC profile needs a GPL-free
  encoder such as Kvazaar, as the
  [technology review](technology-review.md) records.
- **WebP is not measured on macOS** because the development generator lacks a
  WebP encoder; the row stays unmeasured rather than inferred.

## Debian 13, system FFmpeg 7.1.5

Measured 2026-11-05 on the `debian` host with `ffmpeg 7.1.5-0+deb13u1`.

| Container | Video | Audio | Import matrix | Decode here | Encoder in this build |
|---|---|---|---|---|---|
| MP4 | H.264 | AAC | accepted | ok | `libx264` |
| MOV | H.264 | AAC | accepted | ok | `libx264` |
| MKV | H.264 | Opus | accepted | ok | `libx264`, `libopus` |
| WebM | VP9 | Opus | accepted | ok | `libvpx-vp9`, `libopus` |
| MKV | FFV1 | FLAC | accepted | ok | `ffv1`, `flac` |
| MKV | AV1 | Opus | accepted | ok | `libsvtav1` |
| MOV | ProRes | — | accepted | ok | `prores_ks` |
| MP4 | HEVC | AAC | accepted | ok | `libx265`, `libx264` |
| WAV | — | PCM s16le | accepted | ok | `pcm_s16le` |
| MP3 | — | MP3 | accepted | ok | `libmp3lame` |
| FLAC | — | FLAC | accepted | ok | `flac` |
| M4A | — | AAC | accepted | ok | `aac` |
| OGG | — | Opus | accepted | ok | `libopus` |
| PNG | — | — | accepted | ok | — |
| JPEG | — | — | accepted | ok | — |
| WebP | — | — | accepted | ok | — |
| GIF | — | — | refused | — | — |
| TIFF | — | — | refused | — | — |

The Debian build is a general-purpose distribution FFmpeg: it has `libx265`
and therefore a GPL-enabled configuration. OR links the system build on Linux
and never asks it for x265; the licence position for that seam is recorded in
[ADR 0011](../adr/0011-linux-delivery-encoder.md). The row is here to say what
that build can decode, not to propose an encoder.

## Windows 11

Not measured. The `win11` host reachable from this session has no FFmpeg
installed, and the product ships its own helper there, so a row produced from a
different build would answer the wrong question. The matrix stays empty for
Windows until the packaged helper is measured on that host; the product claim for
Windows remains "the same import matrix and the same three export profiles",
because both come from shared Rust code, not from a per-platform build.

## What the app tells the user

- Refused at the header: `UNSUPPORTED_CONTAINER`, naming the file.
- Codec outside the profile: `UNSUPPORTED_CODEC`.
- No decoder in this build: `UNSUPPORTED_CODEC` with the codec.
- Decoder present but unusable on this machine: `DECODE_UNAVAILABLE`, with the
  codec, the underlying reason and the advice to import H.264 or VP9 instead.

The last case is why import opens one real decoder. Capability detection is a
measurement, not a lookup table.

## The gate sizes its own budget

`or_media::video_frame_decodes` opens one decoder, one session and one frame, so
it needs four in-flight leases and 64 MiB. A single-lease budget refuses its own
open, and then every import reports `DECODE_UNAVAILABLE` — which is what the
guided first-video journey caught on 2026-11-05. That journey is the positive
control for this row: it imports a phone H.264 file and asserts the media row
and the first clip appear, and
`crates/or_media/tests/software_decode.rs::the_import_decode_gate_admits_a_frame_it_can_decode`
pins the same behaviour in Rust. Without those, a gate that refuses everything
would look like a working refusal.

## The export side is the same measurement

`or_media::describe_export_capability` asks the linked encoders the same
question the writer asks before it starts: can this build open an H.264 encoder
(and AAC), libvpx VP9 with libopus, FFV1 with PCM S16LE. The app reads that
answer once per launch, so the Export dialog offers only formats the engine can
write, defaults to the engine's preferred profile, and Settings → Resources
shows the same Ready/Unavailable list. On this macOS host the panel reports
"This build exports MP4 (VideoToolbox, hardware), WebM (VP9), Lossless (FFV1)."
from the real bridge, which matches the encoder column above.

A row here and a row in that panel therefore cannot disagree: both come from the
same probe in `crates/or_media`, and neither is a list of names kept in Dart.
