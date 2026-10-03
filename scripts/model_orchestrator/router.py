"""Deterministic router plus Jev advisory adapter.

Jev receives structured facts and picks among FIXED labels. Its output is
advisory: deterministic policy validates it. If Jev is unavailable (as in
the installed environment), routing continues deterministically.
"""
from __future__ import annotations

ROUTER_LABELS = (
    "CAUSAL_PRODUCT_REPAIR",
    "HARNESS_INVESTIGATION",
    "HOST_DIAGNOSTIC",
    "ARCHITECTURE_ESCALATION",
    "REVIEW_REQUIRED",
    "STOP_NEEDS_EVIDENCE",
)


def deterministic_route(facts: dict) -> str:
    """Pure function of structured facts; works with zero models."""
    if facts.get("needs_architecture"):
        return "ARCHITECTURE_ESCALATION"
    if facts.get("candidate_ready"):
        return "REVIEW_REQUIRED"
    if facts.get("hosted_inconclusive") and not facts.get("attempts_remaining", True):
        return "HOST_DIAGNOSTIC"
    if facts.get("evidence_ready"):
        return "CAUSAL_PRODUCT_REPAIR"
    if facts.get("harness_suspect"):
        return "HARNESS_INVESTIGATION"
    if facts.get("attempts_remaining", True):
        return "CAUSAL_PRODUCT_REPAIR"
    return "STOP_NEEDS_EVIDENCE"


def validate_router_label(label: str) -> bool:
    return label in ROUTER_LABELS


class JevAdapter:
    """Advisory router. Fake-backed in tests; real calls go through opencode run."""

    def __init__(self, available: bool = False) -> None:
        self.available = available

    def advise(self, facts: dict, scripted: str | None = None) -> dict:
        if scripted is not None:
            label = scripted
        elif not self.available:
            return {"label": None, "used": False, "reason": "Jev unavailable; deterministic routing"}
        else:  # pragma: no cover — real model call, never in tests
            raise NotImplementedError("real Jev call dispatches via opencode run")
        if not validate_router_label(label):
            return {"label": None, "used": False, "reason": f"invalid label: {label}"}
        return {"label": label, "used": True, "reason": "advisory"}


def route(facts: dict, jev: JevAdapter, scripted_jev: str | None = None) -> dict:
    deterministic = deterministic_route(facts)
    advisory = jev.advise(facts, scripted=scripted_jev)
    chosen = deterministic
    if advisory.get("used") and advisory.get("label") != deterministic:
        # Advisory disagreement is recorded, never followed blindly.
        return {"decision": chosen, "deterministic": deterministic,
                "advisory": advisory, "disagreement": True}
    return {"decision": chosen, "deterministic": deterministic,
            "advisory": advisory, "disagreement": False}
