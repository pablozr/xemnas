"""Renders the mascot into apps/desktop-gpui/assets/mascot.

    python tools/mascot/render.py

Smooth motion is a flipbook of close frames played by the clock, so there are
many of them, drawn once here and never again:

- 27 turns of the head, every 0.03 rad from -0.21 (looking right) to 0.57
  (looking left); the resting pose is index 13 (0.18). Neighbours differ by
  under a pixel at the size the app draws, so a turn reads as continuous.
- 4 blink frames (eyes 75, 50, 25 and 2 percent open) at the resting yaw.
- 1 frame with the eyes lit (hover, open panel).

All 32 go into `sheet.png` (8 columns by 4 rows of 160 px cells, cropped to
one common box so frames line up). `idle.png`, `blink.png` and `glow.png`
are the same frames alone, for the portrait, the opening mark and the empty
states. Frames are rendered in parallel; needs numpy and Pillow.
"""
import os
import sys
from concurrent.futures import ProcessPoolExecutor

from PIL import Image

sys.path.insert(0, os.path.dirname(__file__))
from model import render  # noqa: E402

OUT = os.path.join(os.path.dirname(__file__), '..', '..', 'apps', 'desktop-gpui', 'assets', 'mascot')
CELL = 160
COLS = 8
# The part of the 512 px square the figure occupies, with margin for bloom.
REGION = (88, 48, 424, 432)

YAW_FIRST, YAW_STEP, YAW_COUNT, YAW_REST = -0.21, 0.03, 27, 13
BLINK_OPEN = [0.75, 0.50, 0.25, 0.02]


def poses():
    rest = YAW_FIRST + YAW_STEP * YAW_REST
    frames = [dict(yaw=YAW_FIRST + YAW_STEP * k) for k in range(YAW_COUNT)]
    frames += [dict(yaw=rest, blink=b) for b in BLINK_OPEN]
    frames.append(dict(yaw=rest, glow=1.25))
    return frames


def _lower_priority():
    if os.name == 'nt':
        import ctypes
        BELOW_NORMAL = 0x4000
        handle = ctypes.windll.kernel32.GetCurrentProcess()
        ctypes.windll.kernel32.SetPriorityClass(handle, BELOW_NORMAL)


def one(pose):
    return render(**pose, region=REGION).resize((256, 256), Image.LANCZOS)


if __name__ == '__main__':
    frames_in = poses()
    # Two workers at low priority: a render is a one-off, the machine stays usable.
    with ProcessPoolExecutor(max_workers=2, initializer=_lower_priority) as pool:
        frames = list(pool.map(one, frames_in))
    print(len(frames), 'frames rendered', flush=True)

    box = None
    for frame in frames:
        b = frame.getchannel('A').point(lambda a: 255 if a > 8 else 0).getbbox()
        box = b if box is None else (min(box[0], b[0]), min(box[1], b[1]), max(box[2], b[2]), max(box[3], b[3]))
    side = max(box[2] - box[0], box[3] - box[1]) + 12
    cx, cy = (box[0] + box[2]) // 2, (box[1] + box[3]) // 2
    crop = (cx - side // 2, cy - side // 2, cx - side // 2 + side, cy - side // 2 + side)
    cells = [f.crop(crop).resize((CELL, CELL), Image.LANCZOS) for f in frames]

    os.makedirs(OUT, exist_ok=True)
    rows = (len(cells) + COLS - 1) // COLS
    sheet = Image.new('RGBA', (COLS * CELL, rows * CELL), (0, 0, 0, 0))
    for index, cell in enumerate(cells):
        sheet.paste(cell, ((index % COLS) * CELL, (index // COLS) * CELL))
    sheet.save(os.path.join(OUT, 'sheet.png'), optimize=True)
    # Stand-alone portraits at 192 px.
    for name, index in [('idle', YAW_REST), ('blink', YAW_COUNT + len(BLINK_OPEN) - 1), ('glow', len(cells) - 1)]:
        frames[index].crop(crop).resize((192, 192), Image.LANCZOS).save(os.path.join(OUT, name + '.png'), optimize=True)
    print('wrote sheet', sheet.size, flush=True)
