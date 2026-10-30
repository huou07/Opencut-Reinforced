# Media test fixtures

`big_buck_bunny_1080p_h264_aac.mp4` contains a half-second stream-copy excerpt
of Big Buck Bunny (2008) video, © Blender Foundation / www.bigbuckbunny.org,
licensed under Creative Commons Attribution 3.0. The source rendition is the
30-second 1080p H.264 test file at the immutable
[`video-media-samples` revision `997cb58f16bc3433652506910734be75bc64d768`](https://github.com/chthomos/video-media-samples/tree/997cb58f16bc3433652506910734be75bc64d768).
The source file's Git blob is `cfbd184a4b6a26dcbbbfe571cdeb87600d358d68`.
The upstream repository records the film's attribution and license in its
[fixture license file](https://github.com/chthomos/video-media-samples/blob/997cb58f16bc3433652506910734be75bc64d768/LICENSE.md).
The source soundtrack was removed because its separate
[release](https://freemusicarchive.org/music/Jan_Morgenstern/Big_Buck_Bunny)
is noncommercial/no-derivatives. The fixture's 5.1 AAC track is generated
silence, used only to exercise stream decode and surround-to-stereo downmix.
The fixture has 1920×1080 H.264 video at 24 fps and 5.1 AAC audio at 48 kHz.
Its SHA-256 is `7b88ad19dc59986f59a59033e3d7a51d16d5cb3e454796ae6556c61e642b057a`.

The packaged desktop product journey fetches the same 30-second source MP4
from the immutable upstream revision above and verifies its size
(`22718509` bytes) and SHA-256
`07b756a4c7b481829776645c153167ca14c9df802ddbea7dffda3d817aa5261a` before
use. The journey remuxes only its H.264 video with the fixture's generated
silent AAC track into Matroska; the downloaded original soundtrack is never
used, and the source MP4 is not committed or shipped with OR.

`phone_portrait_90_h264_aac.mp4` is a one-second stream-copy excerpt of the
Samsung Galaxy S9 H.264/AAC portrait-orientation sample in the CC0
[`shotstack/test-media` repository](https://github.com/shotstack/test-media/tree/b790a2b72325feecd189291c52663ed8123b1839/orientation).
The selected source is `h1920_w1080_f30_a9-16_r90.mp4`, Git blob
`b66d197c4a92dc1300fc035bc82b81c50eee1be1`. The excerpt was made with
`-noautorotate` and stream copy, retaining its 90-degree display matrix while
stripping global metadata, including device location. It contains 1920×1080
H.264 video and stereo 48 kHz AAC; SHA-256 is
`9e7618482872476a801271b847bedfca1cbf1722b0f7717415c52c960f2d7811`.

`cc0_music_excerpt.mp3` is a three-second stream-copy excerpt of “Try me!” by
iamoneabe, downloaded from [OpenGameArt](https://opengameart.org/content/try-me)
under CC0 / public domain. The source file SHA-256 is
`9dd9c59a16cfc9a6c991c56448c5f33ed1af79f8720dc0c5e91166fded95e4b5`; the
excerpt SHA-256 is
`812b747914fccbf1c59909954b92baec87ecec272223d5898f49e9661acc9f9d`. A
generated 64×64 PNG cover image is embedded as an ID3 attached-picture stream
to verify that desktop and SAF probing keep it out of the editable timeline.
The fixture retains 44.1 kHz stereo MP3 audio and is used for import, decode,
and packaged Android SAF/audio-playback acceptance.

`tiny_still.png`, `tiny_still.jpg`, and `tiny_still.webp` are 16×16 lossless or
high-quality encodings of the generated FFmpeg `testsrc2` pattern. They contain
no third-party media and exercise PNG, JPEG, and WebP import, probing, decoding,
and the packaged helper profile. SHA-256 values are `9aa69e7a7d64614c336f1c8d3f5fbd6b0f92987a780e8ad29e87ce2b8b27cb43`,
`62e0075e9962ffd61ae10fa5c28467aaae8d290e4f2681a8ba53cb8134b8ecee`, and
`9b596d8df41b18cd21c5e39ead637e75382810583f44a59ad4233a6ec741c6af`, respectively.

The remaining tiny fixtures are generated specifically for deterministic
correctness tests and contain no third-party media.
`tiny_offset.mkv` combines `tiny.wav` with a one-second generated test-pattern
video and moves the PCM audio stream 250 ms later while keeping the media
timeline origin at zero. It verifies transcription handoff preserves a real
audio stream offset as leading silence. SHA-256:
`8ffe039d7cd3b44b5582af31b655e16d258e70f988a86cc0e446181b02bd57cc`.
`tiny_gap.mkv` uses the same generated video and PCM source, dropping real
audio packets from the middle while retaining source PTS. It verifies a
forward timestamp gap becomes silence. SHA-256:
`858c4abe961b68b6960c7fe230bc7f8f1caf6701eab293a5bc6998c2dd9c69fe`.
