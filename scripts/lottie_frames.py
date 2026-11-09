#!/usr/bin/env python3
"""Render a Lottie file to a PNG sequence with the python-lottie sidecar.

OR does not own a Lottie runtime, so a Lottie composition becomes frames here
and a normal media file after that. The frames keep the composition's alpha, so
the result drops onto a track over other clips the way any imported material
does. Pass the Lottie file, the output directory and the frame range:

    python3 scripts/lottie_frames.py badge.json /tmp/frames --frames 60 --fps 30

Requires `python-lottie` and a cairo shared library:

    python3 -m pip install lottie cairosvg
    brew install cairo            # macOS
    apt install libcairo2         # Debian

Licence: python-lottie is MIT; cairo is LGPL/MPL. Neither is linked into OR;
this script runs as a separate program, and its output is a media file.
"""

import argparse
import pathlib
import sys


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", help="Lottie JSON (or .lottie archive) to render")
    parser.add_argument("directory", help="directory to write frame-0000.png into")
    parser.add_argument("--frames", type=int, default=0, help="frames to write; 0 uses the file's own out point")
    parser.add_argument("--fps", type=float, default=0.0, help="resample to this rate; 0 keeps the file's rate")
    parser.add_argument("--scale", type=float, default=1.0, help="output scale (1.0 keeps the composition size)")
    parser.add_argument("--start", type=int, default=0, help="first composition frame")
    args = parser.parse_args()

    try:
        import cairosvg  # noqa: F401  (python-lottie's PNG exporter renders through it)
        import lottie.objects
        from lottie.exporters import cairo as cairo_exporter
        from lottie.parsers import tgs as lottie_parser
    except ImportError as error:  # pragma: no cover - depends on the host
        print(
            "the Lottie sidecar is not installed: "
            f"{error}. Run `python3 -m pip install lottie cairosvg` and make sure "
            "libcairo is on the loader path (macOS: DYLD_FALLBACK_LIBRARY_PATH=/opt/homebrew/lib).",
            file=sys.stderr,
        )
        return 2

    source = pathlib.Path(args.source)
    if not source.is_file():
        print(f"{source}: no such file", file=sys.stderr)
        return 2
    if source.suffix == ".lottie":
        print(f"{source}: unpack the .lottie archive first; this sidecar reads Lottie JSON", file=sys.stderr)
        return 2
    try:
        animation = lottie_parser.parse_tgs(str(source))
    except Exception as error:  # the parser raises its own exception types
        print(f"{source}: not a readable Lottie file: {error}", file=sys.stderr)
        return 2
    if not isinstance(animation, lottie.objects.Animation):
        print(f"{source}: the sidecar only renders Lottie files", file=sys.stderr)
        return 2

    directory = pathlib.Path(args.directory)
    directory.mkdir(parents=True, exist_ok=True)

    frame_rate = args.fps or float(animation.frame_rate)
    total = args.frames or int(round(animation.out_point - animation.in_point))
    step = float(animation.frame_rate) / frame_rate
    output_width = max(1, int(round(animation.width * args.scale)))
    output_height = max(1, int(round(animation.height * args.scale)))

    for index in range(total):
        frame = args.start + index * step
        path = directory / f"frame-{index:04d}.png"
        with path.open("wb") as handle:
            animation.width, animation.height = output_width, output_height
            cairo_exporter.export_png(animation, handle, frame=frame)
    print(
        f"lottie_frames: {total} frames {output_width}x{output_height} "
        f"at {frame_rate:g} fps from {source.name} -> {directory}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
