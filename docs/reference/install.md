# Install, permissions and uninstall

Last reviewed: 2026-11-09

Scope: how to install a published Opencut Reinforced build on macOS, Windows,
Debian/Ubuntu and Flatpak, what each package needs from the machine, where it
writes, and how to remove it. What each build is verified to do is in
[docs/reference/quality.md](quality.md); package contents and signing are in
[docs/reference/release.md](release.md).

Every published release carries five assets and one `SHA256SUMS.txt`. Verify a
download before installing it:

```sh
sha256sum -c SHA256SUMS.txt        # Linux
shasum -a 256 -c SHA256SUMS.txt    # macOS
```

## macOS (Apple Silicon, macOS 13 or newer)

1. Open `Opencut-Reinforced-<version>-arm64.dmg`.
2. Drag **Opencut Reinforced** into **Applications**.
3. The build is ad-hoc signed, not notarized. On first launch, right-click the
   app and choose **Open**, or allow it once under **System Settings → Privacy &
   Security**. A plain double-click on a quarantined copy is refused by Gatekeeper.

The app bundle carries FFmpeg 8.1.3, the local Whisper speech runtime, the `or`
CLI and the agent workspace. The CLI is
`/Applications/Opencut Reinforced.app/Contents/MacOS/or`; add it to `PATH` if you
want `or` in a terminal. Uninstall by moving the app to the Trash.

Intel Macs are not a target: the published image is arm64 only.

## Windows (x64, Windows 10 or newer)

- **Installer:** run `Opencut-Reinforced-<version>-windows-x64-setup.exe`. It
  installs for the current user only, so it asks for no administrator rights, and
  it upgrades in place — a running copy is closed first and data is kept.
- **Portable:** unzip `Opencut-Reinforced-<version>-windows-x64.zip` anywhere and
  run `or_app.exe`. The same FFmpeg, speech runtime and `or` CLI sit beside it.

SmartScreen may warn about an unsigned publisher; the build is not
code-signed. Uninstall from **Settings → Apps → Opencut Reinforced** (or run the
portable build's own uninstaller, which removes only files it wrote).

The bundled encoder ladder is NVENC, then Media Foundation, then software. A
machine with no usable H.264 encoder fails an MP4 job with a sentence naming the
missing encoder and pointing at WebM or Matroska.

## Debian and Ubuntu (amd64)

```sh
sudo apt install ./Opencut-Reinforced-<version>-linux-x64.deb
```

The package installs to `/opt/opencut-reinforced`, adds a desktop entry and puts
the `or` CLI at `/opt/opencut-reinforced/or`. It depends on the usual GTK and
audio libraries; `apt` resolves them. The Linux package links an FFmpeg build
that includes x264 (GPL-2.0-or-later), so MP4 delivery encodes locally; the
licences ship under `/opt/opencut-reinforced/licenses`. Uninstall with
`sudo apt remove opencut-reinforced` — projects and media are yours and stay
where you put them.

The encoder ladder is NVENC, then VAAPI, then x264.

## Flatpak (x86_64)

```sh
flatpak install --user ./opencut-reinforced-<version>-x86_64.flatpak
flatpak run io.github.huou07.or_app
```

The sandbox is deliberately narrow and the app says so in the install output:
`--socket=wayland` with `--socket=fallback-x11`, `--socket=pulseaudio` (the mixer
is silent without it), `--share=ipc`, `--device=dri` for the GPU, `--share=network`
for the assistant provider and a loopback TTS server, and `--filesystem=` on
`xdg-documents`, `xdg-videos` and `xdg-download`. A file chosen through the file
chooser outside those folders arrives through the document portal.

**Runtime status: UNVERIFIED.** The package builds, installs and reports its
version and permissions, but the sandbox runtime has not been run on a host where
`bwrap` can create namespaces. On such a host `flatpak run` exits with
`bwrap: Can't mkdir parents for /proc: Permission denied` before the app starts;
that is a host restriction, not an application failure. Until a run is recorded,
treat the Flatpak build as untested rather than supported, and prefer the `.deb`
on Linux.

## Where the app writes

Projects are ordinary files wherever you save them. Per-user state:

| What | macOS | Windows | Linux |
|---|---|---|---|
| Speech runtime installed from Settings | `~/Library/Application Support/Opencut Reinforced/speech-runtime` | `%LOCALAPPDATA%\Opencut Reinforced\speech-runtime` | `$XDG_DATA_HOME/opencut-reinforced/speech-runtime` (or `~/.local/share/…`) |
| Session IPC runtime directory | owner-only directory under the app data folder | same, protected DACL | same, `0700` directory |
| Settings, recent projects, workspace | app data folder above | app data folder above | app data folder above |

The IPC socket or named pipe is owner-only and local; it rejects remote clients.

## Graphics and encoders

**Settings → Performance** reports the backend the machine actually uses —
Metal on macOS, Vulkan or OpenGL on Windows and Linux — plus the encoders the
package offers and the ones this machine opened. A machine whose Vulkan
enumeration is empty or broken falls back to OpenGL instead of crashing; a
machine with no usable GPU renders with the software backend and says so. Rates
per host are recorded in [export-performance.md](export-performance.md) and the
per-host encoder choices in [hardware-profiles.md](hardware-profiles.md).

## Companion tools

Settings → External tools lists FFmpeg, Node, SVGO, Inkscape, Blender, Audacity,
Remotion, Motion Canvas, a Lottie renderer and a glTF renderer, with the state of
each one and the install command for your platform.

- FFmpeg is bundled, so media import and export never need a system FFmpeg.
- The Lottie and glTF sidecars need Node.js and a Chromium browser; their setup
  command is `npm install` inside the project folder.
- Blender, Inkscape and Audacity are used when you send a job to them; OR does
  not install them.
- Results that come back from a tool carry a provenance record, so the origin
  stays readable in the Media Library and on the timeline.

## Troubleshooting

- **The app opens but export fails for MP4.** The machine has no H.264 encoder
  the package can use. Use WebM (VP9/Opus) or Matroska (FFV1/PCM); the error text
  names the encoder that was missing.
- **Playback is silent in the Flatpak build.** The sandbox needs
  `--socket=pulseaudio`; the published manifest has it, a hand-built one may not.
- **A speech model download fails.** The model is an optional download from
  Settings → Resources & updates; it needs network access and writes to the
  speech runtime folder above.
- **A companion tool is listed as missing.** Follow the printed install command;
  project-scoped tools (Remotion, Motion Canvas, Lottie, glTF) install inside the
  project folder and are read while that project is open.
- **The assistant panel cannot connect.** It needs a harness on `PATH` or one
  linked in Settings → Assistant; the panel says which one it looked for.
- **A project refuses to open.** Projects are forward-compatible only; a file
  written by a newer schema opens read-only and says so. Recovery copies live
  beside the project.

## Uninstall

macOS: move the app to the Trash. Windows: **Settings → Apps**. Linux:
`sudo apt remove opencut-reinforced`. Flatpak:
`flatpak uninstall --user io.github.huou07.or_app`. None of them deletes your
projects or media.

## Known limitations

- macOS builds are ad-hoc signed and not notarized.
- The Windows build is unsigned, so SmartScreen warns once.
- The Flatpak sandbox runtime is unverified (see above).
- NVIDIA and AMD GPU paths are untested here; Intel and software paths are
  measured. Hardware encode/decode is probed per machine, never assumed.
- Android is a development target, not a published package.
