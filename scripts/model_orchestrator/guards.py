"""Deterministic candidate diff guards — enforcement, not prompt text.

Runs BEFORE independent review. Simple grep-style guards flag suspicious
changes; the reviewer handles semantics. Rejection is deterministic.
"""
from __future__ import annotations

import subprocess

# Cheap textual signals; each yields a flag string, never a verdict alone.
_GAMING_PATTERNS = (
    ("continue-on-error", "workflow failure tolerance added"),
    ("|| true", "shell failure masked"),
)


def changed_paths(repo: str, base_sha: str, candidate_sha: str) -> list[str]:
    out = subprocess.run(
        ["git", "diff", "--name-only", f"{base_sha}..{candidate_sha}"],
        cwd=repo,
        capture_output=True,
        text=True,
        check=True,
        timeout=60,
    ).stdout
    return sorted(p for p in out.splitlines() if p.strip())


def is_protected(path: str, protected: dict, plan_allowlist: tuple[str, ...] = ()) -> bool:
    if path in plan_allowlist:
        return False
    if path in protected.get("protected_exact", []):
        return True
    if any(path.startswith(prefix) for prefix in protected.get("protected_prefixes", [])):
        return True
    name = path.rsplit("/", 1)[-1]
    return name in protected.get("protected_basenames", [])


def guard_protected_paths(
    paths: list[str], protected: dict, plan_allowlist: tuple[str, ...] = ()
) -> list[str]:
    """Return violation flags for unauthorized protected-path edits."""
    return [f"protected-path: {p}" for p in paths if is_protected(p, protected, plan_allowlist)]


def guard_anti_gaming(repo: str, base_sha: str, candidate_sha: str) -> list[str]:
    """Flag retry/timeout/weakening signals in authoritative surfaces."""
    try:
        out = subprocess.run(
            ["git", "diff", "-U0", f"{base_sha}..{candidate_sha}", "--",
             ".github/workflows/", "scripts/", "apps/or_app/integration_test/",
             "apps/or_app/test_driver/"],
            cwd=repo,
            capture_output=True,
            text=True,
            check=True,
            timeout=60,
        ).stdout
    except subprocess.SubprocessError:
        return ["diff-unavailable"]
    flags = []
    for line in out.splitlines():
        if not line.startswith("+") or line.startswith("+++"):
            continue
        body = line[1:]
        for pattern, label in _GAMING_PATTERNS:
            if pattern in body:
                flags.append(f"anti-gaming: {label}: {body.strip()[:120]}")
        lowered = body.lower()
        if "timeout" in lowered and any(t in lowered for t in ("increase", "7200", "3600")):
            flags.append(f"anti-gaming: timeout change needs review: {body.strip()[:120]}")
    return flags


def guard_candidate(
    repo: str,
    base_sha: str,
    candidate_sha: str,
    protected: dict,
    plan_allowlist: tuple[str, ...] = (),
) -> dict:
    paths = changed_paths(repo, base_sha, candidate_sha)
    violations = guard_protected_paths(paths, protected, plan_allowlist)
    flags = guard_anti_gaming(repo, base_sha, candidate_sha)
    return {
        "changed_paths": paths,
        "protected_violations": violations,
        "gaming_flags": flags,
        "accepted": not violations,
    }
