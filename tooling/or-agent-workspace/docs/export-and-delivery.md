# Export and delivery

Scope: rendering the open sequence to a file, watching that job, and writing
subtitle sidecars. Timeline mechanics are
[timeline-editing.md](timeline-editing.md); caption timing is
[captions.md](captions.md).

## When to export

Export when the user asks for a finished file, or when a delivery step in the
brief names one. It is the most expensive command in the workspace: it renders
every frame on the app's pipeline and can run for minutes. Do not export as a
smoke test, and do not export after every edit — save the project first, re-read
it, then export once.

## Render the sequence

```sh
tools/or export start --attach project.json \
  --destination "$PWD/work/exports/intro.mp4" \
  --profile mp4 --quality quality --json
```

- `--destination` must be an absolute path the app can write; keep outputs under
  `work/exports/` and expand it yourself (`$PWD/work/exports/...`), because the
  app resolves a relative path against its own working directory.
- The extension selects the container and codec: `.mp4` (H.264/AAC), `.webm`
  (VP9/Opus) or `.mkv` (FFV1/PCM, for lossless handoff). `--profile mp4|webm|ffv1`
  is optional and must agree with the extension; a mismatched or missing
  extension is refused before any job starts.
- `--quality speed` trades encode time for size; `quality` is the default.
- `--short-side` caps the output's short edge in pixels; omit it to keep the
  sequence canvas shape.
- The command waits for the job, then prints the job id and destination as JSON.
  Add `--no-wait` when you want to start it and continue reading other state.

The app's Export button and this verb share one queue, so a job the user started
in the app can be polled by id.

## Watch or stop a job

```sh
tools/or export status --attach project.json --job JOB_ID --json
tools/or export cancel --attach project.json --job JOB_ID --json
```

States are `queued`, `running`, `succeeded`, `failed` and `cancelled`; progress
is `completed`/`total` frames. `cancel` asks the app to stop and returns before
the job has fully unwound — poll once after it to confirm the final state.

## Subtitle sidecars

```sh
tools/or captions export --attach project.json --out "$PWD/work/exports/intro.srt" --json
```

Every caption clip on every caption track is collected in start order, so this
matches what the user sees in the app. `--format srt|vtt` is inferred from the
extension; pass it only when the path has neither extension. `CAPTION_TRACK_EMPTY`
means there is nothing to write — import or transcribe captions first, do not
create an empty file by hand.

## Verify before reporting

1. `export status` must report `succeeded` and `progress.completed` equal to
   `progress.total`.
2. Check the file exists and is non-empty (`ls -l` is enough; the app writes
   atomically, so a partial file at the destination is a failure, not progress).
3. For subtitles, read the first and last cue back; an empty or one-cue file on a
   narrated video is a signal the caption track is wrong.
4. Report the path, the profile and the job id. Never claim an export exists
   because `start` returned.

## Failures

- `EXPORT_RUNTIME_UNAVAILABLE` — this build has no encoder runtime; tell the user
  which build they are on, do not retry in a loop.
- `EXPORT_FAILED` — the job stopped; the message carries the concrete cause
  (unsupported source, bad destination, encoder error). Fix the cause, then start
  one new job.
- `EXPORT_CANCELLED` — the user or a cancel call stopped it; do not silently
  restart.
- A refused `export start` usually means no attached project or an unwritable
  destination. `--attach requires a path` is a usage error, not an app failure.
