# Security Boundaries

Last reviewed: 2026-10-26

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

## Deterministic rendering

MotionScene validation, preview, rendering and materialisation make zero
provider or network calls, including no per-frame model call. A declarative
scene cannot fetch remote assets, run scripts, or perform hidden font lookups.

## Enforced boundaries

| Boundary | Enforcement |
|---|---|
| `or_core` stays free of GPU/media/audio/text/AI runtime dependencies | `scripts/check_or_core_deps.py` in repository hygiene |
| No secrets in tracked files or history | `gitleaks detect` in the acceptance lane and pinned in CI |
| Project mutations only through the command path | code review plus command/revision tests |
| Local IPC cannot reach arbitrary paths or shell | protocol surface plus transport tests |

## Related

- [system-overview.md](system-overview.md)
- [reference/security-licensing.md](../reference/security-licensing.md)
- [features/motionscene.md](../features/motionscene.md)
