#!/usr/bin/env python3
"""Regression checks for the claim-evidence validator.

The validator is the only thing standing between the repository and a fabricated
receipt, so these tests feed it both good and bad records and require it to
reject the bad ones. Run directly or from scripts/check-repo.sh:

    python3 scripts/test_check_claims.py
"""
import contextlib
import importlib.util
import io
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPT = Path(__file__).with_name("check_claims.py")
spec = importlib.util.spec_from_file_location("check_claims", SCRIPT)
check_claims = importlib.util.module_from_spec(spec)
spec.loader.exec_module(check_claims)

ROOT = SCRIPT.resolve().parents[1]
HEAD = subprocess.run(["git", "-C", str(ROOT), "rev-parse", "HEAD"],
                      capture_output=True, text=True, check=True).stdout.strip()


def non_ancestor_commit() -> str | None:
    """A commit that exists in this repository but is not reachable from HEAD."""
    result = subprocess.run(
        ["git", "-C", str(ROOT), "rev-list", "--all", "--not", "HEAD"],
        capture_output=True, text=True, check=True)
    commits = result.stdout.split()
    return commits[0] if commits else None


def record(**overrides) -> dict:
    data = {
        "requirement": "B-large-library-media-previews",
        "claim": "A claim used only by the validator regression tests.",
        "implementation_sha": HEAD,
        "verification": {
            "workflow": "Platform verification",
            "run_id": "1",
            "jobs": ["Flutter static and widget checks"],
        },
        "verified_at_utc": "2026-10-17T08:29:29Z",
        "evidence_class": "hosted",
        "limitations": ["This fixture proves the validator only."],
    }
    data.update(overrides)
    return data


@contextlib.contextmanager
def claims_directory(data: dict):
    with tempfile.TemporaryDirectory(prefix="or-claims-test-") as temporary:
        directory = Path(temporary)
        (directory / "2026-10-17-T-fixture.json").write_text(json.dumps(data))
        yield directory


def run_validator(directory: Path) -> tuple[int, str]:
    out, err = io.StringIO(), io.StringIO()
    with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
        code = check_claims.main(["--claims-dir", str(directory), "--repo", str(ROOT)])
    return code, out.getvalue() + err.getvalue()


class ClaimValidatorTests(unittest.TestCase):
    def test_well_formed_record_with_a_real_ancestor_sha_passes(self):
        with claims_directory(record()) as directory:
            code, output = run_validator(directory)
        self.assertEqual(code, 0, output)
        self.assertIn("PASS", output)

    def test_fabricated_sha_is_rejected(self):
        fabricated = "0" * 40
        with claims_directory(record(implementation_sha=fabricated)) as directory:
            code, output = run_validator(directory)
        self.assertEqual(code, 1, output)
        self.assertIn("does not exist in this repository", output)
        self.assertIn(fabricated, output)

    def test_sha_outside_head_history_is_rejected(self):
        commit = non_ancestor_commit()
        if commit is None:
            self.skipTest("no commit exists outside HEAD history in this clone")
        with claims_directory(record(implementation_sha=commit)) as directory:
            code, output = run_validator(directory)
        self.assertEqual(code, 1, output)
        self.assertIn("not an ancestor of HEAD", output)

    def test_missing_hosted_identity_is_rejected(self):
        for verification in (
            {},
            {"run_id": "1", "jobs": ["job"]},
            {"workflow": "Platform verification", "jobs": ["job"]},
            {"workflow": "Platform verification", "run_id": "1"},
            {"workflow": "Platform verification", "run_id": "1", "jobs": []},
        ):
            with self.subTest(verification=verification):
                with claims_directory(record(verification=verification)) as directory:
                    code, output = run_validator(directory)
                self.assertEqual(code, 1, output)
                self.assertIn("verification", output)

    def test_short_or_malformed_sha_is_rejected(self):
        with claims_directory(record(implementation_sha="abc123")) as directory:
            code, output = run_validator(directory)
        self.assertEqual(code, 1, output)
        self.assertIn("40-char SHA", output)

    def test_a_non_repository_cannot_silently_pass(self):
        with tempfile.TemporaryDirectory(prefix="or-claims-nogit-") as temporary:
            directory = Path(temporary) / "claims"
            directory.mkdir()
            (directory / "2026-10-17-T-fixture.json").write_text(json.dumps(record()))
            out, err = io.StringIO(), io.StringIO()
            with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
                code = check_claims.main([
                    "--claims-dir", str(directory),
                    "--repo", str(Path(temporary) / "not-a-repo"),
                ])
        self.assertEqual(code, 1)
        self.assertIn("cannot verify implementation SHAs", out.getvalue() + err.getvalue())


if __name__ == "__main__":
    sys.exit(0 if unittest.main(exit=False).result.wasSuccessful() else 1)
