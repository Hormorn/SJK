//! A small in-place radix-2 FFT for the 3D sound's convolution (`convolver.rs`):
//! one fixed size, its twiddles and bit-reversal table built once, nothing
//! allocated per transform.

/// A complex number of the transforms.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Complex {
    pub re: f32,
    pub im: f32,
}

impl Complex {
    pub const ZERO: Self = Self { re: 0.0, im: 0.0 };

    pub const fn new(re: f32, im: f32) -> Self {
        Self { re, im }
    }

    #[inline]
    pub fn times(self, other: Self) -> Self {
        Self {
            re: self.re * other.re - self.im * other.im,
            im: self.re * other.im + self.im * other.re,
        }
    }

    #[inline]
    pub fn conj(self) -> Self {
        Self {
            re: self.re,
            im: -self.im,
        }
    }
}

/// Transforms of one power-of-two size.
pub struct Fft {
    size: usize,
    twiddles: Box<[Complex]>,
    reversed: Box<[u32]>,
}

impl Fft {
    /// A transform of `size` points (a power of two, at least 2).
    pub fn new(size: usize) -> Self {
        assert!(size.is_power_of_two() && size >= 2, "FFT size {size}");
        let bits = size.trailing_zeros();
        let twiddles = (0..size / 2)
            .map(|k| {
                let angle = -std::f64::consts::TAU * k as f64 / size as f64;
                Complex::new(angle.cos() as f32, angle.sin() as f32)
            })
            .collect();
        let reversed = (0..size as u32)
            .map(|index| index.reverse_bits() >> (32 - bits))
            .collect();
        Self {
            size,
            twiddles,
            reversed,
        }
    }

    /// The forward transform of `data` in place (no scaling).
    pub fn forward(&self, data: &mut [Complex]) {
        self.transform(data, false);
    }

    /// The inverse transform of `data` in place, scaled by `1 / size`.
    pub fn inverse(&self, data: &mut [Complex]) {
        self.transform(data, true);
        let scale = 1.0 / self.size as f32;
        for value in data.iter_mut() {
            value.re *= scale;
            value.im *= scale;
        }
    }

    fn transform(&self, data: &mut [Complex], inverse: bool) {
        let size = self.size;
        assert_eq!(data.len(), size);
        for index in 0..size {
            let other = self.reversed[index] as usize;
            if other > index {
                data.swap(index, other);
            }
        }
        let mut span = 1;
        while span < size {
            let stride = size / (span * 2);
            for start in (0..size).step_by(span * 2) {
                for offset in 0..span {
                    let twiddle = self.twiddles[offset * stride];
                    let twiddle = if inverse { twiddle.conj() } else { twiddle };
                    let low = data[start + offset];
                    let high = data[start + offset + span].times(twiddle);
                    data[start + offset] = Complex::new(low.re + high.re, low.im + high.im);
                    data[start + offset + span] = Complex::new(low.re - high.re, low.im - high.im);
                }
            }
            span *= 2;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_transform_and_its_inverse_give_the_signal_back() {
        let fft = Fft::new(64);
        let signal: Vec<Complex> = (0..64)
            .map(|i| Complex::new((i as f32 * 0.37).sin(), (i as f32 * 0.11).cos()))
            .collect();
        let mut data = signal.clone();
        fft.forward(&mut data);
        fft.inverse(&mut data);
        for (a, b) in data.iter().zip(&signal) {
            assert!((a.re - b.re).abs() < 1e-5 && (a.im - b.im).abs() < 1e-5);
        }
    }

    #[test]
    fn a_cosine_lands_in_its_bin() {
        let fft = Fft::new(32);
        let mut data: Vec<Complex> = (0..32)
            .map(|i| Complex::new((std::f32::consts::TAU * 3.0 * i as f32 / 32.0).cos(), 0.0))
            .collect();
        fft.forward(&mut data);
        assert!((data[3].re - 16.0).abs() < 1e-3);
        assert!((data[29].re - 16.0).abs() < 1e-3);
        assert!(data[5].re.abs() < 1e-3);
    }
}
