# Timeline editing

Scope: the exact-time timeline commands the assistant runs against the live
project. Project state is Rust-owned; read [project-safety.md](project-safety.md)
before saving or undoing.

## Order of operations

1. `tools/or project summary --attach project.json --json` — confirm a project is
   open and read its revision and dirty state.
2. `tools/or timeline tracks --attach project.json --json` and
   `tools/or timeline settings --attach project.json --json` — read the track ids
   and the sequence rate.
3. `tools/or timeline clips --track TRACK_ID --attach project.json --json` — read
   the clips with their start, source range and duration. Page with `--offset`
   and `--limit` when the page reports more.
4. Use only the ids and times those calls returned. Never invent a `clip_id`,
   `track_id` or media id.
5. Make the smallest command the request needs.
6. Re-read the affected track when placement matters.
7. `tools/or project save --attach project.json --json` unless the user asked to
   review first.

## Time form

- Times are exact `NUM/DEN` seconds or decimal seconds (`2.5`).
- A clip's `--at` is its timeline start; `--source-start` addresses the media
  in-point.
- `insert-clip` has no default duration: pass `--duration` taken from the media
  metadata in [media-and-import.md](media-and-import.md).
- Do not guess a position from the prompt. The panel sends the selected clip and
  playhead as context when they exist; otherwise ask or append after the last clip.

## Commands

Every command below takes `--attach project.json --json` plus the flags shown.

| Command | Use |
|---|---|
| `timeline add-track --kind video\|audio\|text\|caption` | Add an empty track. |
| `timeline insert-clip --track --media --at --source-start --duration` | Place imported media at an exact time. `--mode insert` (the default) moves later clips right; `--mode overwrite` clears the range. |
| `timeline insert-text --track --at --duration --text` | Add a text clip; `insert-caption` adds a caption clip instead. |
| `timeline move-clip --clip --track --at` | Move a clip in time, possibly to another track. |
| `timeline trim-clip --clip --edge start\|end --to` | Trim one edge to an exact timeline time; `--ripple` closes the gap. |
| `timeline split-clip --clip --at` | Split a clip; the right-hand part starts there. |
| `timeline delete-clip --clip` | Delete a clip and leave the gap. |
| `timeline ripple-delete-clip --clip` | Delete a clip and close the gap on that track. |
| `timeline set-frame-rate --rate NUM/DEN` | Set the sequence rate; `clear-frame-rate` unsets it. |
| `timeline add-marker --at --label` | Add a labelled marker. |

`tools/or commands` lists every mutating verb and `tools/or queries` lists every
read, with their schema versions, when the exact surface is not obvious.

## Track and clip rules

- Visual tracks (`video`, `text`, `caption`) carry visual clips; `audio` tracks
  carry audio clips. A text clip on a video track is refused.
- Text and caption clips need a text or caption track; create one with `add-track`
  when the timeline has none.
- Clips cannot overlap on one track. `insert` makes room, `overwrite` replaces it.
- Track order on screen is the order `timeline tracks` returns.

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

## Ducking music under narration

`tools/or timeline duck-clip --attach project.json --track MUSIC_TRACK --clip
MUSIC_CLIP --voice-track VOICE_TRACK [--duck-db -12] --json` lowers the music
clip wherever the voice track has clips. It is one command and one undo entry;
the music clip is split at the voice boundaries and the overlapping segments get
the gain. The app's Music dialog applies the same command automatically when a
voice track is selected.
