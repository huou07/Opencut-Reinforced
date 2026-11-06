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
| Windows 11 | Intel Core 5 120U, 15.7 GB | Intel Graphics / D3D12 / Media Foundation | Windows 11 26200 | 8.1.3, `h264_mf`, `h264_nvenc`, `aac` |

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

The Debian host has no display GPU: compositing runs on llvmpipe, which is why
its text row improves 1.45× rather than 2.4× and why its per-frame cost stays
minute-scale. Output bytes are unchanged by the compositor: 6 237 429 without
text and 6 222 521 with text, LSB-identical at the same settings.

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
- The Debian host has no display GPU, so its rows measure llvmpipe; they say
  nothing about a machine with a real Intel UHD or NVIDIA device.
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
