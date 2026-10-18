#!/usr/bin/env python3
"""Hermetic checks for the local performance budget.

Two properties decide whether the budget means anything, and neither needs a
compiler, a GPU or a network to check:

1. every enforced row declares where its threshold came from (a dated baseline)
   and which kind of threshold it uses, so a number can never appear without a
   provenance or a row can never be silently unenforced;
2. a breach is reported as FAIL and returns exit 1, while an in-budget number is
   PASS -- including through the documented `OR_BENCH_THRESHOLD_SCALE` seam, which
   tightens every budget so the enforcement path of each row kind is reachable
   without shipping a regression.

The real injected-regression proof is a separate, documented command
(`OR_BENCH_INJECT_SLEEP_MS=500 OR_BENCH_ONLY=cli-cold-start python3
scripts/bench_quality.py` must exit 1); it is run at phase boundaries because it
needs the built CLI, and this suite must stay runnable inside `check-repo.sh`.
"""
from __future__ import annotations

import importlib.util
import os
import subprocess
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SCRIPT = ROOT / "scripts" / "bench_quality.py"
THRESHOLD_KINDS = ("threshold_ms", "threshold_p95_ms", "threshold_fps",
                   "threshold_ratio", "threshold_loaded_p95_ms")


def load(scoped: bool = True):
    """Import the budget script as a module.

    `threshold_scale` reads the environment at call time, so one import is
    enough for both the scaled and unscaled cases.
    """
    spec = importlib.util.spec_from_file_location("or_bench_quality_under_test", SCRIPT)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


class BudgetTableCase(unittest.TestCase):
    def setUp(self) -> None:
        self.module = load()
        self.rows = self.module.ROWS

    def test_every_row_declares_a_dated_baseline(self) -> None:
        for row_id, row in self.rows.items():
            with self.subTest(row=row_id):
                self.assertIn("description", row)
                baseline = row.get("baseline")
                self.assertIsInstance(baseline, dict, f"{row_id} has no baseline")
                self.assertRegex(baseline.get("date", ""), r"^\d{4}-\d{2}-\d{2}$",
                                 f"{row_id} has no dated baseline")

    def test_every_row_declares_the_kind_of_threshold_it_enforces(self) -> None:
        for row_id, row in self.rows.items():
            with self.subTest(row=row_id):
                declared = [kind for kind in THRESHOLD_KINDS if kind in row]
                self.assertTrue(declared, f"{row_id} declares no threshold")
                for kind in declared:
                    self.assertGreater(float(row[kind]), 0.0,
                                       f"{row_id}.{kind} must be a positive bound")

    def test_the_named_rows_the_product_requires_are_all_enforced(self) -> None:
        required = {
            "cli-cold-start", "ffmpeg-export", "export-throughput",
            "command.create", "command.import", "command.add_clip", "command.trim",
            "command.split", "command.undo", "command.seek_summary",
            "project-open", "project-save", "render-frame-time", "queue-priority",
        }
        self.assertTrue(required.issubset(set(self.rows)),
                        f"missing budget rows: {sorted(required - set(self.rows))}")


class EnforcementCase(unittest.TestCase):
    def setUp(self) -> None:
        os.environ.pop("OR_BENCH_THRESHOLD_SCALE", None)
        self.module = load()

    def tearDown(self) -> None:
        os.environ.pop("OR_BENCH_THRESHOLD_SCALE", None)

    def test_an_in_budget_throughput_passes_and_a_slow_one_fails(self) -> None:
        self.assertEqual(self.module.row_export_throughput(14.4).verdict, "PASS")
        slow = self.module.row_export_throughput(10_000_000.0)
        self.assertEqual(slow.verdict, "FAIL")
        self.assertIn("frames/s", slow.detail)

    def test_a_missing_export_time_is_unverified_never_a_pass(self) -> None:
        result = self.module.row_export_throughput(None)
        self.assertEqual(result.verdict, "UNVERIFIED")
        self.assertIsNotNone(result.reason)

    def test_the_threshold_scale_tightens_ceilings_and_floors(self) -> None:
        # Ceilings shrink, floors grow: either way the budget gets stricter, so
        # the same measured number must not stay green.
        os.environ["OR_BENCH_THRESHOLD_SCALE"] = "0.001"
        self.assertAlmostEqual(self.module.budget(200.0), 0.2)
        self.assertAlmostEqual(self.module.budget_floor(60.0), 60_000.0)
        self.assertEqual(self.module.row_export_throughput(14.4).verdict, "FAIL")

    def test_a_scale_that_would_loosen_a_budget_is_ignored(self) -> None:
        os.environ["OR_BENCH_THRESHOLD_SCALE"] = "10"
        self.assertEqual(self.module.threshold_scale(), 1.0)
        os.environ["OR_BENCH_THRESHOLD_SCALE"] = "not-a-number"
        self.assertEqual(self.module.threshold_scale(), 1.0)

    def test_an_empty_row_selection_is_refused(self) -> None:
        environment = dict(os.environ, OR_BENCH_ONLY="not-a-row")
        completed = subprocess.run(
            [sys.executable, str(SCRIPT), "--json"], cwd=ROOT, env=environment,
            capture_output=True, text=True, timeout=120)
        self.assertEqual(completed.returncode, 2, completed.stdout + completed.stderr)
        self.assertIn("no budget row was selected", completed.stdout)


if __name__ == "__main__":
    sys.exit(0 if unittest.main(exit=False).result.wasSuccessful() else 1)
