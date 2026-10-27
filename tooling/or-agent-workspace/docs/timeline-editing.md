# Timeline editing

Scope: the exact-time timeline commands the assistant may call. Project state is
Rust-owned; read [project-safety.md](project-safety.md) before saving or undoing.

## Order of operations

1. `mcp__or__project_status` — confirm a project is open and read its revision.
2. `mcp__or__timeline_overview` — read the sequence rate, tracks, clips and markers.
3. Use only the ids and times that call returned. Never invent a `clip_id`,
   `track_id` or media id.
4. Make the smallest command the request needs.
5. `mcp__or__timeline_overview` again when the placement matters.
6. `mcp__or__save` unless the user asked to review first.

## Time form

- Times are exact `NUM/DEN` seconds or decimal seconds (`"2.5"`, `"1.25s"`).
- Durations default to the media's full length when omitted.
- A clip's `at` is its timeline start; `source_start` addresses the media in-point.
- Do not guess a position from the prompt. The panel sends the selected clip and
  playhead as context when they exist; otherwise ask or append after the last clip.

## Commands

| Tool | Use |
|---|---|
| `add_track` | Add an empty `video`, `audio`, `text` or `caption` track. |
| `insert_clip` | Place imported media at an exact time. Default mode `insert` moves later clips right; `overwrite` clears the range. |
| `insert_text` | Add a `text` or `caption` clip on a text/caption track. |
| `move_clip` | Move a clip in time, optionally to another track. |
| `trim_clip` | Trim the `start` or `end` edge to an exact timeline time. |
| `split_clip` | Split a clip at a time; the right-hand part starts there. |
| `delete_clip` | Delete a clip; `ripple: true` closes the gap on that track. |
| `set_frame_rate` | Set the sequence rate from `NUM/DEN`. |
| `add_marker` | Add a labelled marker at a time. |

## Track and clip rules

- Visual tracks (`video`, `text`, `caption`) carry visual clips; `audio` tracks
  carry audio clips. A text clip on a video track is refused.
- `insert_text` requires a text or caption track; create one with `add_track` if
  the timeline has none.
- Clips cannot overlap on one track. `insert` makes room, `overwrite` replaces it.
- Track order on screen is the order `timeline_overview` returns.

## Examples

Append a five-second clip to the end of the first video track:

1. `media_list` → choose `media_id`; `timeline_overview` → read the track's last
   clip end.
2. `insert_clip {track_id, media_id, at: "<end>"}` (duration defaults to full).
3. `timeline_overview`, then `save`.

Trim the selected clip to end at 4 s:

1. The panel context names the clip and its track.
2. `trim_clip {clip_id, edge: "end", at: "4"}`.
3. Verify the duration is positive; a trim that would invert the clip is refused.
