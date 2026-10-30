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

## Not available through the CLI

Speed/playback rate (1/16×–16×), reverse, freeze frame, clip volume, mute,
text/caption styling (font, size, anchor, offset, background box, opacity),
multi-clip group move, visual effects and transitions all travel through
`timeline.clip.update`, which has no `tools/or` verb yet. Export has no CLI verb
at all. When the user asks for one of these, say the app owns it: speed and
volume are on the timeline strip once the clip is selected, styling is in the
Edit Title/Caption dialog and the inspector, and export is the Export button.
Let the user make that change, then re-read the timeline and save only what the
commands you ran actually changed. Do not imitate speed with a trim or a split.

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
