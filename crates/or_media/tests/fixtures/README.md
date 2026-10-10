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

The remaining tiny fixtures are generated specifically for deterministic
correctness tests and contain no third-party media.
