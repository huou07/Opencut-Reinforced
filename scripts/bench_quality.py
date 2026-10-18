#!/usr/bin/env python3
"""Bounded local performance budget for OR's named host class.

One small, explicit budget instead of none. Every row is measured end to end on
the host named below, reports the distribution it observed, states what it does
not establish, and fails the gate when a deliberate regression is injected.

Rows
----
``cli-cold-start``      a fresh ``or version --json`` process.
``ffmpeg-export``       the ignored ``software_export`` WebM lane.
``export-throughput``   frames per second derived from that same lane.
``command.*``           the canonical command path on a 200-clip project:
                        create, import, add track, add clip, trim, split, undo
                        and the two reads a seek performs.
``project-open``        loading that project file from disk.
``project-save``        publishing it atomically.
``render-frame-time``   a 640x360 offscreen ``or_render`` frame (submission plus
                        readback) on the host's real GPU.
``queue-priority``      that same frame while the job pool runs CPU-bound
                        opportunistic work, so "playback outranks analysis" is
                        measured rather than asserted.

Method and limits
-----------------
* ``best-of-N`` for the process rows, ``p95`` for the in-process ones. A
  best-case bound stays honest on a host busy with sibling builds while a
  systematic regression still trips; a p95 bound is what a user feels.
* Command, project and render rows run in-process debug builds. They are a
  regression guard for OR's own code, not a claim about a release build.
* A software rasteriser is reported as software and is never a GPU or device
  performance claim. The adapter name, driver and backend are printed with the
  render rows.
* A breached row beside a contended host is a measurement to repeat, not
  automatically a product regression; the report prints MemAvailable and any
  foreign Gradle/Kotlin daemon so the difference is visible.

Calibration host (named in ``docs/QUALITY.md``): Debian 13 (trixie) x86_64
OptiPlex-class workstation, 13th Gen Intel Core i5-13500T, 15 GiB RAM, NVMe SSD,
Intel UHD 770 through Vulkan. Baselines measured 2026-10-18. Hosted runners keep
their own thresholds and this script never runs there.

Testing the budget itself: ``OR_BENCH_INJECT_SLEEP_MS=<ms>`` adds a real delay in
front of the timed CLI command, and ``OR_BENCH_ONLY=<row,row>`` selects rows.
``scripts/test_bench_quality.py`` uses the first to prove a genuine regression
trips the unmodified thresholds.

Exit codes: 0 every measured row is inside budget, 1 a row breached it or a lane
failed, 3 a row could not be measured on this host.
"""
from __future__ import annotations

import argparse
import glob
import json
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
    "(13th Gen Intel Core i5-13500T, 15 GiB RAM, NVMe SSD, Intel UHD 770/Vulkan)"
)

# One entry per enforced row. `baseline` records the measurement the threshold
# was set from, so the margin is auditable rather than folklore.
ROWS = {
    "cli-cold-start": {
        "description": "or version --json (fresh process)",
        "runs": 15,
        "threshold_ms": 200.0,
        "baseline": {"date": "2026-10-18", "min_ms": 1.1, "median_ms": 2.0},
        "settle": "cargo build -p or_cli --locked && python3 scripts/bench_quality.py",
    },
    "ffmpeg-export": {
        "description": "software_export ignored WebM fixture (64x48, 10 frames, VP9/Opus)",
        "runs": 5,
        "threshold_ms": 1500.0,
        "baseline": {"date": "2026-10-18", "min_ms": 35.1, "median_ms": 42.1},
        "settle": (
            "cargo test -p or_media --test software_export --no-run --locked && "
            "python3 scripts/bench_quality.py"
        ),
    },
    "export-throughput": {
        "description": "frames per second through the same WebM export lane",
        "threshold_fps": 60.0,
        "baseline": {"date": "2026-10-18", "fps": 285.0},
        "settle": "same as ffmpeg-export",
    },
    "command.create": {
        "description": "project create on a fresh path",
        "threshold_p95_ms": 5.0,
        "baseline": {"date": "2026-10-18", "p50_ms": 0.052, "p95_ms": 0.089},
    },
    "command.import": {
        "description": "media.add after a real ffprobe of a 1 s clip",
        "threshold_p95_ms": 60.0,
        "baseline": {"date": "2026-10-18", "p50_ms": 14.339, "p95_ms": 20.321},
    },
    "command.add_track": {
        "description": "timeline.track.add on a 200-clip project",
        "threshold_p95_ms": 2.0,
        "baseline": {"date": "2026-10-18", "p50_ms": 0.013, "p95_ms": 0.016},
    },
    "command.add_clip": {
        "description": "timeline.clip.insert, 200 of them",
        "threshold_p95_ms": 2.0,
        "baseline": {"date": "2026-10-18", "p50_ms": 0.027, "p95_ms": 0.038},
    },
    "command.overwrite_insert": {
        "description": "timeline.clip.insert in overwrite mode, replacing one clip",
        "threshold_p95_ms": 6.0,
        "baseline": {"date": "2026-10-19", "p50_ms": 0.260, "p95_ms": 0.286},
    },
    "command.ripple_trim": {
        "description": "timeline.clip.trim with ripple, shifting every later clip",
        "threshold_p95_ms": 6.0,
        "baseline": {"date": "2026-10-19", "p50_ms": 0.295, "p95_ms": 0.312},
    },
    "command.slip": {
        "description": "timeline.clip.slip, moving one clip's source window",
        "threshold_p95_ms": 4.0,
        "baseline": {"date": "2026-10-20", "p50_ms": 0.131, "p95_ms": 0.136},
    },
    "command.roll": {
        "description": "timeline.clip.roll, moving a boundary between two clips",
        "threshold_p95_ms": 4.0,
        "baseline": {"date": "2026-10-20", "p50_ms": 0.229, "p95_ms": 0.238},
    },
    "command.slide": {
        "description": "timeline.clip.slide, moving a clip between two neighbours",
        "threshold_p95_ms": 5.0,
        "baseline": {"date": "2026-10-20", "p50_ms": 0.271, "p95_ms": 0.291},
    },
    "command.trim": {
        "description": "timeline.clip.trim on a 200-clip project",
        "threshold_p95_ms": 5.0,
        "baseline": {"date": "2026-10-18", "p50_ms": 0.139, "p95_ms": 0.195},
    },
    "command.split": {
        "description": "timeline.clip.split on a 200-clip project",
        "threshold_p95_ms": 5.0,
        "baseline": {"date": "2026-10-18", "p50_ms": 0.175, "p95_ms": 0.203},
    },
    "command.undo": {
        "description": "history.undo on a 200-clip project",
        "threshold_p95_ms": 5.0,
        "baseline": {"date": "2026-10-18", "p50_ms": 0.150, "p95_ms": 0.165},
    },
    "command.seek_summary": {
        "description": "project.summary, the read a seek performs",
        "threshold_p95_ms": 1.0,
        "baseline": {"date": "2026-10-18", "p50_ms": 0.000, "p95_ms": 0.000},
    },
    "command.seek_clip_page": {
        "description": "timeline.clips page of 100 on a 200-clip track",
        "threshold_p95_ms": 2.0,
        "baseline": {"date": "2026-10-18", "p50_ms": 0.006, "p95_ms": 0.007},
    },
    "project-open": {
        "description": "ProjectFileSession::open of the 200-clip project file",
        "threshold_p95_ms": 60.0,
        "baseline": {"date": "2026-10-18", "p50_ms": 8.829, "p95_ms": 9.005},
    },
    "project-save": {
        "description": "ProjectFileSession::save (atomic publish)",
        "threshold_p95_ms": 80.0,
        "baseline": {"date": "2026-10-18", "p50_ms": 15.018, "p95_ms": 15.239},
    },
    "render-frame-time": {
        "description": "640x360 offscreen or_render frame plus readback (Vulkan, UHD 770)",
        "threshold_p95_ms": 16.7,
        "baseline": {"date": "2026-10-18", "p50_ms": 1.962, "p95_ms": 2.183},
    },
    "queue-priority": {
        "description": "render-frame p95 ratio while the job pool runs CPU-bound opportunistic work",
        # The product bound is the loaded p95 (a 30 fps frame budget); the ratio
        # guards against a pathological coupling between the two paths. Measured
        # ratios on an idle host were 2.14-3.16, so the ratio budget is 6.0: far
        # enough out that scheduler noise on a busy host does not fail the gate,
        # near enough in that a lock or a priority inversion would trip it.
        "threshold_ratio": 6.0,
        "threshold_loaded_p95_ms": 33.3,
        "baseline": {"date": "2026-10-18", "idle_p95_ms": 2.768, "loaded_p95_ms": 6.295,
                     "ratio_p95": 2.274},
    },
}

EXPORT_TEST = "writer_publishes_webm_with_vp9_and_opus_streams"
EXPORT_FRAMES = 10
COMMAND_LATENCY_TEST = "command_latency_on_a_two_hundred_clip_project"
RENDER_TEST = "offscreen_frame_time_is_measurable_on_this_host"
PRIORITY_TEST = "playback_critical_frame_time_survives_a_saturated_job_pool"


class Unmeasurable(Exception):
    """The host cannot produce an honest number for a row."""

    def __init__(self, row_id: str, reason: str, settle: str) -> None:
        super().__init__(reason)
        self.row_id = row_id
        self.reason = reason
        self.settle = settle


class RowResult:
    def __init__(self, row_id: str, verdict: str, detail: str,
                 data: dict | None = None, reason: str | None = None) -> None:
        self.row_id = row_id
        self.verdict = verdict          # PASS / FAIL / UNVERIFIED
        self.detail = detail
        self.data = data or {}
        self.reason = reason


def selected(row_id: str) -> bool:
    only = os.environ.get("OR_BENCH_ONLY", "").strip()
    if not only:
        return True
    return row_id in {part.strip() for part in only.split(",")}


def threshold_scale() -> float:
    """A documented seam that tightens every budget by one factor.

    `scripts/test_bench_quality.py` sets a tiny scale so that real, measured
    numbers breach the *enforcement path* of every row kind, which is otherwise
    only reachable by introducing a genuine product regression. It never loosens
    a budget: values above 1.0 are ignored.
    """
    try:
        scale = float(os.environ.get("OR_BENCH_THRESHOLD_SCALE", "1") or 1)
    except ValueError:
        return 1.0
    return scale if 0 < scale < 1 else 1.0


def budget(threshold: float) -> float:
    """Resolve one upper-bound threshold (a ceiling) through the scale seam."""
    return threshold * threshold_scale()


def budget_floor(threshold: float) -> float:
    """Resolve one lower-bound threshold (a floor) through the scale seam.

    Tightening a floor means raising it, so the scale divides here instead of
    multiplying. A seam that loosened as it "tightened" would make the injected
    regression proof meaningless for the throughput row.
    """
    return threshold / threshold_scale()


def inject_sleep_ms() -> float:
    """A documented seam that makes a measured command genuinely slower.

    `scripts/test_bench_quality.py` uses it to prove that a real regression trips
    the unmodified thresholds, rather than only exercising the comparison in
    isolation.
    """
    try:
        return float(os.environ.get("OR_BENCH_INJECT_SLEEP_MS", "0") or 0)
    except ValueError:
        return 0.0


def host_notes() -> list[str]:
    notes = []
    meminfo = Path("/proc/meminfo")
    if meminfo.is_file():
        for line in meminfo.read_text(encoding="utf-8").splitlines():
            if line.startswith("MemAvailable:"):
                notes.append(f"MemAvailable {int(line.split()[1]) / 1048576:.2f} GiB")
                break
    daemons = subprocess.run(["pgrep", "-af", "GradleDaemon|KotlinCompileDaemon"],
                             capture_output=True, text=True, check=False)
    for line in daemons.stdout.splitlines():
        notes.append("foreign build process: " + line.strip()[:120])
    return notes


# -- shared helpers --------------------------------------------------------
def build_cli_if_missing() -> Path:
    binary = ROOT / "target" / "debug" / "or"
    if os.access(binary, os.X_OK):
        return binary
    cargo = shutil.which("cargo")
    if cargo is None:
        raise Unmeasurable("cli-cold-start", "no built target/debug/or and cargo is not on PATH",
                           ROWS["cli-cold-start"]["settle"])
    print("bench: target/debug/or is absent; running `cargo build -p or_cli --locked`")
    completed = subprocess.run([cargo, "build", "-p", "or_cli", "--locked"], cwd=ROOT,
                               capture_output=True, text=True, timeout=600, check=False)
    if completed.returncode != 0:
        tail = (completed.stderr or completed.stdout or "").strip().splitlines()[-15:]
        raise Unmeasurable("cli-cold-start",
                           "`cargo build -p or_cli --locked` failed:\n    " + "\n    ".join(tail),
                           ROWS["cli-cold-start"]["settle"])
    if not os.access(binary, os.X_OK):
        raise Unmeasurable("cli-cold-start",
                           "cargo reported success but target/debug/or still does not exist",
                           ROWS["cli-cold-start"]["settle"])
    return binary


def find_test_binary(prefix: str, row_id: str, settle: str) -> Path:
    """Return the newest prebuilt test binary for one lane.

    The benchmark never builds a lane on its own: whether the pinned toolchain is
    usable is the acceptance lane's decision, and a benchmark must not surprise a
    host with a compile.
    """
    candidates = [
        Path(path)
        for path in glob.glob(str(ROOT / "target" / "debug" / "deps" / f"{prefix}-*"))
        if not path.endswith(".d") and os.access(path, os.X_OK)
    ]
    if not candidates:
        raise Unmeasurable(row_id, f"no prebuilt {prefix} test binary", settle)
    return max(candidates, key=lambda path: path.stat().st_mtime)


def wall_time_ms(argv: list[str]) -> float:
    start = time.perf_counter()
    subprocess.run(argv, cwd=ROOT, capture_output=True, check=True)
    return (time.perf_counter() - start) * 1000.0


def best_of(row_id: str, argv: list[str], runs: int) -> tuple[float, float]:
    samples = [wall_time_ms(argv) for _ in range(runs)]
    return min(samples), statistics.median(samples)


def bench_json(binary: Path, test_name: str, row_id: str, settle: str,
               timeout: int = 900) -> dict:
    """Run one ignored measurement lane and return its OR_BENCH_JSON payload."""
    completed = subprocess.run(
        [str(binary), "--ignored", "--exact", test_name, "--test-threads=1", "--nocapture"],
        cwd=ROOT, capture_output=True, text=True, timeout=timeout, check=False,
    )
    if completed.returncode != 0:
        tail = (completed.stderr or completed.stdout or "").strip().splitlines()[-6:]
        raise Unmeasurable(row_id, f"the lane exited {completed.returncode}: " + " | ".join(tail),
                           settle)
    for line in completed.stdout.splitlines():
        # libtest prefixes the marker with its progress text on the same line
        # ("test <name> ... OR_BENCH_JSON={...}"), so search rather than prefix.
        marker = line.find("OR_BENCH_JSON=")
        if marker >= 0:
            payload = json.loads(line[marker + len("OR_BENCH_JSON="):])
            if payload.get("result") != "MEASURED":
                raise Unmeasurable(row_id, f"the lane reported {payload.get('result')}: "
                                           f"{payload.get('reason', '')}", settle)
            return payload
    raise Unmeasurable(row_id, "the lane printed no OR_BENCH_JSON line", settle)


# -- rows ------------------------------------------------------------------
def row_cli_cold_start() -> RowResult:
    row = ROWS["cli-cold-start"]
    binary = build_cli_if_missing()
    argv = [str(binary), "version", "--json"]
    sleep_ms = inject_sleep_ms()
    if sleep_ms:
        print(f"bench: OR_BENCH_INJECT_SLEEP_MS={sleep_ms:.0f} injected into cli-cold-start")
        argv = ["bash", "-c", f"sleep {sleep_ms / 1000.0}; exec " + " ".join(argv)]
    best, median = best_of("cli-cold-start", argv, int(row["runs"]))
    limit = budget(float(row["threshold_ms"]))
    verdict = "PASS" if best <= limit else "FAIL"
    baseline = row["baseline"]
    detail = (f"best {best:.1f} ms / median {median:.1f} ms (budget {limit:.1f} ms; "
              f"baseline {baseline['min_ms']:.1f} ms on {baseline['date']}, "
              f"margin x{limit / baseline['min_ms']:.0f})")
    return RowResult("cli-cold-start", verdict, detail,
                     {"best_ms": round(best, 1), "median_ms": round(median, 1),
                      "threshold_ms": limit})


def find_export_binary() -> Path:
    settle = ROWS["ffmpeg-export"]["settle"]
    return find_test_binary("software_export", "ffmpeg-export", settle)


def row_ffmpeg_export() -> tuple[RowResult, float]:
    row = ROWS["ffmpeg-export"]
    export = find_export_binary()
    argv = [str(export), "--ignored", "--exact", EXPORT_TEST, "--test-threads=1"]
    best, median = best_of("ffmpeg-export", argv, int(row["runs"]))
    limit = budget(float(row["threshold_ms"]))
    verdict = "PASS" if best <= limit else "FAIL"
    baseline = row["baseline"]
    detail = (f"best {best:.1f} ms / median {median:.1f} ms (budget {limit:.1f} ms; "
              f"baseline {baseline['min_ms']:.1f} ms on {baseline['date']}, "
              f"margin x{limit / baseline['min_ms']:.0f})")
    return RowResult("ffmpeg-export", verdict, detail,
                     {"best_ms": round(best, 1), "median_ms": round(median, 1),
                      "threshold_ms": limit}), best


def row_export_throughput(best_ms: float | None) -> RowResult:
    row = ROWS["export-throughput"]
    if best_ms is None or best_ms <= 0:
        return RowResult("export-throughput", "UNVERIFIED", "the export lane did not produce a time",
                         reason="the ffmpeg-export row did not run")
    fps = EXPORT_FRAMES / (best_ms / 1000.0)
    limit = budget_floor(float(row["threshold_fps"]))
    verdict = "PASS" if fps >= limit else "FAIL"
    baseline = row["baseline"]
    detail = (f"{fps:.0f} frames/s (floor {limit:.0f} frames/s; baseline "
              f"{baseline['fps']:.0f} frames/s on {baseline['date']}, "
              f"margin x{fps / baseline['fps']:.2f} of baseline)")
    return RowResult("export-throughput", verdict, detail,
                     {"fps": round(fps, 1), "threshold_fps": limit})


def command_latency_rows() -> list[RowResult]:
    settle = ("cargo test -p or_core --test command_latency --no-run --locked && "
              "python3 scripts/bench_quality.py")
    binary = find_test_binary("command_latency", "command.create", settle)
    payload = bench_json(binary, COMMAND_LATENCY_TEST, "command.create", settle)
    results = []
    for entry in payload["rows"]:
        source = entry["id"]
        # The lane names its project-file rows `project_open` / `project_save`
        # because they are session operations, not commands; the budget names
        # them `project-open` / `project-save`.
        row_id = source.replace("_", "-") if source.startswith("project_") else f"command.{source}"
        row = ROWS.get(row_id)
        if row is None:
            # The lane gained a row this script does not budget yet. Report it
            # rather than silently dropping the measurement.
            results.append(RowResult(row_id, "UNVERIFIED",
                                     f"measured p95 {entry['p95_ms']:.3f} ms but has no budget",
                                     reason="no threshold is declared for this measured row"))
            continue
        limit = budget(float(row["threshold_p95_ms"]))
        verdict = "PASS" if entry["p95_ms"] <= limit else "FAIL"
        detail = (f"p50 {entry['p50_ms']:.3f} ms / p95 {entry['p95_ms']:.3f} ms "
                  f"(budget p95 {limit:.3f} ms; baseline p95 {row['baseline']['p95_ms']:.3f} ms "
                  f"on {row['baseline']['date']})")
        results.append(RowResult(row_id, verdict, detail, {
            "min_ms": entry["min_ms"], "p50_ms": entry["p50_ms"], "p95_ms": entry["p95_ms"],
            "max_ms": entry["max_ms"], "count": entry["count"], "threshold_p95_ms": limit,
        }))
    print(f"bench: command lane built a {payload['clips']}-clip project of "
          f"{payload['project_bytes']} bytes; {payload['build']} build; "
          f"{payload['iterations']} iterations per command")
    return results


def render_frame_time_row() -> RowResult:
    settle = ("cargo test -p or_render --test render_frame_time --no-run --locked && "
              "python3 scripts/bench_quality.py")
    binary = find_test_binary("render_frame_time", "render-frame-time", settle)
    payload = bench_json(binary, RENDER_TEST, "render-frame-time", settle)
    row = ROWS["render-frame-time"]
    limit = budget(float(row["threshold_p95_ms"]))
    verdict = "PASS" if payload["p95_ms"] <= limit else "FAIL"
    detail = (f"p50 {payload['p50_ms']:.3f} ms / p95 {payload['p95_ms']:.3f} ms over "
              f"{payload['frames']} frames (budget p95 {limit:.1f} ms; baseline p95 "
              f"{row['baseline']['p95_ms']:.3f} ms on {row['baseline']['date']}) on "
              f"{payload['adapter']} / {payload['driver']} / {payload['backend']} "
              f"[{payload['adapter_class']}]; {payload['includes']}")
    return RowResult("render-frame-time", verdict, detail, {
        "p50_ms": payload["p50_ms"], "p95_ms": payload["p95_ms"], "frames": payload["frames"],
        "adapter": payload["adapter"], "driver": payload["driver"], "backend": payload["backend"],
        "adapter_class": payload["adapter_class"], "threshold_p95_ms": limit,
    })


def queue_priority_row() -> RowResult:
    settle = ("cargo test -p or_render --test queue_priority --no-run --locked && "
              "python3 scripts/bench_quality.py")
    binary = find_test_binary("queue_priority", "queue-priority", settle)
    payload = bench_json(binary, PRIORITY_TEST, "queue-priority", settle)
    row = ROWS["queue-priority"]
    ratio_budget = budget(float(row["threshold_ratio"]))
    loaded_budget = budget(float(row["threshold_loaded_p95_ms"]))
    breached = []
    if payload["ratio_p95"] > ratio_budget:
        breached.append(f"p95 ratio {payload['ratio_p95']:.2f} > {ratio_budget:.2f}")
    if payload["loaded_p95_ms"] > loaded_budget:
        breached.append(f"loaded p95 {payload['loaded_p95_ms']:.2f} ms > {loaded_budget:.1f} ms")
    verdict = "PASS" if not breached else "FAIL"
    detail = (f"idle p95 {payload['idle_p95_ms']:.3f} ms -> loaded p95 "
              f"{payload['loaded_p95_ms']:.3f} ms, ratio {payload['ratio_p95']:.2f} "
              f"(ratio budget {ratio_budget:.2f}, loaded budget {loaded_budget:.1f} ms; "
              f"baseline ratio {row['baseline']['ratio_p95']:.2f} on {row['baseline']['date']}) "
              f"with {payload['submitted']} opportunistic jobs on {payload['workers']} workers"
              + (f"; BREACH: {', '.join(breached)}" if breached else ""))
    return RowResult("queue-priority", verdict, detail, {
        "idle_p95_ms": payload["idle_p95_ms"], "loaded_p95_ms": payload["loaded_p95_ms"],
        "ratio_p95": payload["ratio_p95"], "workers": payload["workers"],
        "submitted": payload["submitted"], "adapter_class": payload["adapter_class"],
        "threshold_ratio": ratio_budget, "threshold_loaded_p95_ms": loaded_budget,
    })


def run_row(row_id: str, work) -> RowResult:
    """Run one row, converting a host limitation into an UNVERIFIED result."""
    try:
        return work()
    except Unmeasurable as error:
        print(f"row {row_id:<20}: UNVERIFIED — {error.reason}")
        print(f"    settle with: {error.settle}")
        return RowResult(row_id, "UNVERIFIED", error.reason, reason=error.reason)
    except subprocess.CalledProcessError as error:
        print(f"row {row_id:<20}: FAIL — the lane exited {error.returncode}")
        return RowResult(row_id, "FAIL", f"the lane exited {error.returncode}",
                         reason="lane failure")
    except subprocess.TimeoutExpired:
        print(f"row {row_id:<20}: FAIL — the lane exceeded its time budget")
        return RowResult(row_id, "FAIL", "the lane exceeded its time budget",
                         reason="lane timeout")


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--json", action="store_true",
                        help="also print one machine-readable summary object")
    args = parser.parse_args(argv)

    print("== Performance budget (local, bounded) ==")
    print(f"host class : {HOST_CLASS}")
    for note in host_notes():
        print(f"host note  : {note}")

    if not any(selected(row_id) for row_id in ROWS):
        print("bench: no budget row was selected by OR_BENCH_ONLY; nothing to measure")
        return 2

    results: list[RowResult] = []
    export_best: float | None = None

    if selected("cli-cold-start"):
        results.append(run_row("cli-cold-start", row_cli_cold_start))
    # The export row needs its best time for the throughput row, so it is measured
    # once and both verdicts are derived from that one measurement.
    if selected("ffmpeg-export") or selected("export-throughput"):
        try:
            export_row, export_best = row_ffmpeg_export()
        except Unmeasurable as error:
            export_row = RowResult("ffmpeg-export", "UNVERIFIED", error.reason,
                                   reason=error.reason)
            print(f"row {'ffmpeg-export':<20}: UNVERIFIED — {error.reason}")
            print(f"    settle with: {error.settle}")
        results = [entry for entry in results if entry.row_id != "ffmpeg-export"]
        if selected("ffmpeg-export"):
            results.append(export_row)
        if selected("export-throughput"):
            results.append(row_export_throughput(export_best))
    if any(selected(row_id) for row_id in
           ("command.create", "command.import", "command.add_track", "command.add_clip",
            "command.overwrite_insert", "command.ripple_trim", "command.slip",
            "command.roll", "command.slide", "command.trim", "command.split",
            "command.undo",
            "command.seek_summary",
            "command.seek_clip_page", "project-open", "project-save")):
        try:
            results.extend(command_latency_rows())
        except Unmeasurable as error:
            print(f"row {'command.*':<20}: UNVERIFIED — {error.reason}")
            print(f"    settle with: {error.settle}")
            results.append(RowResult(error.row_id, "UNVERIFIED", error.reason,
                                     reason=error.reason))
    if selected("render-frame-time"):
        results.append(run_row("render-frame-time", render_frame_time_row))
    if selected("queue-priority"):
        results.append(run_row("queue-priority", queue_priority_row))

    for result in results:
        if result.verdict != "UNVERIFIED":
            print(f"row {result.row_id:<20}: {result.verdict} — {result.detail}")

    if args.json:
        print("BENCH_JSON=" + json.dumps({
            "host_class": HOST_CLASS,
            "host_notes": host_notes(),
            "rows": [{"id": r.row_id, "result": r.verdict, "detail": r.detail, **r.data}
                     for r in results],
        }))

    if any(result.verdict == "FAIL" for result in results):
        print("RESULT: PERFORMANCE_BUDGET=FAIL")
        return 1
    if any(result.verdict == "UNVERIFIED" for result in results):
        print("RESULT: PERFORMANCE_BUDGET=UNVERIFIED")
        return 3
    print("RESULT: PERFORMANCE_BUDGET=PASS")
    return 0


if __name__ == "__main__":
    sys.exit(main())
