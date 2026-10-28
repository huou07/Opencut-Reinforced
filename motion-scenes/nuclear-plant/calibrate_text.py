#!/usr/bin/env python3
"""Measure rendered ink widths for the strings the scene centres.

The MotionScene text rasteriser lays a text node out in a box that runs from the
node's x to the right edge of the canvas, and it insets the glyphs by 5% of that
box. So a node's x is not where its ink starts, and centring needs the real ink
width rather than an estimate. This probe renders one text node per string and
reads the exact ink extents out of the PNG.

Usage: python3 calibrate_text.py [--or PATH]
Writes text_metrics.json next to this file.
"""

import json
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
CANVAS_W = 1920
ROW_H = 120
STRINGS = [
    ("NHÀ MÁY ĐIỆN HẠT NHÂN", 78),
    ("Nguyên lý hoạt động — từ phân hạch đến lưới điện", 30),
    ("LÒ NƯỚC ÁP LỰC (PWR) · NHIÊN LIỆU U-235", 22),
    ("TÓM LẠI", 24),
    ("MỘT CHUỖI CHUYỂN HÓA NĂNG LƯỢNG", 48),
    ("mỗi mắt lưới chỉ đổi một dạng năng lượng", 24),
    ("Nhiên liệu U-235 · lò nước áp lực (PWR) · chu trình kín, vận hành không phát thải CO₂", 23),
    ("PHÂN HẠCH", 26),
    ("NHIỆT", 26),
    ("HƠI NƯỚC", 26),
    ("QUAY", 26),
    ("ĐIỆN", 26),
]


def main():
    or_path = sys.argv[sys.argv.index("--or") + 1] if "--or" in sys.argv else "or"
    nodes = [
        {
            "node_id": "bg",
            "kind": "rectangle",
            "position": {"x": 0, "y": 0},
            "size": {"width": CANVAS_W, "height": ROW_H * len(STRINGS)},
            "fill": {"red": 0, "green": 0, "blue": 0},
        }
    ]
    for index, (content, size) in enumerate(STRINGS):
        nodes.append(
            {
                "node_id": "row-%d" % index,
                "kind": "text",
                "text": content,
                "position": {"x": 0, "y": MAX_TOP(index)},
                "font_size": size,
                "fill": {"red": 255, "green": 255, "blue": 255},
            }
        )
    scene = {
        "format": "opencut-reinforced-motion-scene",
        "schema_version": 1,
        "scene_id": "5c1d2a70-3b44-4f0a-9e21-7a6b8c9d0e1f",
        "name": "Text metrics probe",
        "duration": {"numerator": 1, "denominator": 1},
        "canvas": {"width": CANVAS_W, "height": ROW_H * len(STRINGS)},
        "assets": [],
        "nodes": nodes,
        "animations": [],
    }
    scene_path = Path("/tmp/or-text-probe.ormotion.json")
    png_path = Path("/tmp/or-text-probe.png")
    raw_path = Path("/tmp/or-text-probe.raw")
    scene_path.write_text(json.dumps(scene, ensure_ascii=False), encoding="utf-8")
    subprocess.run(
        [or_path, "motion", "render", "--file", str(scene_path), "--at", "0/1", "--output", str(png_path)],
        check=True,
        capture_output=True,
    )
    height = ROW_H * len(STRINGS)
    subprocess.run(
        ["ffmpeg", "-y", "-loglevel", "error", "-i", str(png_path), "-f", "rawvideo", "-pix_fmt", "gray", str(raw_path)],
        check=True,
    )
    data = raw_path.read_bytes()

    metrics = {}
    for index, (content, size) in enumerate(STRINGS):
        top = MAX_TOP(index)
        min_x, max_x = CANVAS_W, -1
        for y in range(top, min(top + ROW_H, height)):
            row = data[y * CANVAS_W : (y + 1) * CANVAS_W]
            for x in range(CANVAS_W):
                if row[x] > 30:
                    min_x = min(min_x, x)
                    max_x = max(max_x, x)
        if max_x < 0:
            raise SystemExit("no ink for %r" % content)
        metrics[content] = {
            "size": size,
            "width": max_x - min_x + 1,
            "lead": min_x,
        }
    out = HERE / "text_metrics.json"
    out.write_text(json.dumps(metrics, ensure_ascii=False, indent=1) + "\n", encoding="utf-8")
    for content, entry in metrics.items():
        print("%4d  %3d  %s" % (entry["width"], entry["lead"], content))
    print("wrote", out)


def MAX_TOP(index):
    return index * ROW_H + 40


if __name__ == "__main__":
    main()
