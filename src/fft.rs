use std::error::Error;
use crate::complex::Complex;

fn is_power_of_two(x: usize) -> bool {
    (x > 0) & ((x & (x - 1)) == 0)
}

pub fn do_fft(input: &[Complex], inverse: bool) -> Result<Vec<Complex>, Box<dyn Error>> {
    // TODO: We can do this in-place. Refactor.
    if !is_power_of_two(input.len()) {
        return Err("expected a power of two".into())
    }

    if input.len() < 2 {
      return Ok(input.to_vec());
    }

    let len = input.len();

    let mut even = Vec::new();
    let mut odd = Vec::new();

    for i in 0..(len / 2) {
        even.push(input[2*i]);
        odd.push(input[2*i+1]);
    }

    let even = do_fft(&even, inverse)?;
    let odd = do_fft(&odd, inverse)?;

    let inverse_angle = if inverse { -1. } else { 1. };
    let angle = inverse_angle * 2. * std::f64::consts::PI / (len as f64);
    let mut w = Complex::real(1.);
    let c_angle = Complex::complex(angle.cos(), angle.sin());
    let mut result = vec![Complex::real(0.); len];

    for i in 0..(len / 2) {
        result[i] = even[i] + (w * odd[i]);
        result[i + (len / 2)] = even[i] - (w * odd[i]);

        if inverse {
            result[i] /= Complex::real(2.);
            result[i + (len / 2)] /= Complex::real(2.);
        }

        w *= c_angle;
    }

    Ok(result)
}

#[cfg(test)]
mod fft_test {

    fn vec(sz: usize) -> Vec<Complex> {
        let mut f = Vec::with_capacity(sz);
        for i in 0..sz {
            f.push(i + 20);
        }
        f
    }

    #[test]
    fn test_no_pow_2() {
        let inp = vec(20);
        assert_eq!(do_fft(&inp, false).is_err(), true);
    }
}
