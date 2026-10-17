#!/usr/bin/env python3
"""Bounded local performance budget for OR's named host class.

OR has one small, explicit performance budget instead of none. It covers two
rows that this Debian workstation can measure honestly, both end to end:

* ``cli-cold-start``  — a fresh ``or version --json`` process, so process
  startup, dynamic linking and argument handling are included.
* ``ffmpeg-export``   — the existing ignored ``software_export`` WebM lane
  (``writer_publishes_webm_with_vp9_and_opus_streams``): a 64x48, 10-frame
  VP9/Opus fixture through the real FFmpeg encoder and muxer.

Both rows are enforced on the **best of N runs**, not the mean: a best-case
bound stays honest on a host that is busy with sibling builds, while a
systematic regression (an accidental sleep, a retry loop, a re-encode, an
O(n^2) walk) still trips. The thresholds are deliberately coarse regression
guards, not the frame-time or latency budget the product roadmap still owes:
software-rendered preflight and SwiftShader emulator numbers describe the
absence of a GPU, so no GPU-dependent budget is asserted here.

Calibration host (named in ``docs/QUALITY.md``): Debian 13 (trixie) x86_64
OptiPlex-class workstation, 13th Gen Intel Core i5-13500T, 15 GiB RAM, NVMe SSD.
Baselines measured 2026-10-18 on an idle host with the pinned FFmpeg 8 binaries
on PATH. Any number here is a property of that class; hosted runners keep their
own thresholds and this script never runs there.

Exit codes: 0 when every row is inside budget, 1 when a row breaches it or the
lane itself fails, 3 when a row cannot be measured on this host. A row that
cannot run is never reported as passing; the exact command that would settle it
is printed instead.
"""
from __future__ import annotations

import argparse
import glob
import os
import shutil
import statistics
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

HOST_CLASS = (
    "Debian 13 (trixie) x86_64 OptiPlex-class workstation "
    "(13th Gen Intel Core i5-13500T, 15 GiB RAM, NVMe SSD)"
)

# One entry per enforced row. `baseline` records the measurement the threshold
# was set from, so the margin is auditable rather than folklore.
ROWS = (
    {
        "id": "cli-cold-start",
        "description": "or version --json (fresh process)",
        "runs": 15,
        "threshold_ms": 200.0,
        "baseline": {"date": "2026-10-18", "min_ms": 1.1, "median_ms": 2.0},
        "settle": "cargo build -p or_cli --locked && python3 scripts/bench_quality.py",
    },
    {
        "id": "ffmpeg-export",
        "description": "software_export ignored WebM fixture (64x48, 10 frames, VP9/Opus)",
        "runs": 5,
        "threshold_ms": 1500.0,
        "baseline": {"date": "2026-10-18", "min_ms": 35.1, "median_ms": 42.1},
        "settle": (
            "cargo test -p or_media --test software_export --no-run --locked && "
            "python3 scripts/bench_quality.py"
        ),
    },
)

# The ignored test the export row times. Kept exact so the row measures one
# named lane rather than whatever else the test binary happens to carry.
EXPORT_TEST = "writer_publishes_webm_with_vp9_and_opus_streams"


class Unmeasurable(Exception):
    """The host cannot produce an honest number for a row."""

    def __init__(self, row_id: str, reason: str, settle: str) -> None:
        super().__init__(reason)
        self.row_id = row_id
        self.reason = reason
        self.settle = settle


def build_cli_if_missing() -> Path:
    """Return the debug CLI binary, building it once when it is absent."""
    binary = ROOT / "target" / "debug" / "or"
    if os.access(binary, os.X_OK):
        return binary
    cargo = shutil.which("cargo")
    if cargo is None:
        raise Unmeasurable(
            "cli-cold-start",
            "no built target/debug/or and cargo is not on PATH",
            ROWS[0]["settle"],
        )
    print("bench: target/debug/or is absent; running `cargo build -p or_cli --locked`")
    completed = subprocess.run(
        [cargo, "build", "-p", "or_cli", "--locked"],
        cwd=ROOT,
        capture_output=True,
        text=True,
        timeout=600,
        check=False,
    )
    if completed.returncode != 0:
        tail = (completed.stderr or completed.stdout or "").strip().splitlines()[-15:]
        raise Unmeasurable(
            "cli-cold-start",
            "`cargo build -p or_cli --locked` failed:\n    " + "\n    ".join(tail),
            ROWS[0]["settle"],
        )
    if not os.access(binary, os.X_OK):
        raise Unmeasurable(
            "cli-cold-start",
            "cargo reported success but target/debug/or still does not exist",
            ROWS[0]["settle"],
        )
    return binary


def find_export_binary() -> Path:
    """Return the newest prebuilt software_export test binary.

    The benchmark never builds this lane on its own: whether the pinned FFmpeg
    toolchain is usable is the acceptance media lane's decision, and a benchmark
    must not surprise a host with a codec build. When the binary is absent the
    row is unmeasurable, with the command that would produce it.
    """
    candidates = [
        Path(path)
        for path in glob.glob(str(ROOT / "target" / "debug" / "deps" / "software_export-*"))
        if not path.endswith(".d") and os.access(path, os.X_OK)
    ]
    if not candidates:
        raise Unmeasurable("ffmpeg-export", "no prebuilt software_export test binary", ROWS[1]["settle"])
    return max(candidates, key=lambda path: path.stat().st_mtime)


def wall_time_ms(argv: list[str]) -> float:
    start = time.perf_counter()
    subprocess.run(argv, cwd=ROOT, capture_output=True, check=True)
    return (time.perf_counter() - start) * 1000.0


def measure(row: dict, argv: list[str]) -> tuple[float, float]:
    """Run one row `runs` times; return (best_ms, median_ms)."""
    samples = [wall_time_ms(argv) for _ in range(int(row["runs"]))]
    return min(samples), statistics.median(samples)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--json",
        action="store_true",
        help="also print one machine-readable summary object",
    )
    args = parser.parse_args(argv)

    print("== Performance budget (local, bounded) ==")
    print(f"host class : {HOST_CLASS}")

    plan: dict[str, list[str]] = {}
    unmeasurable: list[Unmeasurable] = []
    try:
        cli = build_cli_if_missing()
        plan["cli-cold-start"] = [str(cli), "version", "--json"]
    except Unmeasurable as error:
        unmeasurable.append(error)
    try:
        export = find_export_binary()
        plan["ffmpeg-export"] = [
            str(export),
            "--ignored",
            "--exact",
            EXPORT_TEST,
            "--test-threads=1",
        ]
    except Unmeasurable as error:
        unmeasurable.append(error)

    results: list[dict] = []
    failed = False
    for row in ROWS:
        row_id = row["id"]
        if row_id not in plan:
            error = next(item for item in unmeasurable if item.row_id == row_id)
            print(f"row {row_id:<16}: UNVERIFIED — {error.reason}")
            print(f"    settle with: {error.settle}")
            results.append({"id": row_id, "result": "UNVERIFIED", "reason": error.reason})
            continue
        try:
            best, median = measure(row, plan[row_id])
        except subprocess.CalledProcessError as error:
            failed = True
            print(f"row {row_id:<16}: FAIL — the lane exited {error.returncode}")
            results.append({"id": row_id, "result": "FAIL", "returncode": error.returncode})
            continue
        budget = float(row["threshold_ms"])
        baseline = row["baseline"]
        verdict = "PASS" if best <= budget else "FAIL"
        if verdict == "FAIL":
            failed = True
        print(
            f"row {row_id:<16}: {verdict} — best {best:.1f} ms / median {median:.1f} ms "
            f"(budget {budget:.1f} ms; baseline {baseline['min_ms']:.1f} ms on "
            f"{baseline['date']}, margin x{budget / baseline['min_ms']:.0f})"
        )
        results.append(
            {
                "id": row_id,
                "result": verdict,
                "best_ms": round(best, 1),
                "median_ms": round(median, 1),
                "threshold_ms": budget,
                "baseline_date": baseline["date"],
            }
        )

    if args.json:
        import json

        print("BENCH_JSON=" + json.dumps({"host_class": HOST_CLASS, "rows": results}))

    if failed:
        print("RESULT: PERFORMANCE_BUDGET=FAIL")
        return 1
    if any(item["result"] == "UNVERIFIED" for item in results):
        print("RESULT: PERFORMANCE_BUDGET=UNVERIFIED")
        return 3
    print("RESULT: PERFORMANCE_BUDGET=PASS")
    return 0


if __name__ == "__main__":
    sys.exit(main())
