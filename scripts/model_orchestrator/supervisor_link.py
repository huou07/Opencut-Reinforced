"""Runner integration with the existing authoritative supervisor.

Handoff is tied to the promoted task authorization, not merely to a SHA
that happens to equal HEAD/origin-main. Before constructing
`agent_supervisor.py --resume-sha ...` the link verifies, from trusted
runtime memory: task status PROMOTED or SUPERVISOR_READY, candidate ==
promoted SHA, HEAD == origin/main == promoted SHA, clean worktree,
expected checkpoint still NEXT, task checkpoint matches. Anything else is
a structured refusal BEFORE invoking the supervisor. Supervisor validation
stays intact as defense in depth.
"""
from __future__ import annotations

import json
import subprocess
from pathlib import Path

from . import runtime_memory as mem


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


def check_handoff(repo: str, checkpoint: str, sha: str, task_id: str | None = None) -> dict:
    """Local preconditions + promoted-authorization binding.

    Returns {'ok': bool, 'refusals': [...]}. Without a matching
    PROMOTED/SUPERVISOR_READY authorization for task_id, a SHA that merely
    equals HEAD/origin-main is still refused.
    """
    refusals = []
    if not validate_sha(sha):
        return {"ok": False, "refusals": [f"invalid SHA: {sha!r}"]}
    if task_id is not None:
        with mem.LockedState(repo) as locked:
            record = locked.read_promotion(task_id)
            run = locked.read_run(task_id)
        if run.get("status") not in ("PROMOTED", "SUPERVISOR_READY"):
            refusals.append(f"refusing: no promoted authorization for task {task_id}")
        elif record.get("candidate_sha") != sha:
            refusals.append("refusing: SHA != promoted candidate SHA")
        elif record.get("checkpoint") != checkpoint:
            refusals.append("refusing: checkpoint != promoted checkpoint")
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
                          timeout_s: int = 7800, invoke=None, task_id: str | None = None) -> dict:
    """Verify preconditions + authorization, then invoke the supervisor (or a fake in tests)."""
    precheck = check_handoff(repo, checkpoint, sha, task_id=task_id)
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
