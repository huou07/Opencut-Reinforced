"""Deterministic orchestrator tests — no real model calls, no quota spent.

Covers: packets, fake worker lifecycle, strict success contract, scope and
forbidden enforcement, supervisor-protection parity, review execution and
mutation/self-review, promotion success/remote-advanced, handoff refusal and
eligibility, reasoning effort, Jev fallback, Codex quota, thrash policy, the
known 9B hosted fixture, and the end-to-end fake pipeline through the real
orchestrator engine (the same method `run --auto` invokes).
"""
import json
import os
import subprocess
import sys
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
SCRIPTS = os.path.dirname(os.path.dirname(HERE))
sys.path.insert(0, SCRIPTS)

from model_orchestrator import dispatcher_safety, escalation, failures  # noqa: E402
from model_orchestrator import guards, model_discovery, orchestrator  # noqa: E402
from model_orchestrator import packets, policies, promotion, review, router  # noqa: E402
from model_orchestrator import scope, supervisor_link, worker  # noqa: E402
from model_orchestrator import runtime_memory as mem  # noqa: E402

REPO = os.path.dirname(SCRIPTS)
MUSE = "opencode/muse-spark-1.3-contributor-free"
NEMOTRON = "opencode/nemotron-3-ultra-free"


def git(*args, cwd):
    return subprocess.run(["git", *args], cwd=cwd, capture_output=True, text=True, check=True, timeout=30)


def make_repo():
    tmp = tempfile.mkdtemp(prefix="or-orch-")
    git("init", "-q", "-b", "main", cwd=tmp)
    git("config", "user.email", "test@example.com", cwd=tmp)
    git("config", "user.name", "test", cwd=tmp)
    with open(os.path.join(tmp, "README.md"), "w") as handle:
        handle.write("x\n")
    git("add", "-A", cwd=tmp)
    git("commit", "-qm", "init", cwd=tmp)
    return tmp


def write(repo, rel, content):
    dest = os.path.join(repo, rel)
    os.makedirs(os.path.dirname(dest), exist_ok=True)
    with open(dest, "w") as handle:
        handle.write(content)


def commit_all(repo, subject):
    git("add", "-A", cwd=repo)
    git("commit", "-qm", subject, cwd=repo)
    return worker.git_head(repo)


def packet_for(worktree, branch="wip/t1", **over):
    base = packets.build_task_packet(
        task_id="T1", checkpoint="9B", base_sha=worker.git_head(worktree), goal="bounded edit",
        branch=branch, worktree=worktree, role="implementation", model=MUSE,
        allowed_paths=["src/a/**"], forbidden_paths=[],
        invariants=["INV-RT-001"], acceptance=["a"], required_tests=["t"],
        stop_conditions=["2 attempts"], max_repair_attempts=2, attempt=1,
    )
    base.update(over)
    return base


def make_task_branch(repo, branch="wip/t1"):
    git("checkout", "-qb", branch, cwd=repo)
    return worker.git_head(repo)


def engine_with(fakes, discovered=None):
    loaded = policies.load_policies(REPO)
    discovered = discovered if discovered is not None else [
        model_discovery.ModelInfo(MUSE, "AVAILABLE"),
        model_discovery.ModelInfo(NEMOTRON, "AVAILABLE"),
    ]
    schemas = loaded
    return orchestrator.Orchestrator(REPO, loaded, schemas, discovered), fakes


class PacketTests(unittest.TestCase):
    def test_task_packet_schema(self):
        schemas = {"TASK_SCHEMAS.json": {"task_packet_required": ["task_id", "base_sha"]}}
        self.assertEqual(packets.validate_task_packet({"task_id": "T", "base_sha": "s"}, schemas), [])
        self.assertTrue(packets.validate_task_packet({}, schemas))

    def test_escalation_packet_schema(self):
        schemas = {"TASK_SCHEMAS.json": {"escalation_packet_required": ["question"]}}
        self.assertEqual(packets.validate_escalation_packet({"question": "q"}, schemas), [])


class ScopeTests(unittest.TestCase):
    def test_exact_recursive_and_broad(self):
        self.assertTrue(scope.pattern_matches("src/a.rs", "src/a.rs"))
        self.assertFalse(scope.pattern_matches("src/a.rs", "src/b.rs"))
        self.assertTrue(scope.pattern_matches("src/a/**", "src/a/good.rs"))
        self.assertTrue(scope.pattern_matches("src/a/**", "src/a/deep/x.rs"))
        self.assertFalse(scope.pattern_matches("src/a/**", "src/b/bad.rs"))
        self.assertTrue(scope.pattern_matches("**", "anything/at/all.rs"))

    def test_empty_allowed_means_nothing(self):
        result = scope.scope_violations(["src/a.rs"], [], [])
        self.assertEqual(result["out_of_scope"], ["src/a.rs"])

    def test_forbidden_wins_over_allowed(self):
        result = scope.scope_violations(["src/secrets/bad.rs"], ["src/**"], ["src/secrets/**"])
        self.assertEqual(result["forbidden"], ["src/secrets/bad.rs"])
        self.assertEqual(result["out_of_scope"], [])

    def test_normalization(self):
        self.assertTrue(scope.pattern_matches("src/a/**", ".\\src\\a\\x.rs".replace("\\", "/")))
        self.assertTrue(scope.pattern_matches("./src/a.rs", "src/a.rs"))


class WorkerContractTests(unittest.TestCase):
    def test_dirty_zero_exit_is_contract_violation(self):
        # §25: worker returns zero with dirty allowed file => NOT SUCCESS.
        repo = make_repo()
        base = make_task_branch(repo)
        packet = packet_for(repo, base_sha=base)
        write(repo, "src/a/dirty.rs", "dirty\n")
        fake = worker.FakeWorkerAdapter(behavior="dirty")
        result = fake.launch(packet, 60)
        # Engine re-verification (same call the engine performs):
        reverified = worker.classify_worker_output(packet, base, repo)
        self.assertEqual(reverified.status, "WORKER_CONTRACT_VIOLATION")
        self.assertNotEqual(result.candidate_sha, worker.git_head(repo) + "-changed")
        # Dirty file intact (never reset, never cleaned).
        with open(os.path.join(repo, "src/a/dirty.rs")) as handle:
            self.assertEqual(handle.read(), "dirty\n")
        self.assertNotEqual(reverified.candidate_sha, base)

    def test_committed_candidate_classifies_success(self):
        repo = make_repo()
        base = make_task_branch(repo)
        packet = packet_for(repo, base_sha=base)
        fake = worker.FakeWorkerAdapter(files={"src/a/good.rs": "ok\n"})
        result = fake.launch(packet, 60)
        self.assertEqual(result.status, "SUCCESS")
        self.assertTrue(worker.valid_sha(result.candidate_sha or ""))
        self.assertNotEqual(result.candidate_sha, base)

    def test_no_change_detected(self):
        repo = make_repo()
        make_task_branch(repo)
        packet = packet_for(repo)
        result = worker.FakeWorkerAdapter(behavior="no-change").launch(packet, 60)
        self.assertEqual(result.status, "NO_CHANGE")


class GuardTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.supervisor = guards.load_supervisor(REPO)

    def guard(self, *args, **kwargs):
        kwargs.setdefault("supervisor", self.supervisor)
        return guards.guard_candidate(*args, **kwargs)
    def test_protected_state_edit_rejected_before_review(self):
        repo = make_repo()
        base = make_task_branch(repo)
        packet = packet_for(repo, base_sha=base, allowed_paths=["**"])
        worker.FakeWorkerAdapter(behavior="protected-edit").launch(packet, 60)
        candidate = worker.git_head(repo)
        verdict = self.guard(repo, base, candidate, allowed=["**"])
        self.assertFalse(verdict["accepted"])
        self.assertTrue(any("STATE.json" in v for v in verdict["protected_violations"]))

    def test_out_of_scope_rejected_with_exact_path(self):
        # §26: allows src/a/**, worker commits src/a/good.rs + src/b/bad.rs.
        repo = make_repo()
        base = make_task_branch(repo)
        packet = packet_for(repo, base_sha=base)
        worker.FakeWorkerAdapter(files={"src/a/good.rs": "ok\n", "src/b/bad.rs": "bad\n"}).launch(packet, 60)
        candidate = worker.git_head(repo)
        verdict = self.guard(repo, base, candidate, allowed=["src/a/**"])
        self.assertFalse(verdict["accepted"])
        self.assertIn("out-of-scope: src/b/bad.rs", verdict["scope_violations"])

    def test_forbidden_path_rejected(self):
        # §27: allows src/**, forbids src/secrets/**.
        repo = make_repo()
        base = make_task_branch(repo)
        packet = packet_for(repo, base_sha=base, allowed_paths=["src/**"],
                            forbidden_paths=["src/secrets/**"])
        worker.FakeWorkerAdapter(files={"src/secrets/bad.rs": "x\n"}).launch(packet, 60)
        candidate = worker.git_head(repo)
        verdict = self.guard(repo, base, candidate,
                                   allowed=["src/**"], forbidden=["src/secrets/**"])
        self.assertFalse(verdict["accepted"])
        self.assertIn("forbidden-path: src/secrets/bad.rs", verdict["scope_violations"])

    def test_anti_gaming_blocks_promotion_not_review(self):
        repo = make_repo()
        base = make_task_branch(repo)
        # scripts/ tool file is NOT supervisor-protected, but || true in it
        # must still block automatic promotion via gaming flags.
        write(repo, "scripts/fake_tool.py", "run: test || true\n")
        candidate = commit_all(repo, "gaming")
        verdict = self.guard(repo, base, candidate, allowed=["**"])
        self.assertTrue(verdict["accepted"])
        self.assertTrue(verdict["promotion_blocked_by_gaming"])


class SupervisorParityTests(unittest.TestCase):
    PROTECTED = [
        "AGENTS.md",
        ".github/workflows/platform-verification.yml",
        "docs/execution/STATE.json",
        "docs/execution/PLAN.json",
        "docs/execution/phases/PHASE_9.md",
        "docs/execution/evidence/9B.json",
        "docs/execution/EVIDENCE_POLICY.json",
        "docs/execution/ARCHITECTURE_INVARIANTS.md",
        "docs/execution/architecture-policy.json",
        "docs/execution/AMENDMENT_BASELINE.json",
        "scripts/agent_supervisor.py",
        "scripts/execution_plan.py",
        "scripts/execution_evidence.py",
        "scripts/check_execution_plan.py",
        "scripts/check_architecture_policy.py",
        "scripts/test_execution_infra.py",
    ]

    def test_orchestrator_agrees_with_supervisor(self):
        for path in self.PROTECTED:
            with self.subTest(path=path):
                self.assertTrue(guards.is_supervisor_protected(REPO, path), path)

    def test_normal_product_file_not_protected(self):
        for path in ("crates/or_media/src/decoder.rs",
                     "scripts/model_orchestrator/worker.py",
                     "docs/execution/automation/README.md"):
            with self.subTest(path=path):
                self.assertFalse(guards.is_supervisor_protected(REPO, path), path)


class ReviewExecutionTests(unittest.TestCase):
    def _engine(self, tmp):
        loaded = policies.load_policies(REPO)
        discovered = [model_discovery.ModelInfo(MUSE, "AVAILABLE"),
                      model_discovery.ModelInfo(NEMOTRON, "AVAILABLE")]
        return orchestrator.Orchestrator(tmp, loaded, loaded, discovered)

    def test_reviewer_mutation_invalidates_review(self):
        # §29: PASS prose + mutated tree => invalid, never PROMOTION_READY.
        repo = make_repo()
        base = make_task_branch(repo)
        write(repo, "src/a/good.rs", "ok\n")
        candidate = commit_all(repo, "candidate")
        outcome = review.execute_review(
            worktree=repo, task_id="T1", base_sha=base, candidate_sha=candidate,
            changed_paths=["src/a/good.rs"], guard_summary={},
            adapter=review.FakeReviewAdapter(verdict="PASS", mutate=True),
            schemas=policies.load_policies(REPO))
        self.assertFalse(outcome["ok"])
        self.assertNotEqual(outcome["verdict"], "PASS")

    def test_self_review_stays_pending(self):
        # §30: implementer Muse, only Muse reviewers => REVIEW_PENDING.
        discovered = [{"model_id": MUSE, "state": "AVAILABLE"}]
        policy = {"roles": {"review": {"preferred": [{"model": MUSE, "family": "muse"}]}}}
        self.assertEqual(review.reviewer_candidates(policy, discovered, exclude_family="muse"), [])
        self.assertEqual(review.review_decision(
            implementer_family="muse", reviewer_model=None, reviewer_family=None), "REVIEW_PENDING")

    def test_arbitrary_verdict_rejected(self):
        errors = review.validate_review_report(
            {"task_id": "T", "candidate_sha": "c", "reviewer_model": "m",
             "reviewer_family": "f", "verdict": "looks fine to me", "findings": [], "tests_rerun": []},
            policies.load_policies(REPO))
        self.assertTrue(any("verdict" in e for e in errors))


class PromotionTests(unittest.TestCase):
    def _remote_setup(self):
        work = tempfile.mkdtemp(prefix="or-promo-")
        remote = os.path.join(work, "remote.git")
        subprocess.run(["git", "init", "-q", "--bare", remote], check=True, timeout=30)
        repo = os.path.join(work, "local")
        subprocess.run(["git", "clone", "-q", remote, repo], check=True, timeout=30)
        git("config", "user.email", "t@t", cwd=repo)
        git("config", "user.name", "t", cwd=repo)
        write(repo, "README.md", "x\n")
        base = commit_all(repo, "base")
        git("push", "-q", "origin", "main:main", cwd=repo)
        return work, repo, remote, base

    def test_promotion_success_fast_forward(self):
        # §31: BASE -> candidate fast-forwards remote and local main.
        work, repo, remote, base = self._remote_setup()
        git("checkout", "-qb", "wip/t1", cwd=repo)
        write(repo, "src/a/good.rs", "ok\n")
        candidate = commit_all(repo, "candidate")
        git("checkout", "-q", "main", cwd=repo)
        cand_wt = os.path.join(work, "cand")
        subprocess.run(["git", "worktree", "add", "--detach", cand_wt, candidate],
                       cwd=repo, check=True, timeout=30, capture_output=True)
        try:
            pre = promotion.check_promotion(
                main_repo=repo, candidate_repo=cand_wt, candidate_sha=candidate,
                reviewed_sha=candidate, base_sha=base,
                expected_checkpoint="9B", state_next="9B")
            self.assertTrue(pre["ok"], pre.get("failures"))
            result = promotion.promote(main_repo=repo, candidate_sha=candidate, precheck=pre)
            self.assertTrue(result["ok"], result)
            self.assertEqual(worker.git_head(repo), candidate)
            origin = subprocess.run(["git", "rev-parse", "origin/main"], cwd=repo,
                                    capture_output=True, text=True, check=True,
                                    timeout=30).stdout.strip()
            self.assertEqual(origin, candidate)
            log = subprocess.run(["git", "log", "--oneline", "origin/main"], cwd=repo,
                                 capture_output=True, text=True, check=True,
                                 timeout=30).stdout
            self.assertNotIn("Merge", log)
        finally:
            subprocess.run(["git", "worktree", "remove", "--force", cand_wt],
                           cwd=repo, timeout=30, capture_output=True)

    def test_remote_advanced_refuses(self):
        # §32: origin/main advanced to B; candidate from A => REFUSE.
        work, repo, remote, base = self._remote_setup()
        git("checkout", "-qb", "wip/t1", cwd=repo)
        write(repo, "src/a/good.rs", "ok\n")
        candidate = commit_all(repo, "candidate")
        git("checkout", "-q", "main", cwd=repo)
        write(repo, "other.txt", "b\n")
        commit_all(repo, "independent advance")
        git("push", "-q", "origin", "main:main", cwd=repo)
        git("fetch", "-q", "origin", cwd=repo)
        cand_wt = os.path.join(work, "cand")
        subprocess.run(["git", "worktree", "add", "--detach", cand_wt, candidate],
                       cwd=repo, check=True, timeout=30, capture_output=True)
        try:
            pre = promotion.check_promotion(
                main_repo=repo, candidate_repo=cand_wt, candidate_sha=candidate,
                reviewed_sha=candidate, base_sha=base,
                expected_checkpoint="9B", state_next="9B")
            self.assertFalse(pre["ok"])
            self.assertTrue(any("remote advanced" in f for f in pre["failures"]))
            result = promotion.promote(main_repo=repo, candidate_sha=candidate, precheck=pre)
            self.assertFalse(result["ok"])
        finally:
            subprocess.run(["git", "worktree", "remove", "--force", cand_wt],
                           cwd=repo, timeout=30, capture_output=True)


class HandoffTests(unittest.TestCase):
    def test_review_branch_handoff_refused(self):
        # §33: SHA only on review branch => structured refusal, no supervisor call.
        repo = make_repo()
        base = worker.git_head(repo)
        git("checkout", "-qb", "wip/review", cwd=repo)
        write(repo, "src/a/good.rs", "ok\n")
        candidate = commit_all(repo, "candidate")
        git("checkout", "-q", "main", cwd=repo)
        called = []

        def invoke(_command):
            called.append(True)
            return {"ok": True}

        result = supervisor_link.handoff_to_supervisor(repo, "9B", candidate, invoke=invoke)
        self.assertFalse(result["ok"])
        self.assertTrue(result.get("refused"))
        self.assertEqual(called, [])
        _ = base

    def test_handoff_eligible_after_promotion(self):
        # §34: HEAD == origin/main == candidate, clean, NEXT => invokable.
        work = tempfile.mkdtemp(prefix="or-hand-")
        remote = os.path.join(work, "remote.git")
        subprocess.run(["git", "init", "-q", "--bare", remote], check=True, timeout=30)
        repo = os.path.join(work, "local")
        subprocess.run(["git", "clone", "-q", remote, repo], check=True, timeout=30)
        git("config", "user.email", "t@t", cwd=repo)
        git("config", "user.name", "t", cwd=repo)
        write(repo, "README.md", "x\n")
        os.makedirs(os.path.join(repo, "docs", "execution"), exist_ok=True)
        with open(os.path.join(repo, "docs", "execution", "STATE.json"), "w") as handle:
            json.dump({"checkpoints": {"9B": "NEXT"}}, handle)
        commit_all(repo, "base")
        git("push", "-q", "origin", "main:main", cwd=repo)
        write(repo, "src/a/good.rs", "ok\n")
        candidate = commit_all(repo, "candidate")
        git("push", "-q", "origin", "main:main", cwd=repo)
        seen = []

        def invoke(command):
            seen.append(command)
            return {"ok": True, "fake": True}

        result = supervisor_link.handoff_to_supervisor(repo, "9B", candidate, invoke=invoke)
        self.assertTrue(result["ok"], result)
        self.assertTrue(any("--resume-sha" in c for c in seen))
        self.assertIn(candidate, seen[0])


class ReasoningEffortTests(unittest.TestCase):
    def test_supported_effort_uses_real_flag(self):
        adapter = worker.OpenCodeWorkerAdapter()
        command = adapter.build_command(
            packet_for(make_repo()), MUSE, {"variants": ["high"]}, "HIGH")
        self.assertIn("--variant", command)
        self.assertIn("high", command)
        variant, effective = adapter.reasoning_plan({"variants": ["high"]}, "HIGH")
        self.assertEqual((variant, effective), ("high", "HIGH"))

    def test_unsupported_effort_records_default(self):
        adapter = worker.OpenCodeWorkerAdapter()
        command = adapter.build_command(packet_for(make_repo()), MUSE, {}, "HIGH")
        self.assertNotIn("--variant", command)
        variant, effective = adapter.reasoning_plan({}, "HIGH")
        self.assertEqual((variant, effective), (None, "DEFAULT_PROVIDER"))


class JevTests(unittest.TestCase):
    def test_absent_jev_deterministic(self):
        jev = router.JevAdapter(available=False)
        result = router.route({"evidence_ready": True}, jev)
        self.assertEqual(result["decision"], "CAUSAL_PRODUCT_REPAIR")
        self.assertFalse(result["advisory"]["used"])

    def test_invalid_label_ignored(self):
        jev = router.JevAdapter(available=True)
        result = router.route({"evidence_ready": True}, jev, scripted_jev="MAKE_IT_SO")
        self.assertEqual(result["decision"], "CAUSAL_PRODUCT_REPAIR")

    def test_tool_failure_falls_back_without_raise(self):
        jev = router.JevAdapter(available=True, model="opencode/jev-x", opencode_bin="/nonexistent-jev")
        result = router.route({"evidence_ready": True}, jev)
        self.assertEqual(result["decision"], "CAUSAL_PRODUCT_REPAIR")
        self.assertFalse(result["advisory"]["used"])


class CodexQuotaTests(unittest.TestCase):
    QUOTA = "error: usage limit reached, limit resets in 4h59m"

    def test_quota_defers_preserves_packet(self):
        adapter = escalation.FakeCodexAdapter(transcript=self.QUOTA, returncode=1)
        first = adapter.escalate({"question": "q"})
        self.assertEqual(first.state, "ESCALATION_DEFERRED_QUOTA")
        self.assertEqual(adapter.escalate({"question": "q"}).state, "ESCALATION_DEFERRED_QUOTA")

    def test_unavailable_and_resolved(self):
        self.assertEqual(
            escalation.CodexAdapter(codex_bin="/nonexistent-codex-bin").escalate({"question": "q"}).state,
            "UNAVAILABLE")
        self.assertEqual(
            escalation.FakeCodexAdapter(transcript='{"d":1}', returncode=0).escalate({"q": 1}).state,
            "RESOLVED")


class AntiThrashTests(unittest.TestCase):
    SIG = {"checkpoint": "9B", "gate": "android", "job": "apk", "step": "saf",
           "error_class": "driver", "assertion": "frame", "error_fingerprint": "disposed"}

    def _history(self, sig, n):
        return [{"signature_id": failures.signature_id(sig), "evidence_backed": True} for _ in range(n)]

    def test_two_attempts_then_diagnostic(self):
        self.assertEqual(failures.thrash_decision(self._history(self.SIG, 2), self.SIG), "DIAGNOSTIC")

    def test_architecture_class_escalates(self):
        sig = dict(self.SIG, error_class="architecture")
        self.assertEqual(failures.thrash_decision(self._history(sig, 2), sig), "ARCHITECTURE_ESCALATION")

    def test_first_attempt_allowed(self):
        self.assertEqual(failures.thrash_decision(self._history(self.SIG, 1), self.SIG), "ALLOW_REPAIR")


class HostedFixtureTests(unittest.TestCase):
    def test_9b_stays_inconclusive(self):
        outcome = failures.classify_known_9b()
        self.assertEqual(outcome["verdict"], "HOSTED_VERIFY_INCONCLUSIVE")
        self.assertFalse(outcome["auto_rerun"])
        self.assertEqual(outcome["fixture"]["candidate_sha"], "beaef3b7dba878d705dba07d7b9232860e184f83")
        self.assertEqual(outcome["fixture"]["hosted_run"], "37110360212")


class DispatcherSafetyTests(unittest.TestCase):
    def test_dispatcher_enforced_readonly(self):
        self.assertEqual(dispatcher_safety.check_dispatcher(REPO), [])

    def test_worker_reviewer_files_constrained(self):
        self.assertEqual(dispatcher_safety.check_file_rules(
            REPO, "orch-worker", {"task": "deny", "external_directory": "deny"}), [])
        problems = dispatcher_safety.check_file_rules(
            REPO, "orch-reviewer",
            {"edit": "deny", "task": "deny", "external_directory": "deny",
             "bash": {"*": "deny", "git diff *": "allow"}})
        self.assertEqual(problems, [])


class EndToEndPipelineTests(unittest.TestCase):
    def test_fake_pipeline_reaches_promotion_ready(self):
        # §24: same engine method `run --auto` invokes.
        work = tempfile.mkdtemp(prefix="or-e2e-")
        subprocess.run(["git", "init", "-q", "-b", "main", work], check=True, timeout=30)
        git("config", "user.email", "t@t", cwd=work)
        git("config", "user.name", "t", cwd=work)
        write(work, "README.md", "x\n")
        commit_all(work, "init")
        git("checkout", "-qb", "wip/t1", cwd=work)
        base = worker.git_head(work)
        loaded = policies.load_policies(REPO)
        discovered = [model_discovery.ModelInfo(MUSE, "AVAILABLE"),
                      model_discovery.ModelInfo(NEMOTRON, "AVAILABLE")]
        packet = packets.build_task_packet(
            task_id="E2E", checkpoint="9B", base_sha=base, goal="bounded edit",
            branch="wip/t1", worktree=work, role="implementation", model=MUSE,
            allowed_paths=["src/a/**"], forbidden_paths=[],
            invariants=["INV-RT-001"], acceptance=["a"], required_tests=["t"],
            stop_conditions=["2 attempts"])
        adapters = orchestrator.Adapters(
            worker_factory=lambda _role, _model: worker.FakeWorkerAdapter(
                files={"src/a/good.rs": "ok\n"}),
            review_adapter=review.FakeReviewAdapter(verdict="PASS"))
        engine = orchestrator.Orchestrator(work, loaded, loaded, discovered, adapters,
                                           supervisor=guards.load_supervisor(REPO))
        self.assertEqual(engine.create_task(packet), [])
        result = engine.run_cycle("E2E")
        self.assertEqual(result.state, "PROMOTION_READY", result.detail or result.failures)
        self.assertTrue(worker.valid_sha(result.candidate_sha or ""))
        with mem.LockedState(work) as locked:
            run = locked.read_run("E2E")
        self.assertEqual(run["status"], "PROMOTION_READY")
        self.assertEqual(run["review"]["verdict"], "PASS")
        self.assertNotEqual(run["review"]["family"], "muse")
        # No STATE/PLAN change, no supervisor invocation (no evidence dir).
        self.assertFalse(os.path.exists(os.path.join(work, "docs", "execution", "STATE.json")))

    def test_e2e_dirty_worker_never_promotes(self):
        work = tempfile.mkdtemp(prefix="or-e2e-dirty-")
        subprocess.run(["git", "init", "-q", "-b", "main", work], check=True, timeout=30)
        git("config", "user.email", "t@t", cwd=work)
        git("config", "user.name", "t", cwd=work)
        write(work, "README.md", "x\n")
        commit_all(work, "init")
        git("checkout", "-qb", "wip/t1", cwd=work)
        base = worker.git_head(work)
        loaded = policies.load_policies(REPO)
        discovered = [model_discovery.ModelInfo(MUSE, "AVAILABLE"),
                      model_discovery.ModelInfo(NEMOTRON, "AVAILABLE")]
        packet = packets.build_task_packet(
            task_id="E2ED", checkpoint="9B", base_sha=base, goal="bounded edit",
            branch="wip/t1", worktree=work, role="implementation", model=MUSE,
            allowed_paths=["src/a/**"], forbidden_paths=[],
            invariants=[], acceptance=[], required_tests=[], stop_conditions=[])
        # Dirt arrives from the worker itself (zero exit, uncommitted files),
        # never pre-existing: preconditions require a clean tree.
        adapters = orchestrator.Adapters(
            worker_factory=lambda _role, _model: worker.FakeWorkerAdapter(behavior="dirty"),
            review_adapter=review.FakeReviewAdapter(verdict="PASS"))
        engine = orchestrator.Orchestrator(work, loaded, loaded, discovered, adapters,
                                           supervisor=guards.load_supervisor(REPO))
        self.assertEqual(engine.create_task(packet), [])
        result = engine.run_cycle("E2ED")
        self.assertEqual(result.state, "WORKER_CONTRACT_VIOLATION", result.detail)
        with open(os.path.join(work, "src/a/dirty.rs")) as handle:
            self.assertEqual(handle.read(), "dirty\n")

    def test_e2e_out_of_scope_rejected_before_review(self):
        work = tempfile.mkdtemp(prefix="or-e2e-scope-")
        subprocess.run(["git", "init", "-q", "-b", "main", work], check=True, timeout=30)
        git("config", "user.email", "t@t", cwd=work)
        git("config", "user.name", "t", cwd=work)
        write(work, "README.md", "x\n")
        commit_all(work, "init")
        git("checkout", "-qb", "wip/t1", cwd=work)
        base = worker.git_head(work)
        loaded = policies.load_policies(REPO)
        discovered = [model_discovery.ModelInfo(MUSE, "AVAILABLE"),
                      model_discovery.ModelInfo(NEMOTRON, "AVAILABLE")]
        packet = packets.build_task_packet(
            task_id="E2ES", checkpoint="9B", base_sha=base, goal="bounded edit",
            branch="wip/t1", worktree=work, role="implementation", model=MUSE,
            allowed_paths=["src/a/**"], forbidden_paths=[],
            invariants=[], acceptance=[], required_tests=[], stop_conditions=[])
        adapters = orchestrator.Adapters(
            worker_factory=lambda _role, _model: worker.FakeWorkerAdapter(
                files={"src/a/good.rs": "ok\n", "src/b/bad.rs": "bad\n"}),
            review_adapter=review.FakeReviewAdapter(verdict="PASS"))
        engine = orchestrator.Orchestrator(work, loaded, loaded, discovered, adapters,
                                           supervisor=guards.load_supervisor(REPO))
        self.assertEqual(engine.create_task(packet), [])
        result = engine.run_cycle("E2ES")
        self.assertEqual(result.state, "REJECTED")
        self.assertTrue(any("src/b/bad.rs" in f for f in result.failures))


class PolicyTests(unittest.TestCase):
    def test_tracked_policies_validate(self):
        loaded = policies.load_policies(REPO)
        self.assertEqual(policies.validate_policies(loaded), [])

    def test_runtime_memory_roundtrip(self):
        with tempfile.TemporaryDirectory() as fake_repo:
            git("init", "-q", cwd=fake_repo)
            with mem.LockedState(fake_repo) as locked:
                locked.write_current({"active_task": "T9"})
                self.assertEqual(locked.current()["active_task"], "T9")


if __name__ == "__main__":
    unittest.main()
