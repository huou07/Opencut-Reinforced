# Local release evidence, 2026-11-09

Last reviewed: 2026-11-09

This directory records what the 2026-11-09 local release run actually produced on
the three mission hosts. Revision `b8feb51f` is the packaging commit; the app
code it ships is `5a2bef94` plus documentation and the Flatpak packaging files.
The DMG was rebuilt at `5967969c` with a properly targeted `whisper-cli`, and the
hosts later moved to `f32479b2` (companion-tool MCP surface and `or tools run`).

## Revision per host

| Host | Path | Revision | Working tree |
| --- | --- | --- | --- |
| macOS M5 | this checkout | `f32479b2` | clean |
| Debian (`ssh debian`) | `/home/huou07/or-mission` | `f32479b2` | clean |
| Windows 11 (`ssh win11`) | `C:\Users\hp\src\or-mission` | `f32479b2` | clean except untracked `.cargo/` |

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

## Verification after the provenance work

- `flutter test` (apps/or_app): 618 tests, all pass.
- `cargo test -p or_core -p or_tools -p or_app_bridge`: pass, except two
  pre-existing `project_storage` tests that asserted a literal
  `schema_version: 9` while the schema is 11; fixed to use
  `CURRENT_PROJECT_SCHEMA_VERSION`, after which `project_storage` is 11/11.
- `or tools provenance write/read` verified on macOS (Blender record) and
  Debian (`FFmpeg produced /tmp/x.mp4`).
- Media library shows `From <tool> <version>` for a media file with a sidecar
  (`apps/or_app/test/media_provenance_row_test.dart`, 2 tests).

## Session 2 — provenance, audit, benchmark, sidecars

The 2026-11-09 session continued from `1476e77c` and did not rewrite anything it
inherited. Four commits carry the work: `b39dc6b5` (a companion result's origin
reaches the library, the dialog and the timeline), `118fe99d` (the audit sweeps
that read every screen surface), `813ed9b2` (the Lottie sidecar, the compact
layout fix, the missing install commands) and `e49cb478` (the glTF sidecar, the
three-host benchmark table and the two closed performance questions).

### Revision per host

| Host | Path | Revision | Working tree |
| --- | --- | --- | --- |
| macOS M5 | this checkout | `e49cb478` | clean |
| Debian (`ssh debian`) | `/home/huou07/or-mission` | `e49cb478` | clean |
| Windows 11 (`ssh win11`) | `C:\Users\hp\src\or-mission` | `e49cb478` | clean except untracked `.cargo/` |

Synced with a local git bundle; nothing was pushed, and no CI job, tag or
release ran.

### Provenance, proven in the real app

Four journeys drive the shipped app through the semantic harness on macOS, and
one repeats the Lottie journey on Debian:

| Journey | Steps | macOS | Debian | What it proves |
| --- | --- | --- | --- | --- |
| `scripts/ui_scenarios/companion-tools-catalog.json` | 24 | PASS | not run | the catalog's states, versions, health checks, install guidance and the absent launch control for a headless tool |
| `scripts/ui_scenarios/companion-provenance.json` | 37 | PASS | not run | an FFmpeg render, its record, the library row, the origin dialog and the clip that keeps the origin |
| `scripts/ui_scenarios/companion-lottie.json` | 38 | PASS | PASS | the same, through the Lottie sidecar: the animation stays the source |
| `scripts/ui_scenarios/companion-gltf.json` | 38 | PASS | not run | the same, through the glTF sidecar: the model stays the source |

The Fixtures are real renders, not stored samples: `scripts/make_provenance_fixture.sh`,
`scripts/make_lottie_fixture.sh` and `scripts/make_gltf_fixture.sh` run the tool
and write the record with `or tools provenance write`, so the record names the
host, the version and the argument vector that produced the file.

The two sidecar records committed in this revision:

| Fixture | Bytes | Record |
| --- | --- | --- |
| `scripts/ui_scenarios/companion-lottie-badge.mp4` | 17,868 | `lottie`, `Lottie renderer`, version `1.0.9`, manifest `scripts/lottie-render/or-badge.json` |
| `scripts/ui_scenarios/companion-gltf-cube.mp4` | 65,826 | `gltf`, `glTF renderer`, version `0.137.0`, manifest `scripts/gltf-render/or-cube.gltf` |

### Live MCP handshake, and the defect it found

`scripts/mcp_live_handshake.py` starts `or mcp --pointer …` against a real
`or session serve` descriptor and speaks the protocol: `initialize` (protocol
`2025-06-18`, server `opencut-reinforced` 0.1.27), `tools/list` (39 tools),
`project_status`, `tools_list`, `tools_provenance_read` and a
`tools_provenance_write` round trip. Result: `RESULT: OR_MCP_HANDSHAKE=PASS`.

The same handshake passed on Debian against the installed `0.1.27+e49cb47871e0`
CLI (`initialize`, 39 tools, `project_status`, `tools_list` → `2 of 10 ready`,
`RESULT: OR_MCP_HANDSHAKE=PASS`, clean `session shutdown`). The first attempt
there used `./target/debug/or` from an older revision, which does not advertise
`tools_provenance_read` — use the installed CLI or rebuild before believing a
"server does not advertise" error.

It found a real defect that compile checks and unit tests did not: `or tools
provenance write … --json` printed a bare path, so the MCP layer failed with
`the or CLI returned invalid JSON: expected value at line 1 column 1`. The verb
now returns `{"file": SIDECAR, "record": …}` and `crates/or_cli/tests/mcp.rs`
regression-tests the MCP path end to end.

### UI audit

Every shipped journey and every new sweep was run through the real app:

- the 12 pre-existing journeys: 11 PASS; `compact-editor-journey` FAILED with
  `A RenderFlex overflowed by 6.0 pixels on the right`, and passes after the fix;
- `ui-audit-chrome.json` (27 steps), `ui-audit-editor.json` (36),
  `ui-audit-inspector.json` (27): PASS, with tree and screenshot artifacts for
  Home, Projects, Library, the six Settings sections, the editor's four tool
  panels, the inspector, the clip menu, the advanced clip sheet and export.

Defects found and fixed:

1. **Compact top bar overflowed by 6 px at 320 px.** The fixed children came to
   302 px against 296 px of usable width. The brand box and its divider now
   render only at ≥360 px, where the leading back control already covers them;
   `compact-editor-journey` passes at 320x440 and at 390x844.
2. **Two catalog entries had no install line outside Linux.** `remotion` and
   `motion-canvas` carried only a `Platform::Linux` command, so a macOS or
   Windows user saw a tool with no installation guidance. All npm-based entries
   now name all three platforms.

Selector mistakes in the audit scenarios themselves were fixed rather than
worked around: the advanced clip sheet's buttons carry labels but no keys, and
the export control is `export-project`, not `Export`.

### Benchmark

The three-host table, the reviewer's method and both closed questions are in
`docs/reference/export-performance.md` under "2026-11-09 — three hosts at one
revision". Headline: 1080p full pipeline 230.2 fps (macOS M5), 281.1 (Debian
UHD 770), 132.9 (Windows Intel Graphics); 4K full pipeline at 3840x2160 output
61.0 / 67.5 / 29.3. The 4K→1080p slowdown is FFmpeg's `swscale` inside decode
(10.0 / 12.6 / 18.6 ms per frame), and the Windows WebM row is the same
per-frame handoff as Debian (0.46 against 0.42 of the VP9 ceiling).

Fixtures were verified on all three hosts before measuring:
`bench1080.mp4` 11,341,971 B `877f78da…`, `bench4k.mp4` 19,888,977 B
`c0185f8e…`.

### Packages

Every package below was rebuilt at `e49cb478`, so the version stamp and the
catalog inside match the revision the journeys and benchmarks ran on.

| Package | Size (bytes) | SHA-256 |
| --- | --- | --- |
| `dist/Opencut-Reinforced-0.1.27-arm64.dmg` | 37,528,456 | `f06878928dd76d5db181e0fa5e30b66b61b7196013579addcbcc231603b0e26c` |
| `dist/opencut-reinforced_0.1.27_amd64.deb` (Debian) | 23,443,656 | `743120c26e3d8c0e090e6f6c44c6f64e94fafdeccbf6ac6b73c232546306a55f` |
| `dist/opencut-reinforced-0.1.27-x86_64.flatpak` (Debian) | 9,710,634 | `c91dc520cd5d49f2f3cc1ef70e211b980f59251a9c7b98dd2d7226dc574ca87f` |
| `apps/or_app/build/windows/Opencut-Reinforced-0.1.27-windows-x64-setup.exe` | 25,408,581 | `1a91dbe4e787cb80f3ca5d5364bb21e15201ac994f16b562ef617645b84a506c` |
| `apps/or_app/build/windows/or-windows-x64-release.zip` | 35,058,369 | `864c7811116598042494ce752a5b5a01b5bf3fb29a9148621571134f83573837` |

The macOS DMG was rebuilt at `e49cb478` with `whisper-cli` from
`scripts/build_macos_whisper_cli.sh` (`minos 13.0`), installed over the previous
copy in `/Applications`, and smoke-tested: one 1470x923 window
(`macos/installed-session2.png`), `or version` → `Opencut Reinforced 0.1.27`,
`or tools list` → all nine catalog entries including `lottie` and `gltf`, and
`whisper-cli` reports `minos 13.0`. It stays ad-hoc signed and not notarised, so
`spctl` rejects it.

### Install and smoke test, per package

| Package | Build | Install | Smoke test | State |
| --- | --- | --- | --- | --- |
| macOS DMG | PASS | copied over `/Applications` | window 1470x923; `or version`; `or tools list` names `lottie` and `gltf`; `whisper-cli` `minos 13.0` | PASS |
| Debian DEB | PASS (`run_local_lane.sh linux --packaged --steps deb`) | `dpkg -i` over `0.1.27+21ca8876940b`, now `0.1.27+e49cb47871e0` | `/opt/opencut-reinforced/or version`, `or tools list` with the new entries, payload `or`/`ffmpeg`/`ffprobe`/`lib`/`licenses` | PASS |
| Debian DEB, packaged journey | PASS | `.deb` extracted into a disposable root | `run_packaged_desktop_journey.py` against the installed payload | PASS |
| Debian Flatpak | PASS | `flatpak install --user` (GNOME 49 runtime, 0.1.27) | launch | **BLOCKED by the host** |
| Windows setup + ZIP | PASS (`WINDOWS_LANE=PASS`) | `/VERYSILENT`; the lane's ZIP install/replacement/removal step also passed | `"$LOCALAPPDATA\Programs\Opencut Reinforced\or.exe" version` → 0.1.27, `or tools list` with `lottie` and `gltf` | PASS |
| Windows GUI | — | — | no desktop session (`Services` session 0) | **BLOCKED by the host** |

The Debian lane was run twice: the debug bundle first (deb 101 s, journey
317 s, `RESULT: LOCAL_LANE=PASS`), then the release bundle for the shipped
artifact (`deb` 87 s, `RESULT: LOCAL_LANE=PASS`).

### Flatpak: package PASS, sandbox runtime BLOCKED

The rebuilt Flatpak installs and reports `Version: 0.1.27`,
`Runtime: org.gnome.Platform/x86_64/49`, but it cannot start on this host:

```
$ flatpak run --user --command=sh io.github.huou07.or_app -c "echo inside"
error: opendir(/): Permission denied
$ flatpak run --user --command=sh org.gnome.Sdk/x86_64/49 -c "echo stock"
error: opendir(/): Permission denied
$ flatpak install --user -y dist/opencut-reinforced-0.1.27-x86_64.flatpak
bwrap: Can't mkdir parents for /proc: Permission denied
```

The stock GNOME SDK fails with the same error, so the refusal is not OR's
manifest; `CapEff` inside the Debian host is `0000000000000000`, which is why the
nested user-namespace sandbox cannot be created — the Debian mission host is
itself a container. The condition needed to verify the sandbox path is a Debian
(or other Linux) machine where `bwrap` can create namespaces, not a container in
a container. Until then this stays **BLOCKED**, and the package build is **PASS**
on its own.
