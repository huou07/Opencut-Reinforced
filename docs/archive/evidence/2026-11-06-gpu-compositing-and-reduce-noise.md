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
| Windows | `flutter build windows --debug` PASS (`or_app.exe`, bridge DLL, Flutter DLL) | launches in the SSH session and drives a journey that decodes no images: `text-journey.json` PASS 23 steps (`ANGLE_DEFAULT_PLATFORM=warp`). Image-backed journeys stay **BLOCKED**: the first decoded image throws `Exception: No Impeller context is available` (session 0 has no GPU-backed window station) |

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
- **Real-app interaction proof** (`scripts/ui_scenarios/reduce-noise.json`,
  `python3 scripts/or_ui.py --target macos run …` → PASS): the inspector is a
  lazy list, so the scenario scrolls `inspector-scroll` into view, reads the
  field at `0`, taps the 18 dB chip, reads `18` back and undoes once to read
  `0` again — three `expect_value` rows in the harness report, at the same
  revision as the widget and CLI tests.
- Video-embedded audio is **absent from the product**: a clip on a video track
  contributes no sound, so reducing the audio of a video clip cannot be
  implemented until that pipeline exists. The media-library "clean audio" path
  still writes a new WAV and is not the timeline feature.

## Copies, adapters, and what the rows attribute to a device (later still)

- The direct export path handed the encoder a **copy** of the decoder's NV12
  frame every frame. `Nv12Planes` now holds either owned planes or a window into
  the lease's shared `Arc<[u8]>`, so a frame reaches the writer without the copy.
  Interleaved A/B, 300 frames, identical output bytes: macOS 244.8 → 247.8 fps
  (median, ≈ +1.2 %), Debian 256.1 → 260.5 fps (≈ +1.7 %). Recorded as a small
  win, not a headline.
- The benchmark now prints the adapter that served it
  (`OR_EXPORT_BENCH_ADAPTER`). macOS prints `Apple M5 (, Metal)`; Debian prints
  `Intel(R) UHD Graphics 770 (ADL-S GT1) (Intel open-source Mesa driver,
  Vulkan)`. This **corrects an earlier note in this receipt**: the Debian host is
  not a software rasteriser, so its 49.8 fps text row is an integrated-GPU
  number, and the earlier "llvmpipe-class" explanation was wrong. Why that row
  is about half the Windows row on the same workload stays unresolved and is
  recorded as an open question rather than a cause.
- A compositor identity bug was found while reviewing this work: the text-upload
  cache keyed on a rasterizer-local id, but the pool is process-wide, so a second
  project's first title could draw the first project's pixels. Fixed with a
  process-wide counter; the regression test fails when the counter is forced to
  collide.

## Video clips carry their own audio (later in the session)

A video clip's soundtrack used to be ignored by the program builder, so Reduce
Noise could only reach an audio track. `VisualSettings.audio` now holds the same
`AudioSettings`, the program pushes an audio clip for a video clip whose media has
an audio stream, the inspector shows the audio fields beside the picture fields,
and a visual update carries the audio half in the same command, so one Apply is
one revision and one undo step.

Evidence, all local:

- Fixture: 10 s 640×360 H.264 + mono AAC (3 s tone, 7 s hiss at 0.02).
- CLI: `or timeline set-clip-audio --noise-reduction-db 18` on the video clip
  printed "Updated timeline clip_id … at revision 4"; the saved project is
  `schema_version 11` with `audio.noise_reduction_decibels: 18`.
- Export (bench harness, MP4 quality 300 frames): before `audio=1 bytes=264158`,
  after `audio=1 bytes=264353`; `volumedetect` on the two MP4s: tone −24.1 dB
  before and after, hiss −51.4 dB → **−66.8 dB**.
- Schema: version 11 with an optional `audio` block; version 9 and 10 files open
  unchanged (a real 12-audio-clip version 9 file was opened, edited and rewritten
  as version 11).
- UI: `scripts/ui_scenarios/video-clip-audio.json` imports the repository's
  `tiny_h264_aac.mp4` as a video clip, scrolls the inspector to the audio fields,
  picks 12 dB, and one `cmd+z` returns the field to 0 — `OR_UI=PASS` on macOS.
- Final-revision export rows (`0eb453c8`, same fixture and profile): macOS
  234.1 fps / 223.6 with text, Debian 232.0 fps (6,237,429 bytes, h264 High +
  aac LC), Windows 111.5 fps (6,055,754 bytes, h264 Constrained Baseline + aac
  LC) with the writer logging `[h264_mf @ …] MFT name: 'H264 Encoder MFT'`.
- Three-host proof of the soundtrack path, same fixture and project at
  `0eb453c8` (300 frames, MP4 quality, `OR_PROJECT_BENCH`): macOS 264,158 →
  264,353 bytes with hiss −51.4 → −66.8 dB; Debian 257,928 → 257,932 bytes with
  hiss −51.3 → −66.7 dB; Windows 261,515 → 261,566 bytes with hiss −51.3 →
  −66.8 dB. Tone stays −24.1 dB on all three hosts. The byte counts differ per
  host because each uses its own H.264 encoder (VideoToolbox, VA-API, Media
  Foundation), which is why the level measurement, not the file size, is the
  evidence.
- Host trap: the shipped Windows FFmpeg has neither the `null` muxer nor the
  `volumedetect` filter, so the Windows levels were measured by decoding each
  MP4 to PCM WAV with the bundled ffmpeg and computing RMS in Python; the
  Debian build does have both.
- Cross-host confirmation at `0eb453c8` (the commit after the feature):
  `cargo test -p or_core -p or_audio -p or_app_bridge --lib` gives 378 / 18 / 54
  passed on macOS and Debian and 376 / 18 / 50 on Windows (the bridge count
  differs because four tests are host-gated), `cargo test -p or_cli` gives
  44 + 4 + 4 on macOS and Debian, and `flutter build {macos,linux,windows}
  --debug` succeeds on all three hosts at that revision. The macOS bundle was
  relaunched and the new scenario re-ran to PASS; the Windows GUI stays BLOCKED
  (no interactive desktop over SSH) and Debian editing stays UNVERIFIED.

## Bugs found and fixed during the session

- A BT.709 inverse in the NV12 layer shader against BT.601 planes decoded by
  libswscale: PSNR 24.9 dB, a visible colour shift. Now BT.601 (42.9 dB luma,
  40.6 dB average), documented with the caveat about missing colour metadata.
- The compositor's text-upload cache took its identity from the rasterizer, but
  the texture pool belongs to the one process-wide `RenderDevice` while a
  rasterizer belongs to a preview session. Opening a second project would have
  re-used the first project's first title as a cached upload and drawn the wrong
  pixels. The identity is now process-wide, and a regression test composites
  "Left" then "Right" from two rasterizers and requires the second upload (it
  fails when the counter is forced to collide).
- The gate reference decayed ≈8.3 dB/s while its comment claimed 0.9 dB/s; the
  end-to-end export exposed it. Now derived from a documented 1 dB/s constant.
- Windows and Debian had both "no H.264 encoder" from hand-built prefixes, not
  from the operating systems. Both now build from repository scripts that
  assert the encoders in `config_components.h`.
- `git bundle create <sha>..<sha>` produced empty bundles; bundles must name a
  ref (`<sha>..branch`), and the range must start at a commit the receiving
  clone already has.

## Blending only the ink (later in the session)

The text layer is a full-canvas RGBA buffer that is empty almost everywhere, yet
the compositor shaded all 2.07 M pixels for it. `TextInkBounds` now reports the
pixels with alpha, `RgbaVideoLayer::with_scissor` carries it, and `compose_into`
clips the draw to that rectangle (and drops a layer with no ink). It applies only
to an untransformed, uncropped layer.

Interleaved against `f5465b40`, 300 frames: macOS render-only with one text layer
219.5 / 246.2 / 247.9 → 385.2 / 388.8 / 374.9 fps; macOS MP4 quality 150 frames
188.4 / 204.7 → 215.5 / 221.2; Debian render-only 50.4 / 49.7 / 50.0 → 52.8 /
52.3 / 52.2, and its export rows (60.6 / 75.9 → 89.6 / 76.7) are too noisy on
that VM to claim. The macOS gain is the fill-rate win the 720p/1080p diagnostic
predicted; the Debian near-miss says its composited path costs its ~19 ms
somewhere other than text fragments, which is recorded as unresolved.

## Admission gate on Debian at the scissor revision

`bash scripts/acceptance.sh --timeout 900` in `~/or-wt-perf` at `f0b44a24` wrote
`.hub/gate-report.json` with `covered_gates: ["source-integrity"]`, `skipped: []`
and eight `not_run` lanes: the performance budget ("a benchmark row could not be
measured on this host"), `or_render`, `or_audio`, `or_ai`, `or_whisper_provider`
(each "hosted-CI authority" / "no audio device on this host"), and the hosted
`platform-verification`, `release-gate` and `packaging` gates. So the gate agrees
with this receipt's own boundary: the local lanes pass, and the GPU, audio-device,
packaging and cross-platform lanes stay UNVERIFIED here.

## Verification sweep at the scissor revision

Run on the macOS host at `ceb03031`, after every change in this receipt:

| Check | Command | Result |
| --- | --- | --- |
| Rust libraries | `cargo test -p or_core -p or_audio -p or_media -p or_render -p or_runtime -p or_app_bridge -p or_cli --lib` | 549 passed, 0 failed, 9 ignored (`or_core` 379, `or_render` 51, `or_app_bridge` 56 + 8 ignored, `or_media` 23, `or_runtime` 22 + 1 ignored, `or_audio` 18) |
| Dart widget tests | `flutter test` in `apps/or_app` | 513 passed, 0 failed |
| UI journey, captures and noise presets | `python3 scripts/or_ui.py --target macos run scripts/ui_scenarios/reduce-noise.json` | PASS, 34 steps |
| UI journey, a video clip's own sound | `… video-clip-audio.json` | PASS, 54 steps |
| UI journey, clip speed | `… clip-speed.json` | PASS, 45 steps |
| Repository gate | `bash scripts/check-repo.sh` | PASS, 894 tracked files |

The Debian equivalent is the admission gate recorded above (`source-integrity`
covered, the other eight lanes named as not run). The Windows equivalent is the
bench harness plus the `or_core` ducking tests, its GUI journeys remaining
BLOCKED by the session-0 window station.

## Not done, and why

- The 20-repo technology evaluation was re-read and a dated re-check was added
  to [technology review](../../reference/technology-review.md); no new runtime
  dependency was adopted. A per-library table was not rewritten because the
  existing review already carries verdicts and nothing measured this week
  changed them.
- Windows journeys that decode an image stay blocked, and the reason is measured
  rather than inferred: the SSH session is Windows **session 0** while
  `explorer.exe` and `dwm.exe` live in session 1, so the harness can launch
  `or_app.exe` and the first ten scenario steps run, but the first decoded image
  throws `Exception: No Impeller context is available` from the image resource
  service. `ANGLE_DEFAULT_PLATFORM=swiftshader` stops the app from starting at
  all; `warp` reaches the same image failure. There is no GPU-backed window
  station in session 0 to give the engine.
- Windows is not blocked as a *host*: a journey that decodes no images now
  passes there. `scripts/ui_scenarios/text-journey.json` creates a project, adds
  a text track and a title through the text dialog, selects the clip and undoes
  it — PASS 23 steps on macOS, Debian (Xvfb, software GL) and Windows (session 0,
  `ANGLE_DEFAULT_PLATFORM=warp`). So project creation, track creation, the text
  dialog, clip selection and undo are exercised on all three hosts; only the
  media thumbnail path needs a window station with a GPU context.

## Ducking under a video track (later in the session)

`timeline.clip.duck` used to refuse a voice track that was not `TrackKind::Audio`,
so a talking-head video could not duck the music bed even though its clip now
carries a soundtrack. The command accepts an audio or a video voice track; the
music track stays audio because ducking rewrites its gain.

Proof on macOS: project `duck2.orproj` (silent 1080p video 0–10 s, 20 s music
bed 0–20 s), `or timeline duck-clip --track …3333 --clip …285c13dd --voice-track
…2222 --duck-db -12` split the bed at 10 s with gains `[−12000, 0]`. Exported
twice (600 frames, MP4 quality): before `-44.1 dBFS` in both windows; after
`-56.1 dBFS` from 1–9 s (exactly the requested −12 dB) and `-44.1 dBFS` from
11–19 s. `cargo test -p or_core --lib` 379 passed, including
`a_video_track_voice_ducks_the_music_under_its_own_soundtrack`.
