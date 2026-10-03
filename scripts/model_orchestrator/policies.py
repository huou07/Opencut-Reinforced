"""Tracked-policy loading and validation (docs/execution/automation/*.json)."""
from __future__ import annotations

import json
from pathlib import Path

POLICY_FILES = (
    "MODEL_POLICY.json",
    "ROUTING_POLICY.json",
    "ESCALATION_POLICY.json",
    "PROTECTED_PATHS.json",
    "TASK_SCHEMAS.json",
)


def automation_dir(repo: str | Path) -> Path:
    return Path(repo) / "docs" / "execution" / "automation"


def load_policies(repo: str | Path) -> dict[str, dict]:
    base = automation_dir(repo)
    policies = {}
    for name in POLICY_FILES:
        with open(base / name, encoding="utf-8") as handle:
            policies[name] = json.load(handle)
    return policies


def validate_policies(policies: dict[str, dict]) -> list[str]:
    errors = []
    for name in POLICY_FILES:
        doc = policies.get(name)
        if not isinstance(doc, dict) or doc.get("schema_version") != 1:
            errors.append(f"{name}: missing or bad schema_version")
    routing = policies.get("ROUTING_POLICY.json", {})
    if routing.get("attempts", {}).get("max_evidence_backed_corrective_attempts_per_signature") != 2:
        errors.append("ROUTING_POLICY.json: attempt budget must stay 2")
    return errors


def protected_paths(policies: dict[str, dict]) -> dict:
    return policies.get("PROTECTED_PATHS.json", {})
