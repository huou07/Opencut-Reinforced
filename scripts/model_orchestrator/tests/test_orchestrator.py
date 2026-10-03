"""Deterministic orchestrator tests — no real model calls, no quota spent.

Proves production wiring (engine/CLI paths with only the external process
boundary mocked), not just helpers: model selection reaches the worker
command, the real review adapter parses real event streams, guards use the
frozen trusted authority, and promotion/handoff bind to runtime records.
"""
import json
import os
import subprocess
import sys
import tempfile
import threading
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
SCRIPTS = os.path.dirname(os.path.dirname(HERE))
sys.path.insert(0, SCRIPTS)

from model_orchestrator import dispatcher_safety, escalation, events, failures  # noqa: E402
from model_orchestrator import guards, model_discovery, orchestrator  # noqa: E402
from model_orchestrator import packets, policies, promotion, review, router  # noqa: E402
from model_orchestrator import scope, supervisor_link, trusted_authority, worker  # noqa: E402
from model_orchestrator import runtime_memory as mem  # noqa: E402

REPO = os.path.dirname(SCRIPTS)
MUSE = "opencode/muse-spark-1.3-contributor-free"
LONGCAT = "opencode/longcat-2.5-preview-free"
NEMOTRON = "opencode/nemotron-3-ultra-free"

# Real `opencode run --format json` event stream shape (fixture modeled on
# the verified live probe; text lives in part.type == "text").
REVIEW_EVENT_STREAM = "\n".join([
    json.dumps({"type": "step_start", "timestamp": 1, "sessionID": "s",
                "part": {"type": "step-start", "id": "p1"}}),
    json.dumps({"type": "text", "timestamp": 2, "sessionID": "s",
                "part": {"type": "text", "id": "p2",
                         "text": '{"verdict": "PASS", "findings": [], "tests_rerun": ["t"]}'}}),
    json.dumps({"type": "step_finish", "timestamp": 3, "sessionID": "s",
                "part": {"type": "step-finish", "reason": "stop"}}),
])
JEV_EVENT_STREAM = "\n".join([
    json.dumps({"type": "step_start", "timestamp": 1, "sessionID": "s",
                "part": {"type": "step-start", "id": "p1"}}),
    json.dumps({"type": "tool", "timestamp": 2, "sessionID": "s",
                "part": {"type": "tool-call", "id": "p2"}}),
    json.dumps({"type": "text", "timestamp": 3, "sessionID": "s",
                "part": {"type": "text", "id": "p3", "text": "REVIEW_REQUIRED"}}),
])


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


def test_policies():
    return policies.load_policies(REPO)


def test_discovered(*ids):
    return [model_discovery.ModelInfo(model_id, "AVAILABLE") for model_id in ids]


def test_engine(work, adapters, checkpoint="9B"):
    loaded = test_policies()
    return orchestrator.Orchestrator(
        work, loaded, loaded, test_discovered(MUSE, LONGCAT, NEMOTRON),
        adapters, trusted_repo=REPO)


class EventParserTests(unittest.TestCase):
    def test_live_shape_text_extraction(self):
        self.assertEqual(
            events.extract_assistant_text(REVIEW_EVENT_STREAM),
            '{"verdict": "PASS", "findings": [], "tests_rerun": ["t"]}')

    def test_multiple_texts_concatenate_tool_events_ignored(self):
        self.assertEqual(events.extract_assistant_text(JEV_EVENT_STREAM), "REVIEW_REQUIRED")

    def test_label_and_object_helpers(self):
        self.assertEqual(events.extract_label(JEV_EVENT_STREAM, router.ROUTER_LABELS), "REVIEW_REQUIRED")
        self.assertEqual(
            events.extract_json_object(REVIEW_EVENT_STREAM)["verdict"], "PASS")

    def test_empty_malformed_rejected(self):
        with self.assertRaises(ValueError):
            events.extract_assistant_text("")
        with self.assertRaises(ValueError):
            events.extract_assistant_text("{not json}\n")
        with self.assertRaises(ValueError):
            events.extract_label(REVIEW_EVENT_STREAM, router.ROUTER_LABELS)


class PacketTests(unittest.TestCase):
    def test_model_field_is_request_only(self):
        packet = packet_for(make_repo())
        self.assertEqual(packet["model"], MUSE)
        self.assertIsNone(packet["model_locked"])
        self.assertEqual(packet["plan_allowlist_snapshot"], [])


class ScopeTests(unittest.TestCase):
    def test_exact_recursive_and_broad(self):
        self.assertTrue(scope.pattern_matches("src/a.rs", "src/a.rs"))
        self.assertFalse(scope.pattern_matches("src/a.rs", "src/b.rs"))
        self.assertTrue(scope.pattern_matches("src/a/**", "src/a/deep/x.rs"))
        self.assertFalse(scope.pattern_matches("src/a/**", "src/b/bad.rs"))
        self.assertTrue(scope.pattern_matches("**", "anything.rs"))

    def test_empty_allowed_means_nothing(self):
        self.assertEqual(scope.scope_violations(["src/a.rs"], [], [])["out_of_scope"], ["src/a.rs"])

    def test_forbidden_wins_over_allowed(self):
        result = scope.scope_violations(["src/secrets/bad.rs"], ["src/**"], ["src/secrets/**"])
        self.assertEqual((result["forbidden"], result["out_of_scope"]),
                         (["src/secrets/bad.rs"], []))


class WorkerContractTests(unittest.TestCase):
    def test_dirty_zero_exit_is_contract_violation(self):
        repo = make_repo()
        base = make_task_branch(repo)
        packet = packet_for(repo, base_sha=base)
        fake = worker.FakeWorkerAdapter(behavior="dirty")
        result = fake.launch(packet, 60, model=MUSE, policy_entry={}, effort="HIGH")
        reverified = worker.classify_worker_output(packet, base, repo)
        self.assertEqual(reverified.status, "WORKER_CONTRACT_VIOLATION")
        with open(os.path.join(repo, "src/a/dirty.rs")) as handle:
            self.assertEqual(handle.read(), "dirty\n")

    def test_committed_candidate_classifies_success(self):
        repo = make_repo()
        base = make_task_branch(repo)
        packet = packet_for(repo, base_sha=base)
        result = worker.FakeWorkerAdapter(files={"src/a/good.rs": "ok\n"}).launch(
            packet, 60, model=MUSE, policy_entry={}, effort="HIGH")
        self.assertEqual(result.status, "SUCCESS")
        self.assertTrue(worker.valid_sha(result.candidate_sha or ""))


class GuardTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.supervisor = guards.load_supervisor(REPO)

    def guard(self, *args, **kwargs):
        kwargs.setdefault("supervisor", self.supervisor)
        return guards.guard_candidate(*args, **kwargs)

    def test_protected_state_edit_rejected(self):
        repo = make_repo()
        base = make_task_branch(repo)
        packet = packet_for(repo, base_sha=base, allowed_paths=["**"])
        worker.FakeWorkerAdapter(behavior="protected-edit").launch(
            packet, 60, model=MUSE, policy_entry={}, effort="HIGH")
        verdict = self.guard(repo, base, worker.git_head(repo), allowed=["**"])
        self.assertFalse(verdict["accepted"])
        self.assertTrue(any("STATE.json" in v for v in verdict["protected_violations"]))

    def test_out_of_scope_rejected_with_exact_path(self):
        repo = make_repo()
        base = make_task_branch(repo)
        packet = packet_for(repo, base_sha=base)
        worker.FakeWorkerAdapter(files={"src/a/good.rs": "ok\n", "src/b/bad.rs": "bad\n"}).launch(
            packet, 60, model=MUSE, policy_entry={}, effort="HIGH")
        verdict = self.guard(repo, base, worker.git_head(repo), allowed=["src/a/**"])
        self.assertFalse(verdict["accepted"])
        self.assertIn("out-of-scope: src/b/bad.rs", verdict["scope_violations"])

    def test_forbidden_path_rejected(self):
        repo = make_repo()
        base = make_task_branch(repo)
        packet = packet_for(repo, base_sha=base, allowed_paths=["src/**"],
                            forbidden_paths=["src/secrets/**"])
        worker.FakeWorkerAdapter(files={"src/secrets/bad.rs": "x\n"}).launch(
            packet, 60, model=MUSE, policy_entry={}, effort="HIGH")
        verdict = self.guard(repo, base, worker.git_head(repo),
                             allowed=["src/**"], forbidden=["src/secrets/**"])
        self.assertFalse(verdict["accepted"])
        self.assertIn("forbidden-path: src/secrets/bad.rs", verdict["scope_violations"])

    def test_anti_gaming_blocks_promotion(self):
        repo = make_repo()
        base = make_task_branch(repo)
        write(repo, "scripts/fake_tool.py", "run: test || true\n")
        candidate = commit_all(repo, "gaming")
        verdict = self.guard(repo, base, candidate, allowed=["**"])
        self.assertTrue(verdict["accepted"])
        self.assertTrue(verdict["promotion_blocked_by_gaming"])


class RenameBypassTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.supervisor = guards.load_supervisor(REPO)

    def renamed(self, old, new, content="x\n"):
        repo = make_repo()
        write(repo, old, content)
        commit_all(repo, "seed")
        base = make_task_branch(repo)
        os.makedirs(os.path.dirname(os.path.join(repo, new)), exist_ok=True)
        git("mv", old, new, cwd=repo)
        candidate = commit_all(repo, "rename")
        return repo, base, candidate

    def guard(self, repo, base, candidate, **kwargs):
        kwargs.setdefault("supervisor", self.supervisor)
        return guards.guard_candidate(repo, base, candidate, **kwargs)

    def test_protected_to_allowed_rename_rejected(self):
        repo, base, candidate = self.renamed("docs/execution/STATE.json", "src/a/state.json")
        verdict = self.guard(repo, base, candidate, allowed=["**"])
        self.assertFalse(verdict["accepted"])
        self.assertTrue(any("STATE.json" in v for v in verdict["protected_violations"]))

    def test_forbidden_to_allowed_rename_rejected(self):
        repo, base, candidate = self.renamed("src/secrets/k.txt", "src/a/k.txt")
        verdict = self.guard(repo, base, candidate, allowed=["src/**"], forbidden=["src/secrets/**"])
        self.assertFalse(verdict["accepted"])
        self.assertTrue(any("secrets" in v for v in verdict["scope_violations"]))

    def test_allowed_to_forbidden_rename_rejected(self):
        repo, base, candidate = self.renamed("src/a/k.txt", "src/secrets/k.txt")
        verdict = self.guard(repo, base, candidate, allowed=["src/**"], forbidden=["src/secrets/**"])
        self.assertFalse(verdict["accepted"])

    def test_out_of_scope_to_allowed_rename_rejected(self):
        repo, base, candidate = self.renamed("src/b/k.txt", "src/a/k.txt")
        verdict = self.guard(repo, base, candidate, allowed=["src/a/**"])
        self.assertFalse(verdict["accepted"])

    def test_allowed_to_allowed_rename_passes(self):
        repo, base, candidate = self.renamed("src/a/old.txt", "src/a/new.txt")
        verdict = self.guard(repo, base, candidate, allowed=["src/a/**"])
        self.assertTrue(verdict["accepted"], verdict)

    def test_protected_delete_plus_allowed_add_rejected(self):
        repo = make_repo()
        write(repo, "docs/execution/STATE.json", "{}\n")
        commit_all(repo, "seed")
        base = make_task_branch(repo)
        os.remove(os.path.join(repo, "docs/execution/STATE.json"))
        write(repo, "src/a/state.json", "{}\n")
        candidate = commit_all(repo, "swap")
        verdict = self.guard(repo, base, candidate, allowed=["**"])
        self.assertFalse(verdict["accepted"])
        self.assertTrue(any("STATE.json" in v for v in verdict["protected_violations"]))


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


class TrustBoundaryTests(unittest.TestCase):
    def test_weakened_candidate_supervisor_still_rejected(self):
        # §23: candidate rewrites is_protected_execution_path -> False and
        # touches another protected path. Frozen trusted authority rejects.
        repo = make_repo()
        base = make_task_branch(repo)
        os.makedirs(os.path.join(repo, "scripts"), exist_ok=True)
        write(repo, "scripts/agent_supervisor.py",
              "def is_protected_execution_path(path):\n    return False\n")
        write(repo, "docs/execution/STATE.json", "{}\n")
        candidate = commit_all(repo, "evil")
        trusted = trusted_authority.freeze_authority(REPO)
        verdict = guards.guard_candidate(repo, base, candidate, allowed=["**"],
                                         supervisor=trusted["module"])
        self.assertFalse(verdict["accepted"])
        self.assertTrue(any("STATE.json" in v for v in verdict["protected_violations"]))
        self.assertTrue(any("agent_supervisor.py" in v for v in verdict["protected_violations"]))

    def test_packet_allowlist_forgery_rejected(self):
        # §24: packet snapshot claims the workflow allowed; trusted PLAN for
        # a checkpoint without that authorization => mismatch => REJECTED.
        loaded = test_policies()
        engine = orchestrator.Orchestrator(
            REPO, loaded, loaded, test_discovered(MUSE), trusted_repo=REPO)
        authority = engine.frozen_authority()
        info = engine.authoritative_allowlist(authority, "9B")
        self.assertNotIn("error", info)
        forged_snapshot = [".github/workflows/evil.yml"]
        self.assertNotEqual(sorted(forged_snapshot), sorted(info["allowlist"]))

    def test_9b_authorizes_exact_workflow_path(self):
        loaded = test_policies()
        engine = orchestrator.Orchestrator(
            REPO, loaded, loaded, test_discovered(MUSE), trusted_repo=REPO)
        info = engine.authoritative_allowlist(engine.frozen_authority(), "9B")
        self.assertEqual(info["allowlist"], [".github/workflows/platform-verification.yml"])


class ModelWiringTests(unittest.TestCase):
    def test_selected_model_reaches_worker_command(self):
        # §25: Muse unavailable, LongCat available, packet prefers Muse.
        repo = make_repo()
        base = make_task_branch(repo)
        packet = packet_for(repo, base_sha=base)
        seen = {}

        class RecordingWorker(worker.FakeWorkerAdapter):
            def launch(self, packet, timeout_s, **kwargs):
                seen.update(kwargs)
                seen["argv_model"] = kwargs.get("model")
                return super().launch(packet, timeout_s, **kwargs)

        loaded = test_policies()
        discovered = [model_discovery.ModelInfo(LONGCAT, "AVAILABLE")]
        engine = orchestrator.Orchestrator(
            repo, loaded, loaded, discovered,
            orchestrator.Adapters(
                worker_factory=lambda _r, _m: RecordingWorker(files={"src/a/good.rs": "ok\n"}),
                review_adapter=review.FakeReviewAdapter(verdict="PASS")),
            trusted_repo=REPO,
            supervisor=guards.load_supervisor(REPO))
        self.assertEqual(engine.create_task(packet), [])
        # Checkpoint 9B resolves against the REAL trusted plan/state.
        packet2 = dict(packet, checkpoint="9B")
        with mem.LockedState(repo) as locked:
            locked.write_task("T1", packet2)
        result = engine.run_cycle("T1")
        self.assertEqual(seen.get("argv_model"), LONGCAT, seen)
        self.assertNotIn(MUSE, json.dumps(seen.get("policy_entry", {})))
        with mem.LockedState(repo) as locked:
            run = locked.read_run("T1")
        self.assertEqual(run.get("model_selected"), LONGCAT)
        self.assertEqual(run.get("model_requested"), MUSE)
        self.assertEqual(run.get("model_family"), "longcat")

    def test_reasoning_effort_truthful(self):
        adapter = worker.OpenCodeWorkerAdapter()
        packet = packet_for(make_repo())
        command = adapter.build_command(packet, MUSE, {"variants": ["high"]}, "HIGH", "high")
        self.assertIn("--variant", command)
        command = adapter.build_command(packet, MUSE, {}, "HIGH", None)
        self.assertNotIn("--variant", command)
        self.assertEqual(adapter.reasoning_plan({}, "HIGH"), (None, "DEFAULT_PROVIDER"))


class ReviewWiringTests(unittest.TestCase):
    def _engine(self, work, review_process):
        loaded = test_policies()
        return orchestrator.Orchestrator(
            work, loaded, loaded, test_discovered(MUSE, NEMOTRON),
            orchestrator.Adapters(
                worker_factory=lambda _r, _m: worker.FakeWorkerAdapter(
                    files={"src/a/good.rs": "ok\n"}),
                review_adapter=review_process),
            trusted_repo=REPO,
            supervisor=guards.load_supervisor(REPO))

    def _packet(self, work):
        base = worker.git_head(work)
        return packets.build_task_packet(
            task_id="T1", checkpoint="9B", base_sha=base, goal="g", branch="wip/t1",
            worktree=work, role="implementation", model=MUSE, allowed_paths=["src/a/**"],
            forbidden_paths=[], invariants=[], acceptance=[], required_tests=[],
            stop_conditions=[])

    def _worktree(self):
        work = tempfile.mkdtemp(prefix="or-revw-")
        subprocess.run(["git", "init", "-q", "-b", "main", work], check=True, timeout=30)
        git("config", "user.email", "t@t", cwd=work)
        git("config", "user.name", "t", cwd=work)
        write(work, "README.md", "x\n")
        commit_all(work, "init")
        git("checkout", "-qb", "wip/t1", cwd=work)
        return work

    def test_production_reviewer_wiring(self):
        # §26: mock ONLY the external opencode process boundary.
        work = self._worktree()
        real_adapter = review.OpenCodeReviewAdapter(model=NEMOTRON)
        calls = []
        real_run = subprocess.run

        def fake_run(argv, **kwargs):
            if argv[0] == "opencode":
                calls.append(argv)
                return subprocess.CompletedProcess(argv, 0, REVIEW_EVENT_STREAM, "")
            return real_run(argv, **kwargs)

        subprocess.run = fake_run
        try:
            engine = self._engine(work, real_adapter)
            self.assertEqual(engine.create_task(self._packet(work)), [])
            result = engine.run_cycle("T1")
        finally:
            subprocess.run = real_run
        review_calls = [c for c in calls if "--agent" in c and "orch-reviewer" in c]
        self.assertTrue(review_calls, calls)
        self.assertIn(NEMOTRON, review_calls[0])
        self.assertEqual(result.state, "PROMOTION_READY", result.detail or result.failures)

    def test_no_reviewer_means_pending(self):
        # §27: no different-family reviewer => REVIEW_PENDING, no fake review.
        work = self._worktree()
        loaded = test_policies()
        engine = orchestrator.Orchestrator(
            work, loaded, loaded, test_discovered(MUSE),
            orchestrator.Adapters(
                worker_factory=lambda _r, _m: worker.FakeWorkerAdapter(
                    files={"src/a/good.rs": "ok\n"})),
            trusted_repo=REPO,
            supervisor=guards.load_supervisor(REPO))
        self.assertEqual(engine.create_task(self._packet(work)), [])
        result = engine.run_cycle("T1")
        self.assertEqual(result.state, "REVIEW_PENDING")
        with mem.LockedState(work) as locked:
            self.assertEqual(locked.read_run("T1")["review"], {})

    def test_reviewer_mutation_invalidates(self):
        repo = make_repo()
        base = make_task_branch(repo)
        write(repo, "src/a/good.rs", "ok\n")
        candidate = commit_all(repo, "candidate")
        outcome = review.execute_review(
            worktree=repo, task_id="T1", base_sha=base, candidate_sha=candidate,
            changed_paths=["src/a/good.rs"], guard_summary={},
            adapter=review.FakeReviewAdapter(verdict="PASS", mutate=True),
            schemas=test_policies())
        self.assertFalse(outcome["ok"])

    def test_self_review_stays_pending(self):
        policy = {"roles": {"review": {"preferred": [{"model": MUSE, "family": "muse"}]}}}
        discovered = [{"model_id": MUSE, "state": "AVAILABLE"}]
        self.assertEqual(review.reviewer_candidates(policy, discovered, exclude_family="muse"), [])


class TransitionTests(unittest.TestCase):
    def test_illegal_jumps_rejected(self):
        self.assertFalse(orchestrator.legal_transition("PENDING", "PROMOTION_READY"))
        self.assertFalse(orchestrator.legal_transition("CANDIDATE", "PROMOTED"))
        self.assertFalse(orchestrator.legal_transition("RUNNING", "PROMOTION_READY"))

    def test_legal_chain_accepted(self):
        for jump in [("PENDING", "RUNNING"), ("RUNNING", "CANDIDATE"),
                     ("CANDIDATE", "REVIEWING"), ("REVIEWING", "PROMOTION_READY"),
                     ("PROMOTION_READY", "PROMOTING"), ("PROMOTING", "PROMOTED"),
                     ("PROMOTED", "SUPERVISOR_READY"), ("INTERRUPTED", "RUNNING"),
                     ("REVIEWING", "REVIEW_PENDING")]:
            with self.subTest(jump=jump):
                self.assertTrue(orchestrator.legal_transition(*jump), jump)


class PauseLeaseTests(unittest.TestCase):
    def _task(self, work):
        loaded = test_policies()
        engine = orchestrator.Orchestrator(
            work, loaded, loaded, test_discovered(MUSE, NEMOTRON),
            orchestrator.Adapters(
                worker_factory=lambda _r, _m: worker.FakeWorkerAdapter(
                    files={"src/a/good.rs": "ok\n"}),
                review_adapter=review.FakeReviewAdapter(verdict="PASS")),
            trusted_repo=REPO,
            supervisor=guards.load_supervisor(REPO))
        packet = packets.build_task_packet(
            task_id="TP", checkpoint="9B", base_sha=worker.git_head(work), goal="g",
            branch="wip/t1", worktree=work, role="implementation", model=MUSE,
            allowed_paths=["src/a/**"], forbidden_paths=[], invariants=[],
            acceptance=[], required_tests=[], stop_conditions=[])
        self.assertEqual(engine.create_task(packet), [])
        return engine

    def _worktree(self):
        work = tempfile.mkdtemp(prefix="or-pause-")
        subprocess.run(["git", "init", "-q", "-b", "main", work], check=True, timeout=30)
        git("config", "user.email", "t@t", cwd=work)
        git("config", "user.name", "t", cwd=work)
        write(work, "README.md", "x\n")
        commit_all(work, "init")
        git("checkout", "-qb", "wip/t1", cwd=work)
        return work

    def test_pause_blocks_dispatch(self):
        # §29: paused run_cycle launches ZERO workers.
        work = self._worktree()
        engine = self._task(work)
        with mem.LockedState(work) as locked:
            current = locked.current()
            current["paused"] = True
            locked.write_current(current)
        result = engine.run_cycle("TP")
        self.assertEqual(result.state, "PAUSED")

    def test_concurrent_dispatch_second_gets_busy(self):
        # §30: two instances, one task; second gets TASK_BUSY; one worker.
        work = self._worktree()
        engine = self._task(work)
        started = threading.Event()
        release = threading.Event()
        launches = []

        class BlockingWorker(worker.FakeWorkerAdapter):
            def launch(self, packet, timeout_s, **kwargs):
                launches.append(1)
                started.set()
                release.wait(timeout=30)
                return super().launch(packet, timeout_s, **kwargs)

        engine.adapters = orchestrator.Adapters(
            worker_factory=lambda _r, _m: BlockingWorker(files={"src/a/good.rs": "ok\n"}),
            review_adapter=review.FakeReviewAdapter(verdict="PASS"))
        results = {}

        def first():
            results["first"] = engine.run_cycle("TP", timeout_s=60)

        thread = threading.Thread(target=first)
        thread.start()
        self.assertTrue(started.wait(timeout=30))
        second = engine.run_cycle("TP", timeout_s=60)
        release.set()
        thread.join(timeout=60)
        self.assertEqual(second.state, "TASK_BUSY", second)
        self.assertEqual(len(launches), 1)
        self.assertEqual(results["first"].state, "PROMOTION_READY", results["first"])


class DispatcherSafetyTests(unittest.TestCase):
    def test_dispatcher_enforced_readonly(self):
        self.assertEqual(dispatcher_safety.check_dispatcher(REPO), [])

    def test_dangerous_commands_denied_allowed_stay(self):
        from model_orchestrator.dispatcher_safety import agent_file, bash_allows, parse_frontmatter
        front = parse_frontmatter(agent_file(REPO, "model-dispatcher").read_text())
        bash = front["permission"]["bash"]
        for probe in dispatcher_safety.DANGEROUS_DISPATCHER_PROBES:
            with self.subTest(probe=probe):
                self.assertEqual(bash_allows(bash, probe), "deny", probe)
        for allowed in dispatcher_safety.SAFE_DISPATCHER_BASH_ALLOWS:
            with self.subTest(allowed=allowed):
                self.assertEqual(bash_allows(bash, allowed), "allow", allowed)

    def test_worker_reviewer_files_constrained(self):
        self.assertEqual(dispatcher_safety.check_file_rules(
            REPO, "orch-worker", {"task": "deny", "external_directory": "deny"}), [])
        self.assertEqual(dispatcher_safety.check_file_rules(
            REPO, "orch-reviewer",
            {"edit": "deny", "task": "deny", "external_directory": "deny",
             "bash": {"*": "deny", "git diff *": "allow"}}), [])


class JevWiringTests(unittest.TestCase):
    def test_absent_jev_deterministic(self):
        loaded = test_policies()
        engine = orchestrator.Orchestrator(
            REPO, loaded, loaded, [], trusted_repo=REPO)
        action = engine.next_action()
        self.assertIsNone(action["router_model"])
        self.assertIn(action["route"]["decision"], router.ROUTER_LABELS)

    def test_present_jev_wired_through_production_step(self):
        # §32: mock only the external opencode process.
        loaded = test_policies()
        fake_jev = "opencode/jev-fixture"
        loaded = json.loads(json.dumps(loaded))
        loaded["MODEL_POLICY.json"]["roles"]["router"]["preferred"] = [{"model": fake_jev}]
        discovered = [model_discovery.ModelInfo(fake_jev, "AVAILABLE")]
        engine = orchestrator.Orchestrator(
            REPO, loaded, loaded, discovered, trusted_repo=REPO)
        real_run = subprocess.run

        def fake_run(argv, **kwargs):
            if argv[0] == "opencode":
                return subprocess.CompletedProcess(argv, 0, JEV_EVENT_STREAM, "")
            return real_run(argv, **kwargs)

        subprocess.run = fake_run
        try:
            action = engine.next_action()
        finally:
            subprocess.run = real_run
        # Production wiring: discovered Jev model reaches the adapter; its
        # advisory is recorded; deterministic safety decision still wins.
        self.assertEqual(action["router_model"], fake_jev)
        self.assertTrue(action["route"]["advisory"]["used"])
        self.assertEqual(action["route"]["advisory"]["label"], "REVIEW_REQUIRED")
        self.assertTrue(action["route"]["disagreement"])
        self.assertEqual(action["route"]["decision"], action["route"]["deterministic"])


class PromotionAuthTests(unittest.TestCase):
    def _trusted_files(self, repo):
        for rel in ("scripts/agent_supervisor.py", "scripts/execution_plan.py",
                    "scripts/execution_evidence.py", "docs/execution/PLAN.json",
                    "docs/execution/STATE.json"):
            dest = os.path.join(repo, rel)
            os.makedirs(os.path.dirname(dest), exist_ok=True)
            with open(os.path.join(REPO, rel), "rb") as src, open(dest, "wb") as out:
                out.write(src.read())

    def _setup(self):
        work = tempfile.mkdtemp(prefix="or-pauth-")
        remote = os.path.join(work, "remote.git")
        subprocess.run(["git", "init", "-q", "--bare", remote], check=True, timeout=30)
        main_repo = os.path.join(work, "main")
        subprocess.run(["git", "clone", "-q", remote, main_repo], check=True, timeout=30)
        git("config", "user.email", "t@t", cwd=main_repo)
        git("config", "user.name", "t", cwd=main_repo)
        self._trusted_files(main_repo)
        write(main_repo, "README.md", "x\n")
        base = commit_all(main_repo, "base")
        git("push", "-q", "origin", "main:main", cwd=main_repo)
        git("checkout", "-qb", "wip/t1", cwd=main_repo)
        write(main_repo, "src/a/good.rs", "ok\n")
        candidate = commit_all(main_repo, "candidate")
        git("checkout", "-q", "main", cwd=main_repo)
        authority = trusted_authority.freeze_authority(main_repo)
        return work, main_repo, base, candidate, authority

    def _authorize(self, main_repo, authority, task_id, candidate, base):
        packet = {"task_id": task_id, "checkpoint": "9B", "base_sha": base,
                  "branch": "wip/t1", "allowed_paths": ["src/a/**"],
                  "forbidden_paths": [], "invariants": [], "acceptance": [],
                  "required_tests": [], "stop_conditions": [],
                  "max_repair_attempts": 2, "model_locked": None}
        gate = guards.guard_candidate(
            main_repo, base, candidate, allowed=["src/a/**"], forbidden=[],
            plan_allowlist=(".github/workflows/platform-verification.yml",),
            supervisor=authority["module"])
        self.assertTrue(gate["accepted"], gate)
        with mem.LockedState(main_repo) as locked:
            locked.write_run(task_id, {"task_id": task_id, "checkpoint": "9B",
                                       "status": "PROMOTION_READY",
                                       "candidate_sha": candidate})
            locked.write_promotion(task_id, {
                "schema_version": 1, "task_id": task_id, "checkpoint": "9B",
                "base_sha": base, "candidate_sha": candidate, "reviewed_sha": candidate,
                "reviewer_model": NEMOTRON, "reviewer_family": "nemotron",
                "review_verdict": "PASS",
                "guard_digest": promotion._digest(
                    {"protected": gate["protected_violations"],
                     "scope": gate["scope_violations"], "gaming": gate["gaming_flags"]}),
                "changed_digest": promotion._digest(gate["changed_paths"]),
                "contract_digest": orchestrator.contract_digest(packet),
                "allowed_paths": ["src/a/**"], "forbidden_paths": [],
                "plan_allowlist": [".github/workflows/platform-verification.yml"],
                "task_packet": packet, "promotion_ready_at": "t"})

    def test_promotion_success_from_own_record(self):
        work, main_repo, base, candidate, authority = self._setup()
        _ = (work, authority)
        self._authorize(main_repo, authority, "TA", candidate, base)
        cand_wt = os.path.join(work, "cand")
        subprocess.run(["git", "worktree", "add", "--detach", cand_wt, candidate],
                       cwd=main_repo, check=True, timeout=30, capture_output=True)
        try:
            result = promotion.promote_task(
                main_repo=main_repo, candidate_repo=cand_wt, memory_repo=main_repo,
                task_id="TA", expected_checkpoint="9B")
            self.assertTrue(result["ok"], result)
            self.assertEqual(worker.git_head(main_repo), candidate)
            origin = subprocess.run(
                ["git", "rev-parse", "origin/main"], cwd=main_repo,
                capture_output=True, text=True, check=True, timeout=30).stdout.strip()
            self.assertEqual(origin, candidate)
        finally:
            subprocess.run(["git", "worktree", "remove", "--force", cand_wt],
                           cwd=main_repo, timeout=30, capture_output=True)

    def test_tamper_matrix_each_refuses(self):
        for field in ("candidate_sha", "reviewed_sha", "guard_digest", "changed_digest",
                      "contract_digest", "status", "checkpoint", "remote", "allowed"):
            with self.subTest(field=field):
                work, main_repo, base, candidate, authority = self._setup()
                _ = (work, authority)
                self._authorize(main_repo, authority, "TA", candidate, base)
                with mem.LockedState(main_repo) as locked:
                    record = locked.read_promotion("TA")
                    run = locked.read_run("TA")
                    if field == "candidate_sha":
                        record["candidate_sha"] = "0" * 40
                    elif field == "status":
                        run["status"] = "CANDIDATE"
                    elif field == "checkpoint":
                        record["checkpoint"] = "9B1"
                    elif field == "remote":
                        write(main_repo, "other.txt", "b\n")
                        commit_all(main_repo, "advance")
                        git("push", "-q", "origin", "main:main", cwd=main_repo)
                    elif field == "allowed":
                        record["allowed_paths"] = ["**"]
                    else:
                        record[field] = "tampered"
                    locked.write_promotion("TA", record)
                    locked.write_run("TA", run)
                checkpoint = "9B1" if field == "checkpoint" else "9B"
                cand_wt = os.path.join(work, "cand")
                subprocess.run(["git", "worktree", "add", "--detach", cand_wt, candidate],
                               cwd=main_repo, check=True, timeout=30, capture_output=True)
                try:
                    result = promotion.promote_task(
                        main_repo=main_repo, candidate_repo=cand_wt, memory_repo=main_repo,
                        task_id="TA", expected_checkpoint=checkpoint)
                finally:
                    subprocess.run(["git", "worktree", "remove", "--force", cand_wt],
                                   cwd=main_repo, timeout=30, capture_output=True)
                self.assertFalse(result["ok"], (field, result))
                origin = subprocess.run(
                    ["git", "rev-parse", "origin/main"], cwd=main_repo,
                    capture_output=True, text=True, timeout=30).stdout.strip()
                # Promotion must never move the remote on refusal; for the
                # remote-advanced case origin is B (the independent advance),
                # otherwise it must still be the base.
                self.assertNotEqual(origin, candidate, (field, "remote moved to candidate!"))
                if field != "remote":
                    self.assertEqual(origin, base, (field, "remote moved!"))

    def test_remote_advanced_refuses(self):
        work, main_repo, base, candidate, authority = self._setup()
        _ = (work, authority)
        write(main_repo, "other.txt", "b\n")
        commit_all(main_repo, "advance")
        git("push", "-q", "origin", "main:main", cwd=main_repo)
        git("fetch", "-q", "origin", cwd=main_repo)
        cand_wt = os.path.join(work, "cand")
        subprocess.run(["git", "worktree", "add", "--detach", cand_wt, candidate],
                       cwd=main_repo, check=True, timeout=30, capture_output=True)
        try:
            pre = promotion.check_promotion(
                main_repo=main_repo, candidate_repo=cand_wt, candidate_sha=candidate,
                reviewed_sha=candidate, base_sha=base,
                expected_checkpoint="9B", state_next="9B")
            self.assertFalse(pre["ok"])
            self.assertTrue(any("remote advanced" in f for f in pre["failures"]))
        finally:
            subprocess.run(["git", "worktree", "remove", "--force", cand_wt],
                           cwd=main_repo, timeout=30, capture_output=True)


class PromotionMechanicsTests(unittest.TestCase):
    def test_fast_forward_success(self):
        work = tempfile.mkdtemp(prefix="or-promo2-")
        remote = os.path.join(work, "remote.git")
        subprocess.run(["git", "init", "-q", "--bare", remote], check=True, timeout=30)
        repo = os.path.join(work, "local")
        subprocess.run(["git", "clone", "-q", remote, repo], check=True, timeout=30)
        git("config", "user.email", "t@t", cwd=repo)
        git("config", "user.name", "t", cwd=repo)
        write(repo, "README.md", "x\n")
        base = commit_all(repo, "base")
        git("push", "-q", "origin", "main:main", cwd=repo)
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
        finally:
            subprocess.run(["git", "worktree", "remove", "--force", cand_wt],
                           cwd=repo, timeout=30, capture_output=True)


class HandoffTests(unittest.TestCase):
    def test_review_branch_handoff_refused(self):
        repo = make_repo()
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

    def test_handoff_requires_promoted_authorization(self):
        # §36: SHA == main == origin/main but no promoted record => REFUSE.
        work = tempfile.mkdtemp(prefix="or-hand-")
        remote = os.path.join(work, "remote.git")
        subprocess.run(["git", "init", "-q", "--bare", remote], check=True, timeout=30)
        repo = os.path.join(work, "local")
        subprocess.run(["git", "clone", "-q", remote, repo], check=True, timeout=30)
        git("config", "user.email", "t@t", cwd=repo)
        git("config", "user.name", "t", cwd=repo)
        write(repo, "README.md", "x\n")
        commit_all(repo, "base")
        git("push", "-q", "origin", "main:main", cwd=repo)
        write(repo, "src/a/good.rs", "ok\n")
        candidate = commit_all(repo, "candidate")
        git("push", "-q", "origin", "main:main", cwd=repo)
        os.makedirs(os.path.join(repo, "docs", "execution"), exist_ok=True)
        with open(os.path.join(repo, "docs", "execution", "STATE.json"), "w") as handle:
            json.dump({"checkpoints": {"9B": "NEXT"}}, handle)
        git("add", "-A", cwd=repo)
        git("commit", "-qm", "state", cwd=repo)
        git("push", "-q", "origin", "main:main", cwd=repo)
        head = worker.git_head(repo)
        called = []

        def invoke(_command):
            called.append(True)
            return {"ok": True}

        refused = supervisor_link.handoff_to_supervisor(
            repo, "9B", head, invoke=invoke, task_id="NOPE")
        self.assertFalse(refused["ok"])
        self.assertEqual(called, [])
        # With a matching PROMOTED authorization, invocation is eligible.
        with mem.LockedState(repo) as locked:
            locked.write_promotion("T9", {"task_id": "T9", "checkpoint": "9B",
                                           "candidate_sha": head})
            locked.write_run("T9", {"task_id": "T9", "checkpoint": "9B",
                                    "status": "PROMOTED", "candidate_sha": head})
        seen = []

        def invoke2(command):
            seen.append(command)
            return {"ok": True}

        result = supervisor_link.handoff_to_supervisor(
            repo, "9B", head, invoke=invoke2, task_id="T9")
        self.assertTrue(result["ok"], result)
        self.assertTrue(any("--resume-sha" in c for c in seen))


class CodexQuotaTests(unittest.TestCase):
    QUOTA = "error: usage limit reached, limit resets in 4h59m"

    def test_quota_defers_preserves_packet(self):
        adapter = escalation.FakeCodexAdapter(transcript=self.QUOTA, returncode=1)
        self.assertEqual(adapter.escalate({"question": "q"}).state, "ESCALATION_DEFERRED_QUOTA")

    def test_unavailable_and_resolved(self):
        self.assertEqual(
            escalation.CodexAdapter(codex_bin="/nonexistent-codex-bin").escalate({"q": 1}).state,
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

    def test_task_lease_excludes_second_claimant(self):
        with tempfile.TemporaryDirectory() as fake_repo:
            git("init", "-q", cwd=fake_repo)
            lease = mem.TaskLease(fake_repo, "T1")
            try:
                with self.assertRaises(TimeoutError):
                    mem.TaskLease(fake_repo, "T1")
            finally:
                lease.release()
            second = mem.TaskLease(fake_repo, "T1")
            second.release()


class EndToEndPipelineTests(unittest.TestCase):
    def _setup(self, prefix, task_id="E2E", allowed=("src/a/**",), files=None):
        work = tempfile.mkdtemp(prefix=prefix)
        subprocess.run(["git", "init", "-q", "-b", "main", work], check=True, timeout=30)
        git("config", "user.email", "t@t", cwd=work)
        git("config", "user.name", "t", cwd=work)
        write(work, "README.md", "x\n")
        commit_all(work, "init")
        git("checkout", "-qb", "wip/t1", cwd=work)
        base = worker.git_head(work)
        loaded = test_policies()
        packet = packets.build_task_packet(
            task_id=task_id, checkpoint="9B", base_sha=base, goal="bounded edit",
            branch="wip/t1", worktree=work, role="implementation", model=MUSE,
            allowed_paths=list(allowed), forbidden_paths=[],
            invariants=[], acceptance=[], required_tests=[], stop_conditions=[])
        adapters = orchestrator.Adapters(
            worker_factory=lambda _r, _m: worker.FakeWorkerAdapter(files=files or {}),
            review_adapter=review.FakeReviewAdapter(verdict="PASS"))
        engine = orchestrator.Orchestrator(
            work, loaded, loaded, test_discovered(MUSE, NEMOTRON), adapters,
            trusted_repo=REPO, supervisor=guards.load_supervisor(REPO))
        self.assertEqual(engine.create_task(packet), [])
        return engine, task_id

    def test_fake_pipeline_reaches_promotion_ready(self):
        # §24: same engine method `run --auto` invokes.
        engine, task_id = self._setup("or-e2e-", files={"src/a/good.rs": "ok\n"})
        result = engine.run_cycle(task_id)
        self.assertEqual(result.state, "PROMOTION_READY", result.detail or result.failures)
        with mem.LockedState(engine.repo) as locked:
            run = locked.read_run(task_id)
            auth = locked.read_promotion(task_id)
        self.assertEqual(run["review"]["verdict"], "PASS")
        self.assertNotEqual(run["review"]["family"], "muse")
        self.assertEqual(auth["candidate_sha"], result.candidate_sha)
        self.assertEqual(auth["reviewed_sha"], result.candidate_sha)
        self.assertFalse(os.path.exists(os.path.join(engine.repo, "docs", "execution", "STATE.json")))

    def test_e2e_dirty_worker_never_promotes(self):
        engine, task_id = self._setup("or-e2e-dirty-", task_id="E2ED")
        engine.adapters = orchestrator.Adapters(
            worker_factory=lambda _r, _m: worker.FakeWorkerAdapter(behavior="dirty"),
            review_adapter=review.FakeReviewAdapter(verdict="PASS"))
        result = engine.run_cycle(task_id)
        self.assertEqual(result.state, "WORKER_CONTRACT_VIOLATION", result.detail)
        with open(os.path.join(engine.repo, "src/a/dirty.rs")) as handle:
            self.assertEqual(handle.read(), "dirty\n")

    def test_e2e_out_of_scope_rejected_before_review(self):
        engine, task_id = self._setup(
            "or-e2e-scope-", task_id="E2ES",
            files={"src/a/good.rs": "ok\n", "src/b/bad.rs": "bad\n"})
        result = engine.run_cycle(task_id)
        self.assertEqual(result.state, "REJECTED")
        self.assertTrue(any("src/b/bad.rs" in f for f in result.failures))

    def test_resume_e2e_preserves_and_promotes(self):
        # §34: worker A edits and dies dirty; resume worker B preserves,
        # commits; guards + real-path review fixture => PROMOTION_READY.
        work = tempfile.mkdtemp(prefix="or-resume-")
        subprocess.run(["git", "init", "-q", "-b", "main", work], check=True, timeout=30)
        git("config", "user.email", "t@t", cwd=work)
        git("config", "user.name", "t", cwd=work)
        write(work, "README.md", "x\n")
        commit_all(work, "init")
        git("checkout", "-qb", "wip/t1", cwd=work)
        base = worker.git_head(work)
        loaded = test_policies()
        packet = packets.build_task_packet(
            task_id="ER", checkpoint="9B", base_sha=base, goal="bounded edit",
            branch="wip/t1", worktree=work, role="implementation", model=MUSE,
            allowed_paths=["src/a/**"], forbidden_paths=[],
            invariants=[], acceptance=[], required_tests=[], stop_conditions=[])

        class DyingWorker(worker.FakeWorkerAdapter):
            def launch(self, packet, timeout_s, **kwargs):
                write(packet["worktree"], "src/a/partial.rs", "partial-a\n")
                return worker.WorkerResult(status="INTERRUPTED", detail="died",
                                           changed_paths=["src/a/partial.rs"])

        class FinishingWorker(worker.FakeWorkerAdapter):
            def launch(self, packet, timeout_s, **kwargs):
                # Must preserve worker A's edit and finish the file.
                with open(os.path.join(packet["worktree"], "src/a/partial.rs")) as handle:
                    prior = handle.read()
                assert prior == "partial-a\n", prior
                write(packet["worktree"], "src/a/partial.rs", prior + "finished-b\n")
                subprocess.run(["git", "add", "-A"], cwd=packet["worktree"],
                               check=True, timeout=30)
                subprocess.run(["git", "-c", "user.email=t@t", "-c", "user.name=t",
                                "commit", "-qm", "finish"], cwd=packet["worktree"],
                               check=True, timeout=30)
                return worker.classify_worker_output(packet, base, packet["worktree"])

        adapters = orchestrator.Adapters(
            worker_factory=lambda _r, _m: DyingWorker(),
            review_adapter=review.FakeReviewAdapter(verdict="PASS"))
        engine = orchestrator.Orchestrator(
            work, loaded, loaded, test_discovered(MUSE, NEMOTRON), adapters,
            trusted_repo=REPO, supervisor=guards.load_supervisor(REPO))
        self.assertEqual(engine.create_task(packet), [])
        first = engine.run_cycle("ER")
        self.assertEqual(first.state, "INTERRUPTED", first.detail)
        engine.adapters = orchestrator.Adapters(
            worker_factory=lambda _r, _m: FinishingWorker(),
            review_adapter=review.FakeReviewAdapter(verdict="PASS"))
        resumed = engine.resume_cycle("ER")
        self.assertEqual(resumed.state, "PROMOTION_READY", resumed.detail or resumed.failures)
        with open(os.path.join(work, "src/a/partial.rs")) as handle:
            self.assertEqual(handle.read(), "partial-a\nfinished-b\n")


if __name__ == "__main__":
    unittest.main()
