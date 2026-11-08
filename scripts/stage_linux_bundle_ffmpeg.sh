#!/usr/bin/env bash
# Stages the packaged FFmpeg runtime beside a Flutter Linux bundle.
#
# The release bundle Flutter produces has no FFmpeg: the bridge links the shared
# libraries from the build prefix, and the app looks for `ffmpeg`/`ffprobe`
# beside its own executable. Packaging must therefore copy both and rewrite
# every rpath to a relative one, or the installed package would depend on the
# machine that built it.
set -euo pipefail

if [[ "$#" -ne 2 ]]; then
  echo "usage: stage_linux_bundle_ffmpeg.sh <linux-bundle> <ffmpeg-prefix>" >&2
  exit 2
fi

bundle="$1"
prefix="$2"

for path in "$bundle/or_app" "$prefix/bin/ffmpeg" "$prefix/bin/ffprobe" "$prefix/lib"; do
  [[ -e "$path" ]] || { echo "missing: $path" >&2; exit 1; }
done
command -v patchelf >/dev/null || { echo "missing tool: patchelf" >&2; exit 1; }

runtime="$bundle/lib"
mkdir -p "$runtime"
for pattern in 'libavcodec.so*' 'libavformat.so*' 'libavutil.so*' 'libswresample.so*' 'libswscale.so*' 'libavfilter.so*'; do
  # The glob is intentional: it must expand to the matching files.
  # shellcheck disable=SC2086
  cp -a "$prefix/lib/"$pattern "$runtime/"
done
for library in "$runtime"/libav*.so* "$runtime"/libsw*.so*; do
  [[ -L "$library" ]] && continue
  patchelf --set-rpath '$ORIGIN' "$library"
done

install -m 755 "$prefix/bin/ffmpeg" "$bundle/ffmpeg"
install -m 755 "$prefix/bin/ffprobe" "$bundle/ffprobe"
relative="$(realpath --relative-to="$bundle" "$runtime")"
patchelf --set-rpath "\$ORIGIN/$relative" "$bundle/ffmpeg" "$bundle/ffprobe"

if readelf -d "$bundle/ffmpeg" | grep -F "$prefix"; then
  echo "ffmpeg still references the build prefix" >&2
  exit 1
fi
if readelf -d "$bundle/ffprobe" | grep -F "$prefix"; then
  echo "ffprobe still references the build prefix" >&2
  exit 1
fi
env -u LD_LIBRARY_PATH "$bundle/ffprobe" -hide_banner -version >/dev/null
env -u LD_LIBRARY_PATH "$bundle/ffmpeg" -hide_banner -encoders | grep -q ' png '
echo "staged FFmpeg beside $bundle"
