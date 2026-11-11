"""Make the mystery box's loot of 11/11/2026 (`docs/mystery-boxes.md`, SJK's own): the Ray
Gun, its bolt and the piles of gold coins.

Nothing here comes from another game. The Ray Gun is a simple retro ray gun after Sol's
ask ("based on the raygun, more simplified"): a steel barrel with a round chamber at the
back, three red fins round the front, a cone-tipped emitter, a glowing green tube along
the top and a dark grip. Shapes are made of cylinders, ellipsoids and boxes; the colours
are drawn from noise. This script writes, into the output folder:

- `box_loot.jpg`: 256x256 swatches: steel, red fins, dark grip, gold, the gold's rim;
- `box_loot_glow.jpg`: 64x64, the green glow of the tube and the bolt;
- `raygun.md3`: the gun, about 14 units long, X forward, Y left, Z up (a prop gun's axes,
  `propgun.rs`), its origin in the grip; two surfaces, the gun and its glowing tube;
- `raygun_bolt.md3`: the bolt, a stretched glowing drop along +X;
- `coins_small.md3`, `coins_medium.md3`, `coins_big.md3`: gold coins in a heap of 5, a
  heap of 14 and a heap of 30, about 6, 9 and 12 units across, origin under the middle.

    python scripts/box_loot_model.py crates/sjk-viewer/assets/mystery_box

The output is deterministic (a fixed seed). Needs Pillow and numpy
(pip install pillow numpy). Its shaders are in `mystery_box.shader`, written by hand.
"""

import math
import struct
import sys
from pathlib import Path

import numpy as np
from PIL import Image

SHADER = "models/sjk/box_loot"
GLOW_SHADER = "models/sjk/box_loot_glow"
ATLAS = 256
JPEG_QUALITY = 90
# Swatches, pixels (left, top, right, bottom).
STEEL = (0, 0, 128, 128)
RED = (128, 0, 256, 64)
GRIP = (128, 64, 256, 128)
GOLD = (0, 128, 128, 256)
RIM = (128, 128, 256, 256)
# The glow picture is one swatch.
GLOW = (0, 0, 64, 64)
GLOW_SIZE = 64
SEGMENTS = 16
RINGS = 8


def noise(rng, size, scale: float) -> np.ndarray:
    """Smooth value noise of `size` (h, w), about `scale` pixels per feature, in 0..1."""
    h, w = size
    gh, gw = int(h / scale) + 2, int(w / scale) + 2
    grid = Image.fromarray((rng.random((gh, gw)) * 255).astype(np.uint8))
    big = grid.resize((int(gw * scale), int(gh * scale)), Image.BICUBIC)
    return np.asarray(big, dtype=float)[:h, :w] / 255.0


def pictures(output: Path) -> None:
    rng = np.random.default_rng(2611)
    atlas = np.zeros((ATLAS, ATLAS, 3))

    def fill(region, colour, rough: float, streak: bool = False) -> None:
        left, top, right, bottom = region
        h, w = bottom - top, right - left
        shade = 1.0 + rough * (noise(rng, (h, w), 3.0) - 0.5)
        if streak:
            # Brushed metal: fine lines across.
            lines = rng.random((h, 1)) * 0.25 - 0.125
            shade = shade + lines
        atlas[top:bottom, left:right] = np.array(colour)[None, None, :] * shade[:, :, None]

    fill(STEEL, (168.0, 174.0, 184.0), 0.25, streak=True)
    fill(RED, (196.0, 30.0, 26.0), 0.2)
    fill(GRIP, (46.0, 40.0, 38.0), 0.35)
    fill(GOLD, (232.0, 178.0, 52.0), 0.3)
    fill(RIM, (255.0, 214.0, 98.0), 0.2)
    Image.fromarray(np.clip(atlas, 0, 255).astype(np.uint8)).save(
        output / "box_loot.jpg", quality=JPEG_QUALITY
    )
    # The glow: bright green, whitest in the middle.
    y, x = np.mgrid[0:GLOW_SIZE, 0:GLOW_SIZE] / (GLOW_SIZE - 1) - 0.5
    core = np.clip(1.0 - np.hypot(x, y) * 1.6, 0.0, 1.0)
    glow = np.stack([60 + 150 * core, 200 + 55 * core, 40 + 120 * core], axis=-1)
    Image.fromarray(np.clip(glow, 0, 255).astype(np.uint8)).save(
        output / "box_loot_glow.jpg", quality=JPEG_QUALITY
    )


def encode_normal(n) -> int:
    # The inverse of sjk-model's `decode_normal` (stock's MD3 normal).
    around = round(math.atan2(n[1], n[0]) * 255 / (2 * math.pi)) & 0xFF
    from_up = round(math.acos(max(-1.0, min(1.0, n[2]))) * 255 / (2 * math.pi)) & 0xFF
    return (around << 8) | from_up


class Mesh:
    """One surface's vertices and triangles."""

    def __init__(self, atlas: int):
        self.positions, self.normals, self.uvs, self.triangles = [], [], [], []
        self.atlas = atlas

    def uv(self, region, s: float, t: float):
        left, top, right, bottom = region
        inset = 4
        return (
            (left + inset + (right - left - 2 * inset) * s) / self.atlas,
            (top + inset + (bottom - top - 2 * inset) * t) / self.atlas,
        )

    def add_triangle(self, tri, outward) -> None:
        p = [np.array(self.positions[i]) for i in tri]
        cross = np.cross(p[1] - p[0], p[2] - p[0])
        if np.linalg.norm(cross) < 1e-9:
            return
        # Clockwise seen from outside, stock's front side: the cross product points in.
        if np.dot(cross, outward) > 0:
            tri = (tri[0], tri[2], tri[1])
        self.triangles.append(tri)

    def ellipsoid(self, centre, radii, region, axis=None) -> None:
        """An ellipsoid; `axis` a 3x3 turn applied to its local frame."""
        turn = np.eye(3) if axis is None else np.array(axis)
        base = len(self.positions)
        for ring in range(RINGS + 1):
            lat = math.pi * ring / RINGS
            for segment in range(SEGMENTS + 1):
                lon = 2 * math.pi * segment / SEGMENTS
                unit = np.array(
                    (math.sin(lat) * math.cos(lon), math.sin(lat) * math.sin(lon), math.cos(lat))
                )
                local = unit * np.array(radii)
                self.positions.append(tuple(np.array(centre) + turn @ local))
                n = turn @ (unit / np.array(radii))
                self.normals.append(tuple(n / np.linalg.norm(n)))
                self.uvs.append(self.uv(region, segment / SEGMENTS, ring / RINGS))
        row = SEGMENTS + 1
        for ring in range(RINGS):
            for segment in range(SEGMENTS):
                a = base + ring * row + segment
                b, c, d = a + 1, a + row + 1, a + row
                for tri in ((a, b, c), (a, c, d)):
                    out = sum(np.array(self.normals[i]) for i in tri)
                    self.add_triangle(tri, out)

    def cylinder(self, start, end, radius_start, radius_end, region, caps=True) -> None:
        """A (possibly tapered) cylinder from `start` to `end`, with its caps."""
        start, end = np.array(start, float), np.array(end, float)
        axis = end - start
        length = np.linalg.norm(axis)
        w = axis / length
        u = np.cross(w, (0.0, 0.0, 1.0) if abs(w[2]) < 0.9 else (1.0, 0.0, 0.0))
        u /= np.linalg.norm(u)
        v = np.cross(w, u)
        slope = (radius_start - radius_end) / length
        base = len(self.positions)
        for end_index, (centre, radius) in enumerate(((start, radius_start), (end, radius_end))):
            for segment in range(SEGMENTS + 1):
                angle = 2 * math.pi * segment / SEGMENTS
                around = math.cos(angle) * u + math.sin(angle) * v
                self.positions.append(tuple(centre + around * radius))
                n = around + w * slope
                self.normals.append(tuple(n / np.linalg.norm(n)))
                self.uvs.append(self.uv(region, segment / SEGMENTS, float(end_index)))
        row = SEGMENTS + 1
        for segment in range(SEGMENTS):
            a = base + segment
            b, c, d = a + 1, a + row + 1, a + row
            for tri in ((a, b, c), (a, c, d)):
                out = sum(np.array(self.normals[i]) for i in tri)
                self.add_triangle(tri, out)
        if not caps:
            return
        for centre, radius, facing in ((start, radius_start, -w), (end, radius_end, w)):
            if radius <= 0.01:
                continue
            middle = len(self.positions)
            self.positions.append(tuple(centre))
            self.normals.append(tuple(facing))
            self.uvs.append(self.uv(region, 0.5, 0.5))
            first = len(self.positions)
            for segment in range(SEGMENTS):
                angle = 2 * math.pi * segment / SEGMENTS
                around = math.cos(angle) * u + math.sin(angle) * v
                self.positions.append(tuple(centre + around * radius))
                self.normals.append(tuple(facing))
                self.uvs.append(
                    self.uv(region, 0.5 + 0.5 * math.cos(angle), 0.5 + 0.5 * math.sin(angle))
                )
            for segment in range(SEGMENTS):
                a = first + segment
                b = first + (segment + 1) % SEGMENTS
                self.add_triangle((middle, a, b), facing)

    def box(self, centre, size, region, tilt_y: float = 0.0) -> None:
        """A box of `size` round `centre`, pitched `tilt_y` radians about Y."""
        c, s = math.cos(tilt_y), math.sin(tilt_y)
        turn = np.array([[c, 0, s], [0, 1, 0], [-s, 0, c]])
        half = np.array(size) / 2
        for axis in range(3):
            for sign in (-1.0, 1.0):
                normal = np.zeros(3)
                normal[axis] = sign
                others = [i for i in range(3) if i != axis]
                base = len(self.positions)
                for corner in ((-1, -1), (1, -1), (1, 1), (-1, 1)):
                    local = np.zeros(3)
                    local[axis] = sign * half[axis]
                    local[others[0]] = corner[0] * half[others[0]]
                    local[others[1]] = corner[1] * half[others[1]]
                    self.positions.append(tuple(np.array(centre) + turn @ local))
                    self.normals.append(tuple(turn @ normal))
                    self.uvs.append(self.uv(region, (corner[0] + 1) / 2, (corner[1] + 1) / 2))
                out = turn @ normal
                self.add_triangle((base, base + 1, base + 2), out)
                self.add_triangle((base, base + 2, base + 3), out)


def text(value: str, size: int) -> bytes:
    return value.encode("ascii").ljust(size, b"\0")


def surface_bytes(name: str, shader: str, mesh: Mesh) -> bytes:
    shaders = text(shader, 64) + struct.pack("<i", 0)
    tris = b"".join(struct.pack("<3i", *t) for t in mesh.triangles)
    sts = b"".join(struct.pack("<2f", *uv) for uv in mesh.uvs)
    verts = b"".join(
        struct.pack("<3hH", *(round(c * 64) for c in p), encode_normal(n))
        for p, n in zip(mesh.positions, mesh.normals)
    )
    header = 108
    ofs_tris = header + len(shaders)
    ofs_st = ofs_tris + len(tris)
    ofs_verts = ofs_st + len(sts)
    end = ofs_verts + len(verts)
    return (
        b"IDP3"
        + text(name, 64)
        + struct.pack(
            "<10i", 0, 1, 1, len(mesh.positions), len(mesh.triangles),
            ofs_tris, header, ofs_st, ofs_verts, end,
        )
        + shaders + tris + sts + verts
    )


def write_md3(path: Path, name: str, surfaces) -> None:
    """`surfaces`: (surface name, shader, mesh)."""
    blobs = [surface_bytes(n, shader, mesh) for n, shader, mesh in surfaces]
    points = np.array([p for _, _, mesh in surfaces for p in mesh.positions])
    mins, maxs = points.min(axis=0), points.max(axis=0)
    centre = (mins + maxs) / 2
    radius = float(np.linalg.norm(maxs - centre))
    frame = struct.pack("<10f", *mins, *maxs, *centre, radius) + text(name, 16)
    ofs_frames = 108
    ofs_surfaces = ofs_frames + len(frame)
    end = ofs_surfaces + sum(len(blob) for blob in blobs)
    header = (
        b"IDP3"
        + struct.pack("<i", 15)
        + text(f"models/sjk/{name}.md3", 64)
        + struct.pack(
            "<9i", 0, 1, 0, len(blobs), 0, ofs_frames, ofs_surfaces, ofs_surfaces, end
        )
    )
    path.write_bytes(header + frame + b"".join(blobs))


def raygun(output: Path) -> None:
    gun = Mesh(ATLAS)
    glow = Mesh(GLOW_SIZE)
    # The barrel along +X, over the grip.
    gun.cylinder((-3.0, 0, 2.6), (6.5, 0, 2.6), 1.15, 0.95, STEEL)
    # The round chamber at the back.
    gun.ellipsoid((-3.2, 0, 2.6), (2.0, 1.75, 1.75), STEEL)
    # Three red fins round the front, the biggest at the back.
    for x, radius in ((3.0, 2.3), (4.3, 1.95), (5.6, 1.6)):
        gun.cylinder((x - 0.22, 0, 2.6), (x + 0.22, 0, 2.6), radius, radius, RED)
    # The emitter: a cone, then a small red ring at its tip.
    gun.cylinder((6.5, 0, 2.6), (8.6, 0, 2.6), 0.95, 0.35, STEEL)
    gun.cylinder((8.4, 0, 2.6), (8.9, 0, 2.6), 0.55, 0.55, RED)
    # The grip, raked back, and a trigger guard's block.
    gun.box((-1.4, 0, -0.4), (1.7, 1.3, 4.6), GRIP, tilt_y=0.32)
    gun.box((0.4, 0, 0.9), (1.6, 0.5, 0.5), GRIP)
    # The green tube along the top, glowing (its own surface).
    glow.cylinder((-2.0, 0, 4.15), (2.8, 0, 4.15), 0.55, 0.55, GLOW)
    glow.ellipsoid((-2.0, 0, 4.15), (0.55, 0.55, 0.55), GLOW)
    glow.ellipsoid((2.8, 0, 4.15), (0.55, 0.55, 0.55), GLOW)
    # The tube's steel mounts.
    for x in (-1.4, 2.2):
        gun.box((x, 0, 3.6), (0.5, 0.5, 0.9), STEEL)
    write_md3(
        output / "raygun.md3",
        "raygun",
        [("raygun", SHADER, gun), ("raygun_glow", GLOW_SHADER, glow)],
    )
    bolt = Mesh(GLOW_SIZE)
    bolt.ellipsoid((0, 0, 0), (5.0, 1.6, 1.6), GLOW)
    write_md3(output / "raygun_bolt.md3", "raygun_bolt", [("raygun_bolt", GLOW_SHADER, bolt)])


def coins(output: Path, name: str, count: int, spread: float, seed: int) -> None:
    """A heap of `count` coins: lying flat in a rough cone, the top ones tilted."""
    rng = np.random.default_rng(seed)
    gold = Mesh(ATLAS)
    radius, thick = 1.25, 0.32
    placed = []
    for index in range(count):
        # Fill from the bottom: each coin rests on what lies under it.
        for _ in range(40):
            r = spread * math.sqrt(rng.random())
            a = rng.random() * 2 * math.pi
            x, y = r * math.cos(a), r * math.sin(a)
            under = [z for (px, py, z) in placed if math.hypot(px - x, py - y) < radius * 1.6]
            z = max(under) + thick if under else thick / 2
            # Keep a heap: higher only towards the middle.
            if z <= (spread - r) * 0.9 + thick:
                break
        placed.append((x, y, z))
        tilt = 0.0 if not under else rng.random() * 0.35
        heading = rng.random() * 2 * math.pi
        normal = np.array(
            (math.sin(tilt) * math.cos(heading), math.sin(tilt) * math.sin(heading), math.cos(tilt))
        )
        centre = np.array((x, y, z))
        gold.cylinder(
            tuple(centre - normal * thick / 2), tuple(centre + normal * thick / 2),
            radius, radius, GOLD,
        )
        # A brighter rim face on top.
        gold.cylinder(
            tuple(centre + normal * thick * 0.45), tuple(centre + normal * thick * 0.55),
            radius * 0.82, radius * 0.82, RIM, caps=True,
        )
    write_md3(output / f"{name}.md3", name, [(name, SHADER, gold)])


def main() -> None:
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    output = Path(sys.argv[1])
    output.mkdir(parents=True, exist_ok=True)
    pictures(output)
    raygun(output)
    coins(output, "coins_small", 5, 1.6, 11)
    coins(output, "coins_medium", 14, 3.0, 12)
    coins(output, "coins_big", 30, 4.4, 13)


if __name__ == "__main__":
    main()
