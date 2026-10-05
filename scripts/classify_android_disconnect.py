#!/usr/bin/env python3
"""Classify an Android hosted-acceptance disconnect before reporting it.

The platform-verification Android job already preserves driver, guest logcat,
health and post-drive state logs. When `flutter drive` fails, a bare non-zero
exit says nothing about *why*, which is what made earlier failures ambiguous.

This reads only the preserved logs and names the failure class with the exact
evidence lines that produced it. It never judges product acceptance: it
discriminates the disconnect cause so a real product defect is not confused
with emulator, driver or harness instability.

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
    ), ("state", "health")),
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
    matched, evidence = [], {}
    for name, patterns, sources in CLASSES:
        hits = []
        for pattern in patterns:
            regex = re.compile(pattern, re.MULTILINE)
            for source in sources:
                for line in texts.get(source, "").splitlines():
                    if regex.search(line):
                        hits.append({"source": source, "pattern": pattern, "line": line.strip()[:300]})
        if hits:
            matched.append(name)
            evidence[name] = hits[:5]
    started = any(re.search(NEVER_STARTED, texts.get(source, ""))
                  for source in ("guest", "driver"))
    primary = matched[0] if matched else ("PASS" if started else "NEVER_REACHED_APP")
    return {
        "disconnect_classified": bool(matched),
        "primary_class": primary,
        "all_classes": matched,
        "vm_service_announced": started,
        "evidence": evidence,
        "note": ("classification explains the disconnect only; it is not product "
                 "acceptance evidence"),
    }


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