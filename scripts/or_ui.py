#!/usr/bin/env python3
"""The OR agent-facing UI harness: observe and drive the real app by semantics.

One instrument for the interface dimension of OR's acceptance bar. It runs the
real application -- the real Rust host, the real command path -- on Linux (under
Xvfb) or on the Android emulator, and answers five questions:

  screens   which screens does the running app expose, and what does each one say?
  tree      what is the semantics tree right now (JSON plus the framework dump)?
  shot      one screenshot at the viewport the caller named, and its path.
  act       perform one action against a control named by key, label, role or text.
  run       execute a scenario file step by step and write the transcript.

Every action is semantic. A scenario names a widget key, a semantics label (exact
or regular expression), visible text, or a role; it never names a screen
coordinate, and this harness has no option that injects canonical project state.
The only host integration it replaces is the OS file picker, which a test cannot
drive: `integration_test/or_ui_scenario_test.dart` supplies a synthetic fixture
picker so an import journey is still a real import through the real command path.

Evidence classes this can establish, and what they are not:

* ``VERIFIED_SEMANTIC`` -- roles, labels, states, actions and keyboard
  reachability read from the Flutter semantics tree.
* ``VERIFIED_BEHAVIORAL`` -- a journey that ran through the real app and left the
  canonical project state the steps reported.
* ``VERIFIED_VISUAL`` -- a screenshot at a named viewport, theme and state.

It never establishes ``VERIFIED_NATIVE``: a Linux debug app under Xvfb is not an
installed package, and an emulator result is not a physical device. Timing
captured here is software-path evidence on this host and is never a GPU or
device performance claim.

Exit codes: 0 the scenario passed; 1 a step failed or a selector matched nothing;
2 the invocation or a scenario file is wrong; 3 the target could not be measured
on this host (no guest, no Xvfb, no built APK) -- BLOCKED, with the reason and the
command that would settle it.
"""
from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
APP = ROOT / "apps/or_app"
SCENARIO_RUNNER = "integration_test/or_ui_scenario_test.dart"
DRIVER = "test_driver/integration_test.dart"
LANE_DIR = APP / "build/local-lanes"
DEFAULT_REPORT_DIR = APP / "build/ui-harness"
ANDROID_APP_ID = "io.github.huou07.or_app"
ANDROID_SERIAL = os.environ.get("OR_ANDROID_DEVICE_ID", "emulator-5580")
GUEST_SCENARIO = "files/or-ui-scenario.json"
GUEST_ARTIFACTS = "files/or-ui-artifacts"
DEFAULT_VIEWPORT = (1280, 800)
LINUX_BUDGET_SECONDS = 900
ANDROID_BUDGET_SECONDS = 1800


class Blocked(Exception):
    """The host cannot produce an honest result for this target."""

    def __init__(self, reason: str, settle: str) -> None:
        super().__init__(reason)
        self.reason = reason
        self.settle = settle


class ScenarioError(Exception):
    """The invocation or the scenario file is wrong."""


# -- scenario construction -------------------------------------------------
def scenario(name: str, steps: list[dict], *, screens: bool = False,
             viewport: tuple[int, int] | None = None,
             fixtures: list[str] | None = None,
             continue_on_failure: bool = False) -> dict:
    return {
        "name": name,
        "harness": "or_ui",
        "screens": screens,
        "continue_on_failure": continue_on_failure,
        "viewport": {"width": (viewport or DEFAULT_VIEWPORT)[0],
                     "height": (viewport or DEFAULT_VIEWPORT)[1]},
        "fixtures": fixtures or [],
        "steps": steps,
    }


def load_scenario(path: Path) -> dict:
    if not path.is_file():
        raise ScenarioError(f"no such scenario file: {path}")
    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as error:
        raise ScenarioError(f"{path} is not JSON: {error}") from error
    if not isinstance(payload, dict):
        raise ScenarioError(f"{path} must contain a JSON object")
    if not payload.get("name"):
        payload["name"] = path.stem
    if not isinstance(payload.get("steps"), list) or not payload["steps"]:
        raise ScenarioError(f"{path} has no steps")
    for index, step in enumerate(payload["steps"]):
        if not isinstance(step, dict) or not step.get("op"):
            raise ScenarioError(f"{path}: step {index} has no op")
    return payload


# -- host preparation ------------------------------------------------------
def require_linux_tools() -> None:
    missing = [tool for tool in ("flutter", "xvfb-run") if not shutil.which(tool)]
    if missing:
        raise Blocked(
            "missing required tool(s) on PATH: " + ", ".join(missing),
            "install the missing tool; this harness does not change host privileges",
        )


def require_guest() -> None:
    adb = shutil.which("adb")
    if not adb:
        raise Blocked("adb is not on PATH", "install the Android platform tools")
    completed = subprocess.run([adb, "-s", ANDROID_SERIAL, "get-state"],
                               capture_output=True, text=True, timeout=30)
    if completed.stdout.strip() != "device":
        raise Blocked(
            f"the dedicated guest {ANDROID_SERIAL} is not running "
            f"(adb get-state said {completed.stdout.strip() or completed.stderr.strip()!r})",
            "bash scripts/start-android-emulator.sh  # then rerun this harness",
        )


def adb(*args: str, capture: bool = True) -> subprocess.CompletedProcess[str]:
    command = ["adb", "-s", ANDROID_SERIAL, *args]
    return subprocess.run(command, capture_output=capture, text=True, timeout=300)


def ensure_ui_apk() -> Path:
    """The scenario runner is its own Android entrypoint, so it needs its own APK."""
    apk = LANE_DIR / "android-case-apks/ui.apk"
    if apk.is_file() and apk.stat().st_size > 0:
        return apk
    raise Blocked(
        f"no built UI harness APK at {apk}",
        "bash scripts/build_android_case_apks.sh apps/or_app/build/local-lanes/"
        "android-case-apks  # builds ui.apk with the other cases",
    )


# -- execution -------------------------------------------------------------
def run_linux(payload: dict, report_dir: Path) -> int:
    require_linux_tools()
    report_dir.mkdir(parents=True, exist_ok=True)
    scenario_path = report_dir / "scenario.json"
    scenario_path.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
    artifacts = report_dir / "artifacts"
    if artifacts.exists():
        shutil.rmtree(artifacts)
    artifacts.mkdir(parents=True)

    environment = dict(
        os.environ,
        OR_UI_SCENARIO_FILE=str(scenario_path),
        OR_UI_ARTIFACT_DIR=str(artifacts),
        # A scenario that types a file path names it as $OR_UI_ROOT/..., which the
        # runner expands from this value; the path then holds on every host.
        GITHUB_WORKSPACE=str(ROOT),
        LIBGL_ALWAYS_SOFTWARE="1",
        GDK_BACKEND="x11",
    )
    # The preflight entry point is reused, not reimplemented: it stages the
    # project-local Cargo/pub caches, mirrors a writable Flutter SDK, scopes the
    # bridge's Rust toolchain manifest to this host and restores it byte for
    # byte, and holds the one shared native lock.
    inner = (
        "flutter --no-version-check pub get --offline --enforce-lockfile && "
        "xvfb-run -a -s '-screen 0 1440x900x24' "
        f"flutter --no-version-check test --no-pub -d linux {SCENARIO_RUNNER}"
    )
    command = [sys.executable, str(ROOT / "scripts/run_linux_ui_preflight.py"),
               "--exec", "--", "bash", "-c", inner]
    completed = subprocess.run(command, cwd=ROOT, env=environment,
                               capture_output=True, text=True,
                               timeout=LINUX_BUDGET_SECONDS)
    (report_dir / "run.log").write_text(
        f"$ {' '.join(command)}\n{completed.stdout}\n{completed.stderr}\n",
        encoding="utf-8")
    if completed.returncode == 3:
        # The preflight classifies a host precondition as exit 3: insufficient
        # RAM or disk, a missing tool, or another lane holding the shared native
        # UI lock. That is BLOCKED with a settle command, never a scenario FAIL.
        detail = [line for line in (completed.stdout + completed.stderr).splitlines()
                  if line.strip()]
        raise Blocked(
            detail[-1].strip() if detail else "the Linux preflight reported BLOCKED",
            "free the named precondition (see the run log) and rerun this harness",
        )
    if completed.returncode != 0:
        reason = (completed.stdout + completed.stderr).strip().splitlines()[-8:]
        payload_out = {
            "harness": "or_ui", "target": "linux", "result": "FAIL",
            "reason": "the scenario runner exited "
                      f"{completed.returncode}: " + " | ".join(reason),
            "log": str(report_dir / "run.log"),
            "steps": read_transcript(artifacts, payload["name"]).get("steps", []),
        }
        (report_dir / "report.json").write_text(json.dumps(payload_out, indent=2) + "\n")
        print(f"RESULT: OR_UI=FAIL ({payload['name']})")
        print(payload_out["reason"])
        return 1
    return finish(report_dir, artifacts, payload, "linux")


def read_transcript(artifacts: Path, name: str) -> dict:
    path = artifacts / f"{name}.json"
    if not path.is_file():
        return {}
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError:
        return {}


def run_android(payload: dict, report_dir: Path) -> int:
    require_guest()
    apk = ensure_ui_apk()
    report_dir.mkdir(parents=True, exist_ok=True)
    artifacts = report_dir / "artifacts"
    if artifacts.exists():
        shutil.rmtree(artifacts)
    artifacts.mkdir(parents=True)

    # The scenario travels to the app-private files directory, which outlives an
    # install and is readable by the app without a permission grant.
    staged = report_dir / "scenario.json"
    staged.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
    adb("push", str(staged), "/data/local/tmp/or-ui-scenario.json")
    adb("shell", "run-as", ANDROID_APP_ID, "mkdir", "-p", "files")
    adb("shell", "run-as", ANDROID_APP_ID, "rm", "-rf", GUEST_ARTIFACTS)
    adb("shell", "run-as", ANDROID_APP_ID, "cp",
        "/data/local/tmp/or-ui-scenario.json", GUEST_SCENARIO)
    installed = adb("install", "-r", str(apk))
    if installed.returncode != 0:
        raise Blocked(
            f"adb could not install {apk}: {installed.stderr.strip()[:400]}",
            "adb devices -l  # and check the guest is settled",
        )

    environment = dict(os.environ, GITHUB_WORKSPACE=str(ROOT))
    command = [
        "flutter", "drive", f"--driver={DRIVER}", f"--target={SCENARIO_RUNNER}",
        "-d", ANDROID_SERIAL, "--no-dds", f"--use-application-binary={apk}",
    ]
    completed = subprocess.run(command, cwd=APP, env=environment,
                               capture_output=True, text=True,
                               timeout=ANDROID_BUDGET_SECONDS)
    (report_dir / "run.log").write_text(
        f"$ {' '.join(command)}\n{completed.stdout}\n{completed.stderr}\n",
        encoding="utf-8")

    pulled = pull_guest_artifacts(artifacts)
    if completed.returncode != 0 or not pulled:
        reason = (completed.stdout + completed.stderr).strip().splitlines()[-8:]
        payload_out = {
            "harness": "or_ui", "target": "android", "result": "FAIL",
            "guest": {"serial": ANDROID_SERIAL, "apk": str(apk)},
            "reason": (f"the scenario runner exited {completed.returncode}"
                       f" and {len(pulled)} artifact(s) were recovered: "
                       + " | ".join(reason)),
            "log": str(report_dir / "run.log"),
        }
        (report_dir / "report.json").write_text(json.dumps(payload_out, indent=2) + "\n")
        print(f"RESULT: OR_UI=FAIL ({payload['name']})")
        print(payload_out["reason"])
        return 1
    return finish(report_dir, artifacts, payload, "android")


def pull_guest_artifacts(destination: Path) -> list[str]:
    """Copy the app's artifact directory off the guest with `run-as`."""
    listing = adb("shell", "run-as", ANDROID_APP_ID, "ls", GUEST_ARTIFACTS)
    if listing.returncode != 0:
        return []
    names = [line.strip() for line in listing.stdout.splitlines() if line.strip()]
    recovered = []
    for name in names:
        if "/" in name or name in (".", ".."):
            continue
        result = subprocess.run(
            ["adb", "-s", ANDROID_SERIAL, "exec-out", "run-as", ANDROID_APP_ID,
             "cat", f"{GUEST_ARTIFACTS}/{name}"],
            capture_output=True, timeout=300)
        if result.returncode != 0 or not result.stdout:
            continue
        # `exec-out` is binary-clean, so screenshots survive intact.
        (destination / name).write_bytes(result.stdout)
        recovered.append(name)
    return recovered


def finish(report_dir: Path, artifacts: Path, payload: dict, target: str) -> int:
    transcript = read_transcript(artifacts, payload["name"])
    if not transcript:
        raise Blocked(
            f"the {target} runner wrote no transcript at "
            f"{artifacts / (payload['name'] + '.json')}",
            "read the run log; the app may have failed before the first step",
        )
    steps = transcript.get("steps", [])
    report = {
        "harness": "or_ui",
        "target": target,
        "scenario": payload["name"],
        "result": transcript.get("result"),
        "viewport": transcript.get("viewport"),
        "evidence_class": {
            "semantics": "VERIFIED_SEMANTIC",
            "behaviour": "VERIFIED_BEHAVIORAL",
            "visual": "VERIFIED_VISUAL",
            "native": "NOT ESTABLISHED — a debug app under Xvfb or an emulator "
                      "is not an installed package or a physical device",
        },
        "limits": [
            "Linux runs under Xvfb with LIBGL_ALWAYS_SOFTWARE; Android runs on a "
            "SwiftShader x86_64 emulator. Timing here is software-path evidence "
            "on this host, never a GPU or physical-device performance claim.",
            "The OS file picker is the one substituted host integration: the "
            "runner supplies synthetic fixtures, so the journey is real up to "
            "and beyond the picker but not a native file-dialog claim.",
        ],
        "artifacts": str(artifacts),
        "steps": steps,
        "screens": load_json(artifacts / "screen-inventory.json"),
        "not_run": payload.get("not_run", []),
    }
    (report_dir / "report.json").write_text(json.dumps(report, indent=2) + "\n")

    print(f"== or_ui {target}: {payload['name']} ==")
    for step in steps:
        selector = step.get("selector")
        selector_text = json.dumps(selector, separators=(",", ":")) if selector else ""
        print(f"  {step['index']:>2} {step['op']:<12} {step['status']:<9} "
              f"{step.get('elapsed_ms', 0):>8.1f} ms  {selector_text}")
        if step.get("status") != "ok" and step.get("detail"):
            print(f"      {step['detail']}")
        if step.get("measurement"):
            measurement = step["measurement"]
            print(f"      measured {measurement['name']}: p50 "
                  f"{measurement['p50_ms']} ms / p95 {measurement['p95_ms']} ms "
                  f"over {measurement['repeat']} runs")
    if report["screens"]:
        print(f"  screens: {report['screens'].get('count')} "
              f"({', '.join(s['destination'] for s in report['screens']['screens'])})")
    print(f"  report: {report_dir / 'report.json'}")
    print(f"RESULT: OR_UI={report['result']}")
    return 0 if report["result"] == "PASS" else 1


def load_json(path: Path) -> dict | None:
    if not path.is_file():
        return None
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError:
        return None


# -- commands --------------------------------------------------------------
def command_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--target", choices=("linux", "android"), default="linux",
                        help="which target to drive (default: linux)")
    parser.add_argument("--report-dir", default=str(DEFAULT_REPORT_DIR),
                        help="where the transcript, artifacts and report go")
    parser.add_argument("--viewport", default=f"{DEFAULT_VIEWPORT[0]}x{DEFAULT_VIEWPORT[1]}",
                        help="WIDTHxHEIGHT for the scenario's logical viewport")
    sub = parser.add_subparsers(dest="command", required=True)

    sub.add_parser("screens", help="list the screens the running app exposes")

    tree = sub.add_parser("tree", help="dump the current semantics tree")
    tree.add_argument("--name", default="current")

    shot = sub.add_parser("shot", help="capture one screenshot")
    shot.add_argument("--name", default="current")

    act = sub.add_parser("act", help="perform one action by semantics, key, role or text")
    act.add_argument("--op", default="tap",
                     choices=("tap", "long_press", "enter_text", "key",
                              "expect", "expect_absent", "expect_state"))
    act.add_argument("--key")
    act.add_argument("--label")
    act.add_argument("--label-regex", dest="label_regex")
    act.add_argument("--text")
    act.add_argument("--role")
    act.add_argument("--value", help="text for enter_text")
    act.add_argument("--chord", help="keyboard chord such as ctrl+d")
    act.add_argument("--state", help="semantic state for expect_state")

    run = sub.add_parser("run", help="run a scenario file")
    run.add_argument("scenario", help="path to a scenario JSON file")
    return parser


def build_command_scenario(args: argparse.Namespace) -> dict:
    viewport = parse_viewport(args.viewport)
    if args.command == "screens":
        return scenario("screens", [], screens=True, viewport=viewport)
    if args.command == "tree":
        return scenario("tree", [{"op": "tree", "name": args.name}], viewport=viewport)
    if args.command == "shot":
        return scenario("shot", [{"op": "shot", "name": args.name}], viewport=viewport)
    if args.command == "act":
        selector = {}
        for field in ("key", "label", "label_regex", "text", "role"):
            value = getattr(args, field)
            if value:
                selector[field] = value
        step: dict = {"op": args.op}
        if args.op != "key":
            if not selector:
                raise ScenarioError(
                    "act needs --key, --label, --label-regex, --text or --role")
            step["selector"] = selector
        if args.value is not None:
            step["value"] = args.value
        if args.chord is not None:
            step["chord"] = args.chord
        if args.state is not None:
            step["state"] = args.state
        return scenario(f"act-{args.op}", [step], viewport=viewport)
    raise ScenarioError("no scenario was named")


def parse_viewport(value: str) -> tuple[int, int]:
    try:
        width, height = (int(part) for part in value.lower().split("x", 1))
    except ValueError as error:
        raise ScenarioError(f"--viewport must be WIDTHxHEIGHT, got {value!r}") from error
    if width < 200 or height < 200:
        raise ScenarioError("--viewport is smaller than any OR layout supports")
    return width, height


def main(argv: list[str] | None = None) -> int:
    parser = command_parser()
    args = parser.parse_args(argv)
    report_dir = Path(args.report_dir)
    if not report_dir.is_absolute():
        report_dir = ROOT / report_dir
    try:
        if args.command == "run":
            payload = load_scenario(Path(args.scenario))
            if args.viewport != f"{DEFAULT_VIEWPORT[0]}x{DEFAULT_VIEWPORT[1]}":
                payload["viewport"] = dict(zip(("width", "height"),
                                               parse_viewport(args.viewport)))
        else:
            payload = build_command_scenario(args)
    except ScenarioError as error:
        print(f"or_ui: {error}", file=sys.stderr)
        return 2

    try:
        if args.target == "linux":
            return run_linux(payload, report_dir)
        return run_android(payload, report_dir)
    except Blocked as blocked:
        report_dir.mkdir(parents=True, exist_ok=True)
        (report_dir / "report.json").write_text(json.dumps({
            "harness": "or_ui",
            "target": args.target,
            "scenario": payload.get("name"),
            "result": "BLOCKED",
            "reason": blocked.reason,
            "settle": blocked.settle,
        }, indent=2) + "\n", encoding="utf-8")
        print(f"BLOCKED: {blocked.reason}")
        print(f"  settle with: {blocked.settle}")
        print("RESULT: OR_UI=BLOCKED")
        return 3


if __name__ == "__main__":
    sys.exit(main())
