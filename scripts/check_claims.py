#!/usr/bin/env python3
"""Validate claim records under docs/evidence/claims/.

Deterministic, read-only, stdlib only. Run from the repository root:

    python3 scripts/check_claims.py

Exits 0 when every record is well formed, 1 otherwise. This proves that a claim
was recorded honestly and machine-readably. It does not prove the claim is
true, important, or accepted for release.

A record is only as good as its source identity, so the validator refuses a
receipt whose `implementation_sha` is not a commit that exists in this
repository and is an ancestor of HEAD, and it requires the hosted verification
identity (workflow, job list and run id) that names where the check ran.
"""
from __future__ import annotations

import argparse
import json
import re
import subprocess
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


def git(repo: Path, *args: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        ["git", "-C", str(repo), *args],
        capture_output=True,
        text=True,
        check=False,
    )


def commit_exists(repo: Path, sha: str) -> bool:
    """True when the SHA names a commit object present in this repository."""
    return git(repo, "cat-file", "-e", f"{sha}^{{commit}}").returncode == 0


def is_ancestor(repo: Path, sha: str, ref: str = "HEAD") -> bool:
    """True when the commit is reachable from `ref`."""
    return git(repo, "merge-base", "--is-ancestor", sha, ref).returncode == 0


def check_globally_resolvable_shas(repo: Path) -> str | None:
    """Return a reason when Git cannot answer identity questions at all.

    A shallow or absent checkout cannot prove that a receipt's SHA exists or is
    an ancestor of HEAD. That is an unavailable check, never a pass, so the
    validator fails loudly instead of silently skipping the guard.
    """
    if git(repo, "rev-parse", "--git-dir").returncode != 0:
        return f"{repo} is not a Git work tree"
    if git(repo, "rev-parse", "--verify", "HEAD").returncode != 0:
        return "HEAD does not resolve to a commit"
    if git(repo, "rev-parse", "--is-shallow-repository").stdout.strip() == "true":
        return ("this is a shallow clone, so historical implementation SHAs cannot "
                "be resolved; fetch full history")
    return None


def validate_record(path: Path, data: object, known_requirements: set[str],
                    repo: Path) -> list[str]:
    """Return every problem with one record. Empty list means well formed."""
    name = path.name
    problems: list[str] = []

    if not isinstance(data, dict):
        return [f"{name}: top level must be an object"]

    name_match = FILENAME.match(name)
    if not name_match:
        problems.append(f"{name}: filename must be YYYY-MM-DD-<id>.json")

    # The filename date is the record's verification date. A mismatch means
    # someone renamed or back-dated a receipt.
    if name_match:
        stamp = data.get("verified_at_utc")
        if isinstance(stamp, str) and stamp[:10] != name_match.group(1):
            problems.append(
                f"{name}: filename date must match verified_at_utc "
                f"({stamp[:10]}), not {name_match.group(1)}")

    for field in REQUIRED:
        if field not in data:
            problems.append(f"{name}: missing required field '{field}'")

    sha = data.get("implementation_sha", "")
    if sha and not SHA.match(str(sha)):
        problems.append(
            f"{name}: implementation_sha must be a full 40-char SHA, got {sha!r}")
    elif sha:
        if not commit_exists(repo, str(sha)):
            problems.append(
                f"{name}: implementation_sha {sha} does not exist in this "
                f"repository; a fabricated receipt is not evidence")
        elif not is_ancestor(repo, str(sha)):
            problems.append(
                f"{name}: implementation_sha {sha} exists but is not an ancestor "
                f"of HEAD, so it does not describe this tree")

    stamp = data.get("verified_at_utc", "")
    if stamp and not TIMESTAMP.match(str(stamp)):
        problems.append(
            f"{name}: verified_at_utc must be UTC ISO-8601 "
            f"(YYYY-MM-DDTHH:MM:SSZ), got {stamp!r}")

    evidence_class = data.get("evidence_class")
    if evidence_class and evidence_class not in EVIDENCE_CLASSES:
        problems.append(
            f"{name}: evidence_class {evidence_class!r} is not one of "
            f"{sorted(EVIDENCE_CLASSES)}")

    verification = data.get("verification")
    if verification is not None and not isinstance(verification, dict):
        problems.append(f"{name}: verification must be an object")
    elif isinstance(verification, dict):
        if not verification:
            problems.append(
                f"{name}: verification is empty; name the run, workflow or "
                f"command that proved the claim")
        else:
            workflow = verification.get("workflow")
            if not isinstance(workflow, str) or not workflow.strip():
                problems.append(
                    f"{name}: verification.workflow must name the workflow that "
                    f"proved the claim")
            run_id = verification.get("run_id")
            if isinstance(run_id, bool) or not (
                (isinstance(run_id, str) and run_id.strip())
                or isinstance(run_id, int)
            ):
                problems.append(
                    f"{name}: verification.run_id must identify the hosted run")
            jobs = verification.get("jobs")
            if (not isinstance(jobs, list) or not jobs
                    or not all(isinstance(job, str) and job.strip() for job in jobs)):
                problems.append(
                    f"{name}: verification.jobs must be a non-empty list of job "
                    f"names")

    if not isinstance(data.get("limitations"), list):
        problems.append(f"{name}: limitations must be a list")
    elif data.get("evidence_class") == "emulator" and not data["limitations"]:
        problems.append(
            f"{name}: emulator evidence without a stated limitation is "
            f"the exact confusion this field exists to prevent")

    requirement = data.get("requirement")
    if (known_requirements and isinstance(requirement, str)
            and requirement not in known_requirements):
        problems.append(
            f"{name}: requirement {requirement!r} is not declared in "
            f"PRODUCT_ROADMAP.md")

    return problems


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--claims-dir", type=Path, default=CLAIMS_DIR,
                        help="directory of claim records (default: %(default)s)")
    parser.add_argument("--repo", type=Path, default=ROOT,
                        help="Git work tree used to resolve SHAs (default: %(default)s)")
    args = parser.parse_args(argv)

    claims_dir: Path = args.claims_dir
    repo: Path = args.repo

    if not claims_dir.is_dir():
        print(f"Claim evidence: FAIL ({claims_dir} is missing)")
        return 1

    unavailable = check_globally_resolvable_shas(repo)
    if unavailable is not None:
        print(f"Claim evidence: FAIL (cannot verify implementation SHAs: {unavailable})",
              file=sys.stderr)
        return 1

    records = sorted(p for p in claims_dir.glob("*.json"))
    known_requirements = roadmap_requirements()
    problems: list[str] = []

    for path in records:
        try:
            data = json.loads(path.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError) as error:
            problems.append(f"{path.name}: not valid JSON ({type(error).__name__})")
            continue
        problems.extend(validate_record(path, data, known_requirements, repo))

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

    print(f"Claim evidence: PASS ({len(records)} record(s); schema, source "
          f"identity and honesty fields valid, claim truth not evaluated)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
