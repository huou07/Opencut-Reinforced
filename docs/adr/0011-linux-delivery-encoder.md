# ADR 0011 — A Linux delivery encoder for MP4 export

- Status: Accepted — implemented in the Linux desktop packages
- Date: 2026-11-03

This record covers one decision: how a Linux build of Opencut Reinforced gets an
H.264 encoder so the MP4 delivery profile works there. Measured behaviour of the
profile lives in [export.md](../features/export.md).

## Context

MP4 delivery is H.264 video with AAC audio. macOS supplies VideoToolbox and
Windows supplies Media Foundation, so both encode without an extra library. Linux
has neither: FFmpeg offers only hardware encoders there (`h264_vaapi`,
`h264_nvenc`, `h264_qsv`) and each needs a device or driver the package cannot
assume. A packaged editor cannot ask a person to own a GPU with a working
encode path.

Before this decision the Linux package shipped no H.264 encoder at all, so the
MP4 profile failed with "this FFmpeg build carries no H.264 encoder" and the
export picker hid MP4 on Linux. The owner asked for MP4 parity on Linux
(2026-11-03).

## Decision

The Linux FFmpeg build links **x264** (`libx264`), pinned to commit
`b35605ace3ddf7c1a5d67a2eb553f034aef41d55` of the `stable` branch, archive
SHA-256 `6eeb82934e69fd51e043bd8c5b0d152839638d1ce7aa4eea65a3fedcf83ff224`,
built statically by `scripts/build_desktop_export_codecs.sh` and enabled with
`--enable-gpl --enable-libx264 --enable-encoder=libx264`.

Only the Linux FFmpeg is built this way. The macOS and Windows builds keep
their operating-system encoders and stay LGPL, because x264 is not linked into
them.

`crates/or_media/src/encoder.rs` treats the encoder as a host capability, not a
constant: `H264_ENCODER_CANDIDATES` is tried in order — VideoToolbox, Media
Foundation, NVENC, QSV, AMF, VAAPI, OpenH264, then `libx264` — and the first one
this FFmpeg build resolves is used. A build with none of them reports the gap
and points at WebM instead of failing obscurely.

## Consequences

- Linux packages gain MP4 delivery with the same profile the other platforms
  use, which is what the owner asked for.
- `licenses/x264/COPYING` ships inside the Linux package and the release assets
  record the source offer. `scripts/stage_codec_licences.sh` stages the text
  only when the codec prefix produced it, and a package that carries the
  directory must carry the text.
- The Linux FFmpeg is a GPL-2.0-or-later binary because of that link. Every
  other platform stays LGPL. The combination is deliberate: the licence applies
  to the encoder library and to the FFmpeg build that links it, not to the
  application code, which is MIT.
- A distribution that wants an LGPL-only Linux build can drop the x264 block in
  `scripts/build_desktop_export_codecs.sh` and the three configure flags in the
  Linux job; MP4 then fails with the capability message and WebM remains the
  default.
- Building x264 adds roughly two minutes to a cold Linux codec build and needs
  `nasm` on the build host.

## Alternatives rejected

- **Hardware encoders only.** `h264_vaapi` needs a render node, NVENC and QSV
  need specific hardware, and none of them is present on a stock desktop.
- **Ship no MP4 on Linux.** That was the state the owner rejected: the picker
  hid the profile and the error told people to use WebM.
- **OpenH264.** Its binary licensing terms require a Cisco-hosted binary and a
  separate patent arrangement, which is a heavier dependency than the source
  build used here.
