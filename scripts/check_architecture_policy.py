#!/usr/bin/env python3
"""Check the locked OR architecture policy without third-party packages."""

from __future__ import annotations

import hashlib
import re
import sys
from pathlib import Path

SCRIPT_DIR = Path(__file__).resolve().parent
REPO_ROOT = SCRIPT_DIR.parent
sys.path.insert(0, str(SCRIPT_DIR))

import execution_plan  # noqa: E402


POLICY_PATH = REPO_ROOT / "docs" / "execution" / "architecture-policy.json"
PROTOTYPE_PATH = REPO_ROOT / "prototypes" / "or-ui-demo.html"
CORE_MANIFEST = REPO_ROOT / "crates" / "or_core" / "Cargo.toml"
AGENTS_PATH = REPO_ROOT / "AGENTS.md"


def _read(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def _constant(path: Path, name: str) -> int:
    pattern = re.compile(
        rf"\b{re.escape(name)}\b\s*(?::\s*[A-Za-z0-9_<>]+)?\s*=\s*(\d+)"
    )
    match = pattern.search(_read(path))
    if match is None:
        raise ValueError(f"could not find {name} in {path}")
    return int(match.group(1))


def _direct_dependency_names(manifest: str) -> set[str]:
    """Read dependency keys from normal and target dependency tables.

    This deliberately handles Cargo's simple key/value dependency declarations
    used by `or_core`; it does not attempt to become a Cargo manifest parser.
    """

    names: set[str] = set()
    in_dependency_table = False
    for raw_line in manifest.splitlines():
        line = raw_line.strip()
        if line.startswith("[") and line.endswith("]"):
            in_dependency_table = line == "[dependencies]" or (
                line.startswith("[target.") and line.endswith(".dependencies]")
            )
            continue
        if not in_dependency_table or not line or line.startswith("#"):
            continue
        match = re.match(r"([A-Za-z0-9_-]+)\s*=", line)
        if match:
            names.add(match.group(1).lower())
    return names


def _has_stale_phase_status(agents: str) -> bool:
    status_words = r"in progress|next|done|complete|planned"
    return any(
        re.search(rf"\bphase\s+\d+[A-Za-z0-9-]*\b.*\b(?:{status_words})\b", line, re.I)
        for line in agents.splitlines()
    )


def main() -> int:
    failures: list[str] = []
    try:
        policy = execution_plan.load_json(POLICY_PATH)
        expected_hash = policy.get("frozen_prototype_sha256")
        if not isinstance(expected_hash, str):
            failures.append("policy frozen_prototype_sha256 is missing")
        elif not PROTOTYPE_PATH.is_file():
            failures.append(f"prototype is missing: {PROTOTYPE_PATH}")
        else:
            actual_hash = hashlib.sha256(PROTOTYPE_PATH.read_bytes()).hexdigest()
            if actual_hash != expected_hash:
                failures.append(
                    f"prototype SHA-256 changed: expected {expected_hash}, got {actual_hash}"
                )

        forbidden = policy.get("or_core_forbidden_direct_dependency_name_patterns", [])
        if not isinstance(forbidden, list) or any(not isinstance(item, str) for item in forbidden):
            failures.append("policy forbidden dependency patterns are invalid")
        elif CORE_MANIFEST.is_file():
            names = _direct_dependency_names(_read(CORE_MANIFEST))
            for name in sorted(names):
                if any(pattern.lower() in name for pattern in forbidden):
                    failures.append(f"forbidden direct or_core dependency: {name}")
        else:
            failures.append(f"or_core manifest is missing: {CORE_MANIFEST}")

        required_docs = policy.get("required_execution_docs", [])
        if not isinstance(required_docs, list):
            failures.append("policy required_execution_docs is invalid")
        else:
            for relative_path in required_docs:
                if not isinstance(relative_path, str) or not (REPO_ROOT / relative_path).is_file():
                    failures.append(f"required execution file is missing: {relative_path}")

        expected_versions = policy.get("current_versions", {})
        if not isinstance(expected_versions, dict):
            failures.append("policy current_versions is invalid")
        else:
            constants = {
                "project_schema": (REPO_ROOT / "crates" / "or_core" / "src" / "project_document.rs", "CURRENT_PROJECT_SCHEMA_VERSION"),
                "recovery_schema": (REPO_ROOT / "crates" / "or_core" / "src" / "project_recovery.rs", "CURRENT_RECOVERY_SCHEMA_VERSION"),
                "ipc_protocol": (REPO_ROOT / "crates" / "or_ipc" / "src" / "protocol.rs", "OR_LOCAL_IPC_PROTOCOL_VERSION"),
            }
            for key, (path, constant_name) in constants.items():
                try:
                    actual = _constant(path, constant_name)
                except ValueError as exc:
                    failures.append(str(exc))
                    continue
                if actual != expected_versions.get(key):
                    failures.append(
                        f"{constant_name} is {actual}, expected policy value {expected_versions.get(key)}"
                    )

        agents = _read(AGENTS_PATH)
        if _has_stale_phase_status(agents):
            failures.append("AGENTS.md contains a manually maintained phase status")

        plan, state = execution_plan.load_plan_state(REPO_ROOT)
        execution_plan.validate_plan(plan, state, REPO_ROOT)
    except (execution_plan.PlanError, OSError, ValueError) as exc:
        failures.append(str(exc))

    if failures:
        for failure in failures:
            print(f"Architecture policy check failed: {failure}", file=sys.stderr)
        return 1
    print(
        "Architecture policy check passed "
        "(prototype hash, versions, or_core dependency boundary, required files, "
        "AGENTS status rule, and plan/state validation)."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
