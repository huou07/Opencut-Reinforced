"""model-orchestrator CLI — thin interface over the deterministic engine.

status doctor models plan step run resume pause escalate explain.
Default behavior is safe/manual. `--auto` only enables the opt-in
single-cycle dispatch; the engine stops at PROMOTION_READY or another
explicit safe state and never advances checkpoints or promotes to main.
Unimplemented commands exit non-zero; success is never claimed for a no-op.
"""
from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from model_orchestrator import guards, model_discovery, orchestrator, policies  # noqa: E402
from model_orchestrator import runtime_memory as mem  # noqa: E402


def _repo() -> str:
    return str(Path(__file__).resolve().parent.parent.parent)


def _engine(repo: str) -> orchestrator.Orchestrator:
    loaded = policies.load_policies(repo)
    discovered = [
        {"model_id": m.model_id, "state": m.state}
        for m in model_discovery.discover_opencode_models()
    ]
    return orchestrator.Orchestrator(repo, loaded, loaded, discovered)


def cmd_status(args: argparse.Namespace) -> int:
    _ = args
    repo = _repo()
    with mem.LockedState(repo) as locked:
        current = locked.current()
        task_id = current.get("active_task")
        run = locked.read_run(task_id) if task_id else {}
    print(json.dumps({"repo": repo, "current": current, "run": run}, indent=2, sort_keys=True))
    return 0


def cmd_doctor(args: argparse.Namespace) -> int:
    _ = args
    repo = _repo()
    problems: list[str] = []
    try:
        loaded = policies.load_policies(repo)
    except (OSError, ValueError) as error:
        print("DOCTOR: problems found")
        print(f" - tracked policies unreadable: {error}")
        return 1
    problems.extend(policies.validate_policies(loaded))
    try:
        root = mem.state_dir(repo)
        if not root.is_dir():
            problems.append(f"runtime memory missing: {root}")
    except Exception as error:  # noqa: BLE001
        problems.append(f"runtime memory unavailable: {error}")
    # Protection parity: supervisor predicate must load and agree on samples.
    try:
        for sample, expected in (
            ("docs/execution/STATE.json", True),
            (".github/workflows/platform-verification.yml", True),
            ("scripts/agent_supervisor.py", True),
            ("crates/or_media/src/decoder.rs", False),
        ):
            if guards.is_supervisor_protected(repo, sample) != expected:
                problems.append(f"protection parity failed for {sample}")
    except Exception as error:  # noqa: BLE001
        problems.append(f"supervisor predicate unavailable: {error}")
    # Model configuration: discovery must execute (results may be UNAVAILABLE).
    try:
        discovered = model_discovery.discover_opencode_models()
        if not discovered:
            problems.append("model discovery returned nothing")
    except Exception as error:  # noqa: BLE001
        problems.append(f"model discovery broken: {error}")
    # Dispatcher safety capability.
    problems.extend(check_dispatcher(repo))
    # Required executables.
    for binary in ("git", "python3"):
        try:
            subprocess.run([binary, "--version"], capture_output=True, timeout=30, check=True)
        except (OSError, subprocess.SubprocessError):
            problems.append(f"required executable missing: {binary}")
    if problems:
        print("DOCTOR: problems found")
        for problem in problems:
            print(f" - {problem}")
        return 1
    print("DOCTOR: ok")
    return 0


def check_dispatcher(repo: str) -> list[str]:
    """Machine-enforced read-only? Fail closed if not verifiable."""
    from model_orchestrator import dispatcher_safety

    return dispatcher_safety.check_dispatcher(repo)


def cmd_models(args: argparse.Namespace) -> int:
    _ = args
    repo = _repo()
    loaded = policies.load_policies(repo)
    model_policy = loaded.get("MODEL_POLICY.json", {})
    discovered = [
        {"model_id": m.model_id, "state": m.state}
        for m in model_discovery.discover_opencode_models()
    ]
    rows = []
    for role, spec in model_policy.get("roles", {}).items():
        if role == "architecture":
            rows.append({"role": role, "model": "codex-exec (configurable)",
                         "state": "OPTIONAL", "reasoning": spec.get("reasoning")})
            continue
        model_id, readiness = model_discovery.select_for_role(role, model_policy, discovered)
        rows.append({"role": role, "model": model_id,
                     "state": "READY" if readiness == "READY" else "UNAVAILABLE",
                     "reasoning_requested": spec.get("reasoning"),
                     "reasoning_effective": "DEFAULT_PROVIDER"})
    print(json.dumps({"models": rows}, indent=2))
    return 0


def cmd_plan(args: argparse.Namespace) -> int:
    _ = args
    repo = _repo()
    proc = subprocess.run(
        ["python3", "scripts/execution_plan.py", "status"],
        cwd=repo, capture_output=True, text=True, timeout=60,
    )
    print(proc.stdout, end="")
    return proc.returncode


def cmd_step(args: argparse.Namespace) -> int:
    _ = args
    print(json.dumps(_engine(_repo()).next_action(), indent=2, sort_keys=True))
    return 0


def cmd_run(args: argparse.Namespace) -> int:
    repo = _repo()
    if not args.auto:
        print("manual: refusing to launch workers without --auto opt-in")
        return 2
    with mem.LockedState(repo) as locked:
        task_id = locked.current().get("active_task")
    if not task_id:
        print("no active task; create one before run --auto")
        return 2
    result = _engine(repo).run_cycle(task_id)
    print(json.dumps({"state": result.state, "candidate": result.candidate_sha,
                      "detail": result.detail, "failures": result.failures}, indent=2))
    return 0


def cmd_resume(args: argparse.Namespace) -> int:
    _ = args
    repo = _repo()
    with mem.LockedState(repo) as locked:
        task_id = locked.current().get("active_task", "")
    if not task_id:
        print("no active task")
        return 2
    result = _engine(repo).resume_cycle(task_id)
    print(json.dumps({"state": result.state, "candidate": result.candidate_sha,
                      "detail": result.detail, "failures": result.failures}, indent=2))
    return 0 if result.state in ("PROMOTION_READY", "REVIEW_PENDING") else 2


def cmd_pause(args: argparse.Namespace) -> int:
    _ = args
    repo = _repo()
    with mem.LockedState(repo) as locked:
        current = locked.current()
        current["paused"] = True
        locked.write_current(current)
    print("paused: no new worker starts; a legally claimed RUNNING worker is unaffected")
    return 0


def cmd_unpause(args: argparse.Namespace) -> int:
    _ = args
    repo = _repo()
    with mem.LockedState(repo) as locked:
        current = locked.current()
        current["paused"] = False
        locked.write_current(current)
    print("unpaused: dispatch may be claimed again")
    return 0


def cmd_escalate(args: argparse.Namespace) -> int:
    _ = args
    repo = _repo()
    with mem.LockedState(repo) as locked:
        current = locked.current()
    print(json.dumps({"escalation": current.get("escalation", "none"),
                      "note": "escalation executes only when explicitly requested; "
                              "quota exhaustion defers, never fails"}, indent=2))
    return 0


def cmd_explain(args: argparse.Namespace) -> int:
    _ = args
    print("Sources of truth: PLAN, STATE, invariants, phase contracts, exact SHA, tests, CI, evidence policy, supervisor.")
    print("Models are workers/advisers. No model marks DONE or bypasses the supervisor.")
    return 0


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(prog="model-orchestrator")
    parser.add_argument("--auto", action="store_true", help="opt-in single-cycle dispatch (manual by default)")
    sub = parser.add_subparsers(dest="command", required=True)
    for name in ("status", "doctor", "models", "plan", "step", "run", "resume", "pause",
                 "unpause", "escalate", "explain"):
        sub.add_parser(name)
    return parser


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    handlers = {
        "status": cmd_status, "doctor": cmd_doctor, "models": cmd_models,
        "plan": cmd_plan, "step": cmd_step, "run": cmd_run,
        "resume": cmd_resume, "pause": cmd_pause, "unpause": cmd_unpause,
        "escalate": cmd_escalate, "explain": cmd_explain,
    }
    return handlers[args.command](args)


if __name__ == "__main__":
    raise SystemExit(main())
