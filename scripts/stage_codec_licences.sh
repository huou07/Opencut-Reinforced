#!/usr/bin/env bash
# Stage (or verify) the third-party licence tree for a packaged desktop build.
#
#   stage_codec_licences.sh stage  <target-dir> <ffmpeg-copying.lgplv2.1> <codec-notices-dir>
#   stage_codec_licences.sh verify <target-dir>
#
# WHY THIS IS A SCRIPT
#   This layout used to be written out five times: once per platform in
#   developer-preview.yml, not at all in platform-verification.yml, and once more
#   as the assertion list in package_linux_deb.sh. Five copies of one contract,
#   each with its own idea of where the source files live.
#
#   One copy then drifted. The Platform Verification Linux job never staged the
#   tree, so package_linux_deb.sh failed on its first missing-file check and
#   exited under `set -e` before printing anything — a 13 ms failure with no
#   output. The visible defect was a missing file; the reason it existed at all
#   was that nothing owned the contract.
#
#   So the layout lives here, once. Callers pass where their inputs are; they do
#   not decide what the output should contain. `verify` is the same list used by
#   the packaging scripts, so an artifact cannot be packaged with a layout that
#   this script would not have produced.
#
# WHAT IT DOES NOT DO
#   It does not download anything, run network calls, or read credentials. It
#   copies files that the calling job already produced, and fails loudly and
#   completely when one is missing.
#
# WHO CALLS IT TODAY
#   platform-verification.yml  linux job   -> the Flutter bundle
#   developer-preview.yml      build-linux -> the seeded runtime and the bundle
#   package_linux_deb.sh       verify      -> the same list, so the requirement
#                                            and its producer cannot disagree
#
# NOT YET UNIFIED, ON PURPOSE
#   developer-preview.yml build-macos uses Contents/Resources/ThirdPartyNotices/,
#   a genuinely different layout for a genuinely different target. Sharing this
#   script there would need a layout parameter and would not remove a
#   disagreement.
#   developer-preview.yml build-windows writes the same licenses/ layout from
#   PowerShell. That is real duplication with the same drift risk this script
#   exists to remove. It was left alone because that job is green and the change
#   cannot be verified from here; converting it is a deliberate follow-up, not an
#   oversight. The publish job separately asserts the paths inside the built
#   archives: that is a check, not another producer, and should stay.
set -euo pipefail

# The contract. Paths are relative to the target directory. Ordered so the
# first failure reported is the first thing a reader would look for.
LICENCE_PATHS=(
  "licenses/FFmpeg/COPYING.LGPLv2.1"
  "licenses/FFmpeg/FFmpeg-NOTICE.txt"
  "licenses/Opus/COPYING"
  "licenses/libvpx/LICENSE"
  "licenses/libvpx/PATENTS"
)

# The FFmpeg notice is generated rather than copied, because it describes how
# this build links FFmpeg and where the corresponding source is published. Keep
# it identical in every artifact: a reader comparing two packages must not find
# two different statements of the same obligation.
write_ffmpeg_notice() {  # target-dir
  cat > "$1/licenses/FFmpeg/FFmpeg-NOTICE.txt" <<'EOF'
This application dynamically links FFmpeg 8.1.3 under LGPL-2.1-or-later.
The corresponding FFmpeg source archive and exact build configuration
are distributed as Developer Preview release assets. FFmpeg shared
libraries are separate files beside the Rust bridge and may be replaced
with a compatible LGPL-compliant build under the applicable license terms.
The ffmpeg and ffprobe programs beside the application binary are built
from the same source and configuration for import, media previews,
and WebM export. FFmpeg's WebM export statically links libvpx and
libopus; their licenses and the libvpx patent notice are included in
the adjacent Opus and libvpx directories.
EOF
}

verify_layout() {  # target-dir
  local target="$1" path failures=()
  for path in "${LICENCE_PATHS[@]}"; do
    if [[ ! -s "$target/$path" ]]; then
      failures+=("missing or empty: $target/$path")
    fi
  done
  if (( ${#failures[@]} )); then
    echo "licence tree incomplete in $target: ${#failures[@]} of ${#LICENCE_PATHS[@]} missing" >&2
    printf '  %s\n' "${failures[@]}" >&2
    return 1
  fi
  return 0
}

usage() {
  printf '%s\n' \
    "usage: stage_codec_licences.sh stage  <target-dir> <ffmpeg-copying> <codec-notices-dir>" \
    "       stage_codec_licences.sh verify <target-dir>" \
    "       stage_codec_licences.sh list"
  exit 2
}

case "${1:-}" in
  list)
    printf '%s\n' "${LICENCE_PATHS[@]}"
    ;;

  verify)
    target="${2:-}"
    [[ -n "$target" ]] || usage
    [[ -d "$target" ]] || { echo "no such target directory: $target" >&2; exit 1; }
    if verify_layout "$target"; then
      echo "licence tree complete in $target (${#LICENCE_PATHS[@]} files)"
    else
      exit 1
    fi
    ;;

  stage)
    target="${2:-}"
    ffmpeg_copying="${3:-}"
    notices_dir="${4:-}"
    [[ -n "$target" && -n "$ffmpeg_copying" && -n "$notices_dir" ]] || usage

    # Report every missing input at once, so one CI round reveals all of them.
    problems=()
    [[ -d "$target" ]] || problems+=("target directory does not exist: $target")
    [[ -s "$ffmpeg_copying" ]] || problems+=("FFmpeg LGPL text missing or empty: $ffmpeg_copying")
    for name in OPUS-COPYING LIBVPX-LICENSE LIBVPX-PATENTS; do
      [[ -s "$notices_dir/$name" ]] || problems+=("codec notice missing or empty: $notices_dir/$name")
    done
    if (( ${#problems[@]} )); then
      echo "cannot stage the licence tree:" >&2
      printf '  %s\n' "${problems[@]}" >&2
      echo "" >&2
      echo "  These files are produced by earlier steps of the calling job:" >&2
      echo "    FFmpeg-COPYING.LGPLv2.1  extracted from the FFmpeg source archive" >&2
      echo "    OPUS-COPYING             written by build_desktop_export_codecs.sh" >&2
      echo "    LIBVPX-LICENSE/PATENTS   written by build_desktop_export_codecs.sh" >&2
      exit 1
    fi

    mkdir -p "$target/licenses/FFmpeg" "$target/licenses/Opus" "$target/licenses/libvpx"
    install -m 644 "$ffmpeg_copying" "$target/licenses/FFmpeg/COPYING.LGPLv2.1"
    install -m 644 "$notices_dir/OPUS-COPYING" "$target/licenses/Opus/COPYING"
    install -m 644 "$notices_dir/LIBVPX-LICENSE" "$target/licenses/libvpx/LICENSE"
    install -m 644 "$notices_dir/LIBVPX-PATENTS" "$target/licenses/libvpx/PATENTS"
    write_ffmpeg_notice "$target"

    # Verify what was just written, using the same list the packagers check
    # against. Staging and verifying cannot disagree if they share one list.
    verify_layout "$target"
    echo "staged ${#LICENCE_PATHS[@]} licence files into $target"
    ;;

  *)
    usage
    ;;
esac
