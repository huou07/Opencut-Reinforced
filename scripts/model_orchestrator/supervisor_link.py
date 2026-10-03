"""Runner integration with the existing authoritative supervisor.

The orchestrator produces implementation candidates (SHA + local results +
reports). Authoritative verification and STATE transitions stay with
scripts/agent_supervisor.py: this module only shells to `--resume-sha` for an
already-pushed exact SHA. No duplicated CI/evidence/state machinery.
"""
from __future__ import annotations

import subprocess


def supervisor_resume_command(repo: str, checkpoint: str, sha: str) -> list[str]:
    return [
        "python3", "scripts/agent_supervisor.py",
        "--goal", f"checkpoint:{checkpoint}",
        "--resume-sha", sha,
    ]


def validate_sha(sha: str) -> bool:
    return len(sha) == 40 and all(c in "0123456789abcdef" for c in sha)


def handoff_to_supervisor(repo: str, checkpoint: str, sha: str, timeout_s: int = 7200 + 600) -> dict:
    """Hand an exact implementation SHA to the supervisor. Blocking, with timeout."""
    if not validate_sha(sha):
        return {"ok": False, "error": f"invalid SHA: {sha!r}"}
    try:
        proc = subprocess.run(
            supervisor_resume_command(repo, checkpoint, sha),
            cwd=repo,
            capture_output=True, text=True, timeout=timeout_s,
        )
    except subprocess.TimeoutExpired:
        return {"ok": False, "error": "supervisor timed out"}
    except OSError as error:
        return {"ok": False, "error": str(error)}
    return {"ok": proc.returncode == 0, "exit": proc.returncode, "tail": (proc.stdout + proc.stderr)[-4000:]}
