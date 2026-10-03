"""Task packets and escalation packets — machine-readable, schema-checked."""
from __future__ import annotations

import copy


def _require(value: dict, fields: list[str], label: str) -> list[str]:
    return [f"{label}.{name}: missing" for name in fields if name not in value]


def build_task_packet(
    *,
    task_id: str,
    checkpoint: str,
    base_sha: str,
    goal: str,
    branch: str,
    worktree: str,
    role: str,
    model: str,
    allowed_paths: list[str],
    forbidden_paths: list[str],
    invariants: list[str],
    acceptance: list[str],
    required_tests: list[str],
    stop_conditions: list[str],
    max_repair_attempts: int = 2,
    attempt: int = 1,
) -> dict:
    return {
        "task_id": task_id,
        "checkpoint": checkpoint,
        "base_sha": base_sha,
        "goal": goal,
        "branch": branch,
        "worktree": worktree,
        "role": role,
        "model": model,
        "allowed_paths": list(allowed_paths),
        "forbidden_paths": list(forbidden_paths),
        "invariants": list(invariants),
        "acceptance": list(acceptance),
        "required_tests": list(required_tests),
        "stop_conditions": list(stop_conditions),
        "max_repair_attempts": max_repair_attempts,
        "attempt": attempt,
    }


def validate_task_packet(packet: dict, schemas: dict) -> list[str]:
    required = schemas.get("TASK_SCHEMAS.json", {}).get("task_packet_required", [])
    return _require(packet, required, "task_packet")


def build_escalation_packet(
    *,
    checkpoint: str,
    task_id: str,
    base_sha: str,
    candidate_sha: str | None,
    invariants: list[str],
    failure_signature: dict,
    attempt_history: list[dict],
    relevant_files: list[str],
    diff_refs: list[str],
    tests_run: list[str],
    hosted_ids: dict,
    log_excerpts: list[str],
    hypotheses: list[str],
    question: str,
) -> dict:
    return {
        "checkpoint": checkpoint,
        "task_id": task_id,
        "base_sha": base_sha,
        "candidate_sha": candidate_sha,
        "invariants": list(invariants),
        "failure_signature": dict(failure_signature),
        "attempt_history": copy.deepcopy(attempt_history),
        "relevant_files": list(relevant_files),
        "diff_refs": list(diff_refs),
        "tests_run": list(tests_run),
        "hosted_ids": dict(hosted_ids),
        "log_excerpts": list(log_excerpts),
        "hypotheses": list(hypotheses),
        "question": question,
    }


def validate_escalation_packet(packet: dict, schemas: dict) -> list[str]:
    required = schemas.get("TASK_SCHEMAS.json", {}).get("escalation_packet_required", [])
    return _require(packet, required, "escalation_packet")


def validate_run_record(record: dict, schemas: dict) -> list[str]:
    required = schemas.get("TASK_SCHEMAS.json", {}).get("run_record_required", [])
    return _require(record, required, "run_record")
