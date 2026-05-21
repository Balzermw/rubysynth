#!/usr/bin/env python3
"""Annotate a sprite sheet with a coordinate grid so I can eyeball positions.

Usage: tools/inspect_sheet.py image-assets/synth\ knobs.png tmp/knobs_grid.png
"""
import sys
from PIL import Image, ImageDraw, ImageFont

src = sys.argv[1]
dst = sys.argv[2]
step = int(sys.argv[3]) if len(sys.argv) > 3 else 50

img = Image.open(src).convert("RGB")
draw = ImageDraw.Draw(img)
w, h = img.size

for x in range(0, w, step):
    draw.line([(x, 0), (x, h)], fill=(0, 255, 0), width=1)
    draw.text((x + 2, 2), str(x), fill=(0, 255, 0))
for y in range(0, h, step):
    draw.line([(0, y), (w, y)], fill=(0, 255, 0), width=1)
    draw.text((2, y + 2), str(y), fill=(0, 255, 0))

img.save(dst)
print(f"wrote grid overlay to {dst}")
