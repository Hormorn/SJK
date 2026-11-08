#!/usr/bin/env python3
"""Draw the falling blossoms' petal pictures (`crates/sjk-viewer/assets/petals`,
`saber_petals.rs`): one cherry blossom petal, a rounded teardrop with the little notch at its
end, pale at its base and blushing toward its tip, in three tints; and the Thousand Cranes'
paper crane, white paper in three shades that the particle tints (25/10/2026); and the
Hearth's ash, a ragged grey flake, plain and with an edge still glowing; and the Sun's
swing ember, a soft white spark (26/10/2026); and the Moon's moondust, a speck of white light
with four short rays that the particle tints and adds (26/10/2026).

    python3 scripts/petal_assets.py
"""
import math
from pathlib import Path
from PIL import Image

OUT = Path(__file__).resolve().parent.parent / "crates/sjk-viewer/assets/petals"
SIZE = 64
SAMPLES = 4
TINTS = {
    "blossom_pink": ((1.0, 0.86, 0.92), (1.0, 0.45, 0.68)),
    "blossom_pale": ((1.0, 0.95, 0.97), (1.0, 0.66, 0.82)),
    "blossom_white": ((1.0, 1.0, 1.0), (0.96, 0.84, 0.95)),
}


def inside(x, y):
    """Whether (x, y) in -1..1 (y from base -1 to tip 1) is in the petal, and how far along."""
    t = (y + 1.0) / 2.0
    if not 0.0 <= t <= 1.0:
        return False
    # a teardrop: narrow at the base, widest toward the tip, rounded there
    half = 0.8 * math.sin(math.pi * min(t, 0.999) ** 1.7) ** 0.8
    if abs(x) > half:
        return False
    # the notch at the tip
    if t > 0.8 and abs(x) < 0.16 * (t - 0.8) / 0.2 * 1.6 and t > 1.0 - 0.14 * (1.0 - abs(x) / 0.26):
        return False
    return True


def petal(base, tip):
    img = Image.new("RGBA", (SIZE, SIZE))
    px = img.load()
    for j in range(SIZE):
        for i in range(SIZE):
            hits, along = 0, 0.0
            for sj in range(SAMPLES):
                for si in range(SAMPLES):
                    x = ((i + (si + 0.5) / SAMPLES) / SIZE) * 2.0 - 1.0
                    y = 1.0 - ((j + (sj + 0.5) / SAMPLES) / SIZE) * 2.0
                    if inside(x * 1.1, y * 1.05):
                        hits += 1
                        along += (y + 1.0) / 2.0
            if not hits:
                px[i, j] = (0, 0, 0, 0)
                continue
            t = along / hits
            # a faint vein down the middle
            vein = 0.06 * math.exp(-(((i + 0.5) / SIZE * 2.0 - 1.0) / 0.05) ** 2) * (1.0 - t)
            c = [min(1.0, (b + (p - b) * t ** 1.4) - vein) for b, p in zip(base, tip)]
            a = hits / SAMPLES ** 2
            px[i, j] = tuple(int(round(v * 255)) for v in c) + (int(round(a * 235)),)
    return img


def triangle(p, a, b, c):
    """Whether p is inside the triangle a, b, c (either winding)."""
    def side(u, v, w):
        return (v[0] - u[0]) * (w[1] - u[1]) - (v[1] - u[1]) * (w[0] - u[0])
    d1, d2, d3 = side(a, b, p), side(b, c, p), side(c, a, p)
    return not ((d1 < 0 or d2 < 0 or d3 < 0) and (d1 > 0 or d2 > 0 or d3 > 0))


# The crane as the blade's shader draws it (`thousand_cranes.glsl`, `tc_crane`), from its side
# with its wings up: each piece, its triangles and its shade of white paper, back to front.
FLAP = 0.8
CRANE = [
    (0.58, [((-0.22, 0.08), (0.36, 0.08), (0.12, 0.06 + 0.86 * FLAP))]),
    (0.82, [((0.14, 0.04), (0.36, -0.03), (0.7, 0.46)),
            ((0.63, 0.39), (0.72, 0.48), (0.88, 0.35)),
            ((-0.14, 0.04), (-0.36, -0.03), (-0.76, 0.42))]),
    (0.94, [((-0.46, 0.0), (0.0, 0.13), (0.46, 0.0)),
            ((-0.46, 0.0), (0.46, 0.0), (0.0, -0.17))]),
    (1.0, [((-0.36, 0.06), (0.3, 0.06), (-0.02, 0.06 + 0.94 * FLAP))]),
]


def crane_shade(x, y):
    """The paper's shade at (x, y) in -1..1, `None` outside the crane."""
    shade = None
    for tone, triangles in CRANE:
        if any(triangle((x, y), *t) for t in triangles):
            shade = tone
    if shade is None:
        return None
    # the body's crease and the near wing's fold, darker lines of the paper
    if shade == 0.94 and abs(y - 0.01) < 0.03 and abs(x) < 0.42:
        shade *= 0.74
    if shade == 1.0:
        ax, ay, bx, by = 0.0, 0.06, -0.02, 0.06 + 0.94 * FLAP
        t = max(0.0, min(1.0, ((x - ax) * (bx - ax) + (y - ay) * (by - ay)) / ((bx - ax) ** 2 + (by - ay) ** 2)))
        if math.hypot(x - ax - (bx - ax) * t, y - ay - (by - ay) * t) < 0.025:
            shade *= 0.8
    return shade


def crane():
    img = Image.new("RGBA", (SIZE, SIZE))
    px = img.load()
    for j in range(SIZE):
        for i in range(SIZE):
            hits, total = 0, 0.0
            for sj in range(SAMPLES):
                for si in range(SAMPLES):
                    x = ((i + (si + 0.5) / SAMPLES) / SIZE) * 2.0 - 1.0
                    y = 1.0 - ((j + (sj + 0.5) / SAMPLES) / SIZE) * 2.0
                    # the crane is about 1.65 across and 1.25 high: centred in the picture
                    shade = crane_shade(x * 1.0, y * 1.0 + 0.3)
                    if shade is not None:
                        hits += 1
                        total += shade
            if not hits:
                px[i, j] = (0, 0, 0, 0)
                continue
            v = total / hits
            px[i, j] = (int(round(v * 255)),) * 3 + (int(round(hits / SAMPLES ** 2 * 255)),)
    return img


OUT.mkdir(parents=True, exist_ok=True)
for name, (base, tip) in TINTS.items():
    petal(base, tip).save(OUT / f"{name}.png", optimize=True)
    print("written", name)
crane().save(OUT / "crane.png", optimize=True)
crane().transpose(Image.FLIP_LEFT_RIGHT).save(OUT / "crane_left.png", optimize=True)
print("written crane, crane_left")


def ash(glow):
    """A ragged flake of ash: grey, darker at its heart, its edge orange where it still burns."""
    rng = [0.82, 0.95, 0.7, 1.0, 0.78, 0.9, 0.66, 0.97, 0.84, 0.74, 0.92, 0.8]
    img = Image.new("RGBA", (SIZE, SIZE))
    px = img.load()
    for j in range(SIZE):
        for i in range(SIZE):
            hits, edge = 0, 0.0
            for sj in range(SAMPLES):
                for si in range(SAMPLES):
                    x = ((i + (si + 0.5) / SAMPLES) / SIZE) * 2.0 - 1.0
                    y = 1.0 - ((j + (sj + 0.5) / SAMPLES) / SIZE) * 2.0
                    a = math.atan2(y, x) / (2.0 * math.pi) * len(rng)
                    k = int(math.floor(a)) % len(rng)
                    t = a - math.floor(a)
                    reach = 0.8 * (rng[k] * (1.0 - t) + rng[(k + 1) % len(rng)] * t)
                    r = math.hypot(x, y * 1.25)
                    if r < reach:
                        hits += 1
                        edge += r / reach
            if not hits:
                px[i, j] = (0, 0, 0, 0)
                continue
            e = edge / hits
            grey = 0.34 + 0.16 * e
            c = [grey, grey * 0.95, grey * 0.9]
            if glow:
                burn = max(0.0, (e - 0.72) / 0.28) ** 1.5
                c = [c[0] + (1.0 - c[0]) * burn, c[1] + (0.5 - c[1]) * burn, c[2] + (0.12 - c[2]) * burn]
            px[i, j] = tuple(int(round(min(v, 1.0) * 255)) for v in c) + (int(round(hits / SAMPLES ** 2 * 230)),)
    return img


ash(False).save(OUT / "ash_grey.png", optimize=True)
ash(True).save(OUT / "ash_ember.png", optimize=True)
print("written ash_grey, ash_ember")


def ember():
    """The Sun's swing ember: a white spark, hottest at its heart, its glow falling off softly
    (the particle tints it from white gold to deep red and it is drawn added)."""
    img = Image.new("RGBA", (SIZE, SIZE))
    px = img.load()
    for j in range(SIZE):
        for i in range(SIZE):
            x = (i + 0.5) / SIZE * 2.0 - 1.0
            y = (j + 0.5) / SIZE * 2.0 - 1.0
            r = math.hypot(x, y)
            glow = math.exp(-r * r * 9.0) * 0.75 + math.exp(-r * r * 60.0) * 0.6
            glow *= max(0.0, 1.0 - r) ** 0.5
            core = min(1.0, math.exp(-r * r * 60.0) * 1.2)
            px[i, j] = (255, int(round(255 * (0.85 + 0.15 * core))), int(round(255 * (0.7 + 0.3 * core))),
                        int(round(min(glow, 1.0) * 255)))
    return img


ember().save(OUT / "ember.png", optimize=True)
print("written ember")


def moon_dust():
    """A speck of moondust with four short rays: white light, its alpha how bright it is."""
    img = Image.new("RGBA", (SIZE, SIZE))
    px = img.load()
    for j in range(SIZE):
        for i in range(SIZE):
            light = 0.0
            for sj in range(SAMPLES):
                for si in range(SAMPLES):
                    x = ((i + (si + 0.5) / SAMPLES) / SIZE) * 2.0 - 1.0
                    y = 1.0 - ((j + (sj + 0.5) / SAMPLES) / SIZE) * 2.0
                    r = math.hypot(x, y)
                    rays = math.exp(-(abs(x) / 0.05) ** 2) * math.exp(-abs(y) * 3.0) + math.exp(-(abs(y) / 0.05) ** 2) * math.exp(-abs(x) * 3.0)
                    light += min(1.0, math.exp(-(r / 0.14) ** 2) + 0.4 * math.exp(-(r / 0.4) ** 2) + 0.7 * rays)
            v = min(1.0, light / SAMPLES ** 2)
            px[i, j] = (255, 255, 255, int(round(v * 255)))
    return img


moon_dust().save(OUT / "moon_dust.png", optimize=True)
print("written moon_dust")
