#!/usr/bin/env python3
"""Validate exact Android process-stop and project-recovery acceptance reports."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


def _read(path: Path) -> dict[str, Any]:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise ValueError(f"{path} must contain a JSON object")
    return value


def verify(relaunch: dict[str, Any], prepared: dict[str, Any], recovered: dict[str, Any]) -> None:
    if relaunch.get("package") != "io.github.huou07.or_app":
        raise ValueError("process restart report names the wrong application")
    if relaunch.get("result") != "PASS" or relaunch.get("emptyAfterForceStop") is not True:
        raise ValueError("Android did not prove that force-stop removed the app process")
    old_pid, new_pid = relaunch.get("oldPid"), relaunch.get("newPid")
    if type(old_pid) is not int or old_pid < 1 or type(new_pid) is not int or new_pid < 1:
        raise ValueError("process restart report must contain positive old and new PIDs")
    if old_pid == new_pid:
        raise ValueError("the relaunched app reused the pre-stop process identity")

    prepared_report = prepared.get("androidSafAcceptance")
    if not isinstance(prepared_report, dict):
        raise ValueError("initial SAF journey report is missing")
    prepared_checks = prepared_report.get("checks")
    if not isinstance(prepared_checks, dict) or prepared_checks.get(
        "recoveryCheckpointPersistedBeforeProcessStop"
    ) is not True:
        raise ValueError("the first process did not persist a recovery checkpoint")
    if prepared_report.get("recoveryName") != "Process recovery acceptance":
        raise ValueError("prepared recovery checkpoint has the wrong project name")
    if prepared_report.get("recoveryKindAfterClose") != "candidate":
        raise ValueError("recovery checkpoint did not remain available after closing")
    base_revision = int(prepared_report["recoveryBaseRevision"])
    recovery_revision = int(prepared_report["recoveryRevision"])
    if recovery_revision <= base_revision:
        raise ValueError("recovery checkpoint revision does not exceed its saved base")

    recovered_report = recovered.get("androidSafRecoveryAcceptance")
    if not isinstance(recovered_report, dict):
        raise ValueError("post-restart SAF recovery report is missing")
    checks = recovered_report.get("checks")
    required_checks = {
        "nativeDocumentsUiReopenedSameSafProject",
        "recoveryCandidateShownAfterProcessRestart",
        "explicitRecoveryApplied",
        "recoverySidecarRemovedAfterApply",
        "previewPlaybackResumedFromRecoveredProject",
        "noFlutterException",
    }
    if not isinstance(checks, dict) or any(checks.get(name) is not True for name in required_checks):
        raise ValueError("post-restart recovery journey is missing a required product assertion")
    if not str(recovered_report.get("projectPath", "")).startswith("/data/user/0/io.github.huou07.or_app/files/or-projects/"):
        raise ValueError("post-restart journey did not reopen the app-private SAF working copy")
    if recovered_report.get("projectName") != "Process recovery acceptance":
        raise ValueError("reopened project does not contain the recovered unsaved edit")
    if recovered_report.get("recoveryKindAfterApply") != "none":
        raise ValueError("recovery sidecar remained after explicit apply")
    if int(recovered_report.get("projectRevision", "0")) < recovery_revision:
        raise ValueError("recovered project revision is older than the checkpoint")
    if int(recovered_report.get("frameSequence", "0")) < 1:
        raise ValueError("preview did not present a frame from the recovered project")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--process", required=True, type=Path)
    parser.add_argument("--prepared", required=True, type=Path)
    parser.add_argument("--recovered", required=True, type=Path)
    args = parser.parse_args()
    verify(_read(args.process), _read(args.prepared), _read(args.recovered))
    print("Android SAF process relaunch, recovery, and playback evidence: PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
