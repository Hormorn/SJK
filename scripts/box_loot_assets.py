"""Make the Ray Gun's sounds (`docs/mystery-boxes.md`, SJK's own): synthesized here,
nothing taken from another game.

- `raygun_fire.wav`: the shot, a bright falling zap over a wobbling hum, 0.35 s;
- `raygun_impact.wav`: a bolt meeting something, a fizzing crackle, 0.3 s;
- `raygun_raise.wav`: the gun taken in hand, a rising whir and a click, 0.6 s;
- `raygun_shield.wav`: the shield rising round a player struck once, a soft bright
  chord, 0.8 s.

    python scripts/box_loot_assets.py crates/sjk-viewer/assets/mystery_box

22050 Hz, mono, 16-bit. Deterministic (a fixed seed). Needs numpy.
"""

import sys
import wave
from pathlib import Path

import numpy as np

RATE = 22050


def write(path: Path, samples: np.ndarray) -> None:
    peak = np.max(np.abs(samples)) or 1.0
    data = (samples / peak * 0.85 * 32767).astype("<i2")
    with wave.open(str(path), "wb") as out:
        out.setnchannels(1)
        out.setsampwidth(2)
        out.setframerate(RATE)
        out.writeframes(data.tobytes())


def envelope(t: np.ndarray, attack: float, length: float) -> np.ndarray:
    rise = np.clip(t / attack, 0.0, 1.0)
    fall = np.clip(1.0 - (t - attack) / (length - attack), 0.0, 1.0) ** 2
    return np.where(t < attack, rise, fall)


def fire() -> np.ndarray:
    length = 0.35
    t = np.arange(int(RATE * length)) / RATE
    # A zap falling from 2400 Hz to 300 Hz, its phase the integral of the sweep.
    frequency = 300 + 2100 * np.exp(-t * 14)
    phase = 2 * np.pi * np.cumsum(frequency) / RATE
    zap = np.sign(np.sin(phase)) * 0.35 + np.sin(phase) * 0.65
    wobble = np.sin(2 * np.pi * 110 * t + 3 * np.sin(2 * np.pi * 23 * t)) * 0.4
    return (zap + wobble) * envelope(t, 0.005, length)


def impact() -> np.ndarray:
    rng = np.random.default_rng(2611)
    length = 0.3
    t = np.arange(int(RATE * length)) / RATE
    crackle = rng.standard_normal(t.size) * (rng.random(t.size) > 0.7)
    tone = np.sin(2 * np.pi * (900 - 1800 * t) * t)
    return (crackle * 0.7 + tone * 0.5) * envelope(t, 0.003, length)


def raise_() -> np.ndarray:
    length = 0.6
    t = np.arange(int(RATE * length)) / RATE
    frequency = 180 + 900 * (t / length) ** 1.5
    phase = 2 * np.pi * np.cumsum(frequency) / RATE
    whir = np.sin(phase) * 0.6 + np.sin(phase * 2.01) * 0.25
    whir *= envelope(t, 0.05, length)
    click = np.zeros_like(t)
    at = int(RATE * 0.5)
    click[at : at + 120] = np.sin(2 * np.pi * 3000 * t[:120]) * np.linspace(1, 0, 120)
    return whir + click


def shield() -> np.ndarray:
    length = 0.8
    t = np.arange(int(RATE * length)) / RATE
    chord = sum(np.sin(2 * np.pi * f * t) for f in (523.25, 659.25, 783.99, 1046.5))
    shimmer = 1.0 + 0.25 * np.sin(2 * np.pi * 9 * t)
    return chord * shimmer * envelope(t, 0.04, length)


def main() -> None:
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    output = Path(sys.argv[1])
    output.mkdir(parents=True, exist_ok=True)
    write(output / "raygun_fire.wav", fire())
    write(output / "raygun_impact.wav", impact())
    write(output / "raygun_raise.wav", raise_())
    write(output / "raygun_shield.wav", shield())


if __name__ == "__main__":
    main()
