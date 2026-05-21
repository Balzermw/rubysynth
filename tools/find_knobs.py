#!/usr/bin/env python3
"""Auto-locate knob centers in the knob sheet by finding local minima in a darkness map.

Each knob is a roughly circular dark blob surrounded by lighter background. By smoothing
the image and finding regularly-spaced dark centers we can recover the grid.
"""
from pathlib import Path
from PIL import Image, ImageDraw, ImageFilter

ROOT = Path(__file__).resolve().parent.parent
src_path = ROOT / "image-assets" / "synth knobs.png"
out_path = ROOT / "tmp" / "knobs_annotated.png"
out_path.parent.mkdir(parents=True, exist_ok=True)

img = Image.open(src_path).convert("L")  # grayscale luminance
w, h = img.size

# Heavily smooth so we get one big dark blob per knob.
smooth = img.filter(ImageFilter.GaussianBlur(radius=15))
px = smooth.load()

# Compute a darkness threshold from the image stats.
samples = [px[x, y] for y in range(0, h, 10) for x in range(0, w, 10)]
samples_sorted = sorted(samples)
dark_threshold = samples_sorted[len(samples_sorted) // 10]  # 10th percentile

# Scan a coarse grid for dark cells and group adjacent ones.
GRID = 12
cells = {}
for y in range(0, h, GRID):
    for x in range(0, w, GRID):
        if px[x, y] < dark_threshold:
            cells[(x // GRID, y // GRID)] = True

# Connected components (4-conn) → each component represents one knob.
seen = set()
blobs = []
for key in cells:
    if key in seen:
        continue
    stack = [key]
    component = []
    while stack:
        c = stack.pop()
        if c in seen or c not in cells:
            continue
        seen.add(c)
        component.append(c)
        cx, cy = c
        stack.extend([(cx + 1, cy), (cx - 1, cy), (cx, cy + 1), (cx, cy - 1)])
    if component:
        xs = [c[0] for c in component]
        ys = [c[1] for c in component]
        # Filter out very small components (noise) and very large (entire sections).
        size = len(component)
        if 6 < size < 200:
            cx = (sum(xs) / len(xs)) * GRID + GRID / 2
            cy = (sum(ys) / len(ys)) * GRID + GRID / 2
            bw = (max(xs) - min(xs) + 1) * GRID
            bh = (max(ys) - min(ys) + 1) * GRID
            blobs.append((cx, cy, bw, bh, size))

# Annotate.
anno = Image.open(src_path).convert("RGB")
draw = ImageDraw.Draw(anno)
for i, (cx, cy, bw, bh, sz) in enumerate(blobs):
    r = max(bw, bh) / 2
    draw.ellipse([cx - r, cy - r, cx + r, cy + r], outline=(255, 0, 0), width=2)
    draw.text((cx + r + 2, cy - 6), f"{i}", fill=(255, 0, 0))
anno.save(out_path)

print(f"found {len(blobs)} blob candidates; annotated -> {out_path}")
# Dump structured data so we can use it from the slice script.
import json
data_path = ROOT / "tmp" / "knob_blobs.json"
data_path.write_text(json.dumps([
    {"i": i, "cx": round(cx, 1), "cy": round(cy, 1), "bw": bw, "bh": bh, "size": sz}
    for i, (cx, cy, bw, bh, sz) in enumerate(blobs)
], indent=2))
print(f"wrote blob list -> {data_path}")
