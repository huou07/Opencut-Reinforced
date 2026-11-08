# Local release evidence, 2026-11-09

Last reviewed: 2026-11-09

This directory records what the 2026-11-09 local release run actually produced on
the three mission hosts. Revision `b8feb51f` is the packaging commit; the app
code it ships is `5a2bef94` plus documentation and the Flatpak packaging files.

## Revision per host

| Host | Path | Revision | Working tree |
| --- | --- | --- | --- |
| macOS M5 | this checkout | `b8feb51f` | clean at build time |
| Debian (`ssh debian`) | `/home/huou07/or-mission` | `5a2bef94` | clean |
| Windows 11 (`ssh win11`) | `C:\Users\hp\src\or-mission` | `5a2bef94` | clean except untracked `.cargo/` |

The hosts were synced with a local git bundle (`git bundle create` +
`git fetch` + `git merge --ff-only`); nothing was pushed.

## Packages

| Package | Size (bytes) | SHA-256 |
| --- | --- | --- |
| `dist/Opencut-Reinforced-0.1.27-arm64.dmg` | 37,512,539 | `60e8906a516cb050bdcdccf37c7a721e503c5d9fa90ae36b0691665b0b3b1c3d` |
| `dist/opencut-reinforced_0.1.27_amd64.deb` (Debian) | 19,632,240 | `360d71b0fb3d61cc5009637a8c6b2e2675c0c7a1a601ac24240dccb531f8e408` |
| `dist/opencut-reinforced-0.1.27-x86_64.flatpak` (Debian) | 19,073,968 | `53c0aa8461450dab79c70a37588972e6a49bceb9eac95e17a730d873cc720619` |
| `apps/or_app/build/windows/Opencut-Reinforced-0.1.27-windows-x64-setup.exe` | 25,323,707 | `df4465670cbff7a0ff631f1005727fe01e14438673276b074d2d38b72ad56ca9` |
| `apps/or_app/build/windows/or-windows-x64-release.zip` | 35,011,015 | `7faa01ad880b127d324c7e9810e7c932d917e1b48078c839b4957e6124c02fe7` |

The macOS DMG is ad-hoc signed (`codesign -dv` reports `flags=0x2(adhoc)`) and
not notarised. `spctl` rejects it, as it does every ad-hoc local build.

The bundled `whisper-cli` is now built by `scripts/build_macos_whisper_cli.sh`
from whisper.cpp v1.9.4 with `CMAKE_OSX_DEPLOYMENT_TARGET=13.0` and a static
build. An earlier attempt reused the debug bundle's helper, which carried
`minos 26.0` and was rejected by the DMG script; the script is the fix, not a
`vtool` patch. The shipped helper reports `minos 13.0` and runs `--help`.

## Installed runs

- **macOS**: the DMG was attached, copied to `/Applications`, and opened. The app
  reported one window titled "Opencut Reinforced" at 1470x923; the screenshot is
  `macos/installed-home.png`. Ad-hoc signing means Gatekeeper needs the usual
  local override.
- **Debian DEB**: `dpkg -i` installed `0.1.27+21ca8876940b` over the previous
  build; `/opt/opencut-reinforced/{or_app,ffmpeg,ffprobe}` are present and the
  binary starts. GUI rendering is **UNVERIFIED** on this host: under Xvfb the GTK
  window stays `10x10+10+10` because the runner shows the window only after
  Flutter renders, and Weston headless screenshots fail
  (`screenshot_create_shm_buffer: Assertion 'width > 0'`). The app-level workflow
  is still covered by the harness run below.
- **Debian Flatpak**: `flatpak install --user` succeeded and `flatpak info`
  reports version 0.1.27, 65.4 MB installed, runtime `org.gnome.Platform` 49.
  Launch is **BLOCKED** by the host: `flatpak run io.github.huou07.or_app`
  prints `error: opendir(/): Permission denied`, and the stock runtime fails
  identically (`flatpak run --user --command=sh org.gnome.Sdk/x86_64/49 -c "ls /"`).
  Bare `bwrap` fails the same way. This Debian host is itself a Docker container,
  so nested user-namespace sandboxes cannot be created here; it is not an OR
  packaging defect.

## UI harness runs

`scripts/or_ui.py` drives the real app by semantics; a debug Linux build under
Xvfb is not an installed-package result.

- **Debian**: `python3 scripts/or_ui.py --target linux run scripts/ui_scenarios/assistant-plan.json`
  → `RESULT: OR_UI=PASS` (27 steps). Evidence: `debian/report-assistant-plan.json`,
  `debian/shot-assistant-plan-applied-26.png`,
  `debian/tree-assistant-plan-after-apply-24.txt`.
- **Windows**: the same scenario reached step 20 (project, import, plan review)
  and failed at the apply tap with
  `Exception: No Impeller context is available` raised by the image resource
  service while resolving timeline thumbnails. Reproduced twice. The SSH login
  lands in Windows session 0 (`tasklist` shows `or_app.exe ... Services 0`), which
  has no interactive desktop or GPU context, so this is **BLOCKED** on the host
  rather than a pass; the pre-apply steps are real UI steps in the real app.
- **Windows packages**: the release lane (`run_windows_lane.ps1 -Mode release
  -Steps build,stage,zip,installer`) reported `WINDOWS_LANE=PASS`, including the
  desktop ZIP install/launch/replacement/removal check. The setup installer was
  run with `/VERYSILENT`; `%LOCALAPPDATA%\Programs\Opencut Reinforced` reports
  file version `0.1.27+28` and contains `or_app.exe`, `or.exe`, `ffmpeg.exe`.
  The installed app starts in session 0; it cannot show a window there.
- **macOS**: the same scenario passes on the debug build (see
  `docs/features/assistant-plans.md`); the installed DMG cannot be driven by the
  harness, which only launches the Flutter test host.
