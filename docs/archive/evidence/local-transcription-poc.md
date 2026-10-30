# Local transcription integration proof

This is a narrow upstream feasibility experiment, not an OR capability or
performance acceptance claim.

## Source and model identity

- Upstream: [`ggml-org/whisper.cpp` v1.9.4](https://github.com/ggml-org/whisper.cpp/tree/v1.9.4), tag commit `927cfce34f31707e17f2bff35c349632fb9e2c3a`.
- Build: macOS arm64 Release, `whisper-cli`, CMake 4.4.3 and Apple Clang 21; CMake detected Metal and Accelerate. The source remains outside the OR repository.
- Input: upstream [`samples/jfk.wav`](https://github.com/ggml-org/whisper.cpp/blob/v1.9.4/samples/jfk.wav), SHA-256 `59dfb9a4acb36fe2a2affc14bacbee2920ff435cb13cc314a08c13f66ba7860e`.
- Optional model: `ggml-tiny.en.bin`, 77,704,715 bytes, downloaded from the `ggerganov/whisper.cpp` Hugging Face repository at revision `5359861c739e955e79d9a303bcbc70fb988958b1`; SHA-256 `921e4cf8686fdd993dcd081a5da5b6c365bfde1162e72b08d75ac75289920b1f`.
- The [Hugging Face model repository](https://huggingface.co/ggerganov/whisper.cpp) labels this artifact MIT licensed; the [OpenAI Whisper model card](https://github.com/openai/whisper/blob/main/model-card.md) warns that recognition may hallucinate and varies by language and accent. Keep model license and provenance distinct from runtime-code provenance. The model is not bundled.

## Observed behavior

The upstream CLI transcribed the 10.76-second English fixture into two timed
segments:

| Start | End | Text |
|---:|---:|---|
| 0 ms | 7,960 ms | “And so my fellow Americans ask not what your country can do for you” |
| 7,960 ms | 10,760 ms | “ask what you can do for your country.” |

The first invocation, including model and Metal initialization, took 13.67
seconds on the current Apple Silicon host. This is one short fixture and one
host measurement; it does not establish typical speed, language quality,
resource bounds, responsiveness, or Android, Windows or Linux behavior.

The CLI's full JSON output includes token timestamps and token probabilities,
so a provider can produce millisecond timing and an explicitly uncalibrated
confidence estimate. OR must still turn those results into readable caption
segments and let users review and edit the proposal before applying it through
`timeline.captions.import` with the captured project revision.

On 2026-10-12, OR source commit
`5212c9e6f9fdbae51a681d0a299c90421a14c57e` was built in Release mode and its
sidecar was exercised over the real newline-delimited protocol on macOS arm64
with this exact pinned CLI, model, and upstream fixture. It returned one
SHA-256/size-verified proposal containing the expected JFK transcript and shut
down through the protocol.
This verifies the source-sidecar integration, not the Flutter application,
model-download UX, a clean installed package, multilingual quality, or
cross-platform performance.

## Reproduction

Build the pinned upstream source with:

```sh
cmake -S whisper.cpp -B whisper.cpp/build \
  -DCMAKE_BUILD_TYPE=Release \
  -DWHISPER_BUILD_TESTS=OFF \
  -DWHISPER_BUILD_EXAMPLES=ON \
  -DWHISPER_BUILD_SERVER=OFF \
  -DGGML_NATIVE=OFF
cmake --build whisper.cpp/build --config Release --target whisper-cli -j 8
```

Then run `whisper-cli` with the pinned model, upstream fixture, and
`--output-json-full`. No transcript or model file is stored in this repository.

## Decision

The existing v1.9.4 runtime is sufficient for the first integration proof; this
does not justify replacing OR media/editing engines or updating the pin. The
FFmpeg-backed `SoftwareMediaDecoder::write_transcription_wav` now supplies
bounded mono 16 kHz PCM WAV from ordinary WAV, AAC-in-MP4, and MP3 sources,
without a system FFmpeg executable or playback-path changes. Audio source
offsets and forward timestamp gaps are preserved as silence relative to the
container media start; a generated 250 ms offset fixture verifies alignment.
One stream time-base tick is tolerated between contiguous decoded frames and
larger overlaps fail explicitly; real positive-start and packet-gap fixtures
verify silence alignment. Silence writes also check cancellation. Focused media
tests cover the WAV structure, duration rejection, cancellation, AAC and MP3
decode. The pinned CLI's `--output-json-full` segment output now has a bounded
adapter in `or_ai`: it reads the already-millisecond offsets emitted by the
upstream serializer (which converts its internal 10 ms ticks), derives the
explicitly uncalibrated mean token probability, and binds
each segment to the identity supplied by the trusted host before validation. A
fixture shaped from the pinned v1.9.4 serializer verifies the JFK transcript
times and text. This remains a conversion primitive, not a packaged
transcription capability. Remaining work is provider lifecycle and packaging,
optional verified model installation, and human review/apply UX through
existing commands. A separate-process
Android runtime path and representative physical-device measurement remain
open.
