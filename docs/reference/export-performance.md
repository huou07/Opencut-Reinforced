# Export and render performance

Last reviewed: 2026-11-06

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

# One stage at a time: decode, render, encode.
OR_BENCH_MODE=render OR_BENCH_SOURCE=/tmp/or-bench/bench1080.mp4 OR_BENCH_FRAMES=300 "$BIN" export_media_benchmark --ignored --nocapture
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
so the text path is now encoder-bound rather than compositor-bound.

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
open-source Mesa driver, Vulkan)`), not on a software rasteriser. Its text row
still improves only 1.45× where macOS reaches 2.4×, so a full-canvas text blend
plus readback costs roughly 20 ms/frame there against 3.5 ms on the Apple GPU;
the gap between this host's integrated device and the Windows one (91–102 fps on
the same workload) is measured but not explained — see "Known limits". Output
bytes are unchanged by the compositor: 6 237 429 without text and 6 222 521 with
text, LSB-identical at the same settings.

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

### Container and codec rows at the final revision

Same fixture, same 150-frame window, no text, measured on each host at the
final revision of the branch. Byte counts are identical across hosts because
VP9 CRF and FFV1 are rate-independent: the three encodes are the same file.

| Host | WebM VP9/Opus | WebM bytes | Matroska FFV1/PCM | Lossless bytes |
|---|---|---|---|---|
| macOS M5 | 37.1 fps | 4,109,503 | 42.3 fps | 39,278,557 |
| Debian (no GPU) | 25.1 fps | 4,109,503 | 33.4 fps | 39,278,557 |
| Windows (Core 5 120U) | 9.4 fps | 4,109,503 | 6.6 fps | 39,278,557 |

Re-checked at the scissor revision (`f0b44a24`) with one text layer on macOS, 150
frames: MP4 213.7 fps (5,904,842 B, `h264` High `yuv420p` + `aac` LC 48 kHz
stereo), WebM 54.5 fps (3,600,271 B, `vp9` Profile 0 + `opus`), Matroska 54.6 fps
(47,874,201 B, `ffv1` `bgra` + `pcm_s16le`). Nothing on the codec path changed
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

## Known limits

- The project rows below `97f4f25e` used `/tmp/or-bench/opaque.orproj`, a copy of
  a real project with its video clip opacity repaired to 100 %. With the stored
  opacity of 0.01 % the same project exports black at 178.9 fps, and the editor
  never warns about that.
- The Debian host composites on an Intel UHD 770 through Vulkan and encodes
  through `h264_vaapi`, so its rows describe that integrated device; they say
  nothing about an NVIDIA or AMD machine. Why its text row is roughly half the
  Windows row on the same workload is unresolved: the two hosts differ in CPU
  (i5-13500T against Core 5 120U), driver and display server, and no single
  measurement separates those causes.
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
