"""Deterministic candidate diff guards — enforcement, not prompt text.

Protection source of truth is scripts/agent_supervisor.py, loaded directly
from the repository under review (importlib by file path, no sys.path games,
no supervisor modification). The tracked PROTECTED_PATHS.json is
supplemental metadata only and never consulted for decisions.

Runs BEFORE independent review. Guards flag; the reviewer handles semantics.
"""
from __future__ import annotations

import importlib.util
import subprocess
from pathlib import Path

from . import scope as path_scope

# Cheap textual signals; each yields a flag string, never a verdict alone.
_GAMING_PATTERNS = (
    ("continue-on-error", "workflow failure tolerance added"),
    ("|| true", "shell failure masked"),
)

_supervisor_cache: dict[str, object] = {}


def load_supervisor(repo: str):
    """Load the repository's agent_supervisor module without importing by name."""
    key = str(Path(repo).resolve())
    if key not in _supervisor_cache:
        location = Path(repo) / "scripts" / "agent_supervisor.py"
        spec = importlib.util.spec_from_file_location("_or_supervisor_truth", location)
        if spec is None or spec.loader is None:
            raise ImportError(f"cannot load supervisor from {location}")
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        _supervisor_cache[key] = module
    return _supervisor_cache[key]


def is_supervisor_protected(repo: str, path: str) -> bool:
    """Authoritative predicate: agrees with the supervisor for every path."""
    module = load_supervisor(repo)
    return bool(module.is_protected_execution_path(path_scope.normalize(path)))


def is_protected_with(module, path: str) -> bool:
    """Same predicate against an explicitly provided supervisor module."""
    return bool(module.is_protected_execution_path(path_scope.normalize(path)))


def changed_entries(repo: str, base_sha: str, candidate_sha: str) -> list[tuple[str, str | None, str]]:
    """Unambiguous NUL-delimited rename/copy-aware change list.

    Returns (status, old_path, new_path) with A/M/D/R/C parsed; for R/C
    BOTH sides are recorded. A delete+add pair (no rename detected) also
    surfaces both paths, so a protected deletion can never hide behind an
    allowed addition.
    """
    out = subprocess.run(
        ["git", "diff", "--name-status", "-z", "--find-renames", "--find-copies",
         f"{base_sha}..{candidate_sha}"],
        cwd=repo,
        capture_output=True,
        text=True,
        check=True,
        timeout=60,
    ).stdout
    parts = out.split("\0")
    entries: list[tuple[str, str | None, str]] = []
    index = 0
    while index < len(parts):
        token = parts[index]
        index += 1
        if not token:
            continue
        status = token[0]
        if status in ("R", "C"):
            if index + 1 >= len(parts):
                break
            old, new = parts[index], parts[index + 1]
            index += 2
            if old and new:
                entries.append((status, path_scope.normalize(old), path_scope.normalize(new)))
        elif status in ("A", "M", "D", "T", "U"):
            if index >= len(parts):
                break
            path = parts[index]
            index += 1
            if path:
                entries.append((status, None, path_scope.normalize(path)))
    return entries


def changed_paths(repo: str, base_sha: str, candidate_sha: str) -> list[str]:
    """Every path touched by the change: for renames/copies BOTH sides."""
    paths: set[str] = set()
    for status, old, new in changed_entries(repo, base_sha, candidate_sha):
        if old:
            paths.add(old)
        paths.add(new)
    return sorted(paths)


def guard_protected_paths(
    repo: str,
    paths: list[str],
    plan_allowlist: tuple[str, ...] = (),
    supervisor=None,
) -> list[str]:
    """Violation flags for unauthorized supervisor-protected edits.

    Exact-path PLAN allowance uses the same semantics as the supervisor:
    a protected path passes only when exactly listed in the checkpoint's
    `runner_allowed_protected_paths`. `supervisor` may be an already-loaded
    supervisor module (tests); default loads it from `repo` under review.
    """
    module = supervisor if supervisor is not None else load_supervisor(repo)
    allowed = {path_scope.normalize(p) for p in plan_allowlist}
    return [
        f"protected-path: {p}"
        for p in paths
        if bool(module.is_protected_execution_path(path_scope.normalize(p)))
        and path_scope.normalize(p) not in allowed
    ]


def guard_scope(
    changed: list[str],
    allowed: list[str],
    forbidden: list[str],
) -> list[str]:
    """Machine-enforced task scope. Forbidden wins over allowed."""
    result = path_scope.scope_violations(changed, allowed, forbidden)
    flags = [f"out-of-scope: {p}" for p in result["out_of_scope"]]
    flags += [f"forbidden-path: {p}" for p in result["forbidden"]]
    return flags


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
    allowed: list[str] | None = None,
    forbidden: list[str] | None = None,
    plan_allowlist: tuple[str, ...] = (),
    supervisor=None,
) -> dict:
    """Full deterministic gate. `accepted` is False on ANY violation.

    Unresolved anti-gaming flags block automatic promotion: they never yield
    PROMOTION_READY (caller routes to REVIEW_PENDING or REJECTED).
    """
    paths = changed_paths(repo, base_sha, candidate_sha)
    protected = guard_protected_paths(repo, paths, plan_allowlist, supervisor=supervisor)
    scoped = guard_scope(paths, allowed or [], forbidden or [])
    gaming = guard_anti_gaming(repo, base_sha, candidate_sha)
    violations = protected + scoped
    return {
        "changed_paths": paths,
        "protected_violations": protected,
        "scope_violations": scoped,
        "gaming_flags": gaming,
        "accepted": not violations,
        "promotion_blocked_by_gaming": bool(gaming),
    }
