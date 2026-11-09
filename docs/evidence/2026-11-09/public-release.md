# Public release evidence — 0.1.27

What was checked before the 0.1.27 tag, on branch `main` at `dc829d7c`. Every
row is a command that ran on the host in the column, not a claim about another
build.

## Revisions

| Host | Branch | Commit |
| --- | --- | --- |
| macOS (Apple M5) | `main` | `dc829d7c` |
| Debian (Intel i5-13500T, Intel UHD 770) | `main` | `dc829d7ccc8d` |
| Windows 11 (Intel Core 5 120U) | `main` | `dc829d7c` |

`main` was fast-forwarded from `97f4f25e` to the mission branch at `a98341bb`
with `git branch backup/main-pre-merge-2026-11-09` and a bundle backup kept
first; nothing was reset or rewritten.

## Source gate, per host

| Check | macOS | Debian | Windows |
| --- | --- | --- | --- |
| `cargo fmt --all -- --check` | PASS | PASS | PASS |
| `cargo test --workspace` | PASS | PASS | PASS |
| `cargo clippy --workspace --all-targets` | PASS | PASS | PASS |
| `flutter analyze --no-pub` | PASS | PASS | PASS |
| `flutter test --no-pub` | PASS 621 | PASS 621 | PASS 614 + 7 skipped |
| `bash scripts/check-repo.sh` | PASS (1017 files) | — | — |
| `gitleaks detect --source .` | PASS, 1544 commits, no leaks | — | — |

The seven skipped Windows tests are the symlink cases in
`apps/or_app/test/source_path_test.dart`: the SSH account cannot create
symlinks (`errno = 1314`), so the tests carry a skip reason instead of failing.

## UI journeys at the release commit

macOS, all twenty-two shipped scenarios `RESULT: OR_UI=PASS`:

`editor-journey`, `text-journey`, `keyframes`, `motion-scene-edit`,
`audio-cleanup`, `reduce-noise`, `save-reopen`, `reopen-twice`, `export-encode`,
`export-ready`, `export-text`, `companion-provenance`, `companion-lottie`,
`companion-gltf`, `companion-tools-catalog`, `assistant-plan`,
`style-assistant-plan`, `creative-direction-plan`, `templates-edit-style`,
`performance-profiles`, `guided-beginner`, `compact-editor-journey`,
`caption-import`.

Windows, interactive desktop session 1 through a scheduled task:
`editor-journey` PASS at `dc829d7c` after installing the freshly built setup,
plus `companion-provenance` PASS at `65f88f71` (the only Dart change after that
is the motion starter strip, which Windows journeys do not visit and which
`flutter test` covers).

Debian: `companion-provenance`, `companion-lottie`, `companion-gltf`,
`companion-tools-catalog` PASS at `65f88f71`; the release build itself is gate
checked by `scripts/run_local_lane.sh linux --packaged --steps deb,journey`.

## Defects found by this gate and fixed

1. **Media Foundation small canvases** — a 32×32 MP4 export reported "this
   machine has no working H.264 encoder" on a host that had just written three
   MP4s at 128, 64 and 48 px. The probe opens candidates at 64×64 and the plan
   was size-blind. `crates/or_core/src/hardware.rs` now carries
   `h264_canvas_floor` / `apply_canvas_floor`, the plan names the floor, and the
   open-failure message no longer blames the host. Three tests added.
2. **Motion panel overflow** — `A RenderFlex overflowed by 16 pixels` at
   320×418.8 on Linux; the four starter cards are now a chip strip inside the
   scrolling list, so the asset rows keep their viewport. This also fixed a
   regression the change itself caused in `motion-scene-edit`.
3. **Windows licence texts** — the ZIP shipped FFmpeg, libvpx, Opus and
   whisper.cpp binaries with no licence files. `scripts/stage_windows_bundle.ps1`
   stages six files and throws when one is missing; the notice text moved to
   `scripts/ffmpeg-notice.txt` so both packagers write one statement.
4. **Two Windows-only test-harness defects** — a `file://` URI built the wrong
   way in a storage test, and a recent-projects test reading `HOME` on a
   platform that sets `USERPROFILE`.
5. **Stale native-assets temp target** — `flutter test` failed with
   `couldn't read …\libsqlite3-sys-*\out/bindgen.rs` until
   `%TEMP%\frb_native_assets_*` was removed. A host artefact, not a repository
   defect; noted so the next reader does not chase it.

## Packages, built from `dc829d7c`

| Artifact | Bytes | SHA-256 |
| --- | --- | --- |
| `Opencut-Reinforced-0.1.27-arm64.dmg` | 37,528,006 | `22171092ad11232ca192a60495d28ab48e198837082c7723390b1142878f8b8f` |
| `Opencut-Reinforced-0.1.27-linux-x64.deb` | 23,439,512 | `a087d88d285a387bb5154f4c1bbd572cada68ff843c58b0aaaee77f6db1192f8` |
| `Opencut-Reinforced-0.1.27-x86_64.flatpak` | 22,592,440 | `3d36aafc1ffd068ddcfc39eb6f081d82672faac842ed19277d558939c1f0a25c` |
| `Opencut-Reinforced-0.1.27-windows-x64-setup.exe` | 25,420,752 | `37d323fd2c259b4a26688683d504cc72550d95db358266ef2fce419eaaa99a66` |
| `Opencut-Reinforced-0.1.27-windows-x64.zip` | 35,073,186 | `b4b8ee9b82966689f912afd40453b31215d7d877ad1d544b0af01f7b80db272e` |

## Install and smoke, per host

| Host | Install | Smoke |
| --- | --- | --- |
| macOS | DMG mounted, `hdiutil verify` VALID, app copied to `/Applications` | `or version` → `Opencut Reinforced 0.1.27`; `or tools list` shows the ten catalog entries |
| Debian | `sudo dpkg -i` upgraded `0.1.27+436a1c0368c6` → `0.1.27+dc829d7ccc8d` | `/opt/opencut-reinforced/or version`, `or tools list` |
| Windows | setup `/VERYSILENT /SUPPRESSMSGBOXES /NORESTART`, exit 0 | `or version`, `or tools list`, `editor-journey` PASS |
| Flatpak | `flatpak install --user --reinstall` → "Changes complete" | `flatpak info --show-permissions` shows `sockets=x11;wayland;pulseaudio;fallback-x11`, `devices=dri`; `flatpak run` **BLOCKED** by the host |

Flatpak runtime is BLOCKED, not deferred: `bwrap: Can't mkdir parents for
/proc: Permission denied` and `error: opendir(/): Permission denied` occur for
the stock `org.gnome.Sdk` control too, and `/proc/self/status` reports
`CapEff: 0000000000000000` because the Debian host is itself a container. The
package builds, installs and declares its permissions; its sandbox was never
observed running here.

## What is still open

- Flatpak runtime UNVERIFIED pending a host that can create user namespaces.
- NVIDIA/AMD renderer rates UNVERIFIED (no such hardware); the no-device and
  broken-enumeration fallbacks are measured.
- Decoding 4K natively and scaling in the compositor would remove 10–19 ms per
  frame from 4K→1080p exports; recorded with a recommendation, deliberately not
  changed in this release.
