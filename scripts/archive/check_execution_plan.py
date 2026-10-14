#!/usr/bin/env python3
"""Repository check for the OR execution graph and state."""

from __future__ import annotations

import sys
from pathlib import Path

SCRIPT_DIR = Path(__file__).resolve().parent
sys.path.insert(0, str(SCRIPT_DIR))

import execution_plan  # noqa: E402


def main() -> int:
    try:
        plan, state = execution_plan.load_plan_state(execution_plan.REPO_ROOT)
        summary = execution_plan.validate_plan(plan, state, execution_plan.REPO_ROOT)
    except (execution_plan.PlanError, OSError) as exc:
        print(f"Execution plan check failed: {exc}", file=sys.stderr)
        return 1
    print(
        "Execution plan check passed "
        f"({summary['checkpoint_count']} checkpoints; "
        f"current NEXT {summary['next_checkpoint'] or 'none'})."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
