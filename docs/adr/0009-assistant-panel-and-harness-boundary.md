# ADR 0009 — Assistant panel and external harness boundary

- Status: Accepted
- Date: 2026-10-26

## Context

The owner asked for an AI panel beside the editor: choose a workspace and model,
then ask the assistant to edit the open project and to turn code into a
MotionScene illustration inserted on the timeline. DeepSeek Harness already runs
on the host with the user's provider credentials and model catalog.

OR must not gain a second project model, a second provider integration, or a
credential copy. Every edit still belongs to the Rust-owned command path
(ADR 0001, [control-plane.md](../architecture/control-plane.md)), and
MotionScene materialisation must stay deterministic and FFmpeg-only
(ADR 0007).

## Decision

- The assistant is a **client of an external harness**, not an embedded
  provider. OR starts `DeepSeek Harness` in its `acp` profile over stdio and
  speaks the Agent Client Protocol. Provider credentials stay in the harness's
  credential service; OR never reads or forwards them.
- The harness boots with a generated `--patch` overlay. The overlay copies the
  user's active dsh profile `llm-pi-ai` provider catalog and
  `agent-default-model` selection, overrides the `acp` provider/model, and
  replaces the persona with OR's editing contract. It contains no secret.
- Agent edits go through an **MCP stdio server in the `or` CLI** (`or mcp`).
  Its tools attach to the running app over the existing local IPC descriptor and
  use the same typed commands and queries as the CLI and GUI. The agent cannot
  edit a `.orproj` file directly and no MCP tool can reach an arbitrary path or a
  shell.
- The application writes a small **session pointer** (descriptor path, project
  file, workspace). `or mcp` re-reads it on every call, so switching the open
  project does not require a new ACP session and never edits the wrong project.
- MotionScene materialisation is an **application runtime request**
  (`ApplicationRequest::MotionMaterialize`), because only the app links the
  approved FFmpeg encoder. The request returns facts only; the caller registers
  the output through the ordinary `media.add` and `timeline.clip.insert`
  commands. It makes no provider or network call.
- The panel occupies a tab in the existing right rail. It is not a new shell
  region; the assistant transcript and permission prompts are presentation
  state only.

## Alternatives rejected

- Provider calls or API keys inside OR: duplicates the harness, violates
  ADR 0005's boundary and the secrets rule.
- A second project model or direct writes to `.orproj`: breaks revision,
  history, recovery and save semantics.
- An HTTP MCP server inside the app: widens the local network surface that the
  IPC contract deliberately avoids.
- A standalone encoder in the CLI: the shipped CLI has no FFmpeg libraries, and
  a second encoding path would drift from export.
- Tool schemas injected by the ACP client: ACP v1 does not carry client tools;
  MCP is the standard surface and keeps the tools auditable.

## Consequences

- The panel is desktop-only for now: it needs a child process. Android shows the
  tab as unavailable rather than pretending to work.
- macOS Developer Preview builds run **unsandboxed**. App Sandbox cannot grant a
  child process the harness's home-directory credentials and MCP bridge, and a
  separately installed assistant host would add a second service. The app keeps
  its user-selected file access and still binds only a Unix-domain socket; the
  packaging gate now asserts the sandbox entitlement stays absent.
- The user must have a working harness installation and an `or` CLI build. The
  panel surfaces both as explicit setup errors with a fix.
- Tool calls can require a permission prompt; the panel renders the harness's
  options and can auto-approve when the user opts in.
- Edits remain live-but-unsaved until `save` (or the user's Save), matching the
  project's explicit-save contract.
- Harness or MCP protocol changes are gated behind the ACP and MCP versions this
  client implements; a mismatch shows a connection error, not a silent partial
  edit.
