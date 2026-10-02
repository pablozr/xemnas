"""Renders the mascot sprites into apps/desktop-gpui/assets/mascot.

    python tools/mascot/render.py

Each pose is ray-marched from the signed-distance model in `model.py` with
toon shading, an outline and a bloom on the eyes, then every pose is cropped
to the same square (the union of their silhouettes) so frames line up, and
saved at 192 px. Takes about half a minute per pose; needs numpy and Pillow.
"""
import os
import sys

from PIL import Image

sys.path.insert(0, os.path.dirname(__file__))
from model import render  # noqa: E402

OUT = os.path.join(os.path.dirname(__file__), '..', '..', 'apps', 'desktop-gpui', 'assets', 'mascot')
# Smooth motion is made of many close frames, not a few far ones: the app
# crossfades between neighbours. Yaw runs right (-0.20) to left (0.55) with
# the idle pose (0.18) between; the blink has a half-closed frame.
POSES = {
    'idle': dict(yaw=0.18),
    'blink': dict(yaw=0.18, blink=0.05),
    'blinkhalf': dict(yaw=0.18, blink=0.45),
    'glow': dict(yaw=0.18, glow=1.25),
    'left': dict(yaw=0.55),
    'leftmid': dict(yaw=0.37),
    'right': dict(yaw=-0.20),
    'rightmid': dict(yaw=-0.01),
}
# The part of the 512 px square the figure occupies, with margin for bloom.
REGION = (88, 48, 424, 432)

frames = {}
for name, pose in POSES.items():
    frames[name] = render(**pose, region=REGION).resize((256, 256), Image.LANCZOS)
    print(name, flush=True)

box = None
for frame in frames.values():
    b = frame.getchannel('A').point(lambda a: 255 if a > 8 else 0).getbbox()
    box = b if box is None else (min(box[0], b[0]), min(box[1], b[1]), max(box[2], b[2]), max(box[3], b[3]))
side = max(box[2] - box[0], box[3] - box[1]) + 12
cx, cy = (box[0] + box[2]) // 2, (box[1] + box[3]) // 2
crop = (cx - side // 2, cy - side // 2, cx - side // 2 + side, cy - side // 2 + side)
os.makedirs(OUT, exist_ok=True)
for name, frame in frames.items():
    frame.crop(crop).resize((192, 192), Image.LANCZOS).save(os.path.join(OUT, name + '.png'), optimize=True)
