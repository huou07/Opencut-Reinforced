# Feature — Assistant Panel

Last reviewed: 2026-10-28

Scope: the in-editor AI panel, its connection to a local DeepSeek Harness over
ACP, the `or` CLI surface and MCP fallback that edit the open project, and the
states the panel must show. Architecture rationale is
[ADR 0009](../adr/0009-assistant-panel-and-harness-boundary.md).

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

The agent's default editing surface is the `or` CLI bound to this build
(`tools/or` in the workspace). Every command attaches to the running app through
the workspace `project.json` pointer; the CLI follows its `descriptor_path` on
each call, so the app, the CLI and the assistant share one session, one project
and one history. The persona and the workspace guides route reads and mutations
through the CLI and treat the MCP tools as a fallback.

The harness still receives one MCP server named `or`; its tools appear to the
model as `mcp__or__<tool>` and attach over the same local IPC. It exists for the
operations the CLI cannot express — motion scene insertion, the motion asset
library, and a CLI failure its error does not explain:

| CLI verb (`tools/or ...`) | MCP fallback | Effect |
|---|---|---|
| `project summary` | `project_status` | project name, revision, dirty state, project file |
| `timeline tracks`, `timeline settings`, `timeline clips`, `timeline markers` | `timeline_overview` | frame rate, tracks with clips, markers (clips page by track) |
| `media list` | `media_list` | media ids, sources, duration metadata (paged) |
| `project save` | `save` | explicit project save |
| `history undo`, `history redo` | `undo`, `redo` | canonical history |
| `media add` | `add_media` | import a local file into the media library |
| `timeline add-track` | `add_track` | add a video/audio/text/caption track |
| `timeline insert-clip` | `insert_clip` | place media at an exact time; the CLI requires the source range and duration |
| `timeline insert-text`, `timeline insert-caption` | `insert_text` | add a text or caption clip |
| `timeline move-clip`, `trim-clip`, `split-clip`, `delete-clip`, `ripple-delete-clip` | `move_clip`, `trim_clip`, `split_clip`, `delete_clip` | ordinary timeline edits; delete can ripple |
| `timeline set-frame-rate`, `timeline add-marker` | `set_frame_rate`, `add_marker` | sequence and marker edits |
| `captions transcribe` | `captions_transcribe` | local multilingual transcript proposal; review before applying |
| `timeline import-captions` | — | apply a reviewed SRT/WebVTT file as a new caption track |
| `timeline duck-clip` | `duck_music` | lower a music clip under a voice track as one undoable command |
| `motion publish` | `motion_insert_scene` | validate a scene, save it to the library, render it, import it and insert it in one call |
| `motion asset list` | `motion_asset_list` | page the reusable motion library with ids, categories and durations |
| `motion asset insert` | `motion_asset_insert` | instantiate a library asset with parameter values, render it, import it and insert it |
| `motion asset save` | `motion_asset_save` | save an authored scene file as a classified library asset with a preview |

Times accept exact `NUM/DEN` seconds or decimal seconds. Neither surface can
write project files directly, read arbitrary paths through IPC, or run shell
commands through the IPC boundary.

## MotionScene from chat

`motion_insert_scene` is the code-to-video path. The agent writes or chooses a
source file in the workspace, the app materialises it with the deterministic
MotionScene renderer and its own FFmpeg encoder, then the same call imports the
video and inserts an ordinary, editable clip. No provider or network call
happens during rendering. The clip behaves like any other media clip: trim,
split, move, export, and save/reopen. The workspace Motion guide documents
`tools/or motion validate|inspect|evaluate|from-code|render|publish` and the
scene scaffold, so an agent checks a scene with the CLI before paying for a full
materialisation. `or motion publish` performs the whole path (library save,
render, import, insert) through ordinary validated commands; `materialize` only
writes a file, and the MCP insertion tool is its fallback.

## Script, narration, captions and music from chat

The installed workspace carries the production tools, and its `AGENTS.md`
routes them through `docs/script-and-voice.md`:

- `tools/voiceover.mjs` validates an `or-assistant-script`, speaks each scene
  with the macOS `say` voice, measures the real WAV duration, writes a timing
  manifest and SRT, and with `--apply` imports the WAVs, places them on an audio
  track and applies the captions. The voice is the one saved in **Settings →
  Narration** (written to `narration-settings.json` and read through the
  workspace pointer), then the first installed Vietnamese voice; the tool has
  no product-level voice constant. This is the offline narration path; nothing
  leaves the machine and no model is downloaded.
- `tools/music.mjs` synthesises a quiet, royalty-free background bed from a mood,
  tempo and duration. It is deterministic per seed and has no network path.
- `tools/or captions transcribe --media <id>` runs the app's **selected** local
  multilingual whisper model (Vietnamese and English are detected automatically)
  and returns bounded cues; `tools/or timeline import-captions` applies a
  reviewed SRT/WebVTT as an editable caption track. This is the "captions from
  existing audio" path. The user picks Base/Small/Medium/Large v3 Turbo in the
  app; the CLI and MCP tool reuse that choice rather than starting their own.

Narration derived from a written script and captions derived from transcription
are different claims: the agent must say which one produced the result. Every
project write still goes through a validated command, and `--apply` leaves the
project unsaved for review.

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
- CLI and MCP pages are bounded (100 items); the agent must page rather than load
  an unbounded timeline.
- `--attach` accepts the endpoint descriptor or a session pointer such as the
  workspace `project.json`; a pointer is re-read on every call, so a project
  switch never leaves a stale descriptor behind.
- Materialisation keeps the MotionScene frame bound (200,000 frames) and writes
  one persistent file; the caller chooses or inherits a project-adjacent
  `generated/` path.
- The descriptor grants owner-only local access; the session pointer contains
  paths, never the auth token.
- The transcription request is a runtime proposal, not an edit: it never bumps
  the project revision, is bounded to four hours of audio and 10,000 cues, and
  the model is downloaded only after consent.

## Gaps

- Desktop only; no Android assistant.
- No session list, resume or transcript replay; a new app session starts fresh.
- No image/audio prompt attachments.
- No review-before-apply diff for edits; permissions and undo are the safety net.
- The agent can call `undo`, but there is no grouped "revert this turn".
- No in-panel model catalog editing; it reflects the harness configuration.
- Provider credential setup stays in DeepSeek Harness, not in OR.
- The MCP server still exposes every CLI verb as a fallback; a smaller fallback
  surface (motion insertion and the library only) is not yet enforced.
- Narration quality is the installed system `say` voice, or a configured cloud
  provider. The voice is selectable and previewable in Settings → Narration;
  ElevenLabs and Azure keys are stored in the keychain, and an ElevenLabs
  personal voice can be created from a recorded WAV sample with an explicit
  consent checkbox. The offline `tools/voiceover.mjs` path still uses the system
  voice, so a cloud choice applies to narration produced in the app.
- The music bed has no tempo/beat sync to the edit and no per-scene ducking; the
  user adjusts clip gain by hand.
- `--apply` reports a placed track and caption count but has no per-step undo
  group; review before saving.

## Where

- `apps/or_app/lib/assistant/` — ACP client, controller, settings, panel
- `apps/or_app/lib/assistant/agent_workspace.dart` — installed workspace and its runtime files
- `apps/or_app/lib/screens/editor_shell_preview_screen.dart` — right-rail tabs
- `crates/or_cli/src/commands.rs` — CLI verbs and session-pointer attach
- `crates/or_cli/src/mcp.rs` — `or mcp` fallback tools
- `crates/or_core/src/application.rs` — `MotionMaterialize` and `Transcription`
  runtime requests
- `crates/or_ipc/src/live_host.rs` — runtime handler traits and routing
- `crates/or_app_bridge/src/preview.rs` — materialisation handler
- `crates/or_app_bridge/src/transcription.rs` — local transcription job and model store
- `tooling/or-agent-workspace/tools/voiceover.mjs` — offline narration, timing and captions
- `tooling/or-agent-workspace/tools/music.mjs` — generated background bed

## Related

- [agent-workspace.md](agent-workspace.md)
- [workspace-ui.md](workspace-ui.md)
- [motionscene.md](motionscene.md)
- [control-plane.md](../architecture/control-plane.md)
- [security-boundaries.md](../architecture/security-boundaries.md)
