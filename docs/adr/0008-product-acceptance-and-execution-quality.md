# ADR 0008 — Product acceptance and execution quality

- Status: Accepted
- Date: 2026-10-03

## Context and freeze audit

Phase 8 completion showed that green bridge and synthetic checks can coexist
with a desktop package whose normal import and derived-media paths call host
`ffprobe`/`ffmpeg`, whose macOS external-file permissions need packaged
verification, whose preview and export geometry can diverge, and whose decoder
is reconstructed in realtime paths. Phase 9B then accumulated corrective CI
commits without proving Android SAF preview through the actual product. The
9B Android gate failed repeatedly after successful cross-build, native link,
APK packaging, and sometimes a passing first Flutter driver invocation.

The frozen audit baseline is 9A completion commit
`8925a1fbc3d89863f9519302910e219d6c36b5fb`; the pre-amendment 9B HEAD
is `5b6a496e5ebd27252677fc9b6bbcfd06423c7e1d`. `HEAD` and
`origin/main` matched and the worktree was clean. 9B remained `NEXT`, 9C
`PLANNED`, versions 7/1/1, and no 9B completion evidence existed. The complete
diff covered the Android preview bridge, JNI/SurfaceProducer adapter, Flutter
SAF registration, integration tests, the platform workflow, and related docs.

The corrective history is classified below. A commit with mixed content has
multiple classifications; classification describes the change, not its
author's intent.

| Commit | Classification | Audit result |
| --- | --- | --- |
| `0ce44f5` | PRODUCT_FIX, VALID_TEST_FIX | Introduced bounded software preview and Android surface; its local-file fixture did not prove SAF. |
| `1a3b6dc` | PRODUCT_FIX, VALID_TEST_FIX | Passed packaged FFmpeg configuration into the Android bridge hook. |
| `c9dce27` | PRODUCT_FIX | Corrected Android Kotlin/JVM build target. |
| `9f922c5` | PRODUCT_FIX | Corrected `SurfaceProducer.id()` invocation. |
| `684688c` | PRODUCT_FIX | Corrected JNI frame byte-count type. |
| `7075b61` | OBSOLETE_RETRY_WORKAROUND, DIAGNOSTIC_ONLY | Added clean-AVD retries and offline logging without a discriminating root-cause test. |
| `bb2fe06` | VALID_TEST_FIX | Preserved the fixture across driver installation. |
| `7c34f91` | VALID_TEST_FIX | Moved the preview fixture into app-owned storage after driver install. It still did not exercise a content provider. |
| `1bdb44a` | TEST_WEAKENING, OBSOLETE_RETRY_WORKAROUND | Classified pre-test ADB loss as retryable although app/lifecycle causation remained unexcluded. |
| `a2f7252` | PRODUCT_FIX | Used the shared in-process host on Android; desktop IPC behavior remained separate. |
| `d7de23b` | PRODUCT_FIX, VALID_TEST_FIX, OBSOLETE_RETRY_WORKAROUND | Improved presenter scheduling and test waits while continuing lifecycle retry tuning. |
| `5377e1d` | DIAGNOSTIC_ONLY | Bounded one native test wait without changing the failure mechanism. |
| `ad9e17a` | DIAGNOSTIC_ONLY | Bounded remaining waits without changing the failure mechanism. |
| `8b2995b` | OBSOLETE_RETRY_WORKAROUND | Tightened retry classification but retained speculative retry behavior. |
| `56fd47d` | DIAGNOSTIC_ONLY | Captured guest Flutter progress. |
| `7e564a7` | TEST_WEAKENING, SPEC_DRIFT | Used `flutter --ci` to disable macOS sandbox in native tests; it cannot prove sandboxed package access. |
| `5b6a496` | DIAGNOSTIC_ONLY | Verified guest log capture before driver launch. |

The inspected failed Android runs were `36978394060`, `36983768071`,
`36986408916`, `36988473456`, `36990816137`, `36997970520`,
`37000717097`, `37005903291`, `37009313576`, `37013624846`,
`37027986064`, `37031350980`, `37035242863`, `37034330260`,
`37039737354`, and `37043830370`. Their available artifacts were FFmpeg
build archives, not Android crash dumps. Early failures were concrete build
configuration/Kotlin/JNI errors. Later runs repeatedly passed bridge
diagnostics, then a separate driver install/start reached the app and VM
service before `Service connection disposed` and ADB `offline`. The last run
repeated this on clean AVDs and recorded 112–167 skipped frames during app
startup. Some logcat traces show Android stopping the app during driver
lifecycle; others show binder/guest service distress. Those observations do
not distinguish a Flutter-driver install/uninstall interaction from app/native
failure, JNI lifetime failure, emulator failure, or excessive main-thread work.
They require a single-lifecycle discriminating test and startup measurement.

## Decision

The permanent `INV-PRODUCT`, `INV-PACKAGE`, `INV-CAPABILITY`, `INV-PARITY`,
`INV-PERF`, `INV-UX`, `INV-ACCEPT`, and `INV-VERIFY` rules apply to all future
work. A runner is evaluated on working product behavior, not on the smallest
implementation that makes existing tests green. Unit and bridge checks remain
valuable evidence at their actual level.

Every user-visible checkpoint declares machine-validated required evidence
classes. From 9B onward the supervisor records class proofs tied to exact-SHA
hosted workflow jobs and successful named steps. Missing classes, jobs, steps,
or mismatched run IDs fail closed. Historic evidence is retained unchanged;
the new proof schema does not retroactively claim acceptance for Phase 8.
High-fidelity steps must exercise the named product boundary; a renamed echo
or synthetic substitute violates the invariant even if the workflow step
returns success. Reviewer inspection and acceptance tests remain necessary
because machine metadata alone cannot prove semantic quality.

After two speculative corrective attempts for the same subsystem/gate, work
pauses. A further repair needs exact failure evidence, a falsifiable cause,
a reproduction or discriminating test, and a causal explanation of the patch.
The exhausted 9B retry history already triggers this pause. A fresh 9B runner
must perform the discriminating test before another implementation repair.

9B1 is inserted after 9B to own packaged media runtime and acceptance debt.
It does not advance 9B. An explicit guarded amendment baseline records the
old failed SHA and permits only a reviewed control-plane diff to become the
new 9B baseline. Ordinary runner protected-path checks remain in force.

## Consequences

9B remains the Android software fallback gate. 9B1 separately verifies real
packaged desktop import, preview, persistence, export, performance, and clean
environment behavior. Hardware Android acceleration remains optional. This
ADR authorizes no product implementation, schema increment, or completion
evidence by itself.

The amendment's follow-up removes execution tests' dependence on live 9B
`NEXT` state. Historical scenarios reconstruct their own state, and a
regression test repeats the 9B quality checks with live 9B1 `NEXT`. The trusted
baseline may be a contiguous series of control-only commits: each updates its
marker, follows a validated amendment directly, and preserves the locked
plan, execution statuses, and versions. Product commits cannot enter that
series. The original failed implementation and run remain in the first
marker and the immutable Git history.
