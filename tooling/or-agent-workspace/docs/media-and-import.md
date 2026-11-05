# Media and import

Scope: getting a file into the project library, finding the id the timeline
commands take, and keeping sources findable. The library is project state;
importing is a project mutation and is saved like any other edit.

## Order of operations

1. `tools/or media list --attach project.json --json` — check whether the file is
   already imported. The list is paged: pass `--offset` while the page reports
   more items.
2. `tools/or media add --attach project.json --source PATH --json` — import an
   **absolute** path. The result carries the new media id and its metadata.
3. Place it with `timeline insert-clip` (see
   [timeline-editing.md](timeline-editing.md)); use the duration from this result.
4. `tools/or project save --attach project.json --json` after the edit, unless the
   user asked to review first.

## Verbs

| Command | Use |
|---|---|
| `media probe --file PATH` | Read one file's metadata without a project or an app session. |
| `media list [--offset N] [--limit N]` | Read the library page by page. |
| `media get --id MEDIA_ID` | Read one item by id with its source path, duration, dimensions and audio details. |
| `media add --source PATH` | Import one absolute path; returns the new media id and metadata. |
| `media relink --id MEDIA_ID --source PATH` | Point an existing item at a new source under the same `MediaId`; undoable, and timeline references survive. |
| `media remove --id MEDIA_ID` | Remove a library entry; refused while a clip references it. |
| `media clean-audio --source PATH --out PATH [--noise-floor-db N] [--reduction N]` | Noise reduction that reads one file and writes another. Nothing in the project changes until the result is imported; see [audio-prep.md](audio-prep.md). |

## What can be imported

- Containers: Matroska and WebM; MOV/MP4/M4V/M4A/3GP/3G2; WAV, MP3, OGG, FLAC,
  AIFF, AVI; single PNG, JPEG and WebP stills. GIF and TIFF are refused by design.
- Codecs in the shared profile: H.264, HEVC, VP9, AV1, VP8, FFV1, ProRes, MJPEG,
  MPEG-4 and similar video; AAC, Opus, Vorbis, FLAC, ALAC, AC-3/E-AC-3 and PCM
  audio. Being in the list is not a promise that this host can decode it.
- Import proves a decode of the first video frame. A codec outside the profile is
  refused as `UNSUPPORTED_CODEC`; a codec whose decoder exists but cannot run here
  is refused as `DECODE_UNAVAILABLE` with the reason. `DECODE_UNAVAILABLE` is the
  one to expect on Apple silicon for AV1. See
  [capability-and-setup.md](capability-and-setup.md) for the codes and the fix to
  give the user.
- A file must exist and be readable when `media add` runs; the library records a
  source path, not a copy.
- Importing the same path twice is allowed; use the id the new call returns.
- An unsupported file is reported as unsupported, not corrupt. A probe that the
  packaged `ffprobe` cannot complete falls back to the libavformat the app links,
  and the identical matrix is applied.

## Paths

- `--source` must be absolute. The workspace sits outside the project, so a path
  relative to the project is not automatically correct.
- Files the agent downloaded or wrote live under `work/`; keep them there and pass
  that absolute path.
- Do not move or delete a source file after importing it. If it moves, relink it
  with `media relink`; do not import it again unless the user wants a second copy.
- `media remove` removes a library entry, not the file on disk.

## Reading metadata

`media list` returns ids, source paths, duration, dimensions and audio details.
Use the returned duration when a clip needs a length; do not round it into a
different time base. `media probe` answers the same questions for a file that is
not in the project yet and needs the packaged FFmpeg helper; if that helper is
missing, the command reports a helper error and the app still imports the file.
`project summary` reports the project revision; `session describe` adds the
`dirty` save state.
