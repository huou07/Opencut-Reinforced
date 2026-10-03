"""Runner integration with the existing authoritative supervisor.

The orchestrator produces implementation candidates (SHA + local results +
reports). Authoritative verification and STATE transitions stay with
scripts/agent_supervisor.py: this module only shells to `--resume-sha` for an
already-promoted exact SHA, and only after verifying every local
precondition itself: branch == main, clean worktree,
HEAD == origin/main == SHA, expected checkpoint still NEXT.
A candidate living only on a review branch is REFUSED before invoking the
supervisor — never passed through. Supervisor validation stays intact as
defense in depth.
"""
from __future__ import annotations

import json
import subprocess
from pathlib import Path


def supervisor_resume_command(checkpoint: str, sha: str) -> list[str]:
    return [
        "python3", "scripts/agent_supervisor.py",
        "--goal", f"checkpoint:{checkpoint}",
        "--resume-sha", sha,
    ]


def validate_sha(sha: str) -> bool:
    return len(sha) == 40 and all(c in "0123456789abcdef" for c in sha)


def _git(repo: str, *args: str) -> subprocess.CompletedProcess:
    return subprocess.run(["git", *args], cwd=repo, capture_output=True, text=True, timeout=60)


def checkpoint_still_next(repo: str, checkpoint: str) -> bool:
    """Read-only STATE check: expected checkpoint remains NEXT."""
    try:
        state = json.loads((Path(repo) / "docs" / "execution" / "STATE.json").read_text())
    except (OSError, ValueError):
        return False
    return state.get("checkpoints", {}).get(checkpoint) == "NEXT"


def check_handoff(repo: str, checkpoint: str, sha: str) -> dict:
    """Local preconditions. Returns {'ok': bool, 'refusals': [...]}."""
    refusals = []
    if not validate_sha(sha):
        return {"ok": False, "refusals": [f"invalid SHA: {sha!r}"]}
    if _git(repo, "rev-parse", "--abbrev-ref", "HEAD").stdout.strip() != "main":
        refusals.append("refusing: not on branch main")
    if _git(repo, "status", "--porcelain").stdout.strip():
        refusals.append("refusing: worktree not clean")
    head = _git(repo, "rev-parse", "HEAD").stdout.strip()
    if head != sha:
        refusals.append("refusing: HEAD != candidate SHA (promote first)")
    origin = _git(repo, "rev-parse", "origin/main").stdout.strip()
    if origin != sha:
        refusals.append("refusing: origin/main != candidate SHA (promote first)")
    if not checkpoint_still_next(repo, checkpoint):
        refusals.append(f"refusing: checkpoint {checkpoint} no longer NEXT")
    return {"ok": not refusals, "refusals": refusals}


def handoff_to_supervisor(repo: str, checkpoint: str, sha: str,
                          timeout_s: int = 7800, invoke=None) -> dict:
    """Verify preconditions, then invoke the supervisor (or a fake in tests)."""
    precheck = check_handoff(repo, checkpoint, sha)
    if not precheck["ok"]:
        return {"ok": False, "refused": True, "refusals": precheck["refusals"]}
    if invoke is not None:
        return invoke(supervisor_resume_command(checkpoint, sha))
    try:
        proc = subprocess.run(
            supervisor_resume_command(checkpoint, sha),
            cwd=repo,
            capture_output=True, text=True, timeout=timeout_s,
        )
    except subprocess.TimeoutExpired:
        return {"ok": False, "error": "supervisor timed out"}
    except OSError as error:
        return {"ok": False, "error": str(error)}
    return {"ok": proc.returncode == 0, "exit": proc.returncode, "tail": (proc.stdout + proc.stderr)[-4000:]}
