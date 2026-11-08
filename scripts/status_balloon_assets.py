#!/usr/bin/env python3
"""Draw the status balloons' pictures (`crates/sjk-viewer/assets/status_balloons`,
`status_balloons.rs`, 08/11/2026): the stock talk balloon's shape and look (a black oval
with a glowing rim and a tail at the lower left, 128 pixels square like
`gfx/mp/chat_icon`), drawn anew, holding what the player is doing:

- typing: the three dots, lit one after the other (three frames, an `animMap`);
- busy (a menu or the console): a pause sign;
- away (the window alt-tabbed or minimised): a crescent moon and a z rising beside it
  (three frames), in a cool blue so it reads apart from the others from afar.

    python3 scripts/status_balloon_assets.py [preview.png]

With a path, also writes a contact sheet of every frame over a grey ground.
"""
import math
import sys
from pathlib import Path
from PIL import Image, ImageChops, ImageDraw, ImageFilter

OUT = Path(__file__).resolve().parent.parent / "crates/sjk-viewer/assets/status_balloons"
SIZE = 128
SS = 4  # supersampling
BIG = SIZE * SS
AMBER = (255, 166, 30)
BLUE = (120, 176, 255)
RIM = (236, 232, 226)

# The balloon, in the stock picture's place: the oval and its tail.
OVAL = (18.0, 40.5, 116.0, 87.0)
TAIL = [(51.0, 82.0), (63.0, 85.5), (57.0, 89.5), (46.5, 97.0), (50.0, 88.0)]


def s(points):
    return [(x * SS, y * SS) for x, y in points]


def mask_of(draw_fn):
    m = Image.new("L", (BIG, BIG), 0)
    draw_fn(ImageDraw.Draw(m))
    return m


def shape_mask():
    def draw(d):
        d.ellipse([c * SS for c in OVAL], fill=255)
        d.polygon(s(TAIL), fill=255)
    return mask_of(draw)


def edge_of(mask, width):
    """A band `width` pixels (at full size) wide along the inside of the mask's edge."""
    shrunk = mask.filter(ImageFilter.MinFilter(int(width * SS) * 2 + 1))
    return ImageChops.subtract(mask, shrunk)


def solid(colour, alpha):
    layer = Image.new("RGBA", (BIG, BIG), colour + (0,))
    layer.putalpha(alpha)
    return layer


def scaled(alpha, factor):
    return alpha.point(lambda v: min(255, int(v * factor)))


def glow(mask, colour, radius, strength):
    return solid(colour, scaled(mask.filter(ImageFilter.GaussianBlur(radius * SS)), strength))


def balloon(accent):
    """The empty balloon: shadow, black body, the accent's glow inside the rim, the rim."""
    shape = shape_mask()
    img = Image.new("RGBA", (BIG, BIG), (0, 0, 0, 0))
    # a faint dark shadow around it, as the stock picture's
    img.alpha_composite(solid((0, 0, 0), scaled(shape.filter(ImageFilter.GaussianBlur(4 * SS)), 0.5)))
    img.alpha_composite(solid((0, 0, 0), shape))
    rim = edge_of(shape, 1.1)
    inner = ImageChops.multiply(rim.filter(ImageFilter.GaussianBlur(3.0 * SS)), shape)
    img.alpha_composite(solid(accent, scaled(inner, 1.6)))
    img.alpha_composite(glow(rim, accent, 1.6, 1.4))
    img.alpha_composite(solid(RIM, rim))
    return img


def lit(img, mask, accent, core=(255, 255, 255), halo=2.2, strength=1.8):
    """A glyph: a glow in the accent colour and a bright core."""
    img.alpha_composite(glow(mask, accent, halo, strength))
    img.alpha_composite(solid(core, mask))


def dots(accent, on):
    img = balloon(accent)
    for i, x in enumerate((51.0, 65.0, 79.0)):
        r = 3.1 if i == on else 2.6
        m = mask_of(lambda d: d.ellipse([(x - r) * SS, (66 - r) * SS, (x + r) * SS, (66 + r) * SS], fill=255))
        if i == on:
            lit(img, m, accent, halo=2.6, strength=2.2)
        else:
            dim = tuple(int(c * 0.7) for c in accent)
            lit(img, m, accent, core=dim, halo=1.4, strength=0.8)
    return img


def pause(accent):
    img = balloon(accent)

    def draw(d):
        for x in (56.5, 68.5):
            d.rounded_rectangle([x * SS, 54.5 * SS, (x + 5.5) * SS, 74.5 * SS], radius=1.6 * SS, fill=255)
    lit(img, mask_of(draw), accent)
    return img


def z_glyph(d, cx, cy, size, width):
    h = size / 2.0
    pts = [(cx - h, cy - h), (cx + h, cy - h), (cx - h, cy + h), (cx + h, cy + h)]
    d.line(s(pts), fill=255, width=int(width * SS), joint="curve")


def moon(accent, frame):
    """The moon, and one z rising beside it, a step higher and larger each frame: three z's
    drawn at once blurred into a zig-zag at the balloon's size in game."""
    img = balloon(accent)

    def crescent(d):
        d.ellipse([(55 - 12.5) * SS, (64 - 12.5) * SS, (55 + 12.5) * SS, (64 + 12.5) * SS], fill=255)
        d.ellipse([(62 - 11) * SS, (59 - 11) * SS, (62 + 11) * SS, (59 + 11) * SS], fill=0)
    lit(img, mask_of(crescent), accent, core=(232, 240, 255), halo=2.4, strength=1.6)
    cx, cy, size, width = [(80.0, 70.0, 8.0, 2.6), (84.5, 63.0, 10.0, 2.9), (89.0, 56.0, 12.0, 3.2)][frame]
    m = mask_of(lambda d: z_glyph(d, cx, cy, size, width))
    lit(img, m, accent, core=(232, 240, 255), halo=1.8, strength=1.6)
    return img


def down(img):
    return img.resize((SIZE, SIZE), Image.LANCZOS)


PICTURES = {
    "typing_1": lambda: dots(AMBER, 0),
    "typing_2": lambda: dots(AMBER, 1),
    "typing_3": lambda: dots(AMBER, 2),
    "busy": lambda: pause(AMBER),
    "away_1": lambda: moon(BLUE, 0),
    "away_2": lambda: moon(BLUE, 1),
    "away_3": lambda: moon(BLUE, 2),
}


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    made = {}
    for name, make in PICTURES.items():
        img = down(make())
        img.save(OUT / f"{name}.png", optimize=True)
        made[name] = img
    if len(sys.argv) > 1:
        scale = 3
        sheet = Image.new("RGBA", (SIZE * scale * len(made), SIZE * scale), (70, 74, 80, 255))
        for i, img in enumerate(made.values()):
            sheet.alpha_composite(img.resize((SIZE * scale, SIZE * scale), Image.LANCZOS), (i * SIZE * scale, 0))
        sheet.save(sys.argv[1])


if __name__ == "__main__":
    main()
