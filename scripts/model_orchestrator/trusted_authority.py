"""Trusted control authority: frozen supervisor predicate + PLAN allowlist.

NEVER use candidate-controlled code, policy, or metadata to decide whether
that candidate is authorized. The orchestrator freezes the supervisor
predicate and PLAN resolution from a TRUSTED checkout (never the task
candidate worktree) before worker launch and reuses the SAME frozen
authority after the worker returns. If the trusted supervisor definition
no longer matches the expected base, fail closed with TRUSTED_POLICY_MISMATCH.
"""
from __future__ import annotations

import hashlib
import importlib.util
import json
import subprocess
import sys
from pathlib import Path

_SUPERVISOR_FILE = Path("scripts") / "agent_supervisor.py"
_PLAN_FILE = Path("docs") / "execution" / "PLAN.json"
_STATE_FILE = Path("docs") / "execution" / "STATE.json"


def _load_module(path: Path):
    spec = importlib.util.spec_from_file_location("_or_trusted_supervisor", path)
    if spec is None or spec.loader is None:
        raise ImportError(f"cannot load supervisor from {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def supervisor_digest(trusted_repo: str) -> str:
    data = (Path(trusted_repo) / _SUPERVISOR_FILE).read_bytes()
    return hashlib.sha256(data).hexdigest()


def freeze_authority(trusted_repo: str) -> dict:
    """Snapshot the trusted authority. Raises on any unreadable surface."""
    trusted = str(Path(trusted_repo).resolve())
    module = _load_module(Path(trusted) / _SUPERVISOR_FILE)
    plan = json.loads((Path(trusted) / _PLAN_FILE).read_text(encoding="utf-8"))
    state = json.loads((Path(trusted) / _STATE_FILE).read_text(encoding="utf-8"))
    return {
        "trusted_repo": trusted,
        "supervisor_digest": hashlib.sha256(
            (Path(trusted) / _SUPERVISOR_FILE).read_bytes()).hexdigest(),
        "module": module,
        "plan": plan,
        "state": state,
    }


def verify_authority_fresh(authority: dict) -> list[str]:
    """Fail closed when the trusted supervisor changed under the orchestrator."""
    if authority.get("injected"):
        return []
    try:
        current = supervisor_digest(authority["trusted_repo"])
    except OSError as error:
        return [f"TRUSTED_POLICY_MISMATCH: trusted supervisor unreadable: {error}"]
    if current != authority["supervisor_digest"]:
        return ["TRUSTED_POLICY_MISMATCH: trusted supervisor changed; orchestrator not refreshed"]
    return []


def is_protected(authority: dict, path: str) -> bool:
    normalized = path.replace("\\", "/")
    while normalized.startswith("./"):
        normalized = normalized[2:]
    return bool(authority["module"].is_protected_execution_path(normalized))


def _execution_plan_module():
    repo = Path(__file__).resolve().parent.parent.parent
    if str(repo / "scripts") not in sys.path:
        sys.path.insert(0, str(repo / "scripts"))
    import execution_plan

    return execution_plan


def resolve_checkpoint(authority: dict, checkpoint: str) -> dict:
    """Authoritative PLAN resolution from the TRUSTED plan/state.

    Returns {'next', 'allowlist'} or {'error'}. Uses the repository's own
    execution_plan contract; no duplicated authorization logic.
    """
    execution_plan = _execution_plan_module()
    try:
        resolution = execution_plan.resolve_goal(
            authority["plan"], authority["state"], f"checkpoint:{checkpoint}")
    except Exception as error:  # noqa: BLE001 - PlanError or validation failure
        return {"error": f"TASK_CONTRACT_MISMATCH: trusted PLAN rejects checkpoint:{checkpoint}: {error}"}
    return {
        "checkpoint_id": resolution.get("checkpoint_id"),
        "current_next": authority["state"].get("current_next"),
        "allowlist": list(resolution.get("runner_allowed_protected_paths", [])),
    }


def git_head(repo: str) -> str:
    return subprocess.run(
        ["git", "rev-parse", "HEAD"], cwd=repo,
        capture_output=True, text=True, check=True, timeout=30,
    ).stdout.strip()
