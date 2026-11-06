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
path.

## Machines

| Host | CPU | GPU path | OS | Packaged FFmpeg |
|---|---|---|---|---|
| MacBook Air | Apple M5, 10 cores, 16 GB | Metal / VideoToolbox | macOS 27.2 | 8.1.3, `h264_videotoolbox` |
| Debian | Intel Core i5-13500T, 20 threads, 15 GB | Intel UHD 770 / VAAPI | Debian 13, kernel 7.2.6 | 8.1.3 with `h264_vaapi`, `h264_nvenc`, `libx264` |
| Windows 11 | Intel Core 5 120U, 15.7 GB | Intel Graphics / D3D12 | Windows 11 | 8.1.3, no H.264 encoder at all (see below) |

## 2026-11-06 rows

All rows were measured at `97f4f25e` plus the two local commits named below, in
a `--release` build, on an idle host.

macOS (frames per second of the whole export, 300 frames unless stated):

| Workload | Before (`9cddc7e1`, 2026-11-04) | Now |
|---|---|---|
| 1080p MP4, one clip | 226.2 | 242.8 / 247.4 |
| 1080p MP4, one text layer | 102.8 / 104.9 | 118.6 / 127.3 |
| 4K → 1080p, 150 frames | 106.2 | 109.0 |
| Real project, 7318 frames (`/tmp/or-bench/opaque.orproj`) | — | 215.3 |

macOS stage ceilings: NV12 decode 504 fps, RGBA decode 176 fps, render
1279 fps without text and 130 fps with one text layer, VideoToolbox H.264 blank
frames 280.9 fps.

Debian (VAAPI H.264, 300 frames):

| Workload | Before the quality ladder | Speed | Quality |
|---|---|---|---|
| 1080p MP4 | 252.0 / 274.9 | 275.1 / 286.0 | 261.1 / 256.0 |
| Output bytes | 12 168 267 | 8 315 449 | 12 122 320 |
| 1080p + text | — | 34.4 | 34.4 |
| 4K → 1080p, 150 frames | — | 47.7 | — |
| Render only, no text | — | 700.2 | 700.2 |
| Render only, one text layer | — | 36.0 | 36.0 |

Windows 11, WebM VP9/Opus (H.264 is unavailable there, see below):

| Workload | fps |
|---|---|
| 1080p WebM | 8.4 |
| 1080p WebM + text | 8.0 |
| 4K → 1080p WebM, 150 frames | 6.8 |
| Render only, no text | 189.7 |
| Render only, one text layer | 32.9 |

## What the rows prove

- Export is encode-bound on macOS. The full pipeline at 243 fps sits below the
  280 fps VideoToolbox ceiling and far below decode, so the encoder is the
  stage to attack next, not the compositor.
- A text layer costs the whole direct path. Adding one text clip drops render
  from 1279 to 130 fps on macOS, from 700 to 36 fps on Debian and from 190 to
  33 fps on Windows. The cause is not the text rasterizer (it caches) but the
  path switch: any active text clip makes `covers_canvas_clip_at` refuse the
  direct NV12 path, so every frame is decoded to RGBA on the CPU and uploaded.
  Compositing text over an NV12 frame on the GPU is the largest remaining
  render win on all three hosts.
- VAAPI had no quality ladder. Before the change the driver answered
  "No quality level set; using default (20)" and a Speed export cost the same as
  a Quality one. With the ladder, Speed is 8 % faster and writes 32 % fewer
  bytes, and Quality keeps the previous bitrate.
- The audio pre-decode was capped at four workers. On a twelve-clip timeline the
  old cap took 0.456 s against 0.158 s at twelve workers, and the cost is paid
  before the first frame.
- Windows cannot export MP4. The packaged prefix at `C:/orb/tmp/ffmpeg-8.1.3-install`
  lists 18 encoders: `ffv1`, `libvpx-vp9`, `libopus` and the PCM family. No
  `h264_mf`, `h264_nvenc`, `h264_qsv` or `libx264`, so the MP4 profile refuses
  with "this FFmpeg build carries no H.264 encoder". The configure recipe
  enables a hardware H.264 encoder for macOS only
  (`--enable-videotoolbox --enable-encoder=h264_videotoolbox`); the Linux and
  Windows jobs enable none. Media Foundation encoding is the LGPL-compatible
  Windows answer and needs a rebuilt prefix before that row can be measured.

## Known limits

- The Debian rows ran against the local prefix named in that host's
  `~/.cargo/config.toml`; a row measured against a system FFmpeg proves nothing
  about the packaged build.
- Windows rows are WebM only until the prefix carries an H.264 encoder.
- The project rows use the fixture `/tmp/or-bench/opaque.orproj`, a copy of a
  real project with its video clip opacity repaired to 100 %. With the stored
  opacity of 0.01 % the same project exports black at 178.9 fps, and the
  editor never warns about that.
