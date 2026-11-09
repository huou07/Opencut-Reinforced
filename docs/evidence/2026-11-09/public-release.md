# Public release evidence — 0.1.27

What was checked before the 0.1.27 tag, on branch `main` at `9f86d514`. Every
row is a command that ran on the host in the column, not a claim about another
build. The first pass through this gate ran at `dc829d7c` and found four
failures the tag could not carry — unformatted Dart, a stale probe lock and two
denied clippy lints — so every package below was rebuilt from `9f86d514` after
they were fixed, and installed again.

## Revisions

| Host | Branch | Commit |
| --- | --- | --- |
| macOS (Apple M5) | `main` | `9f86d514` |
| Debian (Intel i5-13500T, Intel UHD 770) | `main` | `9f86d51480a6` |
| Windows 11 (Intel Core 5 120U) | `main` | `9f86d514` |

``main` was fast-forwarded from `97f4f25e` to the mission branch at `a98341bb`
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

macOS, all twenty-three shipped scenarios `RESULT: OR_UI=PASS`:

`editor-journey`, `text-journey`, `keyframes`, `motion-scene-edit`,
`audio-cleanup`, `reduce-noise`, `save-reopen`, `reopen-twice`, `export-encode`,
`export-ready`, `export-text`, `companion-provenance`, `companion-lottie`,
`companion-gltf`, `companion-tools-catalog`, `assistant-plan`,
`style-assistant-plan`, `creative-direction-plan`, `templates-edit-style`,
`performance-profiles`, `guided-beginner`, `compact-editor-journey`,
`caption-import`, `export-formats`.

`export-formats` is the one that proves the two non-MP4 profiles through the
real dialog rather than through the benchmark: the same cut exported as
`export.webm` (`vp9` + `opus`, 1440x1080) and `export.mkv` (`ffv1` +
`pcm_s16le`), both `ffprobe`d from `/tmp/or-export-formats/artifacts/replacements`
after the run. `export-encode` covers MP4 H.264/AAC the same way.

Windows, interactive desktop session 1 through a scheduled task:
`editor-journey` PASS at `9f86d514` after installing the freshly built setup,
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

## Packages, built from `9f86d514`

| Artifact | Bytes | SHA-256 |
| --- | --- | --- |
| `Opencut-Reinforced-0.1.27-arm64.dmg` | 37,528,069 | `1ea20060412969c5f4830c6ac610ec46f8bf2f08f338476204c9aa56b64305e9` |
| `Opencut-Reinforced-0.1.27-linux-x64.deb` | 23,449,884 | `0f374e1c9484ebd076d35b3a4a9f83956fc6e227f8aaa6ff5bdd7d4f9f9496bb` |
| `Opencut-Reinforced-0.1.27-x86_64.flatpak` | 22,578,616 | `2c92391fd73ff49cbdb10669cbd7eaf8e0e257cde3bfd597ecce7e639115ac13` |
| `Opencut-Reinforced-0.1.27-windows-x64-setup.exe` | 25,424,296 | `9542ec687a7d2cf8a26d96a074c8fb46e1c053311761e962c907f78ec959435c` |
| `Opencut-Reinforced-0.1.27-windows-x64.zip` | 35,073,625 | `21df2e7508546ad42e6c96bdb5123d84155010d891bbcfafabd4df9287abf649` |

## Install and smoke, per host

| Host | Install | Smoke |
| --- | --- | --- |
| macOS | DMG mounted, `hdiutil verify` VALID, app copied to `/Applications` | `or version` → `Opencut Reinforced 0.1.27`; `or tools list` shows the ten catalog entries |
| Debian | `sudo dpkg -i` upgraded `0.1.27+49ef86d5…` → `0.1.27+9f86d51480a6` | `/opt/opencut-reinforced/or version`, `or tools list` |
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

## 0.1.28 — the export that could not write where it was told (2026-11-09)
**Uploaded bytes.** CI builds its own copies of the DMG, DEB, setup and ZIP while
the Flatpak is only ever built here. The five assets uploaded to the release are
the ones built, installed and smoke-tested on the three hosts at `5b676133`
(`SHA256SUMS.txt` 537 B, written in dmg/deb/flatpak/setup/zip order), replacing
CI's build with `gh release upload --clobber` so the published bytes are the
tested ones. All five were then re-downloaded from
`https://github.com/huou07/Opencut-Reinforced/releases/download/v0.1.28/…` and
checked with `shasum -a 256 -c SHA256SUMS.txt` (five `OK`).


Reported from a real session: an export ended with `Operation not permitted`,
and `ffmpeg` was blamed because that is the only name the message contained.

**Cause.** The writer stages a hidden `.name.or-export-<pid>-<n>` directory
beside the destination and renames the finished file into place
(`crates/or_media/src/encoder.rs::create_staging_directory`). When the folder
refused that new directory the error was formatted as `could not create export
staging directory: Operation not permitted (os error 1)` — no folder, no reason
and nothing to do. macOS withholds Desktop, Documents, Downloads and external
volumes from an app until the owner allows it, and this build is ad-hoc signed,
so every rebuild can look like a new app to that decision; the panel also opened
wherever it was last used, which is often one of those folders. The app itself
was never sandboxed: `codesign -d --entitlements -` on the shipped bundle shows
no `com.apple.security.app-sandbox`.

**Fix** (commit `0fe5fa8f`, portable-helper follow-up `5bf96750`):

- `describe_staging_failure` names the folder and the remedy, with macOS
  pointing at System Settings and the other platforms at a folder under home.
- `check_export_destination` runs in `start_export` before the job is queued: it
  rejects a directory, refuses a destination file it cannot open for writing
  without truncating it, and proves the folder accepts a new directory by
  creating and removing one. A refusal returns
  `EXPORT_DESTINATION_NOT_WRITABLE` while the dialog is still on screen.
- The macOS save panel starts in `~/Movies`; its return value grants nothing on
  its own, and the stale comment claiming otherwise was corrected.
- A failed export repeats its explanation as a notice (12 s) with a close, since
  the top bar keeps one short line and drops the text on a compact layout.
- Four Rust tests (`destination_tests`) cover the writable case, the write-denied
  folder, a read-only destination file, and the writer's own first-failure
  message.

**Proof through the UI, not the API.** New journey
`scripts/ui_scenarios/export-blocked-destination.json` (23 steps) hands the
export a folder the runner made `0500` via the new scenario flag
`exportWriteDenied`, then waits for the folder-naming sentence and the remedy in
the live semantics tree before capturing the tree and a screenshot. PASS on
macOS (`/tmp/or-export-blocked2`) and on Debian (`/tmp/or-deb-export-blocked-destination`);
on Windows the flag has no `chmod` equivalent, so the journey is not run there.
`export-encode` and `export-formats` re-ran green in the same session, so the
fix did not disturb a working export.

**Build at `5bf9675042`** (all three hosts fast-forwarded to it by bundle):

| Asset | Bytes | SHA-256 |
| --- | --- | --- |
| `Opencut-Reinforced-0.1.28-arm64.dmg` | 37,523,733 | `cdec3da6fc00df4d80a8878c88448e6ff82aa82d4131d6cfa9537d2cbfb27e3e` |
| `Opencut-Reinforced-0.1.28-linux-x64.deb` | 23,443,724 | `2fcb2c117b4d8bde250234d1472c4dc0563764571fc14a8ea69e7de1253c1b38` |
| `Opencut-Reinforced-0.1.28-x86_64.flatpak` | 22,595,256 | `ee03812c77cc6c663eaf46b728f28a4678d31ded8ddcb8a8381bed6439261205` |
| `Opencut-Reinforced-0.1.28-windows-x64-setup.exe` | 25,421,080 | `4b1206baf1808c1d98fcf067737fb4001df5df31588b1bb2d33662d180a57afd` |
| `Opencut-Reinforced-0.1.28-windows-x64.zip` | 35,076,362 | `4f435926c93e5d273aa0f40e8230ffa5a7fa9acfca444ef08fc91526ff96e870` |

Lane results at that commit: macOS `cargo test` suites green, `flutter test`
`+621: All tests passed!`, `flutter analyze` clean, `cargo clippy -D warnings`
clean; Debian `RESULT: LOCAL_LANE=PASS` (release DEB) plus the Flatpak build;
Windows `WINDOWS_LANE=PASS` (build, stage, ZIP install/replacement/removal,
setup). `bash scripts/check-repo.sh` passes.

**A defect this release nearly shipped.** The first build of the fix failed on
Windows only: the new helpers were inserted between `#[cfg(unix)]` and the
function it guards, so `or_media` did not compile there (`the name
create_private_directory is defined multiple times`, `cannot find unix in os`).
The macOS lane and every journey were green at that commit; only the Windows
lane caught it. `5bf96750` restores the attribute and words the remedy per
platform. Both hosts were rebuilt afterwards, which is why the hashes above
belong to `5bf96750` and not `0fe5fa8f`.

**Installed and smoke tested at `5bf96750`:** macOS DMG → `/Applications`,
`or version` → `Opencut Reinforced 0.1.28`, window `1470x923`, screenshot
`docs/evidence/2026-11-09/macos/installed-0.1.28.png`; Debian `dpkg -i` →
`/opt/opencut-reinforced/or version` 0.1.28; Flatpak `Changes complete.` with
Version 0.1.28 and the same permissions, runtime still BLOCKED by the host;
Windows setup exit 0 → `Opencut Reinforced 0.1.28`, desktop screenshot
`docs/evidence/2026-11-09/windows/installed-0.1.28.png` (1536x864).

## 0.1.29 — the export that failed for a reason nobody could see

The owner reported an export that stopped working after 0.1.28, with
`/Users/meomeo/Movies/13.orproj` as the failing project. 0.1.28 had explained a
filesystem refusal; this one was not a filesystem refusal at all.

**Reproduction.** The project opens through the UI harness as a `copy:` fixture:
93 media, one video track with 34 FFV1 Matroska clips, two audio tracks, one
caption track with 194 clips. Exporting it printed
`FFmpeg operation failed: Operation not permitted`. Narrowing by track kind left
only the video path failing, and a single imported FFV1 clip reproduced it.
Instrumenting the export put the failure in `prepare_export_audio_cache`, and a
unit-level reproduction put it in `decode_audio` → `seek_to_time`.

**Cause.** The renders declare `pcm_s16le` with `duration` but carry **zero
audio packets**; `ffprobe -count_packets` reports `N/A`. Matroska indexes only
video keyframes, so `av_seek_frame(stream = 1)` answered `-1`, and FFmpeg renders
that bare failure as errno 1 — "Operation not permitted".

**Fix** (`773a499f`). The decoder seeks the container time base and falls back to
decoding from the start, so the empty declared track reads as silence;
`seek_to_pts` keeps the precise stream-level attempt first so the exact-range
test still holds. A fixture (`crates/or_media/tests/fixtures/or-ffv1-empty-audio.mkv`,
SHA-256 `26f20eb001a9e663dfc052aa6317a3cac338c28dc4b101b2a25b8c7ca0b4cb46`) and
`software_decoder_treats_a_track_with_no_audio_packets_as_silence` hold it.

**Proof on the owner's project.** `RESULT: OR_UI=PASS` in 95 s through the
editor's own Export flow; the file is 43,001,495 B and `ffprobe` reports
`h264 1920x1080 30/1`, `9000` packets, `aac` with `14064` packets,
`duration=300.01`.

**Also in 0.1.29:**
- Windows opened every target through `cmd /c start`, so an update could land in
  the "how do you want to open this file?" dialog; installers and executables now
  start directly and documents go to the protocol handler
  (`apps/or_app/lib/support/host_platform.dart` `openCommandFor`, covered by
  `apps/or_app/test/host_platform_test.dart`).
- The assistant dialog reported "not found" for a harness CLI whose ACP adapter
  was missing; it now names the CLI and the adapter command, and detects on open.
- Companion tools gained **Set up here** / **Uninstall**
  (`apps/or_app/lib/updates/companion_setup.dart`) and `or tools setup|remove`.

**Gates at `bd6f6f11`.** macOS: `cargo fmt --check`, `cargo clippy -D warnings`,
whole-workspace `cargo test`, `flutter analyze`, 628 Flutter tests,
`check-repo.sh` — all pass; Debian and Windows ran the same Rust + Flutter lanes.
Journeys on macOS at this revision: `companion-tools-catalog`, `export-encode`,
`editor-journey`, `companion-lottie`, `companion-provenance`,
`motion-scene-edit` and the owner-project export all PASS. Two intermediate
failures were environmental and are recorded here so the next reader does not
misread them: a second app instance (`/Applications/.../or_app`) stole focus from
the harness build, and two journey runs overlapped and locked
`build/macos/Build/Intermediates.noindex/XCBuildData/build.db`.

**Artifacts** (all built at `bd6f6f11`, installed and started on their host):
DMG 37,530,096 B `583c09962076e5f12ee52fada58582fb6ce148ab5e0d27b2b5265220f0081c44`;
DEB 23,456,576 B `202d31805ed10d8734bd1c95c1c897af5cd8c9f796801aaee621b2e20cb2b3b2`
(`0.1.29+bd6f6f11c49b` installed); Flatpak 22,587,256 B
`07ada7796315c9bb697469ad418b66e1cffa4cc7c8bfe771c1073be672a3eb95` (install PASS,
runtime BLOCKED by the host); setup 25,432,095 B
`11b6e97ddabbe82cc95c043f553dc416280d31a3caeec9d4c3aae3edbb29c9a3` (`INSTALL_EXIT=0`,
`or version` → 0.1.29); ZIP 35,089,008 B
`c2dc8d8d63d7c622a1ae072082591da2767fc325c3cb157e655a6aaa9771dc24`.
