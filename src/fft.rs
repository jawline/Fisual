use crate::complex::Complex;

pub fn do_fft(input: &[Complex], inverse: bool) -> Vec<Complex> {
    // TODO: We can do this in-place. Refactor.

    if input.len() < 2 {
      return input.to_vec();
    }

    let len = input.len();

    let mut even = Vec::new();
    let mut odd = Vec::new();

    for i in 0..(len / 2) {
        even.push(input[2*i]);
        odd.push(input[2*i+1]);
    }

    let even = do_fft(&even, inverse);
    let odd = do_fft(&odd, inverse);

    let angle = 2. * std::f64::consts::PI / (len as f64);
    let mut w = Complex::real(1.);
    let c_angle = Complex::complex(angle.cos(), angle.sin());
    let mut result = vec![Complex::real(0.); len];

    for i in 0..(len / 2) {
        result[i] = even[i] + (w * odd[i]);
        result[i + (len / 2)] = even[i] - (w * odd[i]);
        w *= c_angle;
    }

    result
}
