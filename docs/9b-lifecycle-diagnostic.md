# Diagnostic only Android lifecycle experiment

Baseline native/product source: f2b737ba1ac9d7fbaf6c8cb25217d06b14aa9480.
This branch changes only the hosted harness. It cannot supply checkpoint evidence.

The first driver process retains the complete diagnostics and local-file surface
assertions and executes surface assertions twice. The identical assertions in a distinct compiled second target
force an APK replacement and driver install/start tests the lifecycle transition. The original surface target runs
first on a fresh emulator to discriminate target-specific startup. Full guest
logcat, host emulator process state, ADB, app exit reasons, memory and frame
stats are preserved. No retries or timeout increases are used.

Prediction: both first-process controls pass while the repeated lifecycle fails
before assertions; crash/ANR/system events discriminate product and guest causes.

First dispatch 37100135054 / 818b146 did not launch a driver: installing
an app that had never launched left files/ absent, so fixture cp failed.
Runtime discrimination is NOT RUN for that dispatch. The corrected harness
creates files/ through run-as before staging. It records any ADB-blocked
control explicitly and still runs the fresh-AVD surface-first control; this
is not a retry. Host memory/pressure/process observations are also retained.
