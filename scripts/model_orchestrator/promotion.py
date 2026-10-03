"""Candidate -> main promotion gate: fast-forward-only, remote-first.

Never executed against the real repository in repair/test tasks — tests use
temporary repositories and bare remotes. No rebase, no force push, no merge
commit. If the remote advanced, STOP.
"""
from __future__ import annotations

import subprocess


def _git(repo: str, *args: str) -> subprocess.CompletedProcess:
    return subprocess.run(["git", *args], cwd=repo, capture_output=True, text=True, timeout=60)


def _head(repo: str) -> str:
    return _git(repo, "rev-parse", "HEAD").stdout.strip()


def _clean(repo: str) -> bool:
    return _git(repo, "status", "--porcelain").stdout.strip() == ""


def check_promotion(
    *,
    main_repo: str,
    candidate_repo: str,
    candidate_sha: str,
    reviewed_sha: str,
    base_sha: str,
    expected_checkpoint: str,
    state_next: str,
    no_merge_commits: bool = True,
) -> dict:
    """Validate all 11 preconditions. Returns {'ok': bool, 'failures': [...]}."""
    failures = []
    origin_main = ""
    if candidate_sha != reviewed_sha:
        failures.append("candidate SHA != reviewed SHA")
    try:
        cand_clean = _clean(candidate_repo)
    except Exception as error:  # noqa: BLE001
        return {"ok": False, "failures": [f"candidate repo unreadable: {error}"]}
    if not cand_clean:
        failures.append("candidate worktree not clean")
    if _head(candidate_repo) != candidate_sha:
        failures.append("candidate worktree HEAD != candidate SHA")
    ancestry = _git(candidate_repo, "merge-base", "--is-ancestor", base_sha, candidate_sha)
    if ancestry.returncode != 0:
        failures.append("candidate does not descend from task base SHA")
    if no_merge_commits:
        count = _git(candidate_repo, "rev-list", "--count", "--merges", f"{base_sha}..{candidate_sha}")
        if count.stdout.strip() not in ("", "0"):
            failures.append("merge commits present but linear history required")
    if _git(main_repo, "rev-parse", "--abbrev-ref", "HEAD").stdout.strip() != "main":
        failures.append("authoritative worktree not on main")
    if not _clean(main_repo):
        failures.append("authoritative main worktree not clean")
    _git(main_repo, "fetch", "origin")
    origin_main = _git(main_repo, "rev-parse", "origin/main").stdout.strip()
    if origin_main != base_sha:
        failures.append(f"origin/main {origin_main[:8]} != expected base {base_sha[:8]}; remote advanced")
    if _head(main_repo) != origin_main:
        failures.append("local main != fetched origin/main")
    if state_next != expected_checkpoint:
        failures.append(f"checkpoint {expected_checkpoint} no longer NEXT")
    return {"ok": not failures, "failures": failures, "origin_main": origin_main}


def promote(*, main_repo: str, candidate_sha: str, precheck: dict) -> dict:
    """Fast-forward-only promotion. Call ONLY after check_promotion ok.

    Remote-first: push candidate SHA to refs/heads/main WITHOUT force, then
    verify origin/main == candidate, then fast-forward local main.
    """
    if not precheck.get("ok"):
        return {"ok": False, "error": "precheck failed", "failures": precheck.get("failures", [])}
    pushed = _git(main_repo, "push", "origin", f"{candidate_sha}:refs/heads/main")
    if pushed.returncode != 0:
        return {"ok": False, "error": "push rejected (remote advanced or hook)", "stderr": pushed.stderr[-1000:]}
    _git(main_repo, "fetch", "origin")
    origin_main = _git(main_repo, "rev-parse", "origin/main").stdout.strip()
    if origin_main != candidate_sha:
        return {"ok": False, "error": "origin/main != candidate after push"}
    ff = _git(main_repo, "merge", "--ff-only", "origin/main")
    if ff.returncode != 0:
        return {"ok": False, "error": "local main fast-forward failed", "stderr": ff.stderr[-1000:]}
    final = _head(main_repo)
    if final != candidate_sha:
        return {"ok": False, "error": "local HEAD != candidate after fast-forward"}
    return {"ok": True, "head": final}
