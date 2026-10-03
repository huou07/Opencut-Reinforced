"""Deterministic orchestration engine (Layer A core).

One bounded cycle per invocation: load active task -> validate -> select
model -> verify preconditions -> launch ONE worker -> classify -> guards ->
ONE independent review -> persist. Stops at PROMOTION_READY or another
explicit safe state. Never advances checkpoints, never auto-promotes to
main, never loops the roadmap. CLI is a thin wrapper over this engine.
"""
from __future__ import annotations

from dataclasses import dataclass, field

from . import failures, guards, model_discovery, packets, promotion, review, router, supervisor_link, worker
from . import runtime_memory as mem

TERMINAL_SAFE = ("PROMOTION_READY", "REJECTED", "FAILED", "ESCALATED", "DEFERRED",
                 "REVIEW_PENDING", "SUPERVISOR_READY", "PROMOTED")


@dataclass
class Adapters:
    worker_factory: object = None  # (role, model) -> WorkerAdapter
    review_adapter: object = None  # ReviewAdapter
    codex: object = None
    jev_available: bool = False


@dataclass
class CycleResult:
    state: str
    detail: str = ""
    candidate_sha: str | None = None
    failures: list[str] = field(default_factory=list)


class Orchestrator:
    def __init__(self, repo: str, policies: dict, schemas: dict,
                 discovered: list, adapters: Adapters | None = None,
                 supervisor=None) -> None:
        self.repo = repo
        self.policies = policies
        self.schemas = schemas
        self.discovered = discovered
        self.adapters = adapters or Adapters()
        # Supervisor module for protection truth. Default None => loaded from
        # the task worktree under review (production worktrees are repo
        # checkouts). Tests inject the real supervisor explicitly.
        self.supervisor = supervisor

    # -- task setup ----------------------------------------------------
    def create_task(self, packet: dict) -> list[str]:
        errors = packets.validate_task_packet(packet, self.schemas)
        if errors:
            return errors
        with mem.LockedState(self.repo) as locked:
            locked.write_task(packet["task_id"], packet)
            locked.write_run(packet["task_id"], {
                "task_id": packet["task_id"], "checkpoint": packet["checkpoint"],
                "base_sha": packet["base_sha"], "branch": packet["branch"],
                "worktree": packet["worktree"], "role": packet["role"],
                "model": packet["model"], "family": "", "attempt": packet["attempt"],
                "status": "PENDING", "started_at": "", "completed_at": "",
                "changed_paths": [], "local_tests": [], "candidate_sha": None,
                "failure_signature": {}, "hosted_run": {}, "review": {},
                "escalation": {}, "next_action": "run --auto",
            })
            current = locked.current()
            current["active_task"] = packet["task_id"]
            locked.write_current(current)
        return []

    # -- preconditions (§5) --------------------------------------------
    def check_preconditions(self, packet: dict) -> list[str]:
        problems = []
        if packets.validate_task_packet(packet, self.schemas):
            problems.append("task packet invalid")
            return problems
        import os

        worktree = packet["worktree"]
        if not os.path.isdir(worktree):
            return ["worktree missing"]
        try:
            branch = worker.git_branch(worktree)
            head = worker.git_head(worktree)
            dirty = worker.git_dirty_paths(worktree)
        except Exception as error:  # noqa: BLE001
            return [f"worktree unreadable: {error}"]
        if branch != packet["branch"]:
            problems.append(f"branch {branch} != packet {packet['branch']}")
        if head != packet["base_sha"]:
            problems.append("HEAD != packet base SHA")
        if dirty:
            problems.append(f"worktree not clean: {dirty}")
        model_policy = self.policies.get("MODEL_POLICY.json", {})
        model_id, readiness = model_discovery.select_for_role(
            packet["role"], model_policy,
            [{"model_id": m.model_id, "state": m.state} if hasattr(m, "model_id") else m
             for m in self.discovered])
        if readiness != "READY":
            problems.append(f"no available model for role {packet['role']}")
        return problems

    # -- one bounded cycle (§6) -----------------------------------------
    def run_cycle(self, task_id: str, timeout_s: int = 600) -> CycleResult:
        with mem.LockedState(self.repo) as locked:
            packet = locked.read_task(task_id)
            run = locked.read_run(task_id)
        if not packet:
            return CycleResult(state="FAILED", detail="unknown task")
        problems = self.check_preconditions(packet)
        if problems:
            self._persist(task_id, status="FAILED", next_action="fix preconditions",
                           detail="; ".join(problems))
            return CycleResult(state="FAILED", detail="; ".join(problems))
        model_policy = self.policies.get("MODEL_POLICY.json", {})
        model_id, _ = model_discovery.select_for_role(
            packet["role"], model_policy,
            [{"model_id": m.model_id, "state": m.state} if hasattr(m, "model_id") else m
             for m in self.discovered])
        family = model_discovery.model_family(model_id or "", model_policy)
        requested_effort = model_policy.get("roles", {}).get(packet["role"], {}).get("reasoning", "DEFAULT")
        _, effective_effort = worker.OpenCodeWorkerAdapter.reasoning_plan(
            model_discovery.policy_entry_for(model_id or "", model_policy), requested_effort)
        self._persist(task_id, status="RUNNING", model=model_id, family=family,
                       started=worker.current_time(), attempt=packet["attempt"],
                       reasoning_requested=requested_effort, reasoning_effective=effective_effort)
        before_head = worker.git_head(packet["worktree"])
        adapter = self._worker_adapter(packet["role"], model_id)
        result = adapter.launch(packet, timeout_s)
        # Defense in depth: re-verify every SUCCESS claim by inspecting git.
        if result.status == "SUCCESS":
            reverified = worker.classify_worker_output(packet, before_head, packet["worktree"])
            if reverified.status != "SUCCESS":
                result = reverified
            else:
                result.candidate_sha = reverified.candidate_sha
                result.changed_paths = reverified.changed_paths
        if result.status in ("INTERRUPTED", "TIMEOUT"):
            self._persist(task_id, status="INTERRUPTED", next_action="resume",
                           changed=result.changed_paths, detail=result.detail)
            return CycleResult(state="INTERRUPTED", detail=result.detail)
        if result.status == "WORKER_CONTRACT_VIOLATION":
            self._persist(task_id, status="WORKER_CONTRACT_VIOLATION", next_action="resume",
                           changed=result.changed_paths, detail=result.detail)
            return CycleResult(state="WORKER_CONTRACT_VIOLATION", detail=result.detail,
                               failures=[result.detail])
        if result.status in ("NO_CHANGE", "TOOL_ERROR", "PROCESS_DIED"):
            self._persist(task_id, status="FAILED", next_action="diagnose worker",
                           detail=result.detail)
            return CycleResult(state="FAILED", detail=result.detail)
        # SUCCESS with a committed candidate: deterministic guards.
        candidate = result.candidate_sha or ""
        gate = guards.guard_candidate(
            packet["worktree"], packet["base_sha"], candidate,
            allowed=packet.get("allowed_paths", []),
            forbidden=packet.get("forbidden_paths", []),
            plan_allowlist=tuple(packet.get("plan_allowlist", [])),
            supervisor=self.supervisor)
        self._persist(task_id, status="CANDIDATE", candidate=candidate,
                       changed=gate["changed_paths"])
        if not gate["accepted"]:
            self._persist(task_id, status="REJECTED", candidate=candidate,
                           changed=gate["changed_paths"],
                           next_action="fix scope/protection",
                           detail="; ".join(gate["protected_violations"] + gate["scope_violations"]))
            return CycleResult(state="REJECTED", candidate_sha=candidate,
                               failures=gate["protected_violations"] + gate["scope_violations"])
        # Independent review (different family required).
        review_outcome = self._review(task_id, packet, candidate, gate)
        return review_outcome

    def _worker_adapter(self, role: str, model_id: str | None):
        factory = self.adapters.worker_factory
        if factory is not None:
            return factory(role, model_id)
        return worker.OpenCodeWorkerAdapter()

    def _review(self, task_id: str, packet: dict, candidate: str, gate: dict) -> CycleResult:
        model_policy = self.policies.get("MODEL_POLICY.json", {})
        implementer_family = model_discovery.model_family(packet.get("model") or "", model_policy)
        if not implementer_family or implementer_family == "unknown":
            # Family recorded at RUNNING time is authoritative; fall back to run record.
            with mem.LockedState(self.repo) as locked:
                implementer_family = locked.read_run(task_id).get("family") or implementer_family
        discovered = [{"model_id": m.model_id, "state": m.state} if hasattr(m, "model_id") else m
                      for m in self.discovered]
        candidates = review.reviewer_candidates(model_policy, discovered, exclude_family=implementer_family)
        if not candidates:
            self._persist(task_id, status="REVIEW_PENDING", candidate=candidate,
                           changed=gate["changed_paths"], next_action="await independent reviewer")
            return CycleResult(state="REVIEW_PENDING", candidate_sha=candidate,
                               detail="no different-family reviewer available")
        adapter = self.adapters.review_adapter or review.FakeReviewAdapter(verdict="INCONCLUSIVE")
        adapter_model = candidates[0]
        if hasattr(adapter, "model"):
            adapter.model = adapter_model
        self._persist(task_id, status="REVIEWING", candidate=candidate)
        outcome = review.execute_review(
            worktree=packet["worktree"], task_id=task_id, base_sha=packet["base_sha"],
            candidate_sha=candidate, changed_paths=gate["changed_paths"],
            guard_summary={"protected": gate["protected_violations"],
                           "scope": gate["scope_violations"], "gaming": gate["gaming_flags"]},
            adapter=adapter, schemas=self.schemas)
        reviewer_family = model_discovery.model_family(adapter_model, model_policy)
        self._persist(task_id, review={"model": adapter_model, "family": reviewer_family,
                                       "verdict": outcome.get("verdict"),
                                       "findings": outcome.get("report", {}).get("findings", [])})
        if not outcome["ok"]:
            self._persist(task_id, status="REVIEW_REJECTED", candidate=candidate,
                           next_action="review invalid; diagnose",
                           detail="; ".join(outcome["errors"]))
            return CycleResult(state="REVIEW_REJECTED", candidate_sha=candidate,
                               failures=outcome["errors"])
        if outcome["verdict"] != "PASS":
            self._persist(task_id, status="REVIEW_REJECTED", candidate=candidate,
                           next_action="address reviewer findings")
            return CycleResult(state="REVIEW_REJECTED", candidate_sha=candidate,
                               failures=outcome.get("report", {}).get("findings", []))
        if gate.get("promotion_blocked_by_gaming"):
            # Deterministic guard wins over reviewer prose: never auto-promote.
            self._persist(task_id, status="REVIEW_PENDING", candidate=candidate,
                           next_action="resolve anti-gaming flags",
                           detail="; ".join(gate["gaming_flags"]))
            return CycleResult(state="REVIEW_PENDING", candidate_sha=candidate,
                               failures=gate["gaming_flags"])
        self._persist(task_id, status="PROMOTION_READY", candidate=candidate,
                       next_action="promotion gate (explicit, never automatic)")
        return CycleResult(state="PROMOTION_READY", candidate_sha=candidate)

    # -- resume (§8) ------------------------------------------------------
    def resume(self, task_id: str) -> dict:
        with mem.LockedState(self.repo) as locked:
            packet = locked.read_task(task_id)
            run = locked.read_run(task_id)
        if not packet or run.get("status") not in ("INTERRUPTED", "WORKER_CONTRACT_VIOLATION", "FAILED"):
            return {"ok": False, "error": "nothing resumable"}
        try:
            dirty = worker.git_dirty_paths(packet["worktree"])
            head = worker.git_head(packet["worktree"])
        except Exception as error:  # noqa: BLE001
            return {"ok": False, "error": f"worktree unreadable: {error}"}
        prompt = worker.resume_prompt(packet, dirty or ["(clean tree, no dirty paths)"], head)
        return {"ok": True, "task_id": task_id, "packet": packet, "dirty": dirty,
                "head": head, "resume_prompt": prompt,
                "note": "same task, same branch, no reset, no clean"}

    # -- persistence -------------------------------------------------------
    def _persist(self, task_id: str, **fields) -> None:
        with mem.LockedState(self.repo) as locked:
            run = locked.read_run(task_id)
            mapping = {"status": "status", "candidate": "candidate_sha", "changed": "changed_paths",
                       "model": "model", "family": "family", "started": "started_at",
                       "attempt": "attempt", "next_action": "next_action", "detail": "detail",
                       "review": "review", "reasoning_requested": "reasoning_requested",
                       "reasoning_effective": "reasoning_effective"}
            for key, value in fields.items():
                if key in mapping:
                    run[mapping[key]] = value
            if fields.get("status") in ("PROMOTION_READY", "REJECTED", "FAILED", "ESCALATED",
                                         "DEFERRED", "REVIEW_PENDING", "SUPERVISOR_READY", "PROMOTED"):
                run["completed_at"] = worker.current_time()
            locked.write_run(task_id, run)
            current = locked.current()
            current["active_task"] = task_id
            if "candidate" in fields and fields["candidate"]:
                current["candidate_sha"] = fields["candidate"]
            locked.write_current(current)

    # -- routing helper ------------------------------------------------------
    def next_action(self) -> dict:
        with mem.LockedState(self.repo) as locked:
            current = locked.current()
        task_id = current.get("active_task")
        run = {}
        if task_id:
            with mem.LockedState(self.repo) as locked:
                run = locked.read_run(task_id)
        facts = {
            "candidate_ready": bool(run.get("candidate_sha")),
            "attempts_remaining": True,
            "needs_architecture": run.get("status") == "ESCALATED",
            "hosted_inconclusive": current.get("hosted_verdict") == "HOSTED_VERIFY_INCONCLUSIVE",
            "evidence_ready": False,
            "harness_suspect": False,
        }
        jev = router.JevAdapter(available=self.adapters.jev_available)
        return {"task_id": task_id, "run_status": run.get("status"),
                "route": router.route(facts, jev)}
