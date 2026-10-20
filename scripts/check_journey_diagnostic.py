#!/usr/bin/env python3
"""Exercise the existing real UI harness and audit its diagnostic outcome.

Infrastructure PASS does not turn the recorded product journey FAIL into PASS.
A future product repair can pass only by completing the canonical assertions.
"""
import json
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
SCENARIO = ROOT / "scripts/ui_scenarios/split-at-playhead-diagnostic.json"
REPORT = ROOT / "apps/or_app/build/ui-harness/split-at-playhead-diagnostic"


def main():
    completed = subprocess.run([sys.executable, str(ROOT / "scripts/or_ui.py"),
        "--report-dir", str(REPORT), "run", str(SCENARIO)], cwd=ROOT, timeout=960)
    path = REPORT / "report.json"
    if not path.is_file():
        raise SystemExit("INFRASTRUCTURE_CHECK=FAIL: missing diagnostic report")
    report = json.loads(path.read_text())
    steps = report.get("steps", [])
    exact_failure = (
        completed.returncode == 1 and report.get("result") == "FAIL" and steps
        and steps[-1].get("op") == "expect_absent"
        and steps[-1].get("status") == "still_present"
        and steps[-1].get("selector", {}).get("key") == "timeline-split-time"
        and all(step.get("status") == "ok" for step in steps[:-1])
        and any(step.get("op") == "canonical" and
                step.get("observation", {}).get("assertions_passed") is True
                for step in steps))
    complete_success = (completed.returncode == 0 and report.get("result") == "PASS"
                        and report.get("complete") is True
                        and steps[-1].get("observation", {}).get("clip_count") == 2)
    print(f"PRODUCT_JOURNEY={report.get('result')} — {path}")
    if not (exact_failure or complete_success):
        print("INFRASTRUCTURE_CHECK=FAIL: unexpected failure, block or incomplete observation")
        return 1
    print("INFRASTRUCTURE_CHECK=PASS: real UI result classified; product verdict retained")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
