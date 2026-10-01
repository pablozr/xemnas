"""Signed-distance model of the mascot, ray-marched with toon shading (numpy)."""
import numpy as np
from PIL import Image
import sys

S = 512  # render size; downsampled to 256


def length(v):
    return np.sqrt(np.sum(v * v, axis=-1))


def sd_sphere(p, c, r):
    return length(p - c) - r


def sd_ellipsoid(p, c, r):
    q = (p - c) / r
    k0 = length(q)
    k1 = length(q / r)
    return k0 * (k0 - 1.0) / np.maximum(k1, 1e-6)


def sd_capsule(p, a, b, r):
    pa = p - a
    ba = b - a
    h = np.clip(np.sum(pa * ba, -1) / np.dot(ba, ba), 0, 1)[..., None]
    return length(pa - ba * h) - r


def sd_round_cone(p, a, b, r1, r2):
    # vertical round cone from a (radius r1) to b (radius r2)
    pa = p - a
    ba = b - a
    l2 = np.dot(ba, ba)
    rr = r1 - r2
    a2 = l2 - rr * rr
    il2 = 1.0 / l2
    y = np.sum(pa * ba, -1)
    z = y - l2
    xv = pa * l2 - ba * y[..., None]
    x2 = np.sum(xv * xv, -1)
    y2 = y * y * l2
    z2 = z * z * l2
    k = np.sign(rr) * rr * rr * x2
    out = np.where(
        np.sign(z) * a2 * z2 > k, np.sqrt(x2 + z2) * il2 - r2,
        np.where(np.sign(y) * a2 * y2 < k, np.sqrt(x2 + y2) * il2 - r1,
                 (np.sqrt(x2 * a2 * il2) + y * rr) * il2 - r1))
    return out


def smin(a, b, k):
    h = np.clip(0.5 + 0.5 * (b - a) / k, 0, 1)
    return b * (1 - h) + a * h - k * h * (1 - h)


def smax(a, b, k):
    return -smin(-a, -b, k)


V = np.array

# material ids
COAT, FACE, EYE, SILVER, HAIR = 1, 2, 3, 4, 5


def scene(p, t=0.0, blink=1.0):
    # hood: big round head with a soft point at the back-top
    hood = sd_ellipsoid(p, V([0, 1.25, 0]), V([1.02, 0.98, 0.95]))
    tip = sd_capsule(p, V([0, 1.55, -0.25]), V([0, 1.95, -0.75]), 0.18)
    hood = smin(hood, tip, 0.35)
    # face opening: carve the front
    opening = sd_ellipsoid(p, V([0, 1.15, 0.72]), V([0.70, 0.68, 0.55]))
    hood_shell = smax(hood, -opening, 0.06)
    # the face sits recessed inside
    face = sd_ellipsoid(p, V([0, 1.12, -0.02]), V([0.72, 0.70, 0.62]))
    # body: coat as a round cone widening down
    body = sd_round_cone(p, V([0, -0.55, 0]), V([0, 0.55, 0]), 0.78, 0.48)
    body = smax(body, -(p[..., 1] + 0.62), 0.05)  # flat hem
    coat = smin(hood_shell, body, 0.18)
    # arms
    arm_l = sd_capsule(p, V([-0.55, 0.30, 0.05]), V([-0.82, -0.25, 0.18]), 0.19)
    arm_r = sd_capsule(p, V([0.55, 0.30, 0.05]), V([0.82, -0.25, 0.18]), 0.19)
    coat = smin(coat, np.minimum(arm_l, arm_r), 0.12)
    # silver: zipper line, drawstrings with tips, cuffs
    zip_ = sd_capsule(p, V([0, 0.42, 0.62]), V([0, -0.55, 0.80]), 0.028)
    pull = sd_ellipsoid(p, V([0, 0.20, 0.70]), V([0.06, 0.10, 0.04]))
    s1 = sd_capsule(p, V([-0.30, 0.55, 0.66]), V([-0.26, 0.05, 0.70]), 0.03)
    s2 = sd_capsule(p, V([0.30, 0.55, 0.66]), V([0.26, 0.05, 0.70]), 0.03)
    t1 = sd_sphere(p, V([-0.26, 0.0, 0.71]), 0.065)
    t2 = sd_sphere(p, V([0.26, 0.0, 0.71]), 0.065)
    cuff_l = sd_capsule(p, V([-0.78, -0.17, 0.16]), V([-0.84, -0.29, 0.19]), 0.205)
    cuff_r = sd_capsule(p, V([0.78, -0.17, 0.16]), V([0.84, -0.29, 0.19]), 0.205)
    silver = np.minimum.reduce([zip_, pull, s1, s2, t1, t2, cuff_l, cuff_r])
    # silver bangs: tapered locks fanning from the crown of the opening
    locks = [
        ((-0.06, 1.66, 0.52), (-0.46, 1.02, 0.74), 0.12),
        ((0.06, 1.66, 0.52), (0.46, 1.02, 0.74), 0.12),
        ((-0.24, 1.60, 0.50), (-0.66, 1.16, 0.62), 0.10),
        ((0.24, 1.60, 0.50), (0.66, 1.16, 0.62), 0.10),
        ((0.02, 1.68, 0.55), (-0.10, 1.34, 0.80), 0.09),
    ]
    bangs = np.minimum.reduce([
        sd_round_cone(p, V(tip), V(root), 0.025, r) for root, tip, r in locks
    ])
    hair = smax(bangs, hood - 0.01, 0.03)
    # eyes: wide almond slits, tilted toward the nose
    eh = 0.085 * blink + 0.01
    def eye(x, tilt):
        q = p - V([x, 1.06, 0.60])
        c, s_ = np.cos(tilt), np.sin(tilt)
        q = np.stack([q[..., 0] * c - q[..., 1] * s_, q[..., 0] * s_ + q[..., 1] * c, q[..., 2]], -1)
        return sd_ellipsoid(q, V([0, 0, 0]), V([0.15, eh, 0.06]))
    eyes = np.minimum(eye(-0.25, -0.22), eye(0.25, 0.22))

    d = coat
    mat = np.full(d.shape, COAT)
    for dd, m in [(face, FACE), (silver, SILVER), (hair, HAIR), (eyes, EYE)]:
        closer = dd < d
        d = np.where(closer, dd, d)
        mat = np.where(closer, m, mat)
    return d, mat


def normal(p, **kw):
    e = 0.002
    dx = scene(p + V([e, 0, 0]), **kw)[0] - scene(p - V([e, 0, 0]), **kw)[0]
    dy = scene(p + V([0, e, 0]), **kw)[0] - scene(p - V([0, e, 0]), **kw)[0]
    dz = scene(p + V([0, 0, e]), **kw)[0] - scene(p - V([0, 0, e]), **kw)[0]
    n = np.stack([dx, dy, dz], -1)
    return n / np.maximum(length(n)[..., None], 1e-9)


def render(yaw=0.18, bob=0.0, blink=1.0, glow=1.0, size=S):
    ys, xs = np.mgrid[0:size, 0:size]
    u = (xs + 0.5) / size * 2 - 1
    v = 1 - (ys + 0.5) / size * 2
    ro = V([0.0, 0.55 + bob, 6.0])
    fov = 0.42
    rd = np.stack([u * fov, v * fov, -np.ones_like(u)], -1)
    rd /= length(rd)[..., None]
    # rotate the scene by yaw (rotate rays the other way)
    c, s = np.cos(yaw), np.sin(yaw)
    rot = V([[c, 0, s], [0, 1, 0], [-s, 0, c]])
    ro_r = ro @ rot
    rd_r = rd @ rot
    t = np.full(u.shape, 3.0)
    hit = np.zeros(u.shape, bool)
    for _ in range(140):
        p = ro_r + rd_r * t[..., None]
        d, _ = scene(p, blink=blink)
        hit |= d < 0.0015
        t = np.where(hit, t, t + d * 0.9)
        if np.all(hit | (t > 9)):
            break
    p = ro_r + rd_r * t[..., None]
    d, mat = scene(p, blink=blink)
    n = normal(p, blink=blink)
    light = V([-0.5, 0.75, 0.6]); light /= np.linalg.norm(light)
    lam = np.clip(np.sum(n * light, -1), 0, 1)
    view = -rd_r
    rim = np.clip(1 - np.sum(n * view, -1), 0, 1) ** 2.5
    h = light + view
    h /= length(h)[..., None]
    spec = np.clip(np.sum(n * h, -1), 0, 1) ** 40
    # toon ramp
    band = np.where(lam > 0.62, 1.0, np.where(lam > 0.25, 0.62, 0.36))
    col = np.zeros(u.shape + (3,))
    coat_c = V([0.10, 0.10, 0.14])
    col = np.where((mat == COAT)[..., None], coat_c * (0.55 + 0.9 * band)[..., None]
                   + V([0.62, 0.55, 0.95])[None, None] * (rim * 0.55)[..., None]
                   + (spec * 0.35)[..., None], col)
    col = np.where((mat == FACE)[..., None], V([0.025, 0.022, 0.04]) * np.ones_like(col), col)
    silver_c = V([0.80, 0.82, 0.88])
    col = np.where((mat == SILVER)[..., None], silver_c * (0.45 + 0.6 * band)[..., None]
                   + (spec * 0.9)[..., None], col)
    hair_c = V([0.86, 0.87, 0.92])
    col = np.where((mat == HAIR)[..., None], hair_c * (0.5 + 0.55 * band)[..., None]
                   + (spec * 0.5)[..., None], col)
    eye_c = V([1.0, 0.74, 0.30]) * glow
    col = np.where((mat == EYE)[..., None], eye_c * np.ones_like(col), col)
    # glow halo around eyes, in screen space, from distance to eye centers
    alpha = hit.astype(float)
    # bloom: blurred emissive mask added on top (and into alpha)
    from PIL import ImageFilter
    em = ((mat == EYE) & hit).astype(np.uint8) * 255
    blur = np.asarray(Image.fromarray(em).filter(ImageFilter.GaussianBlur(size / 64)), float) / 255
    halo = np.clip(blur * 1.6, 0, 1)
    col = col + eye_c * (halo * 0.55)[..., None]
    # outline: depth / material discontinuity
    depth = np.where(hit, t, 20.0)
    edge = np.zeros(u.shape, bool)
    for dy_, dx_ in [(0, 1), (1, 0), (0, -1), (-1, 0)]:
        sh_d = np.roll(np.roll(depth, dy_, 0), dx_, 1)
        sh_h = np.roll(np.roll(hit, dy_, 0), dx_, 1)
        edge |= (np.abs(sh_d - depth) > 0.12) & (hit | sh_h)
    outline = V([0.03, 0.02, 0.06])
    col = np.where(edge[..., None], outline, col)
    alpha = np.where(edge, 1.0, alpha)
    rgba = np.concatenate([np.clip(col, 0, 1), alpha[..., None]], -1)
    img = Image.fromarray((rgba * 255).astype(np.uint8), 'RGBA')
    return img


if __name__ == '__main__':
    out = sys.argv[1]
    img = render()
    img.resize((256, 256), Image.LANCZOS).save(out)
    print('ok')
