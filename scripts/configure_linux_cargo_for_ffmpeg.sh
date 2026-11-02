#!/usr/bin/env bash
# Pins every Rust build in this checkout to the packaged FFmpeg prefix.
#
# Flutter's native-assets hook builds the bridge in an isolated environment, so
# an exported FFMPEG_DIR does not reach it. The user-level cargo config does, and
# that is why CI and the local lane both write it before building anything.
set -euo pipefail

prefix="${1:-${FFMPEG_INSTALL_PREFIX:-}}"
if [[ -z "$prefix" ]]; then
  echo "usage: $0 <ffmpeg-install-prefix>" >&2
  exit 2
fi
if [[ ! -d "$prefix/lib/pkgconfig" ]]; then
  echo "the FFmpeg prefix has no pkg-config directory: $prefix/lib/pkgconfig" >&2
  exit 2
fi
prefix="$(cd "$prefix" && pwd)"

mkdir -p "$HOME/.cargo"
# The link search order decides which libavutil wins. A dependency that links a
# system library (libsqlite3-sys with pkg-config is one) emits its own
# -L /usr/lib/... before any build-script search path, and the bridge then links
# the host FFmpeg instead of the packaged one. Putting the prefix in rustflags
# places it ahead of every build-script path.
cat > "$HOME/.cargo/config.toml" <<CONFIG
[build]
rustflags = ["-C", "link-arg=-Wl,-rpath,\$ORIGIN", "-L", "native=$prefix/lib"]

[env]
FFMPEG_DIR = { value = "$prefix", force = true }
PKG_CONFIG_PATH = { value = "$prefix/lib/pkgconfig", force = true }
CONFIG

for library in avcodec avformat avutil swresample swscale; do
  if [[ ! -f "$prefix/lib/pkgconfig/lib$library.pc" ]]; then
    echo "the FFmpeg prefix is missing lib$library.pc" >&2
    exit 2
  fi
  if [[ "$(PKG_CONFIG_PATH="$prefix/lib/pkgconfig" pkg-config --variable=prefix "lib$library")" != "$prefix" ]]; then
    echo "lib$library.pc does not point at $prefix" >&2
    exit 2
  fi
done

echo "CARGO_FFMPEG_PREFIX=$prefix"
