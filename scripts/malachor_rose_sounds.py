#!/usr/bin/env python3
"""Make the Malachor Rose blade's sounds: on, off, hum and three swings.

Original sounds, synthesised from nothing but this script (sines, a little shaped noise):
a low saber hum under a thin, slowly breathing shimmer high above it, the light the blade
is made of. Written as 22050 Hz mono 16-bit WAVs into the folder given (default
`target/malachor_rose_sounds`), named as the blade's file asks: `sound/sjk/creyon/malachor_rose/`
in the hub's `sjk_skins_2` pack. The seed keeps them the same from run to run."""

import math
import pathlib
import random
import struct
import sys
import wave

RATE = 22050
HUM_SECONDS = 4.0  # every partial below makes whole cycles in it, so the hum loops seamlessly


def write(path, samples):
    peak = max(1e-9, max(abs(s) for s in samples))
    scale = 0.89 / peak
    with wave.open(str(path), "wb") as out:
        out.setnchannels(1)
        out.setsampwidth(2)
        out.setframerate(RATE)
        out.writeframes(b"".join(struct.pack("<h", int(max(-1.0, min(1.0, s * scale)) * 32767)) for s in samples))


def hum_at(t, pitch=1.0):
    """The blade's voice at time t: a 90 Hz saber drone with its overtones, and a soft
    shimmer of two near high tones beating slowly against each other."""
    base = 90.0 * pitch
    drone = (
        math.sin(2 * math.pi * base * t)
        + 0.5 * math.sin(2 * math.pi * 2 * base * t + 0.4)
        + 0.22 * math.sin(2 * math.pi * 3 * base * t + 1.1)
        + 0.1 * math.sin(2 * math.pi * 5 * base * t + 2.0)
    )
    breath = 0.6 + 0.4 * math.sin(2 * math.pi * 0.5 * t)
    shimmer = math.sin(2 * math.pi * 742.5 * pitch * t) + math.sin(2 * math.pi * 747.5 * pitch * t + 0.7)
    sparkle = 0.5 * math.sin(2 * math.pi * 1485.0 * pitch * t + 1.9)
    return 0.62 * drone + 0.16 * breath * (shimmer + sparkle)


def smooth_noise(rng, count, smoothing):
    value = 0.0
    out = []
    for _ in range(count):
        value += (rng.uniform(-1.0, 1.0) - value) * smoothing
        out.append(value)
    return out


def hum():
    n = int(HUM_SECONDS * RATE)
    return [hum_at(i / RATE) for i in range(n)]


def ignite(length, rising):
    """On (rising) or off (falling): the drone sweeps up out of nothing or down into it,
    a breath of air through it and the shimmer opening or closing like a held chord."""
    rng = random.Random(300 + int(rising))
    n = int(length * RATE)
    air = smooth_noise(rng, n, 0.08)
    out = []
    phase = 0.0
    shimmer_phase = 0.0
    for i in range(n):
        x = i / n
        k = x if rising else 1.0 - x
        pitch = 0.35 + 0.65 * k**0.6
        phase += 2 * math.pi * 90.0 * pitch / RATE
        shimmer_phase += 2 * math.pi * 745.0 * (0.8 + 0.2 * k) / RATE
        level = (math.sin(math.pi * 0.5 * min(1.0, x * 6.0)) if rising else 1.0) * (
            1.0 if rising else (1.0 - x) ** 1.4
        )
        drone = math.sin(phase) + 0.5 * math.sin(2 * phase + 0.4) + 0.2 * math.sin(3 * phase + 1.1)
        bloom = math.sin(math.pi * x) ** 2
        out.append(level * (0.6 * drone + 0.25 * k * math.sin(shimmer_phase)) + 0.35 * bloom * air[i])
    return out


def swing(seed, length, depth):
    """A swing: the hum bends up and back down as the blade passes, with a rush of air
    that rises and falls with it."""
    rng = random.Random(seed)
    n = int(length * RATE)
    air = smooth_noise(rng, n, 0.12)
    out = []
    phase = 0.0
    shimmer_phase = 0.0
    for i in range(n):
        x = i / n
        pass_ = math.sin(math.pi * x) ** 1.5
        pitch = 1.0 + depth * pass_
        phase += 2 * math.pi * 90.0 * pitch / RATE
        shimmer_phase += 2 * math.pi * 745.0 * pitch / RATE
        drone = math.sin(phase) + 0.5 * math.sin(2 * phase + 0.4) + 0.22 * math.sin(3 * phase + 1.1)
        out.append(pass_ * (0.55 * drone + 0.2 * math.sin(shimmer_phase) + 0.55 * air[i]))
    return out


def main():
    folder = pathlib.Path(sys.argv[1]) if len(sys.argv) > 1 else pathlib.Path("target/malachor_rose_sounds")
    folder.mkdir(parents=True, exist_ok=True)
    write(folder / "hum.wav", hum())
    write(folder / "on.wav", ignite(0.9, True))
    write(folder / "off.wav", ignite(0.7, False))
    for index, (seed, length, depth) in enumerate([(11, 0.62, 0.35), (12, 0.74, 0.45), (13, 0.55, 0.28)], 1):
        write(folder / f"swing{index}.wav", swing(seed, length, depth))
    print(f"written: {folder}")


if __name__ == "__main__":
    main()
