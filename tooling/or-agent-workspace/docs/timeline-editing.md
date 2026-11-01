# Timeline editing

Scope: the exact-time timeline commands the assistant runs against the live
project. Project state is Rust-owned; read [project-safety.md](project-safety.md)
before saving or undoing.

## Order of operations

1. `tools/or project summary --attach project.json --json` — confirm a project is
   open and read its revision. `session describe` adds the `dirty` save state.
2. `tools/or timeline tracks --attach project.json --json` and
   `tools/or timeline settings --attach project.json --json` — read the track ids
   with their kind and state, and the sequence rate.
3. `tools/or timeline clips --track TRACK_ID --attach project.json --json` — read
   the clips with their start, source range and duration. One page holds at most
   100 clips: pass `--offset` while the result reports more.
4. `tools/or timeline markers --attach project.json --json` when the request
   names a marker or asks for one; markers page by `--offset` as well.
5. Use only the ids and times those calls returned. Never invent a `clip_id`,
   `track_id` or media id.
6. Make the smallest command the request needs.
7. Re-read the affected track when placement matters.
8. `tools/or project save --attach project.json --json` unless the user asked to
   review first.

## Time form

- Times are exact `NUM/DEN` seconds or decimal seconds (`2.5`).
- A clip's `--at` is its timeline start; `--source-start` addresses the media
  in-point.
- `insert-clip` has no default duration: pass `--duration` taken from the media
  metadata in [media-and-import.md](media-and-import.md).
- `--delta` on slip, roll and slide is a signed duration (`-1/8`, `3/4`).
- Do not guess a position from the prompt. The panel sends the selected clip and
  playhead as context when they exist; otherwise ask, or append after the last
  clip on the track.

## Commands

Every command below takes `--attach project.json --json` plus the flags shown.

| Command | Use |
|---|---|
| `timeline add-track --kind video\|audio\|text\|caption [--id ID]` | Add an empty track. |
| `timeline set-track-state --track ID --locked BOOL --visible BOOL --muted BOOL --solo BOOL` | Set a track's state; all four values are required. |
| `timeline remove-track --track ID` | Remove an empty track. |
| `timeline move-track --id ID --to INDEX` | Reorder one track. |
| `timeline insert-clip --track --media --at --source-start --duration [--id] [--mode insert\|overwrite] [--overwrite-split-id]` | Place imported media at an exact time. `insert` (the default) moves later clips right; `overwrite` clears the range, and an overwrite that lands strictly inside an existing clip must name the id for the split remainder. |
| `timeline insert-text \| insert-caption --track --at --duration --text [--id]` | Add a text clip, or a caption clip on a caption track. |
| `timeline move-clip --clip --track --at` | Move a clip in time, possibly to another track. |
| `timeline set-clip-visual --clip [--x] [--y] [--scale \| --scale-x --scale-y] [--rotation] [--anchor-x] [--anchor-y] [--opacity] [--crop-left ...] [--brightness] [--contrast] [--saturation] [--blur-radius] [--transition-in KIND:NUM/DEN] [--transition-out ...]` | Transform, crop, opacity, effects and transitions. Omitted flags keep their current value; see the units table below. |
| `timeline set-clip-audio --clip [--gain-millidecibels] [--pan-basis-points] [--fade-in NUM/DEN] [--fade-out NUM/DEN]` | Gain, pan and fades. Audio clips only. |
| `timeline set-clip-playback --clip [--rate NUM/DEN] [--reversed true\|false] [--frozen true\|false]` | Speed, reverse and freeze frame. Rate recomputes the clip's duration from its source range; freeze is video only. |
| `timeline move-clips --track --clips ID[,ID...] --delta` | Shift several clips on one track by the same exact delta in one undoable command. |
| `timeline trim-clip --clip --edge start\|end --to [--ripple]` | Trim one edge to an exact timeline time; `--ripple` pulls later clips and is accepted on an end edge only. |
| `timeline slip-clip --clip --delta` | Move the source window inside a fixed clip; the clip stays where it is. |
| `timeline roll-clip --clip --delta` | Move the cut between this clip and its neighbour. |
| `timeline slide-clip --clip --delta` | Move the clip in time and let its neighbours absorb the move. |
| `timeline split-clip --clip --at [--id]` | Split a clip; the right-hand part starts there. |
| `timeline delete-clip --clip` | Delete a clip and leave the gap. |
| `timeline ripple-delete-clip --clip` | Delete a clip and close the gap on that track. |
| `timeline duck-clip --track --clip --voice-track [--duck-db N]` | Lower a music clip under a voice track in one command and one undo step. |
| `timeline import-captions --file PATH.srt\|PATH.vtt` | Add a reviewed subtitle file as a new caption track. |
| `timeline add-marker --at --label [--id]` | Add a labelled marker. |
| `timeline move-marker --id --to` / `rename-marker --id --label` / `delete-marker --id` | Change or remove a marker. |
| `timeline set-frame-rate --rate NUM/DEN` / `clear-frame-rate` | Set or unset the sequence rate. |
| `timeline set-canvas --width N --height N` / `--clear` | Set the finished-video shape, or `--clear` to derive it from the first video clip again. Sides must be even, 16–7680, and at most 4:1. |

`tools/or commands` lists every mutating verb and `tools/or queries` every read,
with their schema versions, when the exact surface is not obvious.
`timeline snap --clip --operation move|trim-start|trim-end --at [--track]` is a
read: it returns the snap candidate the editor would use and mutates nothing.

## Track and clip rules

- Visual tracks (`video`, `text`, `caption`) carry visual clips; `audio` tracks
  carry audio clips. A text clip on a video track is refused.
- Text and caption clips need a text or caption track; create one with `add-track`
  when the timeline has none.
- A locked track refuses edits: read `timeline tracks`, unlock with
  `set-track-state` only when the user asked for the change, then edit.
- Clips cannot overlap on one track. `insert` makes room, `overwrite` replaces it.
- Track order on screen is the order `timeline tracks` returns; `move-track`
  changes it.

## Clip settings

A clip's visual and audio settings travel through the same validated command the
app's inspector uses. Read `timeline clips` first: every verb rewrites the full
current state, so the CLI fills in the fields you did not pass.

Units are the model's own, not seconds or pixels:

| Flag | Unit |
|---|---|
| `--x`, `--y` | milli-canvas: 1000 is 1/1000 of the canvas side |
| `--scale`, `--scale-x`, `--scale-y` | milli: 1000 keeps the source size, 2000 doubles it |
| `--rotation` | milli-degrees: 90000 is a quarter turn |
| `--anchor-x`, `--anchor-y` | basis points: 5000 is the clip centre |
| `--opacity` | basis points: 10000 is opaque, 0 is invisible |
| `--crop-left`, `--crop-top`, `--crop-right`, `--crop-bottom` | basis points of that edge |
| `--brightness` | milli: -1000…1000, 0 neutral |
| `--contrast`, `--saturation` | milli: 1000 neutral |
| `--blur-radius` | milli-pixels of Gaussian blur |
| `--gain-millidecibels` | millidecibels: -600 is -0.6 dB |
| `--pan-basis-points` | basis points: -10000 hard left, 10000 hard right |
| `--fade-in`, `--fade-out` | exact `NUM/DEN` seconds |

- One effect of each kind is kept; pass the neutral value (`--brightness 0`,
  `--contrast 1000`, `--saturation 1000`, `--blur-radius 0`) to remove it.
- Transitions take `cross_dissolve`, `fade_through_black` or `wipe` plus a
  duration, e.g. `--transition-in cross_dissolve:1/2`; `--transition-in none`
  clears it. The transition consumes timeline on the clip, so the server may
  refuse a duration the clip cannot hold.
- `--rate` accepts the app's speed range and recomputes
  `timeline_duration`; do not also trim by hand. Reverse and freeze keep the
  duration. Freeze holds one source frame and is video only.
- A text or caption clip carries visual settings; an audio clip carries audio
  settings. Asking for the wrong family is a usage error, not a no-op.

## Export and subtitle files

The CLI can render the open sequence and write subtitle files:

- `tools/or export start --attach project.json --destination OUT.mp4|.webm|.mkv
  [--quality quality|speed] [--short-side N]` waits for the job and prints its
  id; add `--no-wait` to return a queued job. The extension selects the codec,
  and the destination must be absolute.
- `export status --attach project.json --job JOB_ID` and `export cancel ...`
  poll or stop that job.
- `captions export --attach project.json --out FILE.srt|FILE.vtt` writes every
  caption clip in start order; `--format` is inferred from the extension.

The Export button and the CLI share one job queue, so a job the user started in
the app can be polled by id. See [export-and-delivery.md](export-and-delivery.md).

## Examples

Append a clip to the end of the first video track:

1. `media list` → choose the media id; `timeline clips --track TRACK_ID` → read
   the track's last clip end.
2. `timeline insert-clip --track TRACK_ID --media MEDIA_ID --at "<end>"
   --source-start 0 --duration "<media duration>"`.
3. `timeline clips --track TRACK_ID`, then `project save`.

Trim the selected clip to end at 4 s:

1. The panel context names the clip and its track.
2. `timeline trim-clip --clip CLIP_ID --edge end --to 4`.
3. Verify the duration is positive; a trim that would invert the clip is refused.

Slip the middle clip two frames later inside its own cut:

1. Read the clip page and confirm the source has room on both sides.
2. `timeline slip-clip --clip CLIP_ID --delta 1/15` for 30 fps, or the matching
   fraction of the sequence rate.
3. Re-read the clip: `timeline_start` and duration are unchanged; only the
   source range moved.

## Ducking music under narration

`tools/or timeline duck-clip --attach project.json --track MUSIC_TRACK --clip
MUSIC_CLIP --voice-track VOICE_TRACK [--duck-db -12] --json` lowers the music
clip wherever the voice track has clips. It is one command and one undo entry;
the music clip is split at the voice boundaries and the overlapping segments get
the gain. The app's Music dialog applies the same command automatically when a
voice track is selected.
