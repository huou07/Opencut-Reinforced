# Capability and setup

Scope: answering "can this machine do this?" before promising a codec, an
encoder, a voice or a 3D scene, and telling the user where to fix a missing
piece. This guide is about facts you can observe, not about a feature list.

## What you can ask the binary

| Command | Answers |
|---|---|
| `tools/or version --json` | app name, version, core API version. |
| `tools/or health --json` | whether the core is healthy. |
| `tools/or capabilities --json` | the core API capability ids this build exposes. |
| `tools/or media probe --file PATH --json` | one file's containers, streams, codecs, pixel formats, durations. No project needed. |
| `tools/or export status --job JOB_ID --json` | real progress of a running export. |

`capabilities` is deliberately small: it lists the core API surface, not the
media codecs. There is no command that prints a codec table, and inventing one
from memory is the failure this guide exists to prevent. A codec is "supported"
on this machine only when a probe of a real file of it succeeds and the file
imports.

## Order of operations before promising quality

1. Probe the source: `media probe --file PATH --json`. Read `format_name`, each
   stream's `codec_name`, `width`/`height`, `avg_frame_rate` and audio
   `sample_rate`/`channels`.
2. Import it: `media add --source PATH --json`. Import proves a decode of the
   first video frame, so a successful add is stronger evidence than a probe.
3. Only then describe the result to the user. If import fails, report the code
   and the fix; do not retry the same file with a different claim.

## Import refusal codes and what to say

| Code | Meaning | What the user does |
|---|---|---|
| `UNSUPPORTED_CONTAINER` | the container is not in the shared profile. | Re-encode or convert to MP4/MOV/MKV/WebM, WAV/MP3/FLAC/M4A, or PNG/JPEG/WebP. |
| `UNSUPPORTED_CODEC` | the codec is outside the profile, or this build has no decoder for it. | Same; H.264 or VP9 always imports. |
| `DECODE_UNAVAILABLE` | a decoder exists but this machine cannot run it (AV1 on Apple silicon is the known case). | Import an H.264 or VP9 version, or use a machine whose decoder works. |
| `SOURCE_NOT_FOUND`, `SOURCE_UNAVAILABLE` | the path moved or is unreadable. | Fix the path, or relink the library item. |

Read the message, not just the code: it names the codec and, for
`DECODE_UNAVAILABLE`, the underlying reason. Never translate a refusal into
"the file is corrupt"; the app distinguishes unsupported from unreadable.

## Export choices and their real cost

| Destination | What it is | When it is the right answer |
|---|---|---|
| `.mp4` | H.264 and AAC, the broad-compatibility file. | Sharing, uploads, anything that must play everywhere. |
| `.webm` | VP9 and Opus. | Smaller files and web delivery; slower to encode. |
| `.mkv` | FFV1 lossless video with PCM audio. | Archival or a later re-encode; large files, and `--quality` is ignored. |

`export start` derives the container from the destination extension; `--profile`
must agree with it. `--quality quality` spends more encode time for a cleaner
picture; `--quality speed` is the quick draft. `--short-side` limits the output
resolution to that short side while preserving the aspect ratio; omit it to keep
the project canvas. A 4K canvas exported at `--short-side 1080` is a real
downscale, not a crop.

If the machine has no hardware encoder for the chosen profile the app falls back
to software and says so through the job's progress and error text; a slower
export is not a failure. Do not "fix" it by switching the profile silently.

## Missing pieces the user must install

Nothing is downloaded by an agent. When the user needs an optional component,
the app owns the install: Settings → Resources lists each verifyable download
with its source, version, licence, size and SHA-256, and only installs what the
user asks for. Point the user there with the exact component name and why it is
needed; do not fetch, unpack or run anything yourself, and never ask for a
credential.

For the packaging state (which runtimes are bundled) see
[workspace.md](workspace.md) and [troubleshooting.md](troubleshooting.md).

## Reporting limits honestly

- "Imported and playable on this machine" is a claim you have evidence for.
- "Supports AV1" is a claim you do not have evidence for unless the add
  succeeded on the machine in front of you.
- "Should work on Windows" is not a result. Say what was measured and what was
  not.
