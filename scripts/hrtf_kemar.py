"""Build the head-related impulse responses of SJK's 3D sound (`s_spatial`).

Input: the MIT Media Lab KEMAR measurements, diffuse-field equalized set
(https://sound.media.mit.edu/resources/KEMAR/diffuse.zip, unpacked; Gardner and
Martin, 1994: "provided free with no restrictions on use, provided the authors are
cited"). Output: `crates/sjk-audio/data/hrtf_kemar.bin`, read by
`crates/sjk-audio/src/hrtf.rs`.

Each measured direction becomes a minimum-phase filter per ear (the same magnitude,
its energy first) and an onset delay per ear: how much later than the near ear the
sound reaches it (the near ear's is 0, so turning never slides every delay at once). The
mixer interpolates filters and delays between directions without the comb filtering
interpolating raw impulse responses gives.

Usage: python scripts/hrtf_kemar.py <unpacked diffuse folder> [output]

File layout, little endian:
  b"SJKH", u16 version 1, u32 sample rate, u16 taps, u16 entries, f32 tap scale
  entries x { i16 elevation deg, i16 azimuth deg (0 front, 90 right, 0..180),
              f32 left delay, f32 right delay (samples),
              taps x i16 left, taps x i16 right }
Taps are tap / 32767 * scale.
"""

import os
import re
import struct
import sys
import wave

import numpy as np
from scipy.signal import resample_poly

TAPS = 64
FFT = 2048
UPSAMPLE = 16
ONSET_FRACTION = 0.15
FADE = 16


def read(path):
    with wave.open(path) as w:
        assert w.getnchannels() == 2 and w.getsampwidth() == 2
        rate = w.getframerate()
        data = np.frombuffer(w.readframes(w.getnframes()), dtype="<i2")
    return rate, data.reshape(-1, 2).T.astype(np.float64) / 32768.0


def onset(ear):
    fine = resample_poly(ear, UPSAMPLE, 1)
    peak = np.abs(fine).max()
    return float(np.argmax(np.abs(fine) >= ONSET_FRACTION * peak)) / UPSAMPLE


def minimum_phase(ear):
    spectrum = np.fft.fft(ear, FFT)
    log_magnitude = np.log(np.maximum(np.abs(spectrum), 1e-6))
    cepstrum = np.fft.ifft(log_magnitude).real
    fold = np.zeros(FFT)
    fold[0] = 1.0
    fold[1 : FFT // 2] = 2.0
    fold[FFT // 2] = 1.0
    minimum = np.fft.ifft(np.exp(np.fft.fft(cepstrum * fold))).real[:TAPS]
    minimum[-FADE:] *= 0.5 * (1.0 + np.cos(np.linspace(0.0, np.pi, FADE)))
    return minimum


def main():
    folder = sys.argv[1]
    output = sys.argv[2] if len(sys.argv) > 2 else os.path.join(
        os.path.dirname(__file__), "..", "crates", "sjk-audio", "data", "hrtf_kemar.bin"
    )
    entries = []
    rate = None
    pattern = re.compile(r"H(-?\d+)e(\d{3})a\.wav$")
    for root, _, files in os.walk(folder):
        for name in files:
            match = pattern.match(name)
            if not match:
                continue
            file_rate, ears = read(os.path.join(root, name))
            assert rate in (None, file_rate)
            rate = file_rate
            elevation, azimuth = int(match[1]), int(match[2])
            delays = [onset(ear) for ear in ears]
            filters = [minimum_phase(ear) for ear in ears]
            entries.append((elevation, azimuth, delays, filters))
    entries.sort(key=lambda e: (e[0], e[1]))
    scale = max(np.abs(f).max() for e in entries for f in e[3])
    with open(output, "wb") as out:
        out.write(b"SJKH")
        out.write(struct.pack("<HIHHf", 1, rate, TAPS, len(entries), scale))
        for elevation, azimuth, delays, filters in entries:
            out.write(struct.pack("<hhff", elevation, azimuth, *(d - min(delays) for d in delays)))
            for ear in filters:
                taps = np.round(ear / scale * 32767.0).astype("<i2")
                out.write(taps.tobytes())
    print(f"{len(entries)} directions, {TAPS} taps at {rate} Hz -> {output}")


if __name__ == "__main__":
    main()
