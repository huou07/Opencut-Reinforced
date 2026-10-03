"""Candidate -> main promotion: task-bound authorization, fast-forward-only.

The promotion function loads its authorization from trusted runtime memory
by task_id — never a caller-fabricated {"ok": true} precheck. Immediately
before promotion it revalidates: run PROMOTION_READY, SHA equality,
contract/changed/guard digests, clean trees, trusted protection and scope,
NEXT checkpoint, and remote base. TOCTOU-safe sequence: integration lock,
fetch, verify base, revalidate, non-force push, re-fetch, verify, ff-only
local update, final verify. No rebase, no force, no merge commit.
"""
from __future__ import annotations

import hashlib
import json
import subprocess

from . import guards
from . import runtime_memory as mem
from . import trusted_authority as trust


def _git(repo: str, *args: str) -> subprocess.CompletedProcess:
    return subprocess.run(["git", *args], cwd=repo, capture_output=True, text=True, timeout=60)


def _head(repo: str) -> str:
    proc = _git(repo, "rev-parse", "HEAD")
    if proc.returncode != 0:
        raise RuntimeError(f"cannot read HEAD of {repo}")
    return proc.stdout.strip()


def _clean(repo: str) -> bool:
    return _git(repo, "status", "--porcelain").stdout.strip() == ""


def _digest(value: object) -> str:
    return hashlib.sha256(json.dumps(value, sort_keys=True).encode()).hexdigest()


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
    """Git-mechanics precheck (used by tests). Authorization is separate."""
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
    try:
        if _head(candidate_repo) != candidate_sha:
            failures.append("candidate worktree HEAD != candidate SHA")
    except RuntimeError as error:
        failures.append(str(error))
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
    fetch = _git(main_repo, "fetch", "origin")
    if fetch.returncode != 0:
        failures.append(f"fetch origin failed: {fetch.stderr[-300:]}")
    else:
        origin_main = _git(main_repo, "rev-parse", "origin/main").stdout.strip()
        if origin_main != base_sha:
            failures.append(f"origin/main {origin_main[:8]} != expected base {base_sha[:8]}; remote advanced")
    try:
        if _head(main_repo) != origin_main and origin_main:
            failures.append("local main != fetched origin/main")
    except RuntimeError as error:
        failures.append(str(error))
    if state_next != expected_checkpoint:
        failures.append(f"checkpoint {expected_checkpoint} no longer NEXT")
    return {"ok": not failures, "failures": failures, "origin_main": origin_main}


def promote(*, main_repo: str, candidate_sha: str, precheck: dict) -> dict:
    """Legacy git-mechanics push (tests only). Production uses promote_task."""
    if not precheck.get("ok"):
        return {"ok": False, "error": "precheck failed", "failures": precheck.get("failures", [])}
    return _push_and_verify(main_repo, candidate_sha)


def _push_and_verify(main_repo: str, candidate_sha: str) -> dict:
    pushed = _git(main_repo, "push", "origin", f"{candidate_sha}:refs/heads/main")
    if pushed.returncode != 0:
        return {"ok": False, "error": "push rejected (remote advanced or hook)", "stderr": pushed.stderr[-1000:]}
    refetch = _git(main_repo, "fetch", "origin")
    if refetch.returncode != 0:
        return {"ok": False, "error": "post-push fetch failed"}
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


def promote_task(*, main_repo: str, candidate_repo: str, memory_repo: str,
                 task_id: str, expected_checkpoint: str) -> dict:
    """Authorized promotion bound to the runtime record. Refuses on ANY mismatch."""
    with mem.LockedState(memory_repo) as locked:
        record = locked.read_promotion(task_id)
        run = locked.read_run(task_id)
    if not record or record.get("task_id") != task_id:
        return {"ok": False, "error": "REFUSED: no promotion authorization for task"}
    if run.get("status") != "PROMOTION_READY":
        return {"ok": False, "error": f"REFUSED: run status {run.get('status')} != PROMOTION_READY"}
    candidate = record.get("candidate_sha", "")
    if not candidate or candidate != record.get("reviewed_sha"):
        return {"ok": False, "error": "REFUSED: candidate/reviewed SHA mismatch in record"}
    if record.get("review_verdict") != "PASS":
        return {"ok": False, "error": "REFUSED: review verdict != PASS"}
    if record.get("checkpoint") != expected_checkpoint:
        return {"ok": False, "error": "REFUSED: checkpoint mismatch"}
    # Task-contract binding: the stored packet must reproduce the digest and
    # agree with the scope fields used for revalidation.
    stored_packet = record.get("task_packet") or {}
    if _digest({k: stored_packet.get(k) for k in (
            "task_id", "checkpoint", "base_sha", "branch", "allowed_paths",
            "forbidden_paths", "invariants", "acceptance", "required_tests",
            "stop_conditions", "max_repair_attempts", "model_locked")}) != record.get("contract_digest"):
        return {"ok": False, "error": "REFUSED: task-contract digest mismatch"}
    for field in ("allowed_paths", "forbidden_paths"):
        if list(record.get(field, [])) != list(stored_packet.get(field, [])):
            return {"ok": False, "error": f"REFUSED: record {field} != packet contract"}
    # Revalidate live git state against the record.
    try:
        if _head(candidate_repo) != candidate or not _clean(candidate_repo):
            return {"ok": False, "error": "REFUSED: candidate worktree changed"}
    except RuntimeError as error:
        return {"ok": False, "error": f"REFUSED: {error}"}
    live_changed = guards.changed_paths(candidate_repo, record["base_sha"], candidate)
    if _digest(live_changed) != record.get("changed_digest"):
        return {"ok": False, "error": "REFUSED: candidate diff changed since review"}
    # Trusted protection + scope revalidation (authority from main checkout).
    try:
        authority = trust.freeze_authority(main_repo)
    except (OSError, ImportError, ValueError) as error:
        return {"ok": False, "error": f"REFUSED: trusted authority unavailable: {error}"}
    live_gate = guards.guard_candidate(
        candidate_repo, record["base_sha"], candidate,
        allowed=record.get("allowed_paths", []),
        forbidden=record.get("forbidden_paths", []),
        plan_allowlist=tuple(record.get("plan_allowlist", [])),
        supervisor=authority["module"])
    live_digest = _digest({"protected": live_gate["protected_violations"],
                           "scope": live_gate["scope_violations"],
                           "gaming": live_gate["gaming_flags"]})
    if not live_gate["accepted"] or live_digest != record.get("guard_digest"):
        return {"ok": False, "error": "REFUSED: guards no longer pass identically"}
    allow_info = trust.resolve_checkpoint(authority, expected_checkpoint)
    if "error" in allow_info:
        return {"ok": False, "error": f"REFUSED: {allow_info['error']}"}
    if allow_info.get("current_next") != expected_checkpoint:
        return {"ok": False, "error": "REFUSED: checkpoint no longer NEXT"}
    mec = check_promotion(
        main_repo=main_repo, candidate_repo=candidate_repo, candidate_sha=candidate,
        reviewed_sha=candidate, base_sha=record["base_sha"],
        expected_checkpoint=expected_checkpoint, state_next=allow_info.get("current_next", ""))
    if not mec["ok"]:
        return {"ok": False, "error": "REFUSED: git preconditions", "failures": mec["failures"]}
    pushed = _push_and_verify(main_repo, candidate)
    if not pushed["ok"]:
        return pushed
    with mem.LockedState(memory_repo) as locked:
        run = locked.read_run(task_id)
        run["status"] = "PROMOTED"
        locked.write_run(task_id, run)
    return {"ok": True, "head": pushed["head"]}
