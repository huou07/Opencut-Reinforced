# Feature — Assistant Panel

Last reviewed: 2026-10-27

Scope: the in-editor AI panel, its connection to a local DeepSeek Harness over
ACP, the MCP tools that edit the open project, and the states the panel must
show. Architecture rationale is [ADR 0009](../adr/0009-assistant-panel-and-harness-boundary.md).

## Surface

- The right rail has two tabs: **Inspector** and **Assistant**. The assistant
  reuses the inspector slot; it is not a new shell region.
- Keyboard: `Cmd/Ctrl+Shift+A` opens the Assistant tab. The command palette has
  **Show Assistant Panel** and **Show Inspector Panel**.
- The panel header shows connection state and actions: new conversation,
  reconnect, settings. Switches never fork project state.

## Connection

- OR starts the installed harness as `dsh --profile acp --patch <overlay>` and
  speaks ACP v1 over newline-delimited JSON-RPC stdio. On macOS this requires the
  unsandboxed Developer Preview build: a sandboxed child would inherit the app's
  sandbox and could not read the harness credentials or run its own bridge.
- The overlay is generated per connection: the active dsh profile's provider
  catalog and default provider/model are copied, the `acp` bundle selects that
  model, and the persona is replaced with OR's editing contract. The overlay
  contains no secret; the harness resolves credentials itself.
- **Workspace** selects the session working directory and the address the agent's
  file tools see. The default is the app-installed **agent workspace**
  ([agent-workspace.md](agent-workspace.md)): its `AGENTS.md` routes the agent to
  the one guide a task needs. Assistant settings also offer the open project's
  folder or a chosen folder. Changing the choice starts a new conversation; the
  workspace does not follow the project when another project opens.
- **Model** and **reasoning effort** come from the session's advertised ACP
  configuration options. The choice is persisted and applied to the next
  connection and the live session.
- If the harness or the `or` CLI cannot be found, the panel shows a setup error
  with the concrete fix; it never sends a prompt into a session without tools.

## Editing

The harness receives one MCP server named `or`. Its tools appear to the model as
`mcp__or__<tool>` and every call attaches to the running app over local IPC:

| Tool | Effect |
|---|---|
| `project_status` | project name, revision, dirty state, project file |
| `timeline_overview` | frame rate, tracks with clips, markers |
| `media_list` | media ids, sources, duration metadata (paged) |
| `save` | explicit project save |
| `undo`, `redo` | canonical history |
| `add_media` | import a local file into the media library |
| `add_track` | add a video/audio/text/caption track |
| `insert_clip` | place media at an exact time; duration defaults to the media length |
| `insert_text` | add a text or caption clip |
| `move_clip`, `trim_clip`, `split_clip`, `delete_clip` | ordinary timeline edits; delete can ripple |
| `set_frame_rate`, `add_marker` | sequence and marker edits |
| `motion_insert_scene` | render a code file or `.ormotion.json` scene, import it, insert it |
| `motion_asset_list` | page the reusable motion library with ids, categories and durations |
| `motion_asset_insert` | instantiate a library asset with parameter values, render it, import it and insert it |

Times accept exact `NUM/DEN` seconds or decimal seconds. The tool set is the
agent's whole editing surface: it cannot write project files directly, read
arbitrary paths through IPC, or run shell commands through the MCP server.

## MotionScene from chat

`motion_insert_scene` is the code-to-video path. The agent writes or chooses a
source file in the workspace, the app materialises it with the deterministic
MotionScene renderer and its own FFmpeg encoder, then the same call imports the
video and inserts an ordinary, editable clip. No provider or network call
happens during rendering. The clip behaves like any other media clip: trim,
split, move, export, and save/reopen. The workspace Motion guide also documents
`tools/or motion validate|inspect|from-code` and the scene scaffold, so an agent
can check a scene before paying for a full materialisation.

## Transcript and permissions

- The transcript keeps user prompts, streamed assistant text, collapsible
  reasoning, tool cards with queued/running/done/failed state and expandable
  output, errors and notes. Text and code are selectable; replies render fenced
  code as monospace blocks.
- The editor publishes the selected clip and playhead as prompt context, so
  "this clip" and "here" refer to what the user sees.
- ACP permission requests render inline with the harness's own options. **Allow
  tool calls without asking** is an explicit opt-in in Assistant settings.
- Stop cancels the turn; new conversation closes the ACP session and creates a
  fresh one without restarting the process.

## States

`idle`, `connecting`, `ready`, `error`, `stopped`. Every failure names what is
missing and offers retry or settings. The model picker is disabled until the
session advertises options. Usage shows context tokens when the harness reports
them. Android has no assistant: the desktop process boundary is explicit.

## Bounds

- Prompt text is capped at 16,000 characters; transcripts are in-memory only.
- MCP pages are bounded (100 items); the agent must page rather than load an
  unbounded timeline.
- Materialisation keeps the MotionScene frame bound (200,000 frames) and writes
  one persistent file; the caller chooses or inherits a project-adjacent
  `generated/` path.
- The descriptor grants owner-only local access; the session pointer contains
  paths, never the auth token.

## Gaps

- Desktop only; no Android assistant.
- No session list, resume or transcript replay; a new app session starts fresh.
- No image/audio prompt attachments.
- No review-before-apply diff for edits; permissions and undo are the safety net.
- The agent can call `undo`, but there is no grouped "revert this turn".
- No in-panel model catalog editing; it reflects the harness configuration.
- Provider credential setup stays in DeepSeek Harness, not in OR.

## Where

- `apps/or_app/lib/assistant/` — ACP client, controller, settings, panel
- `apps/or_app/lib/assistant/agent_workspace.dart` — installed workspace and its runtime files
- `apps/or_app/lib/screens/editor_shell_preview_screen.dart` — right-rail tabs
- `crates/or_cli/src/mcp.rs` — `or mcp` tools
- `crates/or_core/src/application.rs` — `MotionMaterialize` runtime request
- `crates/or_app_bridge/src/preview.rs` — materialisation handler

## Related

- [agent-workspace.md](agent-workspace.md)
- [workspace-ui.md](workspace-ui.md)
- [motionscene.md](motionscene.md)
- [control-plane.md](../architecture/control-plane.md)
- [security-boundaries.md](../architecture/security-boundaries.md)
