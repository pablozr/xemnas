"""Renders the built-in backgrounds into apps/desktop-gpui/assets/wallpapers.

    python tools/wallpapers/render.py

Four original images inspired by the Organization's world (a homage: no
official art, logo or emblem is reproduced):

- never     the dark city that never was: towers, a huge pale moon, rain
- castle    a white castle of thin spires floating in a violet night
- thirteen  thirteen tall white thrones in a circle, in the void
- chain     silver chain links and amber embers on black

Each is saved at 2560x1440 (JPEG) plus a 320x180 thumbnail. Deterministic
(fixed seeds). Needs numpy and Pillow.
"""
import math
import os

import numpy as np
from PIL import Image, ImageDraw, ImageFilter

W, H = 2560, 1440
OUT = os.path.join(os.path.dirname(__file__), '..', '..', 'apps', 'desktop-gpui', 'assets', 'wallpapers')


def gradient(top, bottom, curve=1.0):
    t = np.linspace(0, 1, H)[:, None] ** curve
    top, bottom = np.array(top, float), np.array(bottom, float)
    img = top[None, None, :] * (1 - t[..., None]) + bottom[None, None, :] * t[..., None]
    return np.repeat(img, W, axis=1)


def radial(cx, cy, radius, color, strength=1.0, falloff=2.0):
    ys, xs = np.mgrid[0:H, 0:W]
    d = np.sqrt((xs - cx) ** 2 + (ys - cy) ** 2) / radius
    a = np.clip(1 - d, 0, 1) ** falloff * strength
    return a[..., None] * np.array(color, float)[None, None, :]


def grain(img, amount, seed):
    rng = np.random.default_rng(seed)
    return img + rng.normal(0, amount, img.shape[:2])[..., None]


def to_image(arr):
    return Image.fromarray(np.clip(arr, 0, 255).astype(np.uint8), 'RGB')


def save(name, img):
    os.makedirs(OUT, exist_ok=True)
    img.save(os.path.join(OUT, name + '.jpg'), quality=88, optimize=True, progressive=True)
    img.resize((320, 180), Image.LANCZOS).save(os.path.join(OUT, name + '-thumb.jpg'), quality=85)
    print(name, flush=True)


def never():
    rng = np.random.default_rng(13)
    sky = gradient((10, 12, 30), (22, 20, 44), 0.8)
    sky += radial(W * 0.68, H * 0.30, 900, (60, 60, 90), 0.55, 1.6)
    img = to_image(sky)
    # the moon: huge, pale, soft-edged, slightly cratered
    moon = Image.new('L', (W, H), 0)
    d = ImageDraw.Draw(moon)
    mx, my, mr = W * 0.68, H * 0.30, 250
    d.ellipse((mx - mr, my - mr, mx + mr, my + mr), fill=255)
    moon = moon.filter(ImageFilter.GaussianBlur(3))
    tex = np.asarray(moon, float) / 255
    ys, xs = np.mgrid[0:H, 0:W]
    shade = 0.82 + 0.18 * np.clip(1 - np.hypot(xs - (mx - 80), ys - (my - 90)) / (mr * 1.6), 0, 1)
    craters = np.ones((H, W))
    for _ in range(14):
        cx_, cy_ = mx + rng.uniform(-mr, mr) * 0.7, my + rng.uniform(-mr, mr) * 0.7
        cr = rng.uniform(18, 60)
        craters -= 0.06 * np.clip(1 - np.hypot(xs - cx_, ys - cy_) / cr, 0, 1)
    base = np.asarray(img, float)
    moon_col = np.array([226, 224, 236])[None, None, :] * (shade * craters)[..., None]
    base = base * (1 - tex[..., None]) + moon_col * tex[..., None]
    base += radial(mx, my, mr * 2.6, (120, 118, 160), 0.35, 1.8)
    img = to_image(base)
    d = ImageDraw.Draw(img)
    # three layers of towers, far to near
    for layer, (color, top_min, top_max, width, windows) in enumerate([
        ((24, 24, 44), 0.42, 0.62, (40, 120), 0.0),
        ((15, 15, 30), 0.30, 0.58, (60, 170), 0.010),
        ((8, 8, 16), 0.18, 0.50, (90, 230), 0.018),
    ]):
        x = -40
        while x < W:
            w = rng.integers(*width)
            top = int(H * rng.uniform(top_min, top_max))
            d.rectangle((x, top, x + w, H), fill=color)
            if rng.random() < 0.35:  # a spire
                d.polygon([(x + w * 0.35, top), (x + w * 0.5, top - rng.integers(40, 160)), (x + w * 0.65, top)], fill=color)
            for wy in range(top + 14, H, 18):
                for wx in range(x + 8, x + w - 8, 14):
                    if rng.random() < windows:
                        glow = (150, 170, 215) if rng.random() < 0.8 else (230, 214, 170)
                        d.rectangle((wx, wy, wx + 4, wy + 6), fill=glow)
            x += w + rng.integers(4, 30)
    arr = np.asarray(img, float)
    # fog at the street, rain across everything
    fog = np.linspace(0, 1, H)[:, None] ** 3
    arr = arr * (1 - 0.35 * fog[..., None]) + np.array([40, 40, 70])[None, None] * 0.35 * fog[..., None]
    rain = Image.new('L', (W, H), 0)
    rd = ImageDraw.Draw(rain)
    for _ in range(2600):
        x0, y0 = rng.uniform(0, W), rng.uniform(-100, H)
        length = rng.uniform(30, 90)
        rd.line((x0, y0, x0 - length * 0.18, y0 + length), fill=int(rng.uniform(40, 110)), width=1)
    arr += (np.asarray(rain.filter(ImageFilter.GaussianBlur(0.6)), float) / 255)[..., None] * np.array([150, 160, 200])[None, None] * 0.35
    save('never', to_image(grain(arr, 3.0, 1)))


def castle():
    rng = np.random.default_rng(7)
    sky = gradient((14, 10, 26), (34, 22, 52), 1.2)
    sky += radial(W * 0.5, H * 0.42, 1100, (90, 70, 140), 0.45, 1.5)
    img = to_image(sky)
    # stars
    d = ImageDraw.Draw(img)
    for _ in range(500):
        x, y = rng.uniform(0, W), rng.uniform(0, H * 0.7)
        b = int(rng.uniform(90, 220))
        d.point((x, y), fill=(b, b, min(255, b + 20)))
    # the castle: a cluster of thin white spires on a floating base
    mask = Image.new('L', (W, H), 0)
    m = ImageDraw.Draw(mask)
    cx, base_y = W * 0.5, H * 0.66
    for i in range(23):
        offset = (i - 11) * rng.uniform(24, 34) + rng.uniform(-12, 12)
        height = 520 * math.exp(-((i - 11) / 7.5) ** 2) + rng.uniform(60, 200)
        w = rng.uniform(20, 46)
        x = cx + offset
        top = base_y - height
        m.rectangle((x - w / 2, top, x + w / 2, base_y), fill=255)
        m.polygon([(x - w / 2, top), (x, top - rng.uniform(40, 140)), (x + w / 2, top)], fill=255)
        if rng.random() < 0.5:  # a bridge
            m.rectangle((x, top + height * 0.4, x + rng.uniform(30, 80), top + height * 0.4 + 8), fill=255)
    m.polygon([(cx - 420, base_y), (cx + 420, base_y), (cx + 160, base_y + 170), (cx, base_y + 240), (cx - 180, base_y + 160)], fill=255)
    # floating fragments around the base
    for _ in range(26):
        fx, fy = cx + rng.uniform(-620, 620), base_y + rng.uniform(-80, 300)
        fs = rng.uniform(8, 34)
        m.polygon([(fx - fs, fy), (fx, fy - fs * 0.6), (fx + fs, fy), (fx, fy + fs * 1.4)], fill=255)
    glow = mask.filter(ImageFilter.GaussianBlur(40))
    arr = np.asarray(img, float)
    g = np.asarray(glow, float)[..., None] / 255
    arr += g * np.array([150, 140, 200])[None, None] * 0.6
    body = np.asarray(mask.filter(ImageFilter.GaussianBlur(1.2)), float)[..., None] / 255
    ys, xs = np.mgrid[0:H, 0:W]
    # moonlight from the upper left: spires bright on top, cooler below,
    # and a soft vertical banding that reads as faceted towers
    facets = 0.95 + 0.05 * np.sin(xs / 11.0) * np.sin(xs / 37.0)
    light = (np.clip(1 - (ys - (base_y - 700)) / 1000, 0.5, 1.0) * facets)[..., None]
    arr = arr * (1 - body) + np.array([232, 230, 242])[None, None] * light * body
    # mist beneath
    arr += radial(cx, base_y + 300, 900, (70, 60, 110), 0.5, 2.2)
    save('castle', to_image(grain(arr, 3.0, 2)))


def thirteen():
    """Thirteen white thrones of different heights on a round floor, seen
    from the edge of the circle, lit from far above."""
    sky = gradient((6, 6, 10), (14, 13, 20), 1.0)
    arr = sky + radial(W * 0.5, -300, 1600, (120, 120, 150), 0.28, 1.6)
    # the round floor: a pale ellipse fading into the dark
    cx, cy, rx, ry = W * 0.5, H * 0.74, W * 0.40, H * 0.15
    ys, xs = np.mgrid[0:H, 0:W]
    inside = ((xs - cx) / rx) ** 2 + ((ys - cy) / ry) ** 2
    floor = np.clip(1.15 - inside, 0, 1) ** 1.5
    arr += floor[..., None] * np.array([70, 70, 88])[None, None]
    ring = np.exp(-((np.sqrt(inside) - 1.0) / 0.012) ** 2)
    arr += ring[..., None] * np.array([110, 110, 135])[None, None] * 0.6
    img = to_image(arr)
    shaft = Image.new('L', (W, H), 0)
    ImageDraw.Draw(shaft).polygon([(W * 0.45, 0), (W * 0.55, 0), (W * 0.62, cy), (W * 0.38, cy)], fill=55)
    arr = np.asarray(img, float) + (np.asarray(shaft.filter(ImageFilter.GaussianBlur(90)), float) / 255)[..., None] * np.array([170, 170, 200])[None, None]
    img = to_image(arr).convert('RGBA')
    layer = Image.new('RGBA', (W, H), (0, 0, 0, 0))
    d = ImageDraw.Draw(layer)
    heights = [1.0, 0.62, 0.78, 0.55, 0.86, 0.6, 0.72, 0.5, 0.92, 0.58, 0.8, 0.54, 0.68]
    thrones = []
    for i in range(13):
        a = -math.pi / 2 + i * 2 * math.pi / 13
        x, y = cx + rx * 0.92 * math.cos(a), cy + ry * 0.92 * math.sin(a)
        depth = (y - (cy - ry)) / (2 * ry)
        thrones.append((depth, x, y, heights[i]))
    for depth, x, y, h in sorted(thrones):
        s_ = 0.42 + 0.7 * depth
        back_w, back_h = 64 * s_, 760 * h * s_
        seat_w, seat_h = 128 * s_, 70 * s_
        lit = int(170 + 70 * depth)
        side = (lit - 48, lit - 46, lit - 36, 255)
        face = (lit, lit, min(255, lit + 10), 255)
        top = y - seat_h - back_h
        # backrest with a pointed crown
        d.rectangle((x - back_w / 2, top, x + back_w / 2, y - seat_h), fill=face)
        d.polygon([(x - back_w / 2, top), (x, top - 70 * s_), (x + back_w / 2, top)], fill=face)
        d.rectangle((x + back_w * 0.18, top, x + back_w / 2, y - seat_h), fill=side)
        # seat and arms
        d.rectangle((x - seat_w / 2, y - seat_h, x + seat_w / 2, y), fill=face)
        d.rectangle((x - seat_w / 2, y - seat_h * 1.9, x - seat_w / 2 + 16 * s_, y - seat_h), fill=face)
        d.rectangle((x + seat_w / 2 - 16 * s_, y - seat_h * 1.9, x + seat_w / 2, y - seat_h), fill=side)
        d.rectangle((x - seat_w / 2, y - 10 * s_, x + seat_w / 2, y), fill=side)
        # a faint reflection on the floor
        d.rectangle((x - seat_w / 2, y, x + seat_w / 2, y + seat_h * 1.4), fill=(lit, lit, lit + 10, 26))
    glow = layer.filter(ImageFilter.GaussianBlur(36))
    r, g, b, a = glow.split()
    img.alpha_composite(Image.merge('RGBA', (r, g, b, a.point(lambda v: v // 3))))
    img.alpha_composite(layer.filter(ImageFilter.GaussianBlur(0.8)))
    save('thirteen', to_image(grain(np.asarray(img.convert('RGB'), float), 3.5, 3)))


def chain():
    rng = np.random.default_rng(3)
    arr = gradient((8, 8, 10), (14, 12, 16), 1.0)
    arr += radial(W * 0.3, H * 0.4, 1100, (40, 40, 52), 0.5, 1.6)
    img = to_image(arr)
    links = Image.new('RGBA', (W, H), (0, 0, 0, 0))
    d = ImageDraw.Draw(links)
    angle = math.radians(-24)
    ux, uy = math.cos(angle), math.sin(angle)
    for strand, (ox, oy, size) in enumerate([(0, H * 0.62, 1.0), (300, H * 0.28, 0.6)]):
        step = 92 * size
        for k in range(-4, 40):
            x = ox + ux * step * k
            y = oy + uy * step * k
            flat = k % 2 == 0
            w, h = (120 * size, 56 * size) if flat else (36 * size, 70 * size)
            shade = 200 if flat else 150
            col = (shade, shade + 4, shade + 14, 255)
            box = (x - w / 2, y - h / 2, x + w / 2, y + h / 2)
            d.rounded_rectangle(box, radius=h / 2 if flat else w / 2, outline=col, width=max(3, int(11 * size)))
    glint = links.filter(ImageFilter.GaussianBlur(14))
    img = img.convert('RGBA')
    img.alpha_composite(Image.merge('RGBA', list(glint.split()[:3]) + [glint.split()[3].point(lambda v: v // 3)]))
    img.alpha_composite(links.filter(ImageFilter.GaussianBlur(0.9)))
    arr = np.asarray(img.convert('RGB'), float)
    # embers rising
    embers = np.zeros((H, W))
    for _ in range(340):
        x, y = int(rng.uniform(0, W)), int(rng.uniform(H * 0.2, H))
        r = rng.uniform(1.5, 4.5)
        ys, xs = np.ogrid[max(0, y - 12):min(H, y + 12), max(0, x - 12):min(W, x + 12)]
        embers[max(0, y - 12):min(H, y + 12), max(0, x - 12):min(W, x + 12)] += np.clip(1 - np.hypot(xs - x, ys - y) / r, 0, 1) * rng.uniform(0.4, 1)
    blurred = np.asarray(Image.fromarray(np.clip(embers * 255, 0, 255).astype(np.uint8)).filter(ImageFilter.GaussianBlur(3)), float) / 255
    arr += (embers + blurred * 1.5)[..., None] * np.array([242, 170, 80])[None, None] * 0.8
    save('chain', to_image(grain(arr, 3.0, 4)))


if __name__ == '__main__':
    never()
    castle()
    thirteen()
    chain()
