# Export and render performance

Last reviewed: 2026-11-07

Scope: how export and render throughput is measured on a real host, the rows
each machine has recorded, and what those rows already prove about the
pipeline. Workload definitions and the profile contract live in
[export](../features/export.md); this document only carries measurements and the
commands that produce them.

## Measuring

The harnesses are ignored Rust tests in `crates/or_app_bridge/src/preview.rs`
(`mod export_pipeline_tests`). Build them once, then run the binary directly so
every row is taken without cargo in the loop:

```bash
cargo test -p or_app_bridge --release --lib export_media_benchmark --no-run
BIN=$(ls -t target/release/deps/or_app_bridge-* | grep -v '\.d$' | head -1)

# Direct path: one clip covering the canvas, no effects, no text.
OR_BENCH_SOURCE=/tmp/or-bench/bench1080.mp4 OR_BENCH_FRAMES=300 "$BIN" export_media_benchmark --ignored --nocapture

# One text layer over the same clip.
OR_BENCH_TEXT=1 OR_BENCH_SOURCE=/tmp/or-bench/bench1080.mp4 OR_BENCH_FRAMES=300 "$BIN" export_media_benchmark --ignored --nocapture

# 4K source scaled down to 1080p.
OR_BENCH_SOURCE=/tmp/or-bench/bench4k.mp4 OR_BENCH_SRC_W=3840 OR_BENCH_SRC_H=2160 \
  OR_BENCH_OUT_W=1920 OR_BENCH_OUT_H=1080 OR_BENCH_FRAMES=150 \
  "$BIN" export_media_benchmark --ignored --nocapture

# One stage at a time: decode, render, encode. A no-text render without
# OR_BENCH_SCALED=1 is a decode passthrough: one covering clip with no effects
# and no text returns the decoded frame before the compositor runs. Add the
# knob to measure the composited stage (and a text row) fairly.
OR_BENCH_MODE=render OR_BENCH_SCALED=1 OR_BENCH_SOURCE=/tmp/or-bench/bench1080.mp4 OR_BENCH_FRAMES=300 "$BIN" export_media_benchmark --ignored --nocapture
OR_BENCH_MODE=render OR_BENCH_SCALED=1 OR_BENCH_TEXT=1 OR_BENCH_SOURCE=/tmp/or-bench/bench1080.mp4 OR_BENCH_FRAMES=300 "$BIN" export_media_benchmark --ignored --nocapture
OR_BENCH_MODE=decode OR_BENCH_NV12=1 OR_BENCH_SOURCE=/tmp/or-bench/bench1080.mp4 OR_BENCH_FRAMES=300 "$BIN" export_media_benchmark --ignored --nocapture

# A real project through the path the Export dialog starts.
OR_PROJECT_BENCH=/tmp/or-bench/opaque.orproj OR_PROJECT_BENCH_PROFILE=mp4 \
  "$BIN" export_project_benchmark --ignored --nocapture

# Only the audio pre-decode stage of that project.
OR_PROJECT_BENCH=/tmp/or-bench/multiaudio.orproj OR_PROJECT_BENCH_MODE=audio \
  "$BIN" export_project_benchmark --ignored --nocapture
```

Two rules decide whether a row means anything:

- Measure on an idle machine. A concurrent `cargo` build halved the same row
  (225 fps against 243 fps), which is larger than most optimisations.
- Repeat every row. Run-to-run spread on this hardware reaches ±8 %.

The product ships its own FFmpeg, so a row must run against that build, not a
system one: macOS needs
`DYLD_FALLBACK_LIBRARY_PATH=<prefix>/lib`, Linux needs
`LD_LIBRARY_PATH=<prefix>/lib` when the prefix is not on the default search
path. Linux and Windows hosts build that prefix from the repository scripts
(`scripts/build_linux_ffmpeg.sh`, `scripts/build_windows_ffmpeg.sh`) so the
encoders the row claims are the encoders the build contains; a prefix built by
hand is where both hosts lost H.264 once already.

`OR_BENCH_MODE=encode` is an encoder ceiling taken with blank NV12 frames, not an
export row. Leave `OR_BENCH_MODE` unset for a full-pipeline number.

## Machines

| Host | CPU | GPU path | OS | Packaged FFmpeg |
|---|---|---|---|---|
| MacBook Air | Apple M5, 10 cores, 16 GB | Metal / VideoToolbox | macOS 27.2 | 8.1.3, `h264_videotoolbox` |
| Debian | Intel Core i5-13500T, 20 threads, 15 GB | Intel UHD 770 / VAAPI | Debian 13, kernel 7.2.6 | 8.1.3 with `h264_vaapi`, `h264_nvenc`, `libx264` |
| Windows 11 | Intel Core 5 120U, 15.7 GB | Intel Graphics / Vulkan / Media Foundation | Windows 11 26200 | 8.1.3, `h264_mf`, `h264_nvenc`, `aac` |

## 2026-11-06 rows — GPU text compositing

Measured at `8dc2e3ff` (branch `perf/gpu-text-composite`, parent `7bb42803`),
`--release`, 300 frames, `/tmp/or-bench/bench1080.mp4` on all three hosts, on an
idle machine. "Before" is the parent commit measured in the same session by
interleaving the two binaries, not a number taken from an older document: the
host load moved absolute fps by a factor of two between runs, so only an
interleaved A/B is comparable.

macOS M5 (VideoToolbox H.264, 300 frames):

| Workload | Before | Now |
|---|---|---|
| MP4 quality, one clip | 241.8 / 242.3 / 244.4 | 241.8 / 242.3 / 244.4 |
| MP4 quality, one text layer | 112.5 | 208.5 / 224.5 / 233.2 |
| Render only, no text | 1288 | 1278 / 1342 / 1354 |
| Render only, one text layer | 126.6 | 283 / 285 / 313 |

macOS stage ceilings after the change: render 1342 fps without text and 313 fps
with one text layer (was 1279 / 130). VideoToolbox H.264 blank frames 280.9 fps,
so the text path is now encoder-bound rather than compositor-bound. The two
render rows in that table were not the same stage: the no-text row took the
direct decode path while the text row composited and waited for its readback.
The 2026-11-07 section below repeats them like for like.

Debian 13 (VAAPI H.264, 150 frames for the export rows):

| Workload | Before | Now |
|---|---|---|
| MP4 quality, one clip | 225.0 / 232.8 | 225.0 / 232.8 |
| MP4 quality, one text layer | 34.4 | 62.3 / 65.3 |
| Render only, no text | 700.2 | 698.9 / 707.8 / 719.4 |
| Render only, one text layer | 34.4 | 49.5 / 49.9 / 50.4 |
| Encoder only (blank NV12, 150 frames) | — | 442.7 / 445.2 / 446.1 / 447.6 |

The Debian host composites on its integrated Intel UHD 770 through Vulkan
(`OR_EXPORT_BENCH_ADAPTER Intel(R) UHD Graphics 770 (ADL-S GT1) (Intel
open-source Mesa driver, Vulkan)`), not on a software rasteriser. The text rows
in this table composite and wait for the NV12 readback on every frame, while the
no-text rows took the direct path; the like-for-like rows in the 2026-11-07
section show what the text layer itself costs (within noise). The gap between
this host's integrated device and the Windows one is measured but not explained —
see "Known limits". Output bytes are unchanged by the compositor: 6 237 429
without text and 6 222 521 with text, LSB-identical at the same settings.

Windows 11 (Media Foundation H.264, 300 frames):

| Workload | Before | Now |
|---|---|---|
| MP4 quality, one clip | refused | 121.5 / 121.6 / 121.7 |
| MP4 quality, one text layer | refused | 74.7 / 78.2 / 79.4 |
| Render only, no text | 189.7 | 188.2 / 188.2 / 188.5 |
| Render only, one text layer | 32.9 | 91.3 / 99.4 / 101.8 |

Windows MP4 rows exist for the first time: the rebuilt prefix carries
`h264_mf` (Media Foundation), `h264_nvenc` and `aac`, and the encoder probe
reports a Constrained Baseline H.264 stream with an AAC LC track.

Encoder actually used, per host — read from stderr and confirmed with `ffprobe`
on the produced file, never from the profile name:

| Host | Encoder evidence | Stream |
|---|---|---|
| macOS | `h264_videotoolbox` selected by `FfmpegExportWriter` | h264 High 1920×1080 + aac LC |
| Debian | `[CUDA @ …] Could not dynamically load CUDA` then `[h264_vaapi @ …] No quality level set; using default (20).` | h264 High 1920×1080 yuv420p + aac LC |
| Windows | Media Foundation path (no CUDA, no QSV driver); `ffprobe` shows Constrained Baseline | h264 Constrained Baseline 1920×1080 + aac LC |

## 2026-11-07 rows — the text row measured a different stage

At `b1347dfa` (branch `mission/hardware-profiles-and-polish`), 300 frames of the
same `/tmp/or-bench/bench1080.mp4` fixture, 1920×1080, one idle machine per host.
The commit adds `OR_BENCH_SCALED=1`, which shrinks the clip by one thousandth so
`clip_covers_output` declines; both the no-text and text rows then run the same
compositor and the same synchronous readback. Without the knob, a no-text render
is a decode passthrough and the comparison is meaningless.

| Workload | macOS M5 | Debian (UHD 770) | Windows (Intel Graphics) |
| --- | --- | --- | --- |
| Full MP4, one clip | 221.8 | 260.7 | 128.2 |
| Full MP4, one text layer | 219.5 | 196.1 | 95.6 |
| Compositor + sync readback, no text | 116.2 | 88.2 | 41.0 |
| Compositor + sync readback, one text layer | 115.4 | 87.1 | 40.9 |
| Decode only, NV12 | 587.4 | 768.6 | 208.6 |
| Decode only, RGBA | 191.7 | 279.5 | 57.8 |
| Encoder ceiling, blank NV12, 150 frames | 275.7 | 444.3 | 257.1 |

The text layer itself costs 0.7 % on macOS, 1.3 % on Debian and 0.2 % on Windows
once both rows take the same path, so there is no text-compositing problem to
fix: the earlier "6–8× text" reading compared a passthrough with a composited
frame. What the full-pipeline rows still show is the compositor pass plus the
GPU YUV conversion replacing the copy-free passthrough: about 1 % on macOS (the
VideoToolbox encoder is the ceiling), 25 % on Debian and 25 % on Windows. The
fully synchronous pipeline (compositor plus an immediate readback) runs at
8.6 ms/frame on the Apple GPU, 11.3 ms on the UHD 770 and 24.4 ms on the Windows
machine; the export writer queues that readback, which is why the full-pipeline
text row reaches 196 fps on Debian even though the same stage path measures
87 fps.

The Windows render rows are also the slowest of the three on identical work, and
its decode-only NV12 row (208.6 fps) is well below Debian's (768.6 fps), so the
Windows gap is decode throughput and CPU, not the compositor. A hardware Media
Foundation MFT exists (`h264_mf -hw_encoding 1 -pix_fmt nv12` selects "Intel(R)
Quick Sync Video H.264 Encoder MFT") but measured slower than the software MFT in
the command-line test, and the encoder is not the Windows bottleneck (257 fps
ceiling against a 128 fps pipeline), so it is not integrated.

### Container and codec rows at the final revision

Same fixture, same 150-frame window, no text, measured on each host at the
final revision of the branch. Byte counts are identical across hosts because
VP9 CRF and FFV1 are rate-independent: the three encodes are the same file.

| Host | WebM VP9/Opus | WebM bytes | Matroska FFV1/PCM | Lossless bytes |
|---|---|---|---|---|
| macOS M5 | 37.1 fps | 4,109,503 | 42.3 fps | 39,278,557 |
| Debian (no GPU) | 25.1 fps | 4,109,503 | 33.4 fps | 39,278,557 |
| Windows (Core 5 120U) | 9.4 fps | 4,109,503 | 6.6 fps | 39,278,557 |

Re-checked at the scissor revision (`f0b44a24`) with one text layer, 150 frames.
macOS: MP4 213.7 fps (5,904,842 B, `h264` High `yuv420p` + `aac` LC 48 kHz
stereo), WebM 54.5 fps (3,600,271 B, `vp9` Profile 0 + `opus`), Matroska 54.6 fps
(47,874,201 B, `ffv1` `bgra` + `pcm_s16le`). Debian: MP4 114.3 fps (6,222,521 B,
`h264` High + `aac` LC), WebM 24.3 fps (3,606,822 B, `vp9` + `opus`), Matroska
32.2 fps (47,874,935 B, `ffv1` + `pcm_s16le`). The Debian MP4 row with text has
climbed from 62.5 fps at `b3650587` to 114.3 fps. Windows at the same revision:
MP4 81.7 fps (5,895,002 B, `h264` Constrained Baseline + `aac` LC), WebM 9.0 fps
(3,615,894 B), Matroska 6.4 fps (47,874,924 B).

A 4K source is now measured on the two hosts that encode H.264 through the
pinned runtime, 100 frames of the same 3840×2160 fixture, scaled to a 1080p
canvas and then kept at 4K:

| Host | 4K → 1080p | 4K → 4K (direct path) | Encoder evidence |
| --- | --- | --- | --- |
| Debian | 45.8 fps (3,353,843 B) | not measured | `h264_vaapi` |
| Windows | 14.6 fps (3,478,383 B) | 35.2 fps (6,119,753 B) | `[h264_mf @ …] MFT name: 'H264 Encoder MFT'` |

The Windows downscale row was 6.8 fps before this session; the 4K→4K row is
faster than the downscale because it matches the source size exactly and takes
the direct NV12 path with no scaler and no compositor pass, so it measures the
encoder rather than the pipeline.

One caveat the text rows made visible: with a text layer the byte counts are no
longer identical across hosts (47 874 201 / 47 874 935 / 47 874 924 for lossless),
because the composited frame reaches the encoder through the GPU YUV conversion
and drivers round it differently. The no-text rows above stay byte-identical
because that path copies the decoder's planes untouched. Nothing on the codec path changed
after the rows above, so those host rows still describe these containers, and the
text column now shows the scissor helping the VP9 path as well (32.7 → 54.5 fps).

`ffprobe` on all three outputs: `vp9` Profile 0 + `opus` in `matroska,webm`, and
`ffv1` + `pcm_s16le` in `matroska,webm` — the encoders the profile names, not
the profile label. WebM and lossless are software-only paths on every host, so
their rows track CPU throughput rather than the GPU compositor; Windows WebM is
the slowest row in the whole table and remains a known limit, not a regression
(the parent build measured 8.4 fps there).

Re-measured at the final revision of the branch (`ba0034ff`) as a smoke check:
macOS render 1311 fps / 226 fps (text) and MP4 216.8–234.1 fps / 167.0–181.9 fps
(text); Debian render 698.9 / 49.2 fps and MP4 150 frames 239.6 / 62.5 fps;
Windows render 142.0 / 89.7 fps and MP4 114.0 / 76.0 fps. The spread between
batches is host load, not a code change: the three-host tables above were taken
in one quiet window.

Re-measured once more after the video-clip-audio work (`0eb453c8`), same
fixture and profile, MP4 quality: macOS 234.1 fps (300 frames, 11,267,668
bytes) and 223.6 fps with text (11,732,698 bytes); Debian 232.0 fps over 150
frames (6,237,429 bytes, `ffprobe` h264 High + aac LC); Windows 111.5 fps over
150 frames (6,055,754 bytes, `ffprobe` h264 Constrained Baseline + aac LC, and
the writer now logs `[h264_mf @ …] MFT name: 'H264 Encoder MFT'`, which is the
encoder itself rather than an inference from the profile). The audio work adds
nothing to these rows because the fixture has no audio stream.

## Copy-free plane handoff (2026-11-06)

The direct export path (one covering clip, no effects, no text) used to copy the
decoder's 3.1 MiB NV12 frame into `Nv12Planes` on every frame even though the
frame lease already owns the bytes behind an `Arc`. `Nv12Planes` now holds either
owned planes or a window into that shared buffer, and `VideoFrame::shared_bytes`
offers it directly.

| Host | Frames | Before | After | Delta |
| --- | --- | --- | --- | --- |
| macOS M5 | 300 | 241.9 / 242.9 / 246.6 / 248.0 | 243.0 / 247.7 / 247.8 / 248.5 | ≈ +1.2 % (median 244.8 → 247.8) |
| Debian (UHD 770) | 300 | 247.7 / 256.1 / 256.5 | 260.5 / 260.5 / 262.2 | ≈ +1.7 % (median 256.1 → 260.5) |

Output bytes are identical before and after on both hosts (11 267 668 on macOS,
12 122 320 on Debian at 300 frames), so the copy was pure overhead. It is a
small win: the removed copy is cheaper than a pixel-format conversion, and the
gain shows up mainly because one allocation per frame disappears.

## Ink-rectangle scissor (2026-11-06)

A text rasterisation is a full-canvas RGBA buffer that is transparent almost
everywhere, so the compositor used to shade every canvas pixel for it on every
frame. `TextRasterizer` now reports the bounding box of pixels with alpha, the
layer carries it as an optional scissor and the compositor clips the draw to that
rectangle; a rasterisation with no ink is skipped entirely. The scissor is only
used for an untransformed, uncropped text layer with no effects, because a moved
or scaled layer paints outside its canvas rectangle and an effect can spread
pixels past the glyphs; both fall back to blending the whole layer.

| Host | Workload | Frames | Before | After | Delta |
| --- | --- | --- | --- | --- | --- |
| macOS M5 | render-only, one text layer | 300 | 219.5 / 246.2 / 247.9 | 385.2 / 388.8 / 374.9 | ≈ +56 % |
| macOS M5 | MP4 quality export, one text layer | 150 | 188.4 / 204.7 | 215.5 / 221.2 | ≈ +9 % |
| Debian (UHD 770) | render-only, one text layer | 300 | 50.4 / 49.7 / 50.0 | 52.8 / 52.3 / 52.2 | ≈ +4.6 % |
| Debian (UHD 770) | MP4 quality export, one text layer | 150 | 60.6 / 75.9 | 89.6 / 76.7 | inconclusive (VM variance) |

The scissor only skips fragments the layer never wrote, so it must not change the
picture: a 30-frame text export at 1080p compares at `PSNR y:inf` (bit-identical
pixels, 1 209 179 bytes on both sides).

The Debian row is the honest limit of this change: on macOS the blend was fill
bound, so clipping it to the ink was worth half the frame time again, while on
Debian only a few percent moved. That means the Debian compositor spends most of
its frame somewhere other than text fragments — the same host runs the no-text
direct path at ~700 fps and any composited frame at ~50 fps, so the cost is in
the composited path itself (plane upload, pass, readback), not in the text layer.
The cause is recorded as unresolved rather than guessed at, but two probes
narrow it. Render-only with one text layer at a series of canvas sizes on Debian
runs 355 fps at 160x90, 206 at 320x180, 158 at 640x360, 68.8 at 1280x720 and 53.6
at 1920x1080, while `OR_BENCH_MODE=decode` stays flat at ~270 fps — so the cost
tracks canvas memory traffic, not frame count. And lossless `MKV`, which reads
back 8.3 MiB of RGBA instead of 3.1 MiB of NV12, measures 55.5 fps against MP4's
53.4 fps at 1080p, so the readback is not the constraint. That leaves the plane
upload into the compositor (about 225 MB/s effective on this host) as the likely
term, which is exactly what importing hardware decoder frames instead of
software planes would remove.

## Hardware decode, measured and not adopted (2026-11-06)

The macOS runtime is built with `--enable-videotoolbox` but only for the H.264
*encoder*; decode runs through libavcodec on the CPU. That looked like a gap
worth closing, so it was measured before anything was built into the app.

A build with the VideoToolbox hwaccel enabled (`--enable-hwaccel=h264_videotoolbox,hevc_videotoolbox`,
verified as `CONFIG_H264_VIDEOTOOLBOX_HWACCEL 1` in `config_components.h`) was
produced and compared on a 4K H.264 fixture (3840×2160, 200 frames,
`/tmp/or-bench/bench4k.mp4`, `libx264 -preset ultrafast -crf 26`):

| Path | Wall time, three runs | Rate |
| --- | --- | --- |
| software libavcodec (`ffmpeg -i … -f null -`) | 0.16 / 0.19 / 0.17 s | ≈1176 fps |
| VideoToolbox (`ffmpeg -hwaccel videotoolbox … -f null -`) | 1.32 / 1.34 / 1.33 s | ≈150 fps |

Software decode of 4K H.264 on this host is already far faster than the
hardware path, whose frame transfer to system memory dominates. Since OR's
pipeline consumes system-memory frames (the compositor uploads them itself),
hardware decode would be a regression here, not a win: nothing was integrated,
and the recipe keeps the minimal `--disable-everything` posture rather than
carrying a hwaccel it does not use.

The same reasoning applies to the Windows and Linux rows: their decode is
CPU-side, and the measured encode ceilings above (Media Foundation on Windows,
`h264_vaapi` on Debian) show the pipeline is not decode-starved at 1080p.

The two Intel hosts were then measured on 2026-11-07 rather than argued from
the macOS result. Both probes intentionally include the frame transfer, because
OR's compositor consumes system-memory NV12 and would pay it too.

Debian, 300 frames of the 1080p fixture, decode to raw NV12 on `/dev/null`
(static probe build in `~/or-build/ffmpeg-decode-probe`, three runs each):

| Decode path | Wall time | Rate |
| --- | --- | --- |
| software libavcodec | 0.388 / 0.404 / 0.481 s | ≈743 fps |
| VA-API + `hwdownload,format=nv12` | 0.364 / 0.492 / 0.511 s | ≈610 fps |

Windows 11, same 12-second fixture decoded and re-encoded through `h264_mf`
(the encoder is identical in both arms, so the difference is the decoder):

| Decode path | Wall time | Rate |
| --- | --- | --- |
| software libavcodec | 1.437 / 1.454 / 1.491 s | 264 fps |
| D3D11VA + download | 1.515 / 1.524 / 1.561 s | 253 fps |

Hardware decode is 5–18 % slower than software decode on both hosts, and the
export's own decode row (772.6 fps on Debian, 214.7 fps on Windows) is already
far above the composited export rate. Integrating a hwaccel would need a
GPU-resident compositor input to pay off; wgpu has no VAAPI or D3D11VA surface
import, so the frames would be copied back anyway. Not integrated, with the
numbers above as the reason.

## Stage concurrency (2026-11-06)

Per-core and per-GPU-engine counters, taken while the harness ran the same
1080p fixture. They answer "which stage owns the machine", not "how many
workers could exist".

- Debian (20 threads, no display GPU; 600 frames, text layer): the render stage
  ran at 49.8 fps while the busiest core sat at 15.4 % and the mean over all 20
  cores was 3.0 % — about 0.6 cores in total. The full export (84.6 fps,
  VAAPI `h264_vaapi`) used about 1.2 cores with a 20 % peak. Nothing there is
  waiting on a thread pool: the stage is one serial chain of decode, upload,
  composite and submit, and hardware encode is off the CPU.
- Windows (12 logical, Media Foundation): a 1080p export at 111 fps moved the
  GPU engine counters by at most 0.2 % on the 3D engine and produced no
  `engtype_VideoEncode` samples at all, so the MFT in use on this host is the
  software H.264 encoder, not the hardware one. The row is CPU-bound.
- macOS: per-core and per-GPU counters were **not measured**. `powermetrics`
  is the only source for per-core residency and GPU busy on this OS and it
  requires root, which this session does not take. The VideoToolbox rows are
  therefore evidence about wall-clock throughput only.

## What the rows prove

- Compositing text over an NV12 frame on the GPU removes the text penalty. The
  old path refused the NV12 layer as soon as any text clip was active, decoded
  every frame to RGBA on the CPU (5.7 ms against 0.7–2 ms) and uploaded a
  full-canvas RGBA layer per frame. The new path keeps the video in NV12
  planes, converts BT.601 limited range to RGB in the layer shader, and
  composites into a pooled target; the text layer uploads once per cache id.
  Text rows improved 2.3–2.5× on macOS, 2.8–3.1× on Windows and 1.45× on
  Debian, while the no-text rows stayed at parity on all three hosts.
- Export is encode-bound on macOS now, with the text row at 208–233 fps against
  a 280.9 fps VideoToolbox ceiling and a 1278–1354 fps render stage.
- The picture is preserved: PSNR against the parent build is 42.9 dB luma /
  40.6 dB average on the MP4 quality profile and 41.3 dB on WebM. The shader
  deliberately inverts BT.601 because libswscale decodes NV12 with its default
  matrix and frames carry no colour metadata; an earlier BT.709 inverse
  measured 24.9 dB, a visible colour shift.
- Windows was never limited by the operating system. The installed prefix had
  been built before the recipe gained Media Foundation support and without
  ffnvcodec headers; rebuilding it produced `h264_mf`, `h264_nvenc` and `aac`,
  and MP4 exports at 121 fps. The same was true on Debian, whose prefix had
  been configured with the minimal macOS-style arguments instead of the Linux
  build: `scripts/build_linux_ffmpeg.sh` now builds that recipe on any Debian
  host and asserts the encoders in `config_components.h` before `make`.
- VAAPI carries a quality ladder (`[h264_vaapi] No quality level set; using
  default (20)` is gone) and the audio pre-decode scales with the machine.

## 2026-11-07 rows — keyframe baseline

Measured at `4cf49782` with the harness fix `13dca62d` on the MacBook Air M5,
300 frames, `/tmp/or-bench/bench1080.mp4`, `--release`, idle machine, two runs
per row (full pipeline, `OR_BENCH_MODE` unset):

| Row | fps |
|---|---|
| MP4 H.264 + AAC | 239.6, 245.6 |
| MP4 H.264 + AAC + one text layer | 240.4, 234.7 |
| MP4 H.264 + AAC, animated scale keyframes (`OR_BENCH_ANIM=1`) | 238.9, 243.8 |

Animated rows are measured at `37f39a67`; the keyframes sample once per frame
and the row stays inside the static spread, so animation does not add a
measurable export cost on this host. Debian animated rows were re-taken at
`ab5d4620` (below); Windows animated rows are not taken.

These match the 2026-11-06 rows within run-to-run spread (242 and 208–233), so
the keyframe plumbing did not move the static path: a clip with no keyframes
skips the per-frame sampling (`ClipAnimation::is_empty`) and reads its static
transform exactly as before.

### Debian rows at `37f39a67`

Measured on the Debian host (Intel UHD 770 through Vulkan, `h264_vaapi`
encoder, GPL FFmpeg prefix `~/or-build/ffmpeg-8.1.3-install`), 300 frames,
1920x1080, one run per stage unless noted:

| Stage | fps |
|---|---|
| Decode only (`OR_BENCH_MODE=decode`) | 273.8, 276.8 |
| Encode only (`OR_BENCH_MODE=encode`) | 454.3, 447.8 |
| Render only, video (`OR_BENCH_MODE=render`) | 745.9, 779.2 |
| Render only, one text layer | 52.6, 53.2 |
| Full MP4, video only | 256.0, 260.6 |
| Full MP4, one text layer | 65.6, 83.2, 68.1, 59.8 |

The video-only full pipeline improved from 225 fps (2026-11-06) to 256–261; that
path decodes NV12 and hands the planes to the encoder without compositing. Any
text or effect layer forces the composited path: an RGBA target, two full-screen
NV12 conversion passes and two texture-to-buffer copies, then a blocking
`device.poll(PollType::Wait)`. Temporary `OR_TRACE_COMPOSE` instrumentation
(not committed) put the CPU submit at 25–74 µs and the poll at 13.0–14.2 ms per
1080p frame, while the text raster cache hit in 2–4 µs and layer setup took
0.5–0.8 ms. The text row is therefore readback-bound, not shaping-bound; it is
also why the same workload beats 300 fps on the M5 (≈3 ms for the same path).
The one-pass conversion above removes the two extra passes and the per-row
repack at `1feb6108`; what remains is the 1080p host copy itself (~6 ms measured
by the probe) plus the blocking poll, so asynchronous or GPU-resident readback
is still the next step.

A readback probe (`cargo test -p or_render --release readback_cost_by_size
-- --ignored --nocapture`) separates the fixed part from the size part on the
same host: a plain RGBA texture copy to host memory costs 0.56 ms at 64x64,
1.13 ms at 640x360 and **6.15 ms at 1920x1080** (~1.3 GB/s). The composited
NV12 readback costs about twice that because it adds two full-frame conversion
passes and one copy per plane, which is the 13–14 ms seen above: the ceiling is
the host copy plus conversion, not a fixed driver stall.
### Windows rows at `37f39a67`

Measured on the Windows 11 host (Intel Graphics through Vulkan, Media Foundation
encoder), 300 frames, 1920x1080, `C:/Users/hp/bench1080.mp4`:

| Stage | fps |
|---|---|
| Decode only | 63.8 |
| Encode only (Media Foundation) | 278.1 |
| Render only, video | 214.4, 210.2 |
| Render only, one text layer | 104.4, 104.6 |
| Full MP4, video only | 129.1, 127.2 |
| Full MP4, one text layer | 78.8, 82.6 |

The full MP4 row improved from 121 fps (2026-11-06) and the render row from 188;
the text row is unchanged within spread. Unlike Debian, one text layer costs
Windows about 2x rather than 14x: the composited readback is cheaper on this
driver, which is consistent with the readback explanation above and with the
earlier note that the Windows text row was roughly half the Debian one.

### Animated rows (Debian at `ab5d4620`, Windows at `a0d33bc1`)

The same `OR_BENCH_ANIM=1` knob (three scale keyframes, 1000 → 1120 → 1000),
300 frames, `--release`, one-pass conversion:

| Host | Row | fps |
|---|---|---|
| Debian / UHD 770 | MP4 H.264 + AAC, animated | 260.6, 260.6 |
| Debian / UHD 770 | MP4 H.264 + AAC, animated + one text layer | 119.2, 96.7 |
| Windows / Intel Graphics | MP4 H.264 + AAC, animated | 126.9 |
| Windows / Intel Graphics | MP4 H.264 + AAC, animated + one text layer | 80.0 |

Animation samples one transform per frame and the composited path already runs
every frame, so each animated text row sits inside its host's static text
spread (Debian 101.9–118.3, Windows 78.8–88.0); the sample itself is not
measurable next to the readback cost. Windows video-only animation costs
about 5 % against its static row (126.9 vs 133.9–135.1).

## 2026-11-07 rows — one-pass NV12 conversion

Measured at `1feb6108` (branch `mission/hardware-profiles-and-polish`),
`--release`, 300 frames, `bench1080.mp4`, idle machine. The composited path no
longer converts with two full-screen render passes and two padded copies; one
compute shader writes a tight NV12 buffer and the readback copies it once.
"Two-pass" is the same binary at `07d91f64` measured earlier in the session, not
an interleaved A/B, so treat small deltas as spread.

Debian 13 (Intel UHD 770, Vulkan, VAAPI H.264):

| Workload | Two-pass | One compute pass |
|---|---|---|
| Render only, one text layer | 52.6 / 53.2 | 109.1 / 111.3 / 111.9 |
| Full MP4, one text layer | 59.8 / 65.6 / 68.1 / 83.2 | 106.5 / 109.4 / 111.9 |
| Full MP4, no text | 256.0 / 260.6 | 260.5 |

The composited Debian ceiling roughly doubled. The video-only row is unchanged
because that path still hands decoder NV12 straight to the encoder.

macOS M5 (VideoToolbox):

| Workload | Two-pass | One compute pass |
|---|---|---|
| Full MP4, one text layer | 240.4 / 234.7 | 219.8 / 229.2 / 231.7 / 236.7 / 243.6 |
| Full MP4, no text | 239.6 / 245.6 | 242.7 / 243.9 |

Text stays inside the run-to-run spread (the Apple GPU already paid ~3 ms for
the old conversion), so no change is claimed there.

Windows 11 (Intel Graphics, Vulkan, Media Foundation):

| Workload | Two-pass | One compute pass |
|---|---|---|
| Full MP4, one text layer | 78.8 / 82.6 | 70.3 / 74.2 / 74.9 |
| Render only, one text layer | 104.4 / 104.6 | 96.0 / 96.8 |
| Full MP4, no text | 129.1 / 127.2 | 133.9 / 134.7 / 135.1 |

This driver runs the compute conversion about 8 % slower than the two render
passes on the text workload, while the video-only row moves up slightly. Both
hosts report Vulkan, so the choice cannot be made from the backend name. The
renderer therefore times both conversions once on the first composited frame
and keeps the faster one for the life of the device; the rows above are the
measurement that decision is based on. The first frame pays for the warm-up
(two conversions per path), which is inside the noise of a delivery export.

### After the per-device choice

The renderer times both conversions once on the first composited frame and keeps
the faster one for the life of the device. Rows with that choice in place:

| Host | Full MP4, one text layer | Full MP4, no text |
|---|---|---|
| macOS M5 (Metal, VideoToolbox) | 232.2 / 235.4 | — |
| Debian 13, Intel UHD 770 (Vulkan, VA-API) | 101.9 / 104.9 / 118.3 | 256.5 |
| Windows 11, Intel Graphics (Vulkan, Media Foundation) | 88.0 | 135.1 |

The Windows row is above both previous variants (78.8 / 82.6 with two passes,
70.3–74.9 with the forced compute pass) because the choice now picks the two
render passes there and the run followed a warm build; it is one run, so treat
the size of the gain as indicative rather than settled.

## 2026-11-07 rows — queued NV12 readback

Measured at `02cd025c` against its parent `69aa1b31` on the same three hosts,
`--release`, 300 frames, 1080p `bench1080.mp4`, idle machine, one text layer,
`mp4`/`h264` profile. The renderer no longer blocks on its own readback: it
queues the colour conversion and the map, and the export's writer thread waits
for that submission before it hands the planes to the encoder. The render
thread is therefore free to decode and rasterise the next frame while the GPU
converts this one. Each host was measured by checking out the parent, rebuilding
and running the same row in the same session, so the numbers are an interleaved
A/B and not a comparison with an older document.

Debian 13, Intel UHD 770 (Vulkan, `h264_vaapi`):

| Row | Parent `69aa1b31` (waited readback) | `02cd025c` (queued readback) |
|---|---|---|
| Full MP4, one text layer | 107.9 | 190.6 / 193.6 / 198.0 / 201.2 |
| Full MP4, one text layer, animated clip | — | 195.9 |
| Render stage only, one text layer | 52.6 / 53.2 at `1feb6108` | 115.8 |
| Full MP4, no text | — | 260.7 |
| Decode only, NV12 | — | 772.6 |

Both exports are byte-identical: `md5 540f516aeedc13b29cfbcdd3959eb164` for the
parent's file and for the queued one, 12,099,456 bytes each. The 1.77× on the
composited row therefore costs nothing in picture.

macOS M5 (Metal, VideoToolbox):

| Row | Parent `69aa1b31` | `02cd025c` |
|---|---|---|
| Full MP4, one text layer | 179.9 / 196.4 / 212.9 | 187.3 / 190.6 / 215.8 / 220.6 |
| Full MP4, one text layer, animated clip | — | 222.0 |
| Full MP4, no text | — | 228.1 |
| Render stage only, one text layer | — | 256.3 |
| Decode only, NV12 | — | 547.6 |

The two distributions overlap, so no change is claimed on this host: the Apple
GPU already finishes the conversion inside the frame's other work. The decoded
pictures are identical between the two revisions (`PSNR y:inf u:inf v:inf`);
only the bitstream bytes move run to run, because the VideoToolbox encoder is
not bit-exact.

Windows 11, Intel Graphics (Vulkan, Media Foundation):

| Row | Parent `69aa1b3` | `02cd025` |
|---|---|---|
| Full MP4, one text layer | 81.7 / 82.6 | 105.2 / 105.2 |
| Full MP4, one text layer, animated clip | — | 105.2 / 105.2 |
| Full MP4, no text | — | 136.5 / 138.0 |
| Render stage only, one text layer | — | 104.7 / 109.4 |
| Decode only, NV12 | — | 214.7 / 214.8 |

The queued readback is worth +27 % on the composited row here and keeps the
video-only row where it was. The render-stage-only row no longer has a queue to
hide behind, so it stays latency-bound at about 108 fps: that row measures one
serialised submit-wait round trip per frame and is not the export's ceiling.

### What the queue costs

The queue holds at most `EXPORT_RENDER_AHEAD` frames (2, `OR_EXPORT_RENDER_AHEAD`
overrides it) plus the one the render thread is building, so at most four
1080p NV12 buffers are in flight — about 25 MB with the pooled staging buffers.
A frame that is dropped without resolving waits for its own submission and
unmaps its buffer in `Drop`, so the pool can hand it out again.

## 2026-11-07 matrix — formats, profiles and stages at `480e0a53`

One revision, one fixture, one session per host: `--release`, 300 frames of
1080p `bench1080.mp4`, one text layer, idle machine, the product's own FFmpeg
prefix. Every row was produced by the same ignored Rust benchmark
(`export_media_benchmark`) so a row differs from its neighbour only in the
setting named beside it. `fps` is frames of output per second over the whole
pipeline; `render-only` measures the renderer without an encoder, `decode-only`
the decoder alone with `OR_BENCH_NV12=1`, and `encode-only` a blank NV12 frame
through the encoder.

macOS M5 (Metal, `h264_videotoolbox`):

| Row | fps |
|---|---|
| MP4 H.264/AAC, no text | 243.5 |
| MP4 H.264/AAC, one text layer | 240.6 / 237.3 |
| MP4, text plus animated clip | 239.3 |
| WebM VP9/Opus, one text layer | 53.0 |
| Matroska FFV1/PCM (lossless), one text layer | 52.0 |
| MP4, one text layer, Speed quality | 238.0 |
| MP4, one text layer, Quality quality | 236.6 |
| Render stage only, one text layer | 303.6 |
| Decode only, NV12 | 1390.4 / 1486.6 / 1460.5 |
| Decode only, RGBA | 412.7 |
| Encode only, blank NV12 | 280.8 |
| 4K source into 1080p, one text layer | 106.7 |
| 4K source into 4K, one text layer | 63.1 |

Debian 13, Intel UHD 770 (Vulkan, `h264_vaapi`), adapter
`Intel(R) UHD Graphics 770 (ADL-S GT1)`, Mesa 25.0.7:

| Row | fps |
|---|---|
| MP4 H.264/AAC, no text | 256.7 |
| MP4 H.264/AAC, one text layer | 193.1 / 195.5 |
| MP4, text plus animated clip | 196.0 |
| WebM VP9/Opus, one text layer | 25.4 |
| Matroska FFV1/PCM (lossless), one text layer | 32.6 |
| MP4, one text layer, Speed quality | 203.9 |
| MP4, one text layer, Quality quality | 198.5 |
| Render stage only, one text layer | 119.5 |
| Decode only, NV12 | 766.8 |
| Encode only, blank NV12 | 454.9 |
| 4K source into 1080p, one text layer | 45.1 |
| 4K source into 4K, one text layer | 47.9 |

Windows 11, Intel Graphics (Vulkan, Media Foundation), adapter
`Intel(R) Graphics (Intel Corporation, Vulkan)`:

| Row | fps |
|---|---|
| MP4 H.264/AAC, no text | 134.7 |
| MP4 H.264/AAC, one text layer | 103.8 / 105.7 |
| MP4, text plus animated clip | 104.2 |
| WebM VP9/Opus, one text layer | 10.8 |
| Matroska FFV1/PCM (lossless), one text layer | 14.3 |
| MP4, one text layer, Speed quality | 104.8 |
| MP4, one text layer, Quality quality | 103.4 |
| Render stage only, one text layer | 124.5 |
| Decode only, NV12 | 215.2 |
| Encode only, blank NV12 | 274.2 |

### What the matrix shows

- **The composited MP4 row is now within 25 % of the video-only row on macOS and
  Debian** (240.6 vs 243.5, 193.1 vs 256.7), where the same comparison was 2.2×
  apart on Debian before the queued readback. Windows remains the host where the
  text layer costs most relative to its own ceiling (103.8 vs 134.7).
- **Speed and Quality are the same pipeline at different encoder settings**, so a
  row that moves with quality is an encoder cost: macOS 238.0 vs 236.6, Debian
  203.9 vs 198.5, Windows 104.8 vs 103.4. Nothing in the render path is paying for
  quality.
- **VP9 and FFV1 are software encoders on all three hosts** and are the slowest
  rows everywhere. Windows is far slower than the other two on the same fixture
  (10.8 vs 53.0 on macOS, 25.4 on Debian); the cause is not yet measured — the
  candidate is the build's thread count on that host — so this is recorded as an
  open gap, not a claim about the codec.
- **Decode is not the ceiling anywhere**: 1390–1487 fps (NV12) on macOS, 766.8 on
  Debian, 215.2 on Windows, against 104–257 fps for the full pipeline.
- **4K costs what its pixels cost**: 63.1 fps 4K-in/4K-out on macOS, 47.9 on
  Debian. A 4K source downscaled to 1080p is *not* cheaper on Debian (45.1 vs
  47.9) because the decode and the render work on the source grid; on macOS it is
  cheaper (106.7 vs 63.1).

### Render-ahead depth, swept

`OR_EXPORT_RENDER_AHEAD` was swept on the composited MP4 row with everything
else fixed. Depth 1–4 is inside the run-to-run spread on both hosts, so the
shipped default of 2 is not a bottleneck and a deeper queue would only hold more
memory:

| Depth | macOS M5 | Debian UHD 770 |
|---|---|---|
| 1 | 235.6 | 196.1 |
| 2 (default) | 236.1 | 198.0 |
| 3 | 235.4 | 192.9 |
| 4 | 235.8 | 196.0 |
| 6 | 239.8 | — |

## Known limits

- The project rows below `97f4f25e` used `/tmp/or-bench/opaque.orproj`, a copy of
  a real project with its video clip opacity repaired to 100 %. With the stored
  opacity of 0.01 % the same project exports black at 178.9 fps, and the editor
  never warns about that.
- The Debian host composites on an Intel UHD 770 through Vulkan and encodes
  through `h264_vaapi`, so its rows describe that integrated device; they say
  nothing about an NVIDIA or AMD machine. Its text row is now explained: the
  composited path waits 13–14 ms per 1080p frame in the NV12 readback, while the
  Windows host pays a smaller readback cost on a different driver.
- Each host reports the adapter that served the benchmark
  (`OR_EXPORT_BENCH_ADAPTER`), so a row can be attributed to a device instead of
  assumed: macOS prints `Apple M5 (, Metal)`, Debian prints the Intel UHD 770 /
  Vulkan line above, and Windows prints `Intel(R) Graphics (Intel Corporation,
  Vulkan)` — so wgpu picked Vulkan on this Windows host rather than D3D12.
- Windows rows were taken on the Media Foundation encoder, not QSV or NVENC:
  the host has no QSV-capable display driver and no NVIDIA device. The candidate
  list still prefers `h264_nvenc` then `h264_qsv` when they exist.
- WebM and lossless are CPU-bound on every host: the encoders are software
  (libvpx VP9, FFV1) and neither takes a GPU frame. Windows WebM at 9.4 fps is
  roughly what its twelve threads buy next to Debian's twenty, not a missing
  encoder; its MP4 row still uses Media Foundation.
- `OR_BENCH_MODE=encode` measures the writer alone with blank NV12 frames. It is
  an encoder ceiling, not an export row; full-pipeline rows leave `OR_BENCH_MODE`
  unset.
- Rows are taken against the prefix each host links, which the Linux and Windows
  hosts now rebuild from the repository scripts instead of a hand-made prefix.
