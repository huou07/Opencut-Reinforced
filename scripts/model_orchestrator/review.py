"""Independent review dispatch and family-independence enforcement.

The implementer never self-approves. If no different-family reviewer is
available, the candidate stays REVIEW_PENDING — never silent self-approval.
"""
from __future__ import annotations


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
    """APPROVED_ROUTE | REVIEW_PENDING. Reviewers report; they never mark DONE."""
    if not reviewer_model or not reviewer_family:
        return "REVIEW_PENDING"
    if reviewer_family == implementer_family:
        return "REVIEW_PENDING"
    return "APPROVED_ROUTE"


def validate_review_report(report: dict, schemas: dict) -> list[str]:
    required = schemas.get("TASK_SCHEMAS.json", {}).get("review_report_required", [])
    return [f"review_report.{name}: missing" for name in required if name not in report]
