#!/usr/bin/env python3
"""Static skill/catalog regression contracts; not a model-routing evaluation."""
import json
from pathlib import Path
import re
import unittest

ROOT = Path(__file__).resolve().parents[1]


class SkillContracts(unittest.TestCase):
    def test_narrow_project_contracts_and_references(self):
        for path in sorted((ROOT / ".agents/skills").glob("*/SKILL.md")):
            text = path.read_text()
            with self.subTest(skill=path.parent.name):
                self.assertTrue(text.startswith("---\nname: " + path.parent.name + "\n"))
                description = re.search(r"^description: (.+)$", text, re.M).group(1)
                self.assertIn("Exclude", description)
                self.assertIn("Stop", text)
                self.assertIn("Report", text)
                self.assertLessEqual(len(text.splitlines()), 55)
                self.assertNotIn("Phase 0 of", text)
                for target in re.findall(r"\]\(([^)#]+)(?:#[^)]*)?\)", text):
                    self.assertTrue((path.parent / target).exists(), target)

    def test_reviewed_cases_cover_neighbors_and_no_skill(self):
        payload = json.loads((ROOT / "scripts/skill_routing_cases.json").read_text())
        cases = payload["cases"]
        self.assertEqual(len(cases), len({case["id"] for case in cases}))
        required = {"ui-trim": "or-ui-ux", "rust-trim": "or-edit-feature",
                    "media-compatibility": "or-media-ai", "performance": "verify-debug",
                    "release-verification": "or-release", "prose": None,
                    "plan-only": "task-planning", "status": "project-steward"}
        actual = {case["id"]: case["primary"] for case in cases}
        for case, expected in required.items():
            self.assertEqual(actual[case], expected)
        for case in cases:
            self.assertNotIn(case["primary"], case["exclude"])
            self.assertTrue(case["request"] and case["why"])
        self.assertIn("Not live", payload["limits"])


if __name__ == "__main__":
    unittest.main()
