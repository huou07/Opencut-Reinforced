#!/usr/bin/env python3
"""Negative controls for completion claims, not simulated product acceptance."""
import contextlib
import importlib.util
import io
import json
from pathlib import Path
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]


def load(name):
    spec = importlib.util.spec_from_file_location(name, ROOT / "scripts" / f"{name}.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


ui = load("or_ui")
media = load("media_checks")


class EvidenceTests(unittest.TestCase):
    def finish(self, requested, actual, result="PASS", **extra):
        with tempfile.TemporaryDirectory(dir=ROOT / ".hub") as directory:
            path = Path(directory)
            artifacts = path / "artifacts"
            artifacts.mkdir()
            (artifacts / "control.json").write_text(json.dumps(
                {"scenario": "control", "result": result, "steps": actual} | extra))
            with contextlib.redirect_stdout(io.StringIO()):
                code = ui.finish(path, artifacts,
                                 {"name": "control", "steps": requested}, "linux")
            return code, json.loads((path / "report.json").read_text())

    def test_missing_control_cannot_pass_even_with_success_flag(self):
        code, report = self.finish([{"op": "expect"}],
                                  [{"index": 0, "op": "expect", "status": "not_found"}])
        self.assertEqual(code, 1)
        self.assertEqual(report["result"], "FAIL")
        self.assertEqual(report["evidence_class"]["behaviour"], "UNVERIFIED")

    def test_omitted_reordered_and_empty_transcripts_fail(self):
        for steps in ([], [{"index": 0, "op": "shot", "status": "ok"}],
                      [{"index": 9, "op": "expect", "status": "ok"}]):
            with self.subTest(steps=steps):
                self.assertEqual(self.finish([{"op": "expect"}], steps)[0], 1)

    def test_screenshot_alone_is_not_canonical_behavior(self):
        code, report = self.finish([{"op": "shot"}],
                                  [{"index": 0, "op": "shot", "status": "ok"}])
        self.assertEqual(code, 0)
        self.assertEqual(report["evidence_class"]["behaviour"], "UNVERIFIED")
        self.assertEqual(report["evidence_class"]["visual"], "UNVERIFIED")

    def test_canonical_mismatch_cannot_be_promoted(self):
        self.assertEqual(self.finish(
            [{"op": "canonical", "equals": {"clip_count": 1}}],
            [{"index": 0, "op": "canonical", "status": "ok",
              "observation": {"assertions_passed": True, "clip_count": 0}}])[0], 1)

    def test_real_assertion_contract_can_pass(self):
        code, report = self.finish(
            [{"op": "canonical", "equals": {"clip_count": 1}}],
            [{"index": 0, "op": "canonical", "status": "ok",
              "observation": {"assertions_passed": True, "clip_count": 1}}])
        self.assertEqual(code, 0)
        self.assertEqual(report["evidence_class"]["behaviour"], "VERIFIED_BEHAVIORAL")

    def test_blank_or_frozen_preview_cannot_pass(self):
        for changed, frames, times in (
                (False, ["1", "2", "3"], ["0/1", "1/2", "1/1"]),
                (True, ["1"] * 3, ["0/1"] * 3),
                (True, ["1", "2", "3"], [None] * 3)):
            samples = [{"frame_sequence": f, "presented_time": t, "width": 160,
                        "height": 90} for f, t in zip(frames, times)]
            code, report = self.finish([{"op": "preview_probe"}],
                [{"index": 0, "op": "preview_probe", "status": "ok",
                  "observation": {"viewer_pixels_changed": changed, "samples": samples}}])
            self.assertEqual(code, 1)
            self.assertEqual(report["evidence_class"]["preview"], "UNVERIFIED")

    def test_framework_error_cannot_pass(self):
        self.assertEqual(self.finish([{"op": "pump"}],
            [{"index": 0, "op": "pump", "status": "ok"}],
            framework_errors=["RenderFlex overflow"])[0], 1)

    def test_invalid_file_is_not_a_successful_export(self):
        with tempfile.TemporaryDirectory(dir=ROOT / ".hub") as directory:
            path = Path(directory) / "fake.webm"
            path.write_bytes(b"success notification, not media")
            with self.assertRaises(media.MediaFailure):
                media.verify_export(path, self.expected())

    def test_preset_mismatch_and_long_output_fail(self):
        probe = {"format": {"duration": "2", "format_name": "matroska,webm"},
                 "streams": [{"codec_type": "video", "codec_name": "vp9",
                              "width": 160, "height": 90},
                             {"codec_type": "audio", "codec_name": "opus"}]}
        media.check_metadata(probe, self.expected())
        for change in ({"duration": "20"}, {"duration": "nan"}):
            with self.assertRaises(media.MediaFailure):
                media.check_metadata(probe | {"format": probe["format"] | change},
                                     self.expected())
        with self.assertRaises(media.MediaFailure):
            media.check_metadata(probe | {"streams": probe["streams"][:1]},
                                 self.expected())

    def test_malformed_preview_and_unknown_op_rejected_before_launch(self):
        with tempfile.TemporaryDirectory(dir=ROOT / ".hub") as directory:
            path = Path(directory) / "scenario.json"
            path.write_text(json.dumps({"name": "bad", "steps": [{"op": "pretend"}]}))
            with self.assertRaises(ui.ScenarioError):
                ui.load_scenario(path)

    @staticmethod
    def expected():
        return {"width": 160, "height": 90, "duration": 2, "container": "webm",
                "video_codec": "vp9", "audio_codec": "opus"}


if __name__ == "__main__":
    (ROOT / ".hub").mkdir(exist_ok=True)
    unittest.main()
