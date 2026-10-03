"""Failure signatures, attempt tracking, anti-thrashing, hosted classification.

After at most TWO evidence-backed corrective attempts for the same failure
signature: no more repair workers — route to DIAGNOSTIC or
ARCHITECTURE_ESCALATION. Retries/timeout bumps never reset the counter.
"""
from __future__ import annotations

import hashlib
import json

MAX_CORRECTIVE_ATTEMPTS = 2


def normalize_signature(raw: dict) -> dict:
    fields = (
        "checkpoint", "gate", "job", "step",
        "error_class", "assertion", "error_fingerprint",
    )
    return {key: str(raw.get(key, "")) for key in fields}


def signature_id(signature: dict) -> str:
    canonical = json.dumps(normalize_signature(signature), sort_keys=True)
    return hashlib.sha256(canonical.encode()).hexdigest()[:16]


def attempts_for(history: list[dict], signature: dict) -> int:
    want = signature_id(signature)
    return sum(1 for h in history if h.get("signature_id") == want and h.get("evidence_backed"))


def thrash_decision(history: list[dict], signature: dict) -> str:
    """ALLOW_REPAIR | DIAGNOSTIC | ARCHITECTURE_ESCALATION."""
    count = attempts_for(history, signature)
    if count < MAX_CORRECTIVE_ATTEMPTS:
        return "ALLOW_REPAIR"
    error_class = str(signature.get("error_class", ""))
    if error_class in ("architecture", "contract", "licensing", "schema", "security"):
        return "ARCHITECTURE_ESCALATION"
    return "DIAGNOSTIC"


KNOWN_9B_FIXTURE = {
    "candidate_sha": "beaef3b7dba878d705dba07d7b9232860e184f83",
    "hosted_run": "37110360212",
    "local_review": "PASS",
    "android_hosted": "driver connection disposed; ADB offline; SAF journey not reached",
}


def classify_hosted_outcome(
    *,
    product_assertions_reached: bool,
    local_review: str,
    driver_disposed: bool,
    adb_offline: bool,
    same_signature_before_candidate: bool,
    proven_infra_evidence: bool,
) -> str:
    """HOSTED_VERIFY_INCONCLUSIVE unless evidence proves otherwise.

    A pre-candidate pattern is suggestive but never proof; only dedicated
    discriminating evidence may yield PROVEN_INFRASTRUCTURE_FAILURE, and only
    reached product assertions may yield PASS/DEFECT verdicts.
    """
    if product_assertions_reached:
        return "NEEDS_PRODUCT_VERDICT"
    if driver_disposed and adb_offline and local_review == "PASS":
        if proven_infra_evidence:
            return "PROVEN_INFRASTRUCTURE_FAILURE"
        return "HOSTED_VERIFY_INCONCLUSIVE"
    _ = same_signature_before_candidate
    return "INCONCLUSIVE"


def classify_known_9b() -> dict:
    verdict = classify_hosted_outcome(
        product_assertions_reached=False,
        local_review="PASS",
        driver_disposed=True,
        adb_offline=True,
        same_signature_before_candidate=True,
        proven_infra_evidence=False,
    )
    assert verdict == "HOSTED_VERIFY_INCONCLUSIVE"
    return {"fixture": KNOWN_9B_FIXTURE, "verdict": verdict, "auto_rerun": False}
