#!/usr/bin/env python3
"""Regression checks for the aggregate release-gate logic.

The release gate is the only check that can block a merge, so it must reject
every non-success outcome, not just `failure`. These tests feed it synthetic
`needs` contexts. Run directly or from scripts/check-repo.sh:

    python3 scripts/test_release_gate.py
"""

import contextlib
import importlib.util
import io
import json
import sys
import unittest
from pathlib import Path


SCRIPT = Path(__file__).with_name("release_gate.py")
spec = importlib.util.spec_from_file_location("release_gate", SCRIPT)
release_gate = importlib.util.module_from_spec(spec)
spec.loader.exec_module(release_gate)

REQUIRED = list(release_gate.REQUIRED_JOBS)


def needs(**results: str) -> dict[str, dict[str, str]]:
    return {job: {"result": result} for job, result in results.items()}


def all_success(**overrides: str) -> dict[str, dict[str, str]]:
    base = {job: "success" for job in REQUIRED}
    base.update(overrides)
    return {job: {"result": result} for job, result in base.items()}


def run(arguments: list[str]) -> tuple[int, str]:
    out = io.StringIO()
    with contextlib.redirect_stdout(out), contextlib.redirect_stderr(out):
        code = release_gate.main(arguments)
    return code, out.getvalue()


class ReleaseGateTests(unittest.TestCase):
    def test_all_required_jobs_succeed(self) -> None:
        code, output = run(["--needs-json", json.dumps(all_success())])
        self.assertEqual(code, 0, output)
        self.assertIn("PASS", output)

    def test_any_failure_fails_the_gate(self) -> None:
        code, output = run(["--needs-json", json.dumps(all_success(**{"flutter-static": "failure"}))])
        self.assertEqual(code, 1, output)
        self.assertIn("'flutter-static' reported 'failure'", output)

    def test_skipped_job_is_not_success(self) -> None:
        code, output = run(["--needs-json", json.dumps(all_success(rust="skipped"))])
        self.assertEqual(code, 1, output)
        self.assertIn("'rust' reported 'skipped'", output)

    def test_neutral_job_is_not_success(self) -> None:
        code, output = run(["--needs-json", json.dumps(all_success(**{"granted-descriptor-capability": "neutral"}))])
        self.assertEqual(code, 1, output)
        self.assertIn("'granted-descriptor-capability' reported 'neutral'", output)

    def test_cancelled_job_is_not_success(self) -> None:
        code, output = run(["--needs-json", json.dumps(all_success(**{"flutter-static": "cancelled"}))])
        self.assertEqual(code, 1, output)
        self.assertIn("'flutter-static' reported 'cancelled'", output)

    def test_missing_job_is_not_success(self) -> None:
        code, output = run(["--needs-json", json.dumps(needs(rust="success"))])
        self.assertEqual(code, 1, output)
        self.assertIn("'granted-descriptor-capability' was not reported (missing)", output)

    def test_unexpected_extra_job_is_ignored(self) -> None:
        context = all_success()
        context["some-future-job"] = {"result": "failure"}
        code, output = run(["--needs-json", json.dumps(context)])
        self.assertEqual(code, 0, output)

    def test_malformed_needs_context_fails_closed(self) -> None:
        code, output = run(["--needs-json", "not json"])
        self.assertEqual(code, 1, output)
        self.assertIn("not valid JSON", output)

    def test_every_desktop_platform_job_is_required(self) -> None:
        # A green gate must mean all three desktop platforms passed, so a job
        # that silently disappears from REQUIRED_JOBS has to fail this test.
        for job in ("macos", "linux", "windows"):
            self.assertIn(job, release_gate.REQUIRED_JOBS)

    def test_each_desktop_platform_failure_fails_the_gate(self) -> None:
        for job in ("macos", "linux", "windows"):
            with self.subTest(job=job):
                code, output = run(
                    ["--needs-json", json.dumps(all_success(**{job: "failure"}))]
                )
                self.assertEqual(code, 1, output)
                self.assertIn(f"'{job}' reported 'failure'", output)

    def test_desktop_platform_skipped_or_cancelled_is_not_success(self) -> None:
        for job in ("macos", "linux", "windows"):
            for outcome in ("skipped", "neutral", "cancelled"):
                with self.subTest(job=job, outcome=outcome):
                    code, output = run(
                        ["--needs-json", json.dumps(all_success(**{job: outcome}))]
                    )
                    self.assertEqual(code, 1, output)
                    self.assertIn(f"'{job}' reported '{outcome}'", output)

    def test_a_pull_request_run_may_defer_the_desktop_jobs(self) -> None:
        postponed = all_success(macos="skipped", linux="skipped", windows="skipped")
        code, output = run(
            ["--needs-json", json.dumps(postponed), "--event", "pull_request"]
        )
        self.assertEqual(code, 0, output)
        self.assertIn("desktop builds deferred to the merge", output)

    def test_a_pull_request_run_still_fails_on_a_desktop_failure(self) -> None:
        failed = all_success(macos="skipped", linux="skipped", windows="failure")
        code, output = run(
            ["--needs-json", json.dumps(failed), "--event", "pull_request"]
        )
        self.assertEqual(code, 1, output)
        self.assertIn("'windows' reported 'failure'", output)

    def test_a_push_run_still_requires_every_desktop_job(self) -> None:
        postponed = all_success(macos="skipped", linux="skipped", windows="skipped")
        code, output = run(["--needs-json", json.dumps(postponed), "--event", "push"])
        self.assertEqual(code, 1, output)
        self.assertIn("'macos' reported 'skipped'", output)

    def test_cli_requires_the_needs_argument(self) -> None:
        with contextlib.redirect_stderr(io.StringIO()):
            with self.assertRaises(SystemExit) as raised:
                release_gate.main([])
        self.assertNotEqual(raised.exception.code, 0)


if __name__ == "__main__":
    unittest.main()
