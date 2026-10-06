#!/usr/bin/env python3
"""Classify an Android hosted-acceptance disconnect before reporting it.

The platform-verification Android job already preserves driver, guest logcat,
health and post-drive state logs. When `flutter drive` fails, a bare non-zero
exit says nothing about *why*, which is what made earlier failures ambiguous.

This reads only the preserved logs and names the failure class with the exact
evidence lines that produced it: emulator instability, native crash, JNI crash,
app crash, main-thread stall, VM-service failure, driver lifecycle, or a test
that failed with no crash signature at all. It never judges product acceptance:
it discriminates the disconnect cause so a real product defect is not confused
with emulator, driver or harness instability. A failed test is never reported as
PASS.

Deterministic and unit-tested. No LLM, no network, no device access.
"""
from __future__ import annotations

import argparse
import json
import os
import re
import sys

# Order matters: an emulator or native failure explains a disconnect, so it must
# outrank the generic driver/VM symptoms that follow from it.
CLASSES = (
    ("EMULATOR_INSTABILITY", (
        r"adb:\s*no devices/emulators found",
        r"device offline",
        r"device 'emulator-5554' not found",
        r"^\s*emulator-5554\s+offline",
        r"adb:\s*error:\s*cannot connect to daemon",
        r"error:\s*closed",
        r"Timeout waiting for the emulator device",
        # `flutter drive` prints these when the emulator is gone; without them a
        # lost emulator is misread as a driver or product failure.
        r"No supported devices found with name or id matching",
        r"The following devices were found:",
    ), ("state", "health", "driver")),
    ("NATIVE_CRASH", (
        r"\*\*\* \*\*\* \*\*\* \*\*\*",
        r"Fatal signal \d+",
        r"signal \d+ \(SIG",
        r"backtrace:",
        r"Abort message:",
    ), ("guest", "state")),
    ("JNI_CRASH", (
        r"JNI DETECTED ERROR",
        r"CheckJNI",
        r"art::Thread::ThrowPendingException",
        r"Java_vm_ext::",
        r"libc:\s*Fatal abort",
    ), ("guest", "state")),
    ("APP_CRASH", (
        r"FATAL EXCEPTION",
        r"AndroidRuntime:\s*FATAL",
        r"Process io\.github\.huou07\.or_app .* has died",
    ), ("guest", "state")),
    ("MAIN_THREAD_STALL", (
        r"ANR in io\.github\.huou07\.or_app",
        r"Application is not responding",
        r"Skipped (\d{3,}) frames",
    ), ("guest", "state")),
    ("VM_SERVICE_FAILURE", (
        r"Dart VM service is not listening",
        r"Failed to connect to the Dart VM service",
        r"VM service.*terminated",
        r"lost connection to device",
    ), ("driver", "guest")),
    ("TEST_ASSERTION_FAILURE", (
        r"Some tests failed",
        r"Test failed\. See exception logs above",
        r"EXCEPTION CAUGHT BY FLUTTER TEST FRAMEWORK",
        r"\+\d+ -\d+: ",
        r"The test description was:",
    ), ("driver",)),
    ("DRIVER_LIFECYCLE", (
        r"Gradle build failed",
        r"could not find an option named",
        r"No application found for TargetPlatform",
        r"Unable to find a connected device",
        r"Test failed to load",
        r"flutter drive.*exited",
    ), ("driver",)),
)

# A guest log that never announced the VM service never reached product code.
NEVER_STARTED = r"VM service is listening"

# Classes that describe OR's own process. The guest also logs Choreographer and
# ANR lines for Play services and other emulator processes; attributing those to
# OR would blame the product for an emulator problem, which is the exact
# confusion this classifier exists to prevent.
APP_SCOPED = {"NATIVE_CRASH", "JNI_CRASH", "APP_CRASH", "MAIN_THREAD_STALL"}
OR_PACKAGE = "io.github.huou07.or_app"

# logcat threadtime prefix: "MM-DD HH:MM:SS.mmm  PID TID LEVEL tag: message"
THREADTIME_PID = re.compile(r"^\s*\d{2}-\d{2}\s+\d{2}:\d{2}:\d{2}\.\d+\s+(\d+)\s+(\d+)\s")
# in-process tag form: "I/Choreographer( 3304): ..."
TAGGED_PID = re.compile(r"\(\s*(\d+)\)")


def or_pids(texts):
    """PIDs attributable to OR, learned from lines that name the OR package."""
    pids = set()
    for source in ("guest", "driver", "state", "health"):
        for line in texts.get(source, "").splitlines():
            if OR_PACKAGE not in line:
                continue
            match = THREADTIME_PID.match(line) or TAGGED_PID.search(line)
            if match:
                pids.add(match.group(1))
    return pids


def _belongs_to_or(line, pids):
    """True when the line is OR's own, not another guest process."""
    if OR_PACKAGE in line:
        return True
    if not pids:
        # No PID was ever learned; refusing to attribute anything is the only
        # honest answer, so only package-bearing lines qualify.
        return False
    match = THREADTIME_PID.match(line) or TAGGED_PID.search(line)
    return bool(match) and match.group(1) in pids


def _read(path):
    if not path or not os.path.isfile(path):
        return ""
    try:
        with open(path, "r", encoding="utf-8", errors="replace") as handle:
            return handle.read()
    except OSError:
        return ""


def collect(paths):
    """paths: {'driver': p, 'guest': p, 'state': p, 'health': p} -> {source: text}"""
    return {name: _read(paths.get(name)) for name in ("driver", "guest", "state", "health")}


def classify(texts):
    pids = or_pids(texts)
    matched, evidence = [], {}
    for name, patterns, sources in CLASSES:
        scoped = name in APP_SCOPED
        hits = []
        for pattern in patterns:
            regex = re.compile(pattern, re.MULTILINE)
            for source in sources:
                for line in texts.get(source, "").splitlines():
                    if not regex.search(line):
                        continue
                    if scoped and not _belongs_to_or(line, pids):
                        continue
                    hits.append({"source": source, "pattern": pattern, "line": line.strip()[:300]})
        if hits:
            matched.append(name)
            evidence[name] = hits[:5]
    started = any(re.search(NEVER_STARTED, texts.get(source, ""))
                  for source in ("guest", "driver"))
    if matched:
        primary = matched[0]
    elif started:
        # No crash, stall or emulator signature, but a clean drive would say so.
        primary = "FAILURE_WITHOUT_DISCONNECT_SIGNATURE" if _tests_failed(texts) else "PASS"
    else:
        primary = "NEVER_REACHED_APP"
    return {
        "disconnect_classified": bool(matched),
        "primary_class": primary,
        "all_classes": matched,
        "vm_service_announced": started,
        "or_pids": sorted(pids),
        "evidence": evidence,
        "note": ("classification explains the disconnect only; it is not product "
                 "acceptance evidence"),
    }


def _tests_failed(texts):
    patterns = next(p for name, p, _ in CLASSES if name == "TEST_ASSERTION_FAILURE")
    driver = texts.get("driver", "")
    return any(re.search(pattern, driver, re.MULTILINE) for pattern in patterns)


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--driver")
    parser.add_argument("--guest")
    parser.add_argument("--state")
    parser.add_argument("--health")
    parser.add_argument("--case", default="unknown")
    parser.add_argument("--output", help="write the JSON result here as well")
    args = parser.parse_args(argv)
    if not any((args.driver, args.guest, args.state, args.health)):
        parser.error("provide at least one preserved log")
    result = classify(collect({"driver": args.driver, "guest": args.guest,
                               "state": args.state, "health": args.health}))
    result["case"] = args.case
    payload = json.dumps(result, indent=2, sort_keys=True)
    print(payload)
    if args.output:
        os.makedirs(os.path.dirname(os.path.abspath(args.output)) or ".", exist_ok=True)
        with open(args.output, "w", encoding="utf-8") as handle:
            handle.write(payload + "\n")
    return 0


if __name__ == "__main__":
    sys.exit(main())