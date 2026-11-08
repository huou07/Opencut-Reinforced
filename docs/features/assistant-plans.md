# Assistant Plans

Last reviewed: 2026-11-09

Scope: how a cut planned outside OR — by the Assistant, by another agent, or by
hand — is reviewed and written into the timeline. The plan is the model's; the
editing stays the user's.

## Principle

OR does not call a model for this workflow and does not pretend a rule-based
planner is one. The assistant (or any agent the user runs) produces a structured
plan; OR parses it, checks it against the open project, shows every shot and its
reason, and writes only the approved shots through the ordinary clip commands.
A style in the Library is a rule that cuts; a plan is a decision about this
project's footage.

## The document

Schema `or.edit-plan/1`:

```json
{
  "schema": "or.edit-plan/1",
  "frameRate": 30,
  "title": "optional title clip text",
  "notes": ["optional reviewer-facing notes"],
  "shots": [
    {
      "mediaId": "the id the brief listed, or",
      "name": "beach.mp4",
      "sourceStartFrame": 0,
      "frameCount": 90,
      "why": "why this shot is here"
    }
  ]
}
```

Rules enforced by `parseAgentEditPlan` (`apps/or_app/lib/library/agent_edit_plan.dart`):

- `frameRate` is an integer from 1 to 240 and sets the plan's own time base.
- `shots` is non-empty and holds at most 500 entries.
- Each shot has a `mediaId` or a `name`; a shot with neither is rejected.
- `frameCount` is a positive integer no longer than ten minutes at the plan's
  rate; `sourceStartFrame` is a non-negative integer.
- `title` is at most 200 characters; `notes` is at most 20 short strings.
- Unknown media ids and ambiguous names are rejected with the shot number, not
  silently dropped.

Shot frames are scaled from the plan's rate to the project's sequence rate, so a
24 fps plan applies correctly to a 30 fps project.

## Surfaces

- **Library → Edit styles → "From an assistant"** (`library-agent-plan`): enabled
  when a project is open; opens the paste-and-review flow. The card is beside the
  styles because both produce a first cut, and both need an open project.
- **Library → Creative directions → a brief → "Plan a video…"**: the brief
  becomes the plan's creative direction, so the assistant receives the voice,
  script shape, graphics, captions, editing and audio guidance together with the
  media list. The action is offered only while a project is open, because a plan
  is checked against its media.
- **Command palette → "Review an Assistant Plan…"** in the editor.
- The import dialog offers **Copy the brief**, a ready-to-paste prompt containing
  the schema, the rules and the project's media ids, names and durations.
- The review dialog lists the summary (`N shots · T s`), the optional title, each
  shot with its file name, length, source start and `why`, plus the plan's notes.
  Apply is the only way the plan reaches the timeline; Cancel writes nothing.

## What applying writes

Each shot becomes an ordinary video clip on the first unlocked video track (a
video track is created when the project has none), starting after the existing
content. The optional title becomes a three-second text clip on a text track.
Every insert is its own history step: Undo removes the cut one clip at a time,
and each clip is then edited, trimmed, moved or deleted like any other.

Because the media import that precedes a plan advances the project revision, the
writer re-reads the project summary first and sends each command against the
revision the previous command returned. A stale revision is a rejected plan, not
a silently partial one.

## Limits

- The plan is a first cut, not a complete edit: it does not add transitions,
  effects, music or captions. Those remain manual (or another assistant call)
  after the cut exists.
- A creative direction reaches the assistant as text in the brief. OR does not
  call a provider itself, so a direction produces nothing until the configured
  assistant runs with that brief.
- The assistant must be given the brief (or the media ids) to write a plan that
  resolves; OR never guesses a media id.
- The workflow needs a configured assistant harness for the agent to run at all.
  Without one, the paste-and-review flow still works with a plan written by hand
  or copied from elsewhere; nothing here requires a cloud provider.
- The debug UI harness proves the journey on macOS. Linux and Windows runs are
  recorded separately when those hosts are exercised.
