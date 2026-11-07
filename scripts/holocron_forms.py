"""The shapes of the four loot-box holocrons, for `holocron_assets.py`.

Each tier is its own object (Sol's pick, 17/10/2026, from the proposals' second set):

- uncommon, the Jedi cube: Sol's sculpted model (08/11/2026), a steel-framed cube with
  glowing green glass round an emblem plate on each face (a low poly with its detail
  baked into pictures);
- rare, the Great Holocron: a dodecahedron with a raised pentagon in each face, lit
  edges, and a broken halo turning around it;
- legendary, the Sith pyramid: Sol's sculpted model (05/11/2026), a dark frustum on a
  plinth with glowing runes carved down its sides, its cap floating free over a glowing
  eye, both bobbing and turning (a low poly with its detail baked into pictures);
- mythical, the engram (21/10/2026, Sol: "make a cool shader of it like Destiny"; Sol's
  sculpted model since 08/11/2026): a lumpy gem of triangular gold plates with glowing
  seams between them, a sheen running over the plates, eight sparks circling it.

The kyber crystals (`kyber_<tier>`, 21/10/2026) are one more object in the four tiers'
colours: what was the mythical holocron until then, the kyber prism, a broad
eight-sided crystal in a steel sheath, turning, held by gimbal rings that tumble, motes
of light circling it. The commoner the tier the plainer: one ring and four motes for
uncommon, six motes for rare, both rings and eight for legendary, ten for mythical.

A form is one model per part: its body, which never moves against the holocron, and
each moving part (the halo, the cap and the eye, the crystal, the rings, a mote), so
that the client can turn each one ([`PARTS`], mirrored by `holocrons/stage.rs`'s
table: keep the two in step). A moving part is modelled about its own centre, which
the motion turns it about; the body about the holocron's centre. Units are the
model's (the Jedi cube's edge is about 6), z up.

A part's surfaces are grouped by material (one MD3 surface each): a material is a
picture and a shader of the same name (`models/sjk/<material>`), written by
`holocron_assets.py`. The locked look (a tier the player holds none of) is the same
geometry with every material swapped for its locked one ([`locked_material`]).

Also here: a small software renderer that draws the posed parts for the tiers' UI
icons ([`render`]).
"""

import math
from dataclasses import dataclass, field
from pathlib import Path

import numpy as np
from scipy.spatial import ConvexHull

TIER_IDS = ("uncommon", "rare", "legendary", "mythical")
# The kyber crystals' ids: the hub's tier ids of the crystals, in the same order.
KYBER_IDS = tuple(f"kyber_{tier}" for tier in TIER_IDS)
# Every form the stage draws, in the order of the client's table (`FORMS`).
FORM_IDS = TIER_IDS + KYBER_IDS
# Shared materials and their locked ones; a tier's face `holocron_<tier>` locks to
# `holocron_<tier>_locked`, its light to the one shared dark light.
STEEL = "holocron_steel"
LIGHT_LOCKED = "holocron_light_locked"


def light(tier: str) -> str:
    return f"holocron_{tier}_light"


def locked_material(material: str) -> str:
    if material.endswith("_light"):
        return LIGHT_LOCKED
    return f"{material}_locked"


def kyber_rank(form: str) -> int:
    """A kyber crystal's tier as an index into TIER_IDS."""
    return TIER_IDS.index(form.removeprefix("kyber_"))


# ---------------------------------------------------------------------------
# Meshes.


def newell(points) -> np.ndarray:
    n = np.zeros(3)
    for a, b in zip(points, points[1:] + points[:1]):
        n += np.array(
            (
                (a[1] - b[1]) * (a[2] + b[2]),
                (a[2] - b[2]) * (a[0] + b[0]),
                (a[0] - b[0]) * (a[1] + b[1]),
            )
        )
    return n / np.linalg.norm(n)


def plane_basis(normal):
    """The picture's right and up on a plane facing `normal`: up is the world's z
    (x for a plane facing up or down), so a picture reads upright seen from outside."""
    up = np.array((0.0, 0.0, 1.0)) if abs(normal[2]) < 0.9 else np.array((1.0, 0.0, 0.0))
    right = np.cross(up, normal)
    right /= np.linalg.norm(right)
    return right, np.cross(normal, right)


# World units across one repeat of a tiling material's picture.
TILE = 6.0


class Mesh:
    """Flat-shaded polygons grouped by material."""

    def __init__(self):
        self.surfaces: dict[str, tuple[list, list, list, list]] = {}

    def polygon(self, material: str, points, inside, uvs=None, normal=None) -> None:
        """A convex polygon facing away from the point `inside` (its winding is put
        right from that), with its picture coordinates (default: tiled on its plane).
        `normal` replaces the vertices' normal (not the winding): an engram plate's
        walls carry the plate's, so its shader's `deformVertexes wave` moves the whole
        slab as one."""
        points = [np.asarray(p, float) for p in points]
        own = newell(points)
        centre = sum(points) / len(points)
        if np.dot(own, centre - np.asarray(inside, float)) < 0:
            points = points[::-1]
            if uvs is not None:
                uvs = uvs[::-1]
            own = -own
        normal = own if normal is None else np.asarray(normal, float)
        if uvs is None:
            right, up = plane_basis(normal)
            uvs = [(np.dot(p, right) / TILE, -np.dot(p, up) / TILE) for p in points]
        positions, normals, sts, triangles = self.surfaces.setdefault(material, ([], [], [], []))
        base = len(positions)
        for p, uv in zip(points, uvs):
            positions.append(p)
            normals.append(normal)
            sts.append((float(uv[0]), float(uv[1])))
        # Counter-clockwise seen from outside here; stock's front side is clockwise.
        for i in range(1, len(points) - 1):
            triangles.append((base, base + i + 1, base + i))

    def triangle(self, material: str, points, normals, uvs) -> None:
        """A triangle as a modelling program wrote it (counter-clockwise seen from
        outside, its own vertex normals), turned to stock's clockwise front."""
        positions, own_normals, sts, triangles = self.surfaces.setdefault(material, ([], [], [], []))
        base = len(positions)
        for p, n, uv in zip(points, normals, uvs):
            positions.append(np.asarray(p, float))
            own_normals.append(np.asarray(n, float))
            sts.append((float(uv[0]), float(uv[1])))
        triangles.append((base, base + 2, base + 1))

    def materials(self):
        return list(self.surfaces)


def read_obj(path: Path) -> dict:
    """A triangulated Wavefront OBJ's objects by name, each a list of triangles, a
    triangle three (position, normal, picture coordinate) corners; the picture's t
    counts down from its top, as the game's does."""
    positions, normals, uvs, objects = [], [], [], {}
    current = None
    for line in path.read_text(encoding="ascii").splitlines():
        kind, _, rest = line.partition(" ")
        values = rest.split()
        if kind == "o":
            current = objects.setdefault(rest.strip(), [])
        elif kind == "v":
            positions.append(np.array([float(v) for v in values]))
        elif kind == "vn":
            normals.append(np.array([float(v) for v in values]))
        elif kind == "vt":
            uvs.append((float(values[0]), 1.0 - float(values[1])))
        elif kind == "f":
            corners = [[int(i) - 1 for i in value.split("/")] for value in values]
            if len(corners) != 3:
                raise ValueError(f"{path}: export it triangulated")
            current.append([(positions[v], normals[n], uvs[t]) for v, t, n in corners])
    return objects


def box(mesh: Mesh, material: str, centre, half, axes=None) -> None:
    """A box of half sizes `half` along the columns of `axes` (default: the world's)."""
    centre = np.asarray(centre, float)
    axes = np.eye(3) if axes is None else np.asarray(axes, float)
    e = [axes[:, i] * half[i] for i in range(3)]
    for k in range(3):
        a, b = e[(k + 1) % 3], e[(k + 2) % 3]
        for sign in (1, -1):
            c = centre + sign * e[k]
            mesh.polygon(material, [c - a - b, c + a - b, c + a + b, c - a + b], centre)


def frame_along(direction):
    z = np.asarray(direction, float)
    z = z / np.linalg.norm(z)
    helper = np.array((1.0, 0.0, 0.0)) if abs(z[0]) < 0.9 else np.array((0.0, 1.0, 0.0))
    x = np.cross(helper, z)
    x /= np.linalg.norm(x)
    return np.stack([x, np.cross(z, x), z], 1)


def bar(mesh: Mesh, material: str, a, b, thickness: float) -> None:
    a, b = np.asarray(a, float), np.asarray(b, float)
    length = np.linalg.norm(b - a)
    box(mesh, material, (a + b) / 2, (thickness / 2, thickness / 2, length / 2), frame_along(b - a))


def ring_points(sides: int, radius: float, z: float, phase: float):
    return [
        np.array((radius * math.cos(phase + k * math.tau / sides), radius * math.sin(phase + k * math.tau / sides), z))
        for k in range(sides)
    ]


def prism(mesh, material, sides, bottom, top, z0, z1, phase=0.0, side_uv=None,
          bottom_material=None, top_material=None) -> None:
    """A frustum (or a cone, `top` 0) of `sides` sides from z0 to z1, corner radii
    `bottom` and `top`. `side_uv(k)` gives side k's picture coordinates (bottom k,
    bottom k+1, top k+1, top k)."""
    low = ring_points(sides, bottom, z0, phase)
    high = ring_points(sides, top, z1, phase)
    inside = np.array((0.0, 0.0, (z0 + z1) / 2))
    for k in range(sides):
        j = (k + 1) % sides
        uvs = side_uv(k) if side_uv else None
        if top <= 0:
            mesh.polygon(material, [low[k], low[j], high[0]], inside, uvs and uvs[:3])
        else:
            mesh.polygon(material, [low[k], low[j], high[j], high[k]], inside, uvs)
    if bottom > 0:
        mesh.polygon(bottom_material or material, low, inside + (0, 0, 1))
    if top > 0:
        mesh.polygon(top_material or material, high, inside - (0, 0, 1))


def octahedron(mesh, material, centre, radius, stretch=1.0) -> None:
    centre = np.asarray(centre, float)
    axes = [np.array(v, float) for v in ((radius, 0, 0), (0, radius, 0), (0, 0, radius * stretch))]
    for sx in (1, -1):
        for sy in (1, -1):
            for sz in (1, -1):
                mesh.polygon(material, [centre + sx * axes[0], centre + sy * axes[1], centre + sz * axes[2]], centre)


def tetrahedron(mesh, material, radius) -> None:
    corners = [np.array(c, float) * radius / math.sqrt(3) for c in ((1, 1, 1), (1, -1, -1), (-1, 1, -1), (-1, -1, 1))]
    for skip in range(4):
        mesh.polygon(material, [c for i, c in enumerate(corners) if i != skip], (0, 0, 0))


def tube(mesh, material, radius, thickness, sides, segments, start=0.0, end=math.tau,
         phase=0.0, caps=False) -> None:
    """A ring in the xy plane (a torus of `segments` straight pieces, `sides` around
    the tube), or an arc of it from `start` to `end` with closed ends."""
    angles = [start + (end - start) * i / segments for i in range(segments + 1)]

    def centre(a):
        return np.array((radius * math.cos(a + phase), radius * math.sin(a + phase), 0.0))

    def section(a):
        c = centre(a)
        out = c / np.linalg.norm(c)
        return [
            c + thickness * (math.cos(b) * out + math.sin(b) * np.array((0.0, 0.0, 1.0)))
            for b in (math.tau * (j + 0.5) / sides for j in range(sides))
        ]

    rings = [section(a) for a in angles]
    for i in range(segments):
        inside = (centre(angles[i]) + centre(angles[i + 1])) / 2
        for j in range(sides):
            k = (j + 1) % sides
            mesh.polygon(material, [rings[i][j], rings[i + 1][j], rings[i + 1][k], rings[i][k]], inside)
    if caps:
        mesh.polygon(material, rings[0], centre(angles[1]))
        mesh.polygon(material, rings[-1], centre(angles[-2]))


def dodecahedron_faces(radius: float):
    """The twelve pentagons of a dodecahedron of circumradius `radius`, each counter-
    clockwise about its outward normal, its first corner the topmost."""
    phi = (1 + 5 ** 0.5) / 2
    corners = [(x, y, z) for x in (-1, 1) for y in (-1, 1) for z in (-1, 1)]
    for a in (-1, 1):
        for b in (-1, 1):
            corners += [(0, a / phi, b * phi), (a / phi, b * phi, 0), (a * phi, 0, b / phi)]
    corners = np.array(corners, float) * radius / math.sqrt(3)
    hull = ConvexHull(corners)
    faces = {}
    for simplex, equation in zip(hull.simplices, hull.equations):
        key = tuple(np.round(equation[:3], 4))
        faces.setdefault(key, set()).update(simplex.tolist())
    pentagons = []
    for normal, indices in sorted(faces.items()):
        normal = np.array(normal)
        points = corners[list(indices)]
        centre = points.mean(0)
        right, up = plane_basis(normal)
        order = sorted(range(len(points)), key=lambda i: math.atan2(np.dot(points[i] - centre, up), np.dot(points[i] - centre, right)))
        ring = [points[i] for i in order]
        top = max(range(5), key=lambda i: (round(ring[i][2], 4), round(np.dot(ring[i], right), 4)))
        pentagons.append((normal, ring[top:] + ring[:top]))
    assert len(pentagons) == 12
    return pentagons


# ---------------------------------------------------------------------------
# The forms. Sizes are chosen so the four fill about the same room on screen.

ART = Path(__file__).resolve().parent.parent / "crates/sjk-viewer/assets/holocron/art"


def sculpted_body(tier: str, folder: str, scale: float) -> Mesh:
    """A sculpted form's body: the low poly `low_body` of `art/<folder>/<folder>.obj`
    (Blender's units, baked from Sol's model), scaled and centred on the holocron's
    centre, in the tier's face material."""
    triangles = read_obj(ART / folder / f"{folder}.obj")["low_body"]
    points = np.array([p for triangle in triangles for p, _, _ in triangle])
    centre = (points.min(0) + points.max(0)) / 2
    mesh = Mesh()
    for triangle in triangles:
        mesh.triangle(f"holocron_{tier}", [(p - centre) * scale for p, _, _ in triangle],
                      [n for _, n, _ in triangle], [uv for _, _, uv in triangle])
    return mesh


# The sculpted forms' folders in `art/` and their scales (Blender's metre to model units),
# so the four fill about the same room on screen.
JEDI_CUBE = "jedi_cube"
JEDI_CUBE_SCALE = 6.4
GOLD_ENGRAM = "gold_engram"
GOLD_ENGRAM_SCALE = 7.4


def jedi_cube(tier: str) -> dict:
    """The Jedi cube: Sol's sculpted cube (08/11/2026), steel framing glowing green glass
    with an emblem plate on each face."""
    return {"body": sculpted_body(tier, JEDI_CUBE, JEDI_CUBE_SCALE)}


DODECA_RADIUS = 4.0


def great_holocron(tier: str) -> dict:
    body = Mesh()
    face = f"holocron_{tier}"
    inset, raise_by = 0.55, 0.3
    for normal, ring in dodecahedron_faces(DODECA_RADIUS):
        centre = sum(ring) / 5
        up = ring[0] - centre
        circumradius = np.linalg.norm(up)
        up /= circumradius
        right = np.cross(up, normal)

        def uv(p, centre=centre, right=right, up=up, circumradius=circumradius):
            d = p - centre
            return (0.5 + np.dot(d, right) / (2 * circumradius), 0.5 - np.dot(d, up) / (2 * circumradius))

        inner = [centre + (p - centre) * inset + normal * raise_by for p in ring]
        for k in range(5):
            j = (k + 1) % 5
            quad = [ring[k], ring[j], inner[j], inner[k]]
            body.polygon(face, quad, (0, 0, 0), [uv(p) for p in quad])
        body.polygon(face, inner, (0, 0, 0), [uv(p) for p in inner])
    # Lit edges, each once.
    edges = set()
    for _, ring in dodecahedron_faces(DODECA_RADIUS):
        for k in range(5):
            a, b = ring[k], ring[(k + 1) % 5]
            key = tuple(sorted((tuple(np.round(a, 4)), tuple(np.round(b, 4)))))
            if key not in edges:
                edges.add(key)
                along = (b - a) / np.linalg.norm(b - a)
                bar(body, light(tier), a - along * 0.08, b + along * 0.08, 0.22)
    assert len(edges) == 30
    halo = Mesh()
    dashes = 12
    for k in range(dashes):
        start = k * math.tau / dashes
        tube(halo, light(tier), 5.3, 0.14, 6, 4, start, start + math.tau / dashes * 0.7, caps=True)
    return {"body": body, "halo": halo}


# Sol's sculpted Sith pyramid (05/11/2026): a generated model brought down to a low
# poly in Blender, its detail baked into pictures (`art/sith_pyramid/`, README.md). Its
# units are Blender's, the foot on z = 0: scaled up to about the other forms' size, the
# body's flat top kept under the eye.
SITH_SCULPT = ART / "sith_pyramid"
SITH_SCALE = 7.0
SITH_TOP = 0.4


def sith_pyramid(tier: str) -> dict:
    """The Sith pyramid: Sol's sculpted body and floating cap, and a glowing eye
    between them."""
    objects = read_obj(SITH_SCULPT / "sith_pyramid.obj")
    face = f"holocron_{tier}"
    body_top = max(p[2] for triangle in objects["low_body"] for p, _, _ in triangle)
    lift = np.array((0.0, 0.0, SITH_TOP - body_top * SITH_SCALE))
    cap_points = np.array([p for triangle in objects["low_cap"] for p, _, _ in triangle])
    cap_centre = (cap_points.min(0) + cap_points.max(0)) / 2

    def part(name: str, material: str, place) -> Mesh:
        mesh = Mesh()
        for triangle in objects[name]:
            mesh.triangle(material, [place(p) for p, _, _ in triangle], [n for _, n, _ in triangle],
                          [uv for _, _, uv in triangle])
        return mesh

    body = part("low_body", face, lambda p: p * SITH_SCALE + lift)
    # The cap is modelled about its own centre, which its motion turns it about.
    cap = part("low_cap", f"{face}_cap", lambda p: (p - cap_centre) * SITH_SCALE)
    eye = Mesh()
    octahedron(eye, light(tier), (0, 0, 0), 0.55, stretch=1.15)
    return {"body": body, "cap": cap, "eye": eye}


def kyber_crystal(form: str) -> dict:
    """The kyber crystal of `form` (`kyber_<tier>`): the second ring only from
    legendary up (the motes' count is in PARTS)."""
    face = f"holocron_{form}"
    body = Mesh()
    prism(body, STEEL, 8, 2.75, 2.75, -0.4, 0.4, math.pi / 8)
    crystal = Mesh()
    sides, radius, tip = 8, 2.5, 4.2
    girdle = ring_points(sides, radius, 0.0, 0.0)
    for k in range(sides):
        j = (k + 1) % sides
        for end in (tip, -tip):
            apex = np.array((0.0, 0.0, end))
            crystal.polygon(face, [girdle[k], girdle[j], apex], (0, 0, 0), [(0.0, 1.0), (1.0, 1.0), (0.5, 0.0)])
    ring_a = Mesh()
    tube(ring_a, STEEL, 4.5, 0.15, 6, 8, phase=math.pi / 8)
    for k in range(4):
        a = k * math.tau / 4
        octahedron(ring_a, light(form), (4.5 * math.cos(a), 4.5 * math.sin(a), 0.0), 0.26)
    mote = Mesh()
    tetrahedron(mote, light(form), 0.2)
    parts = {"body": body, "crystal": crystal, "ring_a": ring_a, "mote": mote}
    if kyber_rank(form) >= 2:
        ring_b = Mesh()
        tube(ring_b, STEEL, 5.4, 0.11, 6, 8)
        parts["ring_b"] = ring_b
    return parts


def engram(tier: str) -> dict:
    """The engram: Sol's sculpted gold puzzle (08/11/2026), a lumpy gem of triangular
    gold plates with glowing seams between them, and its sparks."""
    spark = Mesh()
    octahedron(spark, light(tier), (0, 0, 0), 0.16, stretch=2.0)
    return {"body": sculpted_body(tier, GOLD_ENGRAM, GOLD_ENGRAM_SCALE), "mote": spark}


FORMS = {
    "uncommon": jedi_cube,
    "rare": great_holocron,
    "legendary": sith_pyramid,
    "mythical": engram,
    **{form: kyber_crystal for form in KYBER_IDS},
}


# ---------------------------------------------------------------------------
# How the parts move, as the client moves them (holocrons/stage.rs's PARTS).


@dataclass(frozen=True)
class Motion:
    """A part's pose at a time: its rest place `at`, a bob along z (units, radians per
    second), an orbit about z (radius, radians per second, start angle), and three turns
    (start angle, radians per second): about its own z (spin), then about x (tumble),
    then about the holocron's z (precess)."""

    part: str
    at: tuple = (0.0, 0.0, 0.0)
    bob: tuple = (0.0, 0.0)
    orbit: tuple = (0.0, 0.0, 0.0)
    spin: tuple = (0.0, 0.0)
    tumble: tuple = (0.0, 0.0)
    precess: tuple = (0.0, 0.0)

    def pose(self, t: float):
        def rz(a):
            c, s = math.cos(a), math.sin(a)
            return np.array(((c, -s, 0), (s, c, 0), (0, 0, 1)))

        def rx(a):
            c, s = math.cos(a), math.sin(a)
            return np.array(((1, 0, 0), (0, c, -s), (0, s, c)))

        rotation = (
            rz(self.precess[0] + self.precess[1] * t)
            @ rx(self.tumble[0] + self.tumble[1] * t)
            @ rz(self.spin[0] + self.spin[1] * t)
        )
        angle = self.orbit[2] + self.orbit[1] * t
        position = np.array(self.at, float) + np.array(
            (self.orbit[0] * math.cos(angle), self.orbit[0] * math.sin(angle), self.bob[0] * math.sin(self.bob[1] * t))
        )
        return rotation, position


def motes(count: int = 10):
    """The kyber crystal's motes, the first `count` of its ten."""
    out = []
    for i in range(count):
        out.append(
            Motion(
                "mote",
                at=(0.0, 0.0, round(((i * 0.37) % 1.0 - 0.5) * 7.0, 3)),
                orbit=(round(3.5 + 0.4 * math.sin(i), 3), 0.6, round(i * 0.63, 3)),
                spin=(0.0, 1.6),
                tumble=(0.6, 0.0),
            )
        )
    return out


def sparks():
    """The engram's eight sparks: wider than its girth, slower near the tips, half of
    them the other way round."""
    out = []
    for i in range(8):
        height = round(((i * 0.41) % 1.0 - 0.5) * 7.5, 3)
        out.append(
            Motion(
                "mote",
                at=(0.0, 0.0, height),
                orbit=(round(4.1 + 0.5 * math.cos(i * 1.7) - 0.12 * abs(height), 3),
                       round((0.75 - 0.05 * abs(height)) * (1 if i % 2 else -1), 3), round(i * 0.79, 3)),
                spin=(0.0, 2.2),
                tumble=(0.35, 0.0),
            )
        )
    return out


def kyber_parts(form: str):
    rank = kyber_rank(form)
    parts = [
        Motion("body"),
        Motion("crystal", spin=(0.0, 0.5)),
        Motion("ring_a", tumble=(math.pi / 2, 0.3), precess=(0.0, 0.25)),
    ]
    if rank >= 2:
        parts.append(Motion("ring_b", tumble=(1.0, -0.22), precess=(0.0, -0.15)))
    return parts + motes(4 + 2 * rank)


PARTS = {
    "uncommon": [Motion("body")],
    "rare": [Motion("body"), Motion("halo", bob=(0.25, 1.3), spin=(0.0, 0.3), tumble=(0.55, 0.0), precess=(0.0, 0.25))],
    "legendary": [
        Motion("body"),
        Motion("cap", at=(0.0, 0.0, 3.2), bob=(0.35, 1.5), spin=(0.0, 0.35)),
        Motion("eye", at=(0.0, 0.0, 1.25), bob=(0.1, 1.5), spin=(0.0, -1.2)),
    ],
    "mythical": [Motion("body"), *sparks()],
    **{form: kyber_parts(form) for form in KYBER_IDS},
}


def model_name(tier: str, part: str) -> str:
    return f"holocron_{tier}" if part == "body" else f"holocron_{tier}_{part}"


# ---------------------------------------------------------------------------
# A small renderer for the UI icons.


@dataclass
class Look:
    """How a material draws in an icon: its picture (float RGB), its glow picture or
    None, and whether it is lit by the light or shines of itself."""

    picture: np.ndarray
    glow: np.ndarray | None = None
    lit: bool = True
    own_light: float = 0.3


@dataclass
class Canvas:
    size: int
    colour: np.ndarray = field(init=False)
    depth: np.ndarray = field(init=False)

    def __post_init__(self):
        self.colour = np.zeros((self.size, self.size, 3))
        self.depth = np.full((self.size, self.size), np.inf)


def sample(picture: np.ndarray, u, v):
    h, w = picture.shape[:2]
    x = np.floor((u % 1.0) * w).astype(int).clip(0, w - 1)
    y = np.floor((v % 1.0) * h).astype(int).clip(0, h - 1)
    return picture[y, x]


VIEW_YAW = math.radians(-32.0)
VIEW_PITCH = math.radians(20.0)
SUN = np.array((-0.45, -0.35, 0.82))


def render(tier: str, looks: dict, size: int, fill: float, t: float = 1.0) -> np.ndarray:
    """The tier's holocron posed at `t` seconds, seen a little from above, as RGBA
    floats (alpha 0..1) in a `size` square, scaled so it spans `fill` of it."""
    meshes = FORMS[tier](tier)
    triangles = []
    for motion in PARTS[tier]:
        rotation, position = motion.pose(t)
        for material, (positions, normals, uvs, tris) in meshes[motion.part].surfaces.items():
            p = np.array(positions) @ rotation.T + position
            n = np.array(normals) @ rotation.T
            uv = np.array(uvs)
            for a, b, c in tris:
                triangles.append((material, p[[a, b, c]], n[a], uv[[a, b, c]]))
    # The view: turned about z, then tipped to look down a little; x right, y up, z
    # toward the viewer.
    cy, sy = math.cos(VIEW_YAW), math.sin(VIEW_YAW)
    cp, sp = math.cos(VIEW_PITCH), math.sin(VIEW_PITCH)
    yaw = np.array(((cy, -sy, 0), (sy, cy, 0), (0, 0, 1)))
    # World y toward the viewer before the tip: the view's z.
    to_view = np.array(((1, 0, 0), (0, 0, 1), (0, -1, 0)), float)
    tip = np.array(((1, 0, 0), (0, cp, -sp), (0, sp, cp)))
    view = tip @ to_view @ yaw
    sun = view @ (SUN / np.linalg.norm(SUN))
    projected = [(m, tri @ view.T, view @ n, uv) for m, tri, n, uv in triangles]
    extent = max(np.abs(tri[:, :2]).max() for _, tri, _, _ in projected)
    scale = size * fill / 2 / extent
    canvas = Canvas(size)
    alpha = np.zeros((size, size))
    for material, tri, normal, uv in projected:
        if normal[2] <= 0:
            continue
        look = looks[material]
        xy = np.stack([size / 2 + tri[:, 0] * scale, size / 2 - tri[:, 1] * scale], 1)
        x0, y0 = np.floor(xy.min(0)).astype(int).clip(0, size - 1)
        x1, y1 = np.ceil(xy.max(0)).astype(int).clip(0, size - 1)
        if x1 < x0 or y1 < y0:
            continue
        ys, xs = np.mgrid[y0 : y1 + 1, x0 : x1 + 1] + 0.5
        (ax, ay), (bx, by), (qx, qy) = xy
        area = (bx - ax) * (qy - ay) - (qx - ax) * (by - ay)
        if abs(area) < 1e-9:
            continue
        w1 = ((xs - ax) * (qy - ay) - (qx - ax) * (ys - ay)) / area
        w2 = ((bx - ax) * (ys - ay) - (xs - ax) * (by - ay)) / area
        w0 = 1 - w1 - w2
        inside = (w0 >= -1e-6) & (w1 >= -1e-6) & (w2 >= -1e-6)
        if not inside.any():
            continue
        depth = -(w0 * tri[0, 2] + w1 * tri[1, 2] + w2 * tri[2, 2])
        region = canvas.depth[y0 : y1 + 1, x0 : x1 + 1]
        nearer = inside & (depth < region)
        if not nearer.any():
            continue
        u = w0 * uv[0, 0] + w1 * uv[1, 0] + w2 * uv[2, 0]
        v = w0 * uv[0, 1] + w1 * uv[1, 1] + w2 * uv[2, 1]
        albedo = sample(look.picture, u[nearer], v[nearer])
        if look.lit:
            shade = 0.38 + 0.9 * max(0.0, float(np.dot(normal, sun)))
            colour = albedo * (shade + look.own_light)
        else:
            colour = albedo * 1.05
        if look.glow is not None:
            colour = colour + 0.8 * sample(look.glow, u[nearer], v[nearer])
        region[nearer] = depth[nearer]
        canvas.colour[y0 : y1 + 1, x0 : x1 + 1][nearer] = colour
        alpha[y0 : y1 + 1, x0 : x1 + 1][nearer] = 1.0
    return np.dstack([canvas.colour.clip(0, 255), alpha])
