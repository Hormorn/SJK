"""Make the WHEEL OF DOOM's sounds (`docs/wheel-of-doom.md`, SJK's own).

Nothing here comes from another game: every sound is synthesized. This script writes,
into the output folder, 22050 Hz mono 16-bit:

- `tick.wav`: a peg clacking past the pointer (the client plays one each time a peg
  passes, so the ticks slow with the wheel);
- `stop.wav`: the wheel stopping, a low thud and a bell;
- `fanfare.wav`: the jackpot, a short brass-like fanfare;
- `quack.wav`: the Quack effect's quack, a nasal buzz falling in pitch.

    python scripts/wheel_of_doom_assets.py crates/sjk-viewer/assets/wheel_of_doom

The output is deterministic (a fixed seed). Needs numpy (pip install numpy); it uses the
WAV writer of mystery_box_assets.py.
"""

import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))
import mystery_box_assets as box  # noqa: E402

RATE = box.RATE


def seconds(length: float) -> np.ndarray:
    return np.arange(int(length * RATE)) / RATE


def tick(rng) -> np.ndarray:
    t = seconds(0.06)
    click = rng.standard_normal(len(t)) * np.exp(-t / 0.004)
    knock = np.sin(2 * np.pi * 1900 * t) * np.exp(-t / 0.012) * 0.6
    return click * 0.7 + knock


def stop(rng) -> np.ndarray:
    t = seconds(1.6)
    thud = np.sin(2 * np.pi * 70 * t) * np.exp(-t / 0.08)
    thud += rng.standard_normal(len(t)) * np.exp(-t / 0.01) * 0.3
    bell = np.zeros_like(t)
    for ratio, gain in ((1.0, 1.0), (2.76, 0.4), (5.4, 0.2)):
        bell += gain * np.sin(2 * np.pi * 520 * ratio * t) * np.exp(-t / (0.5 / ratio))
    return thud + bell * 0.5


def brass(note: int, length: float) -> np.ndarray:
    t = seconds(length)
    f = box.hz(note)
    out = np.zeros_like(t)
    for harmonic in range(1, 9):
        out += np.sin(2 * np.pi * f * harmonic * t) / harmonic**1.1
    attack = np.minimum(t / 0.03, 1.0)
    release = np.clip((length - t) / 0.08, 0.0, 1.0)
    vibrato = 1 + 0.03 * np.sin(2 * np.pi * 5.5 * t) * np.minimum(t / 0.3, 1.0)
    return out * attack * release * vibrato


def fanfare() -> np.ndarray:
    out = np.zeros(int(2.6 * RATE))
    # Da-da-da-DAAA, then a chord.
    for at, note, length in ((0.0, 67, 0.16), (0.18, 67, 0.16), (0.36, 67, 0.16), (0.54, 72, 0.6)):
        box.mix(out, brass(note, length), at)
    for note in (72, 76, 79, 84):
        box.mix(out, brass(note, 1.4) * 0.6, 1.2)
    return out


def quack() -> np.ndarray:
    t = seconds(0.28)
    # A falling buzz through two nasal formants.
    pitch = 520 * (1 - 0.35 * t / t[-1])
    phase = 2 * np.pi * np.cumsum(pitch) / RATE
    buzz = np.sign(np.sin(phase)) * 0.5 + np.sin(phase * 2) * 0.3
    spectrum = np.fft.rfft(buzz)
    freqs = np.fft.rfftfreq(len(buzz), 1 / RATE)
    shape = np.exp(-((freqs - 1100) / 380) ** 2) + 0.7 * np.exp(-((freqs - 2600) / 500) ** 2)
    voiced = np.fft.irfft(spectrum * shape, len(buzz))
    envelope = np.minimum(t / 0.015, 1.0) * np.exp(-t / 0.16)
    return voiced * envelope


def main() -> None:
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    folder = Path(sys.argv[1])
    folder.mkdir(parents=True, exist_ok=True)
    rng = np.random.default_rng(7)
    box.write_wav(folder / "tick.wav", tick(rng))
    box.write_wav(folder / "stop.wav", stop(rng))
    box.write_wav(folder / "fanfare.wav", fanfare())
    box.write_wav(folder / "quack.wav", quack())


if __name__ == "__main__":
    main()
