/**
 * A rust implementation of fast fourier transforms.
 * Uses an algorithm described at https://cp-algorithms.com/algebra/fft.html.
 */

use crate::complex::Complex;
use num::traits::Float;
use std::error::Error;

fn is_power_of_two(x: usize) -> bool {
    (x > 0) & ((x & (x - 1)) == 0)
}

fn c<T: Float>(input: f64) -> Result<T, Box<dyn Error>> {
    Ok(T::from::<f64>(input).ok_or("cannot convert f64 to T")?)
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
    // len tracks the current number of elements for the next butterfly
    while len <= max_len {
        // The complex length we multiply elements by varies by the length of the DFT so we can
        // cache it between iterations at the same level but not between applications at different
        // levels.
        // TODO: If we have some struct FFT we could cache the permutation list and this in memory.
        let angle: f64 = 2. * (std::f64::consts::PI / len as f64) * if inverse { -1. } else { 1. };
        let wlen = Complex::complex(c(angle.cos())?, c(angle.sin())?);
        let half_len = len / 2;

        for i in (0..max_len).step_by(len) {
            let mut w = Complex::real(c(1.)?);
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
            input[i] /= Complex::real(c(max_len as f64)?);
        }
    }

    Ok(())
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
