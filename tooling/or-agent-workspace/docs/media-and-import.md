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
| `media add --source PATH` | Import one absolute path; returns the new media id and metadata. |
| `media relink --id MEDIA_ID --source PATH` | Point an existing item at a new source under the same `MediaId`; undoable, and timeline references survive. |
| `media remove --id MEDIA_ID` | Remove a library entry; refused while a clip references it. |
| `media clean-audio --source PATH --out PATH [--noise-floor-db N] [--reduction N]` | Noise reduction that reads one file and writes another. Nothing in the project changes until the result is imported; see [audio-prep.md](audio-prep.md). |

## What can be imported

- The supported profile is Matroska (FFV1/PCM S16LE, VP9/Opus, H.264/AAC), WAV
  PCM S16LE, MP3 audio and WebM VP9/Opus. MP4-family H.264/AAC import is a
  candidate profile; H.265 and AV1 are not supported.
- A file must exist and be readable when `media add` runs; the library records a
  source path, not a copy.
- Importing the same path twice is allowed; use the id the new call returns.
- An unsupported file is reported as unsupported, not corrupt. A probe that the
  packaged `ffprobe` cannot complete falls back to the libavformat the app links,
  so a QuickTime file the helper rejects can still import.

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
