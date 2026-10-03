"""Worker launch and result parsing (Layer B).

Topology: orchestrator -> one worker -> stop. Workers never spawn workers.
Adapters share an interface so tests use fakes without spending model quota.
Exit classification: SUCCESS | PROCESS_DIED | NO_CHANGE | TIMEOUT | TOOL_ERROR.
"""
from __future__ import annotations

import subprocess
import time
from dataclasses import dataclass, field


@dataclass
class WorkerResult:
    status: str  # SUCCESS | PROCESS_DIED | NO_CHANGE | TIMEOUT | TOOL_ERROR
    candidate_sha: str | None = None
    changed_paths: list[str] = field(default_factory=list)
    report: dict = field(default_factory=dict)
    detail: str = ""


def git_head(repo: str) -> str:
    return subprocess.run(
        ["git", "rev-parse", "HEAD"], cwd=repo,
        capture_output=True, text=True, check=True, timeout=30,
    ).stdout.strip()


def git_dirty_paths(repo: str) -> list[str]:
    out = subprocess.run(
        ["git", "status", "--short"], cwd=repo,
        capture_output=True, text=True, check=True, timeout=30,
    ).stdout
    return sorted(line[3:].strip() for line in out.splitlines() if line.strip())


class WorkerAdapter:
    name = "base"

    def launch(self, packet: dict, timeout_s: int) -> WorkerResult:
        raise NotImplementedError


class FakeWorkerAdapter(WorkerAdapter):
    """Deterministic fake for tests. Behavior scripted, no models involved."""

    name = "fake"

    def __init__(self, behavior: str = "success", files: dict[str, str] | None = None) -> None:
        self.behavior = behavior  # success | die | no-change | protected-edit
        self.files = files or {}

    def launch(self, packet: dict, timeout_s: int) -> WorkerResult:
        _ = timeout_s
        worktree = packet["worktree"]
        if self.behavior == "die":
            return WorkerResult(status="PROCESS_DIED", detail="fake worker died mid-task")
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
        changed = git_dirty_paths(worktree)
        return WorkerResult(status="SUCCESS", changed_paths=changed, report={"fake": True})


class OpenCodeWorkerAdapter(WorkerAdapter):
    """Real worker via `opencode run --model <id> --dir <worktree>`.

    Sends the task packet as the message with JSON format; never lets the
    model choose scope. Subprocess argv arrays only, with timeout.
    """

    name = "opencode-run"

    def __init__(self, opencode_bin: str = "opencode") -> None:
        self.opencode_bin = opencode_bin

    def build_command(self, packet: dict, model: str) -> list[str]:
        import json

        message = (
            "You are an implementation worker. Follow ONLY this task packet; "
            "do not invent scope, do not advance checkpoints, do not spawn workers.\n"
            f"TASK_PACKET={json.dumps(packet, sort_keys=True)}"
        )
        return [
            self.opencode_bin, "run",
            "--model", model,
            "--dir", packet["worktree"],
            "--format", "json",
            message,
        ]

    def launch(self, packet: dict, timeout_s: int) -> WorkerResult:
        before = git_head(packet["worktree"])
        try:
            proc = subprocess.run(
                self.build_command(packet, packet["model"]),
                capture_output=True, text=True, timeout=timeout_s,
            )
        except subprocess.TimeoutExpired:
            return WorkerResult(status="TIMEOUT", detail="worker exceeded timeout")
        except OSError as error:
            return WorkerResult(status="TOOL_ERROR", detail=str(error))
        if proc.returncode != 0:
            return WorkerResult(status="PROCESS_DIED", detail=proc.stderr[-2000:])
        try:
            after = git_head(packet["worktree"])
        except subprocess.SubprocessError:
            return WorkerResult(status="PROCESS_DIED", detail="worktree unreadable")
        if after == before and not git_dirty_paths(packet["worktree"]):
            return WorkerResult(status="NO_CHANGE", detail="worker produced no commit")
        return WorkerResult(
            status="SUCCESS",
            candidate_sha=after,
            changed_paths=git_dirty_paths(packet["worktree"]),
            report={"exit": proc.returncode},
        )


def detect_interruption(run: dict, repo_worktree: str) -> bool:
    """A RUNNING run whose worker is gone and worktree is dirty => INTERRUPTED."""
    if run.get("status") != "RUNNING":
        return False
    return bool(git_dirty_paths(repo_worktree))


def current_time() -> str:
    return time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())
