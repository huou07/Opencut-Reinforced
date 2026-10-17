# Agent authority on this host

Date: 2026-10-17. Source: the owner's direct instruction in the AEH session of
that date. This file exists so a later agent does not have to ask again. It
grants no host privilege, no credential beyond what is named here, and it does
not override the evidence rules in `AGENTS.md`, `docs/TESTING.md` or
`docs/UX_ACCEPTANCE.md`.

## Granted without further approval

- Cohesive commits on the working branch, and pushes to `origin`
  (`huou07/Opencut-Reinforced`). Prefer one reviewed commit per coherent change.
- Opening and updating pull requests, and letting required CI decide.
- Running the local lanes on this host: `bash scripts/check-repo.sh`,
  `cargo test`, and `python3 scripts/run_linux_ui_preflight.py`.
- Publishing a stable release only after the exact-source CI, version, artifact,
  health and rollback gates in `docs/RELEASE.md` pass.
- Cleaning up leftovers from the 2026-10 migration: stale paths, orphaned
  untracked files and superseded tool references may be committed or removed
  when the current source supports it; record what changed and why.
- Installing project-local build tooling and using the owner's recovered shared
  toolchains (`~/.local/share/flutter`, `~/.rustup`, `~/.cargo/bin`,
  `~/.local/share/playwright-cli`).

## Not granted

- Host `sudo`, host account/group changes, host ACL changes, or production
  service restarts outside this project's own release procedure.
- Reading or copying owner credentials. Only the exact GitHub CLI files the
  owner approved are readable, and only for publishing this repository.
- Force-pushing `main`, rewriting tags, or discarding unrelated work.
- Claiming Android, macOS or Windows evidence from this host.

## Verified facts about this host (do not re-derive)

| Fact | Value |
|---|---|
| Flutter | 3.47.7 at `~/.local/share/flutter`; CI pins 3.47.5, so a local pass is developer evidence, not exact-CI evidence |
| Rust | `1.98.0` is installed and is what `crates/or_app_bridge/rust-toolchain.toml` declares; `stable` is also 1.98.0 |
| `~/.cargo` | **not writable** by the worker. Builds use the preflight's project-local offline cache; adding a new dependency needs a writable `CARGO_HOME` |
| Linux UI preflight | `python3 scripts/run_linux_ui_preflight.py` is the supported entry point. A raw `flutter test` fails because rustup then tries to install the macOS targets declared in the bridge manifest; the preflight narrows that list to the host target and restores it byte-for-byte |
| Shared Flutter hook cache | `apps/or_app/.dart_tool/hooks_runner/shared/**` is shared between worker UIDs. Artifacts built by another worker are unreadable and cargo fails with `failed to link or copy … build-script-build`; delete that cache so it is rebuilt by the current UID |
| Android | **unavailable**: no SDK, NDK, `adb`, AVD or emulator on this host, and this UID is not in the `kvm` group. Do not run or claim the Android lane here; use hosted CI |
| GitHub | `gh` is authenticated as `huou07` through the owner-approved file exception; `git push` and release publication work |

## Evidence discipline

Local runs are developer evidence: they prove source behaviour on this host for
this tree, not packaged acceptance. The authoritative gate is the hosted CI run
for the exact commit SHA. Every claim keeps its class — VERIFIED_SOURCE,
VERIFIED_BEHAVIORAL, VERIFIED_SEMANTIC, VERIFIED_NATIVE — and no class promotes
another automatically. A local pass never substitutes for a skipped hosted job.
