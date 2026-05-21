#!/usr/bin/env python3
"""Slice individual control sprites out of the synth asset sheets.

Coordinates were derived via tools/find_grid.py (brightness projection on text/pointer marks).
"""
from pathlib import Path
from PIL import Image

ROOT = Path(__file__).resolve().parent.parent
ASSETS = ROOT / "image-assets"
OUT = ROOT / "web" / "static" / "sprites"
OUT.mkdir(parents=True, exist_ok=True)


def crop(src_name, out_name, center, size):
    cx, cy = center
    half = size // 2
    box = (cx - half, cy - half, cx + half, cy + half)
    img = Image.open(ASSETS / src_name).convert("RGBA")
    crop = img.crop(box)
    crop.save(OUT / out_name)
    print(f"  {out_name:32s} center={center}  size={size}  -> {crop.size}")


def crop_box(src_name, out_name, box):
    img = Image.open(ASSETS / src_name).convert("RGBA")
    crop = img.crop(box)
    crop.save(OUT / out_name)
    print(f"  {out_name:32s} box={box}  -> {crop.size}")


# Large knobs.
# Brightness projection picked up the bright pointer tips (top of each knob), at y=103, 219, 335.
# The pointer sits at the TOP of the knob body, so the real geometric center is below the
# pointer — empirically ~+30px gives a centered crop.
KNOB_LARGE_SIZE = 100
crop("synth knobs.png", "knob_large_neutral.png",   (161, 130), KNOB_LARGE_SIZE)
crop("synth knobs.png", "knob_large_highlight.png", (161, 245), KNOB_LARGE_SIZE)
crop("synth knobs.png", "knob_large_active.png",    (161, 360), KNOB_LARGE_SIZE)

# Medium knobs — same vertical-offset reasoning.
KNOB_MED_SIZE = 80
crop("synth knobs.png", "knob_medium_neutral.png",  (161, 540), KNOB_MED_SIZE)
crop("synth knobs.png", "knob_medium_active.png",   (161, 720), KNOB_MED_SIZE)

# Colored encoders — these are flatter cap knobs. Row centers: 527, 614, 703.
# Cols: teal 1190, orange 1257, mustard 1324, red 1393.
ENC_SIZE = 80
crop("synth knobs.png", "encoder_teal.png",     (1190, 527), ENC_SIZE)
crop("synth knobs.png", "encoder_orange.png",   (1257, 527), ENC_SIZE)
crop("synth knobs.png", "encoder_mustard.png",  (1324, 527), ENC_SIZE)
crop("synth knobs.png", "encoder_red.png",      (1393, 527), ENC_SIZE)
crop("synth knobs.png", "encoder_orange_active.png", (1257, 703), ENC_SIZE)

# Panel background — full plate so we can use it as a backdrop image.
crop_box("synth overview no knobs.png", "panel_full.png", (0, 0, 1448, 1086))
# A clean wood-textured strip from the very top of the panel for the header band.
crop_box("synth overview no knobs.png", "panel_wood.png", (10, 5, 1438, 95))
# A clean metal panel cell (around the LFO area on the overview).
crop_box("synth overview no knobs.png", "panel_cell.png", (35, 130, 470, 295))

print(f"\nDone. Wrote sprites to {OUT}")
