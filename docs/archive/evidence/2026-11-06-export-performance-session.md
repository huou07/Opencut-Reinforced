# Receipt — export regression investigation and audio/VAAPI work, 2026-11-06

Local session record. It states what was observed on the three hosts on this
date and what was left open; current truth lives in
[export performance](../../reference/export-performance.md).

## Starting state

- `main` at `97f4f25e39328b64ca2f93b1dc6baee640f83d7e`, working tree clean,
  macOS host `Mac17,3` (Apple M5, 10 cores, 16 GB, macOS 27.2).
- Two local commits were added on branch `perf/export-audio-cache`:
  `e9b6d1c8` (audio pre-decode scales with the machine) and `05e4fe79`
  (VAAPI quality ladder). `main` was not moved or rewritten.
- Nothing was pushed, tagged, released or run through hosted CI.

## Regression question

The owner reported that export used to be much faster on this machine. The
answer, with measurements, is that the export path did not regress between the
2026-11-04 commit `9cddc7e1` and `97f4f25e`:

| Row (1080p, 300 frames, MP4) | `9cddc7e1` | `97f4f25e` |
|---|---|---|
| One clip, direct path | 226.2 fps | 242.8 / 247.4 fps |
| One text layer | 102.8 / 104.9 fps | 118.6 / 127.3 fps |
| 4K → 1080p, 150 frames | 106.2 fps | 109.0 fps |

Two measurement errors explained the earlier low readings: rows taken while a
`cargo` build was running (about half throughput), and rows taken before the
machine had settled (spread of up to 8 %).

## The slow real project

`/Users/meomeo/Documents/meomeo.orproj` stored `opacity.basis_points = 1` on its
video clip. Export therefore refused the direct NV12 path, composited an
effectively transparent layer and wrote 7318 black frames (content_end came from
a 243.9 s audio clip under a 73 s video). Repairing the opacity alone moved the
same project from 178.9 fps to 215.3 fps and from a black 7 MB file to a real
87 MB file.

## Optimisations that landed locally

- Audio pre-decode: workers were clamped to four with an in-flight budget sized
  for four. Now every usable core, budget sized from the worker count, and each
  clip's footprint reserved before decoding. 12 clips: 0.456 s → 0.158 s.
- VAAPI: the driver logged "No quality level set; using default (20)" and both
  quality preferences cost the same. Speed is now 275–286 fps at 8.3 MB and
  Quality 256–261 fps at 12.1 MB on the Debian host.

## Open items carried forward

- A text layer forces the whole export off the direct NV12 path: render falls
  from 1279 to 130 fps on macOS, 700 to 36 on Debian and 190 to 33 on Windows.
  Compositing text over NV12 frames on the GPU is the next large render win.
- The Windows packaged FFmpeg has no H.264 encoder, so MP4 export is refused
  there. `h264_mf` in the Windows configure args is the LGPL answer.
- Reduce Noise is reachable only from the media library (and the CLI). Putting it
  on a selected timeline clip needs a Rust command that denoises, adds the
  derived media and swaps the clip reference in one undoable change; video clips
  additionally need video-embedded audio to exist in OR, which it does not.
