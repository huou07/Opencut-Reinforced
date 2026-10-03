"""Worker launch and strict result classification (Layer B).

Topology: orchestrator -> one worker -> stop. Workers never spawn workers.
SUCCESS requires a committed, clean, descendant candidate — dirty work is
never a candidate (WORKER_CONTRACT_VIOLATION or INTERRUPTED) and is always
preserved, never reset.

Reasoning effort is truthful: `reasoning_requested` comes from role policy;
`reasoning_effective` is a real supported `--variant` value or
DEFAULT_PROVIDER. No invented flags.
"""
from __future__ import annotations

import re
import subprocess
import time
from dataclasses import dataclass, field

SHA_RE = re.compile(r"^[0-9a-f]{40}$")

# Requested policy effort -> real `opencode run --variant` value, applied
# ONLY when the model entry declares support. Unknown => omitted.
EFFORT_VARIANT = {"HIGH": "high", "LOW": "minimal"}
WORKER_AGENT = "orch-worker"
REVIEWER_AGENT = "orch-reviewer"


@dataclass
class WorkerResult:
    status: str  # SUCCESS | WORKER_CONTRACT_VIOLATION | INTERRUPTED | NO_CHANGE | TIMEOUT | TOOL_ERROR | PROCESS_DIED
    candidate_sha: str | None = None
    changed_paths: list[str] = field(default_factory=list)
    report: dict = field(default_factory=dict)
    detail: str = ""


def valid_sha(value: str | None) -> bool:
    return isinstance(value, str) and bool(SHA_RE.match(value))


def git_head(repo: str) -> str:
    return subprocess.run(
        ["git", "rev-parse", "HEAD"], cwd=repo,
        capture_output=True, text=True, check=True, timeout=30,
    ).stdout.strip()


def git_branch(repo: str) -> str:
    return subprocess.run(
        ["git", "rev-parse", "--abbrev-ref", "HEAD"], cwd=repo,
        capture_output=True, text=True, check=True, timeout=30,
    ).stdout.strip()


def git_dirty_paths(repo: str) -> list[str]:
    out = subprocess.run(
        ["git", "status", "--porcelain", "--untracked-files=all"], cwd=repo,
        capture_output=True, text=True, check=True, timeout=30,
    ).stdout
    return sorted(line[3:].strip().strip('"') for line in out.splitlines() if len(line) >= 4)


def is_ancestor(repo: str, base: str, candidate: str) -> bool:
    proc = subprocess.run(
        ["git", "merge-base", "--is-ancestor", base, candidate],
        cwd=repo, capture_output=True, timeout=30,
    )
    return proc.returncode == 0


def branch_contains(repo: str, branch: str, sha: str) -> bool:
    proc = subprocess.run(
        ["git", "branch", "--contains", sha, "--list", branch],
        cwd=repo, capture_output=True, text=True, timeout=30,
    )
    return branch in proc.stdout.split()


def classify_worker_output(packet: dict, before_head: str, worktree: str) -> WorkerResult:
    """Strict post-run inspection. Model prose is never trusted.

    SUCCESS <=> branch matches AND worktree clean AND HEAD changed AND valid
    SHA AND base is ancestor AND candidate on task branch AND diff exists.
    Zero-exit with dirty files => WORKER_CONTRACT_VIOLATION (dirty preserved).
    """
    try:
        after_head = git_head(worktree)
        after_branch = git_branch(worktree)
        dirty = git_dirty_paths(worktree)
    except subprocess.SubprocessError as error:
        return WorkerResult(status="INTERRUPTED", detail=f"worktree unreadable: {error}")
    expected_branch = packet["branch"]
    if after_branch != expected_branch:
        return WorkerResult(status="WORKER_CONTRACT_VIOLATION",
                            detail=f"branch {after_branch!r} != packet {expected_branch!r}",
                            changed_paths=dirty)
    if dirty:
        return WorkerResult(status="WORKER_CONTRACT_VIOLATION",
                            detail=f"dirty worktree after worker exit: {dirty}",
                            changed_paths=dirty)
    if after_head == before_head:
        return WorkerResult(status="NO_CHANGE", detail="HEAD unchanged and worktree clean")
    if not valid_sha(after_head):
        return WorkerResult(status="WORKER_CONTRACT_VIOLATION",
                            detail=f"invalid candidate SHA: {after_head!r}")
    if not is_ancestor(worktree, packet["base_sha"], after_head):
        return WorkerResult(status="WORKER_CONTRACT_VIOLATION",
                            detail="candidate does not descend from packet base SHA")
    if not branch_contains(worktree, expected_branch, after_head):
        return WorkerResult(status="WORKER_CONTRACT_VIOLATION",
                            detail="candidate not reachable from task branch")
    try:
        diff = subprocess.run(
            ["git", "diff", "--name-only", f"{packet['base_sha']}..{after_head}"],
            cwd=worktree, capture_output=True, text=True, check=True, timeout=60,
        ).stdout.strip()
    except subprocess.SubprocessError as error:
        return WorkerResult(status="WORKER_CONTRACT_VIOLATION", detail=f"diff unreadable: {error}")
    if not diff:
        return WorkerResult(status="NO_CHANGE", detail="empty candidate diff")
    return WorkerResult(status="SUCCESS", candidate_sha=after_head,
                        changed_paths=sorted(diff.splitlines()))


def resume_prompt(packet: dict, dirty: list[str], last_head: str) -> str:
    return (
        "This is EXISTING INTERRUPTED work on the same task — do not start over. "
        f"Task {packet['task_id']} checkpoint {packet['checkpoint']}. "
        f"Current HEAD {last_head}; dirty paths: {', '.join(dirty)}. "
        "Inspect the current diff, preserve valid work, continue the task packet goal, "
        "and commit the result on the same branch."
    )


class WorkerAdapter:
    name = "base"
    agent = WORKER_AGENT

    def launch(self, packet: dict, timeout_s: int) -> WorkerResult:
        raise NotImplementedError


class FakeWorkerAdapter(WorkerAdapter):
    """Deterministic fake for tests. Behavior scripted, no models involved."""

    name = "fake"

    def __init__(self, behavior: str = "success", files: dict[str, str] | None = None,
                 commit: bool = True) -> None:
        self.behavior = behavior  # success | die | no-change | protected-edit | dirty
        self.files = files or {}
        self.commit = commit
        self.received: dict = {}  # records what the engine actually passed

    def launch(self, packet: dict, timeout_s: int, *, model: str = "",
               policy_entry: dict | None = None, effort: str = "DEFAULT") -> WorkerResult:
        _ = timeout_s
        self.received = {"model": model, "policy_entry": policy_entry or {}, "effort": effort}
        worktree = packet["worktree"]
        before = git_head(worktree)
        if self.behavior == "die":
            return WorkerResult(status="INTERRUPTED", detail="fake worker died mid-task",
                                changed_paths=git_dirty_paths(worktree))
        if self.behavior == "no-change":
            return WorkerResult(status="NO_CHANGE", detail="fake worker changed nothing")
        import os

        for rel, content in self.files.items():
            dest = os.path.join(worktree, rel)
            os.makedirs(os.path.dirname(dest), exist_ok=True)
            with open(dest, "w", encoding="utf-8") as handle:
                handle.write(content)
        if self.behavior == "protected-edit":
            dest = os.path.join(worktree, "docs/execution/STATE.json")
            os.makedirs(os.path.dirname(dest), exist_ok=True)
            with open(dest, "w", encoding="utf-8") as handle:
                handle.write("{}")
        if self.behavior == "dirty":
            # Simulates a worker that exits zero but leaves uncommitted files:
            # reports the stale HEAD as candidate. The engine must re-inspect
            # and reject it — never trust adapter prose.
            import os

            dest = os.path.join(worktree, "src/a/dirty.rs")
            os.makedirs(os.path.dirname(dest), exist_ok=True)
            with open(dest, "w", encoding="utf-8") as handle:
                handle.write("dirty\n")
            return WorkerResult(status="SUCCESS", candidate_sha=git_head(worktree),
                                changed_paths=git_dirty_paths(worktree),
                                detail="fake zero-exit with dirty worktree")
        if self.commit:
            subprocess.run(["git", "add", "-A"], cwd=worktree, check=True, timeout=30)
            subprocess.run(["git", "-c", "user.email=t@t", "-c", "user.name=t",
                            "commit", "-qm", f"fake {packet['task_id']}"],
                           cwd=worktree, check=True, timeout=30)
            return classify_worker_output(packet, before, worktree)
        return WorkerResult(status="PROCESS_DIED",
                            detail="fake worker exited zero without committing",
                            changed_paths=git_dirty_paths(worktree))


class OpenCodeWorkerAdapter(WorkerAdapter):
    """Real worker via `opencode run --agent <agent> --model <id> [--variant v] --dir <wt>`.

    `--variant` is passed ONLY when the model entry declares support;
    otherwise provider default applies and reasoning_effective records that.
    """

    name = "opencode-run"

    def __init__(self, opencode_bin: str = "opencode") -> None:
        self.opencode_bin = opencode_bin

    @staticmethod
    def reasoning_plan(policy_entry: dict, requested: str) -> tuple[str | None, str]:
        """Return (variant_flag_or_None, reasoning_effective)."""
        supported = policy_entry.get("variants") or []
        want = EFFORT_VARIANT.get(requested)
        if want and want in supported:
            return want, requested
        return None, "DEFAULT_PROVIDER"

    def build_command(self, packet: dict, model: str, policy_entry: dict,
                      requested_effort: str, variant: str | None) -> list[str]:
        import json

        message = (
            "You are an implementation worker. Follow ONLY this task packet; "
            "do not invent scope, do not advance checkpoints, do not spawn workers. "
            "Commit your result on the packet branch; a dirty worktree is rejected.\n"
            f"TASK_PACKET={json.dumps(packet, sort_keys=True)}"
        )
        command = [self.opencode_bin, "run", "--agent", self.agent,
                   "--model", model, "--dir", packet["worktree"], "--format", "json"]
        if variant:
            command += ["--variant", variant]
        return command + [message]

    def launch(self, packet: dict, timeout_s: int, *, model: str,
               policy_entry: dict | None = None, effort: str = "DEFAULT") -> WorkerResult:
        """The engine-passed `model` is authoritative; packet['model'] is only
        requested/preferred audit metadata (see packets.MODEL_FIELD_SEMANTICS)."""
        policy_entry = policy_entry or {}
        before = git_head(packet["worktree"])
        variant, effective = self.reasoning_plan(policy_entry, effort)
        try:
            proc = subprocess.run(
                self.build_command(packet, model, policy_entry, effort, variant),
                capture_output=True, text=True, timeout=timeout_s,
            )
        except subprocess.TimeoutExpired:
            return WorkerResult(status="INTERRUPTED", detail="worker exceeded timeout",
                                changed_paths=git_dirty_paths(packet["worktree"]),
                                report={"reasoning_effective": effective})
        except OSError as error:
            return WorkerResult(status="TOOL_ERROR", detail=str(error))
        if proc.returncode != 0:
            dirty = []
            with _suppress():
                dirty = git_dirty_paths(packet["worktree"])
            return WorkerResult(status="INTERRUPTED", detail=proc.stderr[-2000:], changed_paths=dirty,
                                report={"reasoning_effective": effective})
        result = classify_worker_output(packet, before, packet["worktree"])
        result.report["reasoning_effective"] = effective
        return result


class _suppress:
    def __enter__(self):
        return self

    def __exit__(self, *exc):
        return True


def detect_interruption(run: dict, repo_worktree: str) -> bool:
    """A RUNNING run whose worktree is dirty => INTERRUPTED (preserved, not reset)."""
    if run.get("status") != "RUNNING":
        return False
    return bool(git_dirty_paths(repo_worktree))


def current_time() -> str:
    return time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())
