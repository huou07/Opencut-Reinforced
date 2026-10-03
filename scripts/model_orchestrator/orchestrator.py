"""Deterministic orchestration engine (Layer A core).

One bounded cycle per invocation: pause check -> claim task lease ->
validate -> select model -> freeze trusted authority -> verify
preconditions -> launch ONE worker -> classify -> guards (trusted
supervisor + authoritative PLAN allowlist) -> ONE independent review ->
persist. Stops at PROMOTION_READY or another explicit safe state. Never
advances checkpoints, never auto-promotes to main, never loops the roadmap.
CLI is a thin wrapper over this engine.

Transition map (all other transitions rejected):
  PENDING -> RUNNING | FAILED
  RUNNING -> INTERRUPTED | WORKER_CONTRACT_VIOLATION | CANDIDATE | FAILED | NO_CHANGE
  INTERRUPTED | WORKER_CONTRACT_VIOLATION -> RUNNING (resume) | FAILED
  CANDIDATE -> REVIEWING | REJECTED
  REVIEWING -> PROMOTION_READY | REVIEW_REJECTED | REVIEW_PENDING
  REVIEW_PENDING -> REVIEWING | FAILED
  PROMOTION_READY -> PROMOTING | FAILED
  PROMOTING -> PROMOTED | FAILED
  PROMOTED -> SUPERVISOR_READY
  any -> ESCALATED | DEFERRED (routing outcomes)

Pause semantics: pause is checked atomically immediately before task claim;
if set, no worker starts (PAUSED refusal). A legally claimed RUNNING worker
is unaffected by a later pause.
"""
from __future__ import annotations

import hashlib
import json
from dataclasses import dataclass, field

from . import guards, model_discovery, packets, review, router, worker
from . import runtime_memory as mem
from . import trusted_authority as trust

TERMINAL_SAFE = ("PROMOTION_READY", "REJECTED", "FAILED", "ESCALATED", "DEFERRED",
                 "REVIEW_PENDING", "SUPERVISOR_READY", "PROMOTED", "PAUSED", "TASK_BUSY")

TRANSITIONS = {
    "PENDING": ("RUNNING", "FAILED"),
    "RUNNING": ("INTERRUPTED", "WORKER_CONTRACT_VIOLATION", "CANDIDATE", "FAILED", "NO_CHANGE"),
    "NO_CHANGE": ("RUNNING", "FAILED"),
    "INTERRUPTED": ("RUNNING", "FAILED"),
    "WORKER_CONTRACT_VIOLATION": ("RUNNING", "FAILED"),
    "CANDIDATE": ("REVIEWING", "REVIEW_PENDING", "REJECTED", "FAILED"),
    "REVIEWING": ("PROMOTION_READY", "REVIEW_REJECTED", "REVIEW_PENDING", "FAILED"),
    "REVIEW_PENDING": ("REVIEWING", "FAILED", "ESCALATED"),
    "REVIEW_REJECTED": ("RUNNING", "FAILED", "ESCALATED"),
    "REJECTED": ("RUNNING", "FAILED"),
    "PROMOTION_READY": ("PROMOTING", "FAILED"),
    "PROMOTING": ("PROMOTED", "FAILED"),
    "PROMOTED": ("SUPERVISOR_READY",),
    "FAILED": ("RUNNING",),
    "ESCALATED": (),
    "DEFERRED": ("RUNNING",),
}


def legal_transition(from_status: str, to_status: str) -> bool:
    if not from_status:
        return to_status in ("PENDING", "RUNNING")
    return to_status in TRANSITIONS.get(from_status, ())


def digest_of(value: object) -> str:
    return hashlib.sha256(json.dumps(value, sort_keys=True).encode()).hexdigest()


def contract_digest(packet: dict) -> str:
    subset = {key: packet.get(key) for key in (
        "task_id", "checkpoint", "base_sha", "branch", "allowed_paths",
        "forbidden_paths", "invariants", "acceptance", "required_tests",
        "stop_conditions", "max_repair_attempts", "model_locked")}
    return digest_of(subset)


@dataclass
class Adapters:
    worker_factory: object = None  # (role, model) -> WorkerAdapter
    review_adapter: object = None  # ReviewAdapter (tests inject; production builds real one)
    codex: object = None


@dataclass
class CycleResult:
    state: str
    detail: str = ""
    candidate_sha: str | None = None
    failures: list[str] = field(default_factory=list)


class Orchestrator:
    def __init__(self, repo: str, policies: dict, schemas: dict,
                 discovered: list, adapters: Adapters | None = None,
                 trusted_repo: str | None = None, supervisor=None) -> None:
        self.repo = repo
        self.policies = policies
        self.schemas = schemas
        self.discovered = discovered
        self.adapters = adapters or Adapters()
        # Trusted control checkout. Defaults to the orchestrator's own repo.
        # NEVER the task candidate worktree.
        self.trusted_repo = trusted_repo or repo
        self._supervisor_override = supervisor

    # -- trusted authority ----------------------------------------------
    def frozen_authority(self) -> dict:
        if self._supervisor_override is not None:
            return {"trusted_repo": self.trusted_repo, "supervisor_digest": "injected",
                    "module": self._supervisor_override, "plan": None, "state": None,
                    "injected": True}
        return trust.freeze_authority(self.trusted_repo)

    def authoritative_allowlist(self, authority: dict, checkpoint: str) -> dict:
        if authority.get("injected"):
            return {"allowlist": [], "injected": True}
        return trust.resolve_checkpoint(authority, checkpoint)

    # -- task setup -------------------------------------------------------
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
                "model_requested": packet.get("model"), "model_locked": packet.get("model_locked"),
                "model_selected": "", "model_family": "", "attempt": packet["attempt"],
                "status": "PENDING", "started_at": "", "completed_at": "",
                "changed_paths": [], "local_tests": [], "candidate_sha": None,
                "failure_signature": {}, "hosted_run": {}, "review": {},
                "escalation": {}, "next_action": "run --auto",
                "reasoning_requested": "", "reasoning_effective": "",
                "supervisor_digest": "", "contract_digest": contract_digest(packet),
            })
            current = locked.current()
            current["active_task"] = packet["task_id"]
            locked.write_current(current)
        return []

    # -- model selection (runtime authoritative) ---------------------------
    def select_model(self, packet: dict) -> tuple[str | None, dict, str]:
        """Return (model_id, policy_entry, family). packet model never decides."""
        model_policy = self.policies.get("MODEL_POLICY.json", {})
        if packet.get("model_locked"):
            locked_id = packet["model_locked"]
            entry = model_discovery.policy_entry_for(locked_id, model_policy)
            if entry and self._is_available(locked_id):
                return locked_id, entry, entry.get("family", "unknown")
            return None, {}, ""
        model_id, readiness = model_discovery.select_for_role(
            packet["role"], model_policy, self.discovered)
        if readiness != "READY" or not model_id:
            return None, {}, ""
        entry = model_discovery.policy_entry_for(model_id, model_policy)
        return model_id, entry, entry.get("family", "unknown")

    def _is_available(self, model_id: str) -> bool:
        return model_discovery.model_available(
            model_id,
            [{"model_id": m.model_id, "state": m.state} if hasattr(m, "model_id") else m
             for m in self.discovered])

    # -- preconditions (§5) --------------------------------------------------
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
        model_id, _, _ = self.select_model(packet)
        if not model_id:
            problems.append(f"no available model for role {packet['role']}")
        return problems

    # -- one bounded cycle (§6) ----------------------------------------------
    def run_cycle(self, task_id: str, timeout_s: int = 600) -> CycleResult:
        with mem.LockedState(self.repo) as locked:
            if locked.current().get("paused"):
                return CycleResult(state="PAUSED", detail="dispatch paused; unpause first")
            packet = locked.read_task(task_id)
            run = locked.read_run(task_id)
        if not packet:
            return CycleResult(state="FAILED", detail="unknown task")
        if run.get("status") == "RUNNING":
            return CycleResult(state="TASK_BUSY", detail="task already claimed")
        if run.get("status") in ("INTERRUPTED", "WORKER_CONTRACT_VIOLATION"):
            return CycleResult(state=run.get("status"),
                               detail="interrupted work exists; use resume (same packet, no reset)")
        if run.get("status") not in ("PENDING", "FAILED", "NO_CHANGE"):
            return CycleResult(state=run.get("status", "FAILED"),
                               detail=f"task not claimable from {run.get('status')}")
        try:
            lease = mem.TaskLease(self.repo, task_id)
        except TimeoutError:
            return CycleResult(state="TASK_BUSY", detail="task lease held by another process")
        try:
            return self._cycle_under_lease(task_id, packet, timeout_s)
        finally:
            lease.release()

    def _cycle_under_lease(self, task_id: str, packet: dict, timeout_s: int) -> CycleResult:
        try:
            authority = self.frozen_authority()
        except (OSError, ImportError, ValueError) as error:
            self._transition(task_id, "FAILED", next_action="fix trusted authority",
                             detail=f"TRUSTED_POLICY_MISMATCH: {error}")
            return CycleResult(state="FAILED", detail="TRUSTED_POLICY_MISMATCH")
        problems = self.check_preconditions(packet)
        if problems:
            self._transition(task_id, "FAILED", next_action="fix preconditions",
                             detail="; ".join(problems))
            return CycleResult(state="FAILED", detail="; ".join(problems))
        model_policy = self.policies.get("MODEL_POLICY.json", {})
        model_id, entry, family = self.select_model(packet)
        requested_effort = model_policy.get("roles", {}).get(packet["role"], {}).get("reasoning", "DEFAULT")
        _, effective_effort = worker.OpenCodeWorkerAdapter.reasoning_plan(entry, requested_effort)
        assert model_id
        self._transition(task_id, "RUNNING", model_selected=model_id, model_family=family,
                         started=worker.current_time(), attempt=packet["attempt"],
                         reasoning_requested=requested_effort, reasoning_effective=effective_effort,
                         supervisor_digest=authority.get("supervisor_digest", ""))
        before_head = worker.git_head(packet["worktree"])
        adapter = self._worker_adapter(packet["role"], model_id)
        if isinstance(adapter, worker.OpenCodeWorkerAdapter):
            result = adapter.launch(packet, timeout_s, model=model_id,
                                    policy_entry=entry, effort=requested_effort)
        else:
            result = adapter.launch(packet, timeout_s, model=model_id,
                                    policy_entry=entry, effort=requested_effort)
        # Defense in depth: re-verify every SUCCESS claim by inspecting git.
        if result.status == "SUCCESS":
            reverified = worker.classify_worker_output(packet, before_head, packet["worktree"])
            if reverified.status != "SUCCESS":
                result = reverified
            else:
                result.candidate_sha = reverified.candidate_sha
                result.changed_paths = reverified.changed_paths
                result.report["reasoning_effective"] = effective_effort
        if result.status in ("INTERRUPTED", "TIMEOUT"):
            self._transition(task_id, "INTERRUPTED", next_action="resume",
                             changed=result.changed_paths, detail=result.detail)
            return CycleResult(state="INTERRUPTED", detail=result.detail)
        if result.status == "WORKER_CONTRACT_VIOLATION":
            self._transition(task_id, "WORKER_CONTRACT_VIOLATION", next_action="resume",
                             changed=result.changed_paths, detail=result.detail)
            return CycleResult(state="WORKER_CONTRACT_VIOLATION", detail=result.detail,
                               failures=[result.detail])
        if result.status in ("NO_CHANGE", "TOOL_ERROR", "PROCESS_DIED"):
            self._transition(task_id, "FAILED", next_action="diagnose worker", detail=result.detail)
            return CycleResult(state="FAILED", detail=result.detail)
        # SUCCESS with a committed candidate: trusted guards (frozen authority).
        if trust.verify_authority_fresh(authority):
            self._transition(task_id, "FAILED", next_action="refresh orchestrator",
                             detail="TRUSTED_POLICY_MISMATCH")
            return CycleResult(state="FAILED", detail="TRUSTED_POLICY_MISMATCH")
        candidate = result.candidate_sha or ""
        allow_info = self.authoritative_allowlist(authority, packet["checkpoint"])
        if "error" in allow_info:
            self._transition(task_id, "REJECTED", candidate=candidate, detail=allow_info["error"])
            return CycleResult(state="REJECTED", candidate_sha=candidate, failures=[allow_info["error"]])
        snapshot = packet.get("plan_allowlist_snapshot") or []
        if sorted(snapshot) != sorted(allow_info["allowlist"]):
            detail = "TASK_CONTRACT_MISMATCH: packet snapshot differs from authoritative PLAN allowlist"
            self._transition(task_id, "REJECTED", candidate=candidate, detail=detail)
            return CycleResult(state="REJECTED", candidate_sha=candidate, failures=[detail])
        gate = guards.guard_candidate(
            packet["worktree"], packet["base_sha"], candidate,
            allowed=packet.get("allowed_paths", []),
            forbidden=packet.get("forbidden_paths", []),
            plan_allowlist=tuple(allow_info["allowlist"]),
            supervisor=authority.get("module"))
        self._transition(task_id, "CANDIDATE", candidate=candidate, changed=gate["changed_paths"])
        if not gate["accepted"]:
            self._transition(task_id, "REJECTED", candidate=candidate,
                             changed=gate["changed_paths"], next_action="fix scope/protection",
                             detail="; ".join(gate["protected_violations"] + gate["scope_violations"]))
            return CycleResult(state="REJECTED", candidate_sha=candidate,
                               failures=gate["protected_violations"] + gate["scope_violations"])
        return self._review(task_id, packet, candidate, gate, allow_info)

    def _worker_adapter(self, role: str, model_id: str | None):
        factory = self.adapters.worker_factory
        if factory is not None:
            produced = factory(role, model_id)
            if produced is not None:
                return produced
        return worker.OpenCodeWorkerAdapter()

    def _review(self, task_id: str, packet: dict, candidate: str, gate: dict,
                allow_info: dict) -> CycleResult:
        model_policy = self.policies.get("MODEL_POLICY.json", {})
        with mem.LockedState(self.repo) as locked:
            implementer_family = locked.read_run(task_id).get("model_family", "")
        candidates = review.reviewer_candidates(model_policy, self.discovered, exclude_family=implementer_family)
        if not candidates:
            self._transition(task_id, "REVIEW_PENDING", candidate=candidate,
                             changed=gate["changed_paths"], next_action="await independent reviewer")
            return CycleResult(state="REVIEW_PENDING", candidate_sha=candidate,
                               detail="no different-family reviewer available")
        adapter = self.adapters.review_adapter
        if adapter is None:
            adapter = review.OpenCodeReviewAdapter(model=candidates[0])
        elif hasattr(adapter, "model"):
            adapter.model = candidates[0]
        reviewer_family = model_discovery.model_family(candidates[0], model_policy)
        if reviewer_family == implementer_family:
            self._transition(task_id, "REVIEW_PENDING", candidate=candidate,
                             next_action="await independent reviewer")
            return CycleResult(state="REVIEW_PENDING", candidate_sha=candidate,
                               detail="reviewer family matches implementer")
        self._transition(task_id, "REVIEWING", candidate=candidate)
        outcome = review.execute_review(
            worktree=packet["worktree"], task_id=task_id, base_sha=packet["base_sha"],
            candidate_sha=candidate, changed_paths=gate["changed_paths"],
            guard_summary={"protected": gate["protected_violations"],
                           "scope": gate["scope_violations"], "gaming": gate["gaming_flags"]},
            adapter=adapter, schemas=self.schemas)
        self._persist_review(task_id, candidates[0], reviewer_family, outcome)
        if not outcome["ok"]:
            self._transition(task_id, "REVIEW_REJECTED", candidate=candidate,
                             next_action="review invalid; diagnose",
                             detail="; ".join(outcome["errors"]))
            return CycleResult(state="REVIEW_REJECTED", candidate_sha=candidate,
                               failures=outcome["errors"])
        if outcome["verdict"] != "PASS":
            self._transition(task_id, "REVIEW_REJECTED", candidate=candidate,
                             next_action="address reviewer findings")
            return CycleResult(state="REVIEW_REJECTED", candidate_sha=candidate,
                               failures=outcome.get("report", {}).get("findings", []))
        if gate.get("promotion_blocked_by_gaming"):
            self._transition(task_id, "REVIEW_PENDING", candidate=candidate,
                             next_action="resolve anti-gaming flags",
                             detail="; ".join(gate["gaming_flags"]))
            return CycleResult(state="REVIEW_PENDING", candidate_sha=candidate,
                               failures=gate["gaming_flags"])
        self._write_promotion_authorization(task_id, packet, candidate, candidates[0],
                                            reviewer_family, gate, allow_info)
        self._transition(task_id, "PROMOTION_READY", candidate=candidate,
                         next_action="promotion gate (explicit, never automatic)")
        return CycleResult(state="PROMOTION_READY", candidate_sha=candidate)

    def _write_promotion_authorization(self, task_id: str, packet: dict, candidate: str,
                                         reviewer_model: str, reviewer_family: str, gate: dict,
                                         allow_info: dict) -> None:
        with mem.LockedState(self.repo) as locked:
            run = locked.read_run(task_id)
            record = {
                "schema_version": 1,
                "task_id": task_id,
                "checkpoint": packet["checkpoint"],
                "base_sha": packet["base_sha"],
                "candidate_sha": candidate,
                "reviewed_sha": candidate,
                "reviewer_model": reviewer_model,
                "reviewer_family": reviewer_family,
                "review_verdict": "PASS",
                "guard_digest": digest_of({"protected": gate["protected_violations"],
                                          "scope": gate["scope_violations"],
                                          "gaming": gate["gaming_flags"]}),
                "changed_digest": digest_of(gate["changed_paths"]),
                "contract_digest": contract_digest(packet),
                "allowed_paths": list(packet.get("allowed_paths", [])),
                "forbidden_paths": list(packet.get("forbidden_paths", [])),
                "plan_allowlist": list(allow_info["allowlist"]),
                "task_packet": {key: packet.get(key) for key in (
                    "task_id", "checkpoint", "base_sha", "branch", "allowed_paths",
                    "forbidden_paths", "invariants", "acceptance", "required_tests",
                    "stop_conditions", "max_repair_attempts", "model_locked")},
                "promotion_ready_at": worker.current_time(),
            }
            locked.write_promotion(task_id, record)
            run["promotion"] = {"authorized": True, "at": record["promotion_ready_at"]}
            locked.write_run(task_id, run)

    # -- resume (§17: real relaunch) -----------------------------------------
    def resume_cycle(self, task_id: str, timeout_s: int = 600) -> CycleResult:
        with mem.LockedState(self.repo) as locked:
            packet = locked.read_task(task_id)
            run = locked.read_run(task_id)
            paused = locked.current().get("paused")
        if not packet:
            return CycleResult(state="FAILED", detail="unknown task")
        if paused:
            return CycleResult(state="PAUSED", detail="dispatch paused; unpause first")
        if run.get("status") not in ("INTERRUPTED", "WORKER_CONTRACT_VIOLATION"):
            return CycleResult(state="FAILED",
                               detail=f"nothing resumable from {run.get('status')}")
        try:
            lease = mem.TaskLease(self.repo, task_id)
        except TimeoutError:
            return CycleResult(state="TASK_BUSY", detail="task lease held by another process")
        try:
            try:
                dirty = worker.git_dirty_paths(packet["worktree"])
                head = worker.git_head(packet["worktree"])
                branch = worker.git_branch(packet["worktree"])
            except Exception as error:  # noqa: BLE001
                return CycleResult(state="FAILED", detail=f"worktree unreadable: {error}")
            if branch != packet["branch"]:
                return CycleResult(state="FAILED", detail="worktree branch changed; refusing")
            if not worker.is_ancestor(packet["worktree"], packet["base_sha"], head):
                return CycleResult(state="FAILED", detail="HEAD no longer descends from base")
            model_policy = self.policies.get("MODEL_POLICY.json", {})
            model_id, entry, family = self.select_model(packet)
            if not model_id:
                return CycleResult(state="FAILED", detail="no available model for resume")
            requested_effort = model_policy.get("roles", {}).get(packet["role"], {}).get("reasoning", "DEFAULT")
            _, effective_effort = worker.OpenCodeWorkerAdapter.reasoning_plan(entry, requested_effort)
            self._transition(task_id, "RUNNING", model_selected=model_id, model_family=family,
                             started=worker.current_time(), attempt=packet.get("attempt", 1),
                             reasoning_requested=requested_effort, reasoning_effective=effective_effort,
                             detail="resume: preserving interrupted work")
            adapter = self._worker_adapter(packet["role"], model_id)
            resume_packet = dict(packet)
            resume_packet["resume_context"] = worker.resume_prompt(packet, dirty or ["(clean tree)"], head)
            if isinstance(adapter, worker.OpenCodeWorkerAdapter):
                result = adapter.launch(resume_packet, timeout_s, model=model_id,
                                        policy_entry=entry, effort=requested_effort)
            else:
                result = adapter.launch(resume_packet, timeout_s, model=model_id,
                                        policy_entry=entry, effort=requested_effort)
            if result.status == "SUCCESS":
                reverified = worker.classify_worker_output(packet, head, packet["worktree"])
                # Resume candidate must still descend from the ORIGINAL base.
                if (reverified.status == "SUCCESS" and reverified.candidate_sha
                        and worker.is_ancestor(packet["worktree"], packet["base_sha"],
                                               reverified.candidate_sha)):
                    result = reverified
                elif reverified.status == "SUCCESS":
                    result = worker.WorkerResult(status="WORKER_CONTRACT_VIOLATION",
                                                 detail="resumed candidate lost base ancestry")
                else:
                    result = reverified
            if result.status != "SUCCESS":
                state = result.status if result.status in TRANSITIONS.get("RUNNING", ()) else "FAILED"
                if state == "FAILED":
                    self._transition(task_id, "FAILED", detail=result.detail)
                else:
                    self._transition(task_id, state, changed=result.changed_paths, detail=result.detail)
                return CycleResult(state=state, detail=result.detail)
            # Reuse the standard post-worker path by re-entering guards+review.
            self._transition(task_id, "CANDIDATE", candidate=result.candidate_sha,
                             changed=result.changed_paths)
            return self._finish_candidate(task_id, packet, result.candidate_sha or "")
        finally:
            lease.release()

    def _finish_candidate(self, task_id: str, packet: dict, candidate: str) -> CycleResult:
        authority = self.frozen_authority()
        if trust.verify_authority_fresh(authority):
            return CycleResult(state="FAILED", detail="TRUSTED_POLICY_MISMATCH")
        allow_info = self.authoritative_allowlist(authority, packet["checkpoint"])
        if "error" in allow_info:
            return CycleResult(state="REJECTED", candidate_sha=candidate, failures=[allow_info["error"]])
        gate = guards.guard_candidate(
            packet["worktree"], packet["base_sha"], candidate,
            allowed=packet.get("allowed_paths", []),
            forbidden=packet.get("forbidden_paths", []),
            plan_allowlist=tuple(allow_info["allowlist"]),
            supervisor=authority.get("module"))
        if not gate["accepted"]:
            self._transition(task_id, "REJECTED", candidate=candidate, detail="scope/protection")
            return CycleResult(state="REJECTED", candidate_sha=candidate,
                               failures=gate["protected_violations"] + gate["scope_violations"])
        return self._review(task_id, packet, candidate, gate, allow_info)

    def resume_info(self, task_id: str) -> dict:
        with mem.LockedState(self.repo) as locked:
            packet = locked.read_task(task_id)
            run = locked.read_run(task_id)
        if not packet or run.get("status") not in ("INTERRUPTED", "WORKER_CONTRACT_VIOLATION"):
            return {"ok": False, "error": "nothing resumable"}
        try:
            dirty = worker.git_dirty_paths(packet["worktree"])
            head = worker.git_head(packet["worktree"])
        except Exception as error:  # noqa: BLE001
            return {"ok": False, "error": f"worktree unreadable: {error}"}
        return {"ok": True, "task_id": task_id, "packet": packet, "dirty": dirty,
                "head": head, "resume_prompt": worker.resume_prompt(packet, dirty, head),
                "note": "same task, same branch, no reset, no clean"}

    # -- transitions ----------------------------------------------------------
    def _transition(self, task_id: str, to_status: str, **fields) -> None:
        with mem.LockedState(self.repo) as locked:
            run = locked.read_run(task_id)
            from_status = run.get("status", "")
            if not legal_transition(from_status, to_status):
                raise ValueError(f"illegal transition {from_status} -> {to_status}")
            run["status"] = to_status
            mapping = {"candidate": "candidate_sha", "changed": "changed_paths",
                       "model_selected": "model_selected", "model_family": "model_family",
                       "started": "started_at", "attempt": "attempt",
                       "next_action": "next_action", "detail": "detail",
                       "reasoning_requested": "reasoning_requested",
                       "reasoning_effective": "reasoning_effective",
                       "supervisor_digest": "supervisor_digest"}
            for key, value in fields.items():
                if key in mapping:
                    run[mapping[key]] = value
            if to_status in TERMINAL_SAFE:
                run["completed_at"] = worker.current_time()
            locked.write_run(task_id, run)
            current = locked.current()
            current["active_task"] = task_id
            if fields.get("candidate"):
                current["candidate_sha"] = fields["candidate"]
            locked.write_current(current)

    def _persist_review(self, task_id: str, model: str, family: str, outcome: dict) -> None:
        with mem.LockedState(self.repo) as locked:
            run = locked.read_run(task_id)
            run["review"] = {"model": model, "family": family,
                             "verdict": outcome.get("verdict"),
                             "findings": outcome.get("report", {}).get("findings", [])}
            locked.write_run(task_id, run)

    # -- routing helper ----------------------------------------------------------
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
        jev_model, jev_available = self._router_model()
        jev = router.JevAdapter(available=jev_available, model=jev_model)
        return {"task_id": task_id, "run_status": run.get("status"),
                "router_model": jev_model, "route": router.route(facts, jev)}

    def _router_model(self) -> tuple[str | None, bool]:
        model_policy = self.policies.get("MODEL_POLICY.json", {})
        preferred = model_policy.get("roles", {}).get("router", {}).get("preferred", [])
        discovered_ids = set()
        for entry in self.discovered:
            if hasattr(entry, "model_id"):
                if entry.state in ("AVAILABLE", "TEMPORARILY_FREE"):
                    discovered_ids.add(entry.model_id)
            elif isinstance(entry, dict) and entry.get("state") in ("AVAILABLE", "TEMPORARILY_FREE"):
                discovered_ids.add(entry.get("model_id", ""))
        for candidate in preferred:
            if candidate.get("model") in discovered_ids:
                return candidate.get("model"), True
        return None, False
