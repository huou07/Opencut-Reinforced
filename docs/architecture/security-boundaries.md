# Security Boundaries

Last reviewed: 2026-10-27

Scope: where trust boundaries sit in the architecture. Dependency, licence and
reporting policy is in [reference/security-licensing.md](../reference/security-licensing.md).

## Untrusted input

Every imported or generated artifact is untrusted and validated at its boundary:
projects, media, subtitles, models, templates, themes, community content, plugin
output and AI output. A project file never causes code execution on open.

## Secrets

- Provider credentials live in OS secure storage.
- The application may report whether a provider is configured; CLI and agents
  never receive plaintext stored secrets, and secrets never appear in project
  data, logs, screenshots or CLI output.
- Network permissions and Offline Mode are enforced at the application/provider
  boundary — below individual UI controls — so GUI, CLI and agents cannot bypass
  them.

## Local IPC

Protocol v1 is local-only (Unix socket / Windows named pipe), owner-only, with a
random per-server token and no TCP/HTTP fallback. It is not a boundary against
malicious code running as the same OS user.

## Agent surface

Agents are clients of the same structured commands and queries as humans. A
generated EditPlan passes the same permission and domain validation. Agents
cannot read arbitrary paths through IPC, run shell commands through it, or
retrieve stored credentials.

The in-editor assistant adds two hops, not a new trust path: an external
DeepSeek Harness process speaks ACP to the app, and its `or mcp` server attaches
to the same local IPC descriptor. The MCP tools expose project commands only;
they cannot read project files directly, execute a shell, or receive the
descriptor token. Provider credentials stay in the harness's credential service,
and the generated `--patch` overlay contains no secret.

The assistant's installed agent workspace is content, not a new authority. The
app packs it (`tooling/or-agent-workspace`), validates every bundle path against
absolute and `..` traversal, and writes it only under application support. It
carries guides, skills and the `or` wrapper — never credentials — and grants no
tool the assistant did not already have. Files the agent writes stay in that
workspace; project changes still happen only through the `or` command path.

## Deterministic rendering

MotionScene validation, preview, rendering and materialisation make zero
provider or network calls, including no per-frame model call. A declarative
scene cannot fetch remote assets, run scripts, or perform hidden font lookups.
PNG assets are decoded with an encoded and pixel bound, and their paths must stay
inside the scene directory or the app-owned library.

## Enforced boundaries

| Boundary | Enforcement |
|---|---|
| `or_core` stays free of GPU/media/audio/text/AI runtime dependencies | `scripts/check_or_core_deps.py` in repository hygiene |
| No secrets in tracked files or history | `gitleaks detect` in the acceptance lane and pinned in CI |
| Project mutations only through the command path | code review plus command/revision tests |
| Local IPC cannot reach arbitrary paths or shell | protocol surface plus transport tests |
| Assistant tools go through the command path | `or mcp` has no file/shell tool; only typed commands and queries |
| Assistant host can reach the harness credentials | macOS Developer Preview runs unsandboxed on purpose (ADR 0009); the harness keeps the credentials, OR never receives them |

## Related

- [system-overview.md](system-overview.md)
- [reference/security-licensing.md](../reference/security-licensing.md)
- [features/motionscene.md](../features/motionscene.md)
