#!/usr/bin/env python3
"""Check the licence stager ships x264 only when the build linked it."""

from __future__ import annotations

import subprocess
import tempfile
import unittest
from pathlib import Path

SCRIPT = Path(__file__).resolve().parent / "stage_codec_licences.sh"


def run(*arguments: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        ["bash", str(SCRIPT), *arguments],
        capture_output=True,
        text=True,
        check=False,
    )


class StageCodecLicencesTest(unittest.TestCase):
    def setUp(self) -> None:
        self.directory = tempfile.TemporaryDirectory(prefix="or-licences-")
        self.root = Path(self.directory.name)
        self.target = self.root / "bundle"
        self.target.mkdir()
        self.notices = self.root / "notices"
        self.notices.mkdir()
        self.ffmpeg_copying = self.root / "FFmpeg-COPYING.LGPLv2.1"
        self.ffmpeg_copying.write_text("GNU LESSER GENERAL PUBLIC LICENSE\n")
        (self.notices / "OPUS-COPYING").write_text("opus licence\n")
        (self.notices / "LIBVPX-LICENSE").write_text("libvpx licence\n")
        (self.notices / "LIBVPX-PATENTS").write_text("libvpx patent notice\n")

    def tearDown(self) -> None:
        self.directory.cleanup()

    def stage(self) -> subprocess.CompletedProcess[str]:
        return run(
            "stage",
            str(self.target),
            str(self.ffmpeg_copying),
            str(self.notices),
        )

    def test_a_package_without_x264_is_complete(self) -> None:
        result = self.stage()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertFalse((self.target / "licenses/x264").exists())
        self.assertEqual(run("verify", str(self.target)).returncode, 0)

    def test_a_package_that_links_x264_ships_its_licence(self) -> None:
        (self.notices / "X264-COPYING").write_text("GNU GENERAL PUBLIC LICENSE\n")
        result = self.stage()
        self.assertEqual(result.returncode, 0, result.stderr)
        copying = self.target / "licenses/x264/COPYING"
        self.assertTrue(copying.is_file())
        self.assertIn("GENERAL PUBLIC LICENSE", copying.read_text())
        notice = (self.target / "licenses/FFmpeg/FFmpeg-NOTICE.txt").read_text()
        self.assertIn("licenses/x264", notice)
        self.assertEqual(run("verify", str(self.target)).returncode, 0)

    def test_an_empty_x264_directory_is_rejected(self) -> None:
        (self.notices / "X264-COPYING").write_text("GNU GENERAL PUBLIC LICENSE\n")
        self.assertEqual(self.stage().returncode, 0)
        (self.target / "licenses/x264/COPYING").write_text("")
        result = run("verify", str(self.target))
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("licenses/x264/COPYING", result.stderr)


if __name__ == "__main__":
    unittest.main()
