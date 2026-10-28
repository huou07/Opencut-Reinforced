#!/usr/bin/env python3
"""Generate the MotionScene for the nuclear power plant explainer.

Declarative data only: rectangles, ellipses, arrows, text and typed animations.
Run this file to rewrite scene.ormotion.json next to it.

Layout: one horizontal chain (reactor -> steam generator -> turbine ->
generator -> transformer -> grid) over a return-loop band, with the condenser
and cooling tower on the lower right.
"""

import json
from pathlib import Path

W, H = 1920, 1080
DUR = 30.0

BG = (11, 14, 19)
PANEL = (20, 26, 35)
PANEL2 = (28, 36, 47)
SHELL = (14, 20, 28)
EDGE = (53, 64, 79)
INK = (233, 238, 247)
MUTED = (147, 161, 178)
DIM = (96, 109, 126)
HEAT = (255, 106, 61)
AMBER = (255, 184, 77)
STEAM = (143, 216, 255)
WATER = (63, 134, 201)
ELEC = (255, 209, 102)
STEEL = (185, 198, 216)

nodes = []
anims = []


def t(sec):
    return {"numerator": int(round(sec * 1000)), "denominator": 1000}


def col(c, alpha=255):
    return {"red": c[0], "green": c[1], "blue": c[2], "alpha": alpha}


def base(nid, kind, **fields):
    node = {"node_id": nid, "kind": kind}
    node.update({key: value for key, value in fields.items() if value is not None})
    nodes.append(node)
    return node


def rect(nid, x, y, w, h, fill=None, stroke=None, sw=None, opacity=None, rot=None, alpha=255):
    return base(
        nid,
        "rectangle",
        position={"x": float(x), "y": float(y)},
        size={"width": float(w), "height": float(h)},
        fill=col(fill, alpha) if fill else None,
        stroke=col(stroke) if stroke else None,
        stroke_width=float(sw) if sw else None,
        opacity=opacity,
        rotation_degrees=rot,
    )


def ellipse(nid, x, y, w, h, fill=None, stroke=None, sw=None, opacity=None, rot=None, alpha=255):
    return base(
        nid,
        "ellipse",
        position={"x": float(x), "y": float(y)},
        size={"width": float(w), "height": float(h)},
        fill=col(fill, alpha) if fill else None,
        stroke=col(stroke) if stroke else None,
        stroke_width=float(sw) if sw else None,
        opacity=opacity,
        rotation_degrees=rot,
    )


# The rasteriser lays a text node out in the box [x, canvas.width] and insets the
# glyphs by 5% of that box, so node x = (glyph_left - 0.05*W) / 0.95.
def node_x(glyph_left):
    return (glyph_left - 0.05 * W) / 0.95


def text(nid, content, x, y, size, fill=INK, opacity=None):
    return base(
        nid,
        "text",
        text=content,
        position={"x": float(node_x(x)), "y": float(y)},
        font_size=float(size),
        fill=col(fill),
        opacity=opacity,
    )


def arrow(nid, x1, y1, x2, y2, color, width=8, opacity=None):
    return base(
        nid,
        "arrow",
        position={"x": float(x1), "y": float(y1)},
        end={"x": float(x2), "y": float(y2)},
        stroke=col(color),
        stroke_width=float(width),
        opacity=opacity,
    )


def add_anim(nid, prop, keys, ease="ease_out"):
    anims.append(
        {
            "node_id": nid,
            "property": prop,
            "keyframes": [{"at": t(sec), "value": float(value)} for sec, value in keys],
            "easing": ease,
        }
    )


def show(nid, tin, dur=0.6, ease="ease_out", dy=0.0, out=None):
    keys = [(0.0, 0.0), (tin, 0.0), (tin + dur, 1.0)]
    if out is not None:
        keys += [(out, 1.0), (out + 0.5, 0.0)]
    add_anim(nid, "opacity", keys, ease)
    if dy:
        add_anim(nid, "translation_y", [(0.0, dy), (tin, dy), (tin + dur, 0.0)], ease)


def show_many(ids, tin, step=0.08, dur=0.5, dy=0.0):
    for index, nid in enumerate(ids):
        show(nid, tin + index * step, dur, "ease_out", dy)


def pulse(nid, t0, period, lo, hi, until=DUR, lead=0.4):
    keys = [(0.0, 0.0), (t0 - lead, 0.0)]
    tt = t0
    while tt < until:
        keys.append((tt, lo))
        keys.append((tt + period / 2.0, hi))
        tt += period
    add_anim(nid, "opacity", keys, "ease_in_out")


def flow(nid, dx, dy, start, end, period, phase=0.0):
    """Loop a dot along an offset, wrapping while invisible."""
    tx, ty, op = [(0.0, 0.0)], [(0.0, 0.0)], [(0.0, 0.0)]
    tt = start + phase
    limit = min(end, DUR)
    while tt + period <= limit:
        tx += [
            (tt, 0.0),
            (tt + period * 0.82, dx),
            (tt + period * 0.95, dx),
            (tt + period * 0.97, 0.0),
        ]
        ty += [
            (tt, 0.0),
            (tt + period * 0.82, dy),
            (tt + period * 0.95, dy),
            (tt + period * 0.97, 0.0),
        ]
        op += [
            (tt - 0.02, 0.0),
            (tt + 0.12, 1.0),
            (tt + period * 0.72, 1.0),
            (tt + period * 0.94, 0.0),
        ]
        tt += period
    add_anim(nid, "translation_x", tx, "linear")
    add_anim(nid, "translation_y", ty, "linear")
    add_anim(nid, "opacity", op, "linear")


def spin(nid, t0, rate, until=DUR):
    add_anim(nid, "rotation", [(0.0, 0.0), (t0, 0.0), (until, rate * (until - t0))], "linear")


CHAR = {
    " ": 0.27, "·": 0.32, "→": 0.92, "&": 0.72, ":": 0.3, ",": 0.28, ".": 0.28,
    "-": 0.38, "²": 0.42, "%": 0.86, "(": 0.34, ")": 0.34, "/": 0.42, "—": 0.9,
    "i": 0.27, "l": 0.27, "j": 0.28, "t": 0.36, "f": 0.33, "r": 0.38,
    "m": 0.85, "w": 0.75, "M": 0.87, "W": 0.95, "I": 0.3, "J": 0.5,
}


def est_width(content, size):
    total = 0.0
    for ch in content:
        if ch in CHAR:
            total += CHAR[ch]
        elif ch.isupper():
            total += 0.66
        elif ch.isdigit():
            total += 0.6
        else:
            total += 0.53
    return total * size


METRICS = json.loads((Path(__file__).with_name("text_metrics.json")).read_text(encoding="utf-8"))


def measured_width(content, size):
    entry = METRICS.get(content)
    if entry and entry["size"] == size:
        return entry["width"]
    return est_width(content, size)


def center_text(nid, content, y, size, fill=INK):
    text(nid, content, (W - measured_width(content, size)) / 2.0, y, size, fill)


# ---------------------------------------------------------------------- backdrop
rect("bg", 0, 0, W, H, fill=BG)
rect("top-rule", 0, 0, W, 5, fill=HEAT)

# ------------------------------------------------------------------- title card
center_text("title-main", "NHÀ MÁY ĐIỆN HẠT NHÂN", 400, 78, INK)
center_text("title-sub", "Nguyên lý hoạt động — từ phân hạch đến lưới điện", 520, 30, STEAM)
rect("title-rule", 810, 604, 300, 4, fill=HEAT)
center_text("title-tag", "LÒ NƯỚC ÁP LỰC (PWR) · NHIÊN LIỆU U-235", 664, 22, MUTED)

show("title-main", 0.25, 0.8, "ease_out", dy=26, out=2.75)
show("title-sub", 0.7, 0.8, "ease_out", dy=20, out=2.75)
show("title-tag", 1.1, 0.8, "ease_out", out=2.75)
show("title-rule", 0.9, 0.9, "ease_out", out=2.75)
add_anim("title-rule", "width", [(0.0, 0.0), (0.9, 0.0), (1.8, 300.0)], "ease_out")

# ---------------------------------------------------------------------- header
text("header-title", "NHÀ MÁY ĐIỆN HẠT NHÂN", 90, 48, 36, INK)
text("header-sub", "Nguyên lý hoạt động · lò PWR · chu trình kín", 92, 98, 21, MUTED)
text("header-tag", "U-235", 1742, 56, 20, AMBER)
rect("header-tag-rule", 1740, 84, 88, 3, fill=AMBER)
show("header-title", 3.0, 0.7, "ease_out", dy=16)
show("header-sub", 3.2, 0.7, "ease_out", dy=12)
show("header-tag", 3.4, 0.6)
show("header-tag-rule", 3.4, 0.6)
show("top-rule", 3.0, 0.8)

# ------------------------------------------------------------------ pipes
rect("pipe-hot", 390, 388, 110, 10, fill=HEAT)
rect("pipe-cold", 390, 586, 110, 10, fill=WATER)
rect("pipe-steam-up", 595, 212, 10, 70, fill=STEAM)
rect("pipe-steam-across", 595, 212, 345, 10, fill=STEAM)
rect("pipe-steam-down", 930, 212, 10, 132, fill=STEAM)
text("steam-tag", "HƠI NƯỚC", 700, 176, 18, STEAM)
rect("pipe-exhaust-1", 935, 596, 10, 58, fill=STEAM)
rect("pipe-exhaust-2", 935, 648, 120, 10, fill=STEAM)
rect("pipe-exhaust-3", 1055, 648, 10, 152, fill=STEAM)
text("exhaust-tag", "HƠI XẢ", 950, 604, 18, STEAM)
rect("pipe-feed-1", 470, 858, 570, 10, fill=WATER)
rect("pipe-feed-2", 470, 620, 10, 250, fill=WATER)
rect("pipe-feed-3", 470, 618, 28, 10, fill=WATER)
text("feed-tag", "nước cấp", 700, 828, 17, MUTED)
rect("pipe-cool-out", 1270, 842, 330, 10, fill=WATER)
rect("pipe-cool-in", 1270, 886, 330, 10, fill=WATER)
text("cool-tag", "nước làm mát", 1430, 812, 17, MUTED)

show("pipe-hot", 5.0, 0.6)
show("pipe-cold", 5.3, 0.6)
show("pipe-steam-up", 7.9, 0.5)
show("pipe-steam-across", 8.2, 0.6)
show("pipe-steam-down", 10.0, 0.5)
show("steam-tag", 8.4, 0.6)
show("pipe-exhaust-1", 18.6, 0.5)
show("pipe-exhaust-2", 18.8, 0.5)
show("pipe-exhaust-3", 19.0, 0.5)
show("exhaust-tag", 18.9, 0.6)
show("pipe-feed-1", 21.6, 0.7)
show("pipe-feed-2", 21.9, 0.6)
show("pipe-feed-3", 22.2, 0.4)
show("feed-tag", 21.9, 0.6)
show("pipe-cool-out", 21.0, 0.6)
show("pipe-cool-in", 21.2, 0.6)
show("cool-tag", 21.3, 0.6)

# ------------------------------------------------------------------ step labels
STEPS = [
    ("1 · LÒ PHẢN ỨNG", "phân hạch U-235 tỏa nhiệt", 100, 700, HEAT, 26, 4.4),
    ("2 · BỘ SINH HƠI", "nhiệt biến nước thành hơi", 490, 700, STEAM, 26, 7.1),
    ("3 · TUABIN", "hơi đẩy cánh quạt quay", 810, 700, STEEL, 26, 10.7),
    ("4 · MÁY PHÁT ĐIỆN", "từ trường quay → điện", 1140, 700, ELEC, 24, 11.9),
    ("5 · BIẾN ÁP", "tăng áp", 1450, 700, ELEC, 20, 15.1),
]
for index, (label, sub, x, y, color, size, tin) in enumerate(STEPS):
    text("label-%d" % (index + 1), label, x, y, size, color)
    text("sub-%d" % (index + 1), sub, x, y + 40, 19, MUTED)
    show("label-%d" % (index + 1), tin, 0.6, "ease_out", dy=12)
    show("sub-%d" % (index + 1), tin + 0.15, 0.6, "ease_out", dy=12)

# ----------------------------------------------------------------- stage 1: core
rect("reactor-hall", 100, 280, 300, 380, fill=PANEL, stroke=EDGE, sw=2)
text("reactor-tag", "LÒ PHẢN ỨNG", 120, 292, 15, HEAT)
rect("vessel", 160, 330, 180, 290, fill=(24, 31, 42), stroke=HEAT, sw=3)
for index in range(6):
    rect("fuel-%d" % index, 182 + index * 28, 355, 14, 210, fill=AMBER)
for index in range(6):
    rect("fuel-hot-%d" % index, 185 + index * 28, 358, 8, 204, fill=(255, 176, 120), opacity=0.0)
    pulse("fuel-hot-%d" % index, 4.2 + index * 0.06, 1.3, 0.08, 0.6)
text("fuel-tag", "thanh nhiên liệu", 120, 630, 14, MUTED)

show("reactor-hall", 3.3, 0.7, "ease_out", dy=18)
show("reactor-tag", 3.5, 0.6)
show("vessel", 3.6, 0.7, "ease_out")
show_many(["fuel-%d" % i for i in range(6)], 3.9, 0.08, 0.5, dy=18)
show("fuel-tag", 4.5, 0.6, "ease_out", dy=10)

# ----------------------------------------------------------------- stage 2: steam
rect("sg-shell", 490, 280, 210, 380, fill=PANEL, stroke=EDGE, sw=2)
text("sg-tag", "BỘ SINH HƠI", 510, 292, 15, STEAM)
rect("sg-drum", 520, 325, 150, 300, fill=SHELL, stroke=STEAM, sw=2)
rect("sg-water", 520, 470, 150, 155, fill=WATER, alpha=225)
for index in range(4):
    rect("sg-tube-%d" % index, 530 + index * 34, 340, 8, 270, fill=HEAT, alpha=210)
for index in range(5):
    ellipse("bubble-%d" % index, 540 + (index % 3) * 44, 556, 16, 16, fill=STEAM, opacity=0.0)
    flow("bubble-%d" % index, 0.0, -110.0, 7.4, 16.0, 1.7, phase=index * 0.34)

show("sg-shell", 6.4, 0.7, "ease_out", dy=18)
show("sg-tag", 6.5, 0.6)
show("sg-drum", 6.6, 0.7, "ease_out")
show_many(["sg-tube-%d" % i for i in range(4)], 6.9, 0.07, 0.5)
show("sg-water", 7.0, 0.7, "ease_out")

# ------------------------------------------------------------- stage 3: turbine
ellipse("turbine-casing", 810, 340, 260, 260, fill=PANEL, stroke=EDGE, sw=3)
for index in range(3):
    rect("blade-%d" % index, 830, 460, 220, 20, fill=STEEL, rot=index * 60)
    spin("blade-%d" % index, 10.4, 432.0)
ellipse("turbine-hub", 915, 445, 50, 50, fill=AMBER)
rect("shaft", 1070, 461, 80, 18, fill=(107, 116, 128))

show("turbine-casing", 10.0, 0.7, "ease_out")
show_many(["blade-%d" % i for i in range(3)], 10.4, 0.08, 0.5)
show("turbine-hub", 10.5, 0.5)
show("shaft", 10.6, 0.5)

# ----------------------------------------------------------- stage 4: generator
ellipse("gen-ring", 1140, 380, 180, 180, fill=ELEC)
ellipse("gen-core", 1152, 392, 156, 156, fill=SHELL)
rect("gen-rotor", 1160, 461, 140, 18, fill=ELEC)
spin("gen-rotor", 11.4, 540.0)
ellipse("gen-hub", 1215, 455, 30, 30, fill=(255, 241, 194))

show("gen-ring", 11.2, 0.7, "ease_out")
show("gen-core", 11.25, 0.7, "ease_out")
show("gen-rotor", 11.5, 0.5)
show("gen-hub", 11.6, 0.5)
for index in range(2):
    ellipse("spark-%d" % index, 1330 + index * 22, 396 - index * 20, 12, 12, fill=(255, 249, 214), opacity=0.0)
    pulse("spark-%d" % index, 12.2 + index * 0.5, 1.1, 0.0, 0.9, until=17.6)

# ----------------------------------------------------- stage 5: transformer/grid
rect("xform-body", 1450, 400, 140, 160, fill=PANEL, stroke=EDGE, sw=2)
text("xform-tag", "MÁY BIẾN ÁP", 1464, 408, 14, ELEC)
for index in range(3):
    rect("xform-coil-%d" % index, 1470 + index * 32, 440, 18, 78, fill=(74, 85, 104))
text("grid-tag", "LƯỚI ĐIỆN", 1730, 356, 19, ELEC)
arrow("grid-line-a", 1590, 450, 1880, 408, ELEC, 5)
arrow("grid-line-b", 1590, 512, 1880, 548, ELEC, 5)
arrow("gen-out", 1360, 470, 1450, 470, ELEC, 7)

show("xform-body", 14.4, 0.7, "ease_out", dy=16)
show("xform-tag", 14.5, 0.6)
show_many(["xform-coil-%d" % i for i in range(3)], 14.6, 0.07, 0.5)
show("gen-out", 14.8, 0.6)
show("grid-line-a", 15.5, 0.7)
show("grid-line-b", 15.7, 0.7)
show("grid-tag", 16.0, 0.6)

# ------------------------------------------------------- stage 6: condenser loop
rect("condenser", 1040, 800, 230, 140, fill=PANEL, stroke=EDGE, sw=2)
text("cond-tag", "BÌNH NGƯNG", 1082, 808, 14, STEAM)
for index in range(4):
    rect("cond-tube-%d" % index, 1058, 826 + index * 20, 194, 7, fill=STEAM, alpha=190)
rect("cond-water", 1058, 908, 194, 24, fill=WATER, alpha=230)
text("label-6", "6 · BÌNH NGƯNG", 1040, 962, 22, STEAM)

show("condenser", 18.9, 0.7, "ease_out", dy=16)
show("cond-tag", 19.0, 0.6)
show_many(["cond-tube-%d" % i for i in range(4)], 19.0, 0.06, 0.5)
show("cond-water", 19.2, 0.6)
show("label-6", 19.4, 0.6, "ease_out", dy=12)

ellipse("tower-base", 1580, 898, 180, 46, fill=PANEL2, stroke=EDGE, sw=2)
rect("tower-body", 1598, 760, 144, 142, fill=PANEL2, stroke=EDGE, sw=2)
ellipse("tower-rim", 1608, 736, 124, 38, fill=PANEL, stroke=EDGE, sw=2)
text("label-7", "7 · THÁP LÀM MÁT", 1580, 962, 22, STEAM)

show("tower-base", 20.0, 0.6, "ease_out")
show("tower-body", 20.2, 0.7, "ease_out")
show("tower-rim", 20.4, 0.6, "ease_out")
show("label-7", 20.6, 0.6, "ease_out", dy=12)
for index in range(9):
    ellipse(
        "plume-%d" % index,
        1638 + (index % 3) * 22,
        728,
        46,
        30,
        fill=(200, 226, 245),
        opacity=0.0,
        alpha=145,
    )
    flow("plume-%d" % index, 0.0, -132.0, 20.6, 29.4, 2.6, phase=index * 0.29)
text("plume-tag", "hơi ra", 1752, 742, 14, MUTED)
show("plume-tag", 21.0, 0.6)

# --------------------------------------------------------- direction particles
def dot(nid, x, y, color, size=14):
    ellipse(nid, x, y, size, size, fill=color)


for index in range(2):
    dot("hot-dot-%d" % index, 400, 386, (255, 220, 190))
    flow("hot-dot-%d" % index, 92.0, 0.0, 5.2, 9.4, 1.6, phase=index * 0.55)
for index in range(2):
    dot("cold-dot-%d" % index, 410, 584, (190, 220, 240))
    flow("cold-dot-%d" % index, -92.0, 0.0, 5.5, 9.6, 1.6, phase=index * 0.55)
for index in range(3):
    dot("steam-dot-%d" % index, 610, 222, (232, 247, 255))
    flow("steam-dot-%d" % index, 310.0, 0.0, 9.2, 10.4, 1.2, phase=index * 0.4)
for index in range(3):
    dot("exhaust-dot-%d" % index, 1045, 662, (232, 247, 255))
    flow("exhaust-dot-%d" % index, 0.0, 128.0, 19.2, 22.0, 1.3, phase=index * 0.42)
for index in range(3):
    dot("feed-dot-%d" % index, 1100, 864, (190, 220, 240))
    flow("feed-dot-%d" % index, -620.0, 0.0, 22.0, 26.2, 2.2, phase=index * 0.7)
for index in range(3):
    dot("cool-dot-%d" % index, 1380, 844, (190, 220, 240))
    flow("cool-dot-%d" % index, 220.0, 0.0, 21.4, 26.0, 1.5, phase=index * 0.5)
for index in range(2):
    dot("cool-dot-b-%d" % index, 1560, 888, (190, 220, 240))
    flow("cool-dot-b-%d" % index, -220.0, 0.0, 21.6, 26.0, 1.5, phase=index * 0.7)
for index in range(4):
    dot("elec-dot-%d" % index, 1610, 438, (255, 249, 214), 12)
    flow("elec-dot-%d" % index, 250.0, -40.0, 16.0, 26.0, 1.4, phase=index * 0.35)
for index in range(4):
    dot("elec-dot-b-%d" % index, 1610, 514, (255, 249, 214), 12)
    flow("elec-dot-b-%d" % index, 250.0, 32.0, 16.2, 26.0, 1.4, phase=index * 0.35)

# ------------------------------------------------------------- closing summary
rect("veil", 0, 0, W, H, fill=BG, opacity=0.0)
add_anim("veil", "opacity", [(0.0, 0.0), (27.0, 0.0), (27.7, 1.0)], "ease_in_out")

center_text("close-kicker", "TÓM LẠI", 286, 24, HEAT)
center_text("close-title", "MỘT CHUỖI CHUYỂN HÓA NĂNG LƯỢNG", 336, 48, INK)
center_text("close-sub", "mỗi mắt lưới chỉ đổi một dạng năng lượng", 404, 24, MUTED)
show("close-kicker", 27.2, 0.5, "ease_out", dy=14)
show("close-title", 27.35, 0.6, "ease_out", dy=18)
show("close-sub", 27.5, 0.6, "ease_out", dy=14)

CHIPS = [
    ("PHÂN HẠCH", HEAT),
    ("NHIỆT", AMBER),
    ("HƠI NƯỚC", STEAM),
    ("QUAY", STEEL),
    ("ĐIỆN", ELEC),
]
CHIP_W, CHIP_H, GAP = 250, 120, 80
start_x = (W - (len(CHIPS) * CHIP_W + (len(CHIPS) - 1) * GAP)) / 2.0
chip_y = 476
for index, (label, color) in enumerate(CHIPS):
    x = start_x + index * (CHIP_W + GAP)
    rect("chip-%d" % index, x, chip_y, CHIP_W, CHIP_H, fill=PANEL2, stroke=color, sw=3)
    text("chip-label-%d" % index, label, x + (CHIP_W - measured_width(label, 26)) / 2.0, chip_y + 47, 26, color)
    show("chip-%d" % index, 27.7 + index * 0.1, 0.55, "ease_out", dy=24)
    show("chip-label-%d" % index, 27.75 + index * 0.1, 0.55, "ease_out", dy=24)
    if index < len(CHIPS) - 1:
        arrow(
            "chip-arrow-%d" % index,
            x + CHIP_W + 12,
            chip_y + CHIP_H / 2,
            x + CHIP_W + GAP - 12,
            chip_y + CHIP_H / 2,
            DIM,
            5,
        )
        show("chip-arrow-%d" % index, 28.05 + index * 0.1, 0.45)

center_text(
    "close-note",
    "Nhiên liệu U-235 · lò nước áp lực (PWR) · chu trình kín, vận hành không phát thải CO₂",
    700,
    23,
    MUTED,
)
show("close-note", 28.5, 0.6, "ease_out", dy=12)

# ------------------------------------------------------------------------ write
scene = {
    "format": "opencut-reinforced-motion-scene",
    "schema_version": 1,
    "scene_id": "7a3f4b2c-91d5-4e6a-8b47-c2d1e5f60a93",
    "name": "Nuclear power plant — how it works",
    "duration": t(DUR),
    "canvas": {"width": W, "height": H},
    "assets": [],
    "nodes": nodes,
    "animations": anims,
}

out = Path(__file__).with_name("scene.ormotion.json")
out.write_text(json.dumps(scene, ensure_ascii=False, indent=1) + "\n", encoding="utf-8")
print("wrote", out, len(nodes), "nodes", len(anims), "animations", out.stat().st_size, "bytes")
