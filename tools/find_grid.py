#!/usr/bin/env python3
"""Find knob/button grid positions in the asset sheets via brightness projection.

For dark control sheets, the brightest pixels are pointer marks and text labels.
Projecting brightness onto each axis reveals the grid structure.
"""
from pathlib import Path
from PIL import Image

ROOT = Path(__file__).resolve().parent.parent


def project(image_path, region, axis_min_brightness=50):
    img = Image.open(image_path).convert("L")
    px = img.load()
    left, top, right, bottom = region

    print(f"\n--- {image_path.name} region {region} ---")

    row_peaks = []
    for y in range(top, bottom):
        s = sum(px[x, y] for x in range(left, right))
        b = s / (right - left)
        if b > axis_min_brightness:
            row_peaks.append((y, b))
    col_peaks = []
    for x in range(left, right):
        s = sum(px[x, y] for y in range(top, bottom))
        b = s / (bottom - top)
        if b > axis_min_brightness:
            col_peaks.append((x, b))

    def cluster(peaks):
        groups = []
        cur = []
        for p, b in peaks:
            if not cur or p - cur[-1][0] <= 4:
                cur.append((p, b))
            else:
                groups.append(cur)
                cur = [(p, b)]
        if cur:
            groups.append(cur)
        return [round(sum(g[0] for g in grp) / len(grp), 1) for grp in groups]

    print(f"  row centers: {cluster(row_peaks)}")
    print(f"  col centers: {cluster(col_peaks)}")


knobs = ROOT / "image-assets" / "synth knobs.png"

# LARGE knobs section
project(knobs, (0, 30, 1100, 410), 50)
# MEDIUM knobs section
project(knobs, (0, 430, 1100, 770), 50)
# SMALL knobs section
project(knobs, (0, 780, 1100, 1086), 50)
# SPECIALTY/CONTROL on right top
project(knobs, (1100, 30, 1448, 410), 50)
# COLORED ENCODER right middle
project(knobs, (1100, 430, 1448, 770), 30)
# MINI knobs right bottom
project(knobs, (1100, 780, 1448, 1086), 30)
