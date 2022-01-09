/**
 * A rust implementation of fast fourier transforms.
 * Uses an algorithm described at https://cp-algorithms.com/algebra/fft.html.
 */
use crate::complex::Complex;
use num::{traits::Float, NumCast};
use std::error::Error;

fn to_t<R: NumCast, T: Float>(input: R) -> Result<T, Box<dyn Error>> {
    Ok(T::from::<R>(input).ok_or("cannot convert R to T")?)
}

fn is_power_of_two(x: usize) -> bool {
    // We need a special case for zero because Rust disallows underflow
    // the implementation would still be correct if x - 1 wrapped to MAX_INT
    match x {
        0 => false,
        x => (x > 0) & ((x & (x - 1)) == 0),
    }
}

#[cfg(test)]
mod is_power_of_two_tests {
    use super::is_power_of_two;

    #[test]
    fn test_zero() {
        assert_eq!(is_power_of_two(0), false);
    }

    #[test]
    fn test_two_powers() {
        let x: usize = 1;
        for i in 0..64 {
            assert_eq!(is_power_of_two(x << i), true);
        }
    }

    #[test]
    fn test_not_two_powers() {
        let x: usize = 3;
        // 3 will overflow if we raise it 2^62
        for i in 0..62 {
            println!("{}, {}", i, x << i);
            assert_eq!(is_power_of_two(x << i), false);
        }
    }
}

/// If a number is a power of two this returns the index of the bit that is set
fn pow2_index(size: usize) -> usize {
    size.trailing_zeros() as usize
}

#[cfg(test)]
mod pow2_index_tests {
    use super::pow2_index;

    #[test]
    fn test_pow2_1() {
        for i in 0..64 {
            assert_eq!(pow2_index(1 << i), i);
        }
    }
}

fn frequency_in_hz_of_sample<R: NumCast, T: Float>(
    sample_index: R,
    num_samples: R,
    sample_rate: T,
) -> Result<T, Box<dyn Error>> {
    let sample_index: T = to_t(sample_index)?;
    let num_samples: T = to_t(num_samples)?;
    Ok(sample_rate * (sample_index / num_samples))
}

#[cfg(test)]
mod frequency_from_index_tests {
    use super::frequency_in_hz_of_sample;

    #[test]
    fn test_conversion() {
        let step = |x: f64| frequency_in_hz_of_sample(x, 4., 44000.).unwrap();

        assert_eq!(step(0.), 0.);
        assert_eq!(step(1.), 11000.);
        assert_eq!(step(2.), 22000.);
        assert_eq!(step(3.), 33000.);
        assert_eq!(step(4.), 44000.);
    }
}

/// Bitwise reverse of a usize with respect to a maximum size of 'at'
fn bitwise_reverse(input: usize, at: usize) -> usize {
    let mut result = 0;
    for i in 0..at {
        // If the bit is set then set the corresponding bit in the reversed usize
        if input & (1 << i) != 0 {
            result = result | 1 << (at - 1 - i);
        }
    }
    result
}

fn bitwise_reverse_permute<T: Float>(input: &mut [Complex<T>], size: usize) {
    // size is a power of two so only one bit in the bitvector that represents it is set.
    // bitwise reverse expects the index of the bit that is set in size. We can work back to it by
    // counting the leading zeros and subtracting them from the size of size as long as size is a
    // power of two (guarded in do_fft).
    let size_bit = pow2_index(size);
    for i in 0..size {
        let ri = bitwise_reverse(i, size_bit);
        if i < ri {
            input.swap(i, ri);
        }
    }
}

// TODO: Tests

// TODO: We could cache the permutation and angle for some small performance increase using a
// struct but that would make the Api a little more complex. Consider it later.
pub fn do_fft<T: Float>(input: &mut [Complex<T>], inverse: bool) -> Result<(), Box<dyn Error>> {
    // This algorithm does not work on non-pow2 input vectors but it is fine to pad the input
    // vector with zero-values.
    if !is_power_of_two(input.len()) {
        return Err("expected a power of two".into());
    }

    // To remove the recursion from the FFT we can do a bitwise permutation
    // on the indices of elements in the array. This relies on a property
    // of the fast fourier transform algorithm that the slices of the array
    // we would recurse on (split the array into odd and even indices and then
    // use the 'butterfly' of x = wx for the half and x = x - wx for the second
    // half to combine the two DFTs into a single array).
    bitwise_reverse_permute(input, input.len());

    // We start by doing a butterfly on all pairs of size 2 in the reversed array
    // then keep doubling the array size until we have done a butterfly on the full
    // array. This does the recursive procedure from the bottom up rather than the
    // top down which we can only do because of the reversed order.
    let mut len = 2;
    let max_len = input.len();

    let one = to_t(1.)?;

    // len tracks the current number of elements for the next butterfly
    while len <= max_len {
        // The complex length we multiply elements by varies by the length of the DFT so we can
        // cache it between iterations at the same level but not between applications at different
        // levels.
        // TODO: If we have some struct FFT we could cache the permutation list and this in memory.
        let angle: f64 = 2. * (std::f64::consts::PI / len as f64) * if inverse { -1. } else { 1. };
        let wlen = Complex::complex(to_t(angle.cos())?, to_t(angle.sin())?);
        let half_len = len / 2;

        for i in (0..max_len).step_by(len) {
            let mut w = Complex::real(one);
            for j in 0..half_len {
                let current = input[i + j];
                let scalar = input[i + j + half_len] * w;
                input[i + j] = current + scalar;
                input[i + j + half_len] = current - scalar;
                w *= wlen;
            }
        }

        // Double len before the next round of DFT applications (move one level up the recursion
        // chain).
        len = len << 1;
    }

    if inverse {
        for i in 0..max_len {
            input[i] /= Complex::real(to_t(max_len)?);
        }
    }

    Ok(())
}

/// Pad a frame to the specified number of elements, filling the remaining with zeros
pub fn round_to(
    mut frame: Vec<Complex<f64>>,
    new_len: usize,
) -> Result<Vec<Complex<f64>>, Box<dyn Error>> {
    match frame.len() {
        sz if sz == new_len => Ok(frame),
        sz if sz > new_len => Err("frame is already larger than desired size".into()),
        sz => {
            let new_entries = new_len - sz;
            for _ in 0..new_entries {
                frame.push(Complex::real(0.));
            }
            Ok(frame)
        }
    }
}

/// Pad a frame to the nearest power of 2 of entries for the fast-fourier transform
pub fn round_to_nearest_pow2(
    mut frame: Vec<Complex<f64>>,
) -> Result<Vec<Complex<f64>>, Box<dyn Error>> {
    let current_len = frame.len();
    let new_len = current_len.next_power_of_two();
    round_to(frame, new_len)
}

#[cfg(test)]
mod round_to_tests {
    use super::{round_to, round_to_nearest_pow2, Complex};

    #[test]
    fn test_round_to_limits() {
        assert_eq!(
            round_to(vec![Complex::real(5.); 1024], 1024).unwrap().len(),
            1024
        );
        assert_eq!(round_to(vec![Complex::real(5.); 1024], 50).is_err(), true);
    }

    #[test]
    fn test_round_to() {
        let rounded = round_to(vec![Complex::complex(5., 3.); 65], 1024).unwrap();
        assert_eq!(rounded.len(), 1024);
        for i in 0..65 {
            assert_eq!(rounded[i].real, 5.);
            assert_eq!(rounded[i].imaginary, 3.);
        }
        for i in 65..1024 {
            assert_eq!(rounded[i].real, 0.);
            assert_eq!(rounded[i].imaginary, 0.);
        }
    }

    #[test]
    fn test_round_to_pow2() {
        assert_eq!(
            round_to_nearest_pow2(vec![Complex::real(5.); 1024])
                .unwrap()
                .len(),
            1024
        );
        assert_eq!(
            round_to_nearest_pow2(vec![Complex::real(5.); 565])
                .unwrap()
                .len(),
            1024
        );
    }
}

#[cfg(test)]
mod fft_test {
    use crate::complex::Complex;
    use crate::fft::do_fft;

    fn vec(sz: usize) -> Vec<Complex<f64>> {
        let mut f = Vec::with_capacity(sz);
        for i in 0..sz {
            f.push(Complex::real((i + 20) as f64));
        }
        f
    }

    #[test]
    fn not_a_power_of_two() {
        let mut inp = vec(20);
        assert_eq!(do_fft(&mut inp, false).is_err(), true);
    }

    // For asserting floats but permitting some rounding errors
    macro_rules! assert_delta {
        ($x:expr, $y:expr, $d:expr) => {
            if !($x - $y < $d || $y - $x < $d) {
                panic!();
            }
        };
    }

    #[test]
    fn inverse_inverses() {
        let mut inp = vec(256);
        let origin = inp.clone();
        assert_eq!(do_fft(&mut inp, false).is_err(), false);
        assert_eq!(do_fft(&mut inp, true).is_err(), false);

        for i in 0..inp.len() {
            assert_delta!(inp[i], origin[i], Complex::complex(0.00002, 0.00002));
        }
    }

    // TODO: Tests
}

/// Performs the FFT on real-value inputs and combines the two symmetric portions
/// buffers are pre-allocated to reduce allocator pressure
pub struct RealFft<T: Float> {
    buffer: Vec<Complex<T>>,
    result_buffer: Vec<(T, T)>,
    sample_rate: T,
}

impl<'a, T: Float> RealFft<T> {
    /// Create a new fft with a buffer of a specific sample size.
    /// sample_size must be a power of two.
    pub fn new(sample_size: usize, sample_rate: T) -> Result<Self, Box<dyn Error>> {
        if is_power_of_two(sample_size) {
            let zero = to_t(0.)?;
            Ok(RealFft {
                buffer: vec![Complex::real(zero); sample_size],
                result_buffer: vec![(zero, zero); sample_size / 2],
                sample_rate,
            })
        } else {
            Err("sample_size is not a power of two".into())
        }
    }

    /// Prepare a set of input reals as complex values in the fft buffer and pad the remaining
    /// buffer space with zeros.
    fn prepare_buffer(&mut self, data: &[T]) -> Result<(), Box<dyn Error>> {
        if data.len() >= self.buffer.capacity() {
            return Err("data supplied is larger than the FFT buffer".into());
        }

        let zero = to_t(0.)?;

        for i in 0..data.len() {
            self.buffer[i] = Complex::real(data[i]);
        }

        // We pad the fft frame to 2^16 elements which has the effect of interpolating values in
        // the fft.
        for i in data.len()..self.buffer.capacity() {
            self.buffer[i] = Complex::real(zero);
        }

        Ok(())
    }

    fn sample_window(&self) -> usize {
        self.buffer.capacity()
    }

    /// For real results the fft is symmetric and we
    /// get the amplitude by summing the magnitudes of
    /// X[k] and X[-k] for 0 <= k < (len(X) / 2)
    /// We place the result in the real buffer.
    fn prepare_real_result_from_fft_buffer(&mut self, input_size: T) -> Result<(), Box<dyn Error>> {
        let datapoints = self.sample_window();
        let half_datapoints = self.sample_window() / 2;

        for sample_index in 0..half_datapoints {
            let first_half_freq = self.buffer[sample_index];
            let second_half_freq = self.buffer[datapoints - 1 - sample_index];
            let frequency = frequency_in_hz_of_sample(sample_index, datapoints, self.sample_rate)?;
            let amplitude = (first_half_freq + second_half_freq).magnitude() / input_size;

            self.result_buffer[sample_index] = (frequency, amplitude);
        }

        Ok(())
    }

    /// Take a set of real values and return the real frequencies in hz and amplitudes from the
    /// FFT.
    pub fn run(&'a mut self, data: &[T]) -> Result<&'a [(T, T)], Box<dyn Error>> {
        let input_size: T = T::from(data.len()).ok_or("could not convert usize to T")?;
        self.prepare_buffer(data)?;
        do_fft(&mut self.buffer, false).expect("do_fft failed. probably not a power of two");
        self.prepare_real_result_from_fft_buffer(input_size)?;
        Ok(&self.result_buffer[..])
    }
}
