"""Make the Mining toy's game files (`toy_mining`, SJK's own).

A pickaxe in the hand, the boulders that come up out of the ground round the player to
be broken, the glint that says when to strike, and what a broken boulder gives. Nothing here comes from another game:
every part is tubes, lumps and flat colours made here, and the rock is drawn with the
installed game's own rock texture (named in `mining.shader`; nothing of it is copied
here). This script writes, into the output folder:

- `mining.png`: the colour atlas, 128x128, a flat colour in each 16x16 cell;
- `mining_glint.png`: the glint's four-pointed star, 64x64, white on black (drawn
  added);
- `mining_pick.md3`: a pickaxe along +X, its handle 34 units from its butt (the origin)
  to its head, the pick's spike pointing down (-Z) and its chisel up;
- `mining_boulder.md3`: a boulder with mineral crystals in it, its origin in the middle
  of where it meets the ground, standing up along +X (turned to the ground's normal);
- `mining_glint.md3`: the glint, three crossed squares round its origin;
- `mining_rubble.md3`: a broken lump of rock, its origin in its middle (what a boulder
  gives when it gives junk);
- `mining_ore_<id>.md3`, one for each ore (`ORES`): a lump of rock with that ore's
  crystals in it, its origin in its middle, the glowing ones drawn full bright;
- `toy_mining.png`: the toy's icon, 128x128 like `gfx/mp/f_icon_*`: a pickaxe over a
  crystal, on transparency.

    python scripts/mining_assets.py crates/sjk-viewer/assets/mining

Standard library only; the output is deterministic. The shader (`mining.shader`) is
written by hand. The MD3 and PNG writers are `fishing_assets.py`'s.
"""

import math
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import fishing_assets as fa  # noqa: E402

SHADER = "models/sjk/mining"
GLOW_SHADER = "models/sjk/mining_glow"
ROCK_SHADER = "models/sjk/mining_rock"
GLINT_SHADER = "models/sjk/mining_glint"
ATLAS = 128
CELL = 16
GLINT = 64
HANDLE_LENGTH = 34.0

# The atlas cells, by name: (column, row) and colour.
COLOURS = {
    "wood": ((0, 0), (122, 84, 52)),
    "wood_dark": ((1, 0), (88, 58, 36)),
    "leather": ((2, 0), (54, 36, 28)),
    "steel": ((3, 0), (150, 156, 164)),
    "steel_dark": ((4, 0), (74, 78, 86)),
    "edge": ((5, 0), (210, 214, 220)),
    "vein": ((0, 1), (214, 184, 92)),
    "vein_pale": ((1, 1), (236, 222, 170)),
    "iron_ore": ((0, 2), (128, 84, 70)),
    "copper_ore": ((1, 2), (196, 112, 60)),
    "silver_nugget": ((2, 2), (206, 212, 220)),
    "glowstone": ((3, 2), (150, 236, 110)),
    "sunstone": ((4, 2), (255, 176, 52)),
    "moonstone": ((5, 2), (176, 196, 255)),
    "star_quartz": ((6, 2), (248, 244, 255)),
    "void_crystal": ((7, 2), (156, 70, 236)),
    "heart_of_the_mountain": ((0, 3), (255, 70, 60)),
}

# The ores (`mining.rs`'s `ORE_MODELS`, in its order): id, crystals, glows.
ORES = [
    ("iron_ore", 3, False),
    ("copper_ore", 3, False),
    ("silver_nugget", 2, False),
    ("glowstone", 4, True),
    ("sunstone", 3, True),
    ("moonstone", 3, False),
    ("star_quartz", 5, False),
    ("void_crystal", 4, True),
    ("heart_of_the_mountain", 6, True),
]


def uv(cell: str) -> tuple:
    (column, row), _ = COLOURS[cell]
    return ((column + 0.5) * CELL / ATLAS, (row + 0.5) * CELL / ATLAS)


class Mesh(fa.Mesh):
    """`fishing_assets.Mesh` on this toy's atlas."""

    def vertex(self, p, n, cell: str) -> int:
        return self.vertex_st(p, n, uv(cell))

    def write(self, output: Path, name: str) -> None:
        fa.write_md3(output, name, [(self, SHADER)])


def atlas(output: Path) -> None:
    pixels = bytearray(ATLAS * ATLAS * 4)
    for (column, row), colour in COLOURS.values():
        for y in range(row * CELL, (row + 1) * CELL):
            for x in range(column * CELL, (column + 1) * CELL):
                i = (y * ATLAS + x) * 4
                pixels[i : i + 4] = bytes((*colour, 255))
    fa.png(output / "mining.png", ATLAS, ATLAS, pixels)


def glint_picture(output: Path) -> None:
    """A soft four-pointed star: bright in the middle, thin rays along the axes."""
    pixels = bytearray(GLINT * GLINT * 4)
    half = (GLINT - 1) / 2
    for y in range(GLINT):
        for x in range(GLINT):
            dx, dy = (x - half) / half, (y - half) / half
            r = math.hypot(dx, dy)
            core = max(0.0, 1.0 - r * 2.2) ** 1.5
            rays = max(0.0, 1.0 - abs(dx) * 14.0) * max(0.0, 1.0 - abs(dy))
            rays += max(0.0, 1.0 - abs(dy) * 14.0) * max(0.0, 1.0 - abs(dx))
            value = min(1.0, core + rays * 0.8) * max(0.0, 1.0 - r)
            i = (y * GLINT + x) * 4
            pixels[i : i + 4] = bytes((round(255 * value), round(246 * value),
                                       round(200 * value), 255))
    fa.png(output / "mining_glint.png", GLINT, GLINT, pixels)


def jitter(a: float, b: float, seed: int) -> float:
    """A deterministic bump in -1..1 for the point at angles `a`, `b`."""
    return math.sin(a * 5.1 + seed * 1.7) * math.cos(b * 3.7 - seed * 0.9) * 0.6 + \
        math.sin(a * 11.3 + b * 7.9 + seed) * 0.4


def lump(radius: float, depth: float, seed: int, half: bool, rough: float = 0.18) -> Mesh:
    """A rough lump of rock round the origin, `radius` across its middle and `depth`
    along X: a half (out along +X from a wall) or whole, its texture wrapped round it."""
    mesh = Mesh()
    rings, sides = 6, 12
    low = 0.0 if half else -math.pi / 2
    loops = []
    for i in range(rings + 1):
        theta = low + (math.pi / 2 - low) * i / rings
        loop = []
        for j in range(sides + 1):
            phi = 2 * math.pi * j / sides
            bump = 1.0 + rough * jitter(theta, phi % (2 * math.pi), seed)
            if half and i == 0:
                bump = 1.0
            x = depth * math.sin(theta) * bump
            spread = math.cos(theta) * bump
            y, z = radius * spread * math.cos(phi), radius * spread * math.sin(phi)
            n = (x / (depth * depth), y / (radius * radius), z / (radius * radius))
            loop.append(mesh.vertex_st((x, y, z), n, (j / sides * 2, i / rings)))
        loops.append(loop)
    for lower, upper in zip(loops, loops[1:]):
        for j in range(sides):
            # Wound as `fishing_assets.rock`'s, so the outside faces out (the rock's
            # shader culls the back).
            mesh.triangles += [(lower[j], upper[j], upper[j + 1]),
                               (lower[j], upper[j + 1], lower[j + 1])]
    return mesh


def crystal(mesh: Mesh, base, way, length: float, radius: float, cell: str) -> None:
    """A six-sided crystal from `base` along `way`, its tip pointed."""
    norm = math.sqrt(sum(c * c for c in way)) or 1.0
    way = [c / norm for c in way]
    shoulder = [base[k] + way[k] * length * 0.75 for k in range(3)]
    tip = [base[k] + way[k] * length for k in range(3)]
    fa.segment(mesh, base, shoulder, radius, radius * 0.9, cell, sides=6)
    fa.segment(mesh, shoulder, tip, radius * 0.9, 0.05, cell, sides=6)


def write(output: Path, name: str, surfaces) -> None:
    fa.write_md3(output, name, [(mesh, shader) for mesh, shader in surfaces
                                if mesh.triangles])


def pick(output: Path) -> None:
    mesh = Mesh()
    mesh.tube(0, 0.0, HANDLE_LENGTH, 0.75, 0.9, "wood", sides=8)
    mesh.tube(0, 0.0, 9.0, 0.95, 0.95, "leather", sides=8)
    mesh.tube(0, 9.0, 9.6, 1.1, 1.1, "wood_dark", sides=8)
    # The head's eye round the handle's end, then the spike down and the chisel up.
    head = HANDLE_LENGTH - 1.5
    mesh.tube(2, -2.0, 2.0, 1.6, 1.6, "steel_dark", sides=8, centre=(head, 0.0))
    spike = [(head, 0.0, -1.8), (head - 1.2, 0.0, -6.0), (head - 3.0, 0.0, -10.0),
             (head - 5.2, 0.0, -13.0)]
    fa.strand(mesh, spike, 1.3, 0.12, "steel")
    fa.segment(mesh, spike[-2], spike[-1], 0.5, 0.08, "edge")
    chisel = [(head, 0.0, 1.8), (head - 0.8, 0.0, 5.0), (head - 1.8, 0.0, 7.5)]
    fa.strand(mesh, chisel, 1.25, 0.8, "steel")
    fa.segment(mesh, chisel[-1], (head - 2.0, 0.0, 8.2), 0.8, 0.6, "edge")
    mesh.write(output, "mining_pick")


# Where the boulder's crystals stand out of it: (up, across) on its face, as a part of
# its radius, the way they lean, and their length.
BOULDER_CRYSTALS = [
    (0.1, 0.0, (1.0, 0.1, 0.2), 6.5),
    (0.45, -0.35, (1.0, -0.5, 0.6), 5.0),
    (-0.4, 0.4, (1.0, 0.5, -0.5), 4.5),
    (0.5, 0.45, (1.0, 0.6, 0.5), 3.8),
    (-0.5, -0.4, (1.0, -0.4, -0.6), 4.2),
    (-0.05, 0.6, (1.0, 0.9, 0.0), 3.2),
    (0.05, -0.65, (1.0, -0.9, 0.1), 3.0),
]


def boulder(output: Path) -> None:
    radius, height = 15.0, 14.0
    rock = lump(radius, height, 3, half=True, rough=0.2)
    crystals = Mesh()
    for index, (up, across, way, length) in enumerate(BOULDER_CRYSTALS):
        y, z = across * radius * 0.8, up * radius * 0.8
        spread = math.sqrt(max(0.0, 1.0 - (up * up + across * across) * 0.64))
        base = (height * spread * 0.7, y, z)
        crystal(crystals, base, way, length * 2.1, 1.5 + 0.4 * (index % 2),
                "vein" if index % 3 else "vein_pale")
    write(output, "mining_boulder", [(rock, ROCK_SHADER), (crystals, GLOW_SHADER)])


def glint(output: Path) -> None:
    mesh = Mesh()
    size = 6.0
    corners = ((0, 0), (1, 0), (1, 1), (0, 1))
    for axis in range(3):
        u, v = [(1, 2), (0, 2), (0, 1)][axis]
        quad = []
        for s, t in corners:
            p = [0.0, 0.0, 0.0]
            p[u], p[v] = (s - 0.5) * size, (t - 0.5) * size
            n = [0.0, 0.0, 0.0]
            n[axis] = 1.0
            quad.append(mesh.vertex_st(p, n, (s, t)))
        mesh.triangles += [(quad[0], quad[1], quad[2]), (quad[0], quad[2], quad[3])]
    write(output, "mining_glint", [(mesh, GLINT_SHADER)])


def chunk(output: Path, name: str, seed: int, cell, count: int, glows: bool) -> None:
    rock = lump(3.2, 2.6, seed, half=False, rough=0.3)
    crystals = Mesh()
    for index in range(count if cell else 0):
        a = 2 * math.pi * (index + 0.37 * seed) / count
        b = 0.9 * math.sin(index * 2.3 + seed)
        way = (math.cos(a) * math.cos(b), math.sin(a) * math.cos(b), math.sin(b) + 0.4)
        base = tuple(way[k] * 1.6 for k in range(3))
        crystal(crystals, base, way, 2.6 + 0.6 * (index % 3), 0.6 + 0.15 * (index % 2), cell)
    shader = GLOW_SHADER if glows else SHADER
    write(output, name, [(rock, ROCK_SHADER), (crystals, shader)])


def chunks(output: Path) -> None:
    chunk(output, "mining_rubble", 1, None, 0, False)
    for index, (id_, count, glows) in enumerate(ORES):
        chunk(output, f"mining_ore_{id_}", index + 5, id_, count, glows)


def icon(output: Path) -> None:
    """A pickaxe raised over a crystal, drawn at four times the size and scaled down."""
    scale = 4
    size = fa.ICON_SIZE * scale
    cover = [[0.0] * size for _ in range(size)]
    colour = [[(0, 0, 0)] * size for _ in range(size)]

    def paint(inside, rgb) -> None:
        for y in range(size):
            for x in range(size):
                if inside(x / scale, y / scale):
                    cover[y][x] = 1.0
                    colour[y][x] = rgb

    def segment(ax, ay, bx, by, width):
        def inside(x, y):
            dx, dy = bx - ax, by - ay
            t = max(0.0, min(1.0, ((x - ax) * dx + (y - ay) * dy) / (dx * dx + dy * dy)))
            return math.hypot(x - (ax + t * dx), y - (ay + t * dy)) <= width
        return inside

    def tapered(ax, ay, bx, by, w0, w1):
        def inside(x, y):
            dx, dy = bx - ax, by - ay
            t = max(0.0, min(1.0, ((x - ax) * dx + (y - ay) * dy) / (dx * dx + dy * dy)))
            return math.hypot(x - (ax + t * dx), y - (ay + t * dy)) <= w0 + (w1 - w0) * t
        return inside

    def polygon(points):
        def inside(x, y):
            hit = False
            for (ax, ay), (bx, by) in zip(points, points[1:] + points[:1]):
                if (ay > y) != (by > y) and x < ax + (y - ay) * (bx - ax) / (by - ay):
                    hit = not hit
            return hit
        return inside

    # The rock and its crystal at the bottom left.
    paint(polygon([(8, 122), (14, 100), (30, 92), (52, 96), (64, 110), (66, 122)]), (92, 88, 84))
    paint(polygon([(26, 104), (32, 62), (40, 54), (46, 64), (44, 104)]), COLOURS["vein"][1])
    paint(polygon([(34, 100), (38, 66), (41, 62), (42, 100)]), COLOURS["vein_pale"][1])
    # The handle from the lower right up to the head at the upper left.
    paint(segment(118, 122, 52, 34, 4.0), COLOURS["wood"][1])
    paint(segment(118, 122, 104, 103, 4.8), COLOURS["leather"][1])
    # The head: the spike curving down to the left, the chisel up to the right.
    paint(tapered(52, 34, 30, 20, 6.0, 4.0), COLOURS["steel"][1])
    paint(tapered(30, 20, 12, 30, 4.0, 1.0), COLOURS["steel"][1])
    paint(tapered(52, 34, 70, 12, 6.0, 4.0), COLOURS["steel"][1])
    paint(lambda x, y: math.hypot(x - 52, y - 34) <= 6.5, COLOURS["steel_dark"][1])
    pixels = bytearray(fa.ICON_SIZE * fa.ICON_SIZE * 4)
    for y in range(fa.ICON_SIZE):
        for x in range(fa.ICON_SIZE):
            r = g = b = a = 0.0
            for sy in range(scale):
                for sx in range(scale):
                    c = cover[y * scale + sy][x * scale + sx]
                    rgb = colour[y * scale + sy][x * scale + sx]
                    r, g, b, a = r + rgb[0] * c, g + rgb[1] * c, b + rgb[2] * c, a + c
            i = (y * fa.ICON_SIZE + x) * 4
            if a > 0:
                pixels[i : i + 4] = bytes(
                    (round(r / a), round(g / a), round(b / a), round(255 * a / scale ** 2))
                )
    fa.png(output / "toy_mining.png", fa.ICON_SIZE, fa.ICON_SIZE, pixels)


def main() -> None:
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    output = Path(sys.argv[1])
    output.mkdir(parents=True, exist_ok=True)
    atlas(output)
    glint_picture(output)
    pick(output)
    boulder(output)
    glint(output)
    chunks(output)
    icon(output)


if __name__ == "__main__":
    main()
