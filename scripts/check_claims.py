#!/usr/bin/env python3
"""Validate claim records under docs/evidence/claims/.

Deterministic, read-only, stdlib only. Run from the repository root:

    python3 scripts/check_claims.py

Exits 0 when every record is well formed, 1 otherwise. This proves that a claim
was recorded honestly and machine-readably. It does not prove the claim is
true, important, or accepted for release.
"""
from __future__ import annotations

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CLAIMS_DIR = ROOT / "docs" / "evidence" / "claims"
ROADMAP = ROOT / "docs" / "PRODUCT_ROADMAP.md"

FILENAME = re.compile(r"^(\d{4}-\d{2}-\d{2})-[A-Za-z0-9][A-Za-z0-9._-]*\.json$")
SHA = re.compile(r"^[0-9a-f]{40}$")
TIMESTAMP = re.compile(r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z$")

REQUIRED = ("requirement", "claim", "implementation_sha", "verification",
            "verified_at_utc", "evidence_class", "limitations")
EVIDENCE_CLASSES = {"hosted", "packaged", "emulator", "local-source", "unverified"}


def roadmap_requirements() -> set[str]:
    """Requirement ids the active roadmap defines, if it declares any.

    The roadmap is prose-plus-tables and does not use a rigid id format, so
    this returns an empty set when nothing machine-readable is found. An empty
    set disables that one check rather than failing every record: a false
    failure would train people to ignore the validator.
    """
    if not ROADMAP.is_file():
        return set()
    text = ROADMAP.read_text(encoding="utf-8", errors="ignore")
    found = set(re.findall(r"\b(?:REQ|PR|WR)-[A-Z0-9]+(?:-[A-Z0-9]+)*\b", text))
    return found


def main() -> int:
    if not CLAIMS_DIR.is_dir():
        print("Claim evidence: FAIL (docs/evidence/claims/ is missing)")
        return 1

    records = sorted(p for p in CLAIMS_DIR.glob("*.json"))
    known_requirements = roadmap_requirements()
    problems: list[str] = []

    for path in records:
        name_match = FILENAME.match(path.name)
        if not name_match:
            problems.append(f"{path.name}: filename must be YYYY-MM-DD-<id>.json")

        try:
            data = json.loads(path.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError) as error:
            problems.append(f"{path.name}: not valid JSON ({type(error).__name__})")
            continue

        if not isinstance(data, dict):
            problems.append(f"{path.name}: top level must be an object")
            continue

        # The filename date is the record's verification date. A mismatch means
        # someone renamed or back-dated a receipt.
        if name_match:
            stamp = data.get("verified_at_utc")
            if isinstance(stamp, str) and stamp[:10] != name_match.group(1):
                problems.append(
                    f"{path.name}: filename date must match verified_at_utc "
                    f"({stamp[:10]}), not {name_match.group(1)}")

        for field in REQUIRED:
            if field not in data:
                problems.append(f"{path.name}: missing required field '{field}'")

        sha = data.get("implementation_sha", "")
        if sha and not SHA.match(str(sha)):
            problems.append(
                f"{path.name}: implementation_sha must be a full 40-char SHA, got {sha!r}")

        stamp = data.get("verified_at_utc", "")
        if stamp and not TIMESTAMP.match(str(stamp)):
            problems.append(
                f"{path.name}: verified_at_utc must be UTC ISO-8601 "
                f"(YYYY-MM-DDTHH:MM:SSZ), got {stamp!r}")

        evidence_class = data.get("evidence_class")
        if evidence_class and evidence_class not in EVIDENCE_CLASSES:
            problems.append(
                f"{path.name}: evidence_class {evidence_class!r} is not one of "
                f"{sorted(EVIDENCE_CLASSES)}")

        verification = data.get("verification")
        if verification is not None and not isinstance(verification, dict):
            problems.append(f"{path.name}: verification must be an object")
        elif isinstance(verification, dict) and not verification:
            problems.append(
                f"{path.name}: verification is empty; name the run, workflow or "
                f"command that proved the claim")

        if not isinstance(data.get("limitations"), list):
            problems.append(f"{path.name}: limitations must be a list")
        elif data.get("evidence_class") == "emulator" and not data["limitations"]:
            problems.append(
                f"{path.name}: emulator evidence without a stated limitation is "
                f"the exact confusion this field exists to prevent")

        requirement = data.get("requirement")
        if (known_requirements and isinstance(requirement, str)
                and requirement not in known_requirements):
            problems.append(
                f"{path.name}: requirement {requirement!r} is not declared in "
                f"PRODUCT_ROADMAP.md")

    if not records:
        print("Claim evidence: PASS (no claim records yet; the directory is "
              "correctly empty)")
        return 0

    if problems:
        for problem in problems:
            print(f"FAIL: {problem}", file=sys.stderr)
        print(f"Claim evidence: FAIL ({len(problems)} problem(s) in "
              f"{len(records)} record(s))", file=sys.stderr)
        return 1

    print(f"Claim evidence: PASS ({len(records)} record(s); schema and honesty "
          f"fields valid, claim truth not evaluated)")
    return 0




if __name__ == "__main__":
    sys.exit(main())
