"""Make the WHEEL OF DOOM's placeholder model and pictures (`docs/wheel-of-doom.md`, SJK's own).

A dark wheel of twenty segments (sixteen before 11/11/2026) on a stand, a red pointer over
its top; the holocron segments in their tiers' colours. Sol models the
final one: this one keeps the contract the client draws by (docs/wheel-of-doom.md, "The
model"), so the files can be swapped one for one. Nothing here comes from another game:
the colours, the words and the shapes are drawn here. This script writes, into the output
folder:

- `wheel_of_doom.jpg`: the atlas, 1024x1024: the wheel's face (a disc, 768x768 top left,
  segment 0 at the top, the others clockwise), and strips for its back, its rim, iron, the
  pointer's red and the stand's dark wood down the right;
- `wheel_of_doom_glow.jpg`: the same layout, black but for what glows (the jackpot's gold,
  the segments' words, the pointer), which the shader adds;
- `wheel_of_doom_stand.md3`: the stand and the pointer, its origin on the floor under the
  wheel's middle, the wheel's face looking along +X;
- `wheel_of_doom_wheel.md3`: the wheel, its origin on its axle (the pivot), which the
  client sets AXLE units over the stand's origin and turns about the model's X axis;
- `toy_wheelofdoom.png`: the toy's icon, 128x128, on transparency.

    python scripts/wheel_of_doom_model.py crates/sjk-viewer/assets/wheel_of_doom

The output is deterministic. Needs Pillow and numpy (pip install pillow numpy); it uses
the MD3 writer of mystery_box_model.py. The shader (`wheel_of_doom.shader`) is written by
hand.
"""

import math
import sys
from pathlib import Path

from PIL import Image, ImageDraw, ImageFilter, ImageFont

sys.path.insert(0, str(Path(__file__).resolve().parent))
import mystery_box_model as md3  # noqa: E402

# The client's numbers (`wheel_of_doom.rs`): the wheel's radius, its thickness and how
# high its axle stands over the stand's origin, in game units; 20 segments.
RADIUS = 40.0
THICK = 4.0
AXLE = 64.0
SEGMENTS = 20
SHADER = "models/sjk/wheel_of_doom"
ATLAS = 1024
JPEG_QUALITY = 90
ICON_SIZE = 128
FACE = (0, 0, 768, 768)
BACK = (768, 0, 1024, 256)
RIM = (768, 256, 1024, 384)
IRON = (768, 384, 1024, 512)
RED = (768, 512, 1024, 640)
WOOD = (768, 640, 1024, 768)
# What each segment says, in the client's segment order, and its colour.
WORDS = [
    "MYTHICAL", "REVERSE", "UNCOMMON", "INVERT", "BOOM", "RARE", "MIRROR", "FLIP",
    "COIN", "BLIND", "LEGENDARY", "DRUNK", "TINY FOV", "NOTHING", "HUGE FOV", "NO SABER",
    "QUACK", "WALK", "BUNNY", "CURSE",
]
GOLD = (214, 168, 58)
BLOOD = (104, 14, 16)
COAL = (26, 22, 24)
GREY = (70, 70, 74)
FIRE = (170, 70, 16)
PLUM = (70, 20, 92)


# The holocron tiers' colours (docs/holocrons.md), dimmed to sit on the dark wheel, and
# the DOOM coin's bronze.
UNCOMMON = (30, 140, 86)
RARE = (30, 86, 190)
LEGENDARY = (118, 52, 182)
BRONZE = (150, 96, 40)


def segment_colour(index: int) -> tuple:
    special = {0: GOLD, 2: UNCOMMON, 4: FIRE, 5: RARE, 8: BRONZE, 10: LEGENDARY, 13: GREY, 19: PLUM}
    return special.get(index, BLOOD if index % 2 else COAL)


# The segments whose words glow, as the jackpot's: the holocrons and the coin.
GLOWING = {0, 2, 5, 8, 10}


def font(size: int):
    try:
        return ImageFont.load_default(size=size)
    except TypeError:
        return ImageFont.load_default()


def face(glow: bool) -> Image.Image:
    """The wheel's face: segment k centred `k * 22.5` degrees clockwise from the top."""
    size = FACE[2] - FACE[0]
    img = Image.new("RGB", (size, size), (0, 0, 0) if glow else (12, 10, 12))
    draw = ImageDraw.Draw(img)
    centre = size / 2
    r = size / 2 - 2
    step = 360 / SEGMENTS
    for k in range(SEGMENTS):
        # PIL's angles run clockwise from 3 o'clock; the top is -90.
        start = -90 + k * step - step / 2
        colour = segment_colour(k)
        if glow:
            colour = tuple(c // 2 for c in segment_colour(k)) if k in GLOWING else (0, 0, 0)
        draw.pieslice((centre - r, centre - r, centre + r, centre + r), start, start + step, fill=colour)
    # Gold dividers, an outer ring and a hub.
    for k in range(SEGMENTS):
        angle = math.radians(k * step - step / 2)
        x, y = centre + r * math.sin(angle), centre - r * math.cos(angle)
        draw.line((centre, centre, x, y), fill=(60, 46, 16) if glow else GOLD, width=5)
    ring = (60, 46, 16) if glow else GOLD
    draw.ellipse((centre - r, centre - r, centre + r, centre + r), outline=ring, width=12)
    draw.ellipse((centre - 60, centre - 60, centre + 60, centre + 60), fill=(0, 0, 0) if glow else (30, 26, 28), outline=ring, width=8)
    # Each segment's word along its radius, reading outwards.
    words = font(28)
    for k, word in enumerate(WORDS):
        box = draw.textbbox((0, 0), word, font=words)
        w, h = box[2] - box[0] + 8, box[3] - box[1] + 8
        label = Image.new("L", (w, h), 0)
        ImageDraw.Draw(label).text((4 - box[0], 4 - box[1]), word, font=words, fill=255)
        a = k * step
        turned = label.rotate(90 - a, resample=Image.BICUBIC, expand=True)
        at = r * 0.6
        x = centre + at * math.sin(math.radians(a)) - turned.width / 2
        y = centre - at * math.cos(math.radians(a)) - turned.height / 2
        ink = (40, 26, 4) if k == 0 else (232, 210, 170)
        if glow:
            ink = (120, 90, 30) if k in GLOWING else (70, 60, 50)
        img.paste(Image.new("RGB", turned.size, ink), (int(x), int(y)), turned)
    return img


def pictures(output: Path) -> Image.Image:
    for glow in (False, True):
        atlas = Image.new("RGB", (ATLAS, ATLAS), (0, 0, 0))
        atlas.paste(face(glow), FACE[:2])
        if not glow:
            for region, colour in ((BACK, (18, 14, 16)), (RIM, (120, 92, 34)), (IRON, (54, 52, 56)), (RED, (190, 20, 20)), (WOOD, (34, 24, 20))):
                atlas.paste(Image.new("RGB", (region[2] - region[0], region[3] - region[1]), colour), region[:2])
        else:
            atlas.paste(Image.new("RGB", (RED[2] - RED[0], RED[3] - RED[1]), (90, 8, 8)), RED[:2])
        name = "wheel_of_doom_glow.jpg" if glow else "wheel_of_doom.jpg"
        atlas.save(output / name, quality=JPEG_QUALITY)
        if not glow:
            kept = atlas
    return kept


def face_uv(y: float, z: float) -> tuple:
    """Where a point of the face (y right, z up, seen from +X) is on the atlas."""
    s = 0.5 + y / (2 * RADIUS)
    t = 0.5 - z / (2 * RADIUS)
    return md3.uv_of(FACE, s, t)


def wheel() -> list:
    quads = []
    steps = 64
    half = THICK / 2
    for i in range(steps):
        a0 = 2 * math.pi * i / steps
        a1 = 2 * math.pi * (i + 1) / steps
        # Clockwise from the top as seen from +X: y = sin, z = cos.
        p0 = (RADIUS * math.sin(a0), RADIUS * math.cos(a0))
        p1 = (RADIUS * math.sin(a1), RADIUS * math.cos(a1))
        front = [(half, 0.0, 0.0), (half, *p0), (half, *p1), (half, *p1)]
        quads.append((front, [(1, 0, 0)] * 4, None, [face_uv(0, 0), face_uv(*p0), face_uv(*p1), face_uv(*p1)]))
        back = [(-half, 0.0, 0.0), (-half, *p1), (-half, *p0), (-half, *p0)]
        quads.append((back, (-1, 0, 0), BACK))
        n0 = (0, math.sin(a0), math.cos(a0))
        n1 = (0, math.sin(a1), math.cos(a1))
        rim = [(half, *p0), (half, *p1), (-half, *p1), (-half, *p0)]
        quads.append((rim, [n0, n1, n1, n0], None, [md3.uv_of(RIM, s, t) for s, t in ((0, 0), (1, 0), (1, 1), (0, 1))]))
    # A peg on each divider at the rim, and a hub cap.
    iron = {"+x": IRON, "-x": IRON, "+y": IRON, "-y": IRON, "+z": IRON, "-z": IRON}
    for k in range(SEGMENTS):
        a = math.radians(k * 360 / SEGMENTS - 360 / SEGMENTS / 2)
        y, z = (RADIUS - 2.5) * math.sin(a), (RADIUS - 2.5) * math.cos(a)
        quads += md3.cuboid(half, half + 2.5, y - 0.9, y + 0.9, z - 0.9, z + 0.9, iron)
    quads += md3.cuboid(half, half + 1.5, -5, 5, -5, 5, iron)
    return quads


def stand() -> list:
    wood = {"+x": WOOD, "-x": WOOD, "+y": WOOD, "-y": WOOD, "+z": WOOD, "-z": WOOD}
    iron = {"+x": IRON, "-x": IRON, "+y": IRON, "-y": IRON, "+z": IRON, "-z": IRON}
    top = AXLE + RADIUS + 12
    quads = []
    quads += md3.cuboid(-16, 10, -30, 30, 0, 4, wood)
    quads += md3.cuboid(-10, -4, -4, 4, 4, top, wood)
    # Two braces from the base's ends to the post.
    for side in (-1, 1):
        quads += md3.cuboid(-10, -4, side * 26 - 3, side * 26 + 3, 4, 14, wood)
        quads += md3.cuboid(-10, -4, min(side * 4, side * 26), max(side * 4, side * 26), 14, 18, wood)
    # The axle through the wheel, and the arm over its top holding the pointer.
    quads += md3.cuboid(-4, THICK / 2 + 1.5, -1.5, 1.5, AXLE - 1.5, AXLE + 1.5, iron)
    quads += md3.cuboid(-10, THICK / 2 + 5.5, -3, 3, top - 4, top, wood)
    # The pointer, a red wedge pointing down at the top of the wheel, before its face.
    tip = AXLE + RADIUS - 7
    for x, normal in ((THICK / 2 + 5.5, (1, 0, 0)), (THICK / 2 + 3.0, (-1, 0, 0))):
        quads.append(([(x, -5, top - 4), (x, 5, top - 4), (x, 0, tip), (x, 0, tip)], normal, RED))
    x0, x1 = THICK / 2 + 3.0, THICK / 2 + 5.5
    for y in (-5, 5):
        quads.append(([(x1, y, top - 4), (x0, y, top - 4), (x0, 0, tip), (x1, 0, tip)], (0, y / 5 * 0.95, 0.32), RED))
    return quads


def icon(output: Path, atlas: Image.Image) -> None:
    scale = 4
    size = ICON_SIZE * scale
    canvas = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    disc = atlas.crop(FACE).resize((int(size * 0.9), int(size * 0.9)), Image.LANCZOS).convert("RGBA")
    mask = Image.new("L", disc.size, 0)
    ImageDraw.Draw(mask).ellipse((0, 0, disc.width - 1, disc.height - 1), fill=255)
    offset = ((size - disc.width) // 2, size - disc.height - 4)
    canvas.paste(disc, offset, mask)
    draw = ImageDraw.Draw(canvas)
    cx = size / 2
    draw.polygon([(cx - 34, 6), (cx + 34, 6), (cx, offset[1] + 70)], fill=(200, 24, 24, 255), outline=(20, 0, 0, 255))
    alpha = canvas.split()[3]
    halo = alpha.filter(ImageFilter.MaxFilter(9)).filter(ImageFilter.GaussianBlur(3))
    ground = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    ground.putalpha(halo.point(lambda v: int(v * 0.7)))
    ground.alpha_composite(canvas)
    ground.resize((ICON_SIZE, ICON_SIZE), Image.LANCZOS).save(output / "toy_wheelofdoom.png")


def main() -> None:
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    output = Path(sys.argv[1])
    output.mkdir(parents=True, exist_ok=True)
    md3.ATLAS = ATLAS
    atlas = pictures(output)
    md3.write_md3(output / "wheel_of_doom_stand.md3", "wheel_of_doom_stand", [("wheel_of_doom_stand", SHADER, stand())])
    md3.write_md3(output / "wheel_of_doom_wheel.md3", "wheel_of_doom_wheel", [("wheel_of_doom_wheel", SHADER, wheel())])
    icon(output, atlas)


if __name__ == "__main__":
    main()
