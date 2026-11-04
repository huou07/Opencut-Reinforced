# Platform Targets and Packaging

Last reviewed: 2026-11-03

Scope: which platforms exist, how each stores projects and media, and what is
proven versus assumed per platform.

## Targets

Release targets: **macOS, Windows, Linux**. iOS and web are not targets.
Android code remains in the tree (floor API 26, matching CPAL's AAudio backend)
but it is not a release target and it is not built in CI.

For the current mission only the owner's macOS machine is a required runtime
target; other platforms are maintained but not acceptance targets.

## Storage and permissions

| Platform | Project storage | Media access |
|---|---|---|
| macOS / Linux | Local `.orproj` file chosen by the user | Local files, bounded folder import |
| Windows | Local `.orproj` file | Local files, bounded folder import |
| Android | SAF document with an app-private canonical working copy and explicit synchronisation | Persistable SAF tree/document grants |

macOS microphone rules that must hold:

- The microphone is requested with `AVCaptureDevice.requestAccess(for: .audio)`
  when the first take starts. Reading `authorizationStatus` alone never creates
  the TCC entry, so an app that only reads it is missing from Privacy & Security
  → Microphone and the user cannot grant access there.
- `NSMicrophoneUsageDescription` stays in the bundle `Info.plist`, and both
  entitlement sets carry `com.apple.security.device.audio-input`, so a return to
  the sandbox would still capture.
- A denial is reported as state plus a route into the microphone pane. The app
  never records silently and never claims a take it did not capture.

Android rules that must hold:

- A SAF reference is never converted into a filesystem path; media access is a
  runtime-only owned-descriptor capability.
- The preview/import descriptor registry holds at most 64 sources; stale preview
  registrations are cleared before folder import uses that capacity.
- A project close cancels and drains decoding before duplicated descriptors are
  cleared.

## Desktop packaging

- App bundles carry their matching FFmpeg shared libraries and licence notices,
  plus packaged-first helper executables.
- macOS: the Developer Preview runs unsandboxed because the in-editor assistant
  starts the local DeepSeek Harness and its `or mcp` bridge; App Sandbox cannot
  grant process launching plus the harness's home-directory credentials. The
  private IPC runtime directory stays owner-only and the code still binds only
  a Unix-domain socket. Reintroducing the sandbox needs a separately installed
  assistant host (ADR 0009).
- Windows: runtime directory, descriptor file and named pipe use protected
  owner-only DACLs; the pipe rejects remote clients.
- Linux: Unix-domain socket with owner-only permissions; desktop package formats
  are maintained by the release lane.
- All three desktops ship the same optional stack: the pinned Whisper engine and
  the provider adapter, the `or` CLI next to the app binary, and the agent
  workspace bundle. `scripts/stage_desktop_runtime.py` owns that copy for every
  platform, so a new helper is added once.
- The app writes the per-user speech runtime to the same folder the Rust bridge
  searches: `~/Library/Application Support/Opencut Reinforced/speech-runtime` on
  macOS, `%LOCALAPPDATA%\Opencut Reinforced\speech-runtime` on Windows and
  `$XDG_DATA_HOME|~/.local/share/opencut-reinforced/speech-runtime` on Linux.
  An engine installed by **Settings → External tools** is therefore found by
  the app, the CLI and agents without environment variables.

## Platform seam

Platform rules live in `apps/or_app/lib/support/host_platform.dart`, not in the
callers. It owns the home directory (`USERPROFILE` on Windows, `HOME` elsewhere),
the `PATH` list separator, path joining, executable suffixes (`.exe`, `.cmd`,
`.bat`, `.com` on Windows), the app data folder, external-link opening and
executable discovery, revealing a file in the host file manager (Finder,
Explorer, the freedesktop default) and the system speech voice (`say` on macOS,
SpeechSynthesis through PowerShell on Windows, `espeak-ng` on Linux). Callers
pass an optional `HostPlatform` so the Windows rules are testable on macOS. Adding a platform means adding a branch to that file
once, not to the assistant, the voiceover dialog and the packaging scripts
separately; a feature that uses it works on every desktop at once.

## Native capability matrix

| Platform | Decode | Viewer presentation | MP4 H.264 encoder | System voice |
|---|---|---|---|---|
| macOS | FFmpeg 8.1.3 software, app-local `.dylib` | bounded BGRA8888 premultiplied pixel-buffer texture | VideoToolbox (`h264_videotoolbox`) | `say` |
| Windows | FFmpeg 8.1.3 software, app-local `.dll` | same | NVENC, else Media Foundation (hardware when the driver offers it) | SpeechSynthesis |
| Linux | FFmpeg 8.1.3 software, app-local `.so` | same | NVENC, else VAAPI, else x264 (GPL build) | `espeak-ng` |
| Android | in-process FFmpeg via SAF descriptors | software preview only, no audio | not a desktop profile | n/a |

Each encoder cell is a capability the package carries; the writer probes the
machine before it commits, so "NVENC, else Media Foundation" falls back to the
software tail on a box without the hardware. Platform-specific interop stays
behind narrow runtime adapters. Project and
timeline semantics stay platform-independent; only pickers, permissions,
packaging, window integration and hardware APIs may differ.

## Evidence per platform

Hosted CI is the authority for native and packaged claims on Windows, Linux and
Android. A local run establishes behaviour only for the tested source, build,
host, profile and journey. See [reference/testing.md](../reference/testing.md).

## Related

- [security-boundaries.md](security-boundaries.md)
- [control-plane.md](control-plane.md) — IPC transport details
- [reference/release.md](../reference/release.md)
