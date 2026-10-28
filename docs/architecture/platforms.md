# Platform Targets and Packaging

Last reviewed: 2026-10-28

Scope: which platforms exist, how each stores projects and media, and what is
proven versus assumed per platform.

## Targets

Release targets: **macOS, Windows, Linux, Android** (Android floor API 26,
Android 8.0, matching CPAL's AAudio backend). iOS and web are not targets.

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

## Native capability matrix

| Platform | Decode | Viewer presentation | Hardware path |
|---|---|---|---|
| macOS | FFmpeg 8.1.3 software, app-local `.dylib` | bounded BGRA8888 premultiplied pixel-buffer texture | none approved |
| Windows | FFmpeg 8.1.3 software, app-local `.dll` | same | none approved |
| Linux | FFmpeg 8.1.3 software, app-local `.so` | same | none approved |
| Android | in-process FFmpeg via SAF descriptors | software preview only, no audio | none approved |

Platform-specific interop stays behind narrow runtime adapters. Project and
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
