#!/usr/bin/env python3
"""Make the Malachor Rose blade's body picture (`assets/blade_body/malachor_rose.png`).

An original picture, drawn from nothing but this script: pale rose energy, clouded with white
and a few deeper magenta knots, seamless along its length (the blade scrolls it), soft at its
two long edges. Run it again to remake the file; the seed keeps it the same."""

import pathlib

import numpy as np
from PIL import Image

WIDTH, HEIGHT = 280, 72
OUT = pathlib.Path(__file__).resolve().parent.parent / "crates/sjk-viewer/assets/blade_body/malachor_rose.png"


def periodic_noise(rng, cells_x, cells_y):
    """Smooth value noise that wraps round left to right."""
    grid = rng.random((cells_y + 1, cells_x))
    xs = np.linspace(0.0, cells_x, WIDTH, endpoint=False)
    ys = np.linspace(0.0, cells_y, HEIGHT)
    x0 = np.floor(xs).astype(int)
    y0 = np.minimum(np.floor(ys).astype(int), cells_y - 1)
    fx = xs - x0
    fy = ys - y0
    fx = fx * fx * (3 - 2 * fx)
    fy = fy * fy * (3 - 2 * fy)
    x1 = (x0 + 1) % cells_x
    a = grid[y0][:, x0]
    b = grid[y0][:, x1]
    c = grid[y0 + 1][:, x0]
    d = grid[y0 + 1][:, x1]
    top = a + (b - a) * fx
    bottom = c + (d - c) * fx
    return top + (bottom - top) * fy[:, None]


def fbm(rng, base_x, base_y, octaves):
    total = np.zeros((HEIGHT, WIDTH))
    weight = 0.0
    amp = 1.0
    for octave in range(octaves):
        total += amp * periodic_noise(rng, base_x * 2**octave, base_y * 2**octave)
        weight += amp
        amp *= 0.55
    return total / weight


def main():
    rng = np.random.default_rng(170)
    clouds = fbm(rng, 6, 2, 5)
    white = fbm(rng, 10, 3, 4)
    knots = fbm(rng, 4, 1, 3)
    rose = np.array([0.93, 0.55, 0.9])
    deep = np.array([0.78, 0.3, 0.8])
    pale = np.array([1.0, 0.93, 0.99])
    knot = np.clip((knots - 0.55) * 4.0, 0.0, 1.0)[..., None]
    colour = rose + (deep - rose) * knot
    mist = np.clip((white - 0.45) * 2.6 + (clouds - 0.5) * 1.2, 0.0, 1.0)[..., None]
    colour = colour + (pale - colour) * mist
    # soft along the two long edges, as light thins at a blade's sides
    edge = np.abs(np.linspace(-1.0, 1.0, HEIGHT))[:, None, None]
    colour = colour + (pale - colour) * np.clip((edge - 0.7) * 2.2, 0.0, 1.0)
    pixels = (np.clip(colour, 0.0, 1.0) * 255.0 + 0.5).astype(np.uint8)
    OUT.parent.mkdir(parents=True, exist_ok=True)
    Image.fromarray(pixels, "RGB").save(OUT, optimize=True)
    print(f"written: {OUT}")


if __name__ == "__main__":
    main()
