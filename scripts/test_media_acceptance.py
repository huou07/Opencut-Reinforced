#!/usr/bin/env python3
"""Bounded real FFmpeg controls. Missing tools exit BLOCKED, never a skipped PASS."""
import importlib.util
import json
from pathlib import Path
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]


def load(name):
    spec = importlib.util.spec_from_file_location(name, ROOT / "scripts" / f"{name}.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def main():
    if not all(shutil.which(tool) for tool in ("ffmpeg", "ffprobe")):
        print("MEDIA_INSTRUMENT=BLOCKED: ffmpeg/ffprobe unavailable")
        return 3
    fixtures, media = load("media_fixtures"), load("media_checks")
    (ROOT / ".hub").mkdir(exist_ok=True)
    with tempfile.TemporaryDirectory(dir=ROOT / ".hub", prefix="media-control-") as temp:
        first, second = Path(temp) / "first", Path(temp) / "second"
        a, b = fixtures.generate(first), fixtures.generate(second)
        assert [(f["name"], f["sha256"]) for f in a["files"]] == [
            (f["name"], f["sha256"]) for f in b["files"]], "nondeterministic fixtures"
        expected = json.loads((ROOT / "scripts/media_presets/smoke-webm.json").read_text())
        positive = media.verify_export(first / "video-audio.webm", expected)
        failures = []
        for name in ("corrupt.mkv", "unsupported.txt"):
            try:
                media.verify_export(first / name, expected)
            except media.MediaFailure:
                failures.append(name)
            else:
                raise AssertionError(f"{name} incorrectly accepted")
        frozen = first / "frozen.webm"
        subprocess.run(["ffmpeg", "-v", "error", "-nostdin", "-f", "lavfi", "-i",
            "color=black:size=160x90:rate=12:duration=2", "-f", "lavfi", "-i",
            "sine=frequency=440:sample_rate=48000:duration=2", "-c:v", "libvpx-vp9",
            "-c:a", "libopus", "-threads", "1", str(frozen)],
            check=True, capture_output=True, timeout=60)
        try:
            media.verify_export(frozen, expected)
        except media.MediaFailure as error:
            assert "frozen" in str(error), error
            failures.append("frozen-video")
        else:
            raise AssertionError("frozen moving-fixture output incorrectly accepted")
        report = {"result": "PASS", "scope": "real media instrument controls; not OR export",
                  "positive": positive, "rejected": failures,
                  "deterministic_files": len(a["files"]),
                  "fixture_manifest": a}
        (ROOT / ".hub/media-instrument.json").write_text(json.dumps(report, indent=2) + "\n")
        print("MEDIA_INSTRUMENT=PASS: deterministic fixtures, decoded positive, corrupt/unsupported/frozen rejected")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
