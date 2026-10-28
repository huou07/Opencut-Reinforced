# Local macOS Development and Verification

Last reviewed: 2026-10-26

Scope: the concrete commands for this host — build, run, check, measure — and
the traps that cost real time. Generic policy is in
[verification.md](verification.md); permissions in [permissions.md](permissions.md).

## Checks

```sh
bash scripts/check-repo.sh                      # hygiene, docs, links, claims, core deps
cargo check -p or_core --all-targets            # focused source check
cargo test -p or_core -p or_ipc -p or_cli       # domain, IPC, CLI contracts
cargo fmt --all -- --check
cd apps/or_app && flutter analyze --no-pub && flutter test --no-pub
```

Run the focused check first; escalate to the workspace only when useful.

## Build and run the app

```sh
cd apps/or_app && flutter build macos --debug
open build/macos/Build/Products/Debug/or_app.app
```

Release bundle, including packaging and codesigning:

```sh
scripts/build_macos_release.sh --package io.github.huou07.orApp.local
```

The packaged bundle carries the `or` CLI in `Contents/MacOS/or`, so the in-app
assistant can edit the live project after the app leaves the checkout.

Use an isolated profile/bundle identifier for destructive or repeatable runs, and
close the app when a run ends.

## Local transcription helpers

A local debug build does not ship `whisper-cli` or `or-whisper-provider`. Install
them into the app bundle (and re-sign it ad hoc) with:

```sh
scripts/install_local_transcription_helpers.sh
# or with a verified small model:
scripts/install_local_transcription_helpers.sh path/to/or_app.app --model-file /tmp/ggml-small.bin
```

The whisper CLI is taken from `OR_WHISPER_CLI_PATH`, a packaged local app, or
`PATH`; the provider is rebuilt from this checkout. The optional model file is
verified against the small model's SHA-256 before it is placed in the app's
private model folder. The script never builds whisper.cpp itself, so a missing
CLI is a setup error rather than a silent fallback.

## FFmpeg library path

Crates that link FFmpeg need the cached prefix on the loader path:

```sh
DYLD_FALLBACK_LIBRARY_PATH="$HOME/Library/Caches/OpencutReinforced/ffmpeg-8.1.3-macos-image-support/ffmpeg-install/lib" \
  cargo test -p or_app_bridge --lib
```

Without it the test binary aborts with `Library not loaded: @rpath/libavutil.60.dylib`.
`cargo test -p or_core` needs no such variable.

## Trap: stale bridge hook cache

After `flutter_rust_bridge_codegen`, a release build can link an old
`libor_app_bridge.dylib` from the native-assets hook cache (not the crate's
`target/`). Symptom: a black window while debug is fine, and
`Bad state: Content hash on Dart side … different from Rust side …` in the
terminal. Fix:

```sh
rm -rf apps/or_app/.dart_tool/hooks_runner/shared/or_app_bridge/build
rm -rf apps/or_app/.dart_tool/flutter_build
cd apps/or_app && flutter --no-version-check build macos --release
```

## Measuring playback pacing

`flutter test` cannot run a release build, so record pacing from the app:

```sh
cd apps/or_app
flutter --no-version-check build macos --release --dart-define=OR_PACING_LOG=/tmp/or-perf.jsonl
open -n build/macos/Build/Products/Release/or_app.app
# play a few seconds, quit
cat /tmp/or-perf.jsonl
```

Each line covers 120 frames (fps, build/raster p50/p95, vsync gaps). Read
`build_p95_ms` and `raster_p95_ms` against the 16.68 ms budget; `fps` includes
idle time between interactions.

## Measuring decode throughput

```sh
DYLD_FALLBACK_LIBRARY_PATH="$HOME/Library/Caches/OpencutReinforced/ffmpeg-8.1.3-macos-image-support/ffmpeg-install/lib" \
  cargo test -p or_media --test decode_throughput -- --ignored --nocapture
```

It is a measurement, not a gate. Quote the printed numbers.

## Driving the built app

- Prefer the semantic harness ([ui-harness.md](ui-harness.md)); it names what it
  acted on.
- For the packaged bundle, capture only its window, never the full screen:

```sh
pid=$(pgrep -f "or_app.app/Contents/MacOS/or_app" | head -1)
rect=$(osascript -e "tell application \"System Events\" to tell (first process whose unix id is $pid) to get {{position, size}} of window 1" | tr -d '{},' | awk '{print $1","$2","$3","$4}')
screencapture -x -R "$rect" /tmp/packaged-window.png
```

- `System Events` cannot type into native AppKit panels on this host; click rows
  and confirm buttons instead. CoreGraphics events (`scripts/packaged_drive.py`)
  reach the Flutter view when the app is frontmost.
- A Retina capture is 2x: compute points-per-pixel with the driver's `scale`
  command instead of assuming 1x.

## Bundled FFmpeg is a media-interchange build

The bundled build ships muxers `image2`, `image2pipe`, `matroska`, `mov`, `wav`,
`webm` and no `null`/raw-PCM muxer; it has `afftdn` but not `astats`/`ametadata`.
Anything that relies on a statistics-printing filter will fail on shipped builds.

## Related

- [ui-harness.md](ui-harness.md)
- [verification.md](verification.md)
- [reference/tooling.md](../reference/tooling.md)
