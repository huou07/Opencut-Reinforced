"""Codex/Sol escalation adapter (Layer C) — optional, quota-aware.

Uses only documented `codex exec` options; model and config are parameters,
never hardcoded undocumented flags. Quota exhaustion => ESCALATION_DEFERRED_QUOTA
with the packet preserved — never a test failure, never a probe loop.
"""
from __future__ import annotations

import json
import subprocess
from dataclasses import dataclass, field

QUOTA_MARKERS = (
    "usage limit",
    "limit reset",
    "quota exceeded",
    "rate limit",
    "try again later",
    "plan limit",
)
AUTH_MARKERS = ("not logged in", "authentication", "login required", "unauthorized")


@dataclass
class EscalationResult:
    state: str  # RESOLVED | ESCALATION_DEFERRED_QUOTA | UNAVAILABLE | AUTH_REQUIRED | TOOL_ERROR
    decision: dict = field(default_factory=dict)
    detail: str = ""


def classify_codex_output(returncode: int, output: str) -> str:
    lowered = output.lower()
    if any(marker in lowered for marker in QUOTA_MARKERS):
        return "ESCALATION_DEFERRED_QUOTA"
    if any(marker in lowered for marker in AUTH_MARKERS):
        return "AUTH_REQUIRED"
    if returncode != 0:
        return "TOOL_ERROR"
    return "RESOLVED"


class CodexAdapter:
    """Real adapter. `probe_only` checks availability without spending quota."""

    def __init__(self, codex_bin: str = "codex", model: str | None = None) -> None:
        self.codex_bin = codex_bin
        self.model = model

    def build_command(self, packet: dict) -> list[str]:
        prompt = (
            "Read the following escalation packet. Reason and return a JSON "
            "decision/contract only, then exit. Do not wait on builds.\n"
            f"ESCALATION_PACKET={json.dumps(packet, sort_keys=True)}"
        )
        command = [self.codex_bin, "exec"]
        if self.model:
            command += ["-m", self.model]
        command += ["-"]
        _ = prompt
        return command

    def prompt_text(self, packet: dict) -> str:
        return (
            "Read the following escalation packet. Reason and return a JSON "
            "decision/contract only, then exit. Do not wait on builds.\n"
            f"ESCALATION_PACKET={json.dumps(packet, sort_keys=True)}"
        )

    def escalate(self, packet: dict, timeout_s: int = 300) -> EscalationResult:
        try:
            proc = subprocess.run(
                self.build_command(packet),
                input=self.prompt_text(packet),
                capture_output=True, text=True, timeout=timeout_s,
            )
        except FileNotFoundError:
            return EscalationResult(state="UNAVAILABLE", detail="codex CLI not installed")
        except subprocess.TimeoutExpired:
            return EscalationResult(state="TOOL_ERROR", detail="codex exec timed out")
        except OSError as error:
            return EscalationResult(state="TOOL_ERROR", detail=str(error))
        combined = (proc.stdout or "") + (proc.stderr or "")
        state = classify_codex_output(proc.returncode, combined)
        if state == "RESOLVED":
            return EscalationResult(state=state, decision={"raw": proc.stdout[-4000:]})
        return EscalationResult(state=state, detail=combined[-2000:])


class FakeCodexAdapter(CodexAdapter):
    """Deterministic fake: scripted transcript, no quota spent."""

    def __init__(self, transcript: str = "", returncode: int = 0) -> None:
        super().__init__(codex_bin="fake-codex")
        self.transcript = transcript
        self.returncode = returncode

    def escalate(self, packet: dict, timeout_s: int = 300) -> EscalationResult:
        _ = (packet, timeout_s)
        state = classify_codex_output(self.returncode, self.transcript)
        if state == "RESOLVED":
            return EscalationResult(state=state, decision={"fake": True})
        return EscalationResult(state=state, detail=self.transcript)
