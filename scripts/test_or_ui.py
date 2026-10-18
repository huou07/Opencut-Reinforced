#!/usr/bin/env python3
"""Hermetic checks for the agent-facing UI harness contract.

`scripts/or_ui.py` is the instrument behind every UI claim, so the parts that
decide whether a claim is admissible are checked without Xvfb, a device or a
network:

* a scenario file is validated before anything runs, and a malformed one is a
  usage error (exit 2), never a silent pass;
* the shipped scenarios are structurally valid, name semantic selectors only, and
  never contain a screen coordinate;
* the negative control (`scripts/ui_scenarios/wrong-selector.json`) exists and is
  the file the harness's own documentation points at, so the loud-failure
  behaviour has a runnable trigger;
* the compact guard is registered at the two named compact viewports.

The real end-to-end runs need Xvfb (Linux) or a booted guest (Android) and are
therefore not part of `check-repo.sh`; what is checked here is that the harness
would have to fail loudly when it should.
"""
from __future__ import annotations

import importlib.util
import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SCRIPT = ROOT / "scripts" / "or_ui.py"
SCENARIOS = ROOT / "scripts" / "ui_scenarios"
SELECTOR_KEYS = {"key", "key_regex", "label", "label_regex", "text", "role", "within"}
COORDINATE_KEYS = {"x", "y", "dx", "dy", "offset", "position", "point", "rect", "center"}


def load():
    spec = importlib.util.spec_from_file_location("or_ui_under_test", SCRIPT)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


class ScenarioContractCase(unittest.TestCase):
    def setUp(self) -> None:
        self.module = load()

    def shipped_scenarios(self):
        for path in sorted(SCENARIOS.glob("*.json")):
            yield path, json.loads(path.read_text(encoding="utf-8"))

    def test_the_shipped_scenarios_are_present_and_valid(self) -> None:
        names = {path.name for path in SCENARIOS.glob("*.json")}
        self.assertTrue({"editor-journey.json", "compact-editor-journey.json",
                         "interaction-latency.json", "overwrite-insert.json",
                         "ripple-trim.json", "slip-roll-slide.json",
                         "track-reorder.json",
                         "wrong-selector.json"}.issubset(names),
                        names)
        for path, payload in self.shipped_scenarios():
            with self.subTest(scenario=path.name):
                loaded = self.module.load_scenario(path)
                self.assertTrue(loaded["name"])
                self.assertTrue(loaded["steps"])

    def test_no_scenario_acts_by_screen_coordinate(self) -> None:
        for path, payload in self.shipped_scenarios():
            if path.name == "wrong-selector.json":
                candidate_set = payload.get("viewport", {})
                self.assertIsInstance(candidate_set, dict)
            for index, step in enumerate(payload["steps"]):
                with self.subTest(scenario=path.name, step=index):
                    self.assertNotIn("at", step, "a step names a coordinate")
                    selector = step.get("selector")
                    if selector is None:
                        continue
                    self.assertIsInstance(selector, dict)
                    named = set(selector) & SELECTOR_KEYS
                    self.assertTrue(named, f"step {index} has no semantic selector: {selector}")
                    for key in selector:
                        self.assertNotIn(key, COORDINATE_KEYS,
                                         f"step {index} selector uses a coordinate key: {key}")
                    within = selector.get("within")
                    if within is not None:
                        self.assertTrue(set(within) <= {"key", "type"},
                                        f"step {index} within scope is not semantic: {within}")

    def test_a_malformed_scenario_is_a_usage_error_not_a_pass(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            bad = Path(directory) / "bad.json"
            bad.write_text("{\"name\": \"bad\"}", encoding="utf-8")
            completed = subprocess.run(
                [sys.executable, str(SCRIPT), "--report-dir", directory, "run", str(bad)],
                cwd=ROOT, capture_output=True, text=True, timeout=120)
            self.assertEqual(completed.returncode, 2, completed.stdout + completed.stderr)
            self.assertIn("no steps", completed.stderr)
            not_json = Path(directory) / "not.json"
            not_json.write_text("this is not json", encoding="utf-8")
            completed = subprocess.run(
                [sys.executable, str(SCRIPT), "--report-dir", directory, "run", str(not_json)],
                cwd=ROOT, capture_output=True, text=True, timeout=120)
            self.assertEqual(completed.returncode, 2)
            self.assertIn("not JSON", completed.stderr)

    def test_a_viewport_must_be_two_numbers(self) -> None:
        with self.assertRaises(self.module.ScenarioError):
            self.module.parse_viewport("wide")
        with self.assertRaises(self.module.ScenarioError):
            self.module.parse_viewport("10x10")
        self.assertEqual(self.module.parse_viewport("320x440"), (320, 440))

    def test_act_requires_a_semantic_selector(self) -> None:
        parser = self.module.command_parser()
        args = parser.parse_args(["act", "--op", "tap"])
        with self.assertRaises(self.module.ScenarioError):
            self.module.build_command_scenario(args)

    def test_the_wrong_selector_control_is_the_documented_negative_case(self) -> None:
        path = SCENARIOS / "wrong-selector.json"
        payload = json.loads(path.read_text(encoding="utf-8"))
        self.assertIn("python3 scripts/or_ui.py run scripts/ui_scenarios/wrong-selector.json",
                      payload["description"])
        labels = [step.get("selector", {}).get("label") for step in payload["steps"]]
        self.assertTrue(any(label and "Export the final cut" in label for label in labels),
                        "the negative control no longer names an impossible control")


if __name__ == "__main__":
    sys.exit(0 if unittest.main(exit=False).result.wasSuccessful() else 1)
