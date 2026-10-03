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
    """Advisory router. Real path invokes the configured Jev model through
    `opencode run` with structured facts only; output must be exactly one
    allowed label. Any failure falls back to deterministic routing."""

    def __init__(self, available: bool = False, model: str | None = None,
                 opencode_bin: str = "opencode", timeout_s: int = 120) -> None:
        self.available = available
        self.model = model
        self.opencode_bin = opencode_bin
        self.timeout_s = timeout_s

    def invoke(self, facts: dict) -> str:
        """Real Jev call. Raises on any problem; caller falls back."""
        import json as _json
        import subprocess as _sp

        from . import events

        if not self.available or not self.model:
            raise RuntimeError("Jev unavailable")
        prompt = (
            "Classify these orchestration facts. Reply with EXACTLY one of: "
            + ", ".join(ROUTER_LABELS) + ". No other text.\n"
            f"FACTS={_json.dumps(facts, sort_keys=True)}"
        )
        proc = _sp.run(
            [self.opencode_bin, "run", "--model", self.model,
             "--format", "json", prompt],
            capture_output=True, text=True, timeout=self.timeout_s,
        )
        if proc.returncode != 0:
            raise RuntimeError(f"Jev tool failure: {proc.stderr[-500:]}")
        return events.extract_label(proc.stdout, ROUTER_LABELS)

    def advise(self, facts: dict, scripted: str | None = None) -> dict:
        if scripted is not None:
            label = scripted
            if not validate_router_label(label):
                return {"label": None, "used": False, "reason": f"invalid label: {label}"}
            return {"label": label, "used": True, "reason": "advisory"}
        if not self.available:
            return {"label": None, "used": False, "reason": "Jev unavailable; deterministic routing"}
        try:
            label = self.invoke(facts)
        except Exception as error:  # noqa: BLE001 - any failure falls back
            return {"label": None, "used": False, "reason": f"Jev failed, deterministic routing: {error}"}
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
