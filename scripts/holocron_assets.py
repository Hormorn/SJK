"""Make the Illuminate holocron's game files from Sol's two generated pictures,
and the four loot-box holocrons from the shipped face.

Illuminate is SJK's own toy (`toy_illuminate`): a holocron that floats by the player's
shoulder and lights the way. Sol generated its pictures (07/10/2026): a holocron
icon in the style of Jedi Academy's Force icons (1024x1024, on a flat grey ground)
and one face of the holocron (2048x2048). This script writes, into the output
folder:

- `toy_illuminate.png`: the icon cut out of its ground with its amber glow, cube
  at the stock icons' size, 128x128 like `gfx/mp/f_icon_*`;
- `holocron.jpg`: the face, 512x512;
- `holocron_glow.jpg`: the face's emblem alone on black, for the additive stage that
  keeps it lit in the dark;
- `holocron.md3`: a cube of HOLOCRON_EDGE units with the face on all six sides.

    python scripts/holocron_assets.py icon.jpg face.jpg crates/sjk-viewer/assets/holocron

That also remakes the tiers (below) from the new face. The tiers alone, from the
shipped `holocron.jpg` in the folder (no originals needed):

    python scripts/holocron_assets.py tiers crates/sjk-viewer/assets/holocron

The tiers (uncommon green, rare blue, legendary purple, mythical gold) are each an
object of their own (`holocron_forms.py`: the Jedi cube, the Great Holocron, the Sith
pyramid, the engram), and the kyber crystals (`kyber_<tier>`, 21/10/2026) one more object
in the same four colours; each form is a model per part so the client can move the parts.
Per form `<id>` it writes the parts' models (`holocron_<id>.md3`, the body, and
`holocron_<id>_<part>.md3`, each also as `..._locked.md3`), the face picture
`holocron_<id>.jpg` with its `_glow.jpg` and `_locked.jpg`, the lit parts' picture
`holocron_<id>_light.jpg`, and the UI icons `holocron_<id>.png` (256x256, drawn from the
parts) and `holocron_<id>_small.png` (64x64); the engram also its seams' and core's
pictures (and locked) and its reflection `holocron_mythical_env.jpg`. Shared: the steel
(`holocron_steel.jpg`, and locked), the locked
light, the sheen, four shaders (`holocron_tiers.shader`: the holocrons' faces;
`holocron_kyber.shader`: the crystals' faces and lights; `holocron_parts.shader`: steel,
the Sith pyramid's cap, the holocrons' lights and the engram's seams and core;
`holocron_locked.shader`: the locked look, every material drained and dark, without
glow), and two Rust tables the client includes (`holocron_parts.rs`: every form's parts
and motion; `holocron_files.rs`: every file above at its game path). The Great
Holocron's face comes from Sol's picture (17/10/2026) in the folder's `art/`
(`rare.jpg`), and the kyber crystals' facets from `art/kyber.jpg` (ART below), or are
drawn here when one is missing. The Jedi cube, the Sith pyramid and the engram are Sol's
sculpted models (05/11 and 08/11/2026, SCULPTS below): their low polys and the pictures
baked from them are in `art/<folder>/` (holocron_forms.py reads the models; their colour,
glow and normal maps are copied from there). `--art
<key>=<picture>` gives another picture for an ART key:

    python scripts/holocron_assets.py tiers crates/sjk-viewer/assets/holocron --art rare=pentagon.png

A face picture shows its shape on black, in any colour and any size: the shape is cut
out and fitted to where the form's picture coordinates expect it (ART), and its lit
parts (bright, or strongly coloured) are painted along the tier's ramp and glow. The
rare one is a pentagon, a corner at the top. The kyber one is
a tiling crystal texture, its light worked into the crystal's facets. The whole table
(colours, glow ladder, shader waves, point lights) is TIERS below (KYBER_TIERS copies
it); the output is deterministic.

Needs Pillow, numpy and scipy (pip install pillow numpy scipy).
"""

import math
import struct
import sys
from dataclasses import dataclass, replace
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw
from scipy import ndimage
from scipy.spatial import ConvexHull

import holocron_forms as forms

ICON_SIZE = 128
# The icon's flat ground, and the share of the frame the cube's box takes (the
# stock icons' cubes span about two thirds of theirs).
ICON_GROUND = 39.0
ICON_CUBE_SHARE = 0.68
ICON_GLOW = np.array([255.0, 168.0, 72.0])
FACE_SIZE = 512
HOLOCRON_EDGE = 6.0
SHADER = "models/sjk/holocron"
JPEG_QUALITY = 88


def icon(source: Path, output: Path) -> None:
    a = np.asarray(Image.open(source).convert("RGB")).astype(float)
    r, b = a[..., 0], a[..., 2]
    # The cube is grey-blue and off the ground; its halo is orange (red over blue).
    cube = (np.abs(a - ICON_GROUND).max(-1) > 14) & (b >= r - 6)
    cube = ndimage.binary_opening(cube, iterations=2)
    labels, count = ndimage.label(cube)
    sizes = ndimage.sum(cube, labels, range(1, count + 1))
    cube = labels == 1 + int(np.argmax(sizes))
    ys, xs = np.nonzero(cube)
    points = np.stack([xs, ys], 1)
    hull = ConvexHull(points)
    mask = Image.new("L", (a.shape[1], a.shape[0]), 0)
    ImageDraw.Draw(mask).polygon([tuple(map(float, points[i])) for i in hull.vertices], fill=255)
    cube_alpha = ndimage.gaussian_filter(np.asarray(mask).astype(float) / 255.0, 0.8)
    # The halo's coverage from its red channel over the ground, in one amber.
    halo_alpha = np.clip((r - ICON_GROUND) / (ICON_GLOW[0] - ICON_GROUND), 0, 1)
    alpha = np.maximum(cube_alpha, halo_alpha)
    rgb = np.where(cube_alpha[..., None] > 0.5, a, ICON_GLOW)
    picture = Image.fromarray(np.dstack([rgb, alpha * 255]).clip(0, 255).astype(np.uint8), "RGBA")
    centre_x, centre_y = (xs.min() + xs.max()) / 2, (ys.min() + ys.max()) / 2
    side = max(xs.max() - xs.min(), ys.max() - ys.min()) / ICON_CUBE_SHARE
    box = tuple(
        round(v)
        for v in (centre_x - side / 2, centre_y - side / 2, centre_x + side / 2, centre_y + side / 2)
    )
    picture = picture.crop(box).convert("RGBa").resize((ICON_SIZE, ICON_SIZE), Image.LANCZOS)
    picture.convert("RGBA").save(output / "toy_illuminate.png", optimize=True)


def face(source: Path, output: Path) -> None:
    picture = Image.open(source).convert("RGB").resize((FACE_SIZE, FACE_SIZE), Image.LANCZOS)
    a = np.asarray(picture).astype(float)
    r, g, b = a[..., 0], a[..., 1], a[..., 2]
    ys, xs = np.mgrid[0:FACE_SIZE, 0:FACE_SIZE]
    from_centre = np.hypot(xs - FACE_SIZE / 2, ys - FACE_SIZE / 2) / FACE_SIZE
    # The emblem and its baked halo are warm; the rays' cores are near white, so
    # inside the emblem's circle brightness counts too. Rust specks on the plates
    # are warm as well, hence the circles.
    warm = np.clip((r - b - 12) / 90.0, 0, 1) * (from_centre < 0.37)
    luminance = 0.3 * r + 0.59 * g + 0.11 * b
    bright = np.clip((luminance - 150) / 70.0, 0, 1) * (from_centre < 0.246)
    emblem = ndimage.gaussian_filter(np.maximum(warm, bright), 0.7)
    picture.save(output / "holocron.jpg", quality=92)
    glow = (a * emblem[..., None]).clip(0, 255).astype(np.uint8)
    Image.fromarray(glow).save(output / "holocron_glow.jpg", quality=92)




def encode_normal(n) -> int:
    # The inverse of sjk-model's `decode_normal` (stock's MD3 normal): the high
    # byte is the angle around Z, the low byte the angle from +Z.
    around = round(math.atan2(n[1], n[0]) * 255 / (2 * math.pi)) & 0xFF
    from_up = round(math.acos(max(-1.0, min(1.0, n[2]))) * 255 / (2 * math.pi)) & 0xFF
    return (around << 8) | from_up


def md3_name(text: str, size: int) -> bytes:
    return text.encode("ascii").ljust(size, b"\0")


def write_md3(output: Path, file: str, surfaces) -> None:
    """Write `<file>.md3`: one frame, a surface per `(shader, positions, normals, uvs,
    triangles)`, triangles clockwise seen from outside (stock's front side).

    The renderer takes a surface's shader from the model file, so a look that is not
    the base one needs its own model: `holocron_<tier>.md3` names `models/sjk/holocron_<tier>`
    and the locked look's models their locked shaders (see `tiers`).
    """
    every = np.array([p for _, positions, _, _, _ in surfaces for p in positions], float)
    low, high = every.min(0), every.max(0)
    radius = float(np.linalg.norm(every, axis=1).max())
    blocks = []
    for index, (shader, positions, normals, uvs, triangles) in enumerate(surfaces):
        shaders = md3_name(shader, 64) + struct.pack("<i", 0)
        tris = b"".join(struct.pack("<3i", *t) for t in triangles)
        sts = b"".join(struct.pack("<2f", *uv) for uv in uvs)
        verts = b"".join(
            struct.pack("<3hH", *(round(c * 64) for c in p), encode_normal(n))
            for p, n in zip(positions, normals)
        )
        surface_header = 108
        ofs_shaders = surface_header
        ofs_tris = ofs_shaders + len(shaders)
        ofs_st = ofs_tris + len(tris)
        ofs_verts = ofs_st + len(sts)
        surface_end = ofs_verts + len(verts)
        blocks.append(
            b"IDP3"
            + md3_name("holocron" if index == 0 else f"holocron_{index}", 64)
            + struct.pack(
                "<10i", 0, 1, 1, len(positions), len(triangles),
                ofs_tris, ofs_shaders, ofs_st, ofs_verts, surface_end,
            )
            + shaders + tris + sts + verts
        )
    frame = struct.pack("<10f", *low, *high, 0, 0, 0, radius) + md3_name("holocron", 16)
    header_size = 108
    ofs_frames = header_size
    ofs_surfaces = ofs_frames + len(frame)
    body = b"".join(blocks)
    end = ofs_surfaces + len(body)
    header = (
        b"IDP3"
        + struct.pack("<i", 15)
        + md3_name(f"models/sjk/{file}.md3", 64)
        + struct.pack("<9i", 0, 1, 0, len(blocks), 0, ofs_frames, ofs_surfaces, ofs_surfaces, end)
    )
    (output / f"{file}.md3").write_bytes(header + frame + body)


def cube_md3(output: Path, file: str = "holocron", shader: str = SHADER) -> None:
    """Write Illuminate's cube as `<file>.md3` with the surface shader `shader`."""
    h = HOLOCRON_EDGE / 2
    # Each face: outward normal and the picture's up; right = up x normal, so the
    # picture reads upright seen from outside.
    faces = [
        ((1, 0, 0), (0, 0, 1)),
        ((-1, 0, 0), (0, 0, 1)),
        ((0, 1, 0), (0, 0, 1)),
        ((0, -1, 0), (0, 0, 1)),
        ((0, 0, 1), (1, 0, 0)),
        ((0, 0, -1), (1, 0, 0)),
    ]
    positions, normals, uvs, triangles = [], [], [], []
    for normal, up in faces:
        n, u = np.array(normal, float), np.array(up, float)
        right = np.cross(u, n)
        base = len(positions)
        for s, t in ((0, 0), (1, 0), (1, 1), (0, 1)):
            positions.append(n * h + right * (2 * s - 1) * h + u * (1 - 2 * t) * h)
            normals.append(n)
            uvs.append((s, t))
        # Clockwise seen from outside: stock's front side.
        triangles += [(base, base + 1, base + 2), (base, base + 2, base + 3)]
    write_md3(output, file, [(shader, positions, normals, uvs, triangles)])


# ---------------------------------------------------------------------------
# The loot-box tiers.

TIER_ICON_SIZE = 256
TIER_SMALL_SIZE = 64
# How much of the icon the holocron spans; the rest is the halo's room.
ICON_FILL = 0.8
SHEEN_SIZE = 128
SHEEN_NAME = "holocron_sheen"
# The look of a tier the player holds none of: every picture with its colour drained and
# its light put out, its own shaders without glow (the tier shows in the dim point light
# the Holocrons tab gives it, and in the shape).
LOCKED_KEEP = 0.14
LOCKED_DIM = 0.42
# Sol's face pictures, by tier, in the folder's `art/`, and where each one's shape goes
# in the face picture (left, top, right, bottom, in shares of it; None: a texture that
# tiles, taken whole). The pentagon's corners are on the picture's inscribed circle.
ART = {
    "rare": (0.5 - 0.5 * math.sin(math.radians(72)), 0.0, 0.5 + 0.5 * math.sin(math.radians(72)), 0.5 + 0.5 * math.cos(math.radians(36))),
    "kyber": None,
}
# A picture's black ground: anything darker is not the shape.
ART_GROUND = 18.0


@dataclass(frozen=True)
class Tier:
    """One tier's look. Colours are RGB 0..255, the rest scales.

    `ramp` is the emblem's gradient from its dark edge to its white-hot core; the
    tiers climb in how much they glow (`ring`: the share of the ring's runes and bevels
    lit, `halo`: the emblem's light on the plate, `tint`: the plate's tint, `emblem`:
    the emblem's brightness), so they tell apart by that and by lightness, not by hue
    alone. `light` is the point light's colour in illuminate.rs's COLOR scale
    ([1.5, 1.3, 1.0]). `self_glow` and `breath` are the shader's waves (base,
    amplitude, phase, frequency): the face's own light, and the emblem's (the lit
    parts' too).
    """

    id: str
    label: str
    ramp: tuple
    emblem: float
    ring: float
    halo: float
    tint: float
    light: tuple
    self_glow: tuple
    breath: tuple


TIERS = (
    Tier(
        "uncommon", "Uncommon (green)",
        ((6, 46, 18), (52, 178, 76), (150, 240, 160), (226, 255, 228)),
        emblem=0.80, ring=0.12, halo=0.08, tint=0.10,
        light=(0.55, 1.50, 0.60),
        self_glow=(0.35, 0.0, 0.0, 0.0), breath=(0.40, 0.10, 0.0, 0.30),
    ),
    Tier(
        "rare", "Rare (blue)",
        ((4, 24, 88), (36, 120, 255), (140, 196, 255), (222, 240, 255)),
        emblem=0.90, ring=0.40, halo=0.16, tint=0.14,
        light=(0.50, 1.00, 1.75),
        self_glow=(0.40, 0.0, 0.0, 0.0), breath=(0.50, 0.15, 0.0, 0.35),
    ),
    Tier(
        "legendary", "Legendary (purple)",
        ((34, 6, 82), (176, 64, 255), (226, 160, 255), (250, 228, 255)),
        emblem=1.00, ring=0.72, halo=0.26, tint=0.18,
        light=(1.25, 0.45, 1.75),
        self_glow=(0.45, 0.0, 0.0, 0.0), breath=(0.60, 0.20, 0.0, 0.40),
    ),
    Tier(
        "mythical", "Mythical (gold, shiny)",
        ((92, 40, 0), (255, 186, 38), (255, 232, 130), (255, 252, 226)),
        emblem=1.00, ring=1.00, halo=0.38, tint=0.22,
        light=(1.85, 1.30, 0.40),
        self_glow=(0.45, 0.20, 0.25, 0.35), breath=(0.85, 0.35, 0.0, 0.55),
    ),
)

# The kyber crystals: one per tier, in its colours, glow ladder and light.
KYBER_TIERS = tuple(
    replace(tier, id=f"kyber_{tier.id}", label=tier.label.replace(" (", " kyber crystal (", 1)) for tier in TIERS
)
# Every form, in the order of the client's table (`holocron_forms.FORM_IDS`).
ALL_TIERS = TIERS + KYBER_TIERS
assert tuple(tier.id for tier in ALL_TIERS) == forms.FORM_IDS


def smoothstep(edge0, edge1, x):
    t = np.clip((x - edge0) / (edge1 - edge0), 0.0, 1.0)
    return t * t * (3.0 - 2.0 * t)


def ramp(values: np.ndarray, stops) -> np.ndarray:
    """Colour the 0..1 `values` along the four stops of a tier's ramp."""
    at = [0.12, 0.50, 0.80, 0.97]
    return np.stack([np.interp(values, at, [stop[c] for stop in stops]) for c in range(3)], -1)


def luminance_of(rgb: np.ndarray) -> np.ndarray:
    return 0.3 * rgb[..., 0] + 0.59 * rgb[..., 1] + 0.11 * rgb[..., 2]


def fitted_art(picture: Path, place) -> np.ndarray:
    """Sol's picture as a FACE_SIZE square (float RGB): its shape (the largest patch off
    the black ground) cut out and stretched over `place` (ART), black around it; a
    tiling texture (`place` None) whole."""
    image = Image.open(picture).convert("RGB")
    if place is None:
        return np.asarray(image.resize((FACE_SIZE, FACE_SIZE), Image.LANCZOS)).astype(float)
    rgb = np.asarray(image).astype(float)
    shape = ndimage.binary_opening(luminance_of(rgb) > ART_GROUND, iterations=2)
    labels, count = ndimage.label(shape)
    sizes = ndimage.sum(shape, labels, range(1, count + 1))
    ys, xs = np.nonzero(labels == 1 + int(np.argmax(sizes)))
    crop = image.crop((int(xs.min()), int(ys.min()), int(xs.max()) + 1, int(ys.max()) + 1))
    left, top, right, bottom = (round(share * FACE_SIZE) for share in place)
    out = Image.new("RGB", (FACE_SIZE, FACE_SIZE))
    out.paste(crop.resize((right - left, bottom - top), Image.LANCZOS), (left, top))
    return np.asarray(out).astype(float)


def art_lit(rgb: np.ndarray) -> np.ndarray:
    """How lit each texel of a picture is, whatever its colour: strongly coloured and
    bright (a glowing rune, red or blue), or near white (a hot core); the steel's greys
    are not."""
    value = rgb.max(-1)
    chroma = value - rgb.min(-1)
    coloured = smoothstep(35.0, 100.0, chroma) * smoothstep(90.0, 170.0, value)
    white = smoothstep(215.0, 245.0, luminance_of(rgb))
    return ndimage.gaussian_filter(np.maximum(coloured, white), 0.7)


def art_layers(picture: Path, tier: Tier):
    """A face from Sol's picture for a tier's form: fitted (ART), lightly tinted in the
    tier's colour, its lit parts painted along the tier's ramp and glowing."""
    rgb = fitted_art(picture, ART[tier.id])
    luminance = luminance_of(rgb)
    lit = art_lit(rgb)[..., None]
    paint = ramp(rgb.max(-1) / 255.0, tier.ramp) * tier.emblem
    mid = np.array(tier.ramp[1], float)
    plate = rgb + tier.tint * (luminance[..., None] * (mid / mid.max()) - rgb)
    face = plate * (1.0 - lit) + paint * lit
    return face.clip(0, 255), (paint * lit).clip(0, 255)


def noise_field(n: int, seed: int, blur, wrap: bool = True) -> np.ndarray:
    """Smooth noise of unit spread, tiling when `wrap`."""
    rng = np.random.default_rng(seed)
    field = ndimage.gaussian_filter(rng.normal(size=(n, n)), blur, mode="wrap" if wrap else "reflect")
    return field / field.std()


def steel_picture(base: np.ndarray, n: int = 256) -> np.ndarray:
    """Brushed steel in the base face's plate colour, tiling."""
    plate = base[16:96, 16:96].reshape(-1, 3).mean(0)
    brushed = noise_field(n, 11, (0.6, 18.0))
    mottle = noise_field(n, 12, 9.0)
    value = 1.0 + 0.07 * brushed + 0.08 * mottle
    return (plate * 1.05 * value[..., None]).clip(0, 255)


def tiled(picture: np.ndarray, n: int) -> np.ndarray:
    reps = -(-n // picture.shape[0])
    return np.tile(picture, (reps, reps, 1))[:n, :n]


def light_picture(tier: Tier, n: int = 64) -> np.ndarray:
    """The tier's lit parts (lamps, edges, halo, seam, eye, motes): its light colour."""
    mid = np.array(tier.ramp[1], float)
    pale = np.array(tier.ramp[2], float)
    mottle = noise_field(n, 15, 4.0)
    colour = (0.45 * mid + 0.55 * pale) * tier.emblem
    return (colour * (1.0 + 0.05 * mottle[..., None])).clip(0, 255)


def segment_distance(xs, ys, a, b):
    ax, ay = a
    bx, by = b
    dx, dy = bx - ax, by - ay
    t = np.clip(((xs - ax) * dx + (ys - ay) * dy) / (dx * dx + dy * dy), 0, 1)
    return np.hypot(xs - ax - t * dx, ys - ay - t * dy)


def pentagon_layers(steel: np.ndarray, tier: Tier):
    """The Great Holocron's face: a steel pentagon (its corners on the picture's
    inscribed circle, one at the top) with a bevelled rim, a ring of lit runes, and a
    glowing pentagon in the middle with a star."""
    n = FACE_SIZE
    ys, xs = (np.mgrid[0:n, 0:n] + 0.5) / n - 0.5
    up = -ys
    normals = [math.radians(126 + 72 * k) for k in range(5)]
    apothem = np.max([xs * math.cos(a) + up * math.sin(a) for a in normals], 0)
    edge = 0.5 * math.cos(math.radians(36))
    mid = np.array(tier.ramp[1], float)
    pale = np.array(tier.ramp[2], float)
    plate = tiled(steel, n) * 0.9
    # The rim's bevel, lit from the top left, and a groove inside it.
    bevel = smoothstep(edge - 0.045, edge - 0.03, apothem)
    slope = np.clip(0.5 - (xs + ys), 0, 1)[..., None]
    plate = plate * (1.0 - bevel[..., None]) + plate * (0.7 + 0.7 * slope) * bevel[..., None]
    groove = smoothstep(0.012, 0.0, np.abs(apothem - (edge - 0.07)))
    plate = plate * (1.0 - 0.55 * groove[..., None])
    # Runes: dashes around a pentagonal band, a few per side.
    angle = np.arctan2(up, xs)
    band = smoothstep(0.014, 0.004, np.abs(apothem - 0.29))
    dashes = (np.sin(angle * 25.0) > 0.25) * band
    dots = smoothstep(0.012, 0.004, np.abs(apothem - 0.255)) * (np.sin(angle * 40.0) > 0.85)
    runes = ndimage.gaussian_filter(np.maximum(dashes, dots).astype(float), 0.6)
    # The middle: glass in the tier's colour, a star and a ring of lines.
    inner = smoothstep(0.215, 0.205, apothem)
    glass = ramp(0.38 + 0.4 * (1.0 - apothem / 0.215).clip(0, 1), tier.ramp) * tier.emblem
    corners = [(0.165 * math.cos(math.radians(90 + 72 * k)), 0.165 * math.sin(math.radians(90 + 72 * k))) for k in range(5)]
    star = np.min([segment_distance(xs, up, corners[k], corners[(k + 2) % 5]) for k in range(5)], 0)
    lines = np.maximum(smoothstep(0.009, 0.003, star), smoothstep(0.007, 0.002, np.abs(apothem - 0.19)))
    lines = np.maximum(lines, smoothstep(0.045, 0.035, np.hypot(xs, up)))
    paint_lines = ramp(np.full_like(xs, 0.95), tier.ramp) * tier.emblem
    spill = ndimage.gaussian_filter(inner, 14)[..., None]
    plate = plate + tier.halo * spill * pale
    rune_light = (0.35 + 0.65 * tier.ring) * runes[..., None] * (mid * 0.55 + pale * 0.45)
    plate = plate * (1.0 - runes[..., None] * 0.6) + rune_light
    lit = inner[..., None]
    middle = glass * (1.0 - lines[..., None]) + paint_lines * lines[..., None]
    face = plate * (1.0 - lit) + middle * lit
    glow = middle * lit * 0.8 + rune_light
    return face.clip(0, 255), glow.clip(0, 255)


def crystal_layers(tier: Tier, texture: Path | None = None):
    """The kyber prism's facet: tip at the picture's top middle, the girdle along its
    bottom; brighter toward the girdle, with streaks running to the tip and lit edges,
    and Sol's crystal texture (a tiling one, ART) worked into it when given."""
    n = FACE_SIZE
    ys, xs = (np.mgrid[0:n, 0:n] + 0.5) / n
    across = (xs - 0.5) / np.maximum(ys, 1e-3)
    rng = np.random.default_rng(17)
    streak = np.zeros_like(xs)
    for at, width, strength in zip(rng.uniform(-0.45, 0.45, 7), rng.uniform(0.01, 0.04, 7), rng.uniform(0.4, 1.0, 7)):
        streak += strength * np.exp(-(((across - at) / width) ** 2))
    edges = np.exp(-(((np.abs(across) - 0.5) / 0.03) ** 2))
    if texture is None:
        value = 0.32 + 0.5 * ys**0.8 + 0.18 * streak.clip(0, 1) + 0.25 * edges
    else:
        grain = luminance_of(fitted_art(texture, None)) / 255.0
        value = 0.18 + 0.38 * ys**0.8 + 0.42 * grain + 0.1 * streak.clip(0, 1) + 0.22 * edges
        streak = np.maximum(streak.clip(0, 1), smoothstep(0.7, 0.95, grain))
    face = ramp(value.clip(0, 1), tier.ramp) * tier.emblem
    glow = face * (0.45 + 0.4 * streak.clip(0, 1) + 0.3 * edges)[..., None]
    return face.clip(0, 255), glow.clip(0, 255)


# The engram's reflection, read by the view (`tcGen environment`): a warm sky over a
# dark ground with two thin streaks across, so each facet takes its own light as the
# shell turns and the streaks sweep the facets (`tcMod scroll`).
ENGRAM_ENV = "holocron_mythical_env"


def engram_env(tier: Tier) -> np.ndarray:
    n = 128
    ys, xs = (np.mgrid[0:n, 0:n] + 0.5) / n
    pale = np.array(tier.ramp[2], float)
    white = np.array(tier.ramp[3], float)
    sky = 0.1 * smoothstep(0.75, 0.15, ys)
    phase = (xs * 2 + ys) % 1.0
    streaks = np.exp(-(((phase - 0.3) / 0.035) ** 2)) + 0.6 * np.exp(-(((phase - 0.72) / 0.02) ** 2))
    return (sky[..., None] * pale + 0.75 * streaks[..., None] * white).clip(0, 255)


# The plate's triangle's incentre down the picture (its tip at 0, its foot at 1).
ENGRAM_INCENTRE = 2 * math.sqrt(1.25) / (1 + 2 * math.sqrt(1.25))


def engram_extras(tier: Tier) -> dict:
    """The engram's other pictures: the seams' glow, the core's facet (white-hot in its
    middle, gold at its edges)."""
    pale = np.array(tier.ramp[2], float)
    white = np.array(tier.ramp[3], float)
    mid = np.array(tier.ramp[1], float)
    seam = (0.55 * mid + 0.45 * pale) * (1.0 + 0.04 * noise_field(64, 23, 3.0))[..., None]
    n = 128
    ys, xs = (np.mgrid[0:n, 0:n] + 0.5) / n
    from_middle = np.hypot(xs - 0.5, ys - ENGRAM_INCENTRE)
    heat = np.clip(1.0 - from_middle / 0.55, 0, 1)[..., None]
    core = mid * (1.0 - heat) + white * heat
    return {"seam": seam.clip(0, 255), "core": core.clip(0, 255)}


def sculpt_picture(folder: str, name: str) -> np.ndarray:
    """A picture baked from one of Sol's sculpted models (`art/<folder>/`), float RGB."""
    return np.asarray(Image.open(forms.ART / folder / f"{name}.jpg").convert("RGB")).astype(float)


# The sculpted forms (Sol's models, their low polys and baked pictures in
# `art/<folder>/`): the tier, its folder, and the share of the baked glow picture its
# glow stage adds (at full strength, over the face's own light, the Sith pyramid's runes
# bloomed to white).
SCULPTS = {
    "uncommon": (forms.JEDI_CUBE, 0.5),
    "legendary": ("sith_pyramid", 0.55),
    "mythical": (forms.GOLD_ENGRAM, 0.35),
}
# The sculpted forms' tangent-space normal maps: the shader's material, then the
# picture in `art/` it is baked into (`<material>_n.png`, `normalMap` in the shader).
NORMAL_MAPS = (
    *((f"holocron_{tier}", forms.ART / folder / "body_n.png") for tier, (folder, _) in SCULPTS.items()),
    ("holocron_legendary_cap", forms.SITH_SCULPT / "cap_n.png"),
)
# The Sith pyramid's cap's own light: its face's, a little under the body's.
SITH_CAP_OWN = 0.3


def normal_map_line(material: str) -> tuple:
    """The `normalMap` line of a material's lit stage, when it has one."""
    return tuple(f"normalMap models/sjk/{material}_n" for name, _ in NORMAL_MAPS if name == material)


def form_layers(base: np.ndarray, steel: np.ndarray, tier: Tier, art: dict):
    """The face and glow pictures of a form: Sol's picture for it when given, else the
    one made here."""
    if tier.id in forms.KYBER_IDS:
        return crystal_layers(tier, art.get("kyber"))
    if tier.id in SCULPTS:
        folder, glow = SCULPTS[tier.id]
        return sculpt_picture(folder, "body"), sculpt_picture(folder, "body_glow") * glow
    if tier.id in art:
        return art_layers(art[tier.id], tier)
    return pentagon_layers(steel, tier)


def tier_icon(tier: Tier, looks: dict) -> Image.Image:
    """The 256x256 icon: the form drawn from its parts, a little from above, with a halo
    in the tier's colour, made for a dark UI (the mythical ones a glint too)."""
    size, oversample = TIER_ICON_SIZE, 4
    big = size * oversample
    drawn = forms.render(tier.id, looks, big, ICON_FILL)
    colour, solid = drawn[..., :3], drawn[..., 3]
    mid = np.array(tier.ramp[1], float)
    spread = ndimage.gaussian_filter(solid, 9.0 * oversample)
    halo = np.clip(spread * 2.0 * (0.45 + 0.55 * tier.ring), 0, 0.85) * (1.0 - solid)
    colour = colour * solid[..., None] + mid * halo[..., None]
    alpha = np.clip(solid + halo, 0, 1)
    if tier.id.endswith("mythical"):
        star = sparkle(big, (0.80 * big, 0.17 * big), 0.13 * big)
        colour = colour * (1.0 - star[..., None]) + np.array((255.0, 250.0, 225.0)) * star[..., None]
        alpha = np.maximum(alpha, star)
    # Average in premultiplied terms down to the icon's size.
    flat = np.dstack([colour * alpha[..., None], alpha * 255.0])
    reduced = flat.reshape(size, oversample, size, oversample, 4).mean((1, 3))
    a = reduced[..., 3:4] / 255.0
    rgb = np.where(a > 1e-4, reduced[..., :3] / np.maximum(a, 1e-4), 0.0)
    return Image.fromarray(np.dstack([rgb, reduced[..., 3]]).clip(0, 255).round().astype(np.uint8), "RGBA")


def sparkle(n: int, centre, reach: float) -> np.ndarray:
    """A four-pointed glint, 0..1."""
    ys, xs = np.mgrid[0:n, 0:n]
    dx, dy = xs - centre[0], ys - centre[1]

    def arm(along, across):
        return np.exp(-3.0 * (along / reach) ** 2) * np.exp(-((across / (reach * 0.07)) ** 2))

    core = np.exp(-((np.hypot(dx, dy) / (reach * 0.22)) ** 2))
    return np.clip(arm(dx, dy) + arm(dy, dx) + core, 0, 1)


def small_icon(picture: Image.Image) -> Image.Image:
    """The 64x64 icon: the large one resized in premultiplied colour."""
    small = picture.convert("RGBa").resize((TIER_SMALL_SIZE, TIER_SMALL_SIZE), Image.LANCZOS)
    return small.convert("RGBA")


def sheen(output: Path) -> None:
    """A soft diagonal stripe that tiles along (s + t): the kyber crystals' shaders
    scroll it over the crystal, one stripe crossing at a time; the engram's reads it
    by the view (`tcGen environment`), so its facets flash as they turn."""
    n = SHEEN_SIZE
    ys, xs = np.mgrid[0:n, 0:n]
    phase = ((xs + ys + 1.0) / n) % 1.0
    distance = np.abs(phase - 0.5)
    band = np.exp(-((distance / 0.07) ** 2)) + 0.18 * np.exp(-((distance / 0.22) ** 2))
    picture = np.clip(band[..., None] * np.array((255.0, 232.0, 160.0)), 0, 255)
    Image.fromarray(picture.astype(np.uint8)).save(output / f"{SHEEN_NAME}.jpg", quality=JPEG_QUALITY)


def wave(values) -> str:
    return "rgbGen wave sin " + " ".join(f"{value:g}" for value in values)


def stage(*lines) -> list:
    return ["\t{", *(f"\t\t{line}" for line in lines), "\t}"]


def face_shader(tier: Tier, sheen_lines=(), sheen_name=SHEEN_NAME) -> list:
    """A form's face: lit, glowing faintly of itself, its lit parts breathing in the
    glow, then the sheen's stage when given."""
    name = f"models/sjk/holocron_{tier.id}"
    lines = [name, "{", "\tq3map_nolightmap"]
    if tier.id == "mythical":
        lines.append(f"\tdeformVertexes wave {ENGRAM_BREATH}")
    lines += stage(f"map {name}", *normal_map_line(name.removeprefix("models/sjk/")), "rgbGen lightingDiffuse")
    # The engram's plates are metal: barely a light of their own, so the seams are
    # what glows.
    own = ENGRAM_OWN_GLOW if tier.id == "mythical" else tier.self_glow
    lines += stage(f"map {name}", "blendFunc GL_ONE GL_ONE", wave(own))
    lines += stage(f"map {name}_glow", "blendFunc GL_ONE GL_ONE", wave(tier.breath), "glow")
    if sheen_lines:
        lines += stage(f"map models/sjk/{sheen_name}", "blendFunc GL_ONE GL_ONE", *sheen_lines)
    return lines + ["}"]


# How the engram's plates breathe: `deformVertexes wave` with a spread so wide every
# plate moves as one (its vertices carry the plate's normal), out by 0 to 0.4 units, so
# the core shows between them at the top of the breath.
ENGRAM_BREATH = "1000 sin 0.2 0.2 0 0.35"
# The engram's plates' own light (the face's second stage), far under the tier's.
ENGRAM_OWN_GLOW = (0.35, 0.08, 0.25, 0.35)


def tier_shader() -> str:
    lines = [
        "// The holocron's loot-box tiers (SJK; see README.md): each tier's face (the Jedi",
        "// cube, the Great Holocron's pentagons, the Sith pyramid, the engram). Lit (through",
        "// its baked normal map when sculpted), also glowing faintly of itself, and its lit",
        "// parts kept bright and in the dynamic glow, breathing; the tiers climb in how",
        "// bright. The engram's plates also breathe out and back (deformVertexes) and flash",
        "// with a sheen read by the view as they turn.",
        "// Made by scripts/holocron_assets.py from its TIERS table: edit that, not this.",
    ]
    for tier in TIERS:
        sheen_lines = ()
        sheen_name = SHEEN_NAME
        if tier.id == "mythical":
            sheen_name = ENGRAM_ENV
            sheen_lines = ("tcGen environment", "tcMod scale 1.5 1.5", "tcMod scroll 0.05 0",
                           "rgbGen wave sin 0.8 0.2 0 0.3")
        lines += face_shader(tier, sheen_lines, sheen_name)
    return "\n".join(lines) + "\n"


def light_shader(tier: Tier) -> list:
    name = f"models/sjk/{forms.light(tier.id)}"
    return (
        [name, "{", "\tq3map_nolightmap"]
        + stage(f"map {name}", "rgbGen identity")
        + stage(f"map {name}", "blendFunc GL_ONE GL_ONE", wave(tier.breath), "glow")
        + ["}"]
    )


def kyber_shader() -> str:
    lines = [
        "// The kyber crystals (SJK; see README.md): each crystal's facets, lit, glowing of",
        "// themselves, their lit parts breathing in the glow and a sheen stripe sliding",
        "// across, brighter the rarer; then each crystal's light (lamps, motes).",
        "// Made by scripts/holocron_assets.py from its TIERS table: edit that, not this.",
    ]
    for rank, tier in enumerate(KYBER_TIERS):
        lines += face_shader(
            tier, ("tcMod scale 0.5 0.5", "tcMod scroll 0.06 0.06", f"rgbGen wave sin {0.2 + 0.1 * rank:g} 0.15 0 0.25")
        )
    for tier in KYBER_TIERS:
        lines += light_shader(tier)
    return "\n".join(lines) + "\n"


def lit_shader(name: str, own: float, extra=()) -> list:
    """A material lit by the world that also glows faintly of itself (a wave of no
    amplitude is a constant), so it never reads black with the light behind it;
    `extra` lines go in its lit stage."""
    return (
        [name, "{", "\tq3map_nolightmap"]
        + stage(f"map {name}", *extra, "rgbGen lightingDiffuse")
        + stage(f"map {name}", "blendFunc GL_ONE GL_ONE", f"rgbGen wave sin {own:g} 0 0 0")
        + ["}"]
    )


def parts_shader() -> str:
    lines = [
        "// The holocrons' other materials (SJK; see README.md): steel (the kyber crystals'",
        "// sheaths and rings), the Sith pyramid's sculpted cap, each tier's light (edges,",
        "// halo, eye, motes, sparks): full bright, breathing in the glow as the tier's face",
        "// does; and the engram's seams (pulsing, breathing with the plates) and its",
        "// white-hot core (pulsing in the glow).",
        "// Made by scripts/holocron_assets.py: edit that, not this.",
    ]
    lines += lit_shader(f"models/sjk/{forms.STEEL}", 0.3)
    cap = "holocron_legendary_cap"
    lines += lit_shader(f"models/sjk/{cap}", SITH_CAP_OWN, normal_map_line(cap))
    for tier in TIERS:
        lines += light_shader(tier)
    name = "models/sjk/holocron_mythical"
    lines += (
        [f"{name}_seam", "{", "\tq3map_nolightmap", f"\tdeformVertexes wave {ENGRAM_BREATH}"]
        + stage(f"map {name}_seam", "rgbGen identity")
        + stage(f"map {name}_seam", "blendFunc GL_ONE GL_ONE", "rgbGen wave sin 0.2 0.18 0 0.8", "glow")
        + ["}"]
    )
    lines += (
        [f"{name}_core", "{", "\tq3map_nolightmap"]
        + stage(f"map {name}_core", "rgbGen identity")
        + stage(f"map {name}_core", "blendFunc GL_ONE GL_ONE", "rgbGen wave sin 0.6 0.4 0.5 0.8", "glow")
        + ["}"]
    )
    return "\n".join(lines) + "\n"


def locked_face(face_rgb: np.ndarray) -> np.ndarray:
    """A picture with its colour drained to a share and darkened (float RGB)."""
    grey = luminance_of(face_rgb)[..., None]
    drained = grey + LOCKED_KEEP * (face_rgb - grey)
    return (drained * LOCKED_DIM * np.array((0.96, 1.0, 1.08))).clip(0, 255)


def form_materials() -> list:
    """Every material the forms' models name, each once, in the order first named."""
    seen = []
    for form in forms.FORM_IDS:
        for mesh in forms.FORMS[form](form).values():
            seen += [material for material in mesh.materials() if material not in seen]
    return seen


def locked_materials() -> list:
    out = []
    for material in form_materials():
        locked = forms.locked_material(material)
        if locked not in out:
            out.append(locked)
    return out


def locked_shader() -> str:
    lines = [
        "// The holocrons and kyber crystals of a tier the player holds none of (SJK; see",
        "// README.md): every material drained and dark, lit by the world and a faint light of",
        "// its own, no glow, nothing moving.",
        "// Made by scripts/holocron_assets.py: edit that, not this.",
    ]
    for material in locked_materials():
        lines += lit_shader(f"models/sjk/{material}", 0.16)
    return "\n".join(lines) + "\n"


def save_jpg(picture: np.ndarray, path: Path) -> None:
    Image.fromarray(picture.round().clip(0, 255).astype(np.uint8)).save(path, quality=JPEG_QUALITY)


def write_form(output: Path, tier: Tier) -> list:
    """The form's parts as models, each in its own look and the locked one; the
    models' names."""
    names = []
    meshes = forms.FORMS[tier.id](tier.id)
    for part, mesh in meshes.items():
        name = forms.model_name(tier.id, part)
        for file, locked in ((name, False), (f"{name}_locked", True)):
            surfaces = [
                (f"models/sjk/{forms.locked_material(material) if locked else material}", *data)
                for material, data in mesh.surfaces.items()
            ]
            write_md3(output, file, surfaces)
            names.append(file)
    return names


def rust_number(value: float) -> str:
    """A Rust f32 literal: a multiple of pi by its constant (clippy reads a rounded pi
    as a mistake), else a few significant digits."""
    for name, constant in (("PI", math.pi), ("FRAC_PI_2", math.pi / 2), ("FRAC_PI_4", math.pi / 4), ("FRAC_PI_8", math.pi / 8)):
        for sign in (1, -1):
            if abs(value - sign * constant) < 1e-9:
                return ("-" if sign < 0 else "") + f"std::f32::consts::{name}"
    text = f"{value:.6g}"
    return text if any(c in text for c in ".e") else f"{text}.0"


def rust_array(values) -> str:
    return "[" + ", ".join(rust_number(v) for v in values) + "]"


def parts_table() -> str:
    """The parts' table for the client (`holocrons/stage.rs` includes it)."""
    lines = [
        "// The loot-box holocrons' and kyber crystals' parts and how they move (SJK; see",
        "// README.md), included by crates/sjk-viewer/src/holocrons/stage.rs. Made by",
        "// scripts/holocron_assets.py from holocron_forms.py's PARTS: edit that, not this.",
        "",
        "/// Every form the stage draws, by the hub's tier id: the four holocrons in `TIERS`",
        "/// order, then the four kyber crystals; each its light and its parts, the body first.",
        "#[rustfmt::skip]",
        f"pub(crate) const FORMS: [Form; {len(ALL_TIERS)}] = [",
    ]
    for tier in ALL_TIERS:
        lines.append(f'    Form {{ id: "{tier.id}", light: {rust_array(tier.light)}, parts: &[')
        for motion in forms.PARTS[tier.id]:
            name = forms.model_name(tier.id, motion.part)
            lines.append(
                f'        Part {{ model: "models/sjk/{name}.md3", locked: "models/sjk/{name}_locked.md3", '
                f"at: {rust_array(motion.at)}, bob: {rust_array(motion.bob)}, orbit: {rust_array(motion.orbit)}, "
                f"spin: {rust_array(motion.spin)}, tumble: {rust_array(motion.tumble)}, precess: {rust_array(motion.precess)} }},"
            )
        lines.append("    ] },")
    lines.append("];")
    return "\n".join(lines) + "\n"


def files_table(files) -> str:
    """The forms' bundled files for the client (`illuminate.rs` includes it, after its
    `model!`, `picture!`, `icon!` and `shader!` macros)."""
    lines = [
        "// The loot-box holocrons' and kyber crystals' bundled files at their game paths",
        "// (SJK; see README.md), included by crates/sjk-viewer/src/illuminate.rs. Made by",
        "// scripts/holocron_assets.py: edit that, not this.",
        "",
        "/// Each form's models, pictures and icons, then the shared pictures and the shaders.",
        "const FORM_FILES: &[(&str, &[u8])] = &[",
    ]
    lines += [f'    {kind}!("{name}"),' for kind, name in files]
    lines.append("];")
    return "\n".join(lines) + "\n"


def tiers(output: Path, art: dict) -> None:
    """The forms' pictures, models and shaders, from the folder's holocron.jpg and
    Sol's pictures in its `art/` (`art`, by ART key, overrides them)."""
    art = {
        **{key: output / "art" / f"{key}.jpg" for key in ART if (output / "art" / f"{key}.jpg").exists()},
        **art,
    }
    base = np.asarray(Image.open(output / "holocron.jpg").convert("RGB")).astype(float)
    steel = steel_picture(base)
    files = []

    def picture(name: str, rgb: np.ndarray) -> None:
        save_jpg(rgb, output / f"{name}.jpg")
        files.append(("picture", name))

    looks = {
        forms.STEEL: forms.Look(steel, own_light=0.3),
    }
    for tier in ALL_TIERS:
        face_picture, glow_picture = form_layers(base, steel, tier, art)
        name = f"holocron_{tier.id}"
        files += [("model", model) for model in write_form(output, tier)]
        picture(name, face_picture)
        picture(f"{name}_glow", glow_picture)
        picture(f"{name}_locked", locked_face(face_picture))
        lit = light_picture(tier)
        picture(forms.light(tier.id), lit)
        own = ENGRAM_OWN_GLOW[0] if tier.id == "mythical" else tier.self_glow[0]
        looks[name] = forms.Look(face_picture, glow_picture, own_light=own * 0.5)
        looks[forms.light(tier.id)] = forms.Look(lit, lit=False)
        if tier.id == "legendary":
            cap = sculpt_picture("sith_pyramid", "cap")
            picture(f"{name}_cap", cap)
            picture(f"{name}_cap_locked", locked_face(cap))
            looks[f"{name}_cap"] = forms.Look(cap, own_light=SITH_CAP_OWN * 0.5)
        if tier.id == "mythical":
            extras = engram_extras(tier)
            for part, rgb in extras.items():
                picture(f"{name}_{part}", rgb)
                # The locked seams and core are dim, not lit.
                picture(f"{name}_{part}_locked", locked_face(rgb) * 0.45)
            picture(ENGRAM_ENV, engram_env(tier))
            looks[f"{name}_seam"] = forms.Look(extras["seam"], lit=False)
            looks[f"{name}_core"] = forms.Look(extras["core"], lit=False)
        files += [("icon", name), ("icon", f"{name}_small")]
    for tier in ALL_TIERS:
        icon_picture = tier_icon(tier, looks)
        icon_picture.save(output / f"holocron_{tier.id}.png", optimize=True)
        small_icon(icon_picture).save(output / f"holocron_{tier.id}_small.png", optimize=True)
    picture(forms.STEEL, steel)
    picture(forms.locked_material(forms.STEEL), locked_face(steel))
    picture(forms.LIGHT_LOCKED, locked_face(light_picture(TIERS[1])))
    sheen(output)
    files.append(("picture", SHEEN_NAME))
    for material, source in NORMAL_MAPS:
        Image.open(source).convert("RGB").save(output / f"{material}_n.png", optimize=True)
        files.append(("normal_map", f"{material}_n"))
    for file, text in (
        ("holocron_tiers", tier_shader()),
        ("holocron_kyber", kyber_shader()),
        ("holocron_parts", parts_shader()),
        ("holocron_locked", locked_shader()),
    ):
        (output / f"{file}.shader").write_text(text, encoding="ascii", newline="\n")
        files.append(("shader", file))
    (output / "holocron_parts.rs").write_text(parts_table(), encoding="ascii", newline="\n")
    (output / "holocron_files.rs").write_text(files_table(files), encoding="ascii", newline="\n")


def art_options(arguments) -> dict:
    """`--art <key>=<picture>` pairs: Sol's face pictures for the forms (ART's keys)."""
    art = {}
    for argument in arguments:
        key, _, picture = argument.partition("=")
        if key not in ART or not picture:
            sys.exit(f"--art takes <key>=<picture>, the key one of {', '.join(ART)}")
        art[key] = Path(picture)
    return art


def main() -> None:
    arguments = sys.argv[1:]
    art = art_options(arguments[i + 1] for i, a in enumerate(arguments) if a == "--art")
    positional = [a for i, a in enumerate(arguments) if a != "--art" and (i == 0 or arguments[i - 1] != "--art")]
    if len(positional) == 2 and positional[0] == "tiers":
        tiers(Path(positional[1]), art)
        return
    icon_source, face_source, output = (Path(arg) for arg in positional[:3])
    output.mkdir(parents=True, exist_ok=True)
    icon(icon_source, output)
    face(face_source, output)
    cube_md3(output)
    tiers(output, art)


if __name__ == "__main__":
    main()
