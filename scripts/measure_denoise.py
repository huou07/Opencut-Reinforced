#!/usr/bin/env python3
"""Measure a packaged FFmpeg helper's noise reduction on a synthetic fixture.

Phase 4 needs a number, not a claim: "the helper can denoise" is only true if the
noise floor actually drops while the signal survives. This builds a fixture whose
two windows are known -- one second of noise alone, then two seconds of a tone
buried in the same noise -- runs the FFmpeg noise-reduction filter over it, and
reports the noise drop and the signal change as `volumedetect` measurements of each
window.

It also demonstrates the part a product cannot skip: `afftdn`'s noise floor is a
parameter, and a hard-coded one either does nothing (too low) or eats the signal
(too high). The script measures the fixture's own noise floor and passes it to the
filter, which is what a real audio path has to do per clip.

Exit codes: 0 the filter reduced the noise floor by at least the required amount
without dropping the signal more than allowed; 1 it did not; 2 the invocation or
the helper is wrong.
"""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
import tempfile
from pathlib import Path

# The fixture is a tone at this fraction of full scale in white noise at this
# level, which puts the noise floor well above a real recording's and makes a
# weak filter obvious rather than marginal.
TONE_VOLUME = 0.5
NOISE_AMPLITUDE = 0.12
NOISE_SECONDS = 1
TONE_SECONDS = 2
NOISE_WINDOW = (0.2, 0.6)
TONE_WINDOW = (1.2, 1.8)

# What "it works" means. Denoising is discrimination, not suppression: a filter that
# lowered everything by 20 dB would "reduce noise" and destroy the audio, so the
# check requires the noise to fall by a real amount *and* to fall at least this much
# further than the signal. The fixture's signal is a pure tone, which is the hardest
# case for an FFT denoiser -- a stationary narrowband signal looks like noise to it --
# so an absolute cap on the signal loss would measure the fixture, not the filter.
REQUIRED_NOISE_DROP_DB = 6.0
REQUIRED_DISCRIMINATION_DB = 3.0


def run(command: list[str], label: str) -> str:
    done = subprocess.run(command, capture_output=True, text=True)
    if done.returncode != 0:
        raise SystemExit(f"{label} failed:\n{done.stderr[-1200:]}")
    return done.stderr


def filter_listing(ffmpeg: Path) -> str:
    """`-filters` prints to stdout; the measurements print to stderr."""
    done = subprocess.run(
        [str(ffmpeg), "-hide_banner", "-filters"], capture_output=True, text=True
    )
    if done.returncode != 0:
        raise SystemExit(f"filter discovery failed:\n{done.stderr[-800:]}")
    return done.stdout


def mean_volume_db(ffmpeg: Path, path: Path, window: tuple[float, float]) -> float:
    start, duration = window
    stderr = run(
        [
            str(ffmpeg), "-hide_banner", "-nostats", "-ss", str(start), "-t", str(duration),
            "-i", str(path), "-map", "0:a", "-af", "volumedetect", "-f", "null", "-",
        ],
        "volumedetect",
    )
    match = re.search(r"mean_volume: (-?[0-9.]+) dB", stderr)
    if match is None:
        raise SystemExit(f"volumedetect reported no mean volume:\n{stderr[-600:]}")
    return float(match.group(1))


def build_fixture(ffmpeg: Path, work: Path) -> Path:
    noise = work / "noise.wav"
    run([
        str(ffmpeg), "-hide_banner", "-loglevel", "error", "-y",
        "-f", "lavfi", "-i",
        f"anoisesrc=d={NOISE_SECONDS}:c=white:a={NOISE_AMPLITUDE}:r=48000",
        "-ac", "1", str(noise),
    ], "noise generation")
    tone = work / "tone.wav"
    run([
        str(ffmpeg), "-hide_banner", "-loglevel", "error", "-y",
        "-f", "lavfi", "-i", f"sine=f=1000:d={TONE_SECONDS}:r=48000",
        "-af", f"volume={TONE_VOLUME}", "-ac", "1", str(tone),
    ], "tone generation")
    speech = work / "speech.wav"
    run([
        str(ffmpeg), "-hide_banner", "-loglevel", "error", "-y", "-i", str(tone),
        "-f", "lavfi", "-i",
        f"anoisesrc=d={TONE_SECONDS}:c=white:a={NOISE_AMPLITUDE}:r=48000",
        "-filter_complex", "[0:a][1:a]amix=inputs=2:duration=first:normalize=0",
        "-ac", "1", str(speech),
    ], "speech generation")
    fixture = work / "fixture.wav"
    run([
        str(ffmpeg), "-hide_banner", "-loglevel", "error", "-y",
        "-i", str(noise), "-i", str(speech),
        "-filter_complex", "[0:a][1:a]concat=n=2:v=0:a=1", "-ac", "1", str(fixture),
    ], "fixture assembly")
    return fixture


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--bin", required=True, type=Path, help="directory holding the FFmpeg helper")
    parser.add_argument("--work", type=Path, help="scratch directory (a temporary one by default)")
    args = parser.parse_args()

    suffix = ".exe" if sys.platform.startswith("win") else ""
    ffmpeg = args.bin / f"ffmpeg{suffix}"
    if not ffmpeg.is_file():
        print(f"the FFmpeg helper is missing from {args.bin}", file=sys.stderr)
        return 2

    filters = filter_listing(ffmpeg)
    if re.search(r"\bafftdn\b", filters) is None:
        print("the packaged helper was built without the afftdn noise filter", file=sys.stderr)
        return 1

    work = args.work or Path(tempfile.mkdtemp(prefix="or-denoise-"))
    work.mkdir(parents=True, exist_ok=True)
    fixture = build_fixture(ffmpeg, work)

    noise_before = mean_volume_db(ffmpeg, fixture, NOISE_WINDOW)
    signal_before = mean_volume_db(ffmpeg, fixture, TONE_WINDOW)
    # The floor the filter is told about comes from the material itself, which is
    # the part a product cannot hard-code: a floor below the real one does nothing
    # and a floor above it eats the signal.
    noise_floor = round(noise_before) + 2
    chain = f"afftdn=nf={noise_floor}:nr=12"

    denoised = work / "denoised.wav"
    run([
        str(ffmpeg), "-hide_banner", "-loglevel", "error", "-y", "-i", str(fixture),
        "-af", chain, "-ac", "1", str(denoised),
    ], "denoise")

    noise_after = mean_volume_db(ffmpeg, denoised, NOISE_WINDOW)
    signal_after = mean_volume_db(ffmpeg, denoised, TONE_WINDOW)
    noise_drop = noise_before - noise_after
    signal_drop = signal_before - signal_after

    print(f"fixture        : {fixture}")
    print(f"chain          : {chain} (noise floor measured at {noise_before:.1f} dB)")
    print(f"noise window   : {noise_before:.2f} dB -> {noise_after:.2f} dB (drop {noise_drop:.2f} dB)")
    print(f"signal window  : {signal_before:.2f} dB -> {signal_after:.2f} dB (drop {signal_drop:.2f} dB)")
    print(f"discrimination : {noise_drop - signal_drop:.2f} dB more noise than signal")

    if noise_drop < REQUIRED_NOISE_DROP_DB:
        print(
            f"the noise floor only fell {noise_drop:.2f} dB, below the "
            f"{REQUIRED_NOISE_DROP_DB:.1f} dB this check requires",
            file=sys.stderr,
        )
        return 1
    discrimination = noise_drop - signal_drop
    if discrimination < REQUIRED_DISCRIMINATION_DB:
        print(
            f"the noise fell only {discrimination:.2f} dB more than the signal, below "
            f"the {REQUIRED_DISCRIMINATION_DB:.1f} dB of discrimination this check "
            "requires -- the filter is attenuating everything, not denoising",
            file=sys.stderr,
        )
        return 1
    print("RESULT: DENOISE_MEASURED=PASS")
    return 0


if __name__ == "__main__":
    sys.exit(main())
