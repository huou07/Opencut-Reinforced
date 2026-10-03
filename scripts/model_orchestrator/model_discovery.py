"""Model discovery — never guess IDs; ask the installed environment.

Availability states: AVAILABLE, UNAVAILABLE, TEMPORARILY_FREE,
PAID_FALLBACK, EXPERIMENTAL, QUOTA_DEFERRED, AUTH_REQUIRED, TOOL_ERROR.
"""
from __future__ import annotations

import subprocess
from dataclasses import dataclass


@dataclass
class ModelInfo:
    model_id: str
    state: str


def discover_opencode_models(timeout_s: int = 60) -> list[ModelInfo]:
    """List models via `opencode models`. Failure => TOOL_ERROR, never fatal."""
    try:
        out = subprocess.run(
            ["opencode", "models"],
            capture_output=True,
            text=True,
            timeout=timeout_s,
        )
    except (OSError, subprocess.SubprocessError):
        return [ModelInfo(model_id="", state="TOOL_ERROR")]
    if out.returncode != 0:
        return [ModelInfo(model_id="", state="TOOL_ERROR")]
    infos = []
    for line in out.stdout.splitlines():
        name = line.strip()
        if not name or "/" not in name:
            continue
        state = "AVAILABLE"
        if name.endswith("-free"):
            state = "TEMPORARILY_FREE"
        elif name.startswith("opencode/"):
            state = "AVAILABLE"
        else:
            state = "PAID_FALLBACK"
        infos.append(ModelInfo(model_id=name, state=state))
    return infos


def _state_of(entry) -> tuple[str, str]:
    if isinstance(entry, dict):
        return entry.get("model_id", ""), entry.get("state", "")
    return entry.model_id, entry.state


def model_available(model_id: str, discovered: list) -> bool:
    return any(mid == model_id and state in ("AVAILABLE", "TEMPORARILY_FREE")
               for mid, state in (_state_of(m) for m in discovered))


def select_for_role(role: str, policy: dict, discovered: list[ModelInfo]) -> tuple[str | None, str]:
    """Return (model_id, availability). None => deterministic fallback continues."""
    for candidate in policy.get("roles", {}).get(role, {}).get("preferred", []):
        model_id = candidate.get("model")
        if not model_id:
            continue
        if model_available(model_id, discovered):
            return model_id, "READY"
    return None, "UNAVAILABLE"


def model_family(model_id: str, policy: dict) -> str:
    entry = policy_entry_for(model_id, policy)
    return entry.get("family", "unknown") if entry else "unknown"


def policy_entry_for(model_id: str, policy: dict) -> dict:
    for role in policy.get("roles", {}).values():
        for candidate in role.get("preferred", []):
            if candidate.get("model") == model_id:
                return candidate
    return {}
