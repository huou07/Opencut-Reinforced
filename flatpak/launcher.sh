#!/bin/sh
# The Flutter bundle ships its own libraries and the FFmpeg runtime beside the
# executable. The launcher keeps the lookup inside the package so the host's
# libraries cannot be picked up by accident.
set -eu
root=/app/opencut-reinforced
export LD_LIBRARY_PATH="$root/lib${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
exec "$root/or_app" "$@"
