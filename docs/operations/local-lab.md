# Local Lab Runbook

Last reviewed: 2026-11-08

Scope: the three machines this project is developed and measured on — how to
reach each one, start a real GPU GUI session, drive the app, benchmark, build,
package, install and check a release. Every command here ran on the named host at
`552cd16b` (optimization work continues on the same branch); commands that were
not re-run are marked. Read this before investigating a host problem: a workflow
that "does not work" is usually a missing environment variable, not a broken
capability.

No credentials, keys or tokens belong in this file. Access is by host alias in
`~/.ssh/config` and existing keys; nothing here provisions access.

## Hosts

| Alias | Machine | Repo | Media path |
|---|---|---|---|
| (local) | MacBook Air, Apple M5, 10 cores, 16 GB, macOS 27.2, Metal + VideoToolbox | this checkout | `h264_videotoolbox` |
| `ssh debian` | Intel i5-13500T, 20 threads, 15 GB, Debian 13, kernel 7.2.6, UHD 770 (Vulkan, Mesa) | `/home/huou07/or-mission` | `h264_vaapi`, `libx264`, `h264_nvenc` |
| `ssh win11` | Intel Core 5 120U, 12 threads, 15.7 GB, Windows 11 build 26200, Intel Graphics (Vulkan) | `C:\Users\hp\src\or-mission` | `h264_mf`, `h264_nvenc` |

Three rules that cost previous sessions a day each:

1. **The Windows shell over SSH is `cmd.exe`, not bash.** `head`, `;` chaining and
   Unix paths fail; use `&&`, `findstr`, `type`.
2. **A GUI on Windows needs an interactive desktop session.** An SSH session is
   session 0 and has no window station; run GUI work through `schtasks /IT` (see
   below), never directly from the SSH command.
3. **A GUI on Debian needs a Wayland compositor, not Xvfb.** Weston headless with
   Mesa GL talks to the UHD 770; `xvfb-run` plus `LIBGL_ALWAYS_SOFTWARE=1` is a
   software path and must not be used for anything that claims GPU evidence.

## Shared benchmark fixture

All three hosts measure the same file: `bench1080.mp4`, 11,341,971 bytes,
1920×1080 H.264, 30 fps, 360 frames, SHA-256
`877f78da1fe16775ee2c11cf745a3ff6ffb78b4687fea3944b307a060c45ea52`.
It lives at `/tmp/or-bench/bench1080.mp4` on macOS and Debian and
`C:\Users\hp\bench1080.mp4` on Windows. A different fixture invalidates every
comparison; move the same bytes, never re-encode.

## macOS

Build and run (local, no SSH):

```sh
cd "<checkout>"
scripts/build_macos_release.sh --package io.github.huou07.orApp   # release bundle + codesign
open "apps/or_app/build/macos/Build/Products/Release/or_app.app"
python3 scripts/or_ui.py run scripts/ui_scenarios/export-text.json
```

The UI harness builds and launches the real app through the Flutter test binding
and writes `apps/or_app/build/ui-harness/<scenario>/`; screenshots work here.
For a packaged-app surface the semantic harness cannot reach, use
`scripts/packaged_drive.py`.

Benchmark (test binary, packaged FFmpeg libraries):

```sh
cargo test -q -p or_app_bridge --release --lib export_media_benchmark --no-run
BIN=$(ls -t target/release/deps/or_app_bridge-* | grep -v '\.d$' | head -1)
export DYLD_FALLBACK_LIBRARY_PATH="$HOME/Library/Caches/OpencutReinforced/ffmpeg-8.1.3-macos-image-support/ffmpeg-install/lib"
OR_BENCH_SOURCE=/tmp/or-bench/bench1080.mp4 OR_BENCH_FRAMES=300 \
  "$BIN" export_media_benchmark --ignored --nocapture
```

Stage cost without export:
`cargo test -q -p or_render --release --test compose_stage_cost -- --ignored --nocapture --test-threads=1`.

Package and install:

```sh
OR_WHISPER_CLI_PATH="/Applications/Opencut Reinforced.app/Contents/MacOS/whisper-cli" \
  scripts/build_macos_dmg.sh
hdiutil attach -nobrowse -readonly dist/Opencut-Reinforced-<ver>-arm64.dmg
ditto "/Volumes/Opencut Reinforced/Opencut Reinforced.app" "/Applications/Opencut Reinforced.app"
"/Applications/Opencut Reinforced.app/Contents/MacOS/or" version
```

The DMG script refuses a `whisper-cli` whose deployment target is newer than
macOS 13.0 (`Contents/MacOS/whisper-cli requires macOS 26.0, expected 13.0 or
earlier`). Pass a macOS-13 build through `OR_WHISPER_CLI_PATH`; the one inside an
installed 0.1.23 app qualifies, a Homebrew `whisper-cli` usually does not.

## Debian

```sh
ssh debian
export PATH=$HOME/.cargo/bin:$HOME/or-build/flutter/bin:$PATH
export LD_LIBRARY_PATH=$HOME/or-build/ffmpeg-8.1.3-install/lib:$LD_LIBRARY_PATH
cd ~/or-mission
```

GPU verification:

```sh
vulkaninfo --summary | grep -E "deviceName|driverName"   # Intel(R) UHD Graphics 770 (ADL-S GT1), Mesa
vainfo | grep -E "Driver version|VAProfileH264High"      # Intel iHD 25.2.3, VLD + EncSlice
```

GUI session — headless Weston on the real GPU:

```sh
export XDG_RUNTIME_DIR=/tmp/xdg-or-$(id -u); mkdir -p $XDG_RUNTIME_DIR; chmod 700 $XDG_RUNTIME_DIR
(setsid weston --backend=headless-backend.so --socket=or-wayland \
   --width=1440 --height=900 --idle-time=0 >/tmp/weston.log 2>&1 &)
export WAYLAND_DISPLAY=or-wayland
OR_UI_WAYLAND_DISPLAY=or-wayland python3 scripts/or_ui.py --target linux \
  run scripts/ui_scenarios/performance-profiles-linux.json
```

`OR_UI_WAYLAND_DISPLAY` makes the harness attach to that compositor with
`GDK_BACKEND=wayland` and drops `LIBGL_ALWAYS_SOFTWARE`; without it the harness
still falls back to `xvfb-run` with software GL, which is only acceptable for a
pure interaction check. Screenshots are written under
`apps/or_app/build/ui-harness/artifacts/`, and the report's evidence text names
which of the two paths ran.

Benchmark:

```sh
cargo test -q -p or_app_bridge --release --lib export_media_benchmark --no-run
BIN=$(ls -t target/release/deps/or_app_bridge-* | grep -v '\.d$' | head -1)
OR_BENCH_SOURCE=/tmp/or-bench/bench1080.mp4 OR_BENCH_FRAMES=300 \
  "$BIN" export_media_benchmark --ignored --nocapture
```

Packaging (release `.deb`, then the packaged desktop journey):

```sh
export OR_LANE_FFMPEG_PREFIX=$HOME/or-build/ffmpeg-8.1.3-install
export OR_LANE_CODEC_PREFIX=$HOME/or-build/ffmpeg-codecs-install-gpl
export OR_LANE_BUILD_MODE=release
bash scripts/run_local_lane.sh linux --packaged --steps deb,journey \
  --report apps/or_app/build/local-lanes/release-deb.json
```

The lane holds `/tmp/aeh-native-ui-<uid>.lock`. A killed lane can leave a child
holding it (`BLOCKED: another OR lane ... holds the shared native UI lock`); find
the holder with `fuser -v /tmp/aeh-native-ui-$(id -u).lock` and let it exit, do
not delete the file.

Install and check:

```sh
sudo dpkg -i apps/or_app/build/local-lanes/opencut-reinforced-linux-x64-release.deb
/opt/opencut-reinforced/or version
```

## Windows

```sh
ssh win11          # cmd.exe
```

Toolchain environment used by every build and benchmark. Order matters: the
CPython interpreter and the MSVC tools must come before `msys64` on `PATH`, or a
lane picks the msys `python` (which cannot open a `C:\...` script path: `can't
open file '/c/.../C:\Users\...\scripts\stage_desktop_runtime.py'`) and the
runtime stager fails with `dumpbin.exe is not recognized`.

```
PATH=C:\Users\hp\AppData\Local\Programs\Python\Python312;C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC\14.44.35207\bin\Hostx64\x64;C:\Users\hp\.cargo\bin;C:\orb\flutter\bin;C:\msys64\clang64\bin;C:\msys64\usr\bin;C:\orb\tmp\ffmpeg-8.1.3-install\bin;%PATH%
FFMPEG_INSTALL_PREFIX=C:\orb\tmp\ffmpeg-8.1.3-install
LIBCLANG_PATH=C:\msys64\clang64\bin\libclang.dll
OR_PYTHON=C:\Users\hp\AppData\Local\Programs\Python\Python312\python.exe
```

`stage_windows_bundle.ps1` insists on `dumpbin.exe` (VS 2022 Build Tools,
`VC\Tools\MSVC\<version>\bin\Hostx64\x64`) because it checks the staged
`ffmpeg.exe` imports; without it the stage step throws
`the shared runtime stager failed` after a successful stage.

Interactive GUI work (a real desktop window, D3D/window station available) goes
through a scheduled task, which runs in the logged-on session:

```
schtasks /Create /TN OrUiJourney /TR "powershell -NoProfile -ExecutionPolicy Bypass -File C:\Users\hp\win_ui_journey.ps1" /SC ONCE /ST 23:59 /IT /F
schtasks /Run /TN OrUiJourney
type C:\Users\hp\ui-journey.log
```

`win_ui_journey.ps1` calls
`C:\Users\hp\AppData\Local\Programs\Python\Python312\python.exe scripts\or_ui.py --target windows run scripts\ui_scenarios\motion-starter.json`.
The absolute interpreter matters: the msys64 Python cannot import `msvcrt`. The
Windows test binding has no raster surface for `RepaintBoundary`, so a screenshot
step records `*.unavailable.txt`; behavioural evidence is complete, pixels are
missing by platform limit.

Benchmark: the same scheduled-task pattern with `win_bench552.ps1`, which builds
the test binary, selects the newest `target\release\deps\or_app_bridge-*.exe`
and writes `C:\Users\hp\win-bench-552.log`. Run it from the task, not from the
SSH shell, so the process has a GPU-visible session.

Packaging (release build, staged bundle, portable zip, Inno Setup installer):

```
pwsh -NoProfile -File scripts\run_windows_lane.ps1 -Mode release -Steps build,stage,zip,installer
```

Inno Setup 6 lives at `%LOCALAPPDATA%\Programs\Inno Setup 6\ISCC.exe`. The lane
needs PowerShell 7 (`pwsh`); Windows PowerShell 5.1 cannot load
`System.Runtime.InteropServices.NativeLibrary`.

Install and check:

```
dist\Opencut-Reinforced-<ver>-windows-x64-setup.exe /VERYSILENT /SUPPRESSMSGBOXES /NORESTART
"%LOCALAPPDATA%\Programs\Opencut Reinforced\or.exe" version
```

A silent install over a running copy closes it first and exits 0; that behaviour
is part of the package, not a workaround.

## Benchmark protocol

- Same revision, same fixture, same 300 frames, same 1920×1080 output on all three
  hosts. Record every row at least three times and report the median; a single
  run is a smoke test.
- Rows: MP4 video-only, MP4 + text, render-only (`OR_BENCH_MODE=render
  OR_BENCH_SCALED=1`), render-only + text, decode-only (`OR_BENCH_MODE=decode
  OR_BENCH_NV12=1`), encode-only (`OR_BENCH_MODE=encode`, 150 frames).
- `OR_EXPORT_TRACE=1` prints per-stage medians and p95 (`decode`, `compose`,
  `resolve`, `encode`) for one export; use it to attribute a change before
  optimizing.
- Keep the machine idle: a concurrent cargo build halves the measured rate.
- Never publish a rate measured on an old revision after the core changed, and
  never call a 30-frame 720p export a performance benchmark.

## Recovering a workflow that stopped working

1. Reproduce the failure and keep the log.
2. Compare the environment variable by variable with the table above — the same
   command without `LD_LIBRARY_PATH`, `FFMPEG_INSTALL_PREFIX` or an interactive
   session fails in a way that looks like a broken feature.
3. Only after the old method fails with that environment in place, write a new
   procedure, and record here what changed and why.
