"""model-orchestrator CLI: status doctor models plan step run resume pause escalate explain.

Default behavior is safe/manual. `--auto` only enables the opt-in autonomous
capability; this task builds and tests it without launching full-roadmap runs.
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from model_orchestrator import model_discovery, policies, router  # noqa: E402
from model_orchestrator import runtime_memory as mem  # noqa: E402


def _repo() -> str:
    from pathlib import Path as P

    return str(P(__file__).resolve().parent.parent.parent)


def cmd_status(args: argparse.Namespace) -> int:
    _ = args
    repo = _repo()
    with mem.LockedState(repo) as locked:
        current = locked.current()
    print(json.dumps({"repo": repo, "current": current}, indent=2, sort_keys=True))
    return 0


def cmd_doctor(args: argparse.Namespace) -> int:
    _ = args
    repo = _repo()
    problems = []
    loaded = policies.load_policies(repo)
    problems.extend(policies.validate_policies(loaded))
    try:
        root = mem.state_dir(repo)
    except Exception as error:  # noqa: BLE001
        problems.append(f"runtime memory unavailable: {error}")
    else:
        if not root.is_dir():
            problems.append(f"runtime memory missing: {root}")
    if problems:
        print("DOCTOR: problems found")
        for problem in problems:
            print(f" - {problem}")
        return 1
    print("DOCTOR: ok")
    return 0


def cmd_models(args: argparse.Namespace) -> int:
    _ = args
    repo = _repo()
    loaded = policies.load_policies(repo)
    model_policy = loaded.get("MODEL_POLICY.json", {})
    discovered = model_discovery.discover_opencode_models()
    rows = []
    for role, spec in model_policy.get("roles", {}).items():
        if role == "architecture":
            rows.append({"role": role, "model": "codex-exec (configurable)", "state": "OPTIONAL"})
            continue
        model_id, readiness = model_discovery.select_for_role(role, model_policy, discovered)
        rows.append({"role": role, "model": model_id, "state": "READY" if readiness == "READY" else "UNAVAILABLE"})
    print(json.dumps({"models": rows}, indent=2))
    return 0


def cmd_plan(args: argparse.Namespace) -> int:
    repo = _repo()
    import subprocess

    proc = subprocess.run(
        ["python3", "scripts/execution_plan.py", "status"],
        cwd=repo, capture_output=True, text=True, timeout=60,
    )
    print(proc.stdout, end="")
    return proc.returncode


def cmd_step(args: argparse.Namespace) -> int:
    _ = args
    repo = _repo()
    with mem.LockedState(repo) as locked:
        current = locked.current()
    facts = {
        "candidate_ready": bool(current.get("candidate_sha")),
        "attempts_remaining": current.get("attempts_remaining", True),
        "needs_architecture": current.get("escalation") == "ARCHITECTURE_ESCALATION",
        "hosted_inconclusive": current.get("hosted_verdict") == "HOSTED_VERIFY_INCONCLUSIVE",
        "evidence_ready": bool(current.get("evidence_ready")),
        "harness_suspect": bool(current.get("harness_suspect")),
    }
    decision = router.route(facts, router.JevAdapter(available=False))
    print(json.dumps({"facts": facts, "route": decision}, indent=2, sort_keys=True))
    return 0


def cmd_run(args: argparse.Namespace) -> int:
    print("--auto" if args.auto else "manual: refusing to launch workers without --auto opt-in")
    return 0 if args.auto else 2


def cmd_resume(args: argparse.Namespace) -> int:
    _ = args
    repo = _repo()
    with mem.LockedState(repo) as locked:
        current = locked.current()
    task_id = current.get("active_task")
    if not task_id:
        print("no active task")
        return 2
    with mem.LockedState(repo) as locked:
        run = locked.read_run(task_id)
    print(json.dumps({"task_id": task_id, "run": run}, indent=2, sort_keys=True))
    return 0


def cmd_pause(args: argparse.Namespace) -> int:
    _ = args
    repo = _repo()
    with mem.LockedState(repo) as locked:
        current = locked.current()
        current["paused"] = True
        locked.write_current(current)
    print("paused")
    return 0


def cmd_escalate(args: argparse.Namespace) -> int:
    _ = args
    repo = _repo()
    with mem.LockedState(repo) as locked:
        current = locked.current()
    print(json.dumps({"escalation": current.get("escalation", "none")}, indent=2))
    return 0


def cmd_explain(args: argparse.Namespace) -> int:
    _ = args
    print("Sources of truth: PLAN, STATE, invariants, phase contracts, exact SHA, tests, CI, evidence policy, supervisor.")
    print("Models are workers/advisers. No model marks DONE or bypasses the supervisor.")
    return 0


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(prog="model-orchestrator")
    parser.add_argument("--auto", action="store_true", help="opt-in autonomous capability (manual by default)")
    sub = parser.add_subparsers(dest="command", required=True)
    for name in ("status", "doctor", "models", "plan", "step", "run", "resume", "pause", "escalate", "explain"):
        sub.add_parser(name)
    return parser


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    handlers = {
        "status": cmd_status, "doctor": cmd_doctor, "models": cmd_models,
        "plan": cmd_plan, "step": cmd_step, "run": cmd_run,
        "resume": cmd_resume, "pause": cmd_pause, "escalate": cmd_escalate,
        "explain": cmd_explain,
    }
    return handlers[args.command](args)


if __name__ == "__main__":
    raise SystemExit(main())
