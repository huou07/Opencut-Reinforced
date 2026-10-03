"""Deterministic orchestrator tests — no real model calls, no quota spent."""
import json
import os
import subprocess
import sys
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
SCRIPTS = os.path.dirname(os.path.dirname(HERE))
sys.path.insert(0, SCRIPTS)

from model_orchestrator import escalation, failures, guards, model_discovery  # noqa: E402
from model_orchestrator import packets, policies, review, router, worker  # noqa: E402
from model_orchestrator import runtime_memory as mem  # noqa: E402

REPO = os.path.dirname(SCRIPTS)


def git(*args, cwd):
    return subprocess.run(["git", *args], cwd=cwd, capture_output=True, text=True, check=True, timeout=30)


def make_repo():
    tmp = tempfile.mkdtemp(prefix="or-orch-")
    git("init", "-q", cwd=tmp)
    git("config", "user.email", "test@example.com", cwd=tmp)
    git("config", "user.name", "test", cwd=tmp)
    with open(os.path.join(tmp, "README.md"), "w") as handle:
        handle.write("x\n")
    git("add", "-A", cwd=tmp)
    git("commit", "-qm", "init", cwd=tmp)
    return tmp


def packet_for(worktree, **over):
    base = packets.build_task_packet(
        task_id="T1", checkpoint="9B", base_sha="0" * 40, goal="bounded edit",
        branch="wip/t1", worktree=worktree, role="implementation", model="fake",
        allowed_paths=["src/a.rs"], forbidden_paths=["docs/execution/STATE.json"],
        invariants=["INV-RT-001"], acceptance=["a"], required_tests=["t"],
        stop_conditions=["2 attempts"], max_repair_attempts=2, attempt=1,
    )
    base.update(over)
    return base


class PacketTests(unittest.TestCase):
    def test_task_packet_schema(self):
        schemas = {"TASK_SCHEMAS.json": {"task_packet_required": ["task_id", "base_sha"]}}
        self.assertEqual(packets.validate_task_packet({"task_id": "T", "base_sha": "s"}, schemas), [])
        self.assertTrue(packets.validate_task_packet({}, schemas))

    def test_escalation_packet_schema(self):
        schemas = {"TASK_SCHEMAS.json": {"escalation_packet_required": ["question"]}}
        self.assertEqual(packets.validate_escalation_packet({"question": "q"}, schemas), [])


class WorkerTests(unittest.TestCase):
    def test_fake_worker_success_writes_files(self):
        repo = make_repo()
        packet = packet_for(repo, base_sha=worker.git_head(repo))
        result = worker.FakeWorkerAdapter(files={"src/a.rs": "fn a() {}\n"}).launch(packet, 60)
        self.assertEqual(result.status, "SUCCESS")
        with open(os.path.join(repo, "src/a.rs")) as handle:
            self.assertEqual(handle.read(), "fn a() {}\n")

    def test_worker_process_death_is_detected(self):
        repo = make_repo()
        packet = packet_for(repo)
        result = worker.FakeWorkerAdapter(behavior="die").launch(packet, 60)
        self.assertEqual(result.status, "PROCESS_DIED")

    def test_interrupted_dirty_worktree_preserved_on_resume(self):
        # Simulate: worker edits files, process dies, worktree stays dirty.
        repo = make_repo()
        packet = packet_for(repo)
        worker.FakeWorkerAdapter(files={"src/a.rs": "dirty\n"}).launch(packet, 60)
        run = {"status": "RUNNING", "task_id": "T1"}
        self.assertTrue(worker.detect_interruption(run, repo))
        # Resume must NOT reset: dirty file still present, same packet reused.
        with open(os.path.join(repo, "src/a.rs")) as handle:
            self.assertEqual(handle.read(), "dirty\n")
        resumed = dict(packet)
        self.assertEqual(resumed["task_id"], "T1")
        self.assertEqual(resumed["attempt"], 1)

    def test_no_change_detected(self):
        repo = make_repo()
        packet = packet_for(repo)
        result = worker.FakeWorkerAdapter(behavior="no-change").launch(packet, 60)
        self.assertEqual(result.status, "NO_CHANGE")


class GuardTests(unittest.TestCase):
    def test_protected_state_edit_rejected_before_review(self):
        repo = make_repo()
        base = worker.git_head(repo)
        packet = packet_for(repo, base_sha=base)
        worker.FakeWorkerAdapter(behavior="protected-edit").launch(packet, 60)
        git("add", "-A", cwd=repo)
        git("commit", "-qm", "evil", cwd=repo)
        candidate = worker.git_head(repo)
        protected = {"protected_exact": ["docs/execution/STATE.json"], "protected_prefixes": [], "protected_basenames": []}
        verdict = guards.guard_candidate(repo, base, candidate, protected)
        self.assertFalse(verdict["accepted"])
        self.assertTrue(any("STATE.json" in v for v in verdict["protected_violations"]))

    def test_clean_candidate_accepted(self):
        repo = make_repo()
        base = worker.git_head(repo)
        os.makedirs(os.path.join(repo, "src"), exist_ok=True)
        with open(os.path.join(repo, "src/a.rs"), "w") as handle:
            handle.write("ok\n")
        git("add", "-A", cwd=repo)
        git("commit", "-qm", "good", cwd=repo)
        candidate = worker.git_head(repo)
        protected = {"protected_exact": ["docs/execution/STATE.json"], "protected_prefixes": [], "protected_basenames": []}
        verdict = guards.guard_candidate(repo, base, candidate, protected)
        self.assertTrue(verdict["accepted"])


class ModelSelectionTests(unittest.TestCase):
    POLICY = {"roles": {"implementation": {"preferred": [
        {"model": "opencode/muse-spark-1.3-contributor-free", "family": "muse"},
        {"model": "opencode-go/deepseek-v4.1-flash", "family": "deepseek"}]}}}

    def test_model_unavailable_falls_back(self):
        discovered = [model_discovery.ModelInfo("opencode-go/deepseek-v4.1-flash", "AVAILABLE")]
        model_id, readiness = model_discovery.select_for_role("implementation", self.POLICY, discovered)
        self.assertEqual((model_id, readiness), ("opencode-go/deepseek-v4.1-flash", "READY"))

    def test_all_unavailable_continues_deterministically(self):
        model_id, readiness = model_discovery.select_for_role("implementation", self.POLICY, [])
        self.assertEqual((model_id, readiness), (None, "UNAVAILABLE"))

    def test_family_lookup(self):
        self.assertEqual(model_discovery.model_family("opencode/muse-spark-1.3-contributor-free", self.POLICY), "muse")


class ReviewTests(unittest.TestCase):
    POLICY = {"roles": {"review": {"preferred": [
        {"model": "opencode/nemotron-3-ultra-free", "family": "nemotron"}]}}}

    def test_different_family_routes_to_review(self):
        discovered = [{"model_id": "opencode/nemotron-3-ultra-free", "state": "AVAILABLE"}]
        candidates = review.reviewer_candidates(self.POLICY, discovered, exclude_family="muse")
        self.assertEqual(candidates, ["opencode/nemotron-3-ultra-free"])
        self.assertEqual(review.review_decision(
            implementer_family="muse", reviewer_model=candidates[0], reviewer_family="nemotron"), "APPROVED_ROUTE")

    def test_self_review_becomes_review_pending(self):
        # Implementation family Muse; only Muse reviewer available => REVIEW_PENDING.
        discovered = [{"model_id": "opencode/muse-spark-1.3-contributor-free", "state": "AVAILABLE"}]
        policy = {"roles": {"review": {"preferred": [
            {"model": "opencode/muse-spark-1.3-contributor-free", "family": "muse"}]}}}
        candidates = review.reviewer_candidates(policy, discovered, exclude_family="muse")
        self.assertEqual(candidates, [])
        self.assertEqual(review.review_decision(
            implementer_family="muse", reviewer_model=None, reviewer_family=None), "REVIEW_PENDING")


class RouterTests(unittest.TestCase):
    def test_jev_unavailable_falls_back_to_deterministic(self):
        jev = router.JevAdapter(available=False)
        result = router.route({"evidence_ready": True}, jev)
        self.assertEqual(result["decision"], "CAUSAL_PRODUCT_REPAIR")
        self.assertFalse(result["advisory"]["used"])

    def test_jev_invalid_label_ignored(self):
        jev = router.JevAdapter(available=True)
        result = router.route({"evidence_ready": True}, jev, scripted_jev="MAKE_IT_SO")
        self.assertEqual(result["decision"], "CAUSAL_PRODUCT_REPAIR")


class CodexQuotaTests(unittest.TestCase):
    QUOTA_TRANSCRIPT = "error: usage limit reached, limit resets in 4h59m"

    def test_quota_exhaustion_defers_without_failure(self):
        adapter = escalation.FakeCodexAdapter(transcript=self.QUOTA_TRANSCRIPT, returncode=1)
        result = adapter.escalate({"question": "q"})
        self.assertEqual(result.state, "ESCALATION_DEFERRED_QUOTA")
        # Packet preserved and resumable: same packet re-escalates identically.
        again = adapter.escalate({"question": "q"})
        self.assertEqual(again.state, "ESCALATION_DEFERRED_QUOTA")

    def test_codex_unavailable_is_not_a_test_failure(self):
        adapter = escalation.CodexAdapter(codex_bin="/nonexistent-codex-bin")
        result = adapter.escalate({"question": "q"})
        self.assertEqual(result.state, "UNAVAILABLE")

    def test_normal_response_resolves(self):
        adapter = escalation.FakeCodexAdapter(transcript='{"decision": "ok"}', returncode=0)
        result = adapter.escalate({"question": "q"})
        self.assertEqual(result.state, "RESOLVED")

    def test_quota_classification_matrix(self):
        self.assertEqual(escalation.classify_codex_output(1, "Quota exceeded for today"), "ESCALATION_DEFERRED_QUOTA")
        self.assertEqual(escalation.classify_codex_output(1, "not logged in, login required"), "AUTH_REQUIRED")
        self.assertEqual(escalation.classify_codex_output(0, "ok"), "RESOLVED")


class AntiThrashTests(unittest.TestCase):
    SIG = {"checkpoint": "9B", "gate": "android", "job": "apk", "step": "saf",
           "error_class": "driver", "assertion": "frame", "error_fingerprint": "disposed"}

    def test_two_attempts_then_diagnostic(self):
        history = [
            {"signature_id": failures.signature_id(self.SIG), "evidence_backed": True},
            {"signature_id": failures.signature_id(self.SIG), "evidence_backed": True},
        ]
        self.assertEqual(failures.thrash_decision(history, self.SIG), "DIAGNOSTIC")

    def test_architecture_class_escalates(self):
        sig = dict(self.SIG, error_class="architecture")
        history = [
            {"signature_id": failures.signature_id(sig), "evidence_backed": True},
            {"signature_id": failures.signature_id(sig), "evidence_backed": True},
        ]
        self.assertEqual(failures.thrash_decision(history, sig), "ARCHITECTURE_ESCALATION")

    def test_retry_does_not_reset_counter(self):
        history = [{"signature_id": failures.signature_id(self.SIG), "evidence_backed": True}]
        self.assertEqual(failures.thrash_decision(history, self.SIG), "ALLOW_REPAIR")


class HostedFixtureTests(unittest.TestCase):
    def test_9b_hosted_failure_is_inconclusive_not_rerun(self):
        outcome = failures.classify_known_9b()
        self.assertEqual(outcome["verdict"], "HOSTED_VERIFY_INCONCLUSIVE")
        self.assertFalse(outcome["auto_rerun"])
        self.assertEqual(outcome["fixture"]["candidate_sha"], "beaef3b7dba878d705dba07d7b9232860e184f83")
        self.assertEqual(outcome["fixture"]["hosted_run"], "37110360212")

    def test_inconclusive_is_never_auto_pass_or_proven_infra(self):
        verdict = failures.classify_hosted_outcome(
            product_assertions_reached=False, local_review="PASS",
            driver_disposed=True, adb_offline=True,
            same_signature_before_candidate=True, proven_infra_evidence=False)
        self.assertNotIn(verdict, ("ACCEPTED", "PRODUCT_DEFECT", "PROVEN_INFRASTRUCTURE_FAILURE"))
        self.assertEqual(verdict, "HOSTED_VERIFY_INCONCLUSIVE")


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
                locked.write_task("T9", packets.build_task_packet(
                    task_id="T9", checkpoint="9B", base_sha="a" * 40, goal="g",
                    branch="b", worktree=fake_repo, role="implementation", model="m",
                    allowed_paths=[], forbidden_paths=[], invariants=[], acceptance=[],
                    required_tests=[], stop_conditions=[]))
                self.assertEqual(locked.read_task("T9")["task_id"], "T9")


if __name__ == "__main__":
    unittest.main()
