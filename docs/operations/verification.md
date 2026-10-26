# Verification and Evidence

Last reviewed: 2026-10-26

Scope: which check answers which question, and how to report the result honestly.
Pick the cheapest check that answers the question, then exercise the real path
needed for the claim.

## Evidence classes

| Class | Means |
|---|---|
| IMPLEMENTED | the source change exists |
| VERIFIED_SOURCE | relevant source checks or automated tests passed |
| VERIFIED_BEHAVIORAL | real app interaction and resulting canonical state were observed |
| VERIFIED_VISUAL | a named real-app view was inspected at a stated size and state |
| VERIFIED_NATIVE | a built macOS app was launched on this host (name debug or packaged) |
| UNVERIFIED / BLOCKED / FAILED / NOT RUN | must stay explicit |

No class promotes another. A build is not a journey; a widget test is not a
journey; a screenshot is not a journey; a passing export job is not a valid file.

## Check ladder

| Question | Check |
|---|---|
| Does the repository satisfy its own rules? | `bash scripts/check-repo.sh` |
| Do Rust contracts hold? | `cargo test -p <crate>` (focused first) |
| Do Flutter contracts hold? | `cd apps/or_app && flutter analyze --no-pub && flutter test --no-pub` |
| Does the real UI do the thing? | `python3 scripts/or_ui.py run scripts/ui_scenarios/<name>.json` |
| Does the built app work? | launch the built bundle and drive its window; see [local-macos.md](local-macos.md) |
| Is the exported file valid? | independent probe/playback of the artifact (`ffprobe`, decode) |
| Is it fast enough? | measured workload with recorded build, fixture, backend and machine |

A missing tool is `LOCAL ENVIRONMENT BLOCKED`, never a source failure.

## Reporting

Report the exact check, build, source revision, artifact and unverified states.
Local Mac evidence is not a hosted cross-platform claim. Hosted CI remains the
authority for native and packaged claims on other platforms.

## Related

- [local-macos.md](local-macos.md)
- [ui-harness.md](ui-harness.md)
- [reference/quality.md](../reference/quality.md)
- [reference/testing.md](../reference/testing.md)
