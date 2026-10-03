"""Independent review dispatch, family independence, verdicts, integrity.

The implementer never self-approves. Reviewers report structured verdicts;
they never mark DONE. Candidate SHA/worktree integrity is authoritative —
reviewer prose never overrides it. Reviewer mutation invalidates the review.
"""
from __future__ import annotations

import json
import subprocess

from . import worker as worker_mod

VERDICTS = ("PASS", "DEFECT_FOUND", "BLOCKED", "INCONCLUSIVE")


def reviewer_candidates(role_policy: dict, discovered: list[dict], exclude_family: str) -> list[str]:
    preferred = role_policy.get("roles", {}).get("review", {}).get("preferred", [])
    available = {m["model_id"] for m in discovered if m.get("state") in ("AVAILABLE", "TEMPORARILY_FREE")}
    out = []
    for candidate in preferred:
        model_id = candidate.get("model")
        if model_id in available and candidate.get("family", "unknown") != exclude_family:
            out.append(model_id)
    return out


def review_decision(
    *,
    implementer_family: str,
    reviewer_model: str | None,
    reviewer_family: str | None,
) -> str:
    """APPROVED_ROUTE | REVIEW_PENDING. No independent family => pending."""
    if not reviewer_model or not reviewer_family:
        return "REVIEW_PENDING"
    if reviewer_family == implementer_family:
        return "REVIEW_PENDING"
    return "APPROVED_ROUTE"


def validate_review_report(report: dict, schemas: dict) -> list[str]:
    required = schemas.get("TASK_SCHEMAS.json", {}).get("review_report_required", [])
    errors = [f"review_report.{name}: missing" for name in required if name not in report]
    if "verdict" in report and report["verdict"] not in VERDICTS:
        errors.append(f"review_report.verdict: arbitrary prose rejected: {report['verdict']!r}")
    return errors


def build_review_prompt(*, task_id: str, base_sha: str, candidate_sha: str,
                        changed_paths: list[str], guard_summary: dict) -> str:
    return (
        "You are an independent reviewer, not the implementer. Review ONLY; "
        "do not modify the repository, do not commit, do not spawn workers. "
        "Return a JSON object with keys verdict (one of PASS, DEFECT_FOUND, "
        "BLOCKED, INCONCLUSIVE), findings (list), tests_rerun (list). "
        "Anything else is rejected.\n"
        f"REVIEW={json.dumps({'task_id': task_id, 'base_sha': base_sha, 'candidate_sha': candidate_sha, 'changed_paths': changed_paths, 'guards': guard_summary}, sort_keys=True)}"
    )


class ReviewAdapter:
    name = "base"
    agent = worker_mod.REVIEWER_AGENT

    def review(self, *, worktree: str, prompt: str, timeout_s: int) -> dict:
        raise NotImplementedError


class FakeReviewAdapter(ReviewAdapter):
    """Scripted reviewer. `mutate=True` simulates a reviewer that edits the tree."""

    name = "fake-review"

    def __init__(self, verdict: str = "PASS", findings: list[str] | None = None,
                 mutate: bool = False) -> None:
        self.verdict = verdict
        self.findings = findings or []
        self.mutate = mutate

    def review(self, *, worktree: str, prompt: str, timeout_s: int) -> dict:
        _ = (prompt, timeout_s)
        if self.mutate:
            import os

            with open(os.path.join(worktree, "REVIEWER_TOUCHED.txt"), "w") as handle:
                handle.write("reviewer mutated the tree\n")
        return {"verdict": self.verdict, "findings": list(self.findings), "tests_rerun": []}


class OpenCodeReviewAdapter(ReviewAdapter):
    """Real reviewer via `opencode run --agent orch-reviewer --model <id>`."""

    name = "opencode-review"

    def __init__(self, opencode_bin: str = "opencode", model: str = "") -> None:
        self.opencode_bin = opencode_bin
        self.model = model

    def review(self, *, worktree: str, prompt: str, timeout_s: int) -> dict:
        try:
            proc = subprocess.run(
                [self.opencode_bin, "run", "--agent", self.agent,
                 "--model", self.model, "--dir", worktree, "--format", "json", prompt],
                capture_output=True, text=True, timeout=timeout_s,
            )
        except (OSError, subprocess.SubprocessError) as error:
            return {"verdict": "INCONCLUSIVE", "findings": [f"reviewer tool error: {error}"], "tests_rerun": []}
        if proc.returncode != 0:
            return {"verdict": "INCONCLUSIVE", "findings": [proc.stderr[-1000:]], "tests_rerun": []}
        try:
            report = json.loads(proc.stdout.strip().splitlines()[-1])
        except (ValueError, IndexError):
            return {"verdict": "INCONCLUSIVE", "findings": ["unparseable reviewer output"], "tests_rerun": []}
        if not isinstance(report, dict) or report.get("verdict") not in VERDICTS:
            return {"verdict": "INCONCLUSIVE", "findings": ["reviewer verdict rejected"], "tests_rerun": []}
        return report


def execute_review(*, worktree: str, task_id: str, base_sha: str, candidate_sha: str,
                   changed_paths: list[str], guard_summary: dict,
                   adapter: ReviewAdapter, schemas: dict, timeout_s: int = 600) -> dict:
    """Run review with worktree integrity checks before and after.

    Returns {'ok', 'verdict', 'report', 'errors'}. Mutation or HEAD change
    during review invalidates it regardless of prose.
    """
    try:
        head_before = worker_mod.git_head(worktree)
        dirty_before = worker_mod.git_dirty_paths(worktree)
    except subprocess.SubprocessError as error:
        return {"ok": False, "verdict": "INCONCLUSIVE", "report": {}, "errors": [f"worktree unreadable: {error}"]}
    if head_before != candidate_sha or dirty_before:
        return {"ok": False, "verdict": "INCONCLUSIVE", "report": {},
                "errors": ["candidate worktree not clean at candidate SHA before review"]}
    prompt = build_review_prompt(task_id=task_id, base_sha=base_sha, candidate_sha=candidate_sha,
                                 changed_paths=changed_paths, guard_summary=guard_summary)
    report = adapter.review(worktree=worktree, prompt=prompt, timeout_s=timeout_s)
    errors = validate_review_report(
        {"task_id": task_id, "candidate_sha": candidate_sha,
         "reviewer_model": adapter.name, "reviewer_family": "?"} | report, schemas)
    try:
        head_after = worker_mod.git_head(worktree)
        dirty_after = worker_mod.git_dirty_paths(worktree)
    except subprocess.SubprocessError as error:
        return {"ok": False, "verdict": "INCONCLUSIVE", "report": report,
                "errors": errors + [f"worktree unreadable after review: {error}"]}
    if head_after != candidate_sha or dirty_after:
        return {"ok": False, "verdict": "INCONCLUSIVE", "report": report,
                "errors": errors + ["reviewer mutated repository: review invalid"]}
    if errors:
        return {"ok": False, "verdict": "INCONCLUSIVE", "report": report, "errors": errors}
    return {"ok": True, "verdict": report["verdict"], "report": report, "errors": []}
