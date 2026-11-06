# Receipt — GPU text compositing, per-OS H.264, and timeline Reduce Noise, 2026-11-06

Local session record. It states what was built, run and measured on the three
hosts on this date, and what was left open; current truth lives in
[export performance](../../reference/export-performance.md),
[media capability matrix](../../reference/media-capability-matrix.md) and
[effects and audio](../../features/effects-audio.md).

## Starting state

- `main` at `7bb42803`, working tree clean. Work happened on local branch
  `perf/gpu-text-composite`; `main` was not moved, rewritten or merged.
- Nothing was pushed, tagged, released or run through hosted CI. All three hosts
  received the branch as `git bundle` files over SSH, into detached worktrees.
- Hosts: macOS `Mac17,3` (Apple M5, 10 cores, 16 GB), Debian 13 VM over
  `ssh debian` (i5-13500T, 20 threads, no display GPU), Windows 11 26200 over
  `ssh win11` (Intel Core 5 120U, 12 threads).

## What changed

| Commit | Change |
|---|---|
| `75796d33` | NV12 video layers composited on the GPU beside RGBA text layers, a pooled compositor, and keyed text uploads |
| `cddb1b8b` | `OR_BUILD_JOBS` widens the native FFmpeg builds instead of a hard `-j2` |
| `17343798` | Reduce Noise as canonical clip state, DSP, inspector field, preset chips, bridge, Dart |
| `8dc2e3ff` | Windows-only fixture fields for the new clip state |
| `60406b1b` | Gate moved to the decode boundary with per-clip state; CLI `--noise-reduction-db`; three-host documentation |
| `ba0034ff` | Gate reference decays one decibel a second instead of nine |

`scripts/build_linux_ffmpeg.sh` is new: the Linux build previously existed only
inside the CI workflow, which is why a Debian host could not reproduce it.

## Results — render and export

Interleaved A/B against the parent commit on the same host in the same session;
absolute numbers alone are not comparable between sessions because host load
changed them by up to 2×.

| Host | Row (1080p, 300 frames) | Before | After |
|---|---|---|---|
| macOS | render, one clip | 1288 fps | 1278 / 1342 / 1355 fps |
| macOS | render, one text layer | 126.6 fps | 283 / 285 / 313 fps |
| macOS | MP4 quality, one clip | 241.8 fps | 216.8 / 234.1 fps (final revision) |
| macOS | MP4 quality, one text layer | 112.5 fps | 167.0 / 181.9 fps (final revision) |
| Debian | render, one clip | 700.2 fps | 698.9 fps |
| Debian | render, one text layer | 34.4 fps | 49.2 fps |
| Debian | MP4 quality, 150 frames, one text layer | 34.4 fps | 62.5 fps |
| Windows | render, one clip | 189.7 fps | 188.2 fps |
| Windows | render, one text layer | 32.9 fps | 91.3 / 99.4 / 101.8 fps |
| Windows | MP4 quality, one clip | refused | 121.5 fps |
| Windows | MP4 quality, one text layer | refused | 74.7 / 78.2 / 79.4 fps |

Encoders, read from stderr and confirmed with `ffprobe` on the written file
rather than from the profile name: macOS `h264_videotoolbox`, Debian
`h264_vaapi` (after `Could not dynamically load CUDA`), Windows Media
Foundation (Constrained Baseline stream). MP4 is H.264 + AAC on all three.

## Results — application builds and runs

| Host | Build | Run |
|---|---|---|
| macOS | `flutter build macos --debug` PASS | launched, stays running, window present PASS |
| Debian | `flutter build linux --debug` PASS | launches under Xvfb, window `1280x720` titled "Opencut Reinforced", shell renders (`0x141414` background) — PASS for start, interactive editing UNVERIFIED |
| Windows | `flutter build windows --debug` PASS (`or_app.exe`, bridge DLL, Flutter DLL) | **BLOCKED**: no interactive desktop reachable over SSH — the process starts in the SSH session without a window, `CopyFromScreen` raises `Win32Exception`, and a scheduled-task launch could not be registered |

## Results — Reduce Noise

- Canonical state: `AudioSettings.noise_reduction_decibels`, 0..=24, absent
  means 0, validated in core; one `timeline.clip.update` command per change, so
  one revision and one undo step (observed 45 → 46 on a real project).
- Inspector: `Reduce noise (dB)` field plus No reduction / 6 / 12 / 18 dB chips
  (keys `inspector-noise-preset-<n>`); `flutter analyze` clean, six widget tests
  pass.
- CLI: `or timeline set-clip-audio --noise-reduction-db N` writes the same
  command; 44 CLI tests pass.
- DSP: a peak-keyed expander, gate state per clip held across blocks in the
  streaming mixer and over the whole clip in the export cache.
- **End-to-end export proof**: a 10 s fixture (3 s of 1 kHz tone, then 7 s of
  pink noise at 0.02) on an audio track, exported to MP4 at 18 dB reduction:
  tone region unchanged at −24.09 dBFS, hiss region −51.4 → −66.3 dBFS
  (astats via Homebrew ffmpeg on the AAC track). This is the check that caught
  the decay bug: the first build reduced the hiss by only 0.9 dB.
- Video-embedded audio is **absent from the product**: a clip on a video track
  contributes no sound, so reducing the audio of a video clip cannot be
  implemented until that pipeline exists. The media-library "clean audio" path
  still writes a new WAV and is not the timeline feature.

## Bugs found and fixed during the session

- A BT.709 inverse in the NV12 layer shader against BT.601 planes decoded by
  libswscale: PSNR 24.9 dB, a visible colour shift. Now BT.601 (42.9 dB luma,
  40.6 dB average), documented with the caveat about missing colour metadata.
- The gate reference decayed ≈8.3 dB/s while its comment claimed 0.9 dB/s; the
  end-to-end export exposed it. Now derived from a documented 1 dB/s constant.
- Windows and Debian had both "no H.264 encoder" from hand-built prefixes, not
  from the operating systems. Both now build from repository scripts that
  assert the encoders in `config_components.h`.
- `git bundle create <sha>..<sha>` produced empty bundles; bundles must name a
  ref (`<sha>..branch`), and the range must start at a commit the receiving
  clone already has.

## Not done, and why

- The 20-repo technology evaluation was re-read and a dated re-check was added
  to [technology review](../../reference/technology-review.md); no new runtime
  dependency was adopted. A per-library table was not rewritten because the
  existing review already carries verdicts and nothing measured this week
  changed them.
- Windows UI journeys, Debian UI journeys, and WebM/lossless rows on the final
  revision were not re-run as UI flows. Benchmarks cover MP4/WebM/lossless on
  the harness path; MP4 rows were re-measured at the final revision on macOS and
  Debian.
- `qsv`, `nvenc`, `amf` and VideoToolbox acceleration paths were not exercised
  on hardware that carries the corresponding devices (the Windows host has no
  QSV-capable display driver and no NVIDIA device, the Debian host has no
  display GPU).
